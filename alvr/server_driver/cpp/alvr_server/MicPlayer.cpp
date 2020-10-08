#include "MicPlayer.h"


static CRITICAL_SECTION waveCriticalSection;
static int waveFreeBlockCount;

void CALLBACK waveOutProc(
	HWAVEOUT hWaveOut,
	UINT uMsg,
	DWORD_PTR  dwInstance,
	DWORD_PTR  dwParam1,
	DWORD_PTR  dwParam2
)
{
	/*
	 * ignore calls that occur due to openining and closing the
	 * device.
	 */
	if (uMsg != WOM_DONE)
		return;

	((MicPlayer*)dwInstance)->waveCallback();

}


void MicPlayer::waveCallback() {
	EnterCriticalSection(&waveCriticalSection);
	waveFreeBlockCount++;
	LeaveCriticalSection(&waveCriticalSection);
}

UINT MicPlayer::getMicHWID() {	

	UINT devs = waveOutGetNumDevs();
	std::wstring micDevId = ToWstring(Settings::Instance().m_microphoneDevice);

	// Include terminating NULL, since the waveOutMessage calls will as well.
	size_t micDevIdSize = (micDevId.length() + 1) * sizeof(WCHAR);

	WCHAR *pwstrDeviceId = (WCHAR *)CoTaskMemAlloc(micDevIdSize);
	if (NULL == pwstrDeviceId) {
		Error("Failed to allocate space for Device ID string.");
		return -1;
	}
	
	// We get a Windows audio endpoint ID from settings. Find which device
	// handle has the same id.
	for (UINT dev = 0; dev < devs; dev++) {
		size_t cbDeviceId = 0;
		MMRESULT mmr = waveOutMessage((HWAVEOUT)dev, DRV_QUERYFUNCTIONINSTANCEIDSIZE, (DWORD_PTR)&cbDeviceId, NULL);

		if (MMSYSERR_NOERROR != mmr) {
			Warn("waveOutMessage (DRV_QUERYFUNCTIONINSTANCEIDSIZE) failed. mmr = 0x%08x", mmr);
			continue;
		}

		if (cbDeviceId != micDevIdSize) {
			Warn("Audio device ID has wrong length: %lld != %lld", cbDeviceId, micDevIdSize);
			continue;
		}

		mmr = waveOutMessage((HWAVEOUT)dev, DRV_QUERYFUNCTIONINSTANCEID, (DWORD_PTR)pwstrDeviceId, cbDeviceId);

		if (MMSYSERR_NOERROR != mmr) {
			Warn("waveOutMessage (DRV_QUERYFUNCTIONINSTANCEID) failed. mmr = 0x%08x", mmr);
			continue;
		}

		if (lstrcmpiW(pwstrDeviceId, micDevId.c_str()) == 0) {
			Debug("Microphone device found: %u", dev);
			CoTaskMemFree(pwstrDeviceId);
			return dev;
		}
	}	

	CoTaskMemFree(pwstrDeviceId);
	return -1;
}



MicPlayer::MicPlayer()
{
		
	  
	/*
	 * initialise the module variables
	 */
	waveBlocks = allocateBlocks(BLOCK_SIZE, BLOCK_COUNT);
	waveFreeBlockCount = BLOCK_COUNT;
	
	waveCurrentBlock = 0;
	InitializeCriticalSection(&waveCriticalSection);
	


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

	deviceID = MicPlayer::getMicHWID();

	if (!Settings::Instance().m_streamMic) {
		return;
	}

	if (deviceID == -1) {
		Log("Microphone Audio device not found");
		return;
	}

	if (waveOutOpen(
		&hWaveOut,
		deviceID,
		&wfx,
		(DWORD_PTR)waveOutProc,
		(DWORD_PTR)&waveFreeBlockCount,
		CALLBACK_FUNCTION
	) != MMSYSERR_NOERROR) {
		Log("unable to open wave mapper device\n");
	}

	Log("Mic Audio device opened");
}


MicPlayer::~MicPlayer()
{
	/*
	* wait for all blocks to complete
	*/
	while (waveFreeBlockCount < BLOCK_COUNT)
		Sleep(10);
	/*
	 * unprepare any blocks that are still prepared
	 */
	for (int i = 0; i < waveFreeBlockCount; i++)
		if (waveBlocks[i].dwFlags & WHDR_PREPARED)
			waveOutUnprepareHeader(hWaveOut, &waveBlocks[i], sizeof(WAVEHDR));

	DeleteCriticalSection(&waveCriticalSection);
	freeBlocks(waveBlocks);
	waveOutClose(hWaveOut);

}


WAVEHDR* MicPlayer::allocateBlocks(int size, int count)
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
		Log("Memory allocation error\n");
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

void MicPlayer::freeBlocks(WAVEHDR* blockArray)
{
	/*
	 * and this is why allocateBlocks works the way it does
	 */
	HeapFree(GetProcessHeap(), 0, blockArray);
}


void MicPlayer::playAudio(LPSTR data, int size)
{

	if (deviceID == -1) {		
		return;
	}
		

	WAVEHDR* current;
	int remain;
	current = &waveBlocks[waveCurrentBlock];	

	while (size > 0) {
		if (!waveFreeBlockCount) {
			Log("Skipped playing mic audio: No free blocks", waveCurrentBlock);
			return;
		}

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

		//Log("Playing block %i, free blocks: %i\n", waveCurrentBlock, waveFreeBlockCount);
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
	}
}