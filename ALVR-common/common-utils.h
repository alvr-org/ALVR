#pragma once

#include <string>
#include <locale>
#include <codecvt>

std::wstring ToWString(const std::string &src);
std::string ToString(const std::wstring& src);
std::string ToUTF8(const std::wstring &src);