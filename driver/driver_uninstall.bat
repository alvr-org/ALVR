@echo off
IF EXIST "C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\" (
"C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrpathreg.exe" removedriver "%~dp0
mshta "javascript:var sh=new ActiveXObject( 'WScript.Shell' ); sh.Popup( 'Driver Removed', 10, 'Title!', 64 );close()"
) ELSE (
mshta "javascript:var sh=new ActiveXObject( 'WScript.Shell' ); sh.Popup( 'SteamVR not located in C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR - Removal Failed', 10, 'Title!', 64 );close()"
)
