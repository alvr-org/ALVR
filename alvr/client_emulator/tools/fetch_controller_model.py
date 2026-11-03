#!/usr/bin/env python3
"""Download the Quest controller models the emulator draws for the Quest profile.

The models are the Quest 2 Touch controllers (`oculus-touch-v3`) from the W3C Immersive Web
group's WebXR Input Profiles, MIT licensed. The licence keeps the manufacturer's trademarks out of
the grant: the models may depict the device, but not be used to promote anything or to make
alternative skins of it. The licence text is written beside the models.

WebXR authors controller models around the grip pose its runtime reports, which for a Quest is
Meta's grip. The emulator's grip is SteamVR's instead -- the `openxr_grip` of SteamVR's own Quest 2
model, which is what ALVR's driver and its Quest correction are matched to -- and the two differ
by the controller's shape. Registering the two meshes against each other (point-to-point ICP over
SteamVR's model converted with `convert_rendermodel.py`) fits them to 1.6 mm RMS with Meta's grip
turned 15.7 degrees about X and 2.5 cm further forward. That is close to the 15 degrees ALVR's
client applies to a real Quest's grip for the same reason, which is a useful cross-check. The
transform is added here as a parent node, so the meshes stay exactly as published.

They are committed in `alvr/client_emulator/models/`, which `build.rs` copies beside the
executable, where `controllers.json` looks for them. This script is how they were obtained, and
refreshes them:

    python alvr/client_emulator/tools/fetch_controller_model.py --force

Writes `quest_controller_left.glb`, `quest_controller_right.glb` and
`quest_controller_LICENSE.md` into `alvr/client_emulator/models/` by default. Pass a directory to
write somewhere else.
"""

import argparse
import hashlib
import json
import struct
import sys
import urllib.error
import urllib.request
from pathlib import Path

# Pinned to the commit that added these files, so a rebuild fetches the same art rather than
# whatever the branch has drifted to.
COMMIT = "afcd4423001acd0ff981c6860c71ca49c7b43e52"
BASE_URL = (
    f"https://raw.githubusercontent.com/immersive-web/webxr-input-profiles/{COMMIT}"
    "/packages/assets"
)

# Output name: (source path, checksum, the model's frame in the emulator's grip frame as a glTF
# rotation quaternion xyzw and translation in metres). Measured per hand; the right is the mirror
# of the left to within the fit's noise.
MODELS = {
    "quest_controller_left.glb": (
        "profiles/oculus-touch-v3/left.glb",
        "5a3015792dac074b31607e63eeb75743eac479540a36739e5f94b65335a7f88a",
        [-0.13623, 0.00935, 0.00420, 0.99062],
        [0.0003, -0.0103, -0.0226],
    ),
    "quest_controller_right.glb": (
        "profiles/oculus-touch-v3/right.glb",
        "18a2e651328b684d3aeeef9f16c083af1eea9c1f98f78f9da94960432c8a021b",
        [-0.13586, -0.00893, -0.00495, 0.99068],
        [-0.0004, -0.0104, -0.0227],
    ),
}

LICENSE = (
    "LICENSE.md",
    "0ef0c87e8ffdd0681f332dc3b39f284dee2166d1f853fabc3853884fe81a6f30",
)
LICENSE_OUTPUT = "quest_controller_LICENSE.md"

ALIGNMENT_NODE = "emulator_grip_alignment"


def fetch(path: str, digest: str) -> bytes:
    url = f"{BASE_URL}/{path}"
    print(f"Fetching {url}")

    with urllib.request.urlopen(url, timeout=60) as response:
        data = response.read()

    actual = hashlib.sha256(data).hexdigest()
    if actual != digest:
        raise SystemExit(
            f"  failed: {path} does not match the expected checksum\n"
            f"    expected {digest}\n    got      {actual}"
        )

    return data


def aligned(glb: bytes, rotation: list, translation: list) -> bytes:
    """The same GLB with its scene's root nodes moved under one node carrying the alignment."""

    magic, version, _ = struct.unpack_from("<4sII", glb, 0)
    if magic != b"glTF" or version != 2:
        raise SystemExit("  failed: not a glTF 2.0 binary")

    json_length, json_type = struct.unpack_from("<I4s", glb, 12)
    if json_type != b"JSON":
        raise SystemExit("  failed: the first GLB chunk is not JSON")

    document = json.loads(glb[20 : 20 + json_length])
    rest = glb[20 + json_length :]

    if any(node.get("name") == ALIGNMENT_NODE for node in document["nodes"]):
        raise SystemExit("  failed: the model is already aligned")

    scene = document["scenes"][document.get("scene", 0)]
    document["nodes"].append(
        {
            "name": ALIGNMENT_NODE,
            "children": scene["nodes"],
            "rotation": rotation,
            "translation": translation,
        }
    )
    scene["nodes"] = [len(document["nodes"]) - 1]

    encoded = json.dumps(document, separators=(",", ":")).encode()
    encoded += b" " * (-len(encoded) % 4)

    header_free = struct.pack("<I4s", len(encoded), b"JSON") + encoded + rest
    return struct.pack("<4sII", b"glTF", 2, 12 + len(header_free)) + header_free


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

    try:
        for output_name, (source, digest, rotation, translation) in MODELS.items():
            output = directory / output_name

            if output.exists() and not arguments.force:
                print(f"{output} already exists; pass --force to replace it")
                continue

            data = aligned(fetch(source, digest), rotation, translation)
            output.write_bytes(data)
            print(f"  wrote {output} ({len(data) // 1024} KiB)")

        license_output = directory / LICENSE_OUTPUT
        if arguments.force or not license_output.exists():
            license_output.write_bytes(fetch(*LICENSE))
            print(f"  wrote {license_output}")
    except urllib.error.URLError as error:
        print(f"  failed: {error}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
