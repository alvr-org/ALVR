@echo off
IF EXIST "C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\" (
"C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrpathreg.exe" adddriver "%~dp0
mshta "javascript:var sh=new ActiveXObject( 'WScript.Shell' ); sh.Popup( 'Driver Probably Installed', 10, 'Driver installer', 64 );close()"
) ELSE (
mshta "javascript:var sh=new ActiveXObject( 'WScript.Shell' ); sh.Popup( 'SteamVR not located in C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR - Installation Failed', 10, 'Driver installer', 64 );close()"
)
