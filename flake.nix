{
  description = "Stream VR games from your PC to your headset via Wi-Fi";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/master";
    nixpkgsStaging.url = "github:NixOS/nixpkgs/staging";
    flakeUtils.url = "github:numtide/flake-utils";  # TODO use upstream nix utils
    openvr = {url = "github:ValveSoftware/openvr"; flake = false;};
  };

  outputs = { self, nixpkgs, nixpkgsStaging, flakeUtils, openvr }:
    flakeUtils.lib.eachDefaultSystem (system:
      with nixpkgs.legacyPackages.${system};
        let
          buildPackages = [
            alsa-lib
            cargo
            libclang
            ffmpeg.dev
            jack1
            git
            llvmPackages.libclang
            openssl
            pipewire.dev
            pkg-config
            nixpkgsStaging.legacyPackages.${system}.rustc
            vulkan-headers
          ];

          dependencyPackages = [
            brotli
            celt
            ffmpeg
            gccStdenv.cc  # g++ for vrcompositor_wrapper
            gccStdenv.cc.cc.lib  # crti.o and friends
            lame
            libdrm
            libglvnd
            libogg
            libpng
            libtheora
            libunwind
            libva
            libvdpau
            libxkbcommon
            nasm
            openssl
            pipewire
            soxr
            stdenv.cc.cc.lib
            vaapiVdpau
            vulkan-headers
            vulkan-headers
            vulkan-loader
            wayland
            x264
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            xvidcore
          ];

          nvidiaPackages = with cudaPackages; [
            cuda_cudart
            cuda_nvcc
            libnpp
          ];

          libsPatch = toString (replaceVars
          ./fix-finding-libs.patch {
            ffmpeg = lib.getDev ffmpeg;
            x264 = lib.getDev x264;
          });

          devShell = {stdenv, nvidia}: (mkShell.override {
            stdenv = stdenv;
           }) {
            # LIBCLANG_PATH="${llvmPackages.libclang.lib}";
            LIBCLANG_PATH="${libclang.lib}/lib";
            buildInputs = buildPackages ++ dependencyPackages ++ (lib.optionals nvidia nvidiaPackages) ++ [
              watchexec
            ];
            # LIBS_PATCH = writeText "libs.patch" libsPatch;
            RUSTFLAGS = map (a: "-C link-arg=${a}") [
              "-Wl,--push-state,--no-as-needed"
              "-lEGL"
              "-lclang"
              "-lva"
              "-lpng"
              "-lbrotlidec"
              "-lwayland-client"
              "-lxkbcommon"
              "-Wl,--pop-state"
            ];
            RUST_BACKTRACE = "1";  # TODO
            shellHook = ''
              git apply ${libsPatch}
            '';
          };
      in
        {
        #   packages.default = pkgs.rustPlatform.buildRustPackage rec {
        #     pname = "alvr";
        #     BINDGEN_EXTRA_CLANG_ARGS = [
        #       ''-I"${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include"''
        #       "-I ${pkgs.glibc.dev}/include"
        #     ];  # TODO
        #     version = "21";  # TODO
        #     OPENVR_PATH = "${openvr}";
        #     doCheck = false;  # TODO
        #     # LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        #     LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        #     postUnpack = ''
        #     # Deal with submodules which is still annoying in Nix.
        #     ln -s $OPENVR_PATH $(ls | grep -- -source)/openvr
        #     '';
        #     src = ./.;
        #     RUST_BACKTRACE = "full";  # TODO
        #     nativeBuildInputs = buildPackages;
        #     buildInputs = buildPackages ++ dependencyPackages;
        #     dontCargoInstall = true;  # TODO
        #     cargoLock = {
        #       lockFile = ./Cargo.lock;
        #       outputHashes = {
        #         "openxr-0.17.1" = "sha256-fG/JEqQQwKP5aerANAt5OeYYDZxcvUKCCaVdWRqHBPU=";
        #         "settings-schema-0.2.0" = "sha256-luEdAKDTq76dMeo5kA+QDTHpRMFUg3n0qvyQ7DkId0k=";
        #       };
        #     };
        #     CARGO_MANIFEST_DIR = ./.;  # probably unneeded
        #   };

          devShells.default = devShell { stdenv = clangStdenv; nvidia = false; };
          devShells.nvidia  = devShell { stdenv = clangStdenv; nvidia = true; };
        }
    );
}
