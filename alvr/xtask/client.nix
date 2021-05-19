{ pkgs ? import <nixpkgs> { config.android_sdk.accept_license = true; } }:

(pkgs.buildFHSUserEnv {
  name = "android-sdk-env3";
  targetPkgs = pkgs:
    (with pkgs; [ androidenv.androidPkgs_9_0.androidsdk gradle python3Minimal ]);
  runScript = "bash";
}).env
