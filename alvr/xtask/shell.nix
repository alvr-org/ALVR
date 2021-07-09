{ pkgs ? import <nixpkgs> { } }:

with pkgs;
mkShell {
  stdenv = pkgs.clangStdenv;
  nativeBuildInputs = [ cmake pkg-config ];
  buildInputs = [
    binutils-unwrapped
    alsaLib
    openssl
    glib
    (enableDebugging (ffmpeg-full.override { nonfreeLicensing = true; }))
    cairo
    pango
    atk
    gdk-pixbuf
    gtk3
    clang
    (pkgs.vulkan-tools-lunarg.overrideAttrs (oldAttrs: rec {
      patches = [
        (fetchurl {
          url =
            "https://gist.githubusercontent.com/ronthecookie/038809f55f658595107b2da41acff298/raw/6d8d0a91bfd335a25e88cc76eec5c22bf1ece611/vulkantools-log.patch";
          sha256 = "14gji272r53pykaadkh6rswlzwhh9iqsy1y4q0gdp8ai4ycqd129";
        })
      ];
    }))
    vulkan-headers
    (enableDebugging vulkan-loader)
    vulkan-validation-layers
    xorg.libX11
    xorg.libXrandr
    libunwind
  ];

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  RUST_ANDROID_GRADLE_PYTHON_COMMAND = "${pkgs.python3Minimal}/bin/python3";
  shellHook = ''
    export PATH=$(pwd)/alvr/client/android:$PATH
  '';
}
