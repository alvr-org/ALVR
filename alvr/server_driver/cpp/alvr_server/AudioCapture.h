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

#include "Logger.h"
#include "Settings.h"
#include "Utils.h"
#include "ClientConnection.h"
#include "ResampleUtils.h"

using Microsoft::WRL::ComPtr;

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

class AudioCapture
{
public:
	AudioCapture(std::shared_ptr<ClientConnection> listener);

	virtual ~AudioCapture();

	void OpenDevice(const std::string &id);
	void Start(const std::string &id);

	void Shutdown();

	static DWORD WINAPI LoopbackCaptureThreadFunction(LPVOID pContext);
	void CaptureRetry();

	void LoopbackCapture();

	void FinishWaveFile(HMMIO hFile, MMCKINFO *pckRIFF, MMCKINFO *pckData);
private:
	Handle m_hThread;
	std::shared_ptr<ClientConnection> m_listener;

	ComPtr<IMMDevice> m_pMMDevice;
	PWAVEFORMATEX m_pwfx;
	UINT32 m_frames;

	IPCEvent m_startedEvent;
	IPCEvent m_stopEvent;

	bool m_canRetry;
	std::string m_errorMessage;

	static const int DEFAULT_SAMPLE_RATE = 48000;
	static const int DEFAULT_CHANNELS = 2;
};

