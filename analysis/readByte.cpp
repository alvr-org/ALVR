#include <stdio.h>
#include <stdlib.h>
#define STB_IMAGE_IMPLEMENTATION
#include "stb_image.h"
#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "stb_image_write.h"
#include <fstream>
#include <iostream>

void DisplayTextureAsPNG(const char* filename)
{
    // Open byte file
    std::ifstream file(filename, std::ios::in | std::ios::binary);
    if (!file.is_open())
    {
        printf("Failed to open byte file\n");
        return;
    }

    // Get file size
    file.seekg(0, std::ios::end);
    size_t fileSize = file.tellg();
    file.seekg(0, std::ios::beg);

    // Read file data
    unsigned char* data = new unsigned char[fileSize];
    file.read((char*)data, fileSize);

    // Close byte file
    file.close();

    // Get texture dimensions
    int width = 1184*2; // Replace with actual width
    int height = 1120; // Replace with actual height

    // int width = fileSize / 3;
    // int height = 1;

    std::cout << width << std::endl;


    // Display texture as PNG using stb_image
    stbi_write_png("texture.png", width, height, 4, data, width * 4);

    // Free memory
    delete[] data;
}

int main()
{
    DisplayTextureAsPNG("1492.bytes");
    return 0;
}