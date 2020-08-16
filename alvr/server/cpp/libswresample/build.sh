# You need Ubuntu 16.04 to build ffmpeg libswresample
# sudo apt install mingw-w64
git clone https://git.ffmpeg.org/ffmpeg.git
cd ffmpeg
git checkout n4.0.1
# build libswresample on linux mingw64
./configure --arch=x86_64 --target-os=mingw64 --cross-prefix=x86_64-w64-mingw32- --disable-avdevice --disable-avcodec --disable-avformat --disable-swscale --disable-postproc --disable-avfilter --enable-shared --disable-static
make -j4
make install DESTDIR="$PWD/installdirs"
