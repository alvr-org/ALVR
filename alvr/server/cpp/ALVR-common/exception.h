#pragma once

#include <string>
#include <stdexcept>

class Exception : public std::exception {
public:
	Exception(std::string what)
		: m_what(what) {
	}
	Exception() {
	}

	const char *what() const noexcept override {
		return m_what.c_str();
	}
private:
	std::string m_what;
};

Exception FormatExceptionV(const char *format, va_list args);
Exception FormatException(const char *format, ...);
