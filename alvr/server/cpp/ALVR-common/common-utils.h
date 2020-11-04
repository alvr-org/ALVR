#pragma once

#include <string>
#include <locale>
#include <codecvt>

std::wstring ToWstring(const std::string &src);
std::string ToUTF8(const std::wstring &src);