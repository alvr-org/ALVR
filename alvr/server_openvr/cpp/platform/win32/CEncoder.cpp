#include "CEncoder.h"

CEncoder::CEncoder()
    : m_bExiting(false)
    , m_targetTimestampNs(0) {
    m_encodeFinished.Set();
}

CEncoder::~CEncoder() {
    if (m_videoEncoder) {
        m_videoEncoder->Shutdown();
        m_videoEncoder.reset();
    }
    if (m_alphaVideoEncoder) {
        m_alphaVideoEncoder->Shutdown();
        m_alphaVideoEncoder.reset();
    }
}

std::shared_ptr<VideoEncoder> CEncoder::CreateAlphaEncoder(
    std::shared_ptr<CD3DRender> d3dRender, uint32_t width, uint32_t height
) {
    auto tryCreate = [&](const char* name,
                         std::function<std::shared_ptr<VideoEncoder>()> make
                     ) -> std::shared_ptr<VideoEncoder> {
        try {
            Debug("Try to use %s for the alpha stream.\n", name);
            auto encoder = make();
            encoder->SetAlphaStream(true);
            encoder->Initialize();
            return encoder;
        } catch (Exception e) {
            Debug("Alpha stream %s unavailable: %s\n", name, e.what());
            return nullptr;
        }
    };

    if (auto encoder = tryCreate("VideoEncoderAMF", [&] {
            return std::make_shared<VideoEncoderAMF>(d3dRender, width, height);
        })) {
        return encoder;
    }
    if (auto encoder = tryCreate("VideoEncoderNVENC", [&] {
            return std::make_shared<VideoEncoderNVENC>(d3dRender, width, height);
        })) {
        return encoder;
    }
    if (auto encoder = tryCreate("VideoEncoderVPL", [&] {
            return std::make_shared<VideoEncoderVPL>(d3dRender, width, height);
        })) {
        return encoder;
    }
#ifdef ALVR_GPL
    if (auto encoder = tryCreate("VideoEncoderSW", [&] {
            return std::make_shared<VideoEncoderSW>(d3dRender, width, height);
        })) {
        return encoder;
    }
#endif

    Error("Failed to create an encoder for the alpha stream. Alpha will not be transmitted.\n");
    return nullptr;
}

void CEncoder::Initialize(std::shared_ptr<CD3DRender> d3dRender) {
    m_FrameRender = std::make_shared<FrameRender>(d3dRender);
    m_FrameRender->Startup();
    uint32_t encoderWidth, encoderHeight;
    m_FrameRender->GetEncodingResolution(&encoderWidth, &encoderHeight);

    if (Settings_Instance()->m_enableAlphaStream) {
        m_alphaVideoEncoder = CreateAlphaEncoder(d3dRender, encoderWidth, encoderHeight);
    }

    Exception vplException;
    Exception vceException;
    Exception nvencException;
#ifdef ALVR_GPL
    Exception swException;

    if (Settings_Instance()->m_forceSwEncoding) {
        try {
            Debug("Try to use VideoEncoderSW.\n");
            m_videoEncoder
                = std::make_shared<VideoEncoderSW>(d3dRender, encoderWidth, encoderHeight);
            m_videoEncoder->Initialize();
            return;
        } catch (Exception e) {
            swException = e;
        }
    }
#endif

    try {
        Debug("Try to use VideoEncoderAMF.\n");
        m_videoEncoder = std::make_shared<VideoEncoderAMF>(d3dRender, encoderWidth, encoderHeight);
        m_videoEncoder->Initialize();
        return;
    } catch (Exception e) {
        vceException = e;
    }
    try {
        Debug("Try to use VideoEncoderNVENC.\n");
        m_videoEncoder
            = std::make_shared<VideoEncoderNVENC>(d3dRender, encoderWidth, encoderHeight);
        m_videoEncoder->Initialize();
        return;
    } catch (Exception e) {
        nvencException = e;
    }
    try {
        Debug("Try to use VideoEncoderVPL.\n");
        m_videoEncoder = std::make_shared<VideoEncoderVPL>(d3dRender, encoderWidth, encoderHeight);
        m_videoEncoder->Initialize();
        return;
    } catch (Exception e) {
        vplException = e;
    }
#ifdef ALVR_GPL
    try {
        Debug("Try to use VideoEncoderSW.\n");
        m_videoEncoder = std::make_shared<VideoEncoderSW>(d3dRender, encoderWidth, encoderHeight);
        m_videoEncoder->Initialize();
        return;
    } catch (Exception e) {
        swException = e;
    }
    throw MakeException(
        "All VideoEncoder are not available. VCE: %s, NVENC: %s, VPL: %s, SW: %s",
        vceException.what(),
        nvencException.what(),
        vplException.what(),
        swException.what()
    );
#else
    throw MakeException(
        "All VideoEncoder are not available. VCE: %s, NVENC: %s, VPL: %s",
        vceException.what(),
        nvencException.what(),
        vplException.what()
    );
#endif
}

void CEncoder::SetViewParams(
    vr::HmdRect2_t projLeft,
    vr::HmdMatrix34_t eyeToHeadLeft,
    vr::HmdRect2_t projRight,
    vr::HmdMatrix34_t eyeToHeadRight
) {
    m_FrameRender->SetViewParams(projLeft, eyeToHeadLeft, projRight, eyeToHeadRight);
}

bool CEncoder::CopyToStaging(
    ID3D11Texture2D* pTexture[][2],
    vr::VRTextureBounds_t bounds[][2],
    vr::HmdMatrix34_t poses[],
    int layerCount,
    bool recentering,
    uint64_t presentationTime,
    uint64_t targetTimestampNs,
    const std::string& message,
    const std::string& debugText
) {
    m_presentationTime = presentationTime;
    m_targetTimestampNs = targetTimestampNs;
    m_FrameRender->Startup();

    m_FrameRender->RenderFrame(
        pTexture, bounds, poses, layerCount, recentering, message, debugText
    );
    return true;
}

void CEncoder::Run() {
    Debug("CEncoder: Start thread. Id=%d\n", GetCurrentThreadId());
    SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_MOST_URGENT);

    while (!m_bExiting) {
        m_newFrameReady.Wait();
        if (m_bExiting)
            break;

        if (m_FrameRender->GetTexture()) {
            // Sampled once so both streams agree: the client pairs frames by timestamp, and a
            // mismatched IDR decision would leave the alpha decoder unable to recover in step.
            bool insertIDR = m_scheduler.CheckIDRInsertion();

            m_videoEncoder->Transmit(
                m_FrameRender->GetTexture().Get(),
                m_presentationTime,
                m_targetTimestampNs,
                insertIDR
            );

            if (m_alphaVideoEncoder && m_FrameRender->GetAlphaTexture()) {
                m_alphaVideoEncoder->Transmit(
                    m_FrameRender->GetAlphaTexture().Get(),
                    m_presentationTime,
                    m_targetTimestampNs,
                    insertIDR
                );
            }
        }

        m_encodeFinished.Set();
    }
}

void CEncoder::Stop() {
    m_bExiting = true;
    m_newFrameReady.Set();
    Join();
    m_FrameRender.reset();
}

void CEncoder::NewFrameReady() {
    m_encodeFinished.Reset();
    m_newFrameReady.Set();
}

void CEncoder::WaitForEncode() { m_encodeFinished.Wait(); }

void CEncoder::OnStreamStart() { m_scheduler.OnStreamStart(); }

void CEncoder::InsertIDR() { m_scheduler.InsertIDR(); }

void CEncoder::CaptureFrame() { }
