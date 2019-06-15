#pragma once

#include <string>

class Exception : public std::exception {
public:
	Exception(std::wstring what)
		: mWhat(what) {
	}
	Exception() {
	}

	virtual const wchar_t *what() {
		return mWhat.c_str();
	}

	Exception& operator=(const Exception &src) {
		mWhat = src.mWhat;
		return *this;
	}
private:
	std::wstring mWhat;
};

Exception FormatExceptionV(const wchar_t *format, va_list args);
Exception FormatExceptionV(const char *format, va_list args);
Exception FormatException(const char *format, ...);