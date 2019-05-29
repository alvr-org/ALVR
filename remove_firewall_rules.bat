@echo off

echo Removing firewall rules for ALVR
echo[

netsh advfirewall firewall delete rule name="ALVR Launcher"
netsh advfirewall firewall delete rule name="SteamVR ALVR vrserver"

if [%1] == [/s] exit
pause

