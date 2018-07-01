@echo off
IF EXIST "C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\" (
"C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrpathreg.exe" removedriver "%~dp0
if "%1"=="/s" goto :end
mshta "javascript:var sh=new ActiveXObject( 'WScript.Shell' ); sh.Popup( 'Driver Removed', 10, 'Driver installer', 64 );close()"
) ELSE (
if "%1"=="/s" goto :end
mshta "javascript:var sh=new ActiveXObject( 'WScript.Shell' ); sh.Popup( 'SteamVR not located in C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR - Removal Failed', 10, 'Driver installer', 64 );close()"
)
:end
