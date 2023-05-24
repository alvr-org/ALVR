#!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

function echog() {
   echo -e "${RED}${STEP_INDEX}${NC} : ${GREEN}$1${NC}"
   sleep 0.5
}
function echor() {
   echo -e "${RED}${STEP_INDEX}${NC} : ${RED}$1${NC}"
   sleep 0.5
}
function cleanup_alvr() {
   echog "Cleaning up ALVR"
   for vrp in vrdashboard vrcompositor vrserver vrmonitor vrwebhelper vrstartup alvr_dashboard SlimeVR-amd64* slimevr openvr-spacecalibrator; do
      pkill -f $vrp
   done
   sleep 3
   for vrp in vrdashboard vrcompositor vrserver vrmonitor vrwebhelper vrstartup alvr_dashboard SlimeVR-amd64* slimevr openvr-spacecalibrator; do
      pkill -f -9 $vrp
   done
}

function init_prefixed_installation() {
   positional=()
   if [[ "$#" -eq 0 ]]; then
      echog "Using default installation with default name"
      return
   fi
   before_prefix="$prefix"
   before_container_name="$container_name"
   while [[ "$#" -gt 0 ]]; do
      case $1 in
      -p | --prefix)
         prefix="$(realpath "$2")"
         shift
         ;;
      -c | --container-name)
         container_name="$2"
         shift
         ;;
      -*)
         echor "Unknown parameter passed: $1"
         exit 1
         ;;
      *)
         positional+=("$1")
         shift
         ;;
      esac
      shift
   done
   if [[ "$before_prefix" == "$prefix" ]] || [[ "$before_container_name" == "$container_name" ]]; then
      echor "You must choose both prefix and container name to use prefixed installation"
      exit 1
   fi
   export prefix
   export container_name
}
