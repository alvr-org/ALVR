#include <windows.h>
#include <mmsystem.h>
#include <stdio.h>
#include <tchar.h>

#pragma comment(lib, "winmm.lib")

/*
 * some good values for block size and count
 */
#define BLOCK_SIZE 8192
#define BLOCK_COUNT 20

 /*
  * function prototypes
  */
static void CALLBACK waveOutProc(HWAVEOUT, UINT, DWORD_PTR, DWORD_PTR, DWORD_PTR);
static WAVEHDR* allocateBlocks(int size, int count);
static void freeBlocks(WAVEHDR* blockArray);
static void writeAudio(HWAVEOUT hWaveOut, LPSTR data, int size);


/*
 * module level variables
 */
static CRITICAL_SECTION waveCriticalSection;
static WAVEHDR* waveBlocks;
static volatile int waveFreeBlockCount;
static int waveCurrentBlock;


int main(int argc, char* argv[])
{
	HWAVEOUT hWaveOut; /* device handle */
	HANDLE hFile;/* file handle */
	WAVEFORMATEX wfx; /* look this up in your documentation */
	char buffer[1024]; /* intermediate buffer for reading */
	int i;


	/*
	 * quick argument check
	 */
	if (argc != 2) {
		fprintf(stderr, "usage: %s <filename>\n", argv[0]);
	//	ExitProcess(1);
	}
	   	

	/*
	 * initialise the module variables
	 */
	waveBlocks = allocateBlocks(BLOCK_SIZE, BLOCK_COUNT);
	waveFreeBlockCount = BLOCK_COUNT;
	waveCurrentBlock = 0;
	InitializeCriticalSection(&waveCriticalSection);
	/*
	 * try and open the file
	 */
	if ((hFile = CreateFile(
		_T("C:\\tmp\\mic.raw"),
		GENERIC_READ,
		FILE_SHARE_READ,
		NULL,
		OPEN_EXISTING,
		0,
		NULL
	)) == INVALID_HANDLE_VALUE) {
		fprintf(stderr, "%s: unable to open file '%s'\n", argv[0], argv[1]);
		ExitProcess(1);
	}
		

	/*
	 * set up the WAVEFORMATEX structure.
	 */
	wfx.nSamplesPerSec = 48000; /* sample rate */
	wfx.wBitsPerSample = 16; /* sample size */
	wfx.nChannels = 1; /* channels*/
	wfx.cbSize = 0; /* size of _extra_ info */
	wfx.wFormatTag = WAVE_FORMAT_PCM;
	wfx.nBlockAlign = (wfx.wBitsPerSample * wfx.nChannels) >> 3;
	wfx.nAvgBytesPerSec = wfx.nBlockAlign * wfx.nSamplesPerSec;
	/*
	 * try to open the default wave device. WAVE_MAPPER is
	 * a constant defined in mmsystem.h, it always points to the
	 * default wave device on the system (some people have 2 or
	 * more sound cards).
	 */

	printf("address %i \n", &waveFreeBlockCount);

	if (waveOutOpen(
		&hWaveOut,
		WAVE_MAPPER,
		&wfx,
		(DWORD_PTR)waveOutProc,
		(DWORD_PTR)&waveFreeBlockCount,
		CALLBACK_FUNCTION
	) != MMSYSERR_NOERROR) {
		fprintf(stderr, "%s: unable to open wave mapper device\n", argv[0]);
		ExitProcess(1);
	}

	printf("Starting playback\n");

	/*
	 * playback loop
	 */
	while (1) {
		DWORD readBytes;
		if (!ReadFile(hFile, buffer, sizeof(buffer), &readBytes, NULL))
			break;
		if (readBytes == 0)
			break;
		if (readBytes < sizeof(buffer)) {
			printf("at end of buffer\n");
			memset(buffer + readBytes, 0, sizeof(buffer) - readBytes);
			printf("after memcpy\n");
		}

		printf("write audio: %i \n", sizeof(buffer));
		writeAudio(hWaveOut, buffer, sizeof(buffer));
	}
	/*
	 * wait for all blocks to complete
	 */
	while (waveFreeBlockCount < BLOCK_COUNT)
		Sleep(10);
	/*
	 * unprepare any blocks that are still prepared
	 */
	for (i = 0; i < waveFreeBlockCount; i++)
		if (waveBlocks[i].dwFlags & WHDR_PREPARED)
			waveOutUnprepareHeader(hWaveOut, &waveBlocks[i], sizeof(WAVEHDR));


	DeleteCriticalSection(&waveCriticalSection);
	freeBlocks(waveBlocks);
	waveOutClose(hWaveOut);
	CloseHandle(hFile);
	return 0;
}

static void CALLBACK waveOutProc(
	HWAVEOUT hWaveOut,
	UINT uMsg,
	DWORD_PTR  dwInstance,
	DWORD_PTR  dwParam1,
	DWORD_PTR  dwParam2
)
{
	/*
	 * pointer to free block counter
	 */
	int* freeBlockCounter = (int*)dwInstance;

	


	/*
	 * ignore calls that occur due to openining and closing the
	 * device.
	 */
	if (uMsg != WOM_DONE)
		return;


	printf("address callback %i \n", freeBlockCounter);
	   	
	EnterCriticalSection(&waveCriticalSection);
	printf("Free blocks %i \n", (*freeBlockCounter));
	(*freeBlockCounter)++;
	printf("Free blocks2 %i \n", (*freeBlockCounter));
	LeaveCriticalSection(&waveCriticalSection);
	
}

WAVEHDR* allocateBlocks(int size, int count)
{
	LPSTR buffer;
	int i;
	WAVEHDR* blocks;
	DWORD totalBufferSize = (size + sizeof(WAVEHDR)) * count;

	/*
	 * allocate memory for the entire set in one go
	 */
	if ((buffer = (LPSTR)HeapAlloc(
		GetProcessHeap(),
		HEAP_ZERO_MEMORY,
		totalBufferSize
	)) == NULL) {
		fprintf(stderr, "Memory allocation error\n");
		ExitProcess(1);
	}
	/*
	 * and set up the pointers to each bit
	 */
	blocks = (WAVEHDR*)buffer;
	buffer += sizeof(WAVEHDR) * count;
	for (i = 0; i < count; i++) {
		blocks[i].dwBufferLength = size;
		blocks[i].lpData = buffer;
		buffer += size;
	}
	return blocks;
}
void freeBlocks(WAVEHDR* blockArray)
{
	/*
	 * and this is why allocateBlocks works the way it does
	 */
	HeapFree(GetProcessHeap(), 0, blockArray);
}

void writeAudio(HWAVEOUT hWaveOut, LPSTR data, int size)
{
	WAVEHDR* current;
	int remain;
	current = &waveBlocks[waveCurrentBlock];
	printf("using current block %i \n", waveCurrentBlock);

	while (size > 0) {
		/*
		 * first make sure the header we're going to use is unprepared
		 */
		if (current->dwFlags & WHDR_PREPARED)
			waveOutUnprepareHeader(hWaveOut, current, sizeof(WAVEHDR));
		if (size < (int)(BLOCK_SIZE - current->dwUser)) {
			memcpy(current->lpData + current->dwUser, data, size);
			current->dwUser += size;
			break;
		}
		remain = BLOCK_SIZE - current->dwUser;
		memcpy(current->lpData + current->dwUser, data, remain);
		size -= remain;
		data += remain;
		current->dwBufferLength = BLOCK_SIZE;
		waveOutPrepareHeader(hWaveOut, current, sizeof(WAVEHDR));

		printf("Playing block %i, free blocks: %i\n", waveCurrentBlock, waveFreeBlockCount);
		waveOutWrite(hWaveOut, current, sizeof(WAVEHDR));

		EnterCriticalSection(&waveCriticalSection);
		waveFreeBlockCount--;
		LeaveCriticalSection(&waveCriticalSection);
		/*
		 * wait for a block to become free
		 */
		while (!waveFreeBlockCount)
			Sleep(10);
		/*
		 * point to the next block
		 */
		waveCurrentBlock++;
		waveCurrentBlock %= BLOCK_COUNT;
		current = &waveBlocks[waveCurrentBlock];
		current->dwUser = 0;

		printf("current block now %i \n", waveCurrentBlock);

	}
}
