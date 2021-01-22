#pragma once

#include <string>

class Exception : public std::exception {
public:
	Exception(std::wstring what)
		: m_what(what) {
	}
	Exception() {
	}
private:
	std::wstring m_what;
};

Exception FormatExceptionV(const wchar_t *format, va_list args);
Exception FormatExceptionV(const char *format, va_list args);
Exception FormatException(const char *format, ...);