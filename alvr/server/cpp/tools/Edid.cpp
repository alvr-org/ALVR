#include "Edid.h"

#include <iostream>
#include <numeric>

static unsigned char letter(const char c) 
{
	return c + 1 - 'A';
}

EDID::EDID(const char vendor[3])
{
	buffer.fill(0);
	// fixed header 00 FF FF FF FF FF FF 00
	for (std::size_t i = 1; i < 7; ++i)
		buffer[i]  = 0xFF;

	uint16_t vendor_edid =
		letter(vendor[0]) << 10
		| letter(vendor[1]) << 5
		| letter(vendor[2]);
	buffer[9] = vendor_edid;
	buffer[8] = vendor_edid >> 8;
	buffer[10] = 1;
	buffer[11] = 0;

	buffer[18] = 1; // EDID version
	buffer[19] = 3; // EDID revision

	buffer[20] = 0b10000000; // digital 8bit per pixel
	buffer[23] = 0b01111000; // gamma
	buffer[24] = 0b11100010; // features (dpms, preferred mode)

	std::fill(buffer.begin() + 38, buffer.begin() + 54, 1);
	for (int i = 0 ; i < 4 ; ++i)
		buffer[57 + 18*i] = 0x10; // dummy descriptor

	checksum();
}

void EDID::checksum()
{
	buffer[127] = -std::accumulate(buffer.begin(), buffer.end() - 1, (unsigned char)0);
}

uint8_t pack_msb(uint16_t bit7_4, uint16_t bit3_0)
{
	uint8_t msb_7_4 = bit7_4 >> 8;
	uint8_t msb_3_0 = bit3_0 >> 8;
	return (msb_7_4 << 4) | msb_3_0;
}

void EDID::add_mode(uint16_t xres, uint16_t yres, int freq)
{
	uint8_t * descriptor = buffer.data() + 54 + descriptors * 18;

	const uint8_t hfp = 8;
	const uint8_t hsync = 32;

	const uint8_t vfp = 50;
	const uint8_t vsync = 2;

	const uint8_t hbp = 32 - (xres + hfp + hsync) % 32;
	const uint8_t vbp = 32 - (yres + vfp + vsync) % 32;

	const uint8_t hblank= hfp + hsync + hbp;
	const uint16_t vblank= vfp + vsync + vbp;

	uint16_t clock = freq * (xres + hblank) * (yres + vblank) / 10000;

	descriptor[0] = clock;
	descriptor[1] = clock >> 8;
	descriptor[2] = xres;
	descriptor[3] = hblank;
	descriptor[4] = pack_msb(xres, hblank);
	descriptor[5] = yres;
	descriptor[6] = vblank & 0xFF;
	descriptor[7] = pack_msb(yres, vblank);
	descriptor[8] = hfp;
	descriptor[9] = hsync;
	descriptor[10] = (vfp & 0b1111) << 4 | (vsync & 0b1111);
	descriptor[11] = (hfp >> 8 &0b11) << 6
		| (hsync >> 8 & 0b11) << 4
		| (vfp >> 4 & 0b11) << 2
		| (vsync >> 4 & 0b11);


	descriptor[12] = 119; // horizontal size in mm
	descriptor[13] = 66; // horizontal size in mm

	descriptor[17] = 0b00011010;

	++descriptors;

	uint8_t display_begin = 54 + descriptors * 18;
	buffer[display_begin+3] = 0xFC;
	const char name[14] = "ALVR display\n";
	std::copy(name, name + 14, buffer.begin() + display_begin + 5);

	++descriptors;

	checksum();
}
