#include "ALVR-common/packet_types.h"
#include <cstdio>
#include <exception>
#include <fstream>
#include <functional>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <chrono>
#include <thread>
#include <vector>

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

namespace {

class AvException: public std::runtime_error
{
public:
	AvException(std::string msg, int averror): std::runtime_error{makemsg(msg, averror)} {}
private:
	static std::string makemsg(const std::string & msg, int averror)
	{
		char av_msg[AV_ERROR_MAX_STRING_SIZE];
		av_strerror(averror, av_msg, sizeof(av_msg));
		return msg + av_msg;
	}
};

void set_hwframe_ctx(AVCodecContext *ctx, AVBufferRef *hw_device_ctx, int width, int height)
{
	AVBufferRef *hw_frames_ref;
	AVHWFramesContext *frames_ctx = NULL;
	int err = 0;

	if (!(hw_frames_ref = av_hwframe_ctx_alloc(hw_device_ctx))) {
		throw std::runtime_error("Failed to create VAAPI frame context.");
	}
	frames_ctx = (AVHWFramesContext *)(hw_frames_ref->data);
	frames_ctx->format = AV_PIX_FMT_VAAPI;
	frames_ctx->sw_format = AV_PIX_FMT_NV12;
	frames_ctx->width = width;
	frames_ctx->height = height;
	frames_ctx->initial_pool_size = 20;
	if ((err = av_hwframe_ctx_init(hw_frames_ref)) < 0) {
		av_buffer_unref(&hw_frames_ref);
		throw AvException("Failed to initialize VAAPI frame context:", err);
	}
	ctx->hw_frames_ctx = av_buffer_ref(hw_frames_ref);
	if (!ctx->hw_frames_ctx)
		err = AVERROR(ENOMEM);

	av_buffer_unref(&hw_frames_ref);
}

auto crate_kmsgrab_ctx(const char * device_name, int framerate)
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
	av_dict_set_int(&opt, "crtc_id", 57, 0); //FIXME: find the crtc automatically (maybe based on resolution ?)
	av_dict_set_int(&opt, "framerate", framerate, 0);

	int err = avformat_open_input(&kmsgrabctx, "-", kmsgrab, &opt);
	if (err) {
		throw AvException("kmsgrab open failed: ", err);
	}
	return std::unique_ptr<AVFormatContext, std::function<void(AVFormatContext *)>>{
		kmsgrabctx,
			[](AVFormatContext *p){avformat_close_input(&p);}
	};
}

void logfn(void*, int level, const char* data, va_list va)
{
	vfprintf(stderr, data, va);
}

void skipAUD_h265(char **buffer, int *length) {
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

const char * encoder(int codec)
{
	switch (codec)
	{
		case ALVR_CODEC_H264:
			return "h264_vaapi";
		case ALVR_CODEC_H265:
			return "hevc_vaapi";
	}
	throw std::runtime_error("invalid codec " + std::to_string(codec));
}

}

int main(int argc, char ** argv)
{
	if (argc != 6)
	{
		std::cerr << "usage: " << argv[0]
			<< " width height refresh_rate(Hw) codec_id(0: h264 1:h265) bitrate(MB/s)" << std::endl;
		return 1;
	}

#if 0
	av_log_set_level(AV_LOG_DEBUG);
	av_log_set_callback(logfn);
#endif

	auto width = std::atoi(argv[1]) ;
	auto height = std::atoi(argv[2]);
	auto refresh = std::atoi(argv[3]);
	auto codec_id = std::atoi(argv[4]);
	auto bitrate = std::atoi(argv[5]);
	const char * encoder_name = encoder(codec_id);

	try {
		AVBufferRef *hw_device_ctx;
		int err = av_hwdevice_ctx_create(&hw_device_ctx, AV_HWDEVICE_TYPE_VAAPI, NULL, NULL, 0);
		if (err < 0) {
			throw AvException("Failed to create a VAAPI device: ", err);
		}

		AVCodec *codec = avcodec_find_encoder_by_name(encoder_name);
		if (codec == nullptr)
		{
			throw std::runtime_error(std::string("Failed to find encoder ") + encoder_name);
		}


		std::unique_ptr<AVCodecContext, std::function<void(AVCodecContext*)>> avctx{
			avcodec_alloc_context3(codec),
				[](AVCodecContext *p) {avcodec_free_context(&p);}
		};

		switch (codec_id)
		{
			case ALVR_CODEC_H264:
				av_opt_set_int(avctx.get(), "rc_mode", 2, 0); // constant bitrate
				av_opt_set_int(avctx.get(), "quality", 7, 0); // low quality, fast encoding
				av_opt_set_int(avctx.get(), "profile", 77, 0); // main
				break;
			case ALVR_CODEC_H265:
				av_opt_set(avctx.get(), "profile", "main", 0);
				break;
		}

		avctx->width = width;
		avctx->height = height;
		avctx->time_base = (AVRational){1, refresh};
		avctx->framerate = (AVRational){refresh, 1};
		avctx->sample_aspect_ratio = (AVRational){1, 1};
		avctx->pix_fmt = AV_PIX_FMT_VAAPI;

		avctx->bit_rate = bitrate * 8 * 1024 * 1024;

		/* set hw_frames_ctx for encoder's AVCodecContext */
		set_hwframe_ctx(avctx.get(), hw_device_ctx, avctx->width, avctx->height);

		if ((err = avcodec_open2(avctx.get(), codec, NULL)) < 0) {
			throw AvException("Cannot open video encoder codec: ", err);
		}

		auto kmsgrabctx = crate_kmsgrab_ctx("/dev/dri/card0", refresh); //FIXME: don't hardcode this

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
			throw AvException("filter_out creation failed: ", err);
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
			throw AvException("avfilter_graph_parse_ptr failed:", err);
		}

		avfilter_inout_free(&outputs);
		avfilter_inout_free(&inputs);

		for (int i = 0 ; i < graph->nb_filters; ++i)
		{
			graph->filters[i]->hw_device_ctx = av_buffer_ref(hw_device_ctx);
		}

		if ((err = avfilter_graph_config(graph.get(), NULL)))
		{
			throw AvException("avfilter_graph_config failed:", err);
		}

		AVFrame *hw_frame = NULL;
		if (!(hw_frame = av_frame_alloc())) {
			throw std::runtime_error("failed to allocate hw frame");
		}

		auto frame_time = std::chrono::duration<double>(1. / refresh);
		auto next_frame = std::chrono::steady_clock::now() + frame_time;
		std::vector<char> frame_data;
		for(int frame_idx = 0; ; ++frame_idx) {
			AVPacket packet;
			av_read_frame(kmsgrabctx.get(), &packet);
			err = av_buffersrc_add_frame_flags(filter_in_ctx, (AVFrame*)packet.data, AV_BUFFERSRC_FLAG_PUSH);
			if (err != 0)
			{
				throw AvException("av_buffersrc_add_frame failed", err);
			}
			av_buffersink_get_frame(filter_out_ctx, hw_frame);
			av_packet_unref(&packet);

			AVPacket enc_pkt;

			av_init_packet(&enc_pkt);
			enc_pkt.data = NULL;
			enc_pkt.size = 0;

			if ((err = avcodec_send_frame(avctx.get(), hw_frame)) < 0) {
				throw AvException("avcodec_send_frame failed: ", err);
			}
			av_frame_unref(hw_frame);

			frame_data.clear();
			int video_len = 0;
			while (1) {
				err = avcodec_receive_packet(avctx.get(), &enc_pkt);
				if (err == AVERROR(EAGAIN)) {
					break;
				} else if (err) {
					throw std::runtime_error("failed to encode");
				}
				frame_data.resize(video_len + enc_pkt.size);
				memcpy(&frame_data[video_len], enc_pkt.data, enc_pkt.size);
				video_len += enc_pkt.size;

				enc_pkt.stream_index = 0;
				av_packet_unref(&enc_pkt);
			}

			char * video = frame_data.data();
			if (codec_id == ALVR_CODEC_H265)
			{
				skipAUD_h265(&video, &video_len);
			}

			std::cout.write((char*)&video_len, sizeof(video_len));
			std::cout.write(video, video_len);

			std::this_thread::sleep_until(next_frame);
			next_frame += frame_time;
		}
		av_frame_free(&hw_frame);
		av_buffer_unref(&hw_device_ctx);
	}
	catch (std::exception &e)
	{
		std::cerr << "uncaught exception " << e.what() << std::endl;
		return 1;
	}
}
