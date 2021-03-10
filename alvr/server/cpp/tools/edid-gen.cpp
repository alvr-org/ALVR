#include "Edid.h"
#include <iostream>

int main(int argc, char ** argv)
{
	if (argc != 4)
	{
		std::cerr << "usage: " << argv[0] << " width height refresh_rate(Hz)" << std::endl;
		return 1;
	}
	EDID edid("OVR");
	edid.add_mode(std::atoi(argv[1]), std::atoi(argv[2]), std::atoi(argv[3]));
	std::cout.write((char *)edid.data(), edid.size());
}
