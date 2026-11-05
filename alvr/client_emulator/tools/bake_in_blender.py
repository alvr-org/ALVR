"""Bakes the lighting of a room made by make_environment.py, in Blender.

make_environment.py runs this itself when it finds Blender; it is not meant to be run by hand:

    blender --background --factory-startup --python bake_in_blender.py -- ROOM LIGHTS OUTPUT

Imports ROOM, a .gltf, lights it with a rectangular area light for each source described in LIGHTS,
a JSON list, bakes with Cycles the light falling on every vertex, direct and bounced, and writes
OUTPUT: for each vertex, in the order the vertices appear in the file, its position (to check the
order against), its light, and 1 if it is covered, as seven little-endian floats. Light only, not
the colour of the surface it falls on, which the room's textures carry already. A vertex is covered
when something lies on it, as the rug lies on the floor: it cannot be seen, and its light is
meaningless.
"""

import json
import struct
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree


def to_blender(vector):
    """glTF is Y up, Blender Z up."""
    x, y, z = vector
    return Vector((x, -z, y))


def from_blender(vector):
    x, y, z = vector
    return (x, z, -y)


def add_light(source):
    light = bpy.data.lights.new(source["name"], "AREA")
    light.shape = "RECTANGLE"
    light.size, light.size_y = source["size"]
    light.energy = source["power"]

    # An area light shines along its own -Z, with its width along its X.
    z = -to_blender(source["facing"]).normalized()
    x = to_blender(source["along"]).normalized()
    y = z.cross(x)
    location = to_blender(source["position"])

    holder = bpy.data.objects.new(source["name"], light)
    holder.matrix_world = Matrix((
        (x.x, y.x, z.x, location.x),
        (x.y, y.y, z.y, location.y),
        (x.z, y.z, z.z, location.z),
        (0.0, 0.0, 0.0, 1.0),
    ))
    bpy.context.scene.collection.objects.link(holder)


def face_the_room(mesh):
    """Turns every face toward the side it can be seen from.

    The generator does not keep its triangles' winding consistent, the emulator drawing both sides
    of everything, but a bake lights only the side a face's normal points to.

    Each connected piece of the mesh is first made consistent and turned outward, which is all a
    curved shape such as a pot, a leaf or the cat needs. A flat piece has no outside, so the floor,
    a wall or the side of a box is then turned by a vote of its faces: each casts a ray from either
    side and votes for the side that sees further into the room, a ray that leaves the room seeing
    nothing. The floor ends up facing up, the walls in, and a box's sides out."""
    shape = bmesh.new()
    shape.from_mesh(mesh)
    bmesh.ops.recalc_face_normals(shape, faces=shape.faces[:])
    tree = BVHTree.FromBMesh(shape)

    def reach(origin, direction):
        hit = tree.ray_cast(origin + direction * 1e-4, direction)
        return hit[3] if hit[0] is not None else 0.0

    unvisited = set(shape.faces)
    flipped = 0
    while unvisited:
        start = unvisited.pop()
        piece, frontier = [start], [start]
        while frontier:
            face = frontier.pop()
            for edge in face.edges:
                for neighbour in edge.link_faces:
                    if neighbour in unvisited:
                        unvisited.remove(neighbour)
                        piece.append(neighbour)
                        frontier.append(neighbour)

        normal = start.normal
        if any(face.normal.dot(normal) < 0.999 for face in piece):
            continue
        vote = 0.0
        for face in piece:
            centre = face.calc_center_median()
            vote += reach(centre, normal) - reach(centre, -normal)
        if vote < 0:
            for face in piece:
                face.normal_flip()
            flipped += len(piece)

    shape.to_mesh(mesh)
    shape.free()
    print(f"Turned {flipped} of {len(mesh.polygons)} faces toward the room")


def use_gpu(scene):
    preferences = bpy.context.preferences.addons["cycles"].preferences
    for kind in ("OPTIX", "CUDA", "HIP", "ONEAPI", "METAL"):
        try:
            preferences.compute_device_type = kind
        except TypeError:
            continue
        preferences.get_devices()
        devices = [device for device in preferences.devices if device.type == kind]
        if devices:
            for device in devices:
                device.use = True
            scene.cycles.device = "GPU"
            print(f"Baking on {', '.join(device.name for device in devices)} ({kind})")
            return
    print("No GPU found for Cycles; baking on the CPU")


def main():
    room, lights, output = sys.argv[sys.argv.index("--") + 1:]

    bpy.ops.wm.read_factory_settings(use_empty=True)
    # Unmerged, so Blender's vertices are the file's, one for one and in order.
    bpy.ops.import_scene.gltf(filepath=room, merge_vertices=False)
    meshes = [item for item in bpy.context.scene.objects if item.type == "MESH"]
    if len(meshes) != 1:
        raise SystemExit(f"Expected the room as one mesh, found {len(meshes)}")
    room_object = meshes[0]
    mesh = room_object.data
    face_the_room(mesh)
    # The file has no normals, so Blender makes every face flat, and a vertex on a curved surface
    # would take its light from whichever of its faces the bake picked. Smooth shading gives each
    # vertex the average of them. Corners stay sharp: the generator gives each face of a box its
    # own vertices.
    mesh.polygons.foreach_set("use_smooth", [True] * len(mesh.polygons))

    with open(lights, encoding="utf-8") as handle:
        for source in json.load(handle):
            add_light(source)

    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    use_gpu(scene)
    scene.cycles.samples = 2048

    # The room is closed, so nothing comes from the world; black keeps it that way.
    scene.world = bpy.data.worlds.new("outside")
    scene.world.color = (0.0, 0.0, 0.0)

    light = mesh.color_attributes.new("Light", "FLOAT_COLOR", "POINT")
    mesh.color_attributes.active_color = light
    mesh.color_attributes.render_color_index = mesh.color_attributes.active_color_index

    bpy.ops.object.select_all(action="DESELECT")
    room_object.select_set(True)
    bpy.context.view_layer.objects.active = room_object
    scene.render.bake.target = "VERTEX_COLORS"
    bpy.ops.object.bake(type="DIFFUSE", pass_filter={"DIRECT", "INDIRECT"}, target="VERTEX_COLORS")

    # Covered: something lying within a centimetre or so in front of it. Rays tilted a little either
    # way as well as the straight one, so a vertex exactly under the edge of a cover counts too.
    tree = BVHTree.FromObject(room_object, bpy.context.evaluated_depsgraph_get())

    def covered(vertex):
        normal = vertex.normal
        across = normal.orthogonal().normalized()
        other = normal.cross(across)
        for tilt in (Vector((0.0, 0.0, 0.0)), across, -across, other, -other):
            direction = (normal + 0.3 * tilt).normalized()
            if tree.ray_cast(vertex.co + direction * 1e-4, direction, 0.012)[0] is not None:
                return 1.0
        return 0.0

    # The import may have placed the room with a transform; positions are compared in the room's
    # own space.
    to_room = room_object.matrix_world
    with open(output, "wb") as handle:
        for vertex, value in zip(mesh.vertices, light.data):
            position = from_blender(to_room @ vertex.co)
            handle.write(struct.pack("<7f", *position, *value.color[:3], covered(vertex)))
    print(f"Baked {len(mesh.vertices)} vertices")


main()
