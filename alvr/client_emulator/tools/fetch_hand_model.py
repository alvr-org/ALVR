#!/usr/bin/env python3
"""Download the hand models the emulator draws for hand tracking emulation.

The models come from Godot XR Tools, whose glTF files declare themselves CC0 Public Domain in
their own `asset.copyright` (the surrounding project is MIT licensed). They fit this use exactly:
one skinned mesh each, rigged to twenty-six joints named after OpenXR's `XR_EXT_hand_tracking`
layout, which is the same set ALVR carries on the wire. Nothing about the emulator depends on
these particular files -- the loader matches joints by name and corrects for whatever bone
convention a rig uses -- so any rigged, skinned hand can replace them by pointing `left_model` and
`right_model` in `hands.json` somewhere else.

They are committed in `alvr/client_emulator/models/`, which `build.rs` copies beside the
executable, where `hands.json` looks for them. This script is how they were obtained, and
refreshes them:

    python alvr/client_emulator/tools/fetch_hand_model.py --force

Writes `hand_left.gltf` and `hand_right.gltf` into `alvr/client_emulator/models/` by default.
Pass a directory to write somewhere else.
"""

import argparse
import hashlib
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path

# Pinned to the commit that last touched these files, so a rebuild fetches the same art rather
# than whatever the branch has drifted to.
COMMIT = "217bda36be91dd8852c6d3c93dc044558a525ed2"
BASE_URL = (
    f"https://raw.githubusercontent.com/GodotVR/godot-xr-tools/{COMMIT}"
    "/addons/godot-xr-tools/hands/model"
)

MODELS = {
    "hand_left.gltf": (
        "Hand_Glove_L.gltf",
        "5afa8503d009e1f024a08ad917729a3ebe50a8f9485681167a3446e7458a7264",
    ),
    "hand_right.gltf": (
        "Hand_Glove_R.gltf",
        "921be5c8095500818e8647dc9258552eaf5cdfcf54f71c7122dd7ffbe9180012",
    ),
}

EXPECTED_JOINTS = 26


def check(data: bytes, name: str) -> None:
    """Confirm the file really is the skinned hand the emulator expects.

    Cheap insurance against a silently repointed URL: a model that is not skinned, or whose joints
    cannot be named, would otherwise only fail at runtime as a hand that does not move.
    """

    document = json.loads(data)

    if not document.get("skins"):
        raise SystemExit(f"{name} has no skin; it cannot be posed")

    joints = len(document["skins"][0]["joints"])
    if joints != EXPECTED_JOINTS:
        print(
            f"  note: {name} has {joints} joints rather than {EXPECTED_JOINTS}; "
            "unmatched ones will simply follow their parent"
        )

    copyright_notice = document.get("asset", {}).get("copyright")
    if copyright_notice:
        print(f"  {name}: {copyright_notice}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "directory",
        nargs="?",
        default=str(Path(__file__).resolve().parent.parent / "models"),
        help="where to write the models (default: alvr/client_emulator/models)",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="download again even if the files are already there",
    )
    arguments = parser.parse_args()

    directory = Path(arguments.directory)
    directory.mkdir(parents=True, exist_ok=True)

    for output_name, (source_name, digest) in MODELS.items():
        output = directory / output_name

        if output.exists() and not arguments.force:
            print(f"{output} already exists; pass --force to replace it")
            continue

        url = f"{BASE_URL}/{source_name}"
        print(f"Fetching {url}")

        try:
            with urllib.request.urlopen(url, timeout=60) as response:
                data = response.read()
        except urllib.error.URLError as error:
            print(f"  failed: {error}", file=sys.stderr)
            return 1

        actual = hashlib.sha256(data).hexdigest()
        if actual != digest:
            print(
                f"  failed: {source_name} does not match the expected checksum\n"
                f"    expected {digest}\n    got      {actual}",
                file=sys.stderr,
            )
            return 1

        check(data, source_name)

        output.write_bytes(data)
        print(f"  wrote {output} ({len(data) // 1024} KiB)")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
