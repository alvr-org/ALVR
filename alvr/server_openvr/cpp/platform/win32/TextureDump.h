#pragma once

#include <d3d11.h>
#include <string>

namespace texture_dump {

/// Saves a D3D11 texture to a PNG file for debugging.
///
/// Handles the RGBA and half-float formats used by the frame composition chain by staging a CPU
/// readable copy first. Returns false and logs on unsupported formats (e.g. the NV12/P010 output
/// of the HDR path) instead of throwing, since this is diagnostic-only code.
///
/// `writeAlphaAsColor` replicates the alpha channel into RGB and forces alpha opaque, so the
/// resulting PNG shows the alpha channel as a visible greyscale image rather than an invisible one.
bool SaveTextureToPng(
    ID3D11Device* device,
    ID3D11DeviceContext* context,
    ID3D11Texture2D* texture,
    const std::string& path,
    bool writeAlphaAsColor = false
);

/// True roughly once per `intervalSeconds` for the given counter slot, used to rate limit dumps.
/// `slot` must be a distinct small integer per call site.
bool ShouldDump(int slot, double intervalSeconds = 1.0);

/// Directory for debug dumps, taken from the capture frame dir setting, or the temp dir when unset.
/// The directory is created if missing.
std::string DumpDir();

}
