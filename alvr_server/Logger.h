#pragma once

#include <iostream>
#include <fstream>

void OpenLog(const char *fileName);

void Log(const char *pFormat, ...);

void FatalLog(const char *format, ...);