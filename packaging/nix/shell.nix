{ pkgs ? import <nixpkgs> { } }:

with pkgs;

let
  lunarg = pkgs.vulkan-tools-lunarg.overrideAttrs (oldAttrs: rec {
    patches = [
      (fetchurl {
        url =
          "https://gist.githubusercontent.com/ckiee/038809f55f658595107b2da41acff298/raw/6d8d0a91bfd335a25e88cc76eec5c22bf1ece611/vulkantools-log.patch";
        sha256 = "14gji272r53pykaadkh6rswlzwhh9iqsy1y4q0gdp8ai4ycqd129";
      })
    ];
  });
in mkShell {
  stdenv = pkgs.clangStdenv;
  nativeBuildInputs = [ cmake pkg-config ];
  buildInputs = [
    binutils-unwrapped
    alsaLib
    openssl
    glib
    (ffmpeg-full.override { samba = null; })
    cairo
    pango
    atk
    gdk-pixbuf
    gtk3
    clang
    lunarg
    vulkan-headers
    vulkan-loader
    vulkan-validation-layers
    xorg.libX11
    xorg.libXrandr
    libunwind
    python3 # for the xcb crate
    libxkbcommon
    jack2
    bear
  ];

  VK_LAYER_PATH = "${lunarg}/etc/vulkan/explicit_layer.d:${vulkan-validation-layers}";
  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  RUST_ANDROID_GRADLE_PYTHON_COMMAND = "${pkgs.python3Minimal}/bin/python3";
  shellHook = ''
    export PATH=$(pwd)/android:$PATH
  '';
  LD_LIBRARY_PATH = lib.makeLibraryPath [
    libGL
    libxkbcommon
    wayland
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
  ];

}
