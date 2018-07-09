#pragma once

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

inline Exception FormatException(const char *format, ...) {
	va_list args;
	va_start(args, format);
	char buf[10000];
	vsnprintf(buf, sizeof(buf), format, args);
	va_end(args);

	return Exception(buf);
}

inline Exception FormatExceptionV(const char *format, va_list args) {
	char buf[10000];
	vsnprintf(buf, sizeof(buf), format, args);

	return Exception(buf);
}