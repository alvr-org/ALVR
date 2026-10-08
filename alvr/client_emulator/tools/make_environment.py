#!/usr/bin/env python3
"""Generates a test environment for the ALVR client emulator.

The emulator renders unlit: it samples the base colour texture and nothing else. So this bakes
shading, ambient occlusion and a soft light gradient directly into the textures, which is also how
a photogrammetry or 3D-scanned room would arrive.

Produces a single self-contained .gltf that can be dropped next to the executable with no sidecar
files.

Textures are embedded as *buffer views*, not as `data:` image URIs. easy-gltf (1.1.5) decodes image
data URIs with a URL-safe base64 alphabet, while the glTF spec requires standard base64, so any
spec-compliant embedded image panics inside the loader. Buffer views avoid that code path entirely
because the buffer is decoded by the `gltf` crate, which is correct.

Usage:
    python make_environment.py [output_path]
"""

import base64
import json
import math
import struct
import sys
from io import BytesIO

from PIL import Image, ImageDraw, ImageFilter

TEXTURE_SIZE = 512

ROOM_HALF = 4.0
ROOM_HEIGHT = 2.6


def bake_shading(image, top_light=0.55, bottom_dark=0.35, edge_ao=0.45):
    """Applies a vertical light gradient and darkened edges to fake baked lighting."""
    width, height = image.size
    pixels = image.load()

    for y in range(height):
        # Bright near the top (closer to the ceiling light), darker toward the floor.
        vertical = 1.0 - bottom_dark * (y / max(height - 1, 1))
        vertical += top_light * math.exp(-6.0 * y / height)

        for x in range(width):
            # Ambient occlusion creeping in from all four edges.
            distance = min(x, y, width - 1 - x, height - 1 - y) / (min(width, height) * 0.5)
            occlusion = 1.0 - edge_ao * math.exp(-4.0 * distance)

            scale = max(0.0, min(1.6, vertical * occlusion))
            r, g, b = pixels[x, y][:3]
            pixels[x, y] = (
                min(255, int(r * scale)),
                min(255, int(g * scale)),
                min(255, int(b * scale)),
            )

    return image


def make_floor_texture():
    """Wooden planks running along one axis."""
    image = Image.new("RGB", (TEXTURE_SIZE, TEXTURE_SIZE), (150, 111, 74))
    draw = ImageDraw.Draw(image)

    plank_height = TEXTURE_SIZE // 8
    for index in range(8):
        y = index * plank_height
        shade = 1.0 + 0.08 * ((index % 3) - 1)
        colour = (int(150 * shade), int(111 * shade), int(74 * shade))
        draw.rectangle([0, y, TEXTURE_SIZE, y + plank_height - 2], fill=colour)
        # Seam between planks.
        draw.line([0, y + plank_height - 1, TEXTURE_SIZE, y + plank_height - 1], fill=(96, 68, 44))

        # Grain.
        for grain in range(14):
            gy = y + 3 + (grain * 7) % max(plank_height - 5, 1)
            draw.line(
                [0, gy, TEXTURE_SIZE, gy + ((grain % 3) - 1)],
                fill=(int(132 * shade), int(96 * shade), int(62 * shade)),
            )

    return bake_shading(image, top_light=0.15, bottom_dark=0.1, edge_ao=0.55)


def make_wall_texture():
    """Painted plaster with a skirting board along the bottom."""
    image = Image.new("RGB", (TEXTURE_SIZE, TEXTURE_SIZE), (203, 198, 188))

    # Subtle plaster noise so large flat areas are not perfectly uniform.
    noise = Image.effect_noise((TEXTURE_SIZE, TEXTURE_SIZE), 8).convert("RGB")
    image = Image.blend(image, noise, 0.06)
    image = image.filter(ImageFilter.GaussianBlur(0.4))

    draw = ImageDraw.Draw(image)
    # UV v=0 is the bottom of the wall in the geometry below, so the skirting goes at the top of the
    # image (textures are addressed with v increasing downward).
    skirting = int(TEXTURE_SIZE * 0.06)
    draw.rectangle([0, TEXTURE_SIZE - skirting, TEXTURE_SIZE, TEXTURE_SIZE], fill=(228, 226, 221))
    draw.line(
        [0, TEXTURE_SIZE - skirting, TEXTURE_SIZE, TEXTURE_SIZE - skirting],
        fill=(150, 146, 138),
    )

    return bake_shading(image, top_light=0.3, bottom_dark=0.4, edge_ao=0.5)


def make_ceiling_texture():
    image = Image.new("RGB", (TEXTURE_SIZE, TEXTURE_SIZE), (232, 232, 230))
    draw = ImageDraw.Draw(image)

    # A soft bright patch standing in for a light fitting.
    centre = TEXTURE_SIZE // 2
    for radius in range(TEXTURE_SIZE // 3, 0, -4):
        intensity = 1.0 - radius / (TEXTURE_SIZE / 3)
        value = int(232 + 23 * intensity)
        draw.ellipse(
            [centre - radius, centre - radius, centre + radius, centre + radius],
            fill=(value, value, min(255, value)),
        )

    image = image.filter(ImageFilter.GaussianBlur(6))
    return bake_shading(image, top_light=0.0, bottom_dark=0.0, edge_ao=0.5)


def make_furniture_texture():
    """Plain darker material for the boxes, with baked edge shading."""
    image = Image.new("RGB", (TEXTURE_SIZE, TEXTURE_SIZE), (96, 104, 120))
    noise = Image.effect_noise((TEXTURE_SIZE, TEXTURE_SIZE), 6).convert("RGB")
    image = Image.blend(image, noise, 0.05)
    return bake_shading(image, top_light=0.4, bottom_dark=0.35, edge_ao=0.4)


def encode_png(image):
    buffer = BytesIO()
    image.save(buffer, format="PNG", optimize=True)
    return buffer.getvalue()


class MeshBuilder:
    """Accumulates interleaved position/uv vertices grouped into one primitive per material."""

    def __init__(self):
        self.vertices = []  # flat list of x, y, z, u, v
        self.groups = {}  # material index -> list of indices

    def vertex_count(self):
        return len(self.vertices) // 5

    def quad(self, material, corners, uv_scale=1.0):
        """Adds a quad. `corners` must be counter-clockwise seen from the visible side."""
        base = self.vertex_count()
        uvs = [(0.0, 0.0), (uv_scale, 0.0), (uv_scale, uv_scale), (0.0, uv_scale)]

        for position, uv in zip(corners, uvs):
            self.vertices.extend([position[0], position[1], position[2], uv[0], uv[1]])

        self.groups.setdefault(material, []).extend(
            [base, base + 1, base + 2, base, base + 2, base + 3]
        )

    def box(self, material, centre, size, uv_scale=1.0):
        cx, cy, cz = centre
        hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2

        x0, x1 = cx - hx, cx + hx
        y0, y1 = cy - hy, cy + hy
        z0, z1 = cz - hz, cz + hz

        # Outward facing, counter-clockwise from outside.
        self.quad(material, [(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)], uv_scale)
        self.quad(material, [(x1, y0, z0), (x0, y0, z0), (x0, y1, z0), (x1, y1, z0)], uv_scale)
        self.quad(material, [(x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0)], uv_scale)
        self.quad(material, [(x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x1, y1, z1)], uv_scale)
        self.quad(material, [(x0, y1, z1), (x1, y1, z1), (x1, y1, z0), (x0, y1, z0)], uv_scale)
        self.quad(material, [(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)], uv_scale)


def build():
    mesh = MeshBuilder()

    FLOOR, WALL, CEILING, FURNITURE = 0, 1, 2, 3
    half, height = ROOM_HALF, ROOM_HEIGHT

    # Floor and ceiling, wound so they face into the room.
    mesh.quad(
        FLOOR,
        [(-half, 0, half), (half, 0, half), (half, 0, -half), (-half, 0, -half)],
        uv_scale=4.0,
    )
    mesh.quad(
        CEILING,
        [(-half, height, -half), (half, height, -half), (half, height, half), (-half, height, half)],
        uv_scale=1.0,
    )

    # Walls, facing inward. v increases upward, so the skirting drawn at the bottom of the texture
    # lands at floor level.
    mesh.quad(WALL, [(-half, 0, -half), (half, 0, -half), (half, height, -half), (-half, height, -half)], 2.0)
    mesh.quad(WALL, [(half, 0, half), (-half, 0, half), (-half, height, half), (half, height, half)], 2.0)
    mesh.quad(WALL, [(-half, 0, half), (-half, 0, -half), (-half, height, -half), (-half, height, half)], 2.0)
    mesh.quad(WALL, [(half, 0, -half), (half, 0, half), (half, height, half), (half, height, -half)], 2.0)

    # Furniture, giving the space depth cues and something to judge scale and parallax against.
    mesh.box(FURNITURE, (-2.2, 0.37, -2.4), (1.6, 0.74, 0.8))   # desk
    mesh.box(FURNITURE, (1.9, 0.55, 1.6), (1.1, 1.1, 1.1))      # cabinet
    mesh.box(FURNITURE, (2.6, 0.22, -2.2), (0.9, 0.44, 0.9))    # low table
    mesh.box(FURNITURE, (-2.9, 1.05, 2.5), (0.5, 2.1, 0.5))     # column
    mesh.box(FURNITURE, (0.2, 0.16, 0.4), (1.4, 0.32, 1.4))     # platform

    # Pack geometry.
    vertex_bytes = struct.pack("<%df" % len(mesh.vertices), *mesh.vertices)

    index_bytes = b""
    primitives_meta = []
    for material, indices in sorted(mesh.groups.items()):
        offset = len(index_bytes)
        index_bytes += struct.pack("<%dI" % len(indices), *indices)
        primitives_meta.append((material, offset, len(indices)))

    positions = [mesh.vertices[i : i + 3] for i in range(0, len(mesh.vertices), 5)]
    min_position = [min(p[axis] for p in positions) for axis in range(3)]
    max_position = [max(p[axis] for p in positions) for axis in range(3)]

    textures = [
        make_floor_texture(),
        make_wall_texture(),
        make_ceiling_texture(),
        make_furniture_texture(),
    ]

    # Append each PNG to the buffer, aligned to 4 bytes as the spec requires for buffer views.
    blob = bytearray(vertex_bytes + index_bytes)
    image_views = []
    for texture in textures:
        while len(blob) % 4:
            blob.append(0)

        png = encode_png(texture)
        image_views.append((len(blob), len(png)))
        blob.extend(png)

    blob = bytes(blob)

    accessors = [
        {
            "bufferView": 0,
            "byteOffset": 0,
            "componentType": 5126,
            "count": mesh.vertex_count(),
            "type": "VEC3",
            "min": min_position,
            "max": max_position,
        },
        {
            "bufferView": 0,
            "byteOffset": 12,
            "componentType": 5126,
            "count": mesh.vertex_count(),
            "type": "VEC2",
        },
    ]

    primitives = []
    for material, offset, count in primitives_meta:
        accessors.append(
            {
                "bufferView": 1,
                "byteOffset": offset,
                "componentType": 5125,  # unsigned int
                "count": count,
                "type": "SCALAR",
            }
        )
        primitives.append(
            {
                "attributes": {"POSITION": 0, "TEXCOORD_0": 1},
                "indices": len(accessors) - 1,
                "material": material,
                "mode": 4,
            }
        )

    gltf = {
        "asset": {"version": "2.0", "generator": "ALVR client emulator test environment"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": "room"}],
        "meshes": [{"name": "room", "primitives": primitives}],
        "materials": [
            {
                "name": name,
                "pbrMetallicRoughness": {
                    "baseColorTexture": {"index": index},
                    "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                    "metallicFactor": 0.0,
                    "roughnessFactor": 1.0,
                },
            }
            for index, name in enumerate(["floor", "wall", "ceiling", "furniture"])
        ],
        "textures": [{"source": index, "sampler": 0} for index in range(len(textures))],
        "samplers": [{"magFilter": 9729, "minFilter": 9729, "wrapS": 10497, "wrapT": 10497}],
        # Referenced by buffer view rather than data URI; see the module docstring.
        "images": [
            {"bufferView": 2 + index, "mimeType": "image/png", "name": name}
            for index, name in enumerate(["floor", "wall", "ceiling", "furniture"])
        ],
        "buffers": [
            {
                "byteLength": len(blob),
                "uri": "data:application/octet-stream;base64,"
                + base64.b64encode(blob).decode(),
            }
        ],
        "bufferViews": [
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
        # Image views carry no target: they are not vertex or index data.
        + [
            {"buffer": 0, "byteOffset": offset, "byteLength": length}
            for offset, length in image_views
        ],
        "accessors": accessors,
    }

    return gltf, mesh.vertex_count(), sum(count for _, _, count in primitives_meta)


def main():
    output = sys.argv[1] if len(sys.argv) > 1 else "environment.gltf"

    gltf, vertex_count, index_count = build()

    with open(output, "w", encoding="utf-8") as handle:
        json.dump(gltf, handle)

    print(
        f"Wrote {output}: {vertex_count} vertices, {index_count} indices, "
        f"{len(gltf['materials'])} materials"
    )


if __name__ == "__main__":
    main()
