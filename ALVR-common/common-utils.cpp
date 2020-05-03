#include "common-utils.h"
#include <Windows.h>


std::wstring ToWString(const std::string& src) {
	int bufferSize = MultiByteToWideChar(CP_ACP, 0, src.c_str(), -1, (wchar_t*)NULL, 0);
	wchar_t* bufWString = (wchar_t*)new wchar_t[bufferSize];
	MultiByteToWideChar(CP_ACP, 0, src.c_str(), -1, bufWString, bufferSize);
	std::wstring retWString(bufWString, bufWString + bufferSize - 1);
	delete[] bufWString;
	return retWString;
}

std::string ToString(const std::wstring& src) {
	int bufferSize = WideCharToMultiByte(CP_ACP, 0, src.c_str(), -1, (char*)NULL, 0, NULL, NULL);
	char* bufString = (char*)new char[bufferSize];
	WideCharToMultiByte(CP_ACP, 0, src.c_str(), -1, bufString, bufferSize, NULL, NULL);
	std::string retString(bufString, bufString + bufferSize - 1);
	delete[] bufString;
	return retString;
}

std::string ToUTF8(const std::wstring &src) {
	std::wstring_convert<std::codecvt_utf8_utf16<wchar_t>> converter;
	return converter.to_bytes(src);
}