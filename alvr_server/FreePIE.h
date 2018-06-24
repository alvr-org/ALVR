#pragma once

#include "ipctools.h"
#include "resource.h"
#include "packet_types.h"
#include "Utils.h"

class FreePIE
{
public:
	static const uint32_t ALVR_FREEPIE_SIGNATURE_V1 = 0x11223344;

	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION = 1 << 0;
	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION = 1 << 1;
	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION = 1 << 2;
	static const uint32_t ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION = 1 << 3;

	static const uint32_t ALVR_FREEPIE_BUTTONS = 20;

	static const int BUTTON_MAP[FreePIE::ALVR_FREEPIE_BUTTONS];

	FreePIE()
		: m_fileMapping(ALVR_FREEPIE_FILEMAPPING_NAME, sizeof(FreePIEFileMapping))
		, m_mutex(ALVR_FREEPIE_MUTEX_NAME) {
		Initialize();
	}
	~FreePIE() {
	}

	void UpdateTrackingInfoByFreePIE(const TrackingInfo &info, vr::HmdQuaternion_t &head_orientation, double *head_position
		, vr::HmdQuaternion_t &controller_orientation, double *controller_position
		, uint32_t *controllerOverrideButtons, uint32_t *controllerButtons) {
		m_mutex.Wait();

		if (m_p->flags & ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION) {
			head_orientation = EulerAngleToQuaternion(m_p->head_orientation);
		}
		else {
			QuaternionToEulerAngle(head_orientation, m_p->head_orientation);
		}

		if (m_p->flags & ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION) {
			memcpy(head_position, m_p->head_position, sizeof(double) * 3);
		}
		else {
			memcpy(m_p->head_position, head_position, sizeof(double) * 3);
		}

		if (m_p->flags & ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION) {
			controller_orientation = EulerAngleToQuaternion(m_p->controller_orientation);
		}
		else {
			QuaternionToEulerAngle(controller_orientation, m_p->controller_orientation);
		}

		if (m_p->flags & ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION) {
			memcpy(controller_position, m_p->controller_position, sizeof(double) * 3);
		}
		else {
			memcpy(m_p->controller_position, controller_position, sizeof(double) * 3);
		}

		*controllerOverrideButtons = m_p->controllerOverrideButtons;
		*controllerButtons = m_p->controllerButtons;

		m_mutex.Release();
	}

private:
	void Initialize() {
		m_mutex.Wait();

		m_p = (FreePIEFileMapping *)m_fileMapping.Map(FILE_MAP_WRITE);
		m_p->version = ALVR_FREEPIE_SIGNATURE_V1;
		m_p->flags = 0;

		for (int i = 0; i < 12; i++) {
			m_p->head_orientation[i] = 0.0;
		}

		m_p->controllerOverrideButtons = 0;
		m_p->controllerButtons = 0;

		m_mutex.Release();
	}

	IPCFileMapping m_fileMapping;
	IPCMutex m_mutex;

	struct FreePIEFileMapping {
		uint32_t version;
		uint32_t flags;
		double head_orientation[3];
		double head_position[3];
		double controller_orientation[3];
		double controller_position[3];
		uint32_t controllerOverrideButtons;
		uint32_t controllerButtons;
	};

	FreePIEFileMapping *m_p;

};

