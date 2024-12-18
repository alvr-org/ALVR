{
  description = "Stream VR games from your PC to your headset via Wi-Fi";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";  # TODO use upstream nix utils
    openvr = {
      url = "github:ValveSoftware/openvr";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, openvr }: flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
          lib = pkgs.lib;
          buildPackages = with pkgs; [rustc cargo pkg-config jack1 alsa-lib pipewire.dev openssl clang libclang glibc llvmPackages.libclang ffmpeg.dev];
          dependencyPackages = with pkgs; [nasm vulkan-headers libva libdrm pipewire openssl ffmpeg];
          nvidiaPackages = with pkgs.cudaPackages; [cuda_nvcc cuda_cudart libnpp];
          devShell = {stdenv, nvidia}: pkgs.mkShell {
            buildInputs = buildPackages ++ dependencyPackages ++ (lib.optionals nvidia nvidiaPackages);
          };
      in
        {
          packages.default = pkgs.rustPlatform.buildRustPackage rec {
            pname = "alvr";
            BINDGEN_EXTRA_CLANG_ARGS = [
              ''-I"${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include"''
              "-I ${pkgs.glibc.dev}/include"
            ];  # TODO
            version = "21";  # TODO
            OPENVR_PATH = "${openvr}";
            doCheck = false;  # TODO
            # LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            # LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            postUnpack = ''
            # Deal with submodules which is still annoying in Nix.
            cp -aR $OPENVR_PATH $(ls | grep -- -source)/openvr
            '';
            src = ./.;
            RUST_BACKTRACE = "full";  # TODO
            nativeBuildInputs = buildPackages;
            buildInputs = buildPackages ++ dependencyPackages;
            dontCargoInstall = true;  # TODO
            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "openxr-0.17.1" = "sha256-fG/JEqQQwKP5aerANAt5OeYYDZxcvUKCCaVdWRqHBPU=";
                "settings-schema-0.2.0" = "sha256-luEdAKDTq76dMeo5kA+QDTHpRMFUg3n0qvyQ7DkId0k=";
              };
            };
            CARGO_MANIFEST_DIR = ./.;  # probably unneeded
          };
          devShells.default = devShell { stdenv = pkgs.stdenv; nvidia = false; };
          devShells.nvidia  = devShell { stdenv = pkgs.stdenv; nvidia = true; };
        }
    );
}
