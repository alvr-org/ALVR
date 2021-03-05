#include "common-utils.h"

#include <codecvt>
#include <locale>

std::wstring ToWstring(const std::string &src) {
	// TODO: src is really UTF-8?
	std::wstring_convert<std::codecvt_utf8_utf16<wchar_t>> converter;
	return converter.from_bytes(src);
}

std::string ToUTF8(const std::wstring &src) {
	std::wstring_convert<std::codecvt_utf8_utf16<wchar_t>> converter;
	return converter.to_bytes(src);
}
