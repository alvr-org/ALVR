#pragma once

#include <string>

class Exception : public std::exception {
public:
	Exception(std::wstring what)
		: m_what(what) {
	}

	virtual const wchar_t *what() {
		return m_what.c_str();
	}
private:
	const std::wstring m_what;
};

Exception FormatExceptionV(const wchar_t *format, va_list args);
Exception FormatExceptionV(const char *format, va_list args);
Exception FormatException(const char *format, ...);