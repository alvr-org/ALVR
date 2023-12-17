#include "settings.h"
#define PICOJSON_USE_INT64
#include "alvr_server/include/picojson.h"
#include <string>
#include <fstream>
#include <streambuf>
#include <filesystem>
#include <cstdlib>
#include "layer.h"
#include "util/logger.h"

using namespace std;

extern uint64_t g_DriverTestMode;

Settings Settings::m_Instance;

Settings::Settings()
	: m_loaded(false)
{
}

Settings::~Settings()
{
}

void Settings::Load()
{
	try
	{
		auto sessionFile = std::ifstream(g_sessionPath);

		auto json = std::string(
			std::istreambuf_iterator<char>(sessionFile),
			std::istreambuf_iterator<char>());

		picojson::value v;
		std::string err = picojson::parse(v, json);
		if (!err.empty())
		{
			Error("Error on parsing session config (%s): %hs\n", g_sessionPath, err.c_str());
			return;
		}

		auto config = v.get("openvr_config");

		m_renderWidth = config.get("eye_resolution_width").get<int64_t>() * 2;
		m_renderHeight = config.get("eye_resolution_height").get<int64_t>();

		m_refreshRate = (int)config.get("refresh_rate").get<int64_t>();
		
		Debug("Config JSON: %hs\n", json.c_str());
		Info("Render Target: %d %d\n", m_renderWidth, m_renderHeight);
		Info("Refresh Rate: %d\n", m_refreshRate);
		m_loaded = true;
	}
	catch (std::exception &e)
	{
		Error("Exception on parsing session config (%s): %hs\n", g_sessionPath, e.what());
	}
}
