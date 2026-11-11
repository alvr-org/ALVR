#!/usr/bin/env python3
"""Generates a test environment for the ALVR client emulator.

The emulator renders unlit: it multiplies each material's colour by its base colour texture and
nothing else. So this bakes shading, ambient occlusion and soft light gradients directly into the
textures, which is also how a photogrammetry or 3D-scanned room would arrive.

Two styles:

- `studio`, the default: a custom 16 x 16 m, 3.9 m high room, navy with a lime accent line and a
  blue line-fan motif on every wall, grey carpet with a blue metre grid, furnished on every wall
  and with a long-haired grey cat.
- `brown`: the original 8 x 8 m test room, warm walls and a wooden floor, kept as a backup.

Produces a single self-contained .gltf that can be dropped into the emulator's user folder as
environment.gltf with no sidecar files.

Textures are embedded as *buffer views*, not as `data:` image URIs. easy-gltf (1.1.5) decodes image
data URIs with a URL-safe base64 alphabet, while the glTF spec requires standard base64, so any
spec-compliant embedded image panics inside the loader. Buffer views avoid that code path entirely
because the buffer is decoded by the `gltf` crate, which is correct.

The studio's lighting is baked into vertex colours by Blender when it is installed (see
bake_in_blender.py): area lights where the ceiling panels and windows are, light bounced round the
room, shadows from the real shapes. Without Blender, or with `--bake python`, a simpler bake here
stands in: direct light with shadows from boxes round the furniture, and an estimate of occlusion.

Usage:
    python make_environment.py [--style studio|brown] [--bake auto|blender|python] [output_path]
"""

import argparse
import base64
import glob
import itertools
import json
import math
import os
import random
import shutil
import struct
import subprocess
import tempfile
from io import BytesIO

from PIL import Image, ImageChops, ImageDraw, ImageFilter

TEXTURE_SIZE = 512


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


def encode_png(image):
    buffer = BytesIO()
    image.save(buffer, format="PNG", optimize=True)
    return buffer.getvalue()


def rgb(hex_colour):
    """`"#06103C"` as an (r, g, b) tuple."""
    value = hex_colour.lstrip("#")
    return tuple(int(value[i : i + 2], 16) for i in (0, 2, 4))


class Material:
    """A colour, and optionally a texture the colour multiplies, as the emulator draws them."""

    def __init__(self, name, colour=(255, 255, 255), texture=None):
        self.name = name
        self.colour = colour
        self.texture = texture


class MeshBuilder:
    """Accumulates interleaved position/uv vertices grouped into one primitive per material.

    `transform`, when set, maps each local position to the room as it is added, so a model such as
    the cat can be built around its own origin and then placed.

    Every vertex also has a colour, white unless `bake_lighting` lights it: the surfaces added with
    `lit_surface` are recorded as receivers of the bake, and the boxes passed to `block` as the
    shadow casters.
    """

    def __init__(self):
        self.vertices = []  # flat list of x, y, z, u, v
        self.colours = []  # r, g, b per vertex, 0 to 1
        self.groups = {}  # material -> list of indices
        self.transform = None
        self.receivers = []  # (vertex index, position, normal)
        self.blockers = []  # (min corner, max corner)
        self.lights = []  # see `light`

    def vertex_count(self):
        return len(self.vertices) // 5

    def vertex(self, position, uv):
        if self.transform is not None:
            position = self.transform(position)

        self.vertices.extend([position[0], position[1], position[2], uv[0], uv[1]])
        self.colours.append((1.0, 1.0, 1.0))
        return self.vertex_count() - 1

    def lit_surface(self, material, corner, along, up, normal, steps, uv):
        """A flat rectangle from `corner` spanning the vectors `along` and `up`, cut into
        steps[0] x steps[1] cells so baked light can vary across it, its vertices recorded as
        receivers of the bake. `uv(a, b)` maps the fractions along each side to texture
        coordinates."""

        def point(i, j):
            a, b = j / steps[0], i / steps[1]
            position = tuple(c + a * u + b * v for c, u, v in zip(corner, along, up))
            return position, uv(a, b)

        start = self.vertex_count()
        self.grid(material, steps[1], steps[0], point)
        for index in range(start, self.vertex_count()):
            self.receivers.append((index, tuple(self.vertices[index * 5 : index * 5 + 3]), normal))

    def light(self, name, position, facing, along, size, power, samples):
        """Records a rectangular light source for the bake: its centre, the direction it shines,
        the direction of its width, its width and height, its power in watts for Blender, and
        sample points across it for the bake here, as `bake_lighting` takes them."""
        self.lights.append({"name": name, "position": position, "facing": facing, "along": along,
                            "size": size, "power": power, "samples": samples})

    def block(self, centre, size):
        """Records a box, in the current `transform`'s frame like the geometry around it, as a
        shadow caster for the bake: its bounding box in the room, which is exact for the quarter
        turns furniture stands at."""
        corners = []
        for signs in itertools.product((-1, 1), repeat=3):
            corner = tuple(c + sign * extent / 2 for c, sign, extent in zip(centre, signs, size))
            corners.append(self.transform(corner) if self.transform is not None else corner)
        self.blockers.append((
            tuple(min(corner[axis] for corner in corners) for axis in range(3)),
            tuple(max(corner[axis] for corner in corners) for axis in range(3)),
        ))

    def triangles(self, material, indices):
        self.groups.setdefault(material, []).extend(indices)

    def quad(self, material, corners, uv_scale=1.0, uvs=None):
        """Adds a quad. `uvs` defaults to the unit square scaled by `uv_scale`."""
        if uvs is None:
            uvs = [(0.0, 0.0), (uv_scale, 0.0), (uv_scale, uv_scale), (0.0, uv_scale)]

        base = [self.vertex(position, uv) for position, uv in zip(corners, uvs)]
        self.triangles(material, [base[0], base[1], base[2], base[0], base[2], base[3]])

    def box(self, material, centre, size, uv_scale=1.0):
        cx, cy, cz = centre
        hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2

        x0, x1 = cx - hx, cx + hx
        y0, y1 = cy - hy, cy + hy
        z0, z1 = cz - hz, cz + hz

        self.quad(material, [(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)], uv_scale)
        self.quad(material, [(x1, y0, z0), (x0, y0, z0), (x0, y1, z0), (x1, y1, z0)], uv_scale)
        self.quad(material, [(x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0)], uv_scale)
        self.quad(material, [(x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x1, y1, z1)], uv_scale)
        self.quad(material, [(x0, y1, z1), (x1, y1, z1), (x1, y1, z0), (x0, y1, z0)], uv_scale)
        self.quad(material, [(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)], uv_scale)

    def grid(self, material, rows, columns, point):
        """A surface from `point(i, j)` over a rows x columns grid, as (position, uv) pairs."""
        start = self.vertex_count()
        for i in range(rows + 1):
            for j in range(columns + 1):
                position, uv = point(i, j)
                self.vertex(position, uv)

        indices = []
        width = columns + 1
        for i in range(rows):
            for j in range(columns):
                a = start + i * width + j
                b = a + 1
                c = a + width
                d = c + 1
                indices.extend([a, c, b, b, c, d])
        self.triangles(material, indices)

    def ellipsoid(self, material, centre, radii, rings=14, segments=20, rotate=None):
        """An ellipsoid whose texture v runs from its top (0) to its bottom (1), so a vertical
        gradient in the texture shades it as if lit from above."""

        def point(i, j):
            theta = math.pi * i / rings
            phi = 2 * math.pi * j / segments
            local = (
                radii[0] * math.sin(theta) * math.cos(phi),
                radii[1] * math.cos(theta),
                radii[2] * math.sin(theta) * math.sin(phi),
            )
            if rotate is not None:
                local = rotate(local)
            position = tuple(c + o for c, o in zip(centre, local))
            return position, (j / segments, i / rings)

        self.grid(material, rings, segments, point)

    def tube(self, material, path, radii, segments=12):
        """A tube along `path`, with radius `radii[i]` at `path[i]`."""

        def frame(index):
            before = path[max(index - 1, 0)]
            after = path[min(index + 1, len(path) - 1)]
            tangent = normalise(tuple(a - b for a, b in zip(after, before)))
            reference = (0.0, 1.0, 0.0) if abs(tangent[1]) < 0.9 else (1.0, 0.0, 0.0)
            side = normalise(cross(tangent, reference))
            up = cross(side, tangent)
            return side, up

        frames = [frame(index) for index in range(len(path))]

        def point(i, j):
            side, up = frames[i]
            angle = 2 * math.pi * j / segments
            offset = tuple(
                radii[i] * (math.cos(angle) * s + math.sin(angle) * u) for s, u in zip(side, up)
            )
            position = tuple(p + o for p, o in zip(path[i], offset))
            # v around the tube, so the gradient lights its upper side.
            return position, (i / max(len(path) - 1, 1), 0.5 - 0.5 * math.sin(angle))

        start = self.vertex_count()
        self.grid(material, len(path) - 1, segments, point)

        # Close both ends: a fan from each end's centre to its ring of the grid.
        for index in (0, len(path) - 1):
            ring = start + index * (segments + 1)
            centre = self.vertex(path[index], (0.5, 0.5))
            indices = []
            for j in range(segments):
                indices.extend([centre, ring + j, ring + j + 1])
            self.triangles(material, indices)

    def cone(self, material, base_centre, tip, radius, segments=10):
        """A cone from a circular base, lying across `up`, to its tip."""
        axis = normalise(tuple(t - b for t, b in zip(tip, base_centre)))
        reference = (0.0, 0.0, 1.0) if abs(axis[2]) < 0.9 else (1.0, 0.0, 0.0)
        side = normalise(cross(axis, reference))
        up = cross(side, axis)

        tip_index = self.vertex(tip, (0.5, 0.0))
        ring = []
        for j in range(segments + 1):
            angle = 2 * math.pi * j / segments
            position = tuple(
                b + radius * (math.cos(angle) * s + math.sin(angle) * u)
                for b, s, u in zip(base_centre, side, up)
            )
            ring.append(self.vertex(position, (j / segments, 1.0)))

        indices = []
        for j in range(segments):
            indices.extend([tip_index, ring[j], ring[j + 1]])
        self.triangles(material, indices)

    def disc(self, material, centre, radius, segments=48, y_offset=0.0):
        """A flat horizontal disc, its texture mapped radially: u around, v from the centre out."""

        def point(i, j):
            r = radius * i
            angle = 2 * math.pi * j / segments
            position = (
                centre[0] + r * math.cos(angle),
                centre[1] + y_offset,
                centre[2] + r * math.sin(angle),
            )
            return position, (j / segments, i)

        self.grid(material, 1, segments, point)

    def ring(self, material, centre, inner, outer, segments=96):
        """A flat horizontal annulus."""

        def point(i, j):
            r = inner + (outer - inner) * i
            angle = 2 * math.pi * j / segments
            position = (centre[0] + r * math.cos(angle), centre[1], centre[2] + r * math.sin(angle))
            return position, (j / segments, i)

        self.grid(material, 1, segments, point)


    def lathe(self, material, centre, profile, segments=24):
        """A surface of revolution about a vertical axis through `centre`, from `profile`, a list
        of (radius, height) from the bottom up. Open unless the profile starts or ends at radius 0.
        Texture u runs around it from +X through -Z at 0.75, v from its top (0) to its bottom (1)."""
        last = len(profile) - 1

        def point(i, j):
            radius, height = profile[i]
            angle = 2 * math.pi * j / segments
            position = (
                centre[0] + radius * math.cos(angle),
                centre[1] + height,
                centre[2] + radius * math.sin(angle),
            )
            return position, (j / segments, 1.0 - i / last)

        self.grid(material, last, segments, point)


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def normalise(v):
    length = math.sqrt(sum(c * c for c in v)) or 1.0
    return tuple(c / length for c in v)


def segment_blocked(origin, direction, length, box):
    """Whether the segment from `origin` along the unit `direction` for `length` meets `box`."""
    near, far = 0.0, length
    for axis in range(3):
        d = direction[axis]
        low, high = box[0][axis], box[1][axis]
        if abs(d) < 1e-9:
            if origin[axis] < low or origin[axis] > high:
                return False
            continue
        a = (low - origin[axis]) / d
        b = (high - origin[axis]) / d
        if a > b:
            a, b = b, a
        near, far = max(near, a), min(far, b)
        if near > far:
            return False
    return True


def hemisphere(count):
    """`count` directions spread evenly over the upper hemisphere, cosine weighted, so each stands
    for an equal share of the light a surface facing +Y receives."""
    golden = math.pi * (3 - math.sqrt(5))
    directions = []
    for index in range(count):
        radius = math.sqrt((index + 0.5) / count)
        angle = index * golden
        directions.append((radius * math.cos(angle), math.sqrt(1 - radius * radius), radius * math.sin(angle)))
    return directions


def bake_lighting(mesh, samples, half, height):
    """Lights the receivers of the bake, writing the result into their vertex colours.

    `samples` are points on the room's light sources, as (position, the direction the source faces,
    strength): along each ceiling panel, and across each window.

    Two parts. Direct light from the samples, falling off with distance and angle, with soft
    shadows wherever a blocker stands between: pools of light under the panels and in front of the
    windows, shadows round the furniture. And ambient light, dimmed by how much of the sky above each point
    is closed off nearby by the blockers, the walls, the floor and the ceiling: darker corners,
    the foot of every wall, and contact shadows under the furniture.

    The floor takes both, normalised so its brightest few percent are white. The walls already
    brighten toward the ceiling in their textures, so they take only the darkening: shadow and
    occlusion.
    """
    directions = hemisphere(24)
    reach = 1.5

    def basis(normal):
        # Two axes across the normal, for turning the hemisphere to face it.
        reference = (0.0, 1.0, 0.0) if abs(normal[1]) < 0.9 else (1.0, 0.0, 0.0)
        tangent = normalise(cross(reference, normal))
        return tangent, cross(normal, tangent)

    def leaves_room(origin, direction):
        # Distance to the nearest wall, floor or ceiling along the ray.
        distance = math.inf
        for axis, low, high in ((0, -half, half), (1, 0.0, height), (2, -half, half)):
            d = direction[axis]
            if d > 1e-9:
                distance = min(distance, (high - origin[axis]) / d)
            elif d < -1e-9:
                distance = min(distance, (low - origin[axis]) / d)
        return distance

    results = []
    for index, position, normal in mesh.receivers:
        origin = tuple(p + 0.01 * n for p, n in zip(position, normal))
        nearby = [box for box in mesh.blockers
                  if all(box[0][a] - reach <= origin[a] <= box[1][a] + reach for a in range(3))]

        lit, unshadowed = 0.0, 0.0
        for sample, emits, strength in samples:
            offset = tuple(s - o for s, o in zip(sample, origin))
            distance = math.sqrt(sum(c * c for c in offset))
            direction = tuple(c / distance for c in offset)
            facing = sum(d * n for d, n in zip(direction, normal))
            towards = -sum(d * e for d, e in zip(direction, emits))
            if facing <= 0 or towards <= 0:
                continue
            amount = strength * facing * towards / (distance * distance)
            unshadowed += amount
            if not any(segment_blocked(origin, direction, distance, box) for box in mesh.blockers):
                lit += amount

        tangent, bitangent = basis(normal)
        open_sky = 0
        for x, y, z in directions:
            direction = tuple(x * t + y * n + z * b for t, n, b in zip(tangent, normal, bitangent))
            if leaves_room(origin, direction) < reach:
                continue
            if any(segment_blocked(origin, direction, reach, box) for box in nearby):
                continue
            open_sky += 1
        ambient = open_sky / len(directions)

        results.append((index, normal, lit, unshadowed, ambient))

    # White at the brightest few percent of the floor rather than its very brightest point, so the
    # pools in front of the windows can run to white without dimming the rest of the room.
    floor_light = sorted(lit for _, normal, lit, _, _ in results if normal[1] > 0.5)
    brightest = floor_light[int(len(floor_light) * 0.97)] if floor_light else 1.0
    for index, normal, lit, unshadowed, ambient in results:
        if normal[1] > 0.5:
            value = 0.35 * ambient + 0.65 * lit / brightest
        else:
            shadow = lit / unshadowed if unshadowed > 0 else 1.0
            value = (0.55 + 0.45 * ambient) * (0.6 + 0.4 * shadow) / WALL_BAKE_GAIN
        value = max(0.0, min(1.0, value))
        mesh.colours[index] = (value, value, value)


def refine(mesh, longest, skip=()):
    """Splits every triangle with an edge longer than `longest` into four, and the pieces again
    until none has, so a bake into vertex colours has vertices close enough together to show where
    light and shadow change. Neighbours share the new vertices on the edges they share. Materials
    whose names start with one in `skip` are left as they are.

    Triangles with no area, such as the ones at the centre of a disc, are dropped: a bake cannot
    light them, and their vertices would darken the real triangles they share."""
    def distance(a, b):
        return math.dist(mesh.vertices[a * 5 : a * 5 + 3], mesh.vertices[b * 5 : b * 5 + 3])

    for material, indices in mesh.groups.items():
        if material.name.startswith(skip):
            continue
        midpoints = {}

        def midpoint(a, b):
            key = (min(a, b), max(a, b))
            if key not in midpoints:
                mesh.vertices.extend((p + q) / 2 for p, q in zip(mesh.vertices[a * 5 : a * 5 + 5],
                                                                 mesh.vertices[b * 5 : b * 5 + 5]))
                mesh.colours.append((1.0, 1.0, 1.0))
                midpoints[key] = mesh.vertex_count() - 1
            return midpoints[key]

        def area(a, b, c):
            p, q, r = (mesh.vertices[i * 5 : i * 5 + 3] for i in (a, b, c))
            return math.sqrt(sum(x * x for x in cross([y - x for x, y in zip(p, q)],
                                                      [y - x for x, y in zip(p, r)]))) / 2

        pending = [tuple(indices[i : i + 3]) for i in range(0, len(indices), 3)]
        pending = [triangle for triangle in pending if area(*triangle) > 1e-9]
        done = []
        while pending:
            a, b, c = pending.pop()
            if max(distance(a, b), distance(b, c), distance(c, a)) <= longest:
                done.append((a, b, c))
                continue
            ab, bc, ca = midpoint(a, b), midpoint(b, c), midpoint(c, a)
            pending += [(a, ab, ca), (ab, b, bc), (ca, bc, c), (ab, bc, ca)]
        mesh.groups[material] = [index for triangle in done for index in triangle]


def find_blender():
    """Blender's executable: $BLENDER, else on the path, else the newest standard install."""
    if os.environ.get("BLENDER"):
        return os.environ["BLENDER"]
    found = shutil.which("blender")
    if found:
        return found
    installs = sorted(glob.glob(r"C:\Program Files\Blender Foundation\Blender *\blender.exe"))
    return installs[-1] if installs else None


# The floor's typical brightness in the Blender bake, which the carpet's texture is made lighter to
# suit (see FLOOR_BAKE_GAIN), and how much of the colour of bounced light to keep.
FLOOR_BRIGHTNESS = 0.7
BOUNCE_TINT = 0.4

# Surfaces that give light rather than receive it, left as they are: their textures are what they
# show.
NOT_BAKED = ("ceiling", "ceiling light", "tactical map", "card light", "memory light", "pc light",
             "window view")


def bake_with_blender(mesh, blender):
    """Lights the room in Blender and writes the result into its vertex colours.

    The room goes to Blender as it will be written, unlit, with a list of its light sources;
    `bake_in_blender.py` sends back the light on each vertex. That is applied to everything but the
    surfaces in `NOT_BAKED`, scaled so the floor is as bright on average as the bake here makes it,
    and with the colour of light bounced off the navy walls kept to a tint.

    Vertices with something lying on them, such as the floor under the rug or a wall behind a
    picture, cannot be seen. They take the light of their visible neighbours, so their darkness
    does not bleed out round the edges of whatever covers them."""
    refine(mesh, 0.3, NOT_BAKED)
    order = [index for used in primitive_vertices(mesh) for index in used]

    with tempfile.TemporaryDirectory() as folder:
        room = os.path.join(folder, "room.gltf")
        lights = os.path.join(folder, "lights.json")
        output = os.path.join(folder, "light.bin")
        gltf, _, _ = write_gltf(mesh)
        with open(room, "w", encoding="utf-8") as handle:
            json.dump(gltf, handle)
        with open(lights, "w", encoding="utf-8") as handle:
            json.dump([{key: value for key, value in light.items() if key != "samples"}
                       for light in mesh.lights], handle)

        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "bake_in_blender.py")
        result = subprocess.run(
            [blender, "--background", "--factory-startup", "--python-exit-code", "1",
             "--python", script, "--", room, lights, output],
            capture_output=True, text=True,
        )
        if result.returncode != 0 or not os.path.exists(output):
            raise SystemExit(f"Blender failed to bake:\n{result.stdout[-3000:]}\n{result.stderr[-3000:]}")
        print(next((line for line in result.stdout.splitlines() if line.startswith("Baking on")),
                   "Baked in Blender"))
        with open(output, "rb") as handle:
            data = handle.read()

    values = struct.unpack("<%df" % (len(data) // 4), data)
    if len(values) != 7 * len(order):
        raise SystemExit(f"Blender sent back {len(values) // 7} vertices for {len(order)}")

    skipped = {index for material, indices in mesh.groups.items()
               if material.name.startswith(NOT_BAKED) for index in indices}
    light, hidden = {}, set()
    for slot, index in enumerate(order):
        position = values[slot * 7 : slot * 7 + 3]
        if math.dist(position, mesh.vertices[index * 5 : index * 5 + 3]) > 1e-3:
            raise SystemExit("Blender's vertices are not in the file's order")
        if index not in skipped:
            light[index] = values[slot * 7 + 3 : slot * 7 + 6]
            if values[slot * 7 + 6]:
                hidden.add(index)

    def luminance(colour):
        return 0.2126 * colour[0] + 0.7152 * colour[1] + 0.0722 * colour[2]

    floor = sorted(luminance(light[index]) for material, indices in mesh.groups.items()
                   if material.name == "floor" for index in set(indices))
    scale = FLOOR_BRIGHTNESS / (floor[len(floor) // 2] or 1.0)
    for index, colour in light.items():
        grey = luminance(colour)
        mesh.colours[index] = tuple(min(1.0, scale * (grey + BOUNCE_TINT * (channel - grey)))
                                    for channel in colour)

    neighbours = {}
    for material, indices in mesh.groups.items():
        for i in range(0, len(indices), 3):
            for a, b in itertools.permutations(indices[i : i + 3], 2):
                neighbours.setdefault(a, set()).add(b)
    while hidden:
        revealed = {}
        for index in hidden:
            seen = [mesh.colours[n] for n in neighbours.get(index, ())
                    if n in light and n not in hidden]
            if seen:
                revealed[index] = tuple(sum(channel) / len(seen) for channel in zip(*seen))
        if not revealed:
            break
        for index, colour in revealed.items():
            mesh.colours[index] = colour
        hidden -= set(revealed)


def rotation(pitch=0.0, yaw=0.0):
    """Rotates a point about X by `pitch`, then about Y by `yaw`, both in radians."""

    def apply(point):
        x, y, z = point
        y, z = y * math.cos(pitch) - z * math.sin(pitch), y * math.sin(pitch) + z * math.cos(pitch)
        x, z = x * math.cos(yaw) + z * math.sin(yaw), -x * math.sin(yaw) + z * math.cos(yaw)
        return (x, y, z)

    return apply


# ---------------------------------------------------------------------------------------------
# The brown room
# ---------------------------------------------------------------------------------------------


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


def build_brown(mesh):
    floor = Material("floor", texture=make_floor_texture())
    wall = Material("wall", texture=make_wall_texture())
    ceiling = Material("ceiling", texture=make_ceiling_texture())
    furniture = Material("furniture", texture=make_furniture_texture())

    half, height = 4.0, 2.6

    # Floor and ceiling, wound so they face into the room.
    mesh.quad(
        floor,
        [(-half, 0, half), (half, 0, half), (half, 0, -half), (-half, 0, -half)],
        uv_scale=4.0,
    )
    mesh.quad(
        ceiling,
        [(-half, height, -half), (half, height, -half), (half, height, half), (-half, height, half)],
    )

    # Walls, facing inward. v increases upward, so the skirting drawn at the bottom of the texture
    # lands at floor level.
    mesh.quad(wall, [(-half, 0, -half), (half, 0, -half), (half, height, -half), (-half, height, -half)], 2.0)
    mesh.quad(wall, [(half, 0, half), (-half, 0, half), (-half, height, half), (half, height, half)], 2.0)
    mesh.quad(wall, [(-half, 0, half), (-half, 0, -half), (-half, height, -half), (-half, height, half)], 2.0)
    mesh.quad(wall, [(half, 0, -half), (half, 0, half), (half, height, half), (half, height, -half)], 2.0)

    # Furniture, giving the space depth cues and something to judge scale and parallax against.
    mesh.box(furniture, (-2.2, 0.37, -2.4), (1.6, 0.74, 0.8))   # desk
    mesh.box(furniture, (1.9, 0.55, 1.6), (1.1, 1.1, 1.1))      # cabinet
    mesh.box(furniture, (2.6, 0.22, -2.2), (0.9, 0.44, 0.9))    # low table
    mesh.box(furniture, (-2.9, 1.05, 2.5), (0.5, 2.1, 0.5))     # column
    mesh.box(furniture, (0.2, 0.16, 0.4), (1.4, 0.32, 1.4))     # platform


# ---------------------------------------------------------------------------------------------
# The studio room
# ---------------------------------------------------------------------------------------------

BLUE = rgb("#2969F1")
LIME = rgb("#B4F62A")
WHITE = (255, 255, 255)

STUDIO_HALF = 8.0
STUDIO_HEIGHT = 3.9

# A deep navy such as #06103C reads as black on walls lit only by baked light, so the walls are a
# lighter navy of the same hue. The floor is a grey carpet, the
# ceiling white.
WALL_NAVY = (24, 42, 112)
CARPET_GREY = (176, 179, 186)
RUG_NAVY = (30, 46, 110)
CEILING_WHITE = (196, 202, 216)

# Ceiling lights, centred on these x and z, each this long along x and wide along z, in metres.
LIGHT_POSITIONS = (-5.0, 0.0, 5.0)
LIGHT_SIZE = (2.0, 0.6)


def blend(a, b, t):
    return tuple(int(x + (y - x) * t) for x, y in zip(a, b))


def make_shading_texture(top_light=0.25, bottom_dark=0.35, edge_ao=0.25, size=128, base=215):
    """A white texture holding only baked light: a material's colour multiplies it."""
    image = Image.new("RGB", (size, size), (base, base, base))
    return bake_shading(image, top_light=top_light, bottom_dark=bottom_dark, edge_ao=edge_ao)


def make_plush_texture(width, height, base, spread, seed, tiling=False):
    """Plush pile: soft fibres in shades either side of `base`, a few with a hint of the room's
    blue and lime.

    `tiling` makes the texture repeat without seams, for a floor: it is blurred as a 3 x 3 mosaic
    of itself, so the softening carries across its edges.
    """
    image = Image.new("RGB", (width, height), base)
    draw = ImageDraw.Draw(image)

    random.seed(seed)
    for _ in range(width * height // 10):
        x = random.uniform(0, width)
        y = random.uniform(0, height)
        shade = random.randint(-spread, spread)
        colour = tuple(max(0, min(255, c + shade)) for c in base)
        tint = random.random()
        if tint < 0.04:
            colour = blend(colour, BLUE, 0.35)
        elif tint < 0.06:
            colour = blend(colour, LIME, 0.3)
        radius = random.uniform(0.8, 2.2)
        draw.ellipse([x - radius, y - radius, x + radius, y + radius], fill=colour)

    if not tiling:
        return image.filter(ImageFilter.GaussianBlur(1.1))

    mosaic = Image.new("RGB", (width * 3, height * 3))
    for row in range(3):
        for column in range(3):
            mosaic.paste(image, (column * width, row * height))
    mosaic = mosaic.filter(ImageFilter.GaussianBlur(1.1))
    return mosaic.crop((width, height, width * 2, height * 2))


# The bake darkens the floor everywhere but right under the lights, so the carpet is made lighter
# by this much to keep the room as bright on average.
FLOOR_BAKE_GAIN = 1.25
# The same for the walls, which the bake darkens further, and more where they are lit less: vertex
# colours can only darken a texture, so this is also the headroom for the walls to be lit brighter
# where the light falls on them.
WALL_BAKE_GAIN = 1.6


def make_studio_floor_texture():
    """Four metres of floor: grey carpet with a blue grid every metre."""
    size = 512
    metre = size // 4
    image = make_plush_texture(size, size, CARPET_GREY, 26, seed=17, tiling=True)

    draw = ImageDraw.Draw(image)
    line = blend(CARPET_GREY, BLUE, 0.55)
    for index in range(4):
        draw.line([index * metre, 0, index * metre, size], fill=line, width=3)
        draw.line([0, index * metre, size, index * metre], fill=line, width=3)

    image = image.filter(ImageFilter.GaussianBlur(0.5))
    return image.point(lambda value: min(255, int(value * FLOOR_BAKE_GAIN)))


def make_studio_wall_texture(fan, mirrored=False, width=2048):
    """A navy wall, lit from the ceiling, with the blue line-fan motif across it.

    `fan` is where along the wall, as a fraction of its width, the fan's lines meet, and
    `mirrored` turns the fan round, so each wall can carry it differently.
    """
    # As many pixels per metre up the wall as along it.
    height = round(width * STUDIO_HEIGHT / (2 * STUDIO_HALF))
    image = Image.new("RGB", (width, height), WALL_NAVY)

    # Squares of one size turned step by step about a shared corner at the foot of the wall, their
    # outlines sweeping up and across it.
    draw = ImageDraw.Draw(image)
    pivot = (width * fan, height * 1.0)
    side = height * 1.9
    for step in range(35):
        angle = math.radians(-62 + step * 2.0)
        corners = [(0, 0), (side, 0), (side, -side), (0, -side)]
        points = [
            (
                pivot[0] + x * math.cos(angle) - y * math.sin(angle),
                pivot[1] + x * math.sin(angle) + y * math.cos(angle),
            )
            for x, y in corners
        ]
        draw.line(points + [points[0]], fill=BLUE, width=width // 1024)

    if mirrored:
        image = image.transpose(Image.Transpose.FLIP_LEFT_RIGHT)

    # Lit from the ceiling: brighter toward the top of the wall, a little darker at its foot.
    pixels = image.load()
    for y in range(height):
        light = (1.0 + 0.5 * math.exp(-5.0 * y / height) - 0.3 * (y / height)) * WALL_BAKE_GAIN
        for x in range(width):
            r, g, b = pixels[x, y]
            pixels[x, y] = (min(255, int(r * light)), min(255, int(g * light)), min(255, int(b * light)))

    return image


def make_studio_ceiling_texture(size=1024):
    """The whole ceiling, with a soft glow baked round each light."""
    half = STUDIO_HALF
    image = Image.new("RGB", (size, size), CEILING_WHITE)
    glow = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(glow)

    def pixel(metres):
        return (metres + half) / (2 * half) * size

    for x in LIGHT_POSITIONS:
        for z in LIGHT_POSITIONS:
            reach_x, reach_z = LIGHT_SIZE[0] * 1.2, LIGHT_SIZE[1] * 2.5
            draw.ellipse(
                [pixel(x - reach_x), pixel(z - reach_z), pixel(x + reach_x), pixel(z + reach_z)],
                fill=170,
            )

    glow = glow.filter(ImageFilter.GaussianBlur(size / 30))
    lit = Image.new("RGB", (size, size), (232, 237, 248))
    return Image.composite(lit, image, glow)


def make_light_panel_texture():
    """A ceiling light: bright white with a cool blue rim, so it still stands out on the white
    ceiling."""
    image = Image.new("RGB", (128, 64), (150, 185, 245))
    draw = ImageDraw.Draw(image)
    draw.rectangle([10, 10, 118, 54], fill=(250, 252, 255))
    return image.filter(ImageFilter.GaussianBlur(5))


def make_fur_texture():
    """Long, wispy fur: streaks running down the body, lit from above, darker underneath."""
    width, height = 512, 512
    image = Image.new("RGB", (width, height), (222, 222, 226))
    draw = ImageDraw.Draw(image)
    random.seed(7)
    for _ in range(6000):
        x = random.uniform(0, width)
        y = random.uniform(0, height)
        length = random.uniform(10, 30)
        shade = random.randint(165, 255)
        draw.line([x, y, x + random.uniform(-5, 5), y + length], fill=(shade, shade, min(255, shade + 3)), width=1)

    image = image.filter(ImageFilter.GaussianBlur(0.8))
    return bake_shading(image, top_light=0.18, bottom_dark=0.5, edge_ao=0.0)


def make_rug_texture():
    """A plush dark blue rug, mapped round a disc: u around it, v from its centre (0) to its rim
    (1), the pile darkening a little toward the rim."""
    width, height = 1024, 256
    image = make_plush_texture(width, height, RUG_NAVY, 18, seed=13)

    pixels = image.load()
    for y in range(height):
        rim = max(0.0, (y / height - 0.8) / 0.2)
        light = 1.0 - 0.3 * rim * rim
        for x in range(width):
            r, g, b = pixels[x, y]
            pixels[x, y] = (int(r * light), int(g * light), int(b * light))

    return image


def make_book_texture():
    """A book's covers and spine: pages along one edge, a band near the head and tail."""
    size = 128
    image = Image.new("RGB", (size, size), (220, 220, 220))
    draw = ImageDraw.Draw(image)
    draw.rectangle([0, 14, size, 18], fill=(170, 170, 170))
    draw.rectangle([0, size - 19, size, size - 15], fill=(170, 170, 170))
    return bake_shading(image, top_light=0.1, bottom_dark=0.25, edge_ao=0.15)


def make_colour_chart_texture():
    """A colour checker: six columns of four patches, the last row a grey ramp, on a black card."""
    patches = [
        "#735244", "#C29682", "#627A9D", "#576C43", "#8580B1", "#67BDAA",
        "#D67E2C", "#505BA6", "#C15A63", "#5E3C6C", "#9DBC40", "#E0A32E",
        "#383D96", "#469449", "#AF363C", "#E7C71F", "#BB5695", "#0885A1",
        "#F3F3F2", "#C8C8C8", "#A0A0A0", "#7A7A79", "#555555", "#343434",
    ]
    cell, gap = 96, 14
    width = 6 * cell + 7 * gap
    height = 4 * cell + 5 * gap
    image = Image.new("RGB", (width, height), (24, 24, 26))
    draw = ImageDraw.Draw(image)
    for index, colour in enumerate(patches):
        row, column = divmod(index, 6)
        x = gap + column * (cell + gap)
        y = gap + row * (cell + gap)
        draw.rounded_rectangle([x, y, x + cell, y + cell], radius=6, fill=rgb(colour))
    return image


def add_cat(mesh, materials, position, facing):
    """A long-haired grey cat, after a Persian plush, lying on its side with its belly and paws
    toward the viewer, its face framed by a ruff of long fur, its head low enough to lay its chin
    on something in front of it, and its fluffy tail curled round to its hind paws. Built head
    toward +X, belly toward -Z, then turned by `facing` radians.

    Made of overlapping shapes, every one sunk well into its neighbours, so the cat reads as one
    furry body rather than a stack of balls."""
    fur, paw, ear, eye, pupil, nose = materials
    turn = rotation(yaw=facing)
    mesh.transform = lambda p: tuple(c + o for c, o in zip(turn(p), position))

    # Head, low enough to rest its chin on a model plane, looking toward -Z: round, with full
    # cheeks, a short muzzle and big eyes.
    hx, hy, hz = 0.24, 0.125, -0.03

    def head(offset):
        return (hx + offset[0], hy + offset[1], hz + offset[2])

    # Body and haunch resting on the side; a deep chest joining them to the head, so there is no
    # neck to see; and the ruff, a mane of fur behind and round the face, wider and taller than
    # the head so the face looks out of it.
    mesh.ellipsoid(fur, (-0.01, 0.09, 0.0), (0.2, 0.09, 0.115))
    mesh.ellipsoid(fur, (-0.14, 0.095, 0.01), (0.115, 0.095, 0.105))
    mesh.ellipsoid(fur, (0.14, 0.095, -0.005), (0.11, 0.09, 0.1))
    mesh.ellipsoid(fur, head((-0.015, -0.01, 0.025)), (0.1, 0.095, 0.08))

    # Short, thick legs, as a long-haired cat's look: the front pair reaching toward the viewer,
    # the upper over the lower, and the hind pair tucked forward under a full thigh.
    mesh.ellipsoid(fur, (-0.15, 0.1, -0.06), (0.085, 0.075, 0.06))
    for shoulder, paw_at, thickness in [
        ((0.12, 0.05, -0.07), (0.18, 0.03, -0.19), 0.045),
        ((0.09, 0.1, -0.07), (0.13, 0.07, -0.18), 0.042),
        ((-0.17, 0.05, -0.08), (-0.12, 0.03, -0.19), 0.042),
        ((-0.12, 0.12, -0.08), (-0.06, 0.08, -0.18), 0.04),
    ]:
        mesh.tube(fur, [shoulder, paw_at], [thickness, thickness * 0.78])
        mesh.ellipsoid(paw, paw_at, (0.034, 0.024, 0.04))

    mesh.ellipsoid(fur, head((0.0, 0.0, 0.0)), (0.082, 0.074, 0.072))
    for side in (-1, 1):
        mesh.ellipsoid(fur, head((side * 0.045, -0.025, -0.02)), (0.042, 0.04, 0.042))
    mesh.ellipsoid(fur, head((0.0, -0.028, -0.06)), (0.03, 0.022, 0.02))
    mesh.ellipsoid(nose, head((0.0, -0.012, -0.075)), (0.009, 0.006, 0.005))
    for side in (-1, 1):
        mesh.ellipsoid(eye, head((side * 0.03, 0.012, -0.062)), (0.019, 0.017, 0.009), rings=8, segments=12)
        mesh.ellipsoid(pupil, head((side * 0.03, 0.012, -0.07)), (0.006, 0.011, 0.003), rings=6, segments=8)
    # Small ears, set wide and half buried in the fur.
    for side in (-1, 1):
        base = head((side * 0.05, 0.06, 0.01))
        tip = head((side * 0.07, 0.11, 0.005))
        mesh.cone(fur, base, tip, 0.026)
        mesh.cone(ear, (base[0], base[1] + 0.002, base[2] - 0.006), (tip[0], tip[1] - 0.008, tip[2] - 0.004), 0.015)

    # A thick, fluffy tail growing out of the haunch, lying along the shelf behind the cat and
    # curling round to its hind paws: a smooth curve through these points.
    points = [(-0.18, 0.09, 0.03), (-0.28, 0.07, 0.04), (-0.35, 0.05, -0.02),
              (-0.35, 0.045, -0.11), (-0.28, 0.04, -0.18), (-0.19, 0.04, -0.2)]
    path = []
    for i in range(len(points) - 1):
        p0, p1, p2 = points[max(i - 1, 0)], points[i], points[i + 1]
        p3 = points[min(i + 2, len(points) - 1)]
        for step in range(6):
            t = step / 6
            path.append(tuple(
                0.5 * (2 * b + (c - a) * t + (2 * a - 5 * b + 4 * c - d) * t * t
                       + (3 * b - a - 3 * c + d) * t * t * t)
                for a, b, c, d in zip(p0, p1, p2, p3)
            ))
    path.append(points[-1])
    mesh.tube(fur, path, [0.055 - 0.022 * index / (len(path) - 1) for index in range(len(path))])

    mesh.transform = None


SHELF_SIZE = (2.2, 2.0, 0.34, 0.03)  # width, height, depth, board thickness


def add_shelf_frame(mesh, frame, back, shelves):
    """The carcass of a shelf unit in its own frame: open front along -Z at z = 0, back panel at
    the far side. Returns the height of each shelf's top face, bottom up."""
    width, height, depth, board = SHELF_SIZE

    mesh.block((0.0, height / 2, depth / 2), (width, height, depth))
    mesh.box(back, (0.0, height / 2, depth - 0.01), (width - 2 * board, height, 0.02))
    for side in (-1, 1):
        mesh.box(frame, (side * (width / 2 - board / 2), height / 2, depth / 2), (board, height, depth))
    floors = []
    for index in range(shelves + 1):
        y = index * (height - board) / shelves + board / 2
        mesh.box(frame, (0.0, y, depth / 2), (width - 2 * board, board, depth))
        floors.append(y + board / 2)
    return floors[:-1]


def place(mesh, position, yaw=0.0, scale=1.0, pitch=0.0):
    """Sets `mesh.transform` so what follows is built around its own origin, then scaled, turned
    and moved to `position`."""
    orient = rotation(pitch=pitch, yaw=yaw)
    mesh.transform = lambda p: tuple(
        c + v for c, v in zip(position, orient(tuple(scale * x for x in p)))
    )


def add_bookshelf(mesh, materials, origin, facing, seed):
    """A white bookshelf against a wall, filled with books of the room's and other colours."""
    shelf_material, book_materials = materials
    turn = rotation(yaw=facing)
    mesh.transform = lambda p: tuple(c + o for c, o in zip(turn(p), origin))

    width, height, depth, board = SHELF_SIZE
    shelves = 5
    add_shelf_frame(mesh, shelf_material, shelf_material, shelves)

    random.seed(seed)
    inner = width - 2 * board
    for index in range(shelves):
        floor = index * (height - board) / shelves + board
        clearance = (height - board) / shelves - board
        x = -inner / 2 + 0.03
        while x < inner / 2 - 0.08:
            thickness = random.uniform(0.025, 0.06)
            tall = random.uniform(0.55, 0.92) * clearance
            book_depth = random.uniform(0.17, 0.26)
            material = random.choice(book_materials)

            if random.random() < 0.08 and x > -inner / 2 + 0.2:
                # A gap, and now and then a small stack lying flat where there is room for one.
                if random.random() < 0.5 and x + 0.25 < inner / 2:
                    for layer in range(random.randint(2, 4)):
                        lying = random.uniform(0.03, 0.045)
                        mesh.box(
                            random.choice(book_materials),
                            (x + 0.12, floor + lying / 2 + layer * lying, depth / 2),
                            (0.22, lying, random.uniform(0.16, 0.22)),
                        )
                    x += 0.27
                else:
                    x += random.uniform(0.08, 0.15)
                continue

            mesh.box(material, (x + thickness / 2, floor + tall / 2, depth / 2 + (0.26 - book_depth) / 2 - 0.02),
                     (thickness, tall, book_depth))
            x += thickness + 0.003

    mesh.transform = None


def add_plant(mesh, materials, position, seed, size=1.0):
    """A potted plant: a white pot and a bushy crown of leaves. A `size` well above 1 makes a small
    indoor tree."""
    pot, soil, leaves, stem = materials
    x, y, z = position
    k = size

    mesh.tube(pot, [(x, y, z), (x, y + 0.4 * k, z)], [0.16 * k, 0.2 * k], segments=24)
    mesh.block((x, y + 0.2 * k, z), (0.36 * k, 0.4 * k, 0.36 * k))
    mesh.disc(soil, (x, y + 0.38 * k, z), 0.18 * k)

    mesh.tube(stem, [(x, y + 0.35 * k, z), (x + 0.02 * k, y + 0.8 * k, z - 0.01 * k)],
              [0.018 * k, 0.012 * k])

    random.seed(seed)
    for _ in range(int(26 * min(k, 2.0))):
        angle = random.uniform(0, 2 * math.pi)
        height = random.uniform(0.6, 1.25)
        reach = random.uniform(0.08, 0.32) * (1.3 - abs(height - 0.95))
        centre = (x + k * reach * math.cos(angle), y + k * height, z + k * reach * math.sin(angle))
        mesh.ellipsoid(
            leaves,
            centre,
            (0.12 * k, 0.025 * k, 0.05 * k),
            rings=6,
            segments=10,
            rotate=rotation(pitch=random.uniform(-0.6, 0.6), yaw=angle),
        )


def add_plane(mesh, materials, position, yaw, pitch=0.0, scale=1.0):
    """A model airliner: a white fuselage, swept wings with an engine under each, and a coloured
    tail. About half a metre long at a scale of 1, nose along its own -Z, turned by `yaw` and tipped
    nose up by a negative `pitch`."""
    body, tail, engine = materials
    orient = rotation(pitch=pitch, yaw=yaw)

    def place(part=None, offset=(0.0, 0.0, 0.0)):
        """Builds what follows rotated by `part` about the plane's origin, then moved by `offset`,
        then placed and turned with the whole plane."""

        def transform(point):
            if part is not None:
                point = part(point)
            point = tuple(scale * (p + o) for p, o in zip(point, offset))
            return tuple(c + v for c, v in zip(position, orient(point)))

        mesh.transform = transform

    place()
    mesh.ellipsoid(body, (0.0, 0.0, 0.0), (0.032, 0.034, 0.25))

    for side in (-1, 1):
        # Wings and tailplanes are swept back by turning them about their roots.
        place(rotation(yaw=-side * 0.5), (0.0, -0.012, 0.02))
        mesh.box(body, (side * 0.13, 0.0, 0.0), (0.24, 0.008, 0.075))

        place(rotation(yaw=-side * 0.55), (0.0, 0.006, 0.2))
        mesh.box(tail, (side * 0.045, 0.0, 0.0), (0.08, 0.005, 0.035))

        place(offset=(side * 0.085, -0.032, -0.03))
        mesh.tube(engine, [(0.0, 0.0, -0.035), (0.0, 0.0, 0.045)], [0.016, 0.013], segments=12)

    # The fin, leaning back.
    place(rotation(pitch=0.55), (0.0, 0.05, 0.205))
    mesh.box(tail, (0.0, 0.0, 0.0), (0.006, 0.085, 0.06))

    mesh.transform = None


def add_helicopter(mesh, materials, position, yaw, scale=1.0):
    """A model helicopter: a white cabin with a tinted windscreen, a coloured engine cowling and
    fin, a four-bladed rotor and skids. About half a metre long at a scale of 1, nose along -Z,
    its skids 0.07 below its origin."""
    body, accent, glass, rotor = materials
    place(mesh, position, yaw, scale)

    mesh.ellipsoid(body, (0.0, 0.0, -0.06), (0.05, 0.055, 0.11))
    mesh.ellipsoid(glass, (0.0, 0.012, -0.115), (0.043, 0.036, 0.06))
    mesh.ellipsoid(accent, (0.0, 0.045, -0.02), (0.03, 0.022, 0.065))
    mesh.tube(body, [(0.0, 0.015, 0.02), (0.0, 0.035, 0.3)], [0.024, 0.009])
    mesh.box(accent, (0.0, 0.06, 0.3), (0.006, 0.07, 0.035))

    # Rotors: flat blades, which the unculled renderer shows from both sides.
    hub = (0.0, 0.09, -0.05)
    mesh.tube(rotor, [(0.0, 0.05, -0.05), hub], [0.008, 0.008], segments=8)
    mesh.ellipsoid(rotor, hub, (0.014, 0.01, 0.014), rings=6, segments=10)
    for blade in range(4):
        angle = 0.4 + blade * math.pi / 2
        along = (math.cos(angle), 0.0, math.sin(angle))
        across = (-math.sin(angle), 0.0, math.cos(angle))
        mesh.quad(rotor, [
            tuple(h + reach * a + side * 0.011 * c for h, a, c in zip(hub, along, across))
            for reach, side in [(0.02, -1), (0.31, -1), (0.31, 1), (0.02, 1)]
        ])
    x = 0.013
    mesh.quad(rotor, [(x, 0.02, 0.294), (x, 0.1, 0.294), (x, 0.1, 0.306), (x, 0.02, 0.306)])
    mesh.quad(rotor, [(x, 0.054, 0.26), (x, 0.054, 0.34), (x, 0.066, 0.34), (x, 0.066, 0.26)])

    for side in (-1, 1):
        x = side * 0.045
        mesh.tube(rotor, [(x, -0.055, -0.19), (x, -0.07, -0.16), (x, -0.07, 0.07)], [0.004] * 3,
                  segments=8)
        for z in (-0.11, 0.02):
            mesh.tube(rotor, [(side * 0.025, -0.03, z), (x, -0.07, z)], [0.003, 0.003], segments=6)

    mesh.transform = None


def add_tank(mesh, materials, position, yaw, scale=1.0):
    """A model battle tank: tracks on road wheels, a hull with fenders, a turret and a long gun.
    About 0.36 m long without the gun at a scale of 1, gun along -Z, standing on its origin."""
    hull, track = materials
    place(mesh, position, yaw, scale)

    for side in (-1, 1):
        mesh.box(track, (side * 0.075, 0.026, 0.0), (0.04, 0.05, 0.34))
        for z in (-0.12, -0.06, 0.0, 0.06, 0.12):
            mesh.tube(hull, [(side * 0.094, 0.026, z), (side * 0.098, 0.026, z)], [0.019, 0.019],
                      segments=12)
    mesh.box(hull, (0.0, 0.04, 0.0), (0.11, 0.04, 0.33))
    mesh.box(hull, (0.0, 0.066, 0.01), (0.2, 0.018, 0.31))

    mesh.box(hull, (0.0, 0.088, 0.04), (0.12, 0.03, 0.12))
    mesh.ellipsoid(hull, (0.0, 0.095, 0.02), (0.07, 0.028, 0.085))
    mesh.tube(track, [(0.028, 0.115, 0.05), (0.028, 0.127, 0.05)], [0.016, 0.016], segments=12)
    mesh.tube(hull, [(0.0, 0.097, -0.05), (0.0, 0.097, -0.28)], [0.008, 0.006], segments=10)
    mesh.tube(track, [(0.0, 0.097, -0.27), (0.0, 0.097, -0.295)], [0.01, 0.01], segments=10)

    mesh.transform = None


def add_headset(mesh, materials, position, yaw):
    """A mixed reality headset, after the HoloLens: a tinted visor under a white brow housing,
    hung from a band that dips toward the back of the head. Life size, visor to -Z, resting on
    its origin."""
    shell, visor, band = materials
    place(mesh, position, yaw)

    def front(material, radius, bottom, top, spread):
        def point(i, j):
            angle = -math.pi / 2 + (j / 16 - 0.5) * 2 * spread
            y = bottom + (top - bottom) * i
            return (radius * math.cos(angle), y, 0.02 + radius * math.sin(angle)), (j / 16, 1.0 - i)

        mesh.grid(material, 1, 16, point)

    front(visor, 0.1, 0.0, 0.06, 1.2)
    front(shell, 0.104, 0.055, 0.1, 1.35)

    tilt = rotation(pitch=0.35)
    loop = []
    for step in range(33):
        angle = 2 * math.pi * step / 32
        x, y, z = tilt((0.1 * math.cos(angle), 0.0, 0.12 * math.sin(angle)))
        loop.append((x, y + 0.06, z + 0.03))
    mesh.tube(band, loop, [0.014] * len(loop), segments=8)

    mesh.transform = None


def add_quest(mesh, materials, position, yaw):
    """A standalone VR headset in the shape of a Meta Quest 3: a white body with rounded sides, a
    white front panel with three dark sensor pills, a face cushion, and soft straps round the sides
    and over the top. Life size, front to -Z, resting on its origin."""
    shell, front, sensor, strap = materials
    place(mesh, position, yaw)

    bottom, top = 0.012, 0.108
    middle = (bottom + top) / 2
    mesh.box(shell, (0.0, middle, 0.0), (0.14, top - bottom, 0.07))
    for side in (-1, 1):
        mesh.tube(shell, [(side * 0.07, bottom, 0.0), (side * 0.07, top, 0.0)], [0.035, 0.035], segments=16)
    mesh.box(front, (0.0, middle, -0.0365), (0.16, 0.074, 0.004))
    for x in (-0.05, 0.0, 0.05):
        mesh.box(sensor, (x, middle, -0.0392), (0.012, 0.03, 0.002))
    mesh.box(strap, (0.0, middle, 0.045), (0.15, 0.078, 0.02))

    sides = [(0.1 * math.cos(a), middle, 0.03 + 0.17 * math.sin(a))
             for a in [math.pi * step / 16 for step in range(17)]]
    mesh.tube(strap, sides, [0.011] * len(sides), segments=8)
    over = [(0.0, top - 0.005 + 0.05 * math.sin(math.pi * t), 0.02 + 0.18 * t)
            for t in [step / 10 for step in range(11)]]
    mesh.tube(strap, over, [0.009] * len(over), segments=8)

    mesh.transform = None


def add_syrup(mesh, materials, position, yaw=0.0):
    """A small glass bottle of maple syrup, its maple-leaf label facing -Z before `yaw`."""
    glass, label, cap = materials
    place(mesh, position, yaw)
    origin = (0.0, 0.0, 0.0)
    mesh.lathe(glass, origin, [(0.0, 0.0), (0.033, 0.0), (0.036, 0.006), (0.036, 0.11),
                               (0.03, 0.128), (0.013, 0.148), (0.011, 0.168)])
    mesh.lathe(label, origin, [(0.0372, 0.025), (0.0372, 0.09)])
    mesh.tube(cap, [(0.0, 0.162, 0.0), (0.0, 0.184, 0.0)], [0.0135, 0.0135], segments=16)
    mesh.transform = None


def make_cereal_texture(kind):
    """A clear canister as the unlit renderer can show one: pale plastic with a highlight above,
    the cereal seen through it below. Mapped by `lathe`, so v runs down the canister and the
    front, -Z, is at u = 0.75."""
    width, height = 512, 256
    image = Image.new("RGB", (width, height), (222, 228, 236))
    draw = ImageDraw.Draw(image)
    for x in range(width):
        # Brighter where the canister turns toward the viewer, a streak just off its front.
        facing = math.cos(2 * math.pi * (x / width - 0.75))
        light = int(18 * max(facing, 0.0) + 30 * math.exp(-((x / width - 0.7) / 0.015) ** 2))
        draw.line([x, 0, x, height], fill=tuple(min(255, c + light) for c in (222, 228, 236)))

    random.seed(29 if kind == "flakes" else 31)
    level = int(height * 0.42)
    if kind == "flakes":
        base, pieces = (214, 150, 46), [(236, 186, 74), (204, 134, 36), (246, 206, 108), (186, 116, 30)]
    else:
        base, pieces = (88, 50, 30), [(104, 60, 34), (74, 40, 24), (122, 74, 42)] * 3 + [(236, 226, 204)] * 2
    draw.rectangle([0, level + 6, width, height], fill=base)
    for _ in range(2600):
        x = random.uniform(0, width)
        y = random.uniform(level - 4 + 6 * math.sin(x / 23), height)
        colour = random.choice(pieces)
        if kind == "flakes":
            rx, ry = random.uniform(5, 11), random.uniform(3, 6)
            angle = random.uniform(0, math.pi)
            points = [(x + rx * math.cos(a) * math.cos(angle) - ry * math.sin(a) * math.sin(angle),
                       y + rx * math.cos(a) * math.sin(angle) + ry * math.sin(a) * math.cos(angle))
                      for a in [k * math.pi / 4 + random.uniform(-0.25, 0.25) for k in range(8)]]
            draw.polygon(points, fill=colour)
        else:
            r = random.uniform(3.5, 5)
            draw.ellipse([x - r, y - r, x + r, y + r], fill=colour)
            draw.ellipse([x - r * 0.5, y - r * 0.7, x - r * 0.1, y - r * 0.3],
                         fill=tuple(min(255, c + 40) for c in colour))
    return image


def add_cereal_dispenser(mesh, materials, position, yaw=0.0):
    """A double cereal dispenser: two clear canisters on black columns joined by a base, each
    with a chrome three-spoked knob and a clear cup over the base below it. Life size, knobs to
    -Z before `yaw`, standing on its origin."""
    plastic, chrome, cup, canisters = materials
    place(mesh, position, yaw)

    # The base, a black slab with rounded ends.
    mesh.box(plastic, (0.0, 0.015, 0.0), (0.27, 0.03, 0.16))
    for side in (-1, 1):
        mesh.tube(plastic, [(side * 0.135, 0.0, 0.0), (side * 0.135, 0.03, 0.0)], [0.08, 0.08], segments=24)

    for side, canister in zip((-1, 1), canisters):
        x = side * 0.105
        mesh.box(plastic, (x, 0.13, 0.035), (0.075, 0.2, 0.07))
        mesh.box(plastic, (x, 0.255, 0.0), (0.105, 0.07, 0.1))

        # Canister, its sides evenly spaced in height so the cereal line in its texture is level.
        bottom, top = 0.29, 0.47
        profile = []
        for step in range(13):
            t = step / 12
            radius = 0.05 + 0.04 * min(t / 0.35, 1.0) ** 0.6
            profile.append((radius, bottom + (top - bottom) * t))
        mesh.lathe(canister, (x, 0.0, 0.0), profile, segments=32)
        mesh.tube(plastic, [(x, top - 0.005, 0.0), (x, top + 0.025, 0.0)], [0.094, 0.094], segments=32)
        mesh.tube(plastic, [(x, top + 0.025, 0.0), (x, top + 0.035, 0.0)], [0.05, 0.05], segments=24)

        # Knob: a hub and three spokes on the front of the collar.
        hub = (x, 0.25, -0.065)
        mesh.tube(chrome, [(x, 0.25, -0.05), hub], [0.016, 0.016], segments=12)
        for spoke in range(3):
            angle = math.pi / 2 + spoke * 2 * math.pi / 3
            end = (x + 0.045 * math.cos(angle), 0.25 + 0.045 * math.sin(angle), -0.068)
            mesh.tube(chrome, [hub, end], [0.01, 0.017], segments=10)

        mesh.lathe(cup, (x, 0.155, -0.045), [(0.025, 0.0), (0.04, 0.03), (0.042, 0.06)], segments=24)

    mesh.transform = None




def add_fruit_bowl(mesh, materials, position):
    """A wide bowl holding a bunch of bananas, their stems joined at a crown."""
    bowl, peel, tip = materials
    mesh.lathe(bowl, position, [(0.0, 0.0), (0.06, 0.0), (0.075, 0.005), (0.115, 0.035),
                                (0.138, 0.075), (0.142, 0.082)], segments=32)

    crown = (position[0] - 0.09, position[1] + 0.07, position[2] + 0.01)
    bend, length = 0.95, 0.2
    radius = length / bend
    start = -bend / 2
    for spread, lift in [(-0.42, 0.0), (-0.14, 0.012), (0.14, 0.006), (0.42, 0.0)]:
        # Each banana leaves the crown towards +X, curving up at its far end.
        place(mesh, crown, yaw=math.pi / 2 + spread, pitch=0.25)
        path, radii = [], []
        for step in range(13):
            t = step / 12
            angle = start + bend * t
            path.append((0.0, lift + radius * (math.cos(start) - math.cos(angle)),
                         radius * (math.sin(angle) - math.sin(start))))
            radii.append(0.017 * (0.3 + 0.7 * math.sin(math.pi * min(max(t, 0.04), 0.96)) ** 0.5))
        mesh.tube(peel, path, radii)
        mesh.ellipsoid(tip, path[-1], (0.005, 0.005, 0.005), rings=4, segments=6)
        mesh.tube(tip, [(0.0, lift, -0.014), path[0]], [0.005, 0.006], segments=6)

    mesh.transform = None


def make_syrup_label_texture():
    """A cream label with a red maple leaf where `lathe` puts -Z, between red bands."""
    width, height = 512, 140
    image = Image.new("RGB", (width, height), (246, 239, 222))
    draw = ImageDraw.Draw(image)
    red = (200, 34, 40)
    draw.rectangle([0, 0, width, 9], fill=red)
    draw.rectangle([0, height - 10, width, height], fill=red)
    leaf = [(0, 1.0), (0.12, 0.75), (0.28, 0.82), (0.24, 0.45), (0.48, 0.62), (0.52, 0.5),
            (0.75, 0.55), (0.66, 0.3), (0.8, 0.24), (0.45, -0.05), (0.52, -0.2), (0.05, -0.15),
            (0.05, -0.5), (-0.05, -0.5), (-0.05, -0.15), (-0.52, -0.2), (-0.45, -0.05),
            (-0.8, 0.24), (-0.66, 0.3), (-0.75, 0.55), (-0.52, 0.5), (-0.48, 0.62),
            (-0.24, 0.45), (-0.28, 0.82), (-0.12, 0.75)]
    cx, cy, size = width * 0.75, height * 0.5 - 4, 52
    draw.polygon([(cx + size * x, cy - size * y) for x, y in leaf], fill=red)
    return image


def make_foosball_field_texture():
    """A foosball pitch in the room's blue, mown in bands, with white markings: u across the
    pitch, v along it."""
    width, height = 420, 720
    field = blend(BLUE, (0, 0, 0), 0.25)
    image = Image.new("RGB", (width, height), field)
    draw = ImageDraw.Draw(image)
    for band in range(0, 8, 2):
        draw.rectangle([0, band * height / 8, width, (band + 1) * height / 8],
                       fill=blend(field, WHITE, 0.06))

    line, inset = (236, 240, 248), 14
    draw.rectangle([inset, inset, width - inset, height - inset], outline=line, width=4)
    draw.line([inset, height / 2, width - inset, height / 2], fill=line, width=4)
    draw.ellipse([width / 2 - 60, height / 2 - 60, width / 2 + 60, height / 2 + 60], outline=line,
                 width=4)
    for top in (inset, height - inset - 90):
        draw.rectangle([width / 2 - 110, top, width / 2 + 110, top + 90], outline=line, width=4)
    return bake_shading(image, top_light=0.0, bottom_dark=0.0, edge_ao=0.35)


def add_foosball(mesh, materials, position, yaw):
    """A foosball table, its pitch along its own Z and its rods across it, each team's handles
    on its own side."""
    cabinet, field, rod, handle, teams, ball, goal = materials
    place(mesh, position, yaw)

    length, width, top, wall = 1.2, 0.7, 0.8, 0.04
    mesh.block((0.0, top - 0.06, 0.0), (width + 2 * wall, 0.28, length + 2 * wall))
    for x in (-1, 1):
        for z in (-1, 1):
            mesh.box(cabinet, (x * (width / 2 - 0.03), (top - 0.2) / 2, z * (length / 2 - 0.03)),
                     (0.07, top - 0.2, 0.07))
    mesh.box(cabinet, (0.0, top - 0.1, 0.0), (width, 0.2, length))
    for side in (-1, 1):
        mesh.box(cabinet, (side * (width / 2 + wall / 2), top + 0.02, 0.0), (wall, 0.24, length + 2 * wall))
        mesh.box(cabinet, (0.0, top + 0.02, side * (length / 2 + wall / 2)), (width, 0.24, wall))
        mesh.box(goal, (0.0, top + 0.04, side * (length / 2 - 0.002)), (0.2, 0.08, 0.004))
    y = top + 0.002
    mesh.quad(field, [(-width / 2, y, -length / 2), (width / 2, y, -length / 2),
                      (width / 2, y, length / 2), (-width / 2, y, length / 2)],
              uvs=[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)])

    # The usual line-up from one goal to the other: keeper, defence, the other side's attack,
    # both midfields, attack, the other side's defence and keeper.
    rod_y = top + 0.085
    reach = width / 2 + 0.3
    lineup = [(1, 0), (2, 0), (3, 1), (5, 0), (5, 1), (3, 0), (2, 1), (1, 1)]
    for index, (players, team) in enumerate(lineup):
        z = -length / 2 + length * (index + 0.5) / len(lineup)
        mesh.tube(rod, [(-reach, rod_y, z), (reach, rod_y, z)], [0.008, 0.008], segments=8)
        side = -1 if team == 0 else 1
        mesh.tube(handle, [(side * (reach - 0.12), rod_y, z), (side * reach, rod_y, z)],
                  [0.017, 0.017], segments=12)

        spacing = width / max(players + 0.6, 2.4)
        for player in range(players):
            x = (player - (players - 1) / 2) * spacing
            mesh.box(teams[team], (x, rod_y - 0.02, z), (0.032, 0.1, 0.022))
            mesh.box(teams[team], (x, top + 0.014, z), (0.026, 0.02, 0.036))
            mesh.ellipsoid(teams[team], (x, rod_y + 0.05, z), (0.018, 0.018, 0.018), rings=8,
                           segments=12)

    mesh.ellipsoid(ball, (0.06, top + 0.018, 0.12), (0.017, 0.017, 0.017), rings=8, segments=12)
    mesh.transform = None


def add_sofa(mesh, materials, position, yaw):
    """A three-seat sofa with two seat and two back cushions and a throw pillow at each end.
    Front along -Z before `yaw`, its back against whatever is behind +Z, standing on its origin."""
    fabric, cushion, pillows, feet = materials
    place(mesh, position, yaw)

    length, depth = 2.0, 0.9
    mesh.block((0.0, 0.3, 0.0), (length, 0.5, depth))
    mesh.block((0.0, 0.575, depth / 2 - 0.1), (length, 0.55, 0.2))
    for x in (-1, 1):
        for z in (-1, 1):
            mesh.box(feet, (x * (length / 2 - 0.08), 0.025, z * (depth / 2 - 0.08)), (0.05, 0.05, 0.05))
    mesh.box(fabric, (0.0, 0.175, 0.0), (length, 0.25, depth))
    mesh.box(fabric, (0.0, 0.575, depth / 2 - 0.1), (length - 0.36, 0.55, 0.2))
    for side in (-1, 1):
        mesh.box(fabric, (side * (length / 2 - 0.09), 0.335, 0.0), (0.18, 0.57, depth))
        mesh.box(cushion, (side * 0.41, 0.375, -0.06), (0.81, 0.15, 0.68))
        mesh.box(cushion, (side * 0.41, 0.62, depth / 2 - 0.27), (0.8, 0.42, 0.16))
    for side, pillow in zip((-1, 1), pillows):
        mesh.ellipsoid(pillow, (side * 0.64, 0.6, -0.02), (0.19, 0.17, 0.07),
                       rotate=rotation(pitch=-0.3, yaw=side * 0.2))

    mesh.transform = None


def add_graphics_card(mesh, materials, position, yaw):
    """A two-fan graphics card standing on its edge on a small stand, fans to -Z, a lit strip
    along its top. Life size, standing on its origin."""
    shroud, frame, blade, light, gold = materials
    place(mesh, position, yaw)

    length, tall, thick, lift = 0.304, 0.137, 0.045, 0.006
    middle = lift + tall / 2
    front = -thick / 2
    for x in (-0.1, 0.1):
        mesh.box(shroud, (x, lift / 2, 0.0), (0.03, lift, 0.08))
    mesh.box(shroud, (0.0, middle, 0.0), (length, tall, thick))
    mesh.box(blade, (0.0, middle, thick / 2 + 0.0015), (length - 0.01, tall - 0.01, 0.003))
    mesh.box(frame, (-length / 2 - 0.0015, middle, 0.0), (0.003, tall + 0.01, 0.022))
    mesh.box(gold, (-0.07, lift + 0.002, 0.0), (0.085, 0.008, 0.0016))

    # A brushed frame round the face, the lit strip just under its top edge.
    for y, height in [(lift + tall - 0.006, 0.012), (lift + 0.006, 0.012)]:
        mesh.box(frame, (0.0, y, front - 0.002), (length, height, 0.004))
    for x in (-1, 1):
        mesh.box(frame, (x * (length / 2 - 0.006), middle, front - 0.002), (0.012, tall, 0.004))
    mesh.box(light, (0.02, lift + tall - 0.019, front - 0.003), (0.17, 0.005, 0.003))

    for cx in (-0.072, 0.072):
        z = front - 0.001

        def disc(material, radius, depth):
            def point(i, j):
                angle = 2 * math.pi * j / 32
                r = radius * i
                return (cx + r * math.cos(angle), middle + r * math.sin(angle), depth), (j / 32, i)

            mesh.grid(material, 1, 32, point)

        disc(frame, 0.052, z)
        disc(shroud, 0.049, z - 0.0005)
        for k in range(9):
            a = 2 * math.pi * k / 9
            corners = [(0.014, a), (0.046, a + 0.35), (0.046, a + 0.62), (0.014, a + 0.3)]
            mesh.quad(blade, [(cx + r * math.cos(t), middle + r * math.sin(t), z - 0.001) for r, t in corners])
        disc(frame, 0.015, z - 0.002)

    mesh.transform = None


def add_ram_stick(mesh, materials, position, yaw):
    """A desktop memory stick in a black heat spreader with a lit bar along its top, standing on a
    small stand, its face to -Z."""
    spreader, light, gold, stand = materials
    place(mesh, position, yaw)
    mesh.box(stand, (0.0, 0.004, 0.0), (0.06, 0.008, 0.04))
    mesh.box(gold, (0.0, 0.012, 0.0), (0.12, 0.008, 0.0012))
    mesh.box(spreader, (0.0, 0.034, 0.0), (0.135, 0.036, 0.007))
    mesh.box(light, (0.0, 0.055, 0.0), (0.13, 0.006, 0.006))
    mesh.transform = None


def add_pc_tower(mesh, materials, position, yaw):
    """A plain black desktop tower, its side panel to -Z and its front to -X, standing on its
    origin."""
    case, bezel, glass, led = materials
    place(mesh, position, yaw)
    mesh.block((0.0, 0.25, 0.0), (0.45, 0.48, 0.21))
    for x in (-0.18, 0.18):
        for z in (-0.07, 0.07):
            mesh.box(bezel, (x, 0.008, z), (0.04, 0.016, 0.03))
    mesh.box(case, (0.0, 0.246, 0.0), (0.45, 0.46, 0.21))
    mesh.box(bezel, (-0.228, 0.246, 0.0), (0.008, 0.44, 0.2))
    mesh.box(led, (-0.233, 0.43, 0.0), (0.004, 0.012, 0.012))
    mesh.box(glass, (0.02, 0.25, -0.1055), (0.38, 0.38, 0.002))
    mesh.transform = None


def make_tactical_map_texture():
    """A military situation map: satellite imagery of farmland, forest, a river, roads and a town,
    a grid over it, and units in NATO symbology, friendly ones as light blue rectangles and hostile
    ones as red diamonds, with an arrow for the friendly axis of advance and a dashed phase line."""
    width, height = 1280, 720
    random.seed(41)

    def noise(scale, seed):
        random.seed(seed)
        small = Image.new("L", (max(2, width // scale), max(2, height // scale)))
        small.putdata([random.randint(0, 255) for _ in range(small.width * small.height)])
        return small.resize((width, height), Image.BICUBIC)

    # Terrain: greens and browns from layered noise, as a satellite sees farmland.
    ground = Image.merge("RGB", [noise(160, 1), noise(80, 2), noise(40, 3)])
    base = Image.new("RGB", (width, height), (96, 112, 64))
    image = Image.blend(base, ground.convert("L").convert("RGB"), 0.25)
    image = ImageChops.multiply(image, Image.new("RGB", (width, height), (230, 235, 205)))
    draw = ImageDraw.Draw(image)

    # Fields: a patchwork of tilted plots in crop colours.
    fields = [(150, 150, 84), (120, 140, 70), (170, 160, 110), (104, 124, 60), (140, 120, 80)]
    random.seed(7)
    for _ in range(140):
        cx, cy = random.uniform(0, width), random.uniform(0, height)
        w, h = random.uniform(40, 110), random.uniform(30, 80)
        a = random.uniform(-0.4, 0.4)
        corners = [(-w, -h), (w, -h), (w, h), (-w, h)]
        draw.polygon([(cx + x * math.cos(a) - y * math.sin(a), cy + x * math.sin(a) + y * math.cos(a))
                      for x, y in corners], fill=random.choice(fields), outline=(90, 100, 60))

    # Forests: dark, clumped.
    forest = Image.new("L", (width, height), 0)
    forest_draw = ImageDraw.Draw(forest)
    for cx, cy, r in [(250, 180, 120), (980, 520, 150), (1100, 130, 90), (420, 600, 100), (700, 90, 70)]:
        for _ in range(40):
            x = cx + random.gauss(0, r * 0.5)
            y = cy + random.gauss(0, r * 0.4)
            rr = random.uniform(15, 40)
            forest_draw.ellipse([x - rr, y - rr, x + rr, y + rr], fill=255)
    forest = forest.filter(ImageFilter.GaussianBlur(6)).point(lambda v: 255 if v > 110 else 0)
    texture = Image.merge("RGB", [noise(6, 4).point(lambda v: 30 + v // 8),
                                  noise(6, 5).point(lambda v: 55 + v // 8),
                                  noise(6, 6).point(lambda v: 30 + v // 10)])
    image.paste(texture, (0, 0), forest.filter(ImageFilter.GaussianBlur(1.5)))
    draw = ImageDraw.Draw(image)

    # A river winding across, and roads.
    river = [(x, 360 + 120 * math.sin(x / 210) + 40 * math.sin(x / 70)) for x in range(-20, width + 30, 10)]
    draw.line(river, fill=(70, 96, 80), width=22, joint="curve")
    draw.line(river, fill=(46, 74, 96), width=14, joint="curve")
    for road in ([(0, 620), (300, 520), (640, 470), (900, 300), (1280, 240)],
                 [(560, 0), (600, 220), (640, 470), (700, 720)]):
        draw.line(road, fill=(206, 196, 170), width=5, joint="curve")

    # A town where the roads cross the river.
    for _ in range(70):
        x, y = random.gauss(640, 60), random.gauss(460, 45)
        w, h = random.uniform(6, 16), random.uniform(6, 14)
        draw.rectangle([x - w, y - h, x + w, y + h], fill=random.choice([(150, 140, 130), (170, 110, 90), (120, 116, 112)]))

    image = image.filter(ImageFilter.GaussianBlur(0.6))
    draw = ImageDraw.Draw(image, "RGBA")

    # Grid.
    for x in range(0, width, 128):
        draw.line([x, 0, x, height], fill=(255, 255, 255, 70), width=1)
    for y in range(0, height, 128):
        draw.line([0, y, width, y], fill=(255, 255, 255, 70), width=1)

    friend, hostile, ink = (128, 224, 255), (255, 80, 80), (0, 0, 0)

    def echelon(x, top, marks):
        for i in range(marks):
            ex = x + (i - (marks - 1) / 2) * 7
            draw.line([ex, top - 12, ex, top - 3], fill=ink, width=3)

    def icon(kind, left, top, right, bottom):
        if kind in ("infantry", "mechanised"):
            draw.line([left, top, right, bottom], fill=ink, width=3)
            draw.line([left, bottom, right, top], fill=ink, width=3)
        if kind in ("armour", "mechanised"):
            mx, my = (left + right) / 2, (top + bottom) / 2
            draw.ellipse([mx - (right - left) * 0.32, my - (bottom - top) * 0.28,
                          mx + (right - left) * 0.32, my + (bottom - top) * 0.28], outline=ink, width=3)
        if kind == "artillery":
            mx, my = (left + right) / 2, (top + bottom) / 2
            draw.ellipse([mx - 6, my - 6, mx + 6, my + 6], fill=ink)
        if kind == "recon":
            draw.line([left, bottom, right, top], fill=ink, width=3)

    def friendly(x, y, kind, marks):
        w, h = 30, 20
        draw.rectangle([x - w, y - h, x + w, y + h], fill=friend, outline=ink, width=3)
        icon(kind, x - w, y - h, x + w, y + h)
        echelon(x, y - h, marks)

    def enemy(x, y, kind, marks):
        r = 30
        diamond = [(x, y - r), (x + r, y), (x, y + r), (x - r, y)]
        draw.polygon(diamond, fill=hostile, outline=ink)
        draw.line(diamond + [diamond[0]], fill=ink, width=3)
        icon(kind, x - r * 0.5, y - r * 0.5, x + r * 0.5, y + r * 0.5)
        echelon(x, y - r, marks)

    # Friendly forces south-west, advancing north-east; hostile forces north-east of the river.
    draw.line([(330, 610), (520, 520), (720, 400), (860, 300)], fill=(128, 224, 255, 160), width=10)
    draw.polygon([(880, 286), (842, 288), (862, 320)], fill=(128, 224, 255, 200))
    for x, y, kind, marks in [(250, 560, "armour", 2), (390, 640, "mechanised", 2),
                              (160, 470, "artillery", 2), (480, 560, "recon", 1),
                              (330, 470, "infantry", 1)]:
        friendly(x, y, kind, marks)
    for x, y, kind, marks in [(930, 200, "infantry", 2), (1060, 270, "armour", 2),
                              (820, 150, "mechanised", 1), (1150, 380, "artillery", 2)]:
        enemy(x, y, kind, marks)
    phase = [(x, 330 - 0.35 * x + 30 * math.sin(x / 90)) for x in range(400, 1280, 8)]
    for i in range(0, len(phase) - 1, 4):
        draw.line(phase[i:i + 3], fill=ink, width=3)

    # Screen edge: a thin dark frame and a status strip along the top.
    draw.rectangle([0, 0, width, 26], fill=(10, 16, 28, 220))
    for i, colour in enumerate([friend, hostile, (255, 220, 120)]):
        draw.rectangle([14 + i * 28, 8, 30 + i * 28, 18], fill=colour)
    draw.rectangle([0, 0, width - 1, height - 1], outline=(10, 16, 28), width=3)
    return image


def make_window_view_texture(seed):
    """The view out of a window over a city's office district on a clear day: a sky with a few
    clouds, hazy towers in the distance, office blocks of glass and stone nearer, a planted roof
    terrace, and the crowns of street trees along the bottom."""
    width, height = 640, 520
    random.seed(seed)
    image = Image.new("RGB", (width, height))
    draw = ImageDraw.Draw(image)
    for y in range(height):
        draw.line([0, y, width, y], fill=blend((92, 146, 220), (214, 228, 242), min(1.0, y / (height * 0.7)) ** 1.4))

    clouds = Image.new("L", (width, height), 0)
    cloud_draw = ImageDraw.Draw(clouds)
    for _ in range(3):
        cx, cy = random.uniform(0, width), random.uniform(height * 0.05, height * 0.25)
        for _ in range(8):
            rx, ry = random.uniform(30, 60), random.uniform(12, 22)
            x, y = cx + random.uniform(-60, 60), cy + random.uniform(-10, 10)
            cloud_draw.ellipse([x - rx, y - ry, x + rx, y + ry], fill=220)
    image.paste((250, 252, 255), (0, 0), clouds.filter(ImageFilter.GaussianBlur(9)))

    def facade(x0, x1, top, body, pane, cell, haze):
        """A building from `top` to the bottom of the view, its windows in a grid of `cell`
        pixels, everything faded toward the sky by `haze`."""
        sky = (200, 216, 234)
        draw.rectangle([x0, top, x1, height], fill=blend(body, sky, haze))
        cw, ch = cell
        for row, y in enumerate(range(int(top) + 6, height, ch)):
            for x in range(int(x0) + 4, int(x1) - cw // 2, cw):
                shade = blend(pane, sky, haze + 0.25 * random.random() * (1 - haze))
                draw.rectangle([x, y, x + cw - 4, y + ch - 5], fill=shade)
        draw.line([x0, top, x1, top], fill=blend((60, 64, 72), sky, haze), width=2)

    glass = [((70, 98, 124), (120, 164, 200)), ((58, 84, 100), (104, 150, 170)),
             ((84, 104, 128), (140, 176, 212))]
    stone = [((196, 186, 168), (70, 80, 92)), ((176, 170, 162), (64, 72, 84)),
             ((210, 204, 194), (84, 92, 104))]

    # Distant towers, hazy, then the nearer blocks, a mix of glass and stone.
    x = -20
    while x < width:
        w = random.uniform(40, 90)
        body, pane = random.choice(glass)
        facade(x, x + w, random.uniform(height * 0.12, height * 0.35), body, pane, (9, 12), 0.6)
        x += w + random.uniform(4, 20)
    x = -30
    while x < width:
        w = random.uniform(80, 160)
        body, pane = random.choice(glass + stone)
        cell = (14, 18) if (body, pane) in glass else (18, 22)
        facade(x, x + w, random.uniform(height * 0.3, height * 0.55), body, pane, cell, 0.15)
        x += w + random.uniform(10, 30)

    # A planted roof terrace on one low block.
    tx = random.uniform(width * 0.2, width * 0.6)
    top = height * 0.62
    facade(tx, tx + 170, top, (188, 180, 166), (72, 80, 90), (18, 22), 0.05)
    for _ in range(14):
        x, r = random.uniform(tx + 10, tx + 160), random.uniform(8, 16)
        draw.ellipse([x - r, top - r * 1.2, x + r, top + 4], fill=random.choice([(70, 118, 60), (88, 134, 66)]))

    # Street trees along the bottom.
    for _ in range(60):
        x = random.uniform(-20, width + 20)
        r = random.uniform(26, 48)
        y = height - random.uniform(10, 70)
        colour = random.choice([(58, 104, 52), (72, 122, 58), (90, 140, 68), (50, 92, 48)])
        draw.ellipse([x - r, y - r, x + r, y + r], fill=colour)
    return image.filter(ImageFilter.GaussianBlur(0.7))


def add_window(mesh, materials, centre, along, normal, width, height):
    """A window on a wall: the view through it, in a white frame with a cross mullion and a sill,
    `along` the wall to the right as seen from inside, facing the room along `normal`. The daylight
    through it is recorded as a light source for the bake."""
    frame, view = materials
    up = (0.0, 1.0, 0.0)

    def at(a, b, out=0.0):
        return tuple(c + a * x + b * y + out * n for c, x, y, n in zip(centre, along, up, normal))

    def box(a, b, out, size_along, size_up, size_out):
        size = tuple(abs(x) * size_along + y * size_up + abs(n) * size_out for x, y, n in zip(along, up, normal))
        mesh.box(frame, at(a, b, out), size)

    w, h = width / 2, height / 2
    mesh.quad(view, [at(-w, -h, 0.004), at(w, -h, 0.004), at(w, h, 0.004), at(-w, h, 0.004)],
              uvs=[(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)])
    bar, depth = 0.06, 0.05
    for b in (-h, h):
        box(0.0, b, depth / 2, width + bar, bar, depth)
    for a in (-w, w):
        box(a, 0.0, depth / 2, bar, height, depth)
    box(0.0, 0.0, depth / 2, 0.035, height, depth * 0.8)
    box(0.0, 0.0, depth / 2, width, 0.035, depth * 0.8)
    box(0.0, -h - bar / 2 - 0.015, 0.07, width + 0.2, 0.03, 0.14)

    mesh.light(f"window {len(mesh.lights)}", at(0.0, 0.0, 0.06), normal, along, (width, height),
               WINDOW_POWER, [(at(a * w * 0.8, b * h * 0.8, 0.01), normal, WINDOW_LIGHT)
                              for a in (-1, 0, 1) for b in (-1, 0, 1)])


# Daylight through each sample of a window, against 1 for each sample along a ceiling panel.
WINDOW_LIGHT = 0.8
# The same balance for Blender, in watts.
PANEL_POWER = 150.0
WINDOW_POWER = 400.0


def add_book_row(mesh, books, left, right, floor, clearance, z, seed):
    """Books standing upright side by side from `left` to `right` along a shelf, centred on `z`."""
    random.seed(seed)
    x = left
    while x < right:
        thickness = random.uniform(0.025, 0.06)
        tall = random.uniform(0.55, 0.92) * clearance
        mesh.box(random.choice(books), (x + thickness / 2, floor + tall / 2, z),
                 (thickness, tall, random.uniform(0.17, 0.24)))
        x += thickness + 0.003


def build_studio(mesh):
    half, height = STUDIO_HALF, STUDIO_HEIGHT

    floor = Material("floor", texture=make_studio_floor_texture())
    # The line fan on every wall, placed and turned differently on each so they do not read as
    # copies. The right wall's stays clear of the colour chart in its middle.
    front_wall = Material("front wall", texture=make_studio_wall_texture(0.7))
    back_wall = Material("back wall", texture=make_studio_wall_texture(0.7, mirrored=True))
    left_wall = Material("left wall", texture=make_studio_wall_texture(0.85))
    right_wall = Material("right wall", texture=make_studio_wall_texture(0.85, mirrored=True))
    ceiling = Material("ceiling", texture=make_studio_ceiling_texture())
    light = Material("ceiling light", texture=make_light_panel_texture())
    lime = Material("lime", colour=LIME)
    shading = make_shading_texture()
    # Round things take shading without edge darkening, which would show where their u wraps.
    round_shading = make_shading_texture(edge_ao=0.0)
    # White furniture takes almost none of its own: the bake shades it, and the two together
    # would turn white grey.
    white_shading = make_shading_texture(top_light=0.05, bottom_dark=0.08, edge_ao=0.08, base=250)

    # Floor, a metre grid tiled every four metres, cut into quarter-metre cells for the bake, and
    # the ceiling.
    tiles = 2 * half / 4
    mesh.lit_surface(floor, (-half, 0, half), (2 * half, 0, 0), (0, 0, -2 * half), (0.0, 1.0, 0.0),
                     (64, 64), lambda a, b: (a * tiles, b * tiles))
    mesh.quad(
        ceiling,
        [(-half, height, -half), (half, height, -half), (half, height, half), (-half, height, half)],
    )

    # Walls, facing inward, v up: from each one's bottom-left corner as seen from inside, along it
    # and up, cut into cells for the bake.
    walls = [
        (front_wall, (-half, 0, -half), (2 * half, 0, 0), (0.0, 0.0, 1.0)),
        (back_wall, (half, 0, half), (-2 * half, 0, 0), (0.0, 0.0, -1.0)),
        (left_wall, (-half, 0, half), (0, 0, -2 * half), (1.0, 0.0, 0.0)),
        (right_wall, (half, 0, -half), (0, 0, 2 * half), (-1.0, 0.0, 0.0)),
    ]
    for material, corner, along, normal in walls:
        mesh.lit_surface(material, corner, along, (0, height, 0), normal, (32, 12),
                         lambda a, b: (a, 1.0 - b))

    # The lime line along the foot of every wall, just proud of it.
    strip, inset = 0.05, 0.004
    edge = half - inset
    mesh.quad(lime, [(-edge, 0, -edge), (edge, 0, -edge), (edge, strip, -edge), (-edge, strip, -edge)])
    mesh.quad(lime, [(edge, 0, edge), (-edge, 0, edge), (-edge, strip, edge), (edge, strip, edge)])
    mesh.quad(lime, [(-edge, 0, edge), (-edge, 0, -edge), (-edge, strip, -edge), (-edge, strip, edge)])
    mesh.quad(lime, [(edge, 0, -edge), (edge, 0, edge), (edge, strip, edge), (edge, strip, -edge)])

    # Ceiling lights in a grid, just below the ceiling.
    length, width = LIGHT_SIZE[0] / 2, LIGHT_SIZE[1] / 2
    y = height - 0.005
    for x in LIGHT_POSITIONS:
        for z in LIGHT_POSITIONS:
            mesh.quad(
                light,
                [(x - length, y, z - width), (x + length, y, z - width),
                 (x + length, y, z + width), (x - length, y, z + width)],
            )
            down = (0.0, -1.0, 0.0)
            mesh.light(f"ceiling light {len(mesh.lights)}", (x, height - 0.03, z), down,
                       (1.0, 0.0, 0.0), LIGHT_SIZE, PANEL_POWER,
                       [((x + offset, height - 0.01, z), down, 1.0) for offset in (-0.6, 0.0, 0.6)])

    # A lime circle round the start position, marking the play area.
    mesh.ring(lime, (0.0, 0.003, 0.0), 1.47, 1.5)

    # Windows: above the sofa on the right wall, and on the left wall between the side table and the
    # tree. Each a different view, so they do not read as copies.
    window_frame = Material("window frame", colour=(240, 242, 246), texture=white_shading)
    for centre, along, normal, seed in [
        ((half - 0.002, 1.75, -3.6), (0.0, 0.0, 1.0), (-1.0, 0.0, 0.0), 61),
        ((-half + 0.002, 1.75, 3.2), (0.0, 0.0, -1.0), (1.0, 0.0, 0.0), 67),
    ]:
        view = Material(f"window view {seed}", texture=make_window_view_texture(seed))
        add_window(mesh, (window_frame, view), centre, along, normal, 2.0, 1.5)

    # A colour chart on the right wall: patches of known colour, for checking the scene renderer's
    # colours and how Mixed blends them.
    chart = Material("colour chart", texture=make_colour_chart_texture())
    x = half - 0.006
    mesh.quad(chart, [(x, 1.1, -0.8), (x, 1.1, 0.8), (x, 2.1, 0.8), (x, 2.1, -0.8)],
              uvs=[(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)])

    # Bookshelves on the left and back walls, and a side table with a stack of books.
    shelf = Material("shelf", colour=(238, 240, 246), texture=white_shading)
    book_colours = [LIME, BLUE, WHITE, rgb("#E7704F"), rgb("#2A9DAF"), rgb("#E8C872"),
                    rgb("#8A2E52"), rgb("#7C8AA5"), rgb("#F2EEE4"), rgb("#1C2C6E")]
    book_texture = make_book_texture()
    books = [Material(f"book {index}", colour=colour, texture=book_texture)
             for index, colour in enumerate(book_colours)]
    # Turned so their open fronts face into the room and their backs stand against the walls.
    against_wall = half - 0.36
    add_bookshelf(mesh, (shelf, books), (-against_wall, 0.0, -1.6), -math.pi / 2, seed=11)
    add_bookshelf(mesh, (shelf, books), (2.8, 0.0, against_wall), 0.0, seed=31)

    table = Material("table", colour=(238, 240, 246), texture=white_shading)
    mesh.box(table, (-7.2, 0.25, 0.9), (0.6, 0.5, 0.6))
    mesh.block((-7.2, 0.25, 0.9), (0.6, 0.5, 0.6))
    random.seed(5)
    lift = 0.5
    for _ in range(4):
        thick = random.uniform(0.03, 0.05)
        mesh.box(random.choice(books), (-7.2 + random.uniform(-0.03, 0.03), lift + thick / 2, 0.9),
                 (random.uniform(0.2, 0.28), thick, random.uniform(0.15, 0.2)))
        lift += thick

    # Plants and small trees around the walls.
    plant = (
        Material("pot", colour=(240, 242, 248), texture=white_shading),
        Material("soil", colour=(60, 45, 35), texture=shading),
        Material("leaves", colour=(70, 150, 80), texture=make_shading_texture(0.4, 0.4, 0.0)),
        Material("stem", colour=(80, 110, 60), texture=shading),
    )
    for position, seed, size in [
        ((-7.3, 0.0, -3.4), 3, 1.0),
        ((-7.2, 0.0, 5.6), 4, 1.7),
        ((-0.1, 0.0, 7.3), 6, 1.2),
        ((-5.6, 0.0, 7.3), 8, 1.0),
        ((5.4, 0.0, 7.2), 9, 1.8),
        ((7.2, 0.0, -7.2), 12, 1.9),
        ((-7.2, 0.0, -7.2), 14, 1.6),
        ((7.2, 0.0, 5.6), 15, 1.7),
    ]:
        add_plant(mesh, plant, position, seed, size)

    # A plush round rug in the middle of the room, inside the lime circle.
    rug = Material("rug", texture=make_rug_texture())
    mesh.disc(rug, (0.0, 0.002, 0.0), 1.4)

    # The cat, lying on top of the bookshelf and looking into the room.
    fur = make_fur_texture()
    cat = (
        Material("cat fur", colour=(156, 158, 166), texture=fur),
        Material("cat paw", colour=(112, 114, 122), texture=fur),
        Material("cat inner ear", colour=(150, 128, 134), texture=shading),
        Material("cat eye", colour=(232, 168, 28), texture=round_shading),
        Material("cat pupil", colour=(20, 20, 24)),
        Material("cat nose", colour=(62, 60, 66), texture=shading),
    )
    shelf_top = (-half + 0.19, 2.0, -1.6)
    cat_at = (shelf_top[0] - 0.04, shelf_top[1], shelf_top[2] + 0.5)
    add_cat(mesh, cat, cat_at, -math.pi / 2)

    # Model planes: a small one on the bookshelf, the cat's chin resting on it, and one on a stand
    # on the side table.
    plane_body = Material("plane body", colour=(244, 246, 250), texture=shading)
    plane_engine = Material("plane engine", colour=(150, 158, 175), texture=shading)
    plane_stand = Material("plane stand", colour=(60, 66, 80), texture=shading)
    tails = [Material(f"plane tail {index}", colour=colour, texture=shading)
             for index, colour in enumerate([BLUE, LIME, WALL_NAVY])]

    # Under the cat's chin, across the shelf with its nose toward the room: the cat's head is 0.25
    # along its body, which runs along the shelf.
    add_plane(mesh, (Material("plane blue", colour=BLUE, texture=shading), tails[1], plane_engine),
              (cat_at[0] + 0.08, cat_at[1] + 0.035, cat_at[2] + 0.25), yaw=-math.pi / 2, scale=0.8)

    mesh.tube(plane_stand, [(-7.2, 0.5, 1.1), (-7.2, 0.72, 1.1)], [0.006, 0.006], segments=8)
    mesh.disc(plane_stand, (-7.2, 0.501, 1.1), 0.05)
    add_plane(mesh, (plane_body, tails[0], plane_engine), (-7.2, 0.76, 1.1),
              yaw=-math.pi / 2 + 0.6, pitch=-0.15)

    helicopter_glass = Material("helicopter glass", colour=(40, 60, 95), texture=shading)

    def helicopter(tail):
        return (plane_body, tail, helicopter_glass, plane_stand)

    # Planes and helicopters hanging from the ceiling, each on two wires.
    for kind, position, yaw, scale, tail in [
        ("helicopter", (4.5, 2.9, -3.6), 1.2, 2.0, tails[0]),
        ("plane", (-4.1, 2.95, -4.6), -0.6, 2.2, tails[1]),
        ("helicopter", (-3.6, 2.9, 3.8), 2.4, 1.8, tails[1]),
        ("plane", (4.1, 2.95, 4.2), -2.2, 2.4, tails[2]),
    ]:
        if kind == "plane":
            pitch, anchors = 0.08, [(0.0, 0.03, -0.13), (0.0, 0.03, 0.12)]
            add_plane(mesh, (plane_body, tail, plane_engine), position, yaw=yaw, pitch=pitch,
                      scale=scale)
        else:
            pitch, anchors = 0.0, [(0.0, 0.1, -0.05), (0.0, 0.045, 0.28)]
            add_helicopter(mesh, helicopter(tail), position, yaw=yaw, scale=scale)
        orient = rotation(pitch=pitch, yaw=yaw)
        for anchor in anchors:
            top = tuple(p + v for p, v in zip(position, orient(tuple(scale * a for a in anchor))))
            mesh.tube(plane_stand, [top, (top[0], height, top[2])], [0.003, 0.003], segments=6)

    # A foosball table by the back wall, in the room's colours, and a sofa by the right wall.
    foosball = (
        Material("foosball cabinet", colour=WALL_NAVY, texture=shading),
        Material("foosball pitch", texture=make_foosball_field_texture()),
        Material("foosball rod", colour=(200, 204, 212), texture=shading),
        Material("foosball handle", colour=(34, 34, 38), texture=shading),
        (Material("foosball lime", colour=LIME, texture=shading),
         Material("foosball white", colour=(246, 246, 250), texture=shading)),
        Material("foosball ball", colour=WHITE, texture=shading),
        Material("foosball goal", colour=(12, 14, 22)),
    )
    add_foosball(mesh, foosball, (-3.0, 0.0, 6.9), math.pi / 2)

    add_sofa(mesh, (
        Material("sofa", colour=(132, 140, 162), texture=shading),
        Material("sofa cushion", colour=(150, 158, 180), texture=shading),
        (Material("pillow lime", colour=LIME, texture=round_shading),
         Material("pillow blue", colour=BLUE, texture=round_shading)),
        Material("sofa feet", colour=(40, 40, 46), texture=shading),
    ), (half - 0.47, 0.0, -3.6), math.pi / 2)

    syrup = (
        Material("syrup", colour=(150, 72, 18), texture=round_shading),
        Material("syrup label", texture=make_syrup_label_texture()),
        Material("syrup cap", colour=(200, 34, 40), texture=shading),
    )
    tank = (
        Material("tank hull", colour=(98, 106, 66), texture=shading),
        Material("tank track", colour=(46, 48, 46), texture=shading),
    )
    headset = (
        Material("headset shell", colour=(34, 35, 40), texture=shading),
        Material("headset visor", colour=(64, 78, 124), texture=make_shading_texture(0.9, 0.5, 0.0)),
        Material("headset band", colour=(58, 60, 66), texture=shading),
    )

    # A display cabinet on the front wall, along from the desk, navy-backed so what is on it stands
    # out: headsets, a model tank and helicopter, a graphics card, maple syrup and a few books.
    # Everything on it is placed in the cabinet's own frame, x along its shelves from the left as
    # seen from the room and z from its open front, and turned with it.
    display, facing = (-4.0, 0.0, -against_wall), math.pi
    place(mesh, display, facing)
    floors = add_shelf_frame(mesh, shelf, Material("display back", colour=WALL_NAVY, texture=shading), 5)
    depth = SHELF_SIZE[2]
    clearance = floors[1] - floors[0] - SHELF_SIZE[3]
    add_book_row(mesh, books, -1.04, -0.3, floors[0], clearance, depth / 2, seed=47)
    add_book_row(mesh, books, 0.1, 1.04, floors[3], clearance, depth / 2, seed=53)
    mesh.transform = None
    turn = rotation(yaw=facing)

    def on_shelf(level, x, z=depth / 2, lift=0.0):
        return tuple(d + o for d, o in zip(display, turn((x, floors[level] + lift, z))))

    for x in (0.2, 0.42, 0.64):
        add_syrup(mesh, syrup, on_shelf(0, x), facing)
    add_tank(mesh, tank, on_shelf(1, -0.35), facing + math.pi / 2 + 0.25, scale=1.5)
    add_helicopter(mesh, helicopter(tails[0]), on_shelf(1, 0.6, lift=0.057),
                   facing - math.pi / 2 + 0.5, scale=0.8)
    quest = (
        Material("quest shell", colour=(236, 237, 240), texture=round_shading),
        Material("quest front", colour=(246, 246, 248), texture=shading),
        Material("quest sensor", colour=(30, 32, 36), texture=shading),
        Material("quest strap", colour=(218, 220, 224), texture=round_shading),
    )
    add_quest(mesh, quest, on_shelf(2, -0.68, 0.13), facing + 0.6)
    add_headset(mesh, headset, on_shelf(2, 0.0, 0.15), facing + 0.35)
    add_quest(mesh, quest, on_shelf(2, 0.68, 0.13), facing - 0.55)
    add_headset(mesh, headset, on_shelf(3, -0.55, 0.15), facing + 0.8)
    add_plane(mesh, (plane_body, tails[0], plane_engine), on_shelf(4, -0.45, lift=0.035),
              yaw=facing - math.pi / 2 + 0.4, scale=0.9)
    add_syrup(mesh, syrup, on_shelf(4, 0.2), facing)
    add_graphics_card(mesh, (
        Material("card shroud", colour=(36, 38, 42), texture=shading),
        Material("card frame", colour=(192, 196, 204), texture=shading),
        Material("card fan", colour=(70, 72, 78), texture=round_shading),
        Material("card light", colour=LIME),
        Material("card contacts", colour=(212, 170, 60), texture=shading),
    ), on_shelf(4, 0.6, 0.15), facing - 0.3)

    # A kitchen counter on the right wall, past the colour chart from the sofa: a cereal dispenser
    # with a bowl under one spout, a bowl of bananas and a bottle of syrup, all facing the room.
    counter_x, counter_z = half - 0.26, 3.4
    counter_top = 0.9

    def on_counter(along, out, lift=0.0):
        # `along` the counter to the right as seen from the room, `out` from the wall.
        return (counter_x - out, counter_top + lift, counter_z + along)

    mesh.box(table, (counter_x, (counter_top - 0.03) / 2, counter_z), (0.5, counter_top - 0.03, 1.4))
    mesh.block((counter_x, counter_top / 2, counter_z), (0.52, counter_top, 1.44))
    mesh.box(Material("counter top", colour=(206, 210, 220), texture=shading),
             (counter_x - 0.005, counter_top - 0.015, counter_z), (0.52, 0.03, 1.44))
    plastic = Material("dispenser plastic", colour=(30, 30, 34), texture=shading)
    add_cereal_dispenser(mesh, (
        plastic,
        Material("dispenser chrome", colour=(214, 218, 226), texture=round_shading),
        Material("dispenser cup", colour=(212, 220, 230), texture=round_shading),
        (Material("cornflakes", texture=make_cereal_texture("flakes")),
         Material("cereal balls", texture=make_cereal_texture("balls"))),
    ), on_counter(-0.4, 0.0), math.pi / 2)
    cereal_bowl = Material("cereal bowl", colour=(246, 246, 250), texture=round_shading)
    mesh.lathe(cereal_bowl, on_counter(-0.505, 0.07, 0.03),
               [(0.0, 0.0), (0.035, 0.0), (0.06, 0.025), (0.07, 0.05)], segments=32)
    add_fruit_bowl(mesh, (
        Material("bowl", colour=BLUE, texture=round_shading),
        Material("banana", colour=(240, 204, 62), texture=round_shading),
        Material("banana tip", colour=(80, 62, 36), texture=shading),
    ), on_counter(0.2, 0.02))
    add_syrup(mesh, syrup, on_counter(0.55, -0.08), math.pi / 2)

    card = (
        Material("card shroud", colour=(36, 38, 42), texture=shading),
        Material("card frame", colour=(192, 196, 204), texture=shading),
        Material("card fan", colour=(70, 72, 78), texture=round_shading),
        Material("card light", colour=LIME),
        Material("card contacts", colour=(212, 170, 60), texture=shading),
    )

    # A second graphics card on top of the back wall's bookshelf, and a memory stick in the cabinet.
    add_graphics_card(mesh, card, (2.4, SHELF_SIZE[1], half - 0.2), 0.2)
    add_ram_stick(mesh, (
        Material("memory spreader", colour=(30, 30, 34), texture=shading),
        Material("memory light", colour=blend(BLUE, WHITE, 0.35)),
        card[4],
        card[0],
    ), on_shelf(3, -0.12, 0.15), facing + 0.15)

    # A desk on the front wall under a screen showing a tactical map, between two connected PCs:
    # a plain black one, and one with a graphics card mounted on the outside of its side panel.
    desk_z = -half + 0.37
    desk = Material("desk", colour=(46, 50, 60), texture=shading)
    mesh.box(desk, (0.0, 0.74, desk_z), (1.6, 0.04, 0.7))
    mesh.block((0.0, 0.74, desk_z), (1.6, 0.04, 0.7))
    for x in (-0.76, 0.76):
        for z in (desk_z - 0.31, desk_z + 0.31):
            mesh.box(desk, (x, 0.36, z), (0.04, 0.72, 0.04))
    keys = Material("keyboard", colour=(28, 28, 32), texture=shading)
    mesh.box(keys, (-0.05, 0.77, desk_z + 0.16), (0.44, 0.02, 0.14))
    mesh.ellipsoid(keys, (0.3, 0.77, desk_z + 0.17), (0.03, 0.015, 0.05), rings=6, segments=12)

    screen_z = -half + 0.04
    bezel = Material("screen bezel", colour=(18, 18, 22), texture=shading)
    mesh.box(bezel, (0.0, 1.75, screen_z - 0.02), (2.46, 1.41, 0.04))
    mesh.quad(Material("tactical map", texture=make_tactical_map_texture()),
              [(-1.2, 1.075, screen_z + 0.001), (1.2, 1.075, screen_z + 0.001),
               (1.2, 2.425, screen_z + 0.001), (-1.2, 2.425, screen_z + 0.001)],
              uvs=[(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)])

    tower = (
        Material("pc case", colour=(22, 22, 26), texture=shading),
        Material("pc bezel", colour=(40, 40, 46), texture=shading),
        Material("pc panel", colour=(34, 40, 56), texture=shading),
        Material("pc light", colour=blend(BLUE, WHITE, 0.35)),
    )
    tower_z = -half + 0.27
    # Turned so the side panels face the room and the fronts face the desk between them.
    add_pc_tower(mesh, tower, (-1.25, 0.0, tower_z), math.pi)
    add_pc_tower(mesh, tower, (1.25, 0.0, tower_z), math.pi)
    # The card hangs on the second tower's side panel on a mount, its fans to the room.
    mount = rotation(yaw=math.pi)
    card_at = tuple(c + o for c, o in zip((1.25, 0.0, tower_z), mount((0.0, 0.12, -0.105 - 0.026))))
    add_graphics_card(mesh, card, card_at, math.pi)
    for x in (-0.12, 0.12):
        mesh.box(card[1], tuple(c + o for c, o in zip((1.25, 0.0, tower_z), mount((x, 0.19, -0.115)))),
                 (0.012, 0.024, 0.02))

    # Cables: between the two towers along the floor behind the desk, and from each up to the
    # screen.
    cable = Material("cable", colour=(16, 16, 18), texture=round_shading)
    back = -half + 0.12
    mesh.tube(cable, [(-1.25, 0.3, back), (-1.0, 0.03, back), (0.0, 0.006, back - 0.02),
                      (1.0, 0.03, back), (1.25, 0.3, back)], [0.006] * 5, segments=8)
    for side in (-1, 1):
        mesh.tube(cable, [(side * 1.25, 0.4, back), (side * 0.9, 0.5, back),
                          (side * 0.4, 0.9, -half + 0.02), (side * 0.2, 1.08, -half + 0.02)],
                  [0.006] * 4, segments=8)


# ---------------------------------------------------------------------------------------------
# Writing the glTF
# ---------------------------------------------------------------------------------------------


def primitive_vertices(mesh):
    """For each material in order, the vertices its primitive uses, in the order `write_gltf`
    writes them."""
    return [list(dict.fromkeys(indices)) for indices in mesh.groups.values()]


def write_gltf(mesh):
    """Packs the mesh, its materials and their textures into a self-contained glTF document."""
    materials = list(mesh.groups)

    textures = []
    for material in materials:
        if material.texture is not None and all(material.texture is not t for t in textures):
            textures.append(material.texture)

    # Each primitive gets its own block of the vertices it uses, with indices local to it: some
    # loaders (easy-gltf among them) copy a primitive's whole POSITION accessor, so one accessor
    # shared by every primitive would be copied once per material.
    vertex_bytes = b""
    index_bytes = b""
    colour_bytes = b""
    primitives_meta = []
    for material_index, (material, used) in enumerate(zip(materials, primitive_vertices(mesh))):
        local = {index: slot for slot, index in enumerate(used)}
        indices = [local[index] for index in mesh.groups[material]]
        floats = [f for index in used for f in mesh.vertices[index * 5 : index * 5 + 5]]

        # Vertex colours only where the bake lit something, as normalised bytes, RGBA so each
        # vertex's colour stays 4-byte aligned.
        colour_offset = None
        if any(mesh.colours[index] != (1.0, 1.0, 1.0) for index in used):
            colour_offset = len(colour_bytes)
            colour_bytes += bytes(
                channel
                for index in used
                for channel in [round(c * 255) for c in mesh.colours[index]] + [255]
            )

        positions = [floats[k : k + 3] for k in range(0, len(floats), 5)]
        primitives_meta.append(
            {
                "material": material_index,
                "vertex_offset": len(vertex_bytes),
                "vertex_count": len(used),
                "index_offset": len(index_bytes),
                "index_count": len(indices),
                "min": [min(p[axis] for p in positions) for axis in range(3)],
                "max": [max(p[axis] for p in positions) for axis in range(3)],
                "colour_offset": colour_offset,
            }
        )
        vertex_bytes += struct.pack("<%df" % len(floats), *floats)
        index_bytes += struct.pack("<%dI" % len(indices), *indices)

    # Append each PNG to the buffer, aligned to 4 bytes as the spec requires for buffer views.
    blob = bytearray(vertex_bytes + index_bytes + colour_bytes)
    image_views = []
    for texture in textures:
        while len(blob) % 4:
            blob.append(0)

        png = encode_png(texture)
        image_views.append((len(blob), len(png)))
        blob.extend(png)

    blob = bytes(blob)

    views = [
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
    if colour_bytes:
        views.append(
            {
                "buffer": 0,
                "byteOffset": len(vertex_bytes) + len(index_bytes),
                "byteLength": len(colour_bytes),
                "target": 34962,
            }
        )

    accessors = []
    primitives = []
    for meta in primitives_meta:
        accessors += [
            {
                "bufferView": 0,
                "byteOffset": meta["vertex_offset"],
                "componentType": 5126,
                "count": meta["vertex_count"],
                "type": "VEC3",
                "min": meta["min"],
                "max": meta["max"],
            },
            {
                "bufferView": 0,
                "byteOffset": meta["vertex_offset"] + 12,
                "componentType": 5126,
                "count": meta["vertex_count"],
                "type": "VEC2",
            },
            {
                "bufferView": 1,
                "byteOffset": meta["index_offset"],
                "componentType": 5125,  # unsigned int
                "count": meta["index_count"],
                "type": "SCALAR",
            },
        ]
        primitive = {
            "attributes": {"POSITION": len(accessors) - 3, "TEXCOORD_0": len(accessors) - 2},
            "indices": len(accessors) - 1,
            "material": meta["material"],
            "mode": 4,
        }
        if meta["colour_offset"] is not None:
            accessors.append(
                {
                    "bufferView": 2,  # colours, present whenever any primitive has them
                    "byteOffset": meta["colour_offset"],
                    "componentType": 5121,  # unsigned byte
                    "normalized": True,
                    "count": meta["vertex_count"],
                    "type": "VEC4",
                }
            )
            primitive["attributes"]["COLOR_0"] = len(accessors) - 1
        primitives.append(primitive)

    def material_json(material):
        pbr = {
            "baseColorFactor": [c / 255 for c in material.colour] + [1.0],
            "metallicFactor": 0.0,
            "roughnessFactor": 1.0,
        }
        if material.texture is not None:
            index = next(i for i, t in enumerate(textures) if t is material.texture)
            pbr["baseColorTexture"] = {"index": index}
        return {"name": material.name, "pbrMetallicRoughness": pbr}

    gltf = {
        "asset": {"version": "2.0", "generator": "ALVR client emulator test environment"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": "room"}],
        "meshes": [{"name": "room", "primitives": primitives}],
        "materials": [material_json(material) for material in materials],
        "textures": [{"source": index, "sampler": 0} for index in range(len(textures))],
        "samplers": [{"magFilter": 9729, "minFilter": 9729, "wrapS": 10497, "wrapT": 10497}],
        # Referenced by buffer view rather than data URI; see the module docstring.
        "images": [
            {"bufferView": len(views) + index, "mimeType": "image/png"}
            for index in range(len(textures))
        ],
        "buffers": [
            {
                "byteLength": len(blob),
                "uri": "data:application/octet-stream;base64,"
                + base64.b64encode(blob).decode(),
            }
        ],
        # Image views carry no target: they are not vertex or index data.
        "bufferViews": views
        + [
            {"buffer": 0, "byteOffset": offset, "byteLength": length}
            for offset, length in image_views
        ],
        "accessors": accessors,
    }

    return (
        gltf,
        sum(meta["vertex_count"] for meta in primitives_meta),
        sum(meta["index_count"] for meta in primitives_meta),
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("output", nargs="?", default="environment.gltf")
    parser.add_argument("--style", choices=["studio", "brown"], default="studio")
    parser.add_argument("--bake", choices=["auto", "blender", "python"], default="auto",
                        help="how to bake the lighting: auto uses Blender when it is installed")
    arguments = parser.parse_args()

    mesh = MeshBuilder()
    {"studio": build_studio, "brown": build_brown}[arguments.style](mesh)

    if mesh.lights:
        blender = find_blender() if arguments.bake != "python" else None
        if blender is not None:
            bake_with_blender(mesh, blender)
        elif arguments.bake == "blender":
            raise SystemExit("Blender not found; set BLENDER to its executable")
        else:
            print("Baking without Blender")
            samples = [sample for light in mesh.lights for sample in light["samples"]]
            bake_lighting(mesh, samples, STUDIO_HALF, STUDIO_HEIGHT)

    gltf, vertex_count, index_count = write_gltf(mesh)

    with open(arguments.output, "w", encoding="utf-8") as handle:
        json.dump(gltf, handle)

    print(
        f"Wrote {arguments.output}: {vertex_count} vertices, {index_count} indices, "
        f"{len(gltf['materials'])} materials, {len(gltf['images'])} textures"
    )


if __name__ == "__main__":
    main()
