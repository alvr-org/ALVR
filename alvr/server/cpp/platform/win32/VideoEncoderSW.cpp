#ifdef ALVR_GPL

#include "VideoEncoderSW.h"

#include "alvr_server/Statistics.h"
#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Utils.h"

#include <iostream>
#include <string>
#include <array>
#include <algorithm>

VideoEncoderSW::VideoEncoderSW(std::shared_ptr<CD3DRender> d3dRender
	, std::shared_ptr<ClientConnection> listener
	, int width, int height)
	: m_d3dRender(d3dRender)
	, m_Listener(listener)
	, m_codec((ALVR_CODEC)Settings::Instance().m_codec)
	, m_refreshRate(Settings::Instance().m_refreshRate)
	, m_renderWidth(width)
	, m_renderHeight(height)
	, m_bitrateInMBits(Settings::Instance().mEncodeBitrateMBs) {
#ifdef ALVR_DEBUG_LOG
	av_log_set_level(AV_LOG_DEBUG);
	av_log_set_callback(LibVALog);
	Debug("Set FFMPEG/LibAV to debug logging");
#endif
	}

VideoEncoderSW::~VideoEncoderSW() {}

void VideoEncoderSW::LibVALog(void* v, int level, const char* data, va_list va) {
	const char* prefix = "[libav]: ";
	std::stringstream sstream;
	sstream << prefix << data;
	vprintf(sstream.str().c_str(), va);
}

void VideoEncoderSW::Initialize() {
	int err;
	Debug("Initializing VideoEncoderSW.\n");

	// Query codec
	AVCodecID codecId = ToFFMPEGCodec(m_codec);
	if(!codecId) throw MakeException("Invalid requested codec %d", m_codec);
	
	const AVCodec *codec = avcodec_find_encoder(codecId);
	if(codec == NULL) throw MakeException("Could not find codec id %d", codecId);

	// Initialize CodecContext
	m_codecContext = avcodec_alloc_context3(codec);
	if(m_codecContext == NULL) throw MakeException("Failed to allocate encoder id %d", codecId);

	// Set codec settings
	AVDictionary* opt = NULL;
	av_dict_set(&opt, "preset", "ultrafast", 0);
	av_dict_set(&opt, "tune", "zerolatency", 0);
	switch (m_codec) {
		case ALVR_CODEC_H264:
			m_codecContext->profile = Settings::Instance().m_use10bitEncoder ? FF_PROFILE_H264_HIGH_10 : FF_PROFILE_H264_HIGH;
			break;
		case ALVR_CODEC_H265:
			m_codecContext->profile = Settings::Instance().m_use10bitEncoder ? FF_PROFILE_HEVC_MAIN_10 : FF_PROFILE_HEVC_MAIN;
			break;
	}

	m_codecContext->width = Settings::Instance().m_renderWidth;
	m_codecContext->height = Settings::Instance().m_renderHeight;
	m_codecContext->time_base = AVRational{std::chrono::steady_clock::period::num, std::chrono::steady_clock::period::den};
	m_codecContext->framerate = AVRational{Settings::Instance().m_refreshRate, 1};
	m_codecContext->sample_aspect_ratio = AVRational{1, 1};
	m_codecContext->pix_fmt = Settings::Instance().m_use10bitEncoder ? AV_PIX_FMT_YUV420P10LE : AV_PIX_FMT_YUV420P;
	m_codecContext->max_b_frames = 0;
	m_codecContext->bit_rate = Settings::Instance().mEncodeBitrateMBs * 1000 * 1000;
	m_codecContext->thread_count = Settings::Instance().m_swThreadCount;

	if((err = avcodec_open2(m_codecContext, codec, &opt))) throw MakeException("Cannot open video encoder codec: %d", err);

	// Config transfer/encode frames
	m_transferredFrame = av_frame_alloc();
	m_transferredFrame->buf[0] = av_buffer_alloc(1);
	m_encoderFrame = av_frame_alloc();
	m_encoderFrame->width = Settings::Instance().m_renderWidth;
	m_encoderFrame->height = Settings::Instance().m_renderHeight;
	m_encoderFrame->format = m_codecContext->pix_fmt;
	if((err = av_frame_get_buffer(m_encoderFrame, 0))) throw MakeException("Error when allocating encoder frame: %d", err);

	Debug("Successfully initialized VideoEncoderSW");
}

void VideoEncoderSW::Shutdown() {
	Debug("Shutting down VideoEncoderSW.\n");

	av_frame_free(&m_transferredFrame);
	av_frame_free(&m_encoderFrame);

	avcodec_free_context(&m_codecContext);
	sws_freeContext(m_scalerContext);
	m_scalerContext = nullptr;

	Debug("Successfully shutdown VideoEncoderSW.\n");
}

bool VideoEncoderSW::should_keep_nal_h264(const uint8_t *header_start) {
 	uint8_t nal_type = (header_start[2] == 0 ? header_start[4] : header_start[3]) & 0x1F;
    switch (nal_type) {
		case 6: // supplemental enhancement information
		case 9: // access unit delimiter
			return false;
		default:
			return true;
    }
}

bool VideoEncoderSW::should_keep_nal_h265(const uint8_t *header_start) {
	uint8_t nal_type = ((header_start[2] == 0 ? header_start[4] : header_start[3]) >> 1) & 0x3F;
	switch (nal_type) {
		case 35: // access unit delimiter
		case 39: // supplemental enhancement information
		return false;
		default:
		return true;
	}
}

void VideoEncoderSW::filter_NAL(const uint8_t *input, size_t input_size, std::vector<uint8_t> &out)
{
	if (input_size < 4) return;
	ALVR_CODEC codec = m_codec;
	std::array<uint8_t, 3> header = {{0, 0, 1}};
	const uint8_t *end = input + input_size;
	const uint8_t *header_start = input;
	while (header_start != end) {
		const uint8_t *next_header = std::search(header_start + 3, end, header.begin(), header.end());
		if (next_header != end && next_header[-1] == 0) next_header--;
		if (codec == ALVR_CODEC_H264 && should_keep_nal_h264(header_start))
		out.insert(out.end(), header_start, next_header);
		if (codec == ALVR_CODEC_H265 && should_keep_nal_h265(header_start))
		out.insert(out.end(), header_start, next_header);
		header_start = next_header;
	}
}

void VideoEncoderSW::Transmit(ID3D11Texture2D *pTexture, uint64_t presentationTime, uint64_t targetTimestampNs, bool insertIDR) {
	// Handle bitrate changes
	if(m_Listener->GetStatistics()->CheckBitrateUpdated()) {
		//Debug("Bitrate changed");
		m_codecContext->bit_rate = m_Listener->GetStatistics()->GetBitrate() * 1000000L;
	}

	// Setup staging texture if not defined yet; we can only define it here as we now have the texture's size
	if(!stagingTex) {
		HRESULT hr = SetupStagingTexture(pTexture);
		if(FAILED(hr)) {
			Error("Failed to create staging texture: %p %ls", hr, GetErrorStr(hr).c_str());
			return;
		}
		Debug("Success in creating staging texture");
	}

	// Copy texture and map it to memory
	/// SteamVR crashes if the swapchain textures are set to staging, which is needed to be read by the CPU.
	/// Unless there's another solution we have to copy the texture every time, which is gonna be another performance hit.
	HRESULT hr = CopyTexture(pTexture);
	if(FAILED(hr)) {
		Error("Failed to copy texture to staging: %p %ls", hr, GetErrorStr(hr).c_str());
		return;
	}
	//Debug("Success in mapping staging texture");

	// Setup software scaler if not defined yet; we can only define it here as we now have the texture's size
	// FIXME: Hardcoded to DirectX's R8G8B8A8, make more robust system if needed
	if(!m_scalerContext) {
		m_scalerContext = sws_getContext(stagingTexDesc.Width, stagingTexDesc.Height, AV_PIX_FMT_RGBA,
		m_codecContext->width, m_codecContext->height, m_codecContext->pix_fmt,
		SWS_BILINEAR, NULL, NULL, NULL);
		if(!m_scalerContext) {
			Error("Couldn't initialize SWScaler.");
			m_d3dRender->GetContext()->Unmap(stagingTex.Get(), 0);
			return;
		}
		Debug("Successfully initialized SWScaler.");
	}

	// We got the texture, populate tansferredFrame with data
	m_transferredFrame->width = stagingTexDesc.Width;
	m_transferredFrame->height = stagingTexDesc.Height;
	m_transferredFrame->data[0] = (uint8_t*)stagingTexMap.pData;
	m_transferredFrame->linesize[0] = stagingTexMap.RowPitch;
	m_transferredFrame->format = AV_PIX_FMT_RGBA;
	m_transferredFrame->pts = std::chrono::steady_clock::now().time_since_epoch().count();

	// Use SWScaler for scaling
	if(sws_scale(m_scalerContext, m_transferredFrame->data, m_transferredFrame->linesize,
				0, m_transferredFrame->height, m_encoderFrame->data, m_encoderFrame->linesize) == 0) {
		Error("SWScale failed.");
		m_d3dRender->GetContext()->Unmap(stagingTex.Get(), 0);
		return;
	}
	//Debug("SWScale succeeded.");

	// Send frame for encoding
	m_encoderFrame->pict_type = insertIDR ? AV_PICTURE_TYPE_I : AV_PICTURE_TYPE_NONE;
	m_encoderFrame->pts = std::chrono::steady_clock::now().time_since_epoch().count();

	int err;
	if((err = avcodec_send_frame(m_codecContext, m_encoderFrame)) < 0) {
		Error("Encoding frame failed: err code %d", err);
		m_d3dRender->GetContext()->Unmap(stagingTex.Get(), 0);
		return;
	}
	//Debug("Send frame succeeded.");

	// Retrieve frames from encoding and send them until buffer is emptied
	while(true) {
		AVPacket *packet = av_packet_alloc();
		if((err = avcodec_receive_packet(m_codecContext, packet))) {
			if(err == AVERROR(EAGAIN)) {
				// Output buffer was emptied, move on
				break;
			} else {
				Error("Received encoded frame failed: err code %d", err);
				av_packet_free(&packet);
				m_d3dRender->GetContext()->Unmap(stagingTex.Get(), 0);
				return;
			}
		}
		//Debug("Received encoded packet");

		// Send encoded frame to client
		std::vector<uint8_t> encoded_data;
		filter_NAL(packet->data, packet->size, encoded_data);
		av_packet_free(&packet);
		m_Listener->SendVideo(encoded_data.data(), encoded_data.size(), targetTimestampNs);
		//Debug("Sent encoded packet to client");
	}

	// Send statistics to client
	m_Listener->GetStatistics()->EncodeOutput(GetTimestampUs() - presentationTime);

	// Unmap the copied texture and delete it
	m_d3dRender->GetContext()->Unmap(stagingTex.Get(), 0);
}

HRESULT VideoEncoderSW::SetupStagingTexture(ID3D11Texture2D *pTexture) {
	D3D11_TEXTURE2D_DESC desc;
	pTexture->GetDesc(&desc);
	stagingTexDesc.Width = desc.Width;
	stagingTexDesc.Height = desc.Height;
	stagingTexDesc.MipLevels = desc.MipLevels;
	stagingTexDesc.ArraySize = desc.ArraySize;
	stagingTexDesc.Format = desc.Format;
	stagingTexDesc.SampleDesc = desc.SampleDesc;
	stagingTexDesc.Usage = D3D11_USAGE_STAGING;
	stagingTexDesc.BindFlags = 0;
	stagingTexDesc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
	stagingTexDesc.MiscFlags = 0;

	return m_d3dRender->GetDevice()->CreateTexture2D(&stagingTexDesc, nullptr, &stagingTex);
}

HRESULT VideoEncoderSW::CopyTexture(ID3D11Texture2D *pTexture) {
	m_d3dRender->GetContext()->CopyResource(stagingTex.Get(), pTexture);
	return m_d3dRender->GetContext()->Map(stagingTex.Get(), 0, D3D11_MAP_READ, 0, &stagingTexMap);
}

AVCodecID VideoEncoderSW::ToFFMPEGCodec(ALVR_CODEC codec) {
	switch (codec) {
		case ALVR_CODEC_H264:
			return AV_CODEC_ID_H264;
		case ALVR_CODEC_H265:
			return AV_CODEC_ID_HEVC;
		default:
			return AV_CODEC_ID_NONE;
	}
}

#endif // ALVR_GPL