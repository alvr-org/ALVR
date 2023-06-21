CC = g++
CFLAGS = -I/usr/local/include
LDFLAGS = -L/usr/local/Cellar/ffmpeg/5.1.2_5/lib -L/usr/local/Cellar/libpng/1.6.39/lib
LDLIBS = -lavformat -lavcodec -lavutil -lswscale -lpng
SRCS = readBin.cpp readByte.cpp
OBJS = $(SRCS:.cpp=.o)

all: readBin.exe readByte.exe

readBin.exe: readBin.o
	$(CC) $(LDFLAGS) -o $@ $^ $(LDLIBS)

readByte.exe: readByte.o
	$(CC) $(LDFLAGS) -o $@ $^ $(LDLIBS)

%.o: %.cpp
	$(CC) $(CFLAGS) -c -o $@ $<

clean:
	rm -f $(OBJS) readBin.exe readByte.exe