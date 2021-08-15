#pragma once

#include <string>

class Settings
{
	static Settings m_Instance;
	bool m_loaded;

	Settings();
	virtual ~Settings();

public:
	void Load();
	static Settings &Instance() {
		return m_Instance;
	}

	bool IsLoaded() {
		return m_loaded;
	}

	int32_t m_nAdapterIndex;

	int m_refreshRate;
	uint32_t m_renderWidth;
	uint32_t m_renderHeight;
	int32_t m_recommendedTargetWidth;
	int32_t m_recommendedTargetHeight;
};
