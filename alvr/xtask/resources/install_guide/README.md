# Installing AIVR test builds

AIVR is our build of [ALVR](https://github.com/alvr-org/ALVR), which streams SteamVR games from a PC
to a standalone headset. It has two halves, and each release has both:

- **On the PC:** `aivr_windows.msix` installs two apps:
  - **AIVR Dashboard**, the ALVR streamer with its SteamVR driver.
  - **AIVR Client Emulator** (orange icon), a desktop stand-in for a headset that connects to the
    streamer on the same PC.
- **On the headset:** `aivr_client_android.apk` installs **AIVR**, the app that receives the
  stream on a Quest, Pico or other Android headset. It's our own build and installs next to the
  official ALVR app without replacing it.

You only need the headset part if you test with a real headset. For the emulator, the PC part is
enough.

**Always use the PC and headset files from the same release.** The two halves only connect when
their versions match.

These are test builds for our dev group. The PC package is signed with our own certificate, so each
PC has to trust that certificate once before Windows will install it.

## What you need

- Windows 10 version 2004 or newer, or Windows 11, 64-bit.
- Steam with SteamVR installed.
- Admin rights on the PC, once, to trust the certificate.
- For a headset: a USB cable for the headset, the PC and the headset on the same network, and
  [SideQuest](https://sidequestvr.com/setup-howto) or `adb` on the PC.

## Download

From the release's **Assets**, download:

- `aivr_windows.msix`: the PC package.
- `AIVR-signing-certificate.cer`: the certificate to trust. You only need it the first time, or if
  we tell you the certificate has changed.
- `aivr_client_android.apk`: the headset app, if you use a headset.

## Part 1: the PC

### 1. Trust the certificate (once per PC)

Open PowerShell **as administrator** (right-click PowerShell or Terminal, then "Run as
administrator"), go to the folder you downloaded to, and run:

```powershell
Import-Certificate -FilePath .\AIVR-signing-certificate.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople
```

It prints one line with the subject `CN=Axodox`.

<details>
<summary>Without PowerShell</summary>

1. Double-click `AIVR-signing-certificate.cer` and click **Install Certificate…**.
2. Choose **Local Machine** and click **Next** (this asks for admin rights).
3. Choose **Place all certificates in the following store**, click **Browse…** and pick
   **Trusted People**.
4. Click **Next**, then **Finish**.

</details>

### 2. Install

Double-click `aivr_windows.msix` and click **Install**.

If double-clicking does nothing (some PCs have the App Installer removed), run this in PowerShell
in the download folder instead. It doesn't need admin rights:

```powershell
Add-AppxPackage .\aivr_windows.msix
```

### 3. First start

1. Open **AIVR Dashboard** from the Start menu. The first time it starts after an install or an
   update, it copies its SteamVR driver to `%LOCALAPPDATA%\ALVR\streamer`. Settings and logs are kept
   there as well.
2. Go through the setup wizard. Click **Add firewall rules** when it asks, and accept the admin prompt.
3. Click **Launch SteamVR**. The dashboard registers its driver with SteamVR on the way.

To use the emulator instead of a headset:

1. With SteamVR running from the dashboard, open **AIVR Client Emulator** from the Start menu.
2. In the dashboard, click **Trust** next to the new device that appears.

The emulator keeps its settings in `%LOCALAPPDATA%\ALVR Client Emulator`. You can edit them there.

## Part 2: the headset

Apps that don't come from the headset's store have to be installed from the PC over USB
("sideloading"). That needs developer mode on the headset, once.

### 1. Turn on developer mode (once per headset)

**Quest:**

1. Your Meta account has to be a developer account. Sign in at
   [developers.meta.com](https://developers.meta.com/horizon/) and create an organization if you
   don't have one. Any name works.
2. In the **Meta Horizon** app on your phone, open **Devices**, choose the headset, then
   **Headset settings → Developer mode**, and turn it on.
3. Restart the headset.

**Pico:** in the headset, open **Settings → General → About** and click **Software version** seven
times. Then turn on **USB debugging** under **Settings → Developer**.

### 2. Install the app

Connect the headset to the PC with the USB cable. In the headset, allow **USB debugging** when it
asks, and tick "Always allow from this computer".

- **With SideQuest:** open SideQuest, wait for the dot at the top left to turn green, then drag
  `aivr_client_android.apk` onto the SideQuest window.
- **With adb:** run this in the download folder:
  ```powershell
  adb install -r .\aivr_client_android.apk
  ```

### 3. First start

1. On the headset, open the app library. On Quest, **AIVR** is under **Unknown Sources** in
   the filter at the top right of the library.
2. Start **AIVR** and allow the microphone when it asks.
3. On the PC, the headset now appears in the dashboard. Click **Trust**.
4. Click **Launch SteamVR** in the dashboard if SteamVR isn't running yet.

You can unplug the USB cable once the app is installed. The stream goes over the network.

## Updating

**PC:**

1. Close SteamVR and the dashboard.
2. Download the new `aivr_windows.msix` and install it the same way. It installs over the old one
   and keeps your settings.

If SteamVR was still running, the dashboard can't replace the driver it has loaded and keeps
using the old one. Close SteamVR, then start the dashboard again to finish the update.

and keeps its settings. Update both halves together.

## Switching between this and a build from source

Only one ALVR driver can be registered with SteamVR at a time. The dashboard you start last
registers its own driver and removes the other one. Close SteamVR before switching between the two.

## Uninstalling

**PC:**

1. In the dashboard, open **Installation**, and under **Registered drivers** click **Remove** next to
   the `...\AppData\Local\ALVR\streamer` entry. Otherwise SteamVR keeps loading the driver after the
   app is gone.
2. Close SteamVR and the dashboard.
3. Uninstall **AIVR** from Settings → Apps → Installed apps.
4. Optionally, delete `%LOCALAPPDATA%\ALVR` and `%LOCALAPPDATA%\ALVR Client Emulator` to remove the
   settings too. Windows does not remove these folders when you uninstall.

**Headset:** in the app library, open the **⋯** menu on **AIVR** and choose **Uninstall**.
Or, with the headset connected: `adb uninstall com.axodox.aivr`.

## Troubleshooting

| Message or symptom | Fix |
|---|---|
| "The publisher certificate could not be verified" or error `0x800B0109` | The certificate is not trusted on this PC. Do step 1 of part 1. It has to go into **Local Machine → Trusted People**, not Current User. |
| "A higher version of this package is already installed" | The package you are installing is older than the installed one, for example an earlier release, or a build someone made on their own PC after it. Uninstall AIVR from the PC first, then install. |
| SteamVR doesn't show AIVR | Start SteamVR from the dashboard, not from Steam. Or click **Register AIVR driver** under **Installation**. |
| The emulator doesn't appear in the dashboard | Make sure SteamVR is running and no other emulator is open. Start the emulator again. |
| The headset doesn't appear in the dashboard | Check that the PC and the headset are on the same network, and that the firewall rules were added (**Installation → Add firewall rules**). Check that both halves come from the same release. |
| `adb` or SideQuest doesn't see the headset | Developer mode is off, or USB debugging wasn't allowed in the headset. Plug the cable in again and look inside the headset for the prompt. |
| `INSTALL_FAILED_UPDATE_INCOMPATIBLE` when installing the APK | An AIVR build signed with a different key is installed. Uninstall it (`adb uninstall com.axodox.aivr`), then install. |
| Something else | The PC logs are in `%LOCALAPPDATA%\ALVR\streamer` (`session_log.txt`, `crash_log.txt`). Send them to the group. |
