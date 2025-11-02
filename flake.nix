{
  description = "Stream VR games from your PC to your headset via Wi-Fi";

  inputs = {
    self.submodules = true;
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flakeUtils.url = "github:numtide/flake-utils"; # TODO use upstream nix utils
  };

  outputs =
    {
      self,
      nixpkgs,
      flakeUtils,
    }:
    flakeUtils.lib.eachDefaultSystem (
      system:
      with nixpkgs.legacyPackages.${system};
      let
        buildPackages = [
          alsa-lib
          cargo
          libclang
          ffmpeg
          ffmpeg.dev
          jack2
          git
          llvmPackages.libclang
          openssl
          pipewire
          pipewire.dev
          pkg-config
          rustc
          rustPlatform.bindgenHook
          vulkan-headers
          vulkan-loader
        ];

        dependencyPackages = [
          brotli
          celt
          ffmpeg
          gccStdenv.cc # g++ for vrcompositor_wrapper
          gccStdenv.cc.cc.lib # crti.o and friends
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
          libva-vdpau-driver
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
          bzip2
          gmp
        ];

        nvidiaPackages = with cudaPackages; [
          cuda_cudart
          cuda_nvcc
          libnpp
        ];

        libsPatch = toString (
          replaceVars ./fix-finding-libs.patch {
            ffmpeg = lib.getDev ffmpeg;
            x264 = lib.getDev x264;
          }
        );

        devShell =
          { stdenv, nvidia }:
          (mkShell.override {
            stdenv = stdenv;
          })
            {
              LIBCLANG_PATH = "${libclang.lib}/lib";
              buildInputs =
                buildPackages
                ++ dependencyPackages
                ++ (lib.optionals nvidia nvidiaPackages)
                ++ [
                  watchexec
                ];
              NIX_CFLAGS_COMPILE = toString [
                "-lbrotlicommon"
                "-lbrotlidec"
                "-lcrypto"
                "-lpng"
                "-lssl"
              ];

              RUSTFLAGS = map (a: "-C link-arg=${a}") [
                "-Wl,--push-state,--no-as-needed"
                "-lEGL"
                "-lwayland-client"
                "-lxkbcommon"
                "-Wl,--pop-state"
              ];
              RUST_BACKTRACE = "1";
              shellHook = ''
                git apply ${libsPatch}
              '';
            };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage rec {
          pname = "alvr";
          LIBCLANG_PATH = "${libclang.lib}/lib";
          env.NIX_CFLAGS_COMPILE = toString [
            "-lbrotlicommon"
            "-lbrotlidec"
            "-lcrypto"
            "-lpng"
            "-lssl"
          ];
          RUSTFLAGS = map (a: "-C link-args=${a}") [
            "-Wl,--push-state,--no-as-needed"
            "-lEGL"
            "-lwayland-client"
            "-lxkbcommon"
            "-Wl,--pop-state"
          ];
          cargoBuildFlags = [
            "--exclude alvr_xtask"
            "--workspace"
          ];
          buildNoDefaultFeatures = true;
          patches = [
            (replaceVars ./fix-finding-libs.patch {
              ffmpeg = lib.getDev ffmpeg;
              x264 = lib.getDev x264;
            })
          ];
          version = "21.0.0-master"; # TODO Change to the release
          doCheck = false; # TODO Broken right now
          src = ./.;
          RUST_BACKTRACE = "full";
          nativeBuildInputs = buildPackages;
          buildInputs = buildPackages ++ dependencyPackages;
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "openxr-0.18.0" = "sha256-v8sY9PROrqzkpuq3laIn2hPaX+DY7Fbca6i/Xiacd1g=";
              "settings-schema-0.2.0" = "sha256-luEdAKDTq76dMeo5kA+QDTHpRMFUg3n0qvyQ7DkId0k=";
            };
          };
          postInstall = ''
            install -Dm755 ${src}/alvr/xtask/resources/alvr.desktop $out/share/applications/alvr.desktop
            install -Dm644 ${src}/resources/ALVR-Icon.svg $out/share/icons/hicolor/scalable/apps/alvr.svg

            # Install SteamVR driver
            mkdir -p $out/{libexec,lib/alvr,share}
            cp -r ./build/alvr_streamer_linux/lib64/. $out/lib
            cp -r ./build/alvr_streamer_linux/libexec/. $out/libexec
            cp -r ./build/alvr_streamer_linux/share/. $out/share
            ln -s $out/lib $out/lib64
          '';
          postBuild = ''
            # Build SteamVR driver ("streamer")
            cargo xtask build-streamer --release
          '';
          meta = {
            description = "Stream VR games from your PC to your headset via Wi-Fi";
            homepage = "https://github.com/alvr-org/ALVR/";
            changelog = "https://github.com/alvr-org/ALVR/releases/tag/v${version}";
            license = lib.licenses.mit;
            mainProgram = "alvr_dashboard";
          };
        };
        formatter = pkgs.nixfmt-tree;
        devShells.default = devShell {
          stdenv = clangStdenv;
          nvidia = false;
        };
        # TODO BROKEN
        #devShells.nvidia = devShell {
        #  stdenv = clangStdenv;
        #  nvidia = true;
        #};
      }
    );
}
