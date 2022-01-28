# alvr_openvr_driver

OpenVR driver written mostly in C++. C++ is used because `openvr_driver.h` header makes extensive use of classes which are expected to be used with subclassing (that maps poorly to Rust). The scope of this driver is reduced at the point that error handling is not needed. Communication with the server happens through IPC.
