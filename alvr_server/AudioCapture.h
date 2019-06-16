#pragma once
#include <stdio.h>
#include <windows.h>
#include <mmsystem.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <avrt.h>
#include <functiondiscoverykeys_devpkey.h>
#include <wrl.h>
#include "openvr-utils\ipctools.h"

#include "Logger.h"
#include "Settings.h"
#include "Utils.h"
#include "Listener.h"
#include "ResampleUtils.h"

using Microsoft::WRL::ComPtr;

class Handle {
public:
	Handle(HANDLE handle = NULL) : mHandle(handle) {
	}
	~Handle() {
		if (mHandle != NULL) {
			CloseHandle(mHandle);
		}
	}
	void Set(HANDLE handle) {
		mHandle = handle;
	}
	bool IsValid() {
		return mHandle != NULL;
	}
	HANDLE Get() {
		return mHandle;
	}

private:
	HANDLE mHandle;
};

class AudioEndPointDescriptor {
public:
	AudioEndPointDescriptor(const ComPtr<IMMDevice> &device, bool isDefault);
	std::wstring GetName() const;
	std::wstring GetId() const;
	bool IsDefault() const;
	bool operator==(const AudioEndPointDescriptor& a);
	bool operator!=(const AudioEndPointDescriptor& a);

	static std::wstring GetDeviceName(const ComPtr<IMMDevice> &pMMDevice);
private:
	std::wstring mName;
	std::wstring mID;
	bool mIsDefault;
};

class AudioCapture
{
public:
	AudioCapture(std::shared_ptr<Listener> listener);

	virtual ~AudioCapture();

	static void list_devices(std::vector<AudioEndPointDescriptor> &deviceList);

	void OpenDevice(const std::wstring &id);
	void Start(const std::wstring &id);

	void Shutdown();

	static DWORD WINAPI LoopbackCaptureThreadFunction(LPVOID pContext);
	void CaptureRetry();

	void LoopbackCapture();

	void WriteWaveHeader(HMMIO hFile, LPCWAVEFORMATEX pwfx, MMCKINFO *pckRIFF, MMCKINFO *pckData);
	void FinishWaveFile(HMMIO hFile, MMCKINFO *pckRIFF, MMCKINFO *pckData);
private:
	Handle mThread;
	std::shared_ptr<Listener> mListener;

	ComPtr<IMMDevice> mMMDevice;
	PWAVEFORMATEX mWaveFormat;
	UINT32 mFrames;

	IPCEvent mStartedEvent;
	IPCEvent mStopEvent;

	bool mCanRetry;
	std::wstring mErrorMessage;

	static const int DEFAULT_SAMPLE_RATE = 48000;
	static const int DEFAULT_CHANNELS = 2;
};

