# This is for Nix(OS) users
{ pkgs ? import <nixpkgs> { } }:

with pkgs;

mkShell {
  nativeBuildInputs = [ cmake pkg-config ];
  buildInputs = [ xorg.libX11 xorg.libXrandr vulkan-headers libunwind ];
}
