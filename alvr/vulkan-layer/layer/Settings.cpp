#include "Settings.h"
#define PICOJSON_USE_INT64
#include "include/picojson.h"
#include <string>
#include <fstream>
#include <streambuf>
#include <filesystem>
#include <cstdlib>
#include "layer.h"

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
			return;
		}

		auto config = v.get("openvr_config");

		m_renderWidth = config.get("eye_resolution_width").get<int64_t>() * 2;
		m_renderHeight = config.get("eye_resolution_height").get<int64_t>();

		m_recommendedTargetWidth = config.get("target_eye_resolution_width").get<int64_t>() * 2;
		m_recommendedTargetHeight = config.get("target_eye_resolution_height").get<int64_t>();

		m_nAdapterIndex = (int32_t)config.get("adapter_index").get<int64_t>();

		m_refreshRate = (int)config.get("refresh_rate").get<int64_t>();

		m_loaded = true;
	}
	catch (std::exception &e)
	{
		exit(1);
	}
}
