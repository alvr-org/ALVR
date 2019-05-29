@echo off

echo Adding firewall rules for ALVR
echo[

set PROGRAM=%~dp0ALVR.exe
netsh advfirewall firewall add rule name="ALVR Launcher" dir=in program="%PROGRAM%" action=allow
if not [%ERRORLEVEL%] == [0] goto err
netsh advfirewall firewall add rule name="SteamVR ALVR vrserver" dir=in program="C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win64\vrserver.exe" action=allow
netsh advfirewall firewall add rule name="SteamVR ALVR vrserver" dir=in program="C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win32\vrserver.exe" action=allow

if [%1] == [/s] exit
pause
exit

:err
echo Please run as administrator.
if [%1] == [/s] exit
pause
