#include "CEncoder.h"

#include <algorithm>
#include <asm-generic/errno-base.h>
#include <chrono>
#include <iterator>
#include <memory>
#include <drm/drm_fourcc.h>

#include "ALVR-common/packet_types.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Statistics.h"

#include "Edid.h"
#include "alvr_server/driverlog.h"

extern "C" {
#include <libavdevice/avdevice.h>
#include <libavcodec/avcodec.h>
#include <libavutil/avutil.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavutil/opt.h>
#include <libavutil/pixdesc.h>
}
#include <thread>

static void throw_averror(const char * msg, int errnum)
{
	char av_msg[AV_ERROR_MAX_STRING_SIZE];
	av_strerror(errnum, av_msg, sizeof(av_msg));
	throw MakeException(msg, av_msg);
}

static const char * encoder_name()
{
	switch (Settings::Instance().m_codec)
	{
		case ALVR_CODEC_H264:
			return "h264_vaapi";
		case ALVR_CODEC_H265:
			return "h265_vaapi";
	}
	throw MakeException("unkown codec %d", Settings::Instance().m_codec);
}

static void skipAUD_h265(char **buffer, int *length) {
	// H.265 encoder always produces AUD NAL even if AMF_VIDEO_ENCODER_HEVC_INSERT_AUD is set. But it is not needed.
	static const int AUD_NAL_SIZE = 7;

	if (*length < AUD_NAL_SIZE + 4) {
		return;
	}

	// Check if start with AUD NAL.
	if (memcmp(*buffer, "\x00\x00\x00\x01\x46", 5) != 0) {
		return;
	}
	// Check if AUD NAL size is AUD_NAL_SIZE bytes.
	if (memcmp(*buffer + AUD_NAL_SIZE, "\x00\x00\x00\x01", 4) != 0) {
		return;
	}
	*buffer += AUD_NAL_SIZE;
	*length -= AUD_NAL_SIZE;
}

static void set_hwframe_ctx(AVCodecContext *ctx, AVBufferRef *hw_device_ctx, int width, int height)
{
	AVBufferRef *hw_frames_ref;
	AVHWFramesContext *frames_ctx = NULL;
	int err = 0;

	if (!(hw_frames_ref = av_hwframe_ctx_alloc(hw_device_ctx))) {
		throw MakeException("Failed to create VAAPI frame context.");
	}
	frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
	frames_ctx->format = AV_PIX_FMT_VAAPI;
	frames_ctx->sw_format = AV_PIX_FMT_NV12;
	frames_ctx->width = width;
	frames_ctx->height = height;
	frames_ctx->initial_pool_size = 20;
	if ((err = av_hwframe_ctx_init(hw_frames_ref)) < 0) {
		av_buffer_unref(&hw_frames_ref);
		throw_averror("Failed to initialize VAAPI frame context. Error code: %s",err);
	}
	ctx->hw_frames_ctx = av_buffer_ref(hw_frames_ref);
	if (!ctx->hw_frames_ctx)
		err = AVERROR(ENOMEM);

	av_buffer_unref(&hw_frames_ref);
}

CEncoder::CEncoder(std::shared_ptr<ClientConnection> listener):
	m_listener(listener)
{
	int err = av_hwdevice_ctx_create(&hw_device_ctx, AV_HWDEVICE_TYPE_VAAPI, NULL, NULL, 0);
	if (err < 0) {
		throw_averror("Failed to create a VAAPI device. Error code: %s", err);
	}

	codec = avcodec_find_encoder_by_name(encoder_name());
	if (codec == nullptr)
	{
		throw MakeException("Failed to find encoder %s", encoder_name());
	}

}

CEncoder::~CEncoder()
{
	Stop();
	av_buffer_unref(&hw_device_ctx);
}

namespace
{

auto crate_kmsgrab_ctx(const char * device_name)
{
	AVInputFormat * kmsgrab = NULL;
	avdevice_register_all();
	while (true)
	{
		kmsgrab = av_input_video_device_next(kmsgrab);
		if (not kmsgrab)
			throw std::runtime_error("failed to find kmsgrab device");
		if (kmsgrab->name == std::string("kmsgrab"))
			break;
	}

	AVFormatContext *kmsgrabctx = avformat_alloc_context();
	AVDictionary * opt = NULL;
	av_dict_set(&opt, "device", device_name, 0);
	av_dict_set_int(&opt, "crtc_id", 57, 0);

	int err = avformat_open_input(&kmsgrabctx, "-", kmsgrab, &opt);
	if (err) {
		throw_averror("kmsgrab open failed: ", err);
	}
	return std::unique_ptr<AVFormatContext, std::function<void(AVFormatContext *)>>{
		kmsgrabctx,
			[](AVFormatContext *p){avformat_close_input(&p);}
	};
}

void logfn(void*, int level, const char* data, va_list va)
{
	DriverLogVarArgs(data, va);
}

}

void CEncoder::Run()
{
	// make sure display is set up, and we need to stream
	do {
	} while (not m_exiting);
	if (m_exiting)
	{
		return;
	}

	av_log_set_level(AV_LOG_DEBUG);
	av_log_set_callback(logfn);

	auto width = Settings::Instance().m_renderWidth;
	auto height = Settings::Instance().m_renderHeight;
	auto refresh = Settings::Instance().m_refreshRate;
	int codec_id = Settings::Instance().m_codec;

	int err;
	std::unique_ptr<AVCodecContext, std::function<void(AVCodecContext*)>> avctx{
		avcodec_alloc_context3(codec),
			[](AVCodecContext *p) {avcodec_free_context(&p);}
	};

	av_opt_set_int(avctx.get(), "rc_mode", 2, 0); // constant bitrate
	av_opt_set_int(avctx.get(), "quality", 7, 0); // low quality, fast encoding
	av_opt_set_int(avctx.get(), "profile", 77, 0); // main

	avctx->width = width;
	avctx->height = height;
	avctx->time_base = (AVRational){1, refresh};
	avctx->framerate = (AVRational){refresh, 1};
	avctx->sample_aspect_ratio = (AVRational){1, 1};
	avctx->pix_fmt = AV_PIX_FMT_VAAPI;

	avctx->bit_rate = Settings::Instance().mEncodeBitrateMBs * 8 * 1024 * 1024;

	/* set hw_frames_ctx for encoder's AVCodecContext */
	set_hwframe_ctx(avctx.get(), hw_device_ctx, avctx->width, avctx->height);

	if ((err = avcodec_open2(avctx.get(), codec, NULL)) < 0) {
		throw_averror("Cannot open video encoder codec. Error code: %s", err);
	}

	auto kmsgrabctx = crate_kmsgrab_ctx("/dev/dri/card0");

	auto filter_in = avfilter_get_by_name("buffer");
	auto filter_out = avfilter_get_by_name("buffersink");

	std::unique_ptr<AVFilterGraph, std::function<void(AVFilterGraph*)>> graph{
		avfilter_graph_alloc(),
			[](AVFilterGraph* p) {avfilter_graph_free(&p);}
	};

	AVFilterInOut *outputs = avfilter_inout_alloc();
	AVFilterInOut *inputs = avfilter_inout_alloc();

	AVPacket packet;
	av_init_packet(&packet);
	av_read_frame(kmsgrabctx.get(), &packet);
	AVFrame * frame = (AVFrame*)packet.data;

	AVFilterContext *filter_in_ctx = avfilter_graph_alloc_filter(graph.get(), filter_in, "in");

	AVBufferSrcParameters *par = av_buffersrc_parameters_alloc();
	memset(par, 0, sizeof(*par));
	auto kmsstream = kmsgrabctx->streams[0];
	par->width = kmsstream->codecpar->width;
	par->height = kmsstream->codecpar->height;
	par->time_base = kmsstream->time_base;
	par->format = kmsstream->codecpar->format;
	par->hw_frames_ctx = av_buffer_ref(frame->hw_frames_ctx);
	av_buffersrc_parameters_set(filter_in_ctx, par);
	av_free(par);

	av_packet_unref(&packet);

	AVFilterContext *filter_out_ctx;
	if ((err = avfilter_graph_create_filter(&filter_out_ctx, filter_out, "out", NULL, NULL, graph.get())))
	{
		throw_averror("filter_out creation failed:", err);
	}

	outputs->name = av_strdup("in");
	outputs->filter_ctx = filter_in_ctx;
	outputs->pad_idx = 0;
	outputs->next = NULL;

	inputs->name = av_strdup("out");
	inputs->filter_ctx = filter_out_ctx;
	inputs->pad_idx = 0;
	inputs->next = NULL;

	if ((err = avfilter_graph_parse_ptr(graph.get(), "hwmap,scale_vaapi=format=nv12",
					&inputs, &outputs, NULL)) < 0)
	{
		throw_averror("avfilter_graph_parse_ptr failed:", err);
	}

	avfilter_inout_free(&outputs);
	avfilter_inout_free(&inputs);

	for (int i = 0 ; i < graph->nb_filters; ++i)
	{
		graph->filters[i]->hw_device_ctx = av_buffer_ref(hw_device_ctx);
	}

	if ((err = avfilter_graph_config(graph.get(), NULL)))
	{
		throw_averror("avfilter_graph_config failed:", err);
	}

	AVFrame *hw_frame = NULL;
	if (!(hw_frame = av_frame_alloc())) {
		throw std::runtime_error("failed to allocate hw frame");
	}

	auto frame_time = std::chrono::duration<double>(1. / refresh);
	auto next_frame = std::chrono::steady_clock::now() + frame_time;
	std::vector<char> frame_data;
	for(int frame_idx = 0; not m_exiting; ++frame_idx) {
		//m_screen.process_events();
		auto frame_start = std::chrono::steady_clock::now();

		AVPacket packet;
		av_read_frame(kmsgrabctx.get(), &packet);
		err = av_buffersrc_add_frame_flags(filter_in_ctx, (AVFrame*)packet.data, AV_BUFFERSRC_FLAG_PUSH);
		if (err != 0)
		{
			throw_averror("av_buffersrc_add_frame failed", err);
		}
		av_buffersink_get_frame(filter_out_ctx, hw_frame);
		av_packet_unref(&packet);

 		{
			int ret = 0;
			AVPacket enc_pkt;

			av_init_packet(&enc_pkt);
			enc_pkt.data = NULL;
			enc_pkt.size = 0;

			if ((ret = avcodec_send_frame(avctx.get(), hw_frame)) < 0) {
				throw_averror("Error code: %s\n", ret);
			}
			av_frame_unref(hw_frame);

			frame_data.clear();
			while (1) {
				ret = avcodec_receive_packet(avctx.get(), &enc_pkt);
				if (ret == AVERROR(EAGAIN)) {
					break;
				} else if (ret) {
					throw std::runtime_error("failed to encode");
				}

				std::copy(enc_pkt.data, enc_pkt.data + enc_pkt.size, std::back_inserter(frame_data));

				enc_pkt.stream_index = 0;
				av_packet_unref(&enc_pkt);
			}
		}

		m_listener->GetStatistics()->EncodeOutput(
				std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::steady_clock::now() - frame_start).count()
				);

		char * video = frame_data.data();
		int video_len = frame_data.size();
		if (codec_id == ALVR_CODEC_H265)
		{
			skipAUD_h265(&video, &video_len);
		}

		m_listener->SendVideo((uint8_t*)video, video_len, frame_idx);

		std::this_thread::sleep_until(next_frame);
		next_frame += frame_time;

		std::this_thread::sleep_until(next_frame);
		next_frame += frame_time;
	}
	av_frame_free(&hw_frame);
}

void CEncoder::Stop()
{
	m_exiting = true;
	Join();
}

void CEncoder::OnPacketLoss()
{
	m_scheduler.OnPacketLoss();
}

void CEncoder::InsertIDR() {
	m_scheduler.InsertIDR();
}
