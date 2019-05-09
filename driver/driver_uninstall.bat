@echo off
IF EXIST "C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\" (
"C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrpathreg.exe" removedriver "%~dp0
if "%1"=="/s" goto :end
echo "Driver Removed"
) ELSE (
if "%1"=="/s" goto :end
echo "SteamVR not located in C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR - Removal Failed"
)
pause
:end
