# Changelog

## Unreleased

* New egui dashboard (by @Kirottu #1247 #1274, @m00nwtchr #1292, @zarik5 ___)
* Send client log to streamer (by @zarik5)
* New more consistent controller prediction algorithm (by @zarik5)
* New wiki hosted in the main git tree (by @m00nwtchr #1309)
* Remove Forward Error Correction (by @zarik5: #1384, #1389; @deiteris: #1386, #1387, #1390)
* Unified code for NAL parsing (by @deiteris #1403, #1422, #1400, #1402)
* New OpenXR-based client, support for Vive Focus 3 and Pico 4/Neo 3
  * @zarik5 #1321
  * @galister #1321, #1442
  * @deiteris #1434, #1439, #1445

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
