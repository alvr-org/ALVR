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

## Caveats

Development purposes only - mainly because nvcc on arch is annoying

If you get a "cannot open file or directory" from flatpak builder then just wait a few seconds and retry - some other process is still exiting from previous attempt