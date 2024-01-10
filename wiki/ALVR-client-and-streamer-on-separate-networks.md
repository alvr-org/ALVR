# ALVR client and streamer on separate networks

# ALVR v14 and Above

Here are explained two methods to connect PC and headset remotely, port-forwarding and ZeroTier. The primary purpose of this is connecting the headset to a Cloud PC (like ShadowPC).

## Port-forwarding

Port-forwarding allows to connect devices that are behind different NATs, i.e. local networks. You need to have administrator access to your router. This method has the best streaming performance.

**IMPORTANT**: ALVR does not use end-to-end encryption of the stream data. By using this method you need to be aware that the connection is vulnerable to "Man In The Middle" attacks.

1. Take note of the public IP of your headset. You can use the online tool [WhatIsMyIP](https://www.whatismyip.com/).
2. Inside your router web interface or app, add a port-forwarding rule for your headset. You need to specify the ports 9943 and 9944 for both TCP and UDP.
3. Connect to the remote PC and open ALVR. In the Connection tab press `Add client manually`. Fill in the fields with a name for your headset (you can use the name you want), the hostname (you can read it in the welcome screen in your headset when you open the ALVR app), the remote IP of the headset (that is the IP you got on step 1.) and then press `Add client`.

You can now use ALVR to connect to your remote PC.

**Note**: The public IP can change often. Every time you want to use ALVR you need to check that your current public IP is the same as the last time. If the IP changed, you can update it using the "Configure client" interface, accessed with the `Configure` button next to your headset name on the streamer.

## ZeroTier

[ZeroTier](https://www.zerotier.com/) is a tunneling software that makes remote devices connect to each other as if they are in the same local network.

Comparing this to the port-forwarding method:

Pros:

* Does not require access to the router interface.
* You don't need to update the public IP often on the streamer.
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

## n2n

[n2n](https://github.com/ntop/n2n) is another P2P VPN solution, just like ZeroTier. You need to run _supernode_ on a server with an IP and port publicly accessible on internet (or at least your PC and Quest can access to it), and run _edge_ node on your PC and Quest.

Its pros and cons are similar to ZeroTier, but it's self-hosted and open-source if you care about privacy, though instead you need some knowledge about networking and server deploying.

### Requirements

- Compile [n2n](https://github.com/ntop/n2n) from source
  - Or you can grab pre-built binaries from [here](https://github.com/lucktu/n2n) directly, compiled by lucktu.
  - Some Linux distribution may have n2n, but be sure you're using the same version. Since the source code is v3, the following steps will also use v3 in the example below.
- [TAP-Windows driver](https://community.openvpn.net/openvpn/wiki/GettingTapWindows) or [OpenVPN](https://openvpn.net/community/) (includes TAP-Windows) if you're using Windows PC
- [hin2n](https://github.com/switch-iot/hin2n) APK
- A server with public IP and allow public ports
- SideQuest or some other method to install the hin2n APK onto your headset

### Installation

We're going to use n2n v3, and set the port of _supernode_ to `1234` as the example. You can change `1234` to any port, but below `1024` requires root.

- Open port `1234` on your server's firewall (usually `iptables`, if you don't know what to do, ask Google).
- Upload _supernode_ binary to your server, run `./supernode -p 1234`.
- Install TAP-Windows driver or OpenVPN on your PC if you're using Windows.
- Upload _edge_ binary to your PC, run `./edge -c [network-name] -k [secret-password] -a 192.168.100.1 -l [your-server-ip]:1234` to connect to the _supernode_, assign the IP `192.168.100.1` to the PC, and use the password you provided for data encryption. 
- Once you see `[OK] edge <<< ================ >>> supernode`, your PC is done, or you need to follow the error logs to see what's wrong.
- Install _hin2n_ on your Quest and open it, click the plus button at the top-right corner to add a new configuration and assign `192.168.100.2` to your Quest:
  - N2N version: v3
  - Supernode: `[your-server-ip]:1234`
  - Community: `[network-name]`
  - Encrypt key: `[secret-password]`
  - IP address: `192.168.100.2`
  - Subnet mask: `255.255.255.0`
- Click "Current Setting" under the connect button, select the configuration we created just now, then click the connect button. If you're asked to allow hin2n to create a VPN connection, allow it.
- Once you see `[OK] edge <<< ================ >>> supernode`, your Quest is done.
- Open ALVR on your headset, record the hostname it shows.
- Open ALVR dashboard on your PC, click "Add client manually" button, put the hostname you just recorded, and set IP address to `192.168.100.2` which is assigned to Quest just now.
- Once it's done, you're all set.

### Troubleshooting

- Make sure you can access to the supernode, your supernode should be run on a server with public IP, and you can ping it on your PC.
- If your Quest cannot connect to ALVR dashboard, ping the IP you assigned to Quest in hin2n. If it fails, try redoing the setup steps.
- If the edge binary or hin2n says the IP has already been assigned and not released by supernode, you can set IP address to another one in the same subnet like `192.168.100.123` to reassign a new IP to the device.
- If you're playing over WAN, you may see more glitches, higher stream latency, or lagger response with TCP. Use adaptive bitrate and UDP may improve your experience.

**Important notes on security!**

* ALVR protocol does not have any encryption or authentication (apart from ALVR client IP address shown in ALVR streamer and the requirement to click _Connect_ on ALVR streamer). 
* It is recommended to run ALVR via encrypted tunnel (VPN) over the internet. In case VPN is not an option, access to ALVR streamer (UDP ports 9943 and 9944) should be restricted by Windows Firewall (only connections from known IP addresses of ALVR clients should be allowed) and ALVR streamer should not be left running unattended. 
* **Warning!** SteamVR allows to control desktop from VR headset (i.e. a **malicious ALVR client could take over the PC**). 
* As the license states ALVR IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND (see the file `LICENSE` in this GitHub repository for legal text/definition). You are on your own (especially if you run ALVR over the Internet without VPN).
