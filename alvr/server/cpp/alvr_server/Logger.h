#pragma once

#include <iostream>
#include <fstream>
#include "exception.h"
#include <wrl.h>

#include "bindings.h"

Exception MakeException(const char *format, ...);

void Error(const char *format, ...);
void Warn(const char *format, ...);
void Info(const char *format, ...);
void Debug(const char *format, ...);