#pragma once
#include "d3drender.h"

#include "threadtools.h"

#include <d3d11.h>
#include <wrl.h>
#include <map>
#include <d3d11_1.h>
#include <ScreenGrab.h>
#include <wincodec.h>
#include <wincodecsdk.h>
#include "ClientConnection.h"
#include "Utils.h"
#include "FrameRender.h"
#include "VideoEncoder.h"
#include "VideoEncoderNVENC.h"
#include "VideoEncoderVCE.h"
#include "IDRScheduler.h"


	using Microsoft::WRL::ComPtr;

	//----------------------------------------------------------------------------
	// Blocks on reading backbuffer from gpu, so WaitForPresent can return
	// as soon as we know rendering made it this frame.  This step of the pipeline
	// should run about 3ms per frame.
	//----------------------------------------------------------------------------
	class CEncoder : public CThread
	{
	public:
		CEncoder();
		~CEncoder();

		void Initialize(std::shared_ptr<CD3DRender> d3dRender, std::shared_ptr<ClientConnection> listener);

		bool CEncoder::CopyToStaging(ID3D11Texture2D *pTexture[][2], vr::VRTextureBounds_t bounds[][2], int layerCount, bool recentering
			, uint64_t presentationTime, uint64_t frameIndex, uint64_t clientTime, const std::string& message, const std::string& debugText);

		virtual void Run();

		virtual void Stop();

		void NewFrameReady();

		void WaitForEncode();

		void OnStreamStart();

		void OnPacketLoss();

		void Reconfigure(int refreshRate, int renderWidth, int renderHeight, int bitrateInMBits);

	private:
		CThreadEvent m_newFrameReady, m_encodeFinished;
		std::shared_ptr<VideoEncoder> m_videoEncoder;
		bool m_bExiting;
		uint64_t m_presentationTime;
		uint64_t m_frameIndex;
		uint64_t m_clientTime;

		uint64_t m_frameIndex2;

		std::shared_ptr<FrameRender> m_FrameRender;

		IDRScheduler m_scheduler;
	};

