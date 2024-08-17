# Flatpak environment for building ALVR 

This is an experimental Flatpak build environment for ALVR! This is only intended for developers to use.

## Usage

1. Install the Flatpak dependencies 

```
flatpak install flathub org.flatpak.Builder org.freedesktop.Sdk//22.08 \
    org.freedesktop.Sdk.Extension.llvm16//22.08 \
    org.freedesktop.Sdk.Extension.rust-stable//22.08
```

2. Clone and enter this repository

```
git clone https://github.com/alvr-org/ALVR.git
cd ALVR
```

3. Build flatpak using convenience script

```
cd alvr/xtask/flatpak_builder
./build.sh
```

## Usage

Copy the binary out of the flatpak and use it on the host - or in the flatpak launcher. Copy it into the local steam flatpak folder.

Example: flatpak run --command=bash com.valvesoftware.Steam 
This will open a shell inside the flatpak environment. Both the builder and the launcher folders are present, so a test build can be copied. 
mkdir $HOME/.local/share/ALVR-Launcher/installations/testfolder
cp -r /app/utils/alvr_builder/alvr_streamer_linux/ $HOME/.local/share/ALVR-Launcher/installations/testfolder/ 

The new build should appear in the launcher - proceed as normal 

## Caveats

Development purposes only - mainly because nvcc on arch is annoying

If you get a "cannot open file or directory" from flatpak builder then just wait a few seconds and retry - some other process is still exiting from previous attempt