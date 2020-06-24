#include "AudioCapture.h"

class TaskMem {
public:
	TaskMem(void *p) {
	}

	~TaskMem() {
		CoTaskMemFree(m_p);
	}
private:
	void *m_p;
};

class MMIOHandle {
public:
	MMIOHandle(HMMIO handle = NULL) : m_handle(handle) {
	}
	~MMIOHandle() {
		Close();
	}

	void Set(HMMIO handle) {
		m_handle = handle;
	}

	void Close() {
		if (m_handle != NULL) {
			mmioClose(m_handle, 0);
		}
		m_handle = NULL;
	}


	bool IsValid() {
		return m_handle != NULL;
	}
	HMMIO Get() {
		return m_handle;
	}

private:
	HMMIO m_handle;
};

class AudioClientStopOnExit {
public:
	AudioClientStopOnExit(IAudioClient *p) : m_p(p) {}
	~AudioClientStopOnExit() {
		HRESULT hr = m_p->Stop();
		if (FAILED(hr)) {
			LogDriver("IAudioClient::Stop failed: hr = 0x%08x", hr);
		}
	}

private:
	IAudioClient * m_p;
};

class AvRevertMmThreadCharacteristicsOnExit {
public:
	AvRevertMmThreadCharacteristicsOnExit(HANDLE hTask) : m_hTask(hTask) {}
	~AvRevertMmThreadCharacteristicsOnExit() {
		if (!AvRevertMmThreadCharacteristics(m_hTask)) {
			LogDriver("AvRevertMmThreadCharacteristics failed: last error is %d", GetLastError());
		}
	}
private:
	HANDLE m_hTask;
};

class CancelWaitableTimerOnExit {
public:
	CancelWaitableTimerOnExit(HANDLE h) : m_h(h) {}
	~CancelWaitableTimerOnExit() {
		if (!CancelWaitableTimer(m_h)) {
			LogDriver("CancelWaitableTimer failed: last error is %d", GetLastError());
		}
	}
private:
	HANDLE m_h;
};

//
// AudioCapture
//

AudioCapture::AudioCapture(std::shared_ptr<ClientConnection> listener)
	: m_pMMDevice(NULL)
	, m_pwfx(NULL)
	, m_startedEvent(NULL)
	, m_stopEvent(NULL)
	, m_listener(listener) {
}

AudioCapture::~AudioCapture() {
}

void AudioCapture::OpenDevice(const std::string &id) {
	CoInitialize(NULL);

	HRESULT hr = S_OK;

	// get an enumerator
	ComPtr<IMMDeviceEnumerator> pMMDeviceEnumerator;

	hr = CoCreateInstance(
		__uuidof(MMDeviceEnumerator), NULL, CLSCTX_ALL,
		__uuidof(IMMDeviceEnumerator),
		(void**)&pMMDeviceEnumerator
	);
	if (FAILED(hr)) {
		throw MakeException("CoCreateInstance(IMMDeviceEnumerator) failed: hr = 0x%08x", hr);
	}
	
	hr = pMMDeviceEnumerator->GetDevice(ToWstring(id).c_str(), &m_pMMDevice);
	if (FAILED(hr)) {
		throw MakeException("Could not find a device id %ls. hr = 0x%08x", id.c_str(), hr);
	}
}

void AudioCapture::Start(const std::string &id) {
	CoInitialize(NULL);

	OpenDevice(id);

	m_hThread.Set(CreateThread(
		NULL, 0,
		LoopbackCaptureThreadFunction, this,
		0, NULL
	));
	if (!m_hThread.IsValid()) {
		throw MakeException("CreateThread failed: last error is %u", GetLastError());
	}

	// wait for either capture to start or the thread to end
	HANDLE waitArray[2] = { m_startedEvent.Get(), m_hThread.Get() };
	DWORD waitResult = WaitForMultipleObjects(
		sizeof(waitArray) / sizeof(waitArray[0]), waitArray,
		FALSE, INFINITE
	);

	if (WAIT_OBJECT_0 + 1 == waitResult) {
		throw MakeException("Thread aborted before starting to loopback capture. message=%s", m_errorMessage.c_str());
	}

	if (WAIT_OBJECT_0 != waitResult) {
		throw MakeException("Unexpected WaitForMultipleObjects return value %u", waitResult);
	}
}

void AudioCapture::Shutdown() {
	m_stopEvent.SetEvent();
	DWORD waitResult = WaitForSingleObject(m_hThread.Get(), INFINITE);
	if (WAIT_OBJECT_0 != waitResult) {
		LogDriver("WaitForSingleObject returned unexpected result 0x%08x, last error is %d", waitResult, GetLastError());
	}

	// at this point the thread is definitely finished

	DWORD exitCode;
	if (!GetExitCodeThread(m_hThread.Get(), &exitCode)) {
		throw MakeException("GetExitCodeThread failed: last error is %u", GetLastError());
	}

	if (0 != exitCode) {
		throw MakeException("Loopback capture thread exit code is %u; expected 0", exitCode);
	}
}


DWORD WINAPI AudioCapture::LoopbackCaptureThreadFunction(LPVOID pContext) {
	AudioCapture *self = (AudioCapture*)pContext;

	HRESULT hr = CoInitialize(NULL);
	if (FAILED(hr)) {
		LogDriver("CoInitialize failed: hr = 0x%08x", hr);
		return 0;
	}

	self->CaptureRetry();

	CoUninitialize();

	return 0;
}

void AudioCapture::CaptureRetry() {
	while (true) {
		try {
			m_canRetry = false;
			LoopbackCapture();
			break;
		}
		catch (Exception e) {
			if (m_canRetry) {
				LogDriver("Exception on sound capture (Retry). message=%s", e.what());
				continue;
			}
			m_errorMessage = e.what();
			LogDriver("Exception on sound capture. message=%s", e.what());
			break;
		}
	}
}

void AudioCapture::LoopbackCapture() {
	HRESULT hr;

	// activate an IAudioClient
	ComPtr<IAudioClient> pAudioClient;
	hr = m_pMMDevice->Activate(
		__uuidof(IAudioClient),
		CLSCTX_ALL, NULL,
		(void**)&pAudioClient
	);
	if (FAILED(hr)) {
		throw MakeException("IMMDevice::Activate(IAudioClient) failed: hr = 0x%08x", hr);
	}

	// get the default device periodicity
	REFERENCE_TIME hnsDefaultDevicePeriod;
	hr = pAudioClient->GetDevicePeriod(&hnsDefaultDevicePeriod, NULL);
	if (FAILED(hr)) {
		throw MakeException("IAudioClient::GetDevicePeriod failed: hr = 0x%08x", hr);
	}

	// get the default device format
	WAVEFORMATEX *pwfx;
	hr = pAudioClient->GetMixFormat(&pwfx);
	if (FAILED(hr)) {
		throw MakeException("IAudioClient::GetMixFormat failed: hr = 0x%08x", hr);
	}
	TaskMem taskmem(pwfx);

	LogDriver("MixFormat: nBlockAlign=%d wFormatTag=%d wBitsPerSample=%d nChannels=%d nSamplesPerSec=%d"
		, pwfx->nBlockAlign, pwfx->wFormatTag, pwfx->wBitsPerSample, pwfx->nChannels, pwfx->nSamplesPerSec);

	// coerce int-16 wave format
	// can do this in-place since we're not changing the size of the format
	// also, the engine will auto-convert from float to int for us
	switch (pwfx->wFormatTag) {
	case WAVE_FORMAT_IEEE_FLOAT:
		pwfx->wFormatTag = WAVE_FORMAT_PCM;
		pwfx->wBitsPerSample = 16;
		pwfx->nBlockAlign = pwfx->nChannels * pwfx->wBitsPerSample / 8;
		pwfx->nAvgBytesPerSec = pwfx->nBlockAlign * pwfx->nSamplesPerSec;
		break;

	case WAVE_FORMAT_EXTENSIBLE:
	{
		// naked scope for case-local variable
		PWAVEFORMATEXTENSIBLE pEx = reinterpret_cast<PWAVEFORMATEXTENSIBLE>(pwfx);
		LogDriver("PWAVEFORMATEXTENSIBLE: SubFormat=%d wValidBitsPerSample=%d"
			, pEx->SubFormat, pEx->Samples.wValidBitsPerSample);
		if (IsEqualGUID(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, pEx->SubFormat)) {
			pEx->SubFormat = KSDATAFORMAT_SUBTYPE_PCM;
			pEx->Samples.wValidBitsPerSample = 16;
			pwfx->wBitsPerSample = 16;
			pwfx->nBlockAlign = pwfx->nChannels * pwfx->wBitsPerSample / 8;
			pwfx->nAvgBytesPerSec = pwfx->nBlockAlign * pwfx->nSamplesPerSec;
		}
		else {
			throw MakeException("Don't know how to coerce mix format to int-16");
		}
	}
	break;

	default:
		throw MakeException("Don't know how to coerce WAVEFORMATEX with wFormatTag = 0x%08x to int-16", pwfx->wFormatTag);
	}

	MMCKINFO ckRIFF = { 0 };
	MMCKINFO ckData = { 0 };
	MMIOHandle hFile;

	// create a periodic waitable timer
	Handle wakeUp(CreateWaitableTimer(NULL, FALSE, NULL));
	if (!wakeUp.IsValid()) {
		throw MakeException("CreateWaitableTimer failed: last error = %u", GetLastError());
	}

	UINT32 nBlockAlign = pwfx->nBlockAlign;
	m_frames = 0;

	// call IAudioClient::Initialize
	// note that AUDCLNT_STREAMFLAGS_LOOPBACK and AUDCLNT_STREAMFLAGS_EVENTCALLBACK
	// do not work together...
	// the "data ready" event never gets set
	// so we're going to do a timer-driven loop
	hr = pAudioClient->Initialize(
		AUDCLNT_SHAREMODE_SHARED,
		AUDCLNT_STREAMFLAGS_LOOPBACK,
		0, 0, pwfx, 0
	);
	if (FAILED(hr)) {
		throw MakeException("IAudioClient::Initialize failed: hr = 0x%08x", hr);
	}

	std::unique_ptr<Resampler> resampler(std::make_unique<Resampler>(pwfx->nSamplesPerSec, DEFAULT_SAMPLE_RATE, pwfx->nChannels, DEFAULT_CHANNELS));

	// activate an IAudioCaptureClient
	ComPtr<IAudioCaptureClient> pAudioCaptureClient;
	hr = pAudioClient->GetService(
		__uuidof(IAudioCaptureClient),
		(void**)&pAudioCaptureClient
	);
	if (FAILED(hr)) {
		throw MakeException("IAudioClient::GetService(IAudioCaptureClient) failed: hr = 0x%08x", hr);
	}

	// register with MMCSS
	DWORD nTaskIndex = 0;
	HANDLE hTask = AvSetMmThreadCharacteristicsW(L"Audio", &nTaskIndex);
	if (NULL == hTask) {
		throw MakeException("AvSetMmThreadCharacteristics failed: last error = %u", GetLastError());
	}
	AvRevertMmThreadCharacteristicsOnExit unregisterMmcss(hTask);

	// set the waitable timer
	LARGE_INTEGER liFirstFire;
	liFirstFire.QuadPart = -hnsDefaultDevicePeriod / 2; // negative means relative time
	LONG lTimeBetweenFires = (LONG)hnsDefaultDevicePeriod / 2 / (10 * 1000); // convert to milliseconds
	BOOL bOK = SetWaitableTimer(
		wakeUp.Get(),
		&liFirstFire,
		lTimeBetweenFires,
		NULL, NULL, FALSE
	);
	if (!bOK) {
		DWORD dwErr = GetLastError();
		throw MakeException("SetWaitableTimer failed: last error = %u", dwErr);
	}
	CancelWaitableTimerOnExit cancelWakeUp(wakeUp.Get());

	// call IAudioClient::Start
	hr = pAudioClient->Start();
	if (FAILED(hr)) {
		throw MakeException("IAudioClient::Start failed: hr = 0x%08x", hr);
	}
	AudioClientStopOnExit stopAudioClient(pAudioClient.Get());

	m_startedEvent.SetEvent();

	// loopback capture loop
	HANDLE waitArray[2] = { m_stopEvent.Get(), wakeUp.Get() };

	bool bDone = false;
	bool bFirstPacket = true;
	for (UINT32 nPasses = 0; !bDone; nPasses++) {
		// drain data while it is available
		UINT32 nNextPacketSize;
		for (
			hr = pAudioCaptureClient->GetNextPacketSize(&nNextPacketSize);
			SUCCEEDED(hr) && nNextPacketSize > 0;
			hr = pAudioCaptureClient->GetNextPacketSize(&nNextPacketSize)
			) {
			// get the captured data
			BYTE *pData;
			UINT32 nNumFramesToRead;
			DWORD dwFlags;

			hr = pAudioCaptureClient->GetBuffer(
				&pData,
				&nNumFramesToRead,
				&dwFlags,
				NULL,
				NULL
			);
			if (FAILED(hr)) {
				throw MakeException("IAudioCaptureClient::GetBuffer failed on pass %u after %u frames: hr = 0x%08x", nPasses, m_frames, hr);
			}

			if (bFirstPacket && AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY == dwFlags) {
				LogDriver("Probably spurious glitch reported on first packet");
			}
			else if (0 != dwFlags) {
				LogDriver("IAudioCaptureClient::GetBuffer set flags to 0x%08x on pass %u after %u frames", dwFlags, nPasses, m_frames);
			}

			if (0 == nNumFramesToRead) {
				throw MakeException("IAudioCaptureClient::GetBuffer said to read 0 frames on pass %u after %u frames", nPasses, m_frames);
			}

			LONG lBytesToWrite = nNumFramesToRead * nBlockAlign;
			resampler->FeedInput(nNumFramesToRead, (uint8_t *)pData);

			m_listener->SendAudio(resampler->GetDest(), resampler->GetDestBufSize(), GetTimestampUs());

			m_frames += nNumFramesToRead;

			if (hFile.IsValid()) {
#pragma prefast(suppress: __WARNING_INCORRECT_ANNOTATION, "IAudioCaptureClient::GetBuffer SAL annotation implies a 1-byte buffer")
				LONG lBytesWritten = mmioWrite(hFile.Get(), reinterpret_cast<PCHAR>(pData), lBytesToWrite);
				if (lBytesToWrite != lBytesWritten) {
					LogDriver("mmioWrite wrote %u bytes on pass %u after %u frames: expected %u bytes", lBytesWritten, nPasses, m_frames, lBytesToWrite);
					hFile.Close();
				}
			}

			hr = pAudioCaptureClient->ReleaseBuffer(nNumFramesToRead);
			if (FAILED(hr)) {
				throw MakeException("IAudioCaptureClient::ReleaseBuffer failed on pass %u after %u frames: hr = 0x%08x", nPasses, m_frames, hr);
			}

			bFirstPacket = false;
		}

		if (FAILED(hr)) {
			if (hr == AUDCLNT_E_DEVICE_INVALIDATED) {
				// When configuration of the sound device was changed. (e.g. sample rate, sample format)
				// We can retry to capture.
				m_canRetry = true;
			}
			throw MakeException("IAudioCaptureClient::GetNextPacketSize failed on pass %u after %u frames: hr = 0x%08x", nPasses, m_frames, hr);
		}

		DWORD waitResult = WaitForMultipleObjects(
			sizeof(waitArray) / sizeof(waitArray[0]), waitArray,
			FALSE, INFINITE
		);

		if (WAIT_OBJECT_0 == waitResult) {
			LogDriver("Received stop event after %u passes and %u frames", nPasses, m_frames);
			bDone = true;
			continue; // exits loop
		}

		if (WAIT_OBJECT_0 + 1 != waitResult) {
			throw MakeException("Unexpected WaitForMultipleObjects return value %u on pass %u after %u frames", waitResult, nPasses, m_frames);
		}
	} // capture loop

	if (hFile.IsValid()) {
		FinishWaveFile(hFile.Get(), &ckData, &ckRIFF);
	}
}

void AudioCapture::FinishWaveFile(HMMIO hFile, MMCKINFO *pckRIFF, MMCKINFO *pckData) {
	MMRESULT result;

	result = mmioAscend(hFile, pckData, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException("mmioAscend(\"data\" failed: MMRESULT = 0x%08x", result);
	}

	result = mmioAscend(hFile, pckRIFF, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException("mmioAscend(\"RIFF/WAVE\" failed: MMRESULT = 0x%08x", result);
	}
}
