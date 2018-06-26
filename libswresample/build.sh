# build libswresample on linux mingw64
./configure --arch=x86_64 --target-os=mingw64 --cross-prefix=x86_64-w64-mingw32- --disable-avdevice  --disable-avcodec --disable-avformat --disable-swscale --disable-postproc --disable-avfilter --enable-shared --disable-static
