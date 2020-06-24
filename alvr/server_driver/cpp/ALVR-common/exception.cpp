#include <stdarg.h>
#include <wchar.h>
#include "exception.h"
#include "common-utils.h"

Exception FormatExceptionV(const char *format, va_list args) {
	char buf[1024];
	vsprintf(buf, format, args);
	return Exception(buf);
}

Exception FormatException(const char *format, ...) {
	va_list args;
	va_start(args, format);
	Exception e = FormatExceptionV(format, args);
	va_end(args);

	return e;
}