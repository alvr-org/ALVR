#pragma once

#include <string>
#include <cstdint>

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

	int m_refreshRate;
	uint32_t m_renderWidth;
	uint32_t m_renderHeight;
};
