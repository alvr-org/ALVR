# alvr_server_driver_ext

Rust extension to the driver C++ core.

For now this contains the crash report logic and imports `alvr_common`.

## Note: how to fix linking errors caused by alvr_server_driver_ext

* Execute `cargo clean; cargo build -p alvr_server_driver_ext --verbose`
* Copy last rustc command (`rustc --crate-name alvr_server_driver_ext ...`)
* Execute same rustc command with `--print native-static-libs` appended
* Take note of .lib dependencies
* On Visual Studio, Properties of alvr_server -> Input -> Additional Dependencies: add missing libs
