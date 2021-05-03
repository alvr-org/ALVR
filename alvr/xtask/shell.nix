{ pkgs ? import <nixpkgs> { } }:

with pkgs;
let
  local = (import (pkgs.fetchFromGitHub {
    owner = "ronthecookie";
    repo = "nixpkgs";
    rev = "9704ac164c630a4e9f50a3448aa165480a78f186";
    sha256 = "1a9zwiw3f3j4ssk86yfw055kbv25vf9pyjf28gkwc15xwfb3jp6v";
    fetchSubmodules = true;
  })) { };
in mkShell {
  stdenv = pkgs.clangStdenv;
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    binutils-unwrapped
    alsaLib
    openssl
    glib
    (enableDebugging (local.ffmpeg-full.override { nonfreeLicensing = true; }))
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
  ];
  shellHook = ''
    export LIBCLANG_PATH="${pkgs.llvmPackages.libclang}/lib"
  '';
}
