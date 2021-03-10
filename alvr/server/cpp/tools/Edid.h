#pragma once

#include <array>
#include <cstdint>

class EDID
{
public:
	EDID(const char vendor[3]);

	void add_mode(uint16_t xres, uint16_t yres, int freq);

	const unsigned char* data() const
	{
		return buffer.data();
	}
	std::size_t size() const
	{
		return buffer.size();
	}
private:
	void checksum();
	std::array<unsigned char, 128> buffer;
	int descriptors = 0;
};
