# How ALVR works

This document details some technologies used by ALVR.

If you have any doubt about what is (or isn't) written in here you can contact @zarik5, preferably on Discord.

**Note: At the time of writing, not all features listed here are implemented**

## Architecture

### The built application

ALVR is made of two applications: the server and client. The server is installed on the PC and the client is installed on the headset. While the client is a single APK, the server is made of three parts: the launcher, the driver and the dashboard. The launcher (`ALVR Launcher.exe`) is the single executable found at the root of the server app installation. The driver is located in `bin/win64/` and named `driver_alvr_server.dll`. The dashboard is a collection of files located in `dashboard/`.

The launcher sets up the PC environment and then opens SteamVR, which loads the ALVR driver. The driver is responsible for loading the dashboard and connecting to the client.

### Programming languages

ALVR is written in multiple languages: Rust, C, C++, Java, HTML, Javascript, HLSL, GLSL. C++ is the most present language in the codebase but Rust is the language that plays the most important role, as it is used as glue and more and more code is getting rewritten in Rust.  
Rust is a system programming language focused on memory safety and ease of use. It is as performant as C++ but code written on it is less likely to be affected by bugs at runtime. A feature of Rust that is extensively used by ALVR is enums, that correspond to tagged unions in C++. Rust's enums are a data type that can store different kinds of data, but only one type can be accessed at a time. For example the type `Result` can contain either an `Ok` value or an `Err` value but not both. Together with pattern matching, this is the foundation of error management in Rust applications.  
C++ and Java code in ALVR is legacy code inherited by the developer @polygraphene; it is almost unmaintained and it is getting replaced by Rust. HTML and Javascript are used to write the dashboard.  

### Source code organization

* `alvr/`: This is where most of the code resides. Each subfolder is a Rust crate ("crate" means a code library or executable).
  * `alvr/client/`: Crate that builds the client application. `alvr/client/android/` is the Android Studio project that builds the final APK.
  * `alvr/common/`: Code shared by both client and server. It contains code for settings generation, networking, audio and logging.
  * `alvr/launcher/`: This crate build the launcher executable.
  * `alvr/server/`: This crate builds the driver DLL. `alvr/server/cpp/` contains the legacy code.
  * `alvr/settings-schema/` and `alvr/settings-schema-derive/`: Utilities for settings code generation.
  * `alvr/xtask/`: Build utilities. The code contained in this crate does not actually end up in the final ALVR applications.
* `server_release_template/`: Contains every file for ALVR server that does not require a build pass. This includes the dashboard.
* `wix/`: WIX project used to crate the ALVR installer on Windows.

## Logging and error management

In ALVR codebase, logging is split into interface and implementation. The interface is defined in `alvr/common/src/logging.rs`, the implementations are defined in `alvr/server/src/logging_backend.rs` and `alvr/client/src/logging_backend.rs`.

ALVR logging system is based on the crate [log](https://crates.io/crates/log). `log` is already very powerful on its own, since the macros `error!`, `warn!`, `info!`, `debug!` and `trace!` can collect messages, file and line number of the invocation. But I needed something more that can reduce boilerplate when doing error management (*Disclaimer: I know that there are tens of already established error management crates but I wanted to have something even more opinionated and custom fitted*).  

ALVR defines some macros and functions to ease error management. The base type used for error management is `StrResult<T>` that is an alias for `Result<T, String>`. Read more about Rust's Result type [here](https://doc.rust-lang.org/std/result/).
`trace_err!` is a macro that takes as input a generic result and outputs and converts it into a `StrResult`. It does not support custom error messages and it should be used only to wrap `Result` types to convert them to `StrResult` when the result is actually not likely to return an error. This way we avoid calling `.unwrap()` that makes the program crash directly. In case of error, the `Err` type is converted to string and is prefixed with the current source code path and line number.  
`trace_none!` works similarly to `trace_err!` but it accepts an `Option` as argument. `None` is mapped to `StrResult::Err()` with no converted error message (because there is none).
`fmt_e!` is a macro to create a `StrResult<T>` from a hand specified error message. The result will be always `Err`.

When chaining `trace_err!` from one function to the other, a stack trace is formed. Unlike other error management crates, I can decide in which point in the stack to insert trace information to make error messages more concise.

To show an error (if present) the function `show_err` is defined. It shows an error popup if supported by the OS (currently only on Windows) and the message is also forwarded to `error!`.  
Other similar functions are defined: `show_e` shows an error unconditionally, `show_err_blocking` blocks the current thread until the popup is closed, `show_warn` opens a warning popup. More similar functions are in `alvr/common/src/logging.rs`.

### The messaging system

The communication between driver and the dashboard uses two methods. The dashboard can interrogate the server through an HTTP API. The server can notify the dashboard through logging. The server uses the function `log_id` to log a `LogId` instance (as JSON text). All log lines are sent to the dashboard though a websocket. The dashboard registers all log lines and searches for the log ID structures contained; the dashboard then reacts accordingly.  
While log IDs can contain any (serializable) type of data, it is preferred to use them only as notifications. Any type of data needed by the dashboard that should be persistent is stored in the session structure (more on this later), and the dashboard can request it any time.

## The launcher

The launcher is the entry point for the server application. It first checks that SteamVR is installed and setup properly and then launches it.  
The launcher requires `%LOCALAPPDATA%/openvr/` to contain a valid UTF-8 formatted file `openvrpaths.vrpath`. This file is crucial because it contains the path of the installation folder of SteamVR, the paths of the current registered drivers and the path of the Steam `config/` folder.  

### The bootstrap lifecycle

1. The launcher is opened. First `openvrpaths.vrpath` is checked to exist and to be valid.
2. From `openvrpaths.vrpath`, the list of registered drivers is obtained. If the current instance of ALVR is registered do nothing. Otherwise stash all driver paths to a file `alvr_drivers_paths_backup.txt` in `%TEMP%` and register the current ALVR path.
3. SteamVR is killed and then launched using the URI `steam://rungameid/250820`.
4. The launcher tries to GET `http://127.0.0.1:8082` until success.
5. The launcher closes itself.
6. Once the driver loads, `alvr_drivers_paths_backup.txt` is restored into `openvrpaths.vrpath`.

### Other launcher functions

The launcher has the button `Reset drivers and retry` that attempts to fix the current ALVR installation. It works as follows:

1. SteamVR is killed.
2. `openvrpaths.vrpath` is deleted and ALVR add-on is unblocked (in `steam/config/steamvr.vrsettings`).
3. SteamVR is launched and then killed again after a timeout. This is done to recreate the file `openvrpaths.vrpath`.
4. The current ALVR path is registered and SteamVR is launched again.

The launcher can also be launched in "restart" mode, that is headless (no window is visible). This is invoked by the driver to bootstrap a SteamVR restart (since the driver cannot restart itself since it is a DLL loaded by SteamVR).

## Settings generation and data storage

A common programming paradigm is to have a strict separation between UI and background logic. This generally helps with maintainability, but for settings management this becomes a burden, because for each change of the settings structure on the backend the UI must be manually updated. ALVR solves by heavily relying on code generation.

### Code generation on the backend (Rust)

On ALVR, settings are defined in one and only place, that is `alvr/common/src/data/settings.rs`. Rust structures and enums are used to construct a tree-like representation of the settings. Structs and enums are decorated with the derive macro `SettingsSchema` that deals with the backend side of the code generation.  
While the hand-defined structs and enums represent the concrete realization of a particular settings configuration, `SettingsSchema` generates two other settings representations, namely the schema and the "default" representation (aka session settings).  
The schema representation defines the structure and metadata of the settings (not the concrete values). While arrangement and position of the fields is inferred by the definition itself of the structures, the fields can also be decorated with metadata like `advanced`, `min`/`max`/`step`, `gui` type, etc. that is needed by the user interface.  
The second generated representation is the "default" representation. This representation has a dual purpose: it is used to define the default values of the settings (used in turn by the schema generation step) and to store the settings values on disk (`session.json`).
But why not use the original hand-defined structures to store the settings on disk? This is because enums (that are tagged unions) creates branching.  
The branching is a desired behavior. Take the `Controllers` setting in the Headset tab as an example. If you uncheck it it means you *now* don't care about any other settings related to controllers. If we store this on disk using the original settings representation, all modifications to the settings related to the controllers are lost, but *then* you may want to recover these settings.
To solve this problem, the default/session representation transforms every enum into a struct, where every branch becomes a field, so every branch coexist at once, even unused ones.

### Code generation on the frontend (Javascript)

One of the main jobs of the dashboard is to let the user interact with settings. The dashboard gets the schema from the driver and uses it to generate the user interface. The schema has every kind of data that the UI needs except for translations which are defined in `server_release_template/dashboard/js/app/nls`. This is because this type of metadata would obscure the original settings definition if it was defined inline, due to the large amount of text. The schema is also used to interpret the session data loaded from the server.

### The schema representation

While the original structs and enums that define settings are named, the schema representation loses the type names; it is based on a single base enum `SchemaNode` that can be nested. `SchemaNode` defines the following variants:

* `Section`: This is translated from `struct`s and struct-like `enum` variants data. It contains a list of named fields, that can be set to `advanced`. In the UI it is represented by a collapsible group of settings controls. The top level section is treated specially and it generates the tabs (Video, Audio, etc).
* `Choice`: This is translated from `enums`. Each variant can have one or zero childs. In the UI this is represented by a stateful button group. Only the active branch content is displayed.
* `Switch`: This is generated by the special struct `Switch`. This node type is used when a settings make sense to be "turned off", and it also had some associated specialized settings only when in the "on" state. In the UI this is similar to `Section` but has also a checkbox. In the future this should be graphically changed to a switch.
* `Boolean`: translated from `bool`.
* `Integer`/`Float`: Translated from integer and floating point type. They accept the metadata `min`, `max`, `step`, `gui`. `gui` can be either `textBox`, `upDown` and `slider`. Only certain combinations of `min`/`max`/`step`/`gui` is valid.
* `Text`: Translated from `String`. In the UI this is a simple textbox.
* `Array`: Translated from rust arrays. In the UI this is represented similarly to `Section`s, with the index as the field name. In the future this should be changed to look more like a table.

There are also currently unused node types:

* `Optional`: This is translated from `Option`. Similarly to `Switch`, this is generated from an enum that has one variant with data and one that doesn't. The reason behind the distinction is about the intention/meaning of the setting. Optional settings can either be "set" or "default". "Default" does not mean that the setting is set to a fixed default value, it means that ALVR can dynamically decide the value or let some other independent source decide the value, that ALVR might not even be aware of.
* `Vector` and `Dictionary`: Translated from `Vec<T>` and `Vec<(String, T)>` respectively. These types are unimplemented in the UI. They should represent a variable-sized collection of values.

### The session

Settings (in the session settings representation) are stored inside `session.json`, together with other session data. The session structure is defined in `alvr/common/src/data/session.rs`. The session supports extrapolation, that is the recovery of data when the structure of `session.json` does not match the schema. This often happens during a server version update. The extrapolation is also used when the dashboard requests saving the settings, where the payload can be a preset, that is a deliberately truncated session file.

## The connection lifecycle

The code responsible for the connection lifecycle is located in `alvr/client/src/connection.rs` and `alvr/server/src/connection.rs`.

The connection lifecycle can be divided into 3 steps: discovery, connection handshake and streaming.

During multiple connection steps, the client behaves like a server and the server behaves like a client. This is because of the balance in responsibility of the two peers. The client becomes the portal though a PC, that can contain sensitive data. For this reason the server has to trust the client before initiating the connection.

### Discovery

ALVR discovery protocol has initial support for a cryptographic handshake but it is currently unused.

When ALVR is launched for the first time on the headset, a hostname, certificate and secret are generated. The client then broadcasts its hostname, certificate and ALVR version (`ClientHandshakePacket`). The server has a looping task that listens for these packets and registers the client entry, saving hostname and certificate, if the client version is compatible.
If the client is visible and trusted on the server side, the connection handshake begins.

### Connection handshake

The client listens for incoming TCP connections with the `ControlSocket` from the server. Once connected the client sends its headset specifications (`HeadsetInfoPacket`). The server then combines this data with the settings to create the configuration used for streaming (`ClientConfigPacket`) that is sent to the client. In particular, this last packet contains the dashboard URL, so the client can access the server dashboard. If this streaming configuration is found to invalidate the current ALVR OpenVR driver initialization settings (`OpenvrConfig` inside the session), SteamVR is restarted.  
After this, if everything went right, the client discovery task is terminated, and after the server sends the control message `StartStream` the two peers are considered connected, but the procedure is not concluded. The next step is the setup of streams with `StreamSocket`.

### Streaming

The streams created from `StreamSocket` (audio, video, tracking, etc) are encapsulated in async loops that are all awaited concurrently. One of these loops is the receiving end of the `ControlSocket`.  
While streaming, the server only sends the control message `KeepAlive` periodically. The client can send `PlayspaceSync` (when the view is recentered), `RequestIDR` (in case of packet loss), and `KeepAlive`.

### Disconnection

When the control sockets encounters an error while sending or receiving a packet (for example with `KeepAlive`) the connection pipeline is interrupted and all looping tasks are canceled. A destructor callback (guard) is then run for objects or tasks that do not directly support canceling.

## The streaming socket

`StreamSocket` is an abstraction layer over multiple network protocols. It currently supports UDP and TCP but it is designed to support also QUIC without a big API refactoring. `StreamSocket` API is inspired by the QUIC protocol, where multiple streams can be multiplexed on the same socket.

Why not using one socket per stream? Regarding UDP, this does not have any particular advantage. The maximum transmission speed is still determined by the physical network controller and router. Regarding TCP, having multiple concurrent open sockets is even disadvantageous. TCP is a protocol that makes adjustments to the transmission speed depending on periodic network tests. Multiple TCP sockets can compete with each other for the available bandwidth, potentially resulting in unbalanced and unpredictable bandwidth between the sockets. Having one single multiplexed socket solves this by moving the bandwidth allocation problem to the application side.

### Packet layout

A packet is laid out as follows:

| Stream ID | Packet index |  Header  | Raw buffer |
| :-------: | :----------: | :------: | :--------: |
|  1 byte   |   8 bytes    | variable |  variable  |

The packet index is relative to a single stream. It is used to detect packet loss.  
Both header and raw buffer can have variable size, even from one packet to the other in the same stream. The header is serialized and deserialized using [bincode](https://github.com/servo/bincode) and so the header size can be obtained deterministically.

### Throttling buffer

A throttling buffer is a traffic shaping tool to avoid packet bursts, that often lead to packet loss.

If the throttling buffer is enabled, the packets are fragmented/recombined into buffers of a predefined size. The size should be set according to the supported MTU of the current network configuration, to avoid undetected packet fragmentation at the IP layer.

The current implementation is similar to the leaky bucket algorithm, but it uses some statistical machinery (`EventTiming` in fixed latency mode to 0) to dynamically determine the optimal time interval between packets such as the "bucket" does not overflow and the latency remains minimal.

## Event timing

`EventTiming` is a general purpose mathematical tool used to manage timing for cyclical processes. Some "enqueue" and "dequeue" events are registered and `EventTiming` outputs some timing hints to minimize the queuing time for the next events.

Currently, `EventTiming` is used for the stream socket throttling buffer and audio implementations, but it will be also used for video frame timing (to reduce latency and jitter), total video latency estimation (to reduce the black pull and positional lag), controller timing and maybe also controller jitter.

`EventTiming` supports two operation modes: fixed latency and automatic latency.

### Fixed latency mode

In fixed latency mode, `EventTiming` calculates the average latency between corresponding enqueue and dequeue events.

Todo

### Automatic latency mode

Todo

## Motion-to-photon pipeline

Todo

## Foveated encoding

Foveated encoding is a technique where frame images are individually compressed in a way that the human eye barely detects the compression. Particularly, the center of the image is kept at original resolution, and the rest is compressed. In practice, first the frames are re-rendered on the server with the outskirts of the frame "squished". The image is then transmitted to the client and then it gets re-expanded by using an inverse procedure.

But why does this work? The human eye has increased acuity in the center of the field of vision (the fovea) with respect to the periphery.

Foveated encoding should not be confused with foveated rendering, where the image is rendered to begin with at a lower resolution in certain spots. Foveated encoding will NOT lower your GPU usage, only the network usage.

Currently ALVR does not directly support foveated encoding in the strict sense, instead it uses *fixed* foveated encoding. In a traditional foveated encoding application, the eyes are tracked, so that only what is directly looked at is rendered at higher resolution. But currently none of the headset supported by ALVR support eye tracking. For this reason, ALVR does foveated encoding by pretending the user is looking straight at the center of the image, which most of time is true.

Here are explained three foveated encoding algorithms.

### Warp

Developed by @zarik5. This algorithm applies an image compression that most adapts to the actual acuity graph of the human eye. It compresses the image radially (with an ellipse as the base) from a chosen spot in the image, with a chosen monotonic function. This algorithm makes heavy use of derivatives and inverse functions. It is implemented using a chain of shaders (shaders are a small piece of code that is run on the GPU for performance reasons). You can explore an interactive demo at [this link](https://www.shadertoy.com/view/3l2GRR).

This algorithm is actually NOT used by ALVR. It used to be, but it got replaced by the "slices" method. The warp method has a fatal flaw: the pixel alignment is not respected. This causes resampling that makes the image look blurry.

### Slices

Developed by @zarik5. This is the current algorithm used by ALVR for foveated encoding. The frame is cut into 9 rectangles (with 2 vertical and 2 horizontal cuts). Each rectangle is rendered at a different compression level. The center rectangle is uncompressed, the top/bottom/left/right rectangle is compressed 2x, the corner rectangles are compressed 4x. These cuts are actually virtual (mathematical) cuts, that are executed all at once in a single shader pass. All slices are neatly packed to form a new rectangular image. You can explore an interactive demo at [this link](https://www.shadertoy.com/view/WddGz8).

This algorithm is much simpler than the warp method but it is still quite complex. The implementation takes into account pixel alignment and uses some margins in the rectangles to avoid color bleeding. Like the warp algorithm, the slices method was designed to support eye tracking support when it will be available in consumer hardware.

### Axis-Aligned Distorted Transfer (AADT)

This algorithm was developed by Oculus for the Oculus Link implementation. It is simpler than the other two methods, the end result looks better but it has less compression power. Like the slices algorithm, the image is cut into 9 rectangles where each rectangle is compressed independently. But actually the top and bottom rectangles are compressed only vertically, and the left and right only horizontally. This type of compression lends itself well to be used for images rendered in VR headsets, since it works in the same direction (and not against) the image distortion needed for lens distortion correction.

It is planned to replace the slices method with AADT in the future.

## Audio

Todo

---------------------------
Document written by @zarik5
