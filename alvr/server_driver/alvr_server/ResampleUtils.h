#pragma once
extern "C" {
#include <libavutil/opt.h>
#include <libavutil/channel_layout.h>
#include <libavutil/samplefmt.h>
#include <libswresample/swresample.h>
};

#include "Logger.h"

class Resampler {
public:
	Resampler(int src_rate, int dst_rate, int src_channels, int dst_channels) {
		this->src_rate = src_rate;
		this->dst_rate = dst_rate;
		this->src_ch_layout = mapChannelLayout(src_channels);
		this->dst_ch_layout = mapChannelLayout(dst_channels);
		Initialize();
	}

	~Resampler() {
		if (src_data)
			av_freep(&src_data[0]);
		av_freep(&src_data);
		if (dst_data)
			av_freep(&dst_data[0]);
		av_freep(&dst_data);
		swr_free(&swr_ctx);
	}

	int64_t mapChannelLayout(int channels) {
		switch (channels) {
		case 1:
			return AV_CH_LAYOUT_MONO;
		case 2:
			return AV_CH_LAYOUT_STEREO;
		case 3:
			return AV_CH_LAYOUT_2POINT1;
		case 4:
			return AV_CH_LAYOUT_3POINT1;
		case 5:
			return AV_CH_LAYOUT_4POINT1;
		case 6:
			return AV_CH_LAYOUT_5POINT1;
		case 7:
			return AV_CH_LAYOUT_6POINT1;
		case 8:
			return AV_CH_LAYOUT_7POINT1;
		default:
			return AV_CH_LAYOUT_MONO;
		}
	}

	void Initialize() {
		int ret;

		LogDriver("Initialize swresample. src_rate=%d dst_rate=%d", src_rate, dst_rate);

		/* create resampler context */
		swr_ctx = swr_alloc();
		if (!swr_ctx) {
			throw MakeException(L"Could not allocate resampler context\n");
		}
		/* set options */
		av_opt_set_int(swr_ctx, "in_channel_layout", src_ch_layout, 0);
		av_opt_set_int(swr_ctx, "in_sample_rate", src_rate, 0);
		av_opt_set_sample_fmt(swr_ctx, "in_sample_fmt", src_sample_fmt, 0);
		av_opt_set_int(swr_ctx, "out_channel_layout", dst_ch_layout, 0);
		av_opt_set_int(swr_ctx, "out_sample_rate", dst_rate, 0);
		av_opt_set_sample_fmt(swr_ctx, "out_sample_fmt", dst_sample_fmt, 0);
		/* initialize the resampling context */
		if ((ret = swr_init(swr_ctx)) < 0) {
			throw MakeException(L"Failed to initialize the resampling context\n");
		}
		/* allocate source and destination samples buffers */
		src_nb_channels = av_get_channel_layout_nb_channels(src_ch_layout);
		ret = av_samples_alloc_array_and_samples(&src_data, &src_linesize, src_nb_channels,
			default_src_nb_samples, src_sample_fmt, 0);
		if (ret < 0) {
			throw MakeException(L"Could not allocate source samples\n");
		}
		/* compute the number of converted samples: buffering is avoided
		* ensuring that the output buffer will contain at least all the
		* converted input samples */
		max_dst_nb_samples = dst_nb_samples =
			(int) av_rescale_rnd(default_src_nb_samples, dst_rate, src_rate, AV_ROUND_UP);
		/* buffer is going to be directly written to a rawaudio file, no alignment */
		dst_nb_channels = av_get_channel_layout_nb_channels(dst_ch_layout);
		ret = av_samples_alloc_array_and_samples(&dst_data, &dst_linesize, dst_nb_channels,
			dst_nb_samples, dst_sample_fmt, 0);
		if (ret < 0) {
			throw MakeException(L"Could not allocate destination samples. %hs", GetErrorStr(ret).c_str());
		}
		LogDriver("swresample successfully initialized. src_rate=%d dst_rate=%d"
			" src_nb_channels=%d dst_nb_channels=%d max_dst_nb_samples=%d"
			, src_rate, dst_rate, src_nb_channels, dst_nb_channels, max_dst_nb_samples);
	}

	void FeedInput(int src_nb_samples, uint8_t *src_frame_data) {
		int ret;

		/* compute destination number of samples */
		dst_nb_samples = (int) av_rescale_rnd(swr_get_delay(swr_ctx, src_rate) +
			src_nb_samples, dst_rate, src_rate, AV_ROUND_UP);
		if (dst_nb_samples > max_dst_nb_samples) {
			av_free(dst_data[0]);
			ret = av_samples_alloc(dst_data, &dst_linesize, dst_nb_channels,
				dst_nb_samples, dst_sample_fmt, 1);
			if (ret < 0) {
				throw MakeException(L"Error on av_samples_alloc. dst_nb_samples=%d", dst_nb_samples);
			}
			max_dst_nb_samples = dst_nb_samples;
		}
		/* convert to destination format */
		ret = swr_convert(swr_ctx, dst_data, dst_nb_samples, (const uint8_t **)&src_frame_data, src_nb_samples);
		if (ret < 0) {
			throw MakeException(L"Error on swr_convert.");
		}
		dst_bufsize = av_samples_get_buffer_size(&dst_linesize, dst_nb_channels,
			ret, dst_sample_fmt, 1);

		LogDriver("Converted. src_sample_fmt=%d dst_sample_fmt=%d src_nb_samples=%d ret=%d dst_bufsize=%d"
			, src_sample_fmt, dst_sample_fmt, src_nb_samples, ret, dst_bufsize);
	}

	uint8_t *GetDest() {
		return *dst_data;
	}

	int GetDestBufSize() {
		return dst_bufsize;
	}
private:
	std::string GetErrorStr(int err) {
		char buf[1000];
		av_strerror(err, buf, sizeof(buf));
		return buf;
	}
	struct SwrContext *swr_ctx = NULL;
	uint8_t **src_data = NULL, **dst_data = NULL;
	int src_linesize, dst_linesize;
	int src_rate;
	int64_t src_ch_layout;
	int dst_rate;
	int64_t dst_ch_layout;
	static const int default_src_nb_samples = 1024;
	int max_dst_nb_samples;
	int dst_nb_samples;
	int src_nb_channels = 0, dst_nb_channels = 0;
	int dst_bufsize;
	enum AVSampleFormat src_sample_fmt = AV_SAMPLE_FMT_S16, dst_sample_fmt = AV_SAMPLE_FMT_S16;
};