#!/bin/bash

source ./helper-functions.sh

prefix="installation"
container_name="arch-alvr"
system_podman_install=1
system_distrobox_install=1

function detect_gpu() {
   local gpu
   gpu=$(lspci | grep -i vga | tr '[:upper:]' '[:lower:]')
   if [[ $gpu == *"amd"* ]]; then
      echo 'amd'
      return
   elif [[ $gpu == *"nvidia"* ]]; then
      echo 'nvidia'
      return
   else
      echo 'intel'
      return
   fi
}

function detect_audio() {
   if [[ -n "$(pgrep pipewire)" ]]; then
      echo 'pipewire'
   elif [[ -n "$(pgrep pulseaudio)" ]]; then
      echo 'pulse'
   else
      echo 'none'
   fi
}

function phase1_distrobox_podman_install() {
   echor "Phase 1"
   mkdir "$prefix"
   cd "$prefix" || exit

   if ! which podman; then
      system_podman_install=0
      echog "Installing rootless podman"
      mkdir podman
      curl -s https://raw.githubusercontent.com/Meister1593/distrobox/main/extras/install-podman | sh -s -- --prefix "$PWD" --prefix-name "$container_name" # TODO temporary linked to own repository until MR passes
   fi

   if ! which distrobox; then
      system_distrobox_install=0
      echog "Installing distrobox"
      # Installing distrobox from git because it is much newer
      mkdir distrobox
      git clone https://github.com/89luca89/distrobox.git distrobox-git

      cd distrobox-git || exit
      ./install --prefix ../distrobox
      cd ..

      rm -rf distrobox-git
   fi
   cd ..
}

function phase2_distrobox_container_creation() {
   echor "Phase 2"
   GPU=$(detect_gpu)
   AUDIO_SYSTEM=$(detect_audio)

   source ./setup-dev-env.sh "$prefix"

   if [[ "$system_podman_install" == 0 ]]; then
      if [[ "$(which podman)" != "$prefix/podman/bin/podman" ]]; then
         echor "Failed to install podman properly"
         exit 1
      fi
   fi
   if [[ "$system_distrobox_install" == 0 ]]; then
      if [[ "$(which distrobox)" != "$prefix/distrobox/bin/distrobox" ]]; then
         echor "Failed to install distrobox properly"
         exit 1
      fi
   else
      if [[ "$GPU" == "nvidia" ]]; then
         echog "This script requires latest git version of distrobox, which has ability to integrate with host using --nvidia flag."
         echog "If you have that version, then write y and press enter to continue."
         read -r HAS_NVIDIA_FLAG
         if [[ "$HAS_NVIDIA_FLAG" != "y" ]]; then
            echor "Aborting installation as per user request."
            echor "Please visit https://github.com/89luca89/distrobox/blob/main/docs/posts/install_rootless.md for installing rootless podman and make sure to run it from git repository instead from curl."
            exit 1
         fi
      fi
   fi

   echo "$GPU" | tee -a "$prefix/specs.conf"
   if [[ "$GPU" == "amd" ]]; then
      distrobox create --pull --image docker.io/library/archlinux:latest \
         --name "$container_name" \
         --home "$prefix/$container_name"
      if [ $? -ne 0 ]; then
         echor "Couldn't create distrobox container, please report it to maintainer."
         echor "GPU: $GPU; AUDIO SYSTEM: $AUDIO_SYSTEM"
         exit 1
      fi
   elif [[ "$GPU" == nvidia* ]]; then
      CUDA_LIBS="$(find /usr/lib* -iname "libcuda*.so*")"
      if [[ -z "$CUDA_LIBS" ]]; then
         echor "Couldn't find CUDA on host, please install it as it's required for NVENC encoder support."
         exit 1
      fi
      distrobox create --pull --image docker.io/library/archlinux:latest \
         --name "$container_name" \
         --nvidia \
         --home "$prefix/$container_name"
      if [ $? -ne 0 ]; then
         echor "Couldn't create distrobox container, please report it to maintainer."
         echor "GPU: $GPU; AUDIO SYSTEM: $AUDIO_SYSTEM"
         exit 1
      fi
   else
      echor "Intel is not supported yet."
      exit 1
   fi

   if [[ "$AUDIO_SYSTEM" == "pipewire" ]]; then
      echo "$AUDIO_SYSTEM" | tee -a "$prefix/specs.conf"
   elif [[ "$AUDIO_SYSTEM" == "pulse" ]]; then
      echo "$AUDIO_SYSTEM" | tee -a "$prefix/specs.conf"
      echor "Do note that pulseaudio doesn't work well with ALVR and automatic microphone routing won't work."
   else
      echor "Unsupported audio system ($AUDIO_SYSTEM). Please report this issue."
      exit 1
   fi

   distrobox enter --name "$container_name" --additional-flags "--env prefix='$prefix' --env container_name='$container_name'" -- ./setup-phase-3.sh
   if [ $? -ne 0 ]; then
      echor "Couldn't install distrobox container first time at phase 3, please report it as an issue with attached setup.log from the directory."
      # envs are required! otherwise first time install won't have those env vars, despite them being even in bashrc, locale conf, profiles, etc
      exit 1
   fi
   distrobox stop --name "$container_name" --yes
   distrobox enter --name "$container_name" --additional-flags "--env prefix='$prefix' --env container_name='$container_name' --env LANG=en_US.UTF-8 --env LC_ALL=en_US.UTF-8" -- ./setup-phase-4.sh
   if [ $? -ne 0 ]; then
      echor "Couldn't install distrobox container first time at phase 4, please report it as an issue with attached setup.log from the directory."
      # envs are required! otherwise first time install won't have those env vars, despite them being even in bashrc, locale conf, profiles, etc
      exit 1
   fi
}

init_prefixed_installation "$@"
if [[ "$prefix" =~ \  ]]; then
   echor "File path to container can't contains spaces as SteamVR will fail to launch if path to it contains spaces."
   echor "Please clone or unpack repository into another directory that doesn't contain spaces."
   exit 1
fi
phase1_distrobox_podman_install
phase2_distrobox_container_creation
