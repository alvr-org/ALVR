#pragma once
#include <stdio.h>
#include <windows.h>
#include <mmsystem.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <avrt.h>
#include <functiondiscoverykeys_devpkey.h>
#include <wrl.h>
#include <ipctools.h>

#include "../Logger.h"
#include "../Settings.h"
#include "../Utils.h"
#include "../Listener.h"

using Microsoft::WRL::ComPtr;

class PropVariant {
public:
	PropVariant() {
		PropVariantInit(&pv);
	}

	~PropVariant() {
		HRESULT hr = PropVariantClear(&pv);
		if (FAILED(hr)) {
			Log("PropVariantClear failed: hr = 0x%08x", hr);
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

class Handle {
public:
	Handle(HANDLE handle = NULL) : m_handle(handle) {
	}
	~Handle() {
		if (m_handle != NULL) {
			CloseHandle(m_handle);
		}
	}
	void Set(HANDLE handle) {
		m_handle = handle;
	}
	bool IsValid() {
		return m_handle != NULL;
	}
	HANDLE Get() {
		return m_handle;
	}

private:
	HANDLE m_handle;
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
			Log("IAudioClient::Stop failed: hr = 0x%08x", hr);
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
			Log("AvRevertMmThreadCharacteristics failed: last error is %d", GetLastError());
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
			Log("CancelWaitableTimer failed: last error is %d", GetLastError());
		}
	}
private:
	HANDLE m_h;
};

class AudioCapture
{
public:
	AudioCapture(std::shared_ptr<Listener> listener)
		: m_pMMDevice(NULL)
		, m_bInt16(false)
		, m_pwfx(NULL)
		, m_startedEvent(NULL)
		, m_stopEvent(NULL)
		, m_listener(listener) {
	}

	virtual ~AudioCapture() {
	}

	static void list_devices(std::vector<std::wstring> &deviceNames) {
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

		ComPtr<IMMDeviceCollection> pMMDeviceCollection;

		// get all the active render endpoints
		hr = pMMDeviceEnumerator->EnumAudioEndpoints(
			eRender, DEVICE_STATE_ACTIVE, &pMMDeviceCollection
		);
		if (FAILED(hr)) {
			throw MakeException("IMMDeviceEnumerator::EnumAudioEndpoints failed: hr = 0x%08x", hr);
		}

		UINT count;
		hr = pMMDeviceCollection->GetCount(&count);
		if (FAILED(hr)) {
			throw MakeException("IMMDeviceCollection::GetCount failed: hr = 0x%08x", hr);
		}
		Log("Active render endpoints found: %u", count);

		for (UINT i = 0; i < count; i++) {
			ComPtr<IMMDevice> pMMDevice;

			// get the "n"th device
			hr = pMMDeviceCollection->Item(i, &pMMDevice);
			if (FAILED(hr)) {
				throw MakeException("IMMDeviceCollection::Item failed: hr = 0x%08x", hr);
			}

			auto name = GetDeviceName(pMMDevice);

			Log("Device%u:%ls", i, name.c_str());
			deviceNames.push_back(name);
		}
	}

	static std::wstring GetDeviceName(const ComPtr<IMMDevice> &pMMDevice) {
		// open the property store on that device
		ComPtr<IPropertyStore> pPropertyStore;
		HRESULT hr = pMMDevice->OpenPropertyStore(STGM_READ, &pPropertyStore);
		if (FAILED(hr)) {
			throw MakeException("IMMDevice::OpenPropertyStore failed: hr = 0x%08x", hr);
		}

		// get the long name property
		PropVariant pv;
		hr = pPropertyStore->GetValue(PKEY_Device_FriendlyName, &pv.Get());
		if (FAILED(hr)) {
			throw MakeException("IPropertyStore::GetValue failed: hr = 0x%08x", hr);
		}

		if (VT_LPWSTR != pv.Get().vt) {
			throw MakeException("PKEY_Device_FriendlyName variant type is %u - expected VT_LPWSTR", pv.Get().vt);
		}
		return pv.Get().pwszVal;
	}

	void OpenDevice(const std::wstring &deviceName) {
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

		ComPtr<IMMDeviceCollection> pMMDeviceCollection;

		// get all the active render endpoints
		hr = pMMDeviceEnumerator->EnumAudioEndpoints(
			eRender, DEVICE_STATE_ACTIVE, &pMMDeviceCollection
		);
		if (FAILED(hr)) {
			throw MakeException("IMMDeviceEnumerator::EnumAudioEndpoints failed: hr = 0x%08x", hr);
		}

		UINT count;
		hr = pMMDeviceCollection->GetCount(&count);
		if (FAILED(hr)) {
			throw MakeException("IMMDeviceCollection::GetCount failed: hr = 0x%08x", hr);
		}

		for (UINT i = 0; i < count; i++) {
			ComPtr<IMMDevice> pMMDevice;

			// get the "n"th device
			hr = pMMDeviceCollection->Item(i, &pMMDevice);
			if (FAILED(hr)) {
				throw MakeException("IMMDeviceCollection::Item failed: hr = 0x%08x", hr);
			}

			auto name = GetDeviceName(pMMDevice);

			// is it a match?
			if (deviceName == name) {
				// did we already find it?
				if (!m_pMMDevice) {
					m_pMMDevice = pMMDevice;
				}
				else {
					throw MakeException("Found (at least) two devices named %ls", deviceName.c_str());
				}
			}
		}

		if (!m_pMMDevice) {
			throw MakeException("Could not find a device named %ls", deviceName.c_str());
		}
	}

	static HRESULT open_file(LPCWSTR szFileName, HMMIO *phFile) {
		MMIOINFO mi = { 0 };

		*phFile = mmioOpenW(
			// some flags cause mmioOpen write to this buffer
			// but not any that we're using
			const_cast<LPWSTR>(szFileName),
			&mi,
			MMIO_WRITE | MMIO_CREATE
		);

		if (NULL == *phFile) {
			Log("mmioOpen(\"%ls\", ...) failed. wErrorRet == %u", szFileName, mi.wErrorRet);
			return E_FAIL;
		}

		return S_OK;
	}

	void Start(const std::wstring &deviceName) {
		CoInitialize(NULL);

		OpenDevice(deviceName);

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
			throw MakeException("Thread aborted before starting to loopback capture.");
		}

		if (WAIT_OBJECT_0 != waitResult) {
			throw MakeException("Unexpected WaitForMultipleObjects return value %u", waitResult);
		}
	}

	void Shutdown() {
		m_stopEvent.SetEvent();
		DWORD waitResult = WaitForSingleObject(m_hThread.Get(), INFINITE);
		if (WAIT_OBJECT_0 != waitResult) {
			Log("WaitForSingleObject returned unexpected result 0x%08x, last error is %d", waitResult, GetLastError());
		}

		// at this point the thread is definitely finished

		DWORD exitCode;
		if (!GetExitCodeThread(m_hThread.Get(), &exitCode)) {
			throw MakeException("GetExitCodeThread failed: last error is %u", GetLastError());
		}

		if (0 != exitCode) {
			throw MakeException("Loopback capture thread exit code is %u; expected 0", exitCode);
		}

		// reopen the file in read/write mode
		MMIOINFO mi = { 0 };
		MMIOHandle file(mmioOpenW((LPWSTR)Settings::Instance().GetAudioOutput().c_str(), &mi, MMIO_READWRITE));
		if (!file.IsValid()) {
			throw MakeException("mmioOpen(\"%ls\", ...) failed. wErrorRet == %u", Settings::Instance().GetAudioOutput().c_str(), mi.wErrorRet);
		}

		// descend into the RIFF/WAVE chunk
		MMCKINFO ckRIFF = { 0 };
		ckRIFF.ckid = MAKEFOURCC('W', 'A', 'V', 'E'); // this is right for mmioDescend
		MMRESULT result = mmioDescend(file.Get(), &ckRIFF, NULL, MMIO_FINDRIFF);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioDescend(\"WAVE\") failed: MMSYSERR = %u", result);
		}

		// descend into the fact chunk
		MMCKINFO ckFact = { 0 };
		ckFact.ckid = MAKEFOURCC('f', 'a', 'c', 't');
		result = mmioDescend(file.Get(), &ckFact, &ckRIFF, MMIO_FINDCHUNK);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioDescend(\"fact\") failed: MMSYSERR = %u", result);
		}

		// write the correct data to the fact chunk
		LONG lBytesWritten = mmioWrite(
			file.Get(),
			reinterpret_cast<PCHAR>(&m_frames),
			sizeof(m_frames)
		);
		if (lBytesWritten != sizeof(m_frames)) {
			throw MakeException("Updating the fact chunk wrote %u bytes; expected %u", lBytesWritten, (UINT32)sizeof(m_frames));
		}

		// ascend out of the fact chunk
		result = mmioAscend(file.Get(), &ckFact, 0);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioAscend(\"fact\") failed: MMSYSERR = %u", result);
		}
	}


	static DWORD WINAPI LoopbackCaptureThreadFunction(LPVOID pContext) {
		AudioCapture *self = (AudioCapture*)pContext;

		HRESULT hr = CoInitialize(NULL);
		if (FAILED(hr)) {
			Log("CoInitialize failed: hr = 0x%08x", hr);
			return 0;
		}

		self->LoopbackCapture();

		CoUninitialize();

		return 0;
	}

	HRESULT LoopbackCapture() {
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

		if (m_bInt16) {
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
				if (IsEqualGUID(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, pEx->SubFormat)) {
					pEx->SubFormat = KSDATAFORMAT_SUBTYPE_PCM;
					pEx->Samples.wValidBitsPerSample = 16;
					pwfx->wBitsPerSample = 16;
					pwfx->nBlockAlign = pwfx->nChannels * pwfx->wBitsPerSample / 8;
					pwfx->nAvgBytesPerSec = pwfx->nBlockAlign * pwfx->nSamplesPerSec;
				}
				else {
					throw MakeException("%s", L"Don't know how to coerce mix format to int-16");
				}
			}
			break;

			default:
				throw MakeException("Don't know how to coerce WAVEFORMATEX with wFormatTag = 0x%08x to int-16", pwfx->wFormatTag);
			}
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
				Log("Error on open audio debug output. mmioOpen(\"%ls\", ...) failed. wErrorRet == %u", Settings::Instance().GetAudioOutput().c_str(), mi.wErrorRet);
			}
			try {
				WriteWaveHeader(hFile.Get(), pwfx, &ckRIFF, &ckData);
			}
			catch (Exception e){
				Log("Error on wrting debug audio output. Close output file.");
				hFile.Close();
			}
		}

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
			return HRESULT_FROM_WIN32(dwErr);
		}
		CancelWaitableTimerOnExit cancelWakeUp(wakeUp.Get());

		// call IAudioClient::Start
		hr = pAudioClient->Start();
		if (FAILED(hr)) {
			throw MakeException("IAudioClient::Start failed: hr = 0x%08x", hr);
			return hr;
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
					Log("%s", L"Probably spurious glitch reported on first packet");
				}
				else if (0 != dwFlags) {
					throw MakeException("IAudioCaptureClient::GetBuffer set flags to 0x%08x on pass %u after %u frames", dwFlags, nPasses, m_frames);
				}

				if (0 == nNumFramesToRead) {
					throw MakeException("IAudioCaptureClient::GetBuffer said to read 0 frames on pass %u after %u frames", nPasses, m_frames);
				}

				LONG lBytesToWrite = nNumFramesToRead * nBlockAlign;

				m_listener->SendAudio((uint8_t *)pData, lBytesToWrite, GetTimestampUs());

				m_frames += nNumFramesToRead;

				if (hFile.IsValid()) {
#pragma prefast(suppress: __WARNING_INCORRECT_ANNOTATION, "IAudioCaptureClient::GetBuffer SAL annotation implies a 1-byte buffer")
					LONG lBytesWritten = mmioWrite(hFile.Get(), reinterpret_cast<PCHAR>(pData), lBytesToWrite);
					if (lBytesToWrite != lBytesWritten) {
						Log("mmioWrite wrote %u bytes on pass %u after %u frames: expected %u bytes", lBytesWritten, nPasses, m_frames, lBytesToWrite);
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
				throw MakeException("IAudioCaptureClient::GetNextPacketSize failed on pass %u after %u frames: hr = 0x%08x", nPasses, m_frames, hr);
			}

			DWORD waitResult = WaitForMultipleObjects(
				sizeof(waitArray) / sizeof(waitArray[0]), waitArray,
				FALSE, INFINITE
			);

			if (WAIT_OBJECT_0 == waitResult) {
				Log("Received stop event after %u passes and %u frames", nPasses, m_frames);
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

	void WriteWaveHeader(HMMIO hFile, LPCWAVEFORMATEX pwfx, MMCKINFO *pckRIFF, MMCKINFO *pckData) {
		MMRESULT result;

		// make a RIFF/WAVE chunk
		pckRIFF->ckid = MAKEFOURCC('R', 'I', 'F', 'F');
		pckRIFF->fccType = MAKEFOURCC('W', 'A', 'V', 'E');

		result = mmioCreateChunk(hFile, pckRIFF, MMIO_CREATERIFF);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioCreateChunk(\"RIFF/WAVE\") failed: MMRESULT = 0x%08x", result);
		}

		// make a 'fmt ' chunk (within the RIFF/WAVE chunk)
		MMCKINFO chunk;
		chunk.ckid = MAKEFOURCC('f', 'm', 't', ' ');
		result = mmioCreateChunk(hFile, &chunk, 0);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioCreateChunk(\"fmt \") failed: MMRESULT = 0x%08x", result);
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
			throw MakeException("mmioWrite(fmt data) wrote %u bytes; expected %u bytes", lBytesWritten, lBytesInWfx);
		}

		// ascend from the 'fmt ' chunk
		result = mmioAscend(hFile, &chunk, 0);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioAscend(\"fmt \" failed: MMRESULT = 0x%08x", result);
		}

		// make a 'fact' chunk whose data is (DWORD)0
		chunk.ckid = MAKEFOURCC('f', 'a', 'c', 't');
		result = mmioCreateChunk(hFile, &chunk, 0);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioCreateChunk(\"fmt \") failed: MMRESULT = 0x%08x", result);
		}

		// write (DWORD)0 to it
		// this is cleaned up later
		DWORD frames = 0;
		lBytesWritten = mmioWrite(hFile, reinterpret_cast<PCHAR>(&frames), sizeof(frames));
		if (lBytesWritten != sizeof(frames)) {
			throw MakeException("mmioWrite(fact data) wrote %u bytes; expected %u bytes", lBytesWritten, (UINT32)sizeof(frames));
		}

		// ascend from the 'fact' chunk
		result = mmioAscend(hFile, &chunk, 0);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioAscend(\"fact\" failed: MMRESULT = 0x%08x", result);
		}

		// make a 'data' chunk and leave the data pointer there
		pckData->ckid = MAKEFOURCC('d', 'a', 't', 'a');
		result = mmioCreateChunk(hFile, pckData, 0);
		if (MMSYSERR_NOERROR != result) {
			throw MakeException("mmioCreateChunk(\"data\") failed: MMRESULT = 0x%08x", result);
		}
	}

	void FinishWaveFile(HMMIO hFile, MMCKINFO *pckRIFF, MMCKINFO *pckData) {
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

private:
	Handle m_hThread;
	std::shared_ptr<Listener> m_listener;

	ComPtr<IMMDevice> m_pMMDevice;
	bool m_bInt16;
	PWAVEFORMATEX m_pwfx;
	UINT32 m_frames;

	IPCEvent m_startedEvent;
	IPCEvent m_stopEvent;
};

