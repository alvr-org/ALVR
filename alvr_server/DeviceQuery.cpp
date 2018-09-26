#include <vector>
#include <picojson.h>
#include "DeviceQuery.h"
#include "AudioCapture.h"

static void Serialize(wchar_t **buf, int *len, const picojson::value &value) {
	std::wstring json = ToWstring(value.serialize());
	*len = json.size();
	*buf = new wchar_t[json.size()];
	memcpy(*buf, json.c_str(), json.size() * sizeof(wchar_t));
}

// Called from C#. Returns string of device list joined by '\0'.
extern "C" __declspec(dllexport)
void GetSoundDevices(wchar_t **buf, int *len) {
	std::vector<AudioEndPointDescriptor> deviceList;
	AudioCapture::list_devices(deviceList);

	picojson::array deviceListObj;
	for (auto it = deviceList.begin(); it != deviceList.end(); it++) {
		picojson::object elem;
		elem.insert(std::make_pair("id", picojson::value(ToUTF8(it->GetId()))));
		elem.insert(std::make_pair("name", picojson::value(ToUTF8(it->GetName()))));
		elem.insert(std::make_pair("is_default", picojson::value(it->IsDefault())));

		deviceListObj.push_back(picojson::value(elem));
	}
	Serialize(buf, len, picojson::value(deviceListObj));
}

// Called from C#.
extern "C" __declspec(dllexport)
void ReleaseBuffer(wchar_t *buf) {
	delete[] buf;
}