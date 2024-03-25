# Changelog

### v20.6.1

* Add AV1 support, only for Linux/VAAPI, with 10bits support (by @wsippel #1955 #1964)
* Fix image corruption on h264/VAAPI (by @galister / @nowrep #1956)

## v20.6.0

* Add tongue tracking for Quest Pro (by @zarik5)
  * This is a breaking change in the protocol, but only affects Quest Pro users.
  * Only VRCFT ALVR module v1.2.0 and up is supported
* Add Quest 3 emulation mode + icons for SteamVR HUD (by @Goodguy140 #1926)
* Add Type of Service (ToS) socket settings, tested only on Linux (by @Vixea #1946)
* Add software decoding option and fallback (by @20kdc #1933)
* Add Baseline encoding option for h264 (by @20kdc #1932)
* Fix ADB connection (by @The-personified-devil #1942)
* Fix rare bug preventing reconnections on wifi (by @zarik5)

## v20.5.0

* Fix Vulkan layer GPU loading (by @nairaner #1847)
* Fix dynamic bitrate for VAAPI (by @nowrep #1863)
* Add notification tips (by @zarik5 #1865)
* Fix hand tracking for Lynx R1 (by @technobaboo #1874)
* Various wiki updates
* Fix battery update during streming (by @zarik5)
* Fix playspace recentering delay (by @zarik5)
* Support eye tracking for Pico 4 Pro (by @zarik5 @Meister1593 #1897)
* Add desktop file for Flatpak (by @Vixea #1906)
* Install audio dependencies from the setup wizard (by @Meister1593 #1893, @zarik5)
* Significantly reduce latency with NVENC on Linux (by @nowrep @Xaphiosis #1911)
* Fix SteamVR hanging when restarting on Linux (by @Vixea @zarik5)
* Other dashboard updates

### v20.4.3

* Fix dashboard crash on Windows
* Fix settings reset bug when upgrading (session extrapolation failed)

### v20.4.2

* Fix YVR crash because of invalid controller bindings

### v20.4.1

* Fix inverted `Enable skeleton` switch
* Add `Only touch` gestures option

## v20.4.0

* Full hand tracking gestures support, with joystick (by @barnabwhy #1794)
* Fully remappable controller buttons (by @zarik5 #1817)
* Custom controller profile (by @zarik5)
* Fix Nvidia support on Linux (by @Killrmemz #1830)

### v20.3.1

* Fix some controller buttons not working
* Fix changing controller emulation profile not triggering a SteamVR restart
* Add back Rift S controller emulation profile

## v20.3.0

* Add Lynx R1 headset support (by @zarik5 #1823)
  * Currently there is an issue with hand tracking which is being investigated
* Make settings sections collapsible (by @zarik5)
* Other UI tweaks (by @zarik5)
* *Actually* fix controller freeze (by @zarik5)
* Fix Pico controller buttons (by @zarik5 @galister @Meister1593 #1820)
* Fix bitrate hikes when "Adapt to framerate" is enabled (by @zarik5)
* Fix Nvenc encoder on Linux (by @Killrmemz #1824)
* Timeout connection if lingering (by @zarik5)
* Fix warmstart crash on client (by @ShootingKing-AM #1813)

### v20.2.1

* Fix VRCFaceTracking mode panicing.
* (Potential) Fix for dashboard crash on Wayland.

## v20.2.0

* Add Flatpak build (by @CharlieQLe #1683 #1724 #1735 #1742, @Meister1593 #1769)
* Finish VRCFaceTracking support (by @zarik5)
  * You can download the ALVR Module from the VRCFaceTracking app itself.
  * Only supports the Quest Pro at the moment.
* New more performant sockets implementation (by @zarik5)
  * Zero copy + zero allocations, and provides better packet prioritization.
* Avoid controller freezing during high latency (by @zarik5)
* Add message popups on Linux (disabled on the appimage build) (by @zarik5 #1711)
* Show backtrace on unhandled exceptions (Windows only) (by @zarik5)
  * Previously these would make SteamVR hard crash without any useful log
* Optionally show full backtraces for logs (by @zarik5)
* Add option to select client log level (by zarik5)
* Make Log tab stick to bottom (by @zarik5)
* Encoder fixes on Linux (by @nowrep #1751 #1753 #1767 #1768 #1796, @Vixea #1805)
* Use Constant bitrate mode by default
* Support rolling video recording (by @zarik5)
* Fix OpenGL crash on the client (by @ShootingKing-AM #1801)
* Fix white dashboard bug on Linux (by @zarik5)

## v20.1.0

* Fix firewall rules on Windows (by @zarik5)
* Fix firewall rules on linux for the tar.gz (by @Vixea #1675)
* Add bitrate graph (by @zarik5 #1689)
* Add encoder latency limiter (by @zarik5 #1678)
* Fix network latency limiter (by @zarik5)
* Fix image corruption on AMD (by @zarik5 #1681)
* Fix dashboard audio dropdowns on Linux (by @zarik5)
* Add connection status for clients (by @zarik5 #1688)
* Fix HMD plugged status (by @zarik5)
* Fix crash on some Unreal Engine 5 games (by @deiteris #1685)
* Add option to disable game render optimization (by @zarik5)
* Add separate history size for bitrate (by @zarik5)

# v20.0.0

* New OpenXR-based client, add support for Vive Focus 3/XR Elite, Pico 4/Neo 3 and YVR 1/2. Worked on by:
  * @zarik5 #1321
  * @galister #1321, #1442
  * @deiteris #1434, #1439, #1445
* New egui (OpenGL) dashboard
  * The launcher is replaced by the new dashboard executable.
  * by @Kirottu #1247 #1274, @zarik5, @m00nwtchr #1292, @TheComputerNerd88 #1574 #1575 #1576 1582
* Add position and rotation recentering modes (by @zarik5 #1321)
  * Defaults to local floor and local yaw.
* Add support for eye and face tracking (by @zarik5 #1577)
  * Currently supporting VRChat Eye OSC, VRCFaceTracking support coming soon
* Reduce game rendering latency (by @zarik5)
* Apply some settings in real-time (by @zarik5 #1635)
* New more consistent controller prediction algorithm (by @zarik5 #1561)
* Controller input fixes (by @zarik5 #1560)
* Soft-toggle controllers at runtime (by @galister #1600)
* New wiki hosted in the main git tree (by @m00nwtchr #1309)
* Send client log to streamer (by @zarik5)
* Encoder improvements (by @nowrep #1562 #1565 #1568, @deiteris #1403 #1422 #1400 #1402, @zarik5 #1564)
* Remove Forward Error Correction (by @zarik5: #1384, #1389; @deiteris: #1386, #1387, #1390)
* Unified code for NAL parsing (by @deiteris #1403, #1422, #1400, #1402)
* Some tweaks for alvr_client_core compatibility (by @ShootingKing-AM #1580 #1578 #1586 #1624 #1621)
* Fix server build with clang (by @nowrep)

### v19.1.1

* Relax discovery protocol for future ALVR versions

## v19.1.0

* Encoder improvements and new Linux server compositor:
  * @deiteris #1227, #1252, #1281, #1287, #1302, #1304, #1318, #1331, #1336, #1368, #1393, #1367, #1397
  * @Vixea #1227, #1254, #1412
  * @nowrep #1251, #1261, #1253, #1267, #1264, #1266, #1273, #1272, #1277, #1280, #1279, #1278, #1282, #1265, #1294, #1295, #1312, #1314, #1316, #1325, #1328, #1326, #1330, #1334, #1338, #1329, #1346, #1350, #1357, #1352, #1348, #1365, #1349, #1361, #1370, #1372, #1393
  * @m00nwtchr #1347
  * @galister #1428, #1429
* Controller fixes (by @Timocop #1236 #1241)
* Vulkan layer fixes (by @nowrep #1291, #1293, #1324, #1339, #1376)
* Show client IP in the headset and dashboard (by @zarik5)
* Disable Wi-Fi scans (by @zarik5)
* Reduce lag after loading screens (by @zarik5)
* Fix server debug builds (by @nowrep #1288)
* Add trigger/grip threshold (by @sctanf)
* Don't spam stdout on Linux (by @nowrep #1317)
* Fix recentering on Linux (by @nowrep #1353)
