export JAVA_HOME=/usr/lib/jvm/java-20-openjdk
export ANDROID_SDK_ROOT=/home/bill/Android/Sdk
export ANDROID_NDK_ROOT=/home/bill/Android/Sdk/ndk/25.2.9519653
#cargo xtask prepare-deps --platform android
cargo xtask build-client --release
/home/bill/Android/Sdk/platform-tools/adb install build/alvr_client_android/alvr_client_android.apk
