#pragma once

#include <string>

std::wstring ToWstring(const std::string &src);
std::string ToUTF8(const std::wstring &src);
