# ALVR v14 and Above

Here are explained two methods to connect PC and headset remotely, port-forwarding and ZeroTier. The primary purpose of this is connecting the headset to a Cloud PC (like ShadowPC).

## Port-forwarding
Port-forwarding allows to connect devices that are behind different NATs, i.e. local networks. You need to have administrator access to your router. This method has the best streaming performance.

**IMPORTANT**: ALVR does not use end-to-end encryption of the stream data. By using this method you need to be aware that the connection is vulnerable to "Man In The Middle" attacks.

1. Take note of the public IP of your headset. You can use the online tool [WhatIsMyIP](https://www.whatismyip.com/).
2. Inside your router web interface or app, add a port-forwarding rule for your headset. You need to specify the ports 9943 and 9944 for both TCP and UDP.
3. Connect to the remote PC and open ALVR. In the Connection tab press `Add client manually`. Fill in the fields with a name for your headset (you can use the name you want), the hostname (you can read it in the welcome screen in your headset when you open the ALVR app), the remote IP of the headset (that is the IP you got on step 1.) and then press `Add client`.

You can now use ALVR to connect to your remote PC.

**Note**: The public IP can change often. Every time you want to use ALVR you need to check that your current public IP is the same as the last time. If the IP changed, you can update it using the "Configure client" interface, accessed with the `Configure` button next to your headset name on the server.

## ZeroTier

[ZeroTier](https://www.zerotier.com/) is a tunneling software that makes remote devices connect to each other as if they are in the same local network.

Comparing this to the port-forwarding method:

Pros:
* Does not require access to the router interface.
* You don't need to update the public IP often on the server.
* The connection in encrypted.

Cons: 
* The streaming performance is worse. You may experience more glitches and loss of quality in the image and audio.

### Requirements
- [ZeroTier](https://www.zerotier.com/) for your PC
- ZeroTier APK for your Quest (you can find it online)
- SideQuest or some other method to install the ZeroTier APK onto your headset

### Installation
Use the "Install APK" function of SideQuest to install the ZeroTier APK to your Quest, and also download and install ZeroTier on your PC. After you've installed ZeroTier, follow Zerotier's official [Getting Started](https://zerotier.atlassian.net/wiki/spaces/SD/pages/8454145/Getting+Started+with+ZeroTier) guide to setup a network for ALVR. Join the network on both the Quest and the PC. On the Quest, make sure that the network is enabled by switching on the slider on the network in the list in the ZeroTier app (you may be prompted to allow ZeroTier to create a VPN connection). 

After both your PC and your Quest are connected to the same ZeroTier network, we'll need to manually add your quest to the ALVR dashboard. To do so, we'll need to find your Quest's ZeroTier IP. There are two ways to do this. 
- Go the the ZeroTier network page, find your quest under "Members", and copy the managed IP from there
- Or, in the ZeroTier app on your quest, click on the network you created. The IP is under the "Managed IPs" section at the bottom.

The IP should look something like this `192.168.143.195`. If there's a `/` at the end with a couple numbers following it, remove them along with the slash. 

Next, we'll need to add the Quest to the ALVR dashboard. On your headset, launch ALVR. The on the ALVR dashboard on your PC, click the "Add Client Manually" button, provide a name and hostname (You can get this from the "trust" screen of ALVR on your Quest), then put in the IP address that we got from ZeroTier. 

At this point, you should be ready to go. Have fun in VR!

### Troubleshooting
- If you can't get your Quest to connect to ALVR, and are stuck on the "Trust" screen, try to ping your Quest's managed IP address (the one we got earlier). If it says "no route to host" or something similar, your Quest can't see your PC. Try running through the steps above to make sure you didn't miss anything. 

## Tailscale

An alternative to ZeroTier with practically the same setup procedure. This could have better latency, depending on your distance to the datacenter.
https://tailscale.com/

# ALVR v11 and Below

ALVR version Experimental v7 or newer is recommended for this configuration.

This configuration is **NOT** supported in ALVR v12. The latest release to still support this is v11.

To run ALVR client and ALVR server on separate networks (broadcast domains) the following things must be done: 
1. UDP ports 9943 and 9944 of ALVR server must be accessible from Oculus Quest device (i.e. firewall openings must be made to allow Oculus Quest to connect to ALVR server UDP ports 9943 and 9944). 
1. Oculus Quest must be connected to computer and command-line `adb shell am startservice -n "com.polygraphene.alvr/.ChangeSettings" --es "targetServers" "10.10.10.10"` must be run in Command Prompt to specify IP address of ALVR server (`10.10.10.10` must be substituted with IP address of ALVR server; the long line is a single command-line).
1. Next time when ALVR client will be started it should try to connect to the specified ALVR server. ALVR server should display the client in _Server_ tab (the same way local-network clients are displayed).


ALVR does **NOT** provide any kind of tunnel, NAT traversal etc. UDP ports 9943 and 9944 of ALVR server (VR gaming PC) must be accessible from ALVR client (Oculus Quest) otherwise this won't work. 


**Important notes on security!**
* ALVR protocol does not have any encryption or authentication (apart from ALVR client IP address shown in ALVR server and the requirement to click _Connect_ on ALVR server). 
* It is recommended to run ALVR via encrypted tunnel (VPN) over the internet. In case VPN is not an option, access to ALVR server (UDP ports 9943 and 9944) should be restricted by Windows Firewall (only connections from known IP addresses of ALVR clients should be allowed) and ALVR server should not be left running unattended. 
* **Warning!** SteamVR allows to control desktop from VR headset (i.e. a **malicious ALVR client could take over the PC**). 
* As the license states ALVR IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND (see the file `LICENSE` in this GitHub repository for legal text/definition). You are on your own (especially if you run ALVR over the Internet without VPN).