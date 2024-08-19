{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell.override { stdenv = pkgs.cudaPackages.backendStdenv; } {
  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = [
    pkgs.cudaPackages.cuda_nvcc
    pkgs.cudaPackages.cuda_cudart
    pkgs.cudaPackages.libnpp

    pkgs.nasm

    pkgs.vulkan-headers
    pkgs.libva
    pkgs.libdrm
  ];

  shellHook = ''
    unset AS
  '';
}
