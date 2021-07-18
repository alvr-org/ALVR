{ pkgs ? import <nixpkgs> { config.android_sdk.accept_license = true; } }:
let
  sdk = pkgs.androidenv.composeAndroidPackages {
    toolsVersion = "26.1.1";
    platformToolsVersion = "31.0.2";
    buildToolsVersions = [ "30.0.3" ];
    includeEmulator = false;
    platformVersions = [ "28" "29" "30" ];
    includeSources = false;
    includeSystemImages = false;
    systemImageTypes = [ "google_apis_playstore" ];
    abiVersions = [ "armeabi-v7a" "arm64-v8a" ];
    cmakeVersions = [ "3.10.2" ];
    includeNDK = true;
    ndkVersions = [ "22.0.7026061" ];
    useGoogleAPIs = false;
    useGoogleTVAddOns = false;
  };
in pkgs.mkShell rec {
  ANDROID_SDK_ROOT = "${sdk.androidsdk}/libexec/android-sdk";
  ANDROID_NDK_ROOT = "${ANDROID_SDK_ROOT}/ndk-bundle";

  buildInputs = with pkgs; [
    androidenv.androidPkgs_9_0.androidsdk
    gradle
    openjdk8
    cmake
  ];

  # Use the same buildToolsVersion here
  GRADLE_OPTS =
    "-Dorg.gradle.project.android.aapt2FromMavenOverride=${ANDROID_SDK_ROOT}/build-tools/${sdk.androidsdk.version}/aapt2";

}
