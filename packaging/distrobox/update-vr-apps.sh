#!/bin/bash

prefix="installation"

source ./links.sh
source ./helper-functions.sh

init_prefixed_installation "$@"
source ./setup-dev-env.sh "$prefix"

echog "Reinstalling alvr"
rm -r $prefix/alvr "${prefix:?}/${ALVR_FILENAME:?}" "${prefix:?}/${ALVR_APK_NAME:?}"
wget -q --show-progress -P $prefix/ "$ALVR_LINK"
chmod +x "$prefix/$ALVR_FILENAME"
"$prefix"/./"$ALVR_FILENAME" --appimage-extract
mv squashfs-root $prefix/alvr
echog "Downloading alvr apk"
wget -q --show-progress -P $prefix/ "$ALVR_APK_LINK"

echog "Reinstalling wlxoverlay"
rm "$prefix"/"$WLXOVERLAY_FILENAME"
wget -O "$prefix/$WLXOVERLAY_FILENAME" -q --show-progress "$WLXOVERLAY_LINK"
chmod +x "$prefix/$WLXOVERLAY_FILENAME"

echog "Installation finished."
