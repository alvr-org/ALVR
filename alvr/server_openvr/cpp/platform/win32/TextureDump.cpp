#include "TextureDump.h"

#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"

#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <direct.h>
#include <vector>
#include <wincodec.h>
#include <wrl.h>

#pragma comment(lib, "windowscodecs.lib")

using Microsoft::WRL::ComPtr;

namespace {

// Half float decode for the HDR composition format.
float HalfToFloat(uint16_t h) {
    uint32_t sign = (uint32_t)(h >> 15) & 0x1;
    uint32_t exponent = (uint32_t)(h >> 10) & 0x1F;
    uint32_t mantissa = (uint32_t)h & 0x3FF;

    uint32_t bits;
    if (exponent == 0) {
        if (mantissa == 0) {
            bits = sign << 31;
        } else {
            // Subnormal: renormalize.
            exponent = 127 - 15 + 1;
            while ((mantissa & 0x400) == 0) {
                mantissa <<= 1;
                exponent--;
            }
            mantissa &= 0x3FF;
            bits = (sign << 31) | (exponent << 23) | (mantissa << 13);
        }
    } else if (exponent == 31) {
        bits = (sign << 31) | (0xFF << 23) | (mantissa << 13);
    } else {
        bits = (sign << 31) | ((exponent - 15 + 127) << 23) | (mantissa << 13);
    }

    float out;
    memcpy(&out, &bits, sizeof(out));
    return out;
}

uint8_t ToByte(float v) {
    if (v <= 0.f) {
        return 0;
    }
    if (v >= 1.f) {
        return 255;
    }
    return (uint8_t)(v * 255.f + 0.5f);
}

/// Maps typeless/sRGB variants onto the plain format so the readback switch stays small.
DXGI_FORMAT NormalizeFormat(DXGI_FORMAT format) {
    switch (format) {
    case DXGI_FORMAT_R8G8B8A8_TYPELESS:
    case DXGI_FORMAT_R8G8B8A8_UNORM_SRGB:
        return DXGI_FORMAT_R8G8B8A8_UNORM;
    case DXGI_FORMAT_B8G8R8A8_TYPELESS:
    case DXGI_FORMAT_B8G8R8A8_UNORM_SRGB:
        return DXGI_FORMAT_B8G8R8A8_UNORM;
    case DXGI_FORMAT_R16G16B16A16_TYPELESS:
        return DXGI_FORMAT_R16G16B16A16_FLOAT;
    default:
        return format;
    }
}

}

namespace texture_dump {

std::string DumpDir() {
    std::string dir = Settings_Instance()->m_captureFrameDir;
    if (dir.empty()) {
        const char* tmp = getenv("TEMP");
        dir = tmp ? std::string(tmp) : std::string(".");
    }
    dir += "/alvr_alpha_debug";
    _mkdir(dir.c_str());
    return dir;
}

bool ShouldDump(int slot, double intervalSeconds) {
    static const int kSlots = 8;
    static std::chrono::steady_clock::time_point last[kSlots];
    static bool initialized[kSlots] = {};

    if (slot < 0 || slot >= kSlots) {
        return false;
    }

    auto now = std::chrono::steady_clock::now();
    if (!initialized[slot]) {
        initialized[slot] = true;
        last[slot] = now;
        return true;
    }

    std::chrono::duration<double> elapsed = now - last[slot];
    if (elapsed.count() >= intervalSeconds) {
        last[slot] = now;
        return true;
    }
    return false;
}

bool SaveTextureToPng(
    ID3D11Device* device,
    ID3D11DeviceContext* context,
    ID3D11Texture2D* texture,
    const std::string& path,
    bool writeAlphaAsColor
) {
    if (!device || !context || !texture) {
        return false;
    }

    D3D11_TEXTURE2D_DESC desc;
    texture->GetDesc(&desc);

    DXGI_FORMAT normalized = NormalizeFormat(desc.Format);
    if (normalized != DXGI_FORMAT_R8G8B8A8_UNORM && normalized != DXGI_FORMAT_B8G8R8A8_UNORM
        && normalized != DXGI_FORMAT_R16G16B16A16_FLOAT) {
        LogPeriodically(
            "TextureDump",
            "Skipping dump: unsupported texture format (likely NV12/P010 from the HDR path)."
        );
        return false;
    }

    // A staging copy is required because the source is GPU resident and not CPU readable.
    D3D11_TEXTURE2D_DESC stagingDesc = desc;
    stagingDesc.Usage = D3D11_USAGE_STAGING;
    stagingDesc.BindFlags = 0;
    stagingDesc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    stagingDesc.MiscFlags = 0;
    stagingDesc.MipLevels = 1;
    stagingDesc.ArraySize = 1;
    stagingDesc.SampleDesc.Count = 1;
    stagingDesc.SampleDesc.Quality = 0;

    ComPtr<ID3D11Texture2D> staging;
    if (FAILED(device->CreateTexture2D(&stagingDesc, nullptr, &staging))) {
        Error("TextureDump: failed to create staging texture\n");
        return false;
    }

    context->CopyResource(staging.Get(), texture);

    D3D11_MAPPED_SUBRESOURCE mapped;
    if (FAILED(context->Map(staging.Get(), 0, D3D11_MAP_READ, 0, &mapped))) {
        Error("TextureDump: failed to map staging texture\n");
        return false;
    }

    // Convert to straight 8 bit BGRA, which is what the WIC encoder is configured for below.
    std::vector<uint8_t> pixels((size_t)desc.Width * desc.Height * 4);
    for (uint32_t y = 0; y < desc.Height; y++) {
        const uint8_t* srcRow = (const uint8_t*)mapped.pData + (size_t)y * mapped.RowPitch;
        uint8_t* dstRow = pixels.data() + (size_t)y * desc.Width * 4;

        for (uint32_t x = 0; x < desc.Width; x++) {
            float r, g, b, a;

            if (normalized == DXGI_FORMAT_R16G16B16A16_FLOAT) {
                const uint16_t* src = (const uint16_t*)(srcRow + (size_t)x * 8);
                r = HalfToFloat(src[0]);
                g = HalfToFloat(src[1]);
                b = HalfToFloat(src[2]);
                a = HalfToFloat(src[3]);
            } else {
                const uint8_t* src = srcRow + (size_t)x * 4;
                if (normalized == DXGI_FORMAT_B8G8R8A8_UNORM) {
                    b = src[0] / 255.f;
                    g = src[1] / 255.f;
                    r = src[2] / 255.f;
                } else {
                    r = src[0] / 255.f;
                    g = src[1] / 255.f;
                    b = src[2] / 255.f;
                }
                a = src[3] / 255.f;
            }

            uint8_t* dst = dstRow + (size_t)x * 4;
            if (writeAlphaAsColor) {
                uint8_t v = ToByte(a);
                dst[0] = v; // B
                dst[1] = v; // G
                dst[2] = v; // R
                dst[3] = 255; // opaque, so the PNG is actually visible
            } else {
                dst[0] = ToByte(b);
                dst[1] = ToByte(g);
                dst[2] = ToByte(r);
                dst[3] = ToByte(a);
            }
        }
    }

    context->Unmap(staging.Get(), 0);

    // WIC needs COM. The encoder thread may not have initialized it, so do it here and tolerate
    // an existing apartment.
    HRESULT initHr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    bool shouldUninit = SUCCEEDED(initHr) || initHr == S_FALSE;

    bool ok = false;
    {
        ComPtr<IWICImagingFactory> factory;
        HRESULT hr = CoCreateInstance(
            CLSID_WICImagingFactory, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&factory)
        );
        if (SUCCEEDED(hr)) {
            std::wstring widePath(path.begin(), path.end());

            ComPtr<IWICStream> stream;
            ComPtr<IWICBitmapEncoder> encoder;
            ComPtr<IWICBitmapFrameEncode> frame;

            hr = factory->CreateStream(&stream);
            if (SUCCEEDED(hr)) {
                hr = stream->InitializeFromFilename(widePath.c_str(), GENERIC_WRITE);
            }
            if (SUCCEEDED(hr)) {
                hr = factory->CreateEncoder(GUID_ContainerFormatPng, nullptr, &encoder);
            }
            if (SUCCEEDED(hr)) {
                hr = encoder->Initialize(stream.Get(), WICBitmapEncoderNoCache);
            }
            if (SUCCEEDED(hr)) {
                hr = encoder->CreateNewFrame(&frame, nullptr);
            }
            if (SUCCEEDED(hr)) {
                hr = frame->Initialize(nullptr);
            }
            if (SUCCEEDED(hr)) {
                hr = frame->SetSize(desc.Width, desc.Height);
            }
            if (SUCCEEDED(hr)) {
                WICPixelFormatGUID format = GUID_WICPixelFormat32bppBGRA;
                hr = frame->SetPixelFormat(&format);
            }
            if (SUCCEEDED(hr)) {
                hr = frame->WritePixels(
                    desc.Height,
                    desc.Width * 4,
                    (UINT)pixels.size(),
                    pixels.data()
                );
            }
            if (SUCCEEDED(hr)) {
                hr = frame->Commit();
            }
            if (SUCCEEDED(hr)) {
                hr = encoder->Commit();
            }

            ok = SUCCEEDED(hr);
            if (!ok) {
                Error("TextureDump: WIC encode failed for %s (hr=%p)\n", path.c_str(), hr);
            }
        } else {
            Error("TextureDump: failed to create WIC factory\n");
        }
    }

    if (shouldUninit) {
        CoUninitialize();
    }

    return ok;
}

}
