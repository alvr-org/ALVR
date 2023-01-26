# PC
- A high-end PC is a requirement; ALVR is not a cheap alternative to a PCVR HMD
- ALVR resolution configuration and SteamVR multi-sampling may be used to influence quality in favor of performance or vice-versa
- Frequent dropped frames can cause a poor experience on ALVR; this can be verified using a tool such as [OVR Advanced Settings](https://github.com/OpenVR-Advanced-Settings/OpenVR-AdvancedSettings)
- Higher bit-rates will cause higher latency
- Ensure all relevant software is up to date; especially graphics and network drivers
- A good starting point is 100% resolution and 30mbit- 200kb buffer settings. In this config it should be butter smooth with almost no lag or packet loss; packet loss seen at this point is likely a result of network issues

# Network
- A wired connection from the PC to the network is **strongly recommended**
- A modern mid to high-end router and / or access point supporting at least 802.11ac (ideally 802.11ax) with regularly updated firmware is recommended

## Wireless
### General Wi-Fi configuration and best practices
- Any device that can be wired should be; each wireless device slows down the overall wireless network
- Devices should have the fewest obstructions and be as close to the access point or router as possible
- Any other wireless networks (ex: a printer's default wireless network) should be disabled; each network slows others down
- Any devices that do not need high speeds but support them (ex: a thermostat) should use 2.4GHz; often middle and higher end access points and routers support methods to "force" clients to use 2.4GHz, and some can even perform this automatically based on signal strength and connection speed
- Only Wi-Fi revisions which are necessary should be enabled; older standards such as 802.11b, 802.11g, and to a lesser extent, 802.11n, will slow down all clients
- Devices that require high speeds should use:
    - 5GHz only
    - The newest WiFi specifications (802.11AX, followed by 802.11AC)
    - In most environments, the largest channel width possible (160MHz for 802.11AX, 80MHz in practice for 802.11AC) (**note: some vendors do not set this to the maximum by default**)
    - The lowest utilization, followed by the lowest channel number (sub-frequency) possible
- **Manually selecting channels should only be done in places with extreme noise, or on older, lower quality, or ISP provided access points or routers**; modern mid to high-end routers and access points should optimize their channels fairly well, and as a result of other routers and clients "channel hopping", static settings are often less optimal
- If a specific WiFi channel range is absolutely necessary, use a WiFi scanning tool on a phone or PC to determine the least used channels; mid to high-end access points and routers may provide an interface for this as well, however, this sometimes causes a disconnect when scanning
- **Manually selecting Wi-Fi signal strength should only be done in places with extreme noise**; modern routers and access points do this well, and it is a complex task
- If a specific transmit power is necessary, keep in mind that stronger is not always better; as transmit power increases, distortion may increase (leading to *lower* speeds), battery life of clients may increase (due to the higher power requested by the access point or router), and issues with sticky clients (devices which stay connected to wifi even with bad signal) may appear
- If you have a significant number of devices, some routers and access points support features such as airtime fairness, which help to limit the amount of airtime slower clients take, improving the performance of higher speed clients

### What to keep in mind when configuring a wireless network and devices
- All devices on the same frequency impact each other (**including other WiFi networks on the same channel**) because only one device can transmit or receive data at a time, meaning that:
    - If one device utilizes WiFi heavily it will impact the latency and throughput of all other clients
    - If a slow device is connected, it can still take a significant amount of "airtime" (time for that dedicated client to transmit / receive data to the access point or router), even though it does so at a slower rate than other clients
    - Each connected device requires additional time, regardless of whether it is actively in use (and often devices send small amounts of data when idle for things such as NTP and DHCP)
- Wi-Fi is [half duplex](https://en.wikipedia.org/wiki/Duplex_(telecommunications)#Half_duplex) by nature of it being radio frequency, meaning data can only ever be transmitted **or** received on the same frequency, not both at the same time; twisted pair (copper ethernet cable) is full duplex
- Wireless frequency bands (ex: 2.4GHz, 5GHz) have separate channels that can be statically assigned if needed, but **these are not mutually exclusive, meaning the channels overlap significantly and interfere with each other**
- Different regions of the world support different channels (sub-frequencies); devices sold in these regions are generally locked to those channels (ex: in the US, 2.4GHz channels 12 - 13 are low power only, and channel 14 is military and EMS use only)
- Different wireless devices support different frequencies, standards, speeds, and features; using these to your advantage is key to getting best performance

## Routing, Switching, Firewalling, And General Info
- Ideally client and server should live on the same logical (layer 2) network and subnet; this allows for no routing overhead, and the correct function of client discovery via mDNS
- Twisted pair (normal copper ethernet cables) should never be run alongside power cables; this can cause signal noise and result in frame loss and lowered auto-negotiation speeds
- High quality CAT5E or higher (ideally CAT6A or CAT7) cabling should be used for modern networks
- In some cases firewall, anti-virus, malware, or EDR (enhanced detection and response) software may interfere with network traffic; Windows Defender and Sophos Endpoint Protection are reported to work without issue
- Pause frames should be disabled where possible, as these introduce additional latency and buffering

***

Someone did a few blog-posts on some of the points:
https://imaginevr.home.blog/author/imaginevrresearch/

Some points came from [FingrMastr](https://github.com/FingrMastr)

