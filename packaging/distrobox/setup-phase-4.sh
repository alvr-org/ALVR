#!/bin/bash

source ./links.sh
source ./helper-functions.sh

if [[ -z "$prefix" ]]; then
   echor "No prefix found inside distrobox, aborting"
   exit 1
fi

echor "Phase 4"
export STEP_INDEX=1
cd "$prefix" || echor "Couldn't go into installation folder, aborting."

# Get current gpu (and version in case if it's nvidia from configuration)
GPU="$(head <specs.conf -1 | tail -2)"
if [[ "$GPU" == nvidia* ]]; then
   GPU=$(echo "$GPU" | cut -d' ' -f1)
fi
AUDIO_SYSTEM="$(head <specs.conf -2 | tail -1)"

echog "Installing packages for base functionality."
sudo pacman -q --noprogressbar -Syu git vim base-devel noto-fonts xdg-user-dirs fuse libx264 sdl2 libva-utils xorg-server --noconfirm || exit 1
echog "Installing steam, audio and driver packages."
if [[ "$GPU" == "amd" ]]; then
   sudo pacman -q --noprogressbar -Syu libva-mesa-driver vulkan-radeon lib32-vulkan-radeon lib32-libva-mesa-driver --noconfirm || exit 1
elif [[ "$GPU" == "nvidia" ]]; then
   echog "Using host system driver mounts, not installing anything inside for nvidia drivers."
else
   echor "Couldn't determine gpu with name: $GPU, exiting!"
   exit 1
fi
if [[ "$AUDIO_SYSTEM" == "pipewire" ]]; then
   sudo pacman -q --noprogressbar -Syu lib32-pipewire pipewire pipewire-pulse pipewire-alsa pipewire-jack wireplumber --noconfirm || exit 1
elif [[ "$AUDIO_SYSTEM" == "pulseaudio" ]]; then
   sudo pacman -q --noprogressbar -Syu pulseaudio pusleaudio-alsa --noconfirm || exit 1
else
   echor "Couldn't determine audio system: $AUDIO_SYSTEM, you may have issues with audio!"
fi

sudo pacman -q --noprogressbar -Syu steam --noconfirm

export STEP_INDEX=2
sleep 2

# Ask user for installing steamvr
echog "Installed base packages and Steam. Opening steam. Please install SteamVR from it."
steam &>/dev/null &
echog "After installing SteamVR, copy (ctrl + shift + c from terminal) and launch command bellow from your host terminal shell (outside this container) and press enter to continue there. This prevents annoying popup (yes/no with asking for superuser) that prevents steamvr from launching automatically."
echog "sudo setcap CAP_SYS_NICE+ep '$HOME/.steam/steam/steamapps/common/SteamVR/bin/linux64/vrcompositor-launcher'"
echog "After you execute that command on host, press enter to continue."
read
echog "Now launch SteamVR once and close it."
echog "At this point you can safely add your external library from the host system ('/home/$USER' is same from inside the script container as from outside)"
echog "When ready for next step, press enter to continue."
read

export STEP_INDEX=3
sleep 2

# installing alvr
echog "Installing alvr"
echog "This installation script will download apk client for the headset later, but you shouldn't connect it to alvr during this script installation, leave it to post install."
wget -q --show-progress "$ALVR_LINK"
chmod +x "$ALVR_FILENAME"
./"$ALVR_FILENAME" --appimage-extract &>/dev/null
mv squashfs-root alvr
./alvr/usr/bin/alvr_dashboard &>/dev/null &
echog "ALVR and dashboard now launch and when it does that, skip setup (X button on right up corner)."
echog "After that, launch SteamVR using button on left lower corner and after starting steamvr, you should see one headset showing up in steamvr menu and 'Streamer: Connected' in ALVR dashboard."
echog "In ALVR Dashboard settings at left side, in the beginning set 'Game Audio' and 'Game Microphone' to pipewire (if possible)."
echog "Scroll all the way down and find 'Driver launch action', set it to 'No action' to prevent alvr from unregistering itself after startup."
echog "Find 'On connect script' and 'On disconnect script' as well and put $(realpath "$PWD"/../audio-setup.sh) (confirm each of them with enter on text box) into both of them. This is for automatic microphone that will load/unload based on connection to the headset"
echog "Next time you connect headset, set 'ALVR-MIC-Source' as your default microphone, it will automatically switch to it whenever it shows up."
echog "Tick 'Open setup wizard' too to prevent popup on dashboard startup."
echor "After you have done with this, press enter here, and don't close alvr dashboard manually."
read
cleanup_alvr
./alvr/usr/bin/alvr_dashboard &>/dev/null &
echor "Go to 'Installation' tab at left and press 'Register ALVR driver'"
echog "After that, press press 'Launch SteamVR' at left corner and hit enter here to continue."
read
echog "Downloading ALVR apk, you can install it now from the installation folder into your headset using either ADB or Sidequest on host."
wget -q --show-progress "$ALVR_APK_LINK"
echog "From this point on, alvr will automatically start with SteamVR. But it's still quite broken mechanism so we need to use additional script for auto-restart to work."
echog "Don't close ALVR."

STEP_INDEX=4
sleep 2

# installing wlxoverlay
echog "SteamVR overlay is partially broken on Linux (it also doesn't open games, only allows to interact with already opened games) and for replacement we will use WlxOverlay, which works with both X11 and Wayland and has ability to control whole desktop from inside VR."
wget -q --show-progress -O "$WLXOVERLAY_FILENAME" "$WLXOVERLAY_LINK"
chmod +x "$WLXOVERLAY_FILENAME"
if [[ "$WAYLAND_DISPLAY" != "" ]]; then
   echog "If you're not (on wlroots-based compositor like Sway), it will ask for display to choose. Choose the one display that contains every other (usually first in list)."
fi
./"$WLXOVERLAY_FILENAME" &>/dev/null &
if [[ "$WAYLAND_DISPLAY" != "" ]]; then
   echog "If everything went well, you might see little icon on your desktop that indicates that screenshare is happening (by WlxOverlay) created by xdg portal."
fi
echog "WlxOverlay adds itself to auto-startup so you don't need to do anything with it to make it autostart."
echog "Press enter to continue."
read

STEP_INDEX=5
sleep 2

# patching steamvr
echog "To prevent issues with SteamVR spamming with messages into it's own web interface, i created patcher that can prevent this spam. Without this, you will have issues with opening Video Setttings per app, bindings, etc."
echog "If you're okay with patching (and have compatible SteamVR version), you can type y and press enter to patch SteamVR. Otherwise just press enter to skip"
read -r DO_PATCH
if [[ "$DO_PATCH" == "y" ]]; then
   ../patch_bindings_spam.sh "$HOME/.steam/steam/steamapps/common/SteamVR"
fi

cleanup_alvr
cd ..

STEP_INDEX=6
sleep 2

# Added color to pacman for user convenience later
echo "Color" | sudo tee -a /etc/pacman.conf

# post messages
echog "From that point on, ALVR should be installed and WlxOverlay should be working. Please refer to https://github.com/galister/WlxOverlay/wiki/Getting-Started to familiarise with controls."
echor "To start alvr now you need to use start-alvr.sh script from this repository. It will also open Steam for you."
echog "In case you want to enter into container, do 'source setup-env.sh && distrobox-enter arch-alvr'"
echog "To close vr, press ctrl+c in terminal where start-alvr.sh script is running. It will automatically close alvr and steamvr."
echor "Very important: to prevent game from looking like it's severily lagging, please turn on legacy reprojection in per-app video settings in steamvr. This improves experience drastically."
echog "Don't forget to enable Steam Play for all supported titles with latest (non-experimental) proton to make all games visible as playable in Steam."
echog "Tip: to prevent double-restart due to how client resets it's settings, you can change settings and then put headset to sleep, and power back. This restarts client and server, and prevents double restart."
echog "Thank you for using the script! Continue with installing alvr apk to headset and with Post-installation notes to configure ALVR and SteamVR"
