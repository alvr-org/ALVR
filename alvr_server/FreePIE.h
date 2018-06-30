#pragma once

#include "ipctools.h"
#include "resource.h"
#include "packet_types.h"
#include "Utils.h"
#include "Logger.h"

class FreePIE
{
public:
	static const uint32_t ALVR_FREEPIE_SIGNATURE_V2 = 0x11223345;

	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION = 1 << 0;
	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION0 = 1 << 1;
	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION = 1 << 2;
	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION0 = 1 << 3;
	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_BUTTONS = 1 << 4;

	static const uint32_t ALVR_FREEPIE_INPUT_BUTTON_TRACKPAD_CLICK = 1 << 0;
	static const uint32_t ALVR_FREEPIE_INPUT_BUTTON_TRACKPAD_TOUCH = 1 << 1;
	static const uint32_t ALVR_FREEPIE_INPUT_BUTTON_TRIGGER = 1 << 2;
	static const uint32_t ALVR_FREEPIE_INPUT_BUTTON_BACK = 1 << 3;
	static const uint32_t ALVR_FREEPIE_INPUT_BUTTON_VOLUME_UP = 1 << 4;
	static const uint32_t ALVR_FREEPIE_INPUT_BUTTON_VOLUME_DOWN = 1 << 5;
	static const uint32_t ALVR_FREEPIE_BUTTONS = 21;
	static const uint32_t ALVR_FREEPIE_MESSAGE_LENGTH = 512;

	static const int BUTTON_MAP[FreePIE::ALVR_FREEPIE_BUTTONS];

#pragma pack(push, 1)
	struct FreePIEFileMapping {
		uint32_t version;
		uint32_t flags;
		double input_head_orientation[3];
		double input_controller_orientation[3];
		double input_head_position[3];
		double input_controller_position[3];
		double input_trackpad[2];
		uint16_t inputControllerButtons;
		uint16_t controllers;
		uint32_t controllerButtons[2];
		double head_orientation[3];
		double controller_orientation[2][3];
		double head_position[3];
		double controller_position[2][3];
		double trigger[2];
		double trigger_left[2];
		double trigger_right[2];
		double joystick_left[2][2];
		double joystick_right[2][2];
		double trackpad[2][2];
		char message[ALVR_FREEPIE_MESSAGE_LENGTH];
	};
#pragma pack(pop)

	FreePIE()
		: m_fileMapping(ALVR_FREEPIE_FILEMAPPING_NAME, sizeof(FreePIEFileMapping))
		, m_mutex(ALVR_FREEPIE_MUTEX_NAME) {
		Initialize();
	}
	~FreePIE() {
	}

	void UpdateTrackingInfoByFreePIE(const TrackingInfo &info, vr::HmdQuaternion_t &head_orientation
		, vr::HmdQuaternion_t &controller_orientation, const TrackingVector3 &head_position, const TrackingVector3 &controller_position) {
		m_mutex.Wait();

		QuaternionToEulerAngle(head_orientation, m_p->input_head_orientation);
		QuaternionToEulerAngle(controller_orientation, m_p->input_controller_orientation);
		m_p->input_head_position[0] = head_position.x;
		m_p->input_head_position[1] = head_position.y;
		m_p->input_head_position[2] = head_position.z;
		m_p->input_controller_position[0] = controller_position.x;
		m_p->input_controller_position[1] = controller_position.y;
		m_p->input_controller_position[2] = controller_position.z;

		m_p->input_trackpad[0] = info.controllerTrackpadPosition.x;
		m_p->input_trackpad[1] = info.controllerTrackpadPosition.y;

		m_p->inputControllerButtons =
			((info.controllerButtons & TrackingInfo::CONTROLLER_BUTTON_TRACKPAD_CLICK) ? ALVR_FREEPIE_INPUT_BUTTON_TRACKPAD_CLICK : 0)
			| ((info.flags & TrackingInfo::FLAG_CONTROLLER_TRACKPAD_TOUCH) ? ALVR_FREEPIE_INPUT_BUTTON_TRACKPAD_TOUCH : 0)
			| ((info.controllerButtons & TrackingInfo::CONTROLLER_BUTTON_TRIGGER_CLICK) ? ALVR_FREEPIE_INPUT_BUTTON_TRIGGER : 0)
			| ((info.flags & TrackingInfo::FLAG_CONTROLLER_BACK) ? ALVR_FREEPIE_INPUT_BUTTON_BACK : 0)
			| ((info.flags & TrackingInfo::FLAG_CONTROLLER_VOLUME_UP) ? ALVR_FREEPIE_INPUT_BUTTON_VOLUME_UP : 0)
			| ((info.flags & TrackingInfo::FLAG_CONTROLLER_VOLUME_DOWN) ? ALVR_FREEPIE_INPUT_BUTTON_VOLUME_DOWN : 0);

		m_p->message[ALVR_FREEPIE_MESSAGE_LENGTH - 1] = 0;
		memcpy(&m_copy, m_p, sizeof(FreePIEFileMapping));

		m_mutex.Release();
	}

	const FreePIEFileMapping& GetData() {
		return m_copy;
	}

private:
	void Initialize() {
		m_mutex.Wait();

		m_p = (FreePIEFileMapping *)m_fileMapping.Map(FILE_MAP_WRITE);
		memset(m_p, 0, sizeof(FreePIEFileMapping));
		m_p->version = ALVR_FREEPIE_SIGNATURE_V2;
		m_p->flags = 0;

		m_p->controllers = 1;

		memcpy(&m_copy, m_p, sizeof(FreePIEFileMapping));

		m_mutex.Release();
	}

	IPCFileMapping m_fileMapping;
	IPCMutex m_mutex;


	FreePIEFileMapping *m_p;
	FreePIEFileMapping m_copy;

};

