#include <ScreenGrab.h>
#include "VideoEncoder.h"

void VideoEncoder::SaveDebugOutput(std::shared_ptr<CD3DRender> m_pD3DRender, std::vector<std::vector<uint8_t>> &vPacket, ID3D11Texture2D *texture, uint64_t frameIndex) {
	if (vPacket.size() == 0) {
		return;
	}
	if (vPacket[0].size() < 10) {
		return;
	}
	int type = vPacket[0][4] & 0x1F;
	if (type == 7) {
		// SPS, PPS, IDR
		char filename[1000];
		wchar_t filename2[1000];
		snprintf(filename, sizeof(filename), "%s\\%llu.h264", Settings::Instance().m_DebugOutputDir.c_str(), frameIndex);
		_snwprintf_s(filename2, sizeof(filename2), L"%hs\\%llu.dds", Settings::Instance().m_DebugOutputDir.c_str(), frameIndex);
		FILE *fp;
		fopen_s(&fp, filename, "wb");
		if (fp) {
			for (auto packet : vPacket) {
				fwrite(&packet[0], packet.size(), 1, fp);
			}
			fclose(fp);
		}
		DirectX::SaveDDSTextureToFile(m_pD3DRender->GetContext(), texture, filename2);
	}
}