#!/bin/sh

cd "$(dirname "$0")"

mkdir -p cpp/platform/linux/generated

./generate_library_loader.py \
	--name avutil \
	--output-cc cpp/platform/linux/generated/avutil_loader.cpp \
	--output-h cpp/platform/linux/generated/avutil_loader.h \
	--header '<stdint.h>
#include <libavutil/avutil.h>
#include <libavutil/dict.h>
#include <libavutil/opt.h>
#include <libavutil/hwcontext.h>
#include <libavutil/hwcontext_vulkan.h>' \
	--use-extern-c \
	av_buffer_alloc av_buffer_ref av_buffer_unref av_dict_set av_frame_alloc av_frame_free av_frame_get_buffer av_frame_unref av_free av_hwdevice_ctx_create av_hwframe_ctx_alloc av_hwframe_ctx_init av_hwframe_get_buffer av_hwframe_map av_hwframe_transfer_data av_log_set_callback av_log_set_level av_opt_set av_strdup av_strerror av_vkfmt_from_pixfmt av_vk_frame_alloc

./generate_library_loader.py \
	--name avcodec \
	--output-cc cpp/platform/linux/generated/avcodec_loader.cpp \
	--output-h cpp/platform/linux/generated/avcodec_loader.h \
	--header '<libavcodec/avcodec.h>' \
	--use-extern-c \
	avcodec_alloc_context3 avcodec_find_encoder_by_name avcodec_free_context avcodec_open2 avcodec_receive_packet avcodec_send_frame av_packet_alloc av_packet_free

./generate_library_loader.py \
	--name avfilter \
	--output-cc cpp/platform/linux/generated/avfilter_loader.cpp \
	--output-h cpp/platform/linux/generated/avfilter_loader.h \
	--header '<stdint.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavfilter/avfilter.h>' \
	--use-extern-c \
	av_buffersink_get_frame av_buffersrc_add_frame_flags av_buffersrc_parameters_alloc av_buffersrc_parameters_set avfilter_get_by_name avfilter_graph_alloc avfilter_graph_alloc_filter avfilter_graph_config avfilter_graph_create_filter avfilter_graph_free avfilter_graph_parse_ptr avfilter_inout_alloc avfilter_inout_free

./generate_library_loader.py \
	--name swscale \
	--output-cc cpp/platform/linux/generated/swscale_loader.cpp \
	--output-h cpp/platform/linux/generated/swscale_loader.h \
	--header '<libswscale/swscale.h>' \
	--use-extern-c \
	sws_getContext sws_scale
