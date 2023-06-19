#include <iostream>
#include <vector>
#include <cstdint>
#include <png.h>
#include <stdlib.h>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libswscale/swscale.h>
#include <libavutil/imgutils.h>
}

void SaveFrameAsPNG(AVFrame* frame, const char* filename)
{
    AVCodecContext* codecContext = (AVCodecContext*)frame->opaque;
    int width = codecContext->width;
    int height = codecContext->height;

    AVFrame* rgbFrame = av_frame_alloc();
    if (!rgbFrame)
    {
        std::cerr << "Error allocating RGB frame\n";
        return;
    }

    int numBytes = av_image_get_buffer_size(AV_PIX_FMT_RGB24, width, height, 1);
    uint8_t* buffer = (uint8_t*)av_malloc(numBytes * sizeof(uint8_t));
    if (!buffer)
    {
        std::cerr << "Error allocating buffer for RGB frame\n";
        av_frame_free(&rgbFrame);
        return;
    }

    av_image_fill_arrays(rgbFrame->data, rgbFrame->linesize, buffer, AV_PIX_FMT_RGB24, width, height, 1);

    struct SwsContext* swsContext = sws_getContext(width, height, codecContext->pix_fmt, width, height, AV_PIX_FMT_RGB24, SWS_BILINEAR, nullptr, nullptr, nullptr);
    if (!swsContext)
    {
        std::cerr << "Error creating SwsContext\n";
        av_freep(&buffer);
        av_frame_free(&rgbFrame);
        return;
    }

    sws_scale(swsContext, frame->data, frame->linesize, 0, height, rgbFrame->data, rgbFrame->linesize);

    FILE* file = fopen(filename, "wb");
    if (!file)
    {
        std::cerr << "Error opening file " << filename << "\n";
        av_freep(&buffer);
        av_frame_free(&rgbFrame);
        sws_freeContext(swsContext);
        return;
    }

    png_structp png = png_create_write_struct(PNG_LIBPNG_VER_STRING, nullptr, nullptr, nullptr);
    if (!png)
    {
        std::cerr << "Error creating PNG write struct\n";
        fclose(file);
        av_freep(&buffer);
        av_frame_free(&rgbFrame);
        sws_freeContext(swsContext);
        return;
    }

    png_infop info = png_create_info_struct(png);
    if (!info)
    {
        std::cerr << "Error creating PNG info struct\n";
        png_destroy_write_struct(&png, nullptr);
        fclose(file);
        av_freep(&buffer);
        av_frame_free(&rgbFrame);
        sws_freeContext(swsContext);
        return;
    }

    if (setjmp(png_jmpbuf(png)))
    {
        std::cerr << "Error writing PNG image\n";
        png_destroy_write_struct(&png, &info);
        fclose(file);
        av_freep(&buffer);
        av_frame_free(&rgbFrame);
        sws_freeContext(swsContext);
        return;
    }

    png_init_io(png, file);

    png_set_IHDR(png, info, width, height, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);

    png_write_info(png, info);

    png_bytep* rowPointers = (png_bytep*)malloc(sizeof(png_bytep) * height);
    for (int i = 0; i < height; ++i)
    {
        rowPointers[i] = rgbFrame->data[0] + i * rgbFrame->linesize[0];
    }

    png_write_image(png, rowPointers);

    png_write_end(png, nullptr);

    png_destroy_write_struct(&png, &info);

    fclose(file);

    av_freep(&buffer);
    av_frame_free(&rgbFrame);
    sws_freeContext(swsContext);
    free(rowPointers);
}

void DecodeH264(const char* inputFilename, const char* outputPrefix)
{
    //av_register_all();

    AVFormatContext* formatContext = nullptr;
    if (avformat_open_input(&formatContext, inputFilename, nullptr, nullptr) != 0)
    {
        std::cerr << "Error opening input file\n";
        return;
    }

    if (avformat_find_stream_info(formatContext, nullptr) < 0)
    {
        std::cerr << "Error finding stream info\n";
        avformat_close_input(&formatContext);
        return;
    }

    AVCodec* codec = nullptr;
    int streamIndex = av_find_best_stream(formatContext, AVMEDIA_TYPE_VIDEO, -1, -1, const_cast<const AVCodec**>(&codec), 0);
    if (streamIndex < 0)
    {
        std::cerr << "Error finding video stream\n";
        avformat_close_input(&formatContext);
        return;
    }

    AVCodecContext* codecContext = avcodec_alloc_context3(codec);
    if (!codecContext)
    {
        std::cerr << "Error allocating codec context\n";
        avformat_close_input(&formatContext);
        return;
    }

    if (avcodec_parameters_to_context(codecContext, formatContext->streams[streamIndex]->codecpar) < 0)
    {
        std::cerr << "Error setting codec parameters\n";
        avcodec_free_context(&codecContext);
        avformat_close_input(&formatContext);
        return;
    }

    if (avcodec_open2(codecContext, codec, nullptr) < 0)
    {
        std::cerr << "Error opening codec\n";
        avcodec_free_context(&codecContext);
        avformat_close_input(&formatContext);
        return;
    }

    AVPacket packet;
    av_init_packet(&packet);
    packet.data = nullptr;
    packet.size = 0;

    AVFrame* frame = av_frame_alloc();
    if (!frame)
    {
        std::cerr << "Error allocating frame\n";
        avcodec_free_context(&codecContext);
        avformat_close_input(&formatContext);
        return;
    }

    int frameCount = 0;
    while (av_read_frame(formatContext, &packet) >= 0)
    {
        if (packet.stream_index == streamIndex)
        {
            int ret = avcodec_send_packet(codecContext, &packet);
            if (ret < 0)
            {
                std::cerr << "Error sending packet to decoder\n";
                continue;
            }

            while (ret >= 0)
            {
                ret = avcodec_receive_frame(codecContext, frame);
                if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF)
                {
                    break;
                }
                else if (ret < 0)
                {
                    std::cerr << "Error receiving frame from decoder\n";
                    continue;
                }

                char outputFilename[256];
                sprintf(outputFilename, "%s_%d.png", outputPrefix, frameCount++);
                SaveFrameAsPNG(frame, outputFilename);
            }
        }

        av_packet_unref(&packet);
    }

    avcodec_free_context(&codecContext);
    avformat_close_input(&formatContext);
    av_frame_free(&frame);
}