#!/usr/bin/env python3
"""Converts a SteamVR render model (OBJ + texture) into a glTF the client emulator can display.

SteamVR ships render models for most controllers under
`Steam/steamapps/common/SteamVR/resources/rendermodels/<name>/<name>.obj`. They cannot be
redistributed, but converting your local copy for your own emulator is fine, which is what this
script is for:

    python convert_rendermodel.py "C:/Program Files (x86)/Steam/steamapps/common/SteamVR/resources/rendermodels/oculus_quest2_controller_left" models/quest_left.gltf
    python convert_rendermodel.py ".../oculus_quest2_controller_right" models/quest_right.gltf

Then point the matching profile in `controllers.json` at the outputs:

    "left_model": "models/quest_left.gltf",
    "right_model": "models/quest_right.gltf"

Render models are authored around the SteamVR device pose, while the emulator draws controller
models at the OpenXR grip pose, the frame WebXR and runtime controller models are authored in. A
SteamVR render model says where the grip sits on it, in the `openxr_grip` component of the JSON
beside the OBJ, so the vertices are moved into that frame here. A model without the component falls
back to translating by ALVR's `left_controller_position_offset` (default 0, 0, -0.11), which only
approximates the grip; pass --offset to match a customised setting.

The output is self-contained: geometry and the base colour texture are embedded as buffer views
(not image data URIs, which the emulator's glTF loader cannot decode; see make_environment.py).
"""

import base64
import json
import math
import struct
import sys
from pathlib import Path


def parse_args(argv):
    offset = (0.0, 0.0, -0.11)
    positional = []

    arguments = iter(argv)
    for argument in arguments:
        if argument == "--offset":
            offset = tuple(float(next(arguments)) for _ in range(3))
        else:
            positional.append(argument)

    if len(positional) != 2:
        print(__doc__)
        sys.exit(1)

    return Path(positional[0]), Path(positional[1]), offset


def find_model_files(source):
    """Accepts either the render model directory or the OBJ file itself."""
    if source.is_dir():
        obj_path = source / (source.name + ".obj")
        if not obj_path.exists():
            candidates = sorted(source.glob("*.obj"))
            if not candidates:
                sys.exit(f"No .obj file found in {source}")
            obj_path = candidates[0]
    else:
        obj_path = source

    if not obj_path.exists():
        sys.exit(f"Not found: {obj_path}")

    # The texture named by the .mtl, or any same-named image next to the OBJ.
    texture_path = None
    mtl_path = obj_path.with_suffix(".mtl")
    if mtl_path.exists():
        for line in mtl_path.read_text(errors="replace").splitlines():
            parts = line.split(None, 1)
            if len(parts) == 2 and parts[0] == "map_Kd":
                texture_path = obj_path.parent / parts[1].strip()
                break

    if texture_path is None or not texture_path.exists():
        for suffix in (".png", ".tga", ".jpg"):
            candidate = obj_path.with_suffix(suffix)
            if candidate.exists():
                texture_path = candidate
                break

    return obj_path, texture_path


def grip_placement(obj_path, offset):
    """The function taking a device-frame vertex into the frame the emulator draws models in.

    The model's own `openxr_grip` component when its JSON has one, so the grip is exactly where
    SteamVR's model puts it; otherwise the translation ALVR's driver applies between the pose it is
    sent and the device pose, which leaves the model's grip only roughly at the emulator's.
    """
    json_path = obj_path.with_suffix(".json")
    component = None
    if json_path.exists():
        components = json.loads(json_path.read_text(errors="replace")).get("components", {})
        component = components.get("openxr_grip", {}).get("component_local")

    if component is None:
        print(f"No openxr_grip component beside {obj_path.name}; translating by {offset} instead")
        return lambda x, y, z: (x + offset[0], y + offset[1], z + offset[2])

    origin = component.get("origin", [0.0, 0.0, 0.0])
    rx, ry, rz = (math.radians(angle) for angle in component.get("rotate_xyz", [0.0, 0.0, 0.0]))

    # rotate_xyz composed as X, then Y, then Z. SteamVR's controller models only rotate their grip
    # about X, where the order makes no difference.
    def rotation(axis, angle):
        c, s = math.cos(angle), math.sin(angle)
        return {
            "x": [[1, 0, 0], [0, c, -s], [0, s, c]],
            "y": [[c, 0, s], [0, 1, 0], [-s, 0, c]],
            "z": [[c, -s, 0], [s, c, 0], [0, 0, 1]],
        }[axis]

    def matmul(a, b):
        return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]

    grip = matmul(matmul(rotation("x", rx), rotation("y", ry)), rotation("z", rz))

    def place(x, y, z):
        # Into the grip's frame: the inverse of the component's transform, so R^T (v - origin).
        d = (x - origin[0], y - origin[1], z - origin[2])
        return tuple(sum(grip[k][i] * d[k] for k in range(3)) for i in range(3))

    print(f"Placing {obj_path.name} at its openxr_grip component")
    return place


def load_texture_png(texture_path):
    """Returns PNG bytes for the texture, converting through Pillow when it is not already PNG."""
    if texture_path is None:
        return None

    if texture_path.suffix.lower() == ".png":
        return texture_path.read_bytes()

    from io import BytesIO

    from PIL import Image

    buffer = BytesIO()
    Image.open(texture_path).convert("RGBA").save(buffer, format="PNG", optimize=True)
    return buffer.getvalue()


def parse_obj(obj_path, place):
    positions = []
    uvs = []
    vertices = []  # flat x, y, z, u, v
    indices = []
    seen = {}  # (position index, uv index) -> vertex index

    def vertex(reference):
        parts = reference.split("/")
        # OBJ indices are 1-based; negative indices count from the end.
        position_index = int(parts[0])
        position_index += len(positions) if position_index < 0 else -1

        uv_index = None
        if len(parts) > 1 and parts[1]:
            uv_index = int(parts[1])
            uv_index += len(uvs) if uv_index < 0 else -1

        key = (position_index, uv_index)
        if key in seen:
            return seen[key]

        x, y, z = positions[position_index]
        u, v = uvs[uv_index] if uv_index is not None else (0.0, 0.0)

        index = len(vertices) // 5
        # glTF UV origin is top-left, OBJ is bottom-left.
        vertices.extend([*place(x, y, z), u, 1.0 - v])
        seen[key] = index
        return index

    with open(obj_path, errors="replace") as handle:
        for line in handle:
            parts = line.split()
            if not parts:
                continue

            if parts[0] == "v":
                positions.append((float(parts[1]), float(parts[2]), float(parts[3])))
            elif parts[0] == "vt":
                uvs.append((float(parts[1]), float(parts[2])))
            elif parts[0] == "f":
                corners = [vertex(reference) for reference in parts[1:]]
                # Triangulate as a fan; render model faces are triangles anyway.
                for second, third in zip(corners[1:], corners[2:]):
                    indices.extend([corners[0], second, third])

    return vertices, indices


def build_gltf(vertices, indices, png, name):
    vertex_count = len(vertices) // 5
    vertex_bytes = struct.pack("<%df" % len(vertices), *vertices)
    index_bytes = struct.pack("<%dI" % len(indices), *indices)

    positions = [vertices[i : i + 3] for i in range(0, len(vertices), 5)]
    min_position = [min(p[axis] for p in positions) for axis in range(3)]
    max_position = [max(p[axis] for p in positions) for axis in range(3)]

    blob = bytearray(vertex_bytes + index_bytes)

    buffer_views = [
        {
            "buffer": 0,
            "byteOffset": 0,
            "byteLength": len(vertex_bytes),
            "byteStride": 20,
            "target": 34962,
        },
        {
            "buffer": 0,
            "byteOffset": len(vertex_bytes),
            "byteLength": len(index_bytes),
            "target": 34963,
        },
    ]

    material = {
        "name": name,
        "pbrMetallicRoughness": {
            "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
            "metallicFactor": 0.0,
            "roughnessFactor": 1.0,
        },
    }

    gltf = {
        "asset": {"version": "2.0", "generator": "ALVR client emulator render model converter"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [
            {
                "name": name,
                "primitives": [
                    {
                        "attributes": {"POSITION": 0, "TEXCOORD_0": 1},
                        "indices": 2,
                        "material": 0,
                        "mode": 4,
                    }
                ],
            }
        ],
        "materials": [material],
        "accessors": [
            {
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC3",
                "min": min_position,
                "max": max_position,
            },
            {
                "bufferView": 0,
                "byteOffset": 12,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC2",
            },
            {
                "bufferView": 1,
                "byteOffset": 0,
                "componentType": 5125,
                "count": len(indices),
                "type": "SCALAR",
            },
        ],
    }

    if png is not None:
        while len(blob) % 4:
            blob.append(0)

        buffer_views.append({"buffer": 0, "byteOffset": len(blob), "byteLength": len(png)})
        blob.extend(png)

        material["pbrMetallicRoughness"]["baseColorTexture"] = {"index": 0}
        gltf["textures"] = [{"source": 0, "sampler": 0}]
        gltf["samplers"] = [{"magFilter": 9729, "minFilter": 9729, "wrapS": 10497, "wrapT": 10497}]
        gltf["images"] = [{"bufferView": 2, "mimeType": "image/png", "name": name}]

    gltf["bufferViews"] = buffer_views
    gltf["buffers"] = [
        {
            "byteLength": len(blob),
            "uri": "data:application/octet-stream;base64," + base64.b64encode(bytes(blob)).decode(),
        }
    ]

    return gltf


def main():
    source, output, offset = parse_args(sys.argv[1:])

    obj_path, texture_path = find_model_files(source)
    png = load_texture_png(texture_path)

    vertices, indices = parse_obj(obj_path, grip_placement(obj_path, offset))
    if not indices:
        sys.exit(f"No triangles found in {obj_path}")

    gltf = build_gltf(vertices, indices, png, obj_path.stem)

    output.parent.mkdir(parents=True, exist_ok=True)
    with open(output, "w", encoding="utf-8") as handle:
        json.dump(gltf, handle)

    texture_note = texture_path.name if texture_path else "no texture"
    print(
        f"Wrote {output}: {len(vertices) // 5} vertices, {len(indices)} indices, {texture_note}"
    )


if __name__ == "__main__":
    main()
