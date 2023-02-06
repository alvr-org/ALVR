# Settings guide

This guide lists all settings supported by ALVR and explain what they are, how they work and when to set them.

The user interface divides the settings into basic and advanced settings. To enable advanced settings you have to click `Show advanced options` in the top right corner in the Settings tab. Usually you should not touch advanced settings unless you know exactly what you are doing. 

Under the hood, basic settings work by modifying some advanced settings.

In this document, settings in **Basic Settings** describe settings that are visible in basic mode, **Advanced Settings** describe settings that are visible only in advanced mode. Some basic settings are also visible in advanced mode.

**Document updated for ALVR v15.1.0**

------------------------------------------

## Basic Video Settings

### Video resolution 

* Percentage of the native resolution of the headset to be used for encoding the video to be transmitted to the headset. 
    * Setting anything higher than 100% can slightly improve visual quality but at the cost of severely worse network performance. 
    * Setting anything lower the 100% can improve latency, stutters and remove encoder errors (especially with the Quest 2), at the cost of worse visual quality.

### Refresh rate 

* Choice between frame rates supported by the Quest headsets screen. 
* If a refresh rate is not supported on the headset (like 90Hz on the Quest 1), the closest supported refresh rate is picked and a warning will appear.

### Video codec 

* Algorithm used to encode the video stream to be transmitted to the headset, where it is decoded. h264 (AVC) and h265 (HEVC) are two codecs that are generally supported by recent GPUs. 
* Sometimes some older GPUs don't support either h264 or HEVC and you get an error message (and a SteamVR crash). 
    * In this case try switching the codec or try lowering the video resolution.

### Video Bitrate 

* Bitrate used for the video streaming. Higher bitrate can increase image quality, it your network setup supports it. If you experience glitches and freezes of the image you should lower this.

### Foveated encoding 

* This is an algorithm used to reduce network usage without sacrificing the image quality too much. You can read more at [this page](How-ALVR-works#foveated-encoding).

### Foveated encoding / Strength 

* Higher value means the foveation effect is more pronounced, but also more flickering artifacts.

### Foveated encoding / Vertical offset 

* Move the central high resolution rectangle higher or lower.

### Color correction 

* Color correction can help to get a more clear image.

### Color correction / Brightness 

* This setting produces a shift in the pixel color values. 1 means the image is completely white, -1 means completely black.

### Color correction / Contrast 

* Contrast regulates the distance of color channels from gray. -1 means completely gray.

### Color correction / Saturation 

* Saturation regulates the vividness of the image. -1 means the image is black and white.

### Color correction / Gamma 

* Gamma is a parameter to regulate the density distribution of brightness levels. You can use this parameter to get deeper blacks.

### Color correction / Sharpening 

* Values greater than 0 crates an embossing effect around elements on screen. This can make text easier to read. Values lower than 0 makes the image more fuzzy.

## Advanced Video Settings

### GPU index 

* Zero-based index of the GPU. For correct compatibility with SteamVR, this must always be set to 0. If you want to change the primary GPU used by SteamVR you have to use the control panel provided by your GPU vendor.

### Video encoding resolution base 

* This corresponds to `Video resolution`, but it gives the choice of specifying the resolution by relative scale or absolute value. Absolute width and height values could not respect the native aspect ratio of the headset screen.

### Preferred game rendering resolution 

* This is reported by ALVR to SteamVR as the screen resolution of the virtual headset. 
* SteamVR usually automatically chooses the game rendering resolution based on the available GPU resources, so most of the time only the aspect ratio matters.

### Custom refresh rate 

* Same as `Refresh rate` but the value can be directly typed.

### Request real-time decoder priority 

* Flag used by the android decoder.

### Use 10-bit encoder 

* Encode the video stream with 10 bit for the luma channel. This is primarily useful for reducing color banding. This flag works only for NVidia graphics cards.

### Seconds from V-Sync to photons 

* This is a timing variable needed by SteamVR drivers. It is not actually correlated to any real display v-sync.

### Foveated encoding / Shape 

* Aspect ratio of the central high resolution rectangle. A value greater than 1 produces a rectangle wider than tall.

***

## Basic audio settings

### Stream game audio 

* Play sounds from the PC on the headset.

### Stream game Select audio device 

* Audio device used to record the output audio to be sent to the headset. You should keep this to "Default". "Default" uses the currently default output audio device on Windows. You can change the default audio device by going in the system tray in the bottom right, click on the speaker icon then click on the audio device name.

* The device selected by ALVR is reported as the virtual headset speaker on SteamVR, so for best compatibility please set "Audio output device" to "Headset" in the SteamVR settings.

* If your speakers don't work with ALVR, try selecting another device. If none works, install the Oculus Runtime, then select `Headphones (Oculus Virtual Audio Device)`.

### Stream game Mute when streaming 

* Mute the selected audio device on the PC. Only the physical device connected to the PC is muted. The streamed audio in unaffected

### Stream game Configuration / Buffering 

* Mean queue time interval for audio samples. Increase this value if you hear audio stutters.

* Audio samples are not immediately played when they are sent from the PC to the headset. Audio samples needs to be played back with high timing accuracy to avoid audio distortions and generally the playback cannot be stopped without causing audible clicks or pops, but when streaming the source of audio the samples can arrive too early or too late.

* For this reason we need some amount of latency (on top of the transport latency) to keep a sample queue. This queue should be big enough so that it never runs out and it never overflows. If the queue underflows or overflows the playback will be disrupted.  

* This setting controls the mean time a sample stays in the buffering queue. Because of network jitter, the actual queue time will be in the interval `[0; 2 * buffering]`

### Stream headset microphone 

* Enable microphone on the headset and sends the audio to the PC. You need to install [VB-CABLE Virtual Audio Device](https://vb-audio.com/Cable/) to be able to stream the microphone.

### Stream headset microphone / Select virtual microphone input 

* This is the output audio device used to replay the microphone input captured on the headset. You cannot set this the same as the game audio device. When set to `Default`, ALVR searches for `Cable Input`.

### Stream headset microphone / Select virtual microphone output 

* This is the other end of the virtual microphone cable. If you have VB-CABLE installed, leave this to default.

* This setting is used only to setup SteamVR microphone device. To make this setting effective you need to leave "Audio input device" to "Headset" in the SteamVR settings.

## Advanced Audio Settings

### Stream game Audio device 

* Output audio device. It can be selected as default, by name and by index. 
* While basic settings do not allow to select microphones as the game audio device, you can do so by selecting "by name" and writing out the name of the device (it can be just a part of the full name, uppercase or lowercase does not matter).

### Stream game Configuration / Batch ms 

* Time interval used to calculate the size of the batch of audio samples to be processed on one go. 
* Lower values reduce latency (marginally), but they can put stress to the Quest when processing audio, that could cause audio artifacts or even crashes. 
* In the current implementation, this setting also controls the duration of fade-in/outs used for pop reduction in case of disruptions (lag, packet loss, packets out of order). A value too low can render the pop reduction algorithm less effective.

### Stream headset microphone / Virtual microphone input 

* Virtual microphone input device. It can be selected as default, by name and by index. It's preferred to use the basic setting.

### Stream headset microphone / Configuration 

* Analog to `Stream game Configuration`.

***

## Basic Headset Settings

### Headset emulation mode 

* SteamVR needs some information about the hardware connected to the PC to stream to. 
* Using ALVR, you don't directly connect the headset to the PC, so we can choose to emulate a headset different than the real one. You can choose between `Oculus Rift S`, `HTC Vive` and `Oculus Quest 2` (via Oculus Link). Some SteamVR games don't support the Rift S or the Quest, so if you encounter any problem you should try switching to `HTC Vive`.

* Currently this setting has a visual bug where `Oculus Quest 2` is always selected after a restart. The actual setting is not reverted. It will be fixed after a dashboard rewrite.

### Force 3DOF 

* Discard positional tracking data from the headset. In the game, the head will be stuck in place even if you move it in real life.

### Controllers 

* Enable controllers. This currently has effect only for the Quest headset.

### Controllers / Controller emulation mode 

* This is similar to `Headset emulation mode` but for the controllers. Usually they should match.

* `"No handtracking pinch"` means that pinch gestures are not registered. A "pinch" is the gesture of touching the tip of the thumb with the tip of any other finger in the same hand. Each pinch gesture is mapped to a different controller button in-game. Handtracking is enabled automatically when the controllers are disabled, so if you don't want to accidentally make button presses you should select no handtracking pinch.

* Currently handtracking does not support the thumbstick for movement.

### Controllers / Tracking speed 

* Regulates the strength of controller pose prediction. `Normal` means that the controllers will lag behind but the movement will be smooth, `Medium` and `Fast` makes the controller more reactive but also more jittery. `Oculus prediction` uses another prediction algorithm and corresponds to `Fast`.

* Why does ALVR need to predict the controller pose? ALVR needs to deal with many sources of latency (Wifi, video encoding, decoding, rendering, etc). Latency causes everything to lag behind. Controller pose is one of the things affected the worst by latency. We cannot predict the future but we can use an algorithm to estimate the controller pose. 
* This algorithm tries looks back at how the controller moved a few instants ago and then tries to continue the movement. This can work decently for low latency and slow movements, since the controller velocity remains almost constant. But fast movements (where the controllers are accelerated back and forth) cause the controllers to jitter, because the acceleration was not taken into account (because acceleration is fundamentally unpredictable).

### Controllers / Haptics intensity 

* Regulate the haptics (vibration) intensity. 0 means the haptics are disabled.

### Tracking space 

* The tracking space is the type of anchor used to make the virtual and real world match. 
* `Local` means that the anchor between the virtual and real worlds is movable: if you press and hold the Oculus button the world will rotate and translate depending on your position and heading at that moment. 
* `Stage` means that the real and virtual worlds are permanently anchored: if you press and hold the Oculus button nothing will happen. If you close the game and reopen it you will be exactly where you left off in the game if you didn't move. 
* `Local` is preferred for seated games and `Stage` is preferred for room scale games with real space walking.

## Advanced Headset Settings

### Universe ID 

* This is a parameter needed by SteamVR to decide how to store the Chaperone boundary settings.

### { mode Idx | Serial Number | ... | Registered device type } 

* These are settings needed by SteamVR to correctly set the headset emulation mode. You should use `Headset emulation mode` instead.

### Tracking frame offset 

* This is a signed integer used as offset when choosing the head tracking data to assign to a certain frame returned by SteamVR.

### Head position offset 

* This should be used as last resort if you can't fix the floor height or Chaperone boundary centering by other means.

### Controllers / { Mode Idx | Tracking system name | ... | Input profile path } 

* These are settings needed by SteamVR to correctly set the controller emulation mode. You should use `Controller emulation mode` instead.

### Controllers / Pose time offset 

* This is the latency offset value used by `Tracking speed`. You can set this value manually to have more control over the controller tracking prediction.

### Controllers / Client-side prediction 

* This corresponds to `Oculus prediction`.

### Controllers / Position offset 

* Position offset used to match the virtual controller position with the real controller position. This is needed because of a long standing bug of SteamVR.

### Controllers / Position rotation 

* Rotation offset used to match the virtual controller position with the real controller position. This is needed because of a long standing bug of SteamVR.

### Controllers / Extra latency mode 

* This should be left off normally
    * This may cause the headset position to be incorrect, if enabled

***

## Basic Connection Settings

### Stream protocol 

* A network protocol is a procedure and set of rules used for communication between devices connected in a network.

* You can choose between UDP, Throttled UDP and TCP socket protocols:
    * UDP has the lowest latency but works best at very low bitrates (<30 Mbps). Higher bitrates cause packet loss and stutter.
    * Throttled UDP is an experimental reimplementation of the previous socket. It works best at medium bitrates (~100 Mbps). At low bitrates it could have excessive delay and at higher bitrates is has the same problems as UDP.
    * TCP works well up at any bitrate up to 250 Mbps. It has the highest latency (but still lower than the previous ALVR versions). This is the new default.

### Aggressive keyframe resend 

* When checked, the encoder is allowed to resend keyframes faster with a timeout of 5ms.

* Usually video codecs compress the video stream by sending only what changed in the image to reduce network usage. This means that most frames actually contain incomplete information, that is completed by information retrieved by previous frames. This is why in case of packet loss the image becomes glitchy and blocky. 
* A keyframe (as known as IDR frame) is a special packet that contains a whole video frame. No previous information is needed to reconstruct this frame. Because of this, IDR frames are really heavy and should be sent only when needed, otherwise the network will completely hog.

## Advanced Connection Settings

### Trust clients automatically 

* If you uncheck this, clients will connect automatically without the need for trusting them. Is is a risk for security and it is off by default.

### Web server port 

* The IP port used to connect to the dashboard. If this is changed, the launcher will stop working.

### Streaming port 

* Port used for streaming (server to client, client to server).

### On connect script 

* Specify a command to be run when the server and headset connects. The environment variable `ACTION` will be set to the string `connect`.

### On disconnect script 

* Specify a command to be run when the server and headset disconnects. The environment variable `ACTION` will be set to the string `disconnect`.

### Enable FEC 

* FEC stands for Forward Error Correction. It is an algorithm used by the video streaming pipeline.

* This setting MUST NOT be set to false. Support for disabling this feature is incomplete and will likely cause a crash.

***

## Basic Extras

### Theme 

* Theme used for the dashboard. `System` can switch between light and dark mode depending on your system preference.

### Client dark mode 

* Simple color invert for the loading room/lobby in the headset. This is applied only after a sleep-wake cycle of the headset.

### Confirm revert 

* Show a confirmation dialog before reverting a setting to the default value.

### Confirm SteamVR restart 

* Show a confirmation dialog before restarting SteamVR. When SteamVR restarts, the VR game that was running gets closed and any unsaved progress is lost.

### Prompt before update 

* When an update is available, install it immediately without asking. Only happens at startup.

### Update channel 

* The update channel is a setting that controls what kind of update to receive. 
    * `No updates` disables updates
    * `Stable` is to receive stable updates
    * `Beta` is to receive pre-release updates that had only limited testing
    * `Nightly` are completely untested releases that may not work at all, but you get the latest features before anyone else

### Log to disk 

* Save the file `session.txt` at the root of the ALVR installation folder. 
* This is useful to get get debug information when a crash happens. By default this is disabled because this file continues to grow as long as ALVR is kept open and it keeps growing until the whole hard-drive is filled.

## Advanced Extras

### Notification level 

* Select what kind of notification should be displayed in the bottom left corner of the dashboard. Each level contains all levels with higher severity.

### Exclude notifications without ID 

* This is a legacy setting. It should be set to false for now.
