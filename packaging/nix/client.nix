# You will still need android studio and
# you'll also need to run this shell inside
# the main shell used to compile the server.
{ pkgs ? import <nixpkgs> { config.android_sdk.accept_license = true; }, ... }:

with pkgs;
(pkgs.buildFHSUserEnv {
  name = "android-sdk-env3";
  targetPkgs = pkgs:
    (with pkgs; [
      androidenv.androidPkgs_9_0.androidsdk
      gradle
      python3Minimal
      openjdk8
    ]);
  runScript = "bash";
}).env
