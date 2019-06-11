#include "AudioCapture.h"

class PropVariant {
public:
	PropVariant() {
		PropVariantInit(&pv);
	}

	~PropVariant() {
		HRESULT hr = PropVariantClear(&pv);
		if (FAILED(hr)) {
			Log(L"PropVariantClear failed: hr = 0x%08x", hr);
		}
	}

	PROPVARIANT &Get() {
		return pv;
	}
private:
	PROPVARIANT pv;
};

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
			Log(L"IAudioClient::Stop failed: hr = 0x%08x", hr);
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
			Log(L"AvRevertMmThreadCharacteristics failed: last error is %d", GetLastError());
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
			Log(L"CancelWaitableTimer failed: last error is %d", GetLastError());
		}
	}
private:
	HANDLE m_h;
};

//
// AudioEndPointDescriptor
//

AudioEndPointDescriptor::AudioEndPointDescriptor(const ComPtr<IMMDevice> &device, bool isDefault) {
	wchar_t *idStr;
	device->GetId(&idStr);
	TaskMem idMem(idStr);

	m_id = idStr;
	m_name = GetDeviceName(device);
	m_isDefault = isDefault;
}
std::wstring AudioEndPointDescriptor::GetName() const {
	return m_name;
}
std::wstring AudioEndPointDescriptor::GetId() const {
	return m_id;
}
bool AudioEndPointDescriptor::IsDefault() const {
	return m_isDefault;
}
bool AudioEndPointDescriptor::operator==(const AudioEndPointDescriptor& a) {
	return a.GetId() == m_id;
}
bool AudioEndPointDescriptor::operator!=(const AudioEndPointDescriptor& a) {
	return !operator==(a);
}

std::wstring AudioEndPointDescriptor::GetDeviceName(const ComPtr<IMMDevice> &pMMDevice) {
	// open the property store on that device
	ComPtr<IPropertyStore> pPropertyStore;
	HRESULT hr = pMMDevice->OpenPropertyStore(STGM_READ, &pPropertyStore);
	if (FAILED(hr)) {
		throw MakeException(L"IMMDevice::OpenPropertyStore failed: hr = 0x%08x", hr);
	}

	// get the long name property
	PropVariant pv;
	hr = pPropertyStore->GetValue(PKEY_Device_FriendlyName, &pv.Get());
	if (FAILED(hr)) {
		throw MakeException(L"IPropertyStore::GetValue failed: hr = 0x%08x", hr);
	}

	if (VT_LPWSTR != pv.Get().vt) {
		throw MakeException(L"PKEY_Device_FriendlyName variant type is %u - expected VT_LPWSTR", pv.Get().vt);
	}
	return pv.Get().pwszVal;
}

//
// AudioCapture
//

AudioCapture::AudioCapture(std::shared_ptr<Listener> listener)
	: m_pMMDevice(NULL)
	, m_pwfx(NULL)
	, m_startedEvent(NULL)
	, m_stopEvent(NULL)
	, m_listener(listener) {
}

AudioCapture::~AudioCapture() {
}

void AudioCapture::list_devices(std::vector<AudioEndPointDescriptor> &deviceList) {
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
		throw MakeException(L"CoCreateInstance(IMMDeviceEnumerator) failed: hr = 0x%08x", hr);
	}

	// TODO: ERole???
	ComPtr<IMMDevice> pDefaultMMDevice;
	hr = pMMDeviceEnumerator->GetDefaultAudioEndpoint(eRender, eConsole, &pDefaultMMDevice);
	if (hr == E_NOTFOUND) {
		Log("No default audio endpoint found. No audio device?");
		return;
	}
	if (FAILED(hr)) {
		throw MakeException(L"IMMDeviceEnumerator::GetDefaultAudioEndpoint failed: hr = 0x%08x", hr);
	}
	AudioEndPointDescriptor defaultDescriptor(pDefaultMMDevice, true);
	deviceList.push_back(defaultDescriptor);

	ComPtr<IMMDeviceCollection> pMMDeviceCollection;

	// get all the active render endpoints
	hr = pMMDeviceEnumerator->EnumAudioEndpoints(
		eRender, DEVICE_STATE_ACTIVE, &pMMDeviceCollection
	);
	if (FAILED(hr)) {
		throw MakeException(L"IMMDeviceEnumerator::EnumAudioEndpoints failed: hr = 0x%08x", hr);
	}

	UINT count;
	hr = pMMDeviceCollection->GetCount(&count);
	if (FAILED(hr)) {
		throw MakeException(L"IMMDeviceCollection::GetCount failed: hr = 0x%08x", hr);
	}
	Log(L"Active render endpoints found: %u", count);

	Log(L"DefaultDevice:%s ID:%s", defaultDescriptor.GetName().c_str(), defaultDescriptor.GetId().c_str());

	for (UINT i = 0; i < count; i++) {
		ComPtr<IMMDevice> pMMDevice;
		wchar_t *id = nullptr;

		// get the "n"th device
		hr = pMMDeviceCollection->Item(i, &pMMDevice);
		if (FAILED(hr)) {
			throw MakeException(L"IMMDeviceCollection::Item failed: hr = 0x%08x", hr);
		}
		AudioEndPointDescriptor descriptor(pMMDevice, false);
		if (descriptor == defaultDescriptor) {
			// Default is already added.
			continue;
		}
		deviceList.push_back(descriptor);

		Log(L"Device%u:%s ID:%s", i, descriptor.GetName().c_str(), descriptor.GetId().c_str());
	}
}

void AudioCapture::OpenDevice(const std::wstring &id) {
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
		throw MakeException(L"CoCreateInstance(IMMDeviceEnumerator) failed: hr = 0x%08x", hr);
	}

	hr = pMMDeviceEnumerator->GetDevice(id.c_str(), &m_pMMDevice);
	if (FAILED(hr)) {
		throw MakeException(L"Could not find a device id %s. hr = 0x%08x", id.c_str(), hr);
	}
}

void AudioCapture::Start(const std::wstring &id) {
	CoInitialize(NULL);

	OpenDevice(id);
	Log(L"Audio device: %s", AudioEndPointDescriptor::GetDeviceName(m_pMMDevice).c_str());

	m_hThread.Set(CreateThread(
		NULL, 0,
		LoopbackCaptureThreadFunction, this,
		0, NULL
	));
	if (!m_hThread.IsValid()) {
		throw MakeException(L"CreateThread failed: last error is %u", GetLastError());
	}

	// wait for either capture to start or the thread to end
	HANDLE waitArray[2] = { m_startedEvent.Get(), m_hThread.Get() };
	DWORD waitResult = WaitForMultipleObjects(
		sizeof(waitArray) / sizeof(waitArray[0]), waitArray,
		FALSE, INFINITE
	);

	if (WAIT_OBJECT_0 + 1 == waitResult) {
		throw MakeException(L"Thread aborted before starting to loopback capture. message=%s", m_errorMessage.c_str());
	}

	if (WAIT_OBJECT_0 != waitResult) {
		throw MakeException(L"Unexpected WaitForMultipleObjects return value %u", waitResult);
	}
}

void AudioCapture::Shutdown() {
	m_stopEvent.SetEvent();
	DWORD waitResult = WaitForSingleObject(m_hThread.Get(), INFINITE);
	if (WAIT_OBJECT_0 != waitResult) {
		Log(L"WaitForSingleObject returned unexpected result 0x%08x, last error is %d", waitResult, GetLastError());
	}

	// at this point the thread is definitely finished

	DWORD exitCode;
	if (!GetExitCodeThread(m_hThread.Get(), &exitCode)) {
		throw MakeException(L"GetExitCodeThread failed: last error is %u", GetLastError());
	}

	if (0 != exitCode) {
		throw MakeException(L"Loopback capture thread exit code is %u; expected 0", exitCode);
	}

	if (Settings::Instance().m_DebugCaptureOutput) {
		// reopen the file in read/write mode
		MMIOINFO mi = { 0 };
		MMIOHandle file(mmioOpenW((LPWSTR)Settings::Instance().GetAudioOutput().c_str(), &mi, MMIO_READWRITE));
		if (!file.IsValid()) {
			throw MakeException(L"mmioOpen(\"%s\", ...) failed. wErrorRet == %u", Settings::Instance().GetAudioOutput().c_str(), mi.wErrorRet);
		}

		// descend into the RIFF/WAVE chunk
		MMCKINFO ckRIFF = { 0 };
		ckRIFF.ckid = MAKEFOURCC('W', 'A', 'V', 'E'); // this is right for mmioDescend
		MMRESULT result = mmioDescend(file.Get(), &ckRIFF, NULL, MMIO_FINDRIFF);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException(L"mmioDescend(\"WAVE\") failed: MMSYSERR = %u", result);
		}

		// descend into the fact chunk
		MMCKINFO ckFact = { 0 };
		ckFact.ckid = MAKEFOURCC('f', 'a', 'c', 't');
		result = mmioDescend(file.Get(), &ckFact, &ckRIFF, MMIO_FINDCHUNK);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException(L"mmioDescend(\"fact\") failed: MMSYSERR = %u", result);
		}

		// write the correct data to the fact chunk
		LONG lBytesWritten = mmioWrite(
			file.Get(),
			reinterpret_cast<PCHAR>(&m_frames),
			sizeof(m_frames)
		);
		if (lBytesWritten != sizeof(m_frames)) {
			throw MakeException(L"Updating the fact chunk wrote %u bytes; expected %u", lBytesWritten, (UINT32)sizeof(m_frames));
		}

		// ascend out of the fact chunk
		result = mmioAscend(file.Get(), &ckFact, 0);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException(L"mmioAscend(\"fact\") failed: MMSYSERR = %u", result);
		}
	}
}


DWORD WINAPI AudioCapture::LoopbackCaptureThreadFunction(LPVOID pContext) {
	AudioCapture *self = (AudioCapture*)pContext;

	HRESULT hr = CoInitialize(NULL);
	if (FAILED(hr)) {
		Log(L"CoInitialize failed: hr = 0x%08x", hr);
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
				Log(L"Exception on sound capture (Retry). message=%s", e.what());
				continue;
			}
			m_errorMessage = e.what();
			Log(L"Exception on sound capture. message=%s", e.what());
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
		throw MakeException(L"IMMDevice::Activate(IAudioClient) failed: hr = 0x%08x", hr);
	}

	// get the default device periodicity
	REFERENCE_TIME hnsDefaultDevicePeriod;
	hr = pAudioClient->GetDevicePeriod(&hnsDefaultDevicePeriod, NULL);
	if (FAILED(hr)) {
		throw MakeException(L"IAudioClient::GetDevicePeriod failed: hr = 0x%08x", hr);
	}

	// get the default device format
	WAVEFORMATEX *pwfx;
	hr = pAudioClient->GetMixFormat(&pwfx);
	if (FAILED(hr)) {
		throw MakeException(L"IAudioClient::GetMixFormat failed: hr = 0x%08x", hr);
	}
	TaskMem taskmem(pwfx);

	Log(L"MixFormat: nBlockAlign=%d wFormatTag=%d wBitsPerSample=%d nChannels=%d nSamplesPerSec=%d"
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
		Log(L"PWAVEFORMATEXTENSIBLE: SubFormat=%d wValidBitsPerSample=%d"
			, pEx->SubFormat, pEx->Samples.wValidBitsPerSample);
		if (IsEqualGUID(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, pEx->SubFormat)) {
			pEx->SubFormat = KSDATAFORMAT_SUBTYPE_PCM;
			pEx->Samples.wValidBitsPerSample = 16;
			pwfx->wBitsPerSample = 16;
			pwfx->nBlockAlign = pwfx->nChannels * pwfx->wBitsPerSample / 8;
			pwfx->nAvgBytesPerSec = pwfx->nBlockAlign * pwfx->nSamplesPerSec;
		}
		else {
			throw MakeException(L"Don't know how to coerce mix format to int-16");
		}
	}
	break;

	default:
		throw MakeException(L"Don't know how to coerce WAVEFORMATEX with wFormatTag = 0x%08x to int-16", pwfx->wFormatTag);
	}

	MMCKINFO ckRIFF = { 0 };
	MMCKINFO ckData = { 0 };
	MMIOHandle hFile;
	if (Settings::Instance().m_DebugCaptureOutput) {
		MMIOINFO mi = { 0 };

		hFile.Set(mmioOpenW(
			// some flags cause mmioOpen write to this buffer
			// but not any that we're using
			const_cast<LPWSTR>(Settings::Instance().GetAudioOutput().c_str()),
			&mi,
			MMIO_WRITE | MMIO_CREATE
		));

		if (!hFile.IsValid()) {
			Log(L"Error on open audio debug output. mmioOpen(\"%s\", ...) failed. wErrorRet == %u", Settings::Instance().GetAudioOutput().c_str(), mi.wErrorRet);
		}
		try {
			WriteWaveHeader(hFile.Get(), pwfx, &ckRIFF, &ckData);
		}
		catch (Exception e) {
			Log(L"Error on wrting debug audio output. Close output file.");
			hFile.Close();
		}
	}

	// create a periodic waitable timer
	Handle wakeUp(CreateWaitableTimer(NULL, FALSE, NULL));
	if (!wakeUp.IsValid()) {
		throw MakeException(L"CreateWaitableTimer failed: last error = %u", GetLastError());
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
		throw MakeException(L"IAudioClient::Initialize failed: hr = 0x%08x", hr);
	}

	std::unique_ptr<Resampler> resampler(std::make_unique<Resampler>(pwfx->nSamplesPerSec, DEFAULT_SAMPLE_RATE, pwfx->nChannels, DEFAULT_CHANNELS));

	// activate an IAudioCaptureClient
	ComPtr<IAudioCaptureClient> pAudioCaptureClient;
	hr = pAudioClient->GetService(
		__uuidof(IAudioCaptureClient),
		(void**)&pAudioCaptureClient
	);
	if (FAILED(hr)) {
		throw MakeException(L"IAudioClient::GetService(IAudioCaptureClient) failed: hr = 0x%08x", hr);
	}

	// register with MMCSS
	DWORD nTaskIndex = 0;
	HANDLE hTask = AvSetMmThreadCharacteristicsW(L"Audio", &nTaskIndex);
	if (NULL == hTask) {
		throw MakeException(L"AvSetMmThreadCharacteristics failed: last error = %u", GetLastError());
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
		throw MakeException(L"SetWaitableTimer failed: last error = %u", dwErr);
	}
	CancelWaitableTimerOnExit cancelWakeUp(wakeUp.Get());

	// call IAudioClient::Start
	hr = pAudioClient->Start();
	if (FAILED(hr)) {
		throw MakeException(L"IAudioClient::Start failed: hr = 0x%08x", hr);
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
				throw MakeException(L"IAudioCaptureClient::GetBuffer failed on pass %u after %u frames: hr = 0x%08x", nPasses, m_frames, hr);
			}

			if (bFirstPacket && AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY == dwFlags) {
				Log(L"Probably spurious glitch reported on first packet");
			}
			else if (0 != dwFlags) {
				Log(L"IAudioCaptureClient::GetBuffer set flags to 0x%08x on pass %u after %u frames", dwFlags, nPasses, m_frames);
			}

			if (0 == nNumFramesToRead) {
				throw MakeException(L"IAudioCaptureClient::GetBuffer said to read 0 frames on pass %u after %u frames", nPasses, m_frames);
			}

			LONG lBytesToWrite = nNumFramesToRead * nBlockAlign;
			resampler->FeedInput(nNumFramesToRead, (uint8_t *)pData);

			m_listener->SendAudio(resampler->GetDest(), resampler->GetDestBufSize(), GetTimestampUs());

			m_frames += nNumFramesToRead;

			if (hFile.IsValid()) {
#pragma prefast(suppress: __WARNING_INCORRECT_ANNOTATION, "IAudioCaptureClient::GetBuffer SAL annotation implies a 1-byte buffer")
				LONG lBytesWritten = mmioWrite(hFile.Get(), reinterpret_cast<PCHAR>(pData), lBytesToWrite);
				if (lBytesToWrite != lBytesWritten) {
					Log(L"mmioWrite wrote %u bytes on pass %u after %u frames: expected %u bytes", lBytesWritten, nPasses, m_frames, lBytesToWrite);
					hFile.Close();
				}
			}

			hr = pAudioCaptureClient->ReleaseBuffer(nNumFramesToRead);
			if (FAILED(hr)) {
				throw MakeException(L"IAudioCaptureClient::ReleaseBuffer failed on pass %u after %u frames: hr = 0x%08x", nPasses, m_frames, hr);
			}

			bFirstPacket = false;
		}

		if (FAILED(hr)) {
			if (hr == AUDCLNT_E_DEVICE_INVALIDATED) {
				// When configuration of the sound device was changed. (e.g. sample rate, sample format)
				// We can retry to capture.
				m_canRetry = true;
			}
			throw MakeException(L"IAudioCaptureClient::GetNextPacketSize failed on pass %u after %u frames: hr = 0x%08x", nPasses, m_frames, hr);
		}

		DWORD waitResult = WaitForMultipleObjects(
			sizeof(waitArray) / sizeof(waitArray[0]), waitArray,
			FALSE, INFINITE
		);

		if (WAIT_OBJECT_0 == waitResult) {
			Log(L"Received stop event after %u passes and %u frames", nPasses, m_frames);
			bDone = true;
			continue; // exits loop
		}

		if (WAIT_OBJECT_0 + 1 != waitResult) {
			throw MakeException(L"Unexpected WaitForMultipleObjects return value %u on pass %u after %u frames", waitResult, nPasses, m_frames);
		}
	} // capture loop

	if (hFile.IsValid()) {
		FinishWaveFile(hFile.Get(), &ckData, &ckRIFF);
	}
}

void AudioCapture::WriteWaveHeader(HMMIO hFile, LPCWAVEFORMATEX pwfx, MMCKINFO *pckRIFF, MMCKINFO *pckData) {
	MMRESULT result;

	// make a RIFF/WAVE chunk
	pckRIFF->ckid = MAKEFOURCC('R', 'I', 'F', 'F');
	pckRIFF->fccType = MAKEFOURCC('W', 'A', 'V', 'E');

	result = mmioCreateChunk(hFile, pckRIFF, MMIO_CREATERIFF);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException(L"mmioCreateChunk(\"RIFF/WAVE\") failed: MMRESULT = 0x%08x", result);
	}

	// make a 'fmt ' chunk (within the RIFF/WAVE chunk)
	MMCKINFO chunk;
	chunk.ckid = MAKEFOURCC('f', 'm', 't', ' ');
	result = mmioCreateChunk(hFile, &chunk, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException(L"mmioCreateChunk(\"fmt \") failed: MMRESULT = 0x%08x", result);
	}

	// write the WAVEFORMATEX data to it
	LONG lBytesInWfx = sizeof(WAVEFORMATEX) + pwfx->cbSize;
	LONG lBytesWritten =
		mmioWrite(
			hFile,
			reinterpret_cast<PCHAR>(const_cast<LPWAVEFORMATEX>(pwfx)),
			lBytesInWfx
		);
	if (lBytesWritten != lBytesInWfx) {
		throw MakeException(L"mmioWrite(fmt data) wrote %u bytes; expected %u bytes", lBytesWritten, lBytesInWfx);
	}

	// ascend from the 'fmt ' chunk
	result = mmioAscend(hFile, &chunk, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException(L"mmioAscend(\"fmt \" failed: MMRESULT = 0x%08x", result);
	}

	// make a 'fact' chunk whose data is (DWORD)0
	chunk.ckid = MAKEFOURCC('f', 'a', 'c', 't');
	result = mmioCreateChunk(hFile, &chunk, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException(L"mmioCreateChunk(\"fmt \") failed: MMRESULT = 0x%08x", result);
	}

	// write (DWORD)0 to it
	// this is cleaned up later
	DWORD frames = 0;
	lBytesWritten = mmioWrite(hFile, reinterpret_cast<PCHAR>(&frames), sizeof(frames));
	if (lBytesWritten != sizeof(frames)) {
		throw MakeException(L"mmioWrite(fact data) wrote %u bytes; expected %u bytes", lBytesWritten, (UINT32)sizeof(frames));
	}

	// ascend from the 'fact' chunk
	result = mmioAscend(hFile, &chunk, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException(L"mmioAscend(\"fact\" failed: MMRESULT = 0x%08x", result);
	}

	// make a 'data' chunk and leave the data pointer there
	pckData->ckid = MAKEFOURCC('d', 'a', 't', 'a');
	result = mmioCreateChunk(hFile, pckData, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException(L"mmioCreateChunk(\"data\") failed: MMRESULT = 0x%08x", result);
	}
}

void AudioCapture::FinishWaveFile(HMMIO hFile, MMCKINFO *pckRIFF, MMCKINFO *pckData) {
	MMRESULT result;

	result = mmioAscend(hFile, pckData, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException(L"mmioAscend(\"data\" failed: MMRESULT = 0x%08x", result);
	}

	result = mmioAscend(hFile, pckRIFF, 0);
	if (MMSYSERR_NOERROR != result) {
		throw MakeException(L"mmioAscend(\"RIFF/WAVE\" failed: MMRESULT = 0x%08x", result);
	}
}
