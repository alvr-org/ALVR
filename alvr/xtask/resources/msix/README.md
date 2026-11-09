# Installing ALVR from the MSIX package

This package installs two apps:

- **ALVR Dashboard**, the ALVR streamer with its SteamVR driver.
- **ALVR Client Emulator** (orange icon), a desktop stand-in for a headset that connects to the
  streamer on the same PC.

These are test builds for our dev group. They are signed with our own certificate, so each PC has
to trust that certificate once before Windows will install the package.

## What you need

- Windows 10 version 2004 or newer, or Windows 11, 64-bit.
- Steam with SteamVR installed.
- Admin rights on the PC, once, to trust the certificate.

## Download

From the release's **Assets**, download both files:

- `alvr_windows.msix`: the package.
- `ALVR-signing-certificate.cer`: the certificate to trust. You only need it the first time, or if
  we tell you the certificate has changed.

## 1. Trust the certificate (once per PC)

Open PowerShell **as administrator** (right-click PowerShell or Terminal, then "Run as
administrator"), go to the folder you downloaded to, and run:

```powershell
Import-Certificate -FilePath .\ALVR-signing-certificate.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople
```

It prints one line with the subject `CN=Axodox`.

<details>
<summary>Without PowerShell</summary>

1. Double-click `ALVR-signing-certificate.cer` and click **Install Certificate…**.
2. Choose **Local Machine** and click **Next** (this asks for admin rights).
3. Choose **Place all certificates in the following store**, click **Browse…** and pick
   **Trusted People**.
4. Click **Next**, then **Finish**.

</details>

## 2. Install

Double-click `alvr_windows.msix` and click **Install**.

If double-clicking does nothing (some PCs have the App Installer removed), run this in PowerShell
in the download folder instead. It doesn't need admin rights:

```powershell
Add-AppxPackage .\alvr_windows.msix
```

## 3. First start

1. Open **ALVR Dashboard** from the Start menu. The first time it starts after an install or an
   update, it copies its SteamVR driver to `%LOCALAPPDATA%\ALVR\streamer`. Settings and logs are kept
   there as well.
2. Go through the setup wizard. Click **Add firewall rules** when it asks, and accept the admin prompt.
3. Click **Launch SteamVR**. The dashboard registers its driver with SteamVR on the way.

To use the emulator instead of a headset:

1. With SteamVR running from the dashboard, open **ALVR Client Emulator** from the Start menu.
2. In the dashboard, click **Trust** next to the new device that appears.

The emulator keeps its settings in `%LOCALAPPDATA%\ALVR Client Emulator`. You can edit them there.

## Updating

1. Close SteamVR and the dashboard.
2. Download the new `alvr_windows.msix` and install it the same way. It installs over the old one
   and keeps your settings.

If SteamVR was still running, the dashboard can't replace the driver it has loaded and keeps
using the old one. Close SteamVR, then start the dashboard again to finish the update.

## Switching between this and a build from source

Only one ALVR driver can be registered with SteamVR at a time. The dashboard you start last
registers its own driver and removes the other one. Close SteamVR before switching between the two.

## Uninstalling

1. In the dashboard, open **Installation**, and under **Registered drivers** click **Remove** next to
   the `...\AppData\Local\ALVR\streamer` entry. Otherwise SteamVR keeps loading the driver after the
   app is gone.
2. Close SteamVR and the dashboard.
3. Uninstall **ALVR** from Settings → Apps → Installed apps.
4. Optionally, delete `%LOCALAPPDATA%\ALVR` and `%LOCALAPPDATA%\ALVR Client Emulator` to remove the
   settings too. Windows does not remove these folders when you uninstall.

## Troubleshooting

| Message or symptom | Fix |
|---|---|
| "The publisher certificate could not be verified" or error `0x800B0109` | The certificate is not trusted on this PC. Do step 1. It has to go into **Local Machine → Trusted People**, not Current User. |
| "A higher version of this package is already installed" | Uninstall ALVR first, then install. This happens after installing a build made on your own PC. |
| SteamVR doesn't show ALVR | Start SteamVR from the dashboard, not from Steam. Or click **Register ALVR driver** under **Installation**. |
| The emulator doesn't appear in the dashboard | Make sure SteamVR is running and no other emulator is open. Start the emulator again. |
| Something else | The logs are in `%LOCALAPPDATA%\ALVR\streamer` (`session_log.txt`, `crash_log.txt`). Send them to the group. |
