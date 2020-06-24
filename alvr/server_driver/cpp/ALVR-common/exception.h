#pragma once

#include <string>

class Exception : public std::exception {
public:
	Exception(std::string what)
		: m_what(what) {
	}
	Exception() {
	}

	virtual const char *what() {
		return m_what.c_str();
	}

	Exception& operator=(const Exception &src) {
		m_what = src.m_what;
		return *this;
	}
private:
	std::string m_what;
};

Exception FormatExceptionV(const char *format, va_list args);
Exception FormatException(const char *format, ...);