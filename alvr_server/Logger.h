#pragma once

#include <iostream>
#include <fstream>

void OpenLog(const char *fileName);

void Log(const char *pFormat, ...);

void FatalLog(const char *format, ...);

class Exception : public std::exception {
public:
	Exception(std::string what)
		: m_what(what) {
	}

	virtual const char *what() {
		return m_what.c_str();
	}
private:
	const std::string m_what;
};

Exception MakeException(const char *format, ...);