#pragma once

#include <openvr_driver.h>

class Settings
{
	static Settings m_Instance;
public:
	Settings();
	virtual ~Settings();

	void Load();
	static Settings &Instance() {
		return m_Instance;
	}

	std::string m_sSerialNumber;
	std::string m_sModelNumber;

	int32_t m_nAdapterIndex;

	int32_t m_renderWidth;
	int32_t m_renderHeight;
	float m_flSecondsFromVsyncToPhotons;
	float m_flDisplayFrequency;
	float m_flIPD;

	std::string m_EncoderOptions;

	std::string m_Host;
	int m_Port;
	std::string m_ControlHost;
	int m_ControlPort;

	bool m_DebugLog;
	bool m_DebugTimestamp;
	bool m_DebugFrameIndex;
	bool m_DebugFrameOutput;
	bool m_DebugCaptureOutput;
	bool m_UseKeyedMutex;


	uint64_t m_SendingTimeslotUs;
	uint64_t m_LimitTimeslotPackets;

	uint32_t m_clientRecvBufferSize;

	// Controller configs
	std::string m_controllerModelNumber;
	std::string m_controllerSerialNumber;
};

