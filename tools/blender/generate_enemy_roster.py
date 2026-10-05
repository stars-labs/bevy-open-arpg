"""Shared enemy geometry helpers and boss-skill VFX.

Character models are authored by generate_reliquary_enemies.py.

Run with:
    blender --background --python tools/blender/generate_enemy_roster.py

The models deliberately use procedural geometry and embedded PBR materials so
the checked-in GLBs remain reproducible, compact, and friendly to the WebGPU
build. Blender-space -Y is character forward; glTF export converts this to the
game's +Z-facing convention.
"""

import math
from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "models"
OUT.mkdir(parents=True, exist_ok=True)


def reset():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    bpy.context.scene.name = "Scene"


def merge_render_meshes(preserve_name):
    """Collapse one VFX into a single node while keeping material primitives."""

    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if len(meshes) <= 1:
        return
    target = next((obj for obj in meshes if obj.name == preserve_name), meshes[0])
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = target
    bpy.ops.object.join()
    target.name = preserve_name


def material(name, color, metallic=0.0, roughness=0.72, emission=None, strength=1.0):
    """Create an opaque Principled material.

    Alpha is forced to one and no Blender blend mode is enabled. Consequently
    the glTF exporter emits OPAQUE (or omits alphaMode, whose glTF default is
    OPAQUE), avoiding Bevy's transparent PBR pipeline for character assets.
    """

    rgba = (color[0], color[1], color[2], 1.0)
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.use_nodes = True
    mat.diffuse_color = rgba
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = rgba
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Alpha"].default_value = 1.0
    if emission is not None:
        emission_rgba = (emission[0], emission[1], emission[2], 1.0)
        bsdf.inputs["Emission Color"].default_value = emission_rgba
        bsdf.inputs["Emission Strength"].default_value = strength
    else:
        bsdf.inputs["Emission Strength"].default_value = 0.0
    return mat


OBSIDIAN = material("roster obsidian", (0.025, 0.032, 0.045), 0.52, 0.34)
IRON = material("roster black iron", (0.13, 0.16, 0.19), 0.70, 0.32)
STEEL = material("roster pale steel", (0.45, 0.50, 0.54), 0.78, 0.25)
BONE = material("roster ash bone", (0.60, 0.56, 0.45), 0.0, 0.84)
CLOTH = material("roster char cloth", (0.055, 0.030, 0.065), 0.0, 0.90)
LEATHER = material("roster scorched leather", (0.19, 0.085, 0.035), 0.0, 0.76)
GOLD = material("roster reliquary gold", (0.72, 0.40, 0.075), 0.72, 0.31)
EMBER = material(
    "roster ember",
    (0.72, 0.055, 0.012),
    0.0,
    0.38,
    (1.0, 0.075, 0.008),
    3.2,
)
SOUL = material(
    "roster hollow soul",
    (0.025, 0.46, 0.58),
    0.05,
    0.28,
    (0.015, 0.78, 1.0),
    3.0,
)
VIOLET = material(
    "roster choir violet",
    (0.30, 0.025, 0.47),
    0.08,
    0.31,
    (0.52, 0.025, 0.92),
    2.8,
)
BLOOD = material(
    "roster nemesis blood",
    (0.34, 0.008, 0.012),
    0.05,
    0.42,
    (0.72, 0.008, 0.012),
    2.5,
)


def finish_mesh(obj, mat, smooth=False):
    obj.data.materials.append(mat)
    if smooth:
        for polygon in obj.data.polygons:
            polygon.use_smooth = True
    return obj


def cube(name, loc, scale, mat, rotation=(0.0, 0.0, 0.0), bevel=0.025):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc, rotation=rotation)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel > 0.0:
        modifier = obj.modifiers.new(name="edge softening", type="BEVEL")
        modifier.width = min(bevel, min(scale) * 0.22)
        modifier.segments = 2
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    return finish_mesh(obj, mat)


def sphere(name, loc, scale, mat, segments=20):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments,
        ring_count=max(8, segments // 2),
        location=loc,
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return finish_mesh(obj, mat, smooth=True)


def cylinder(
    name,
    loc,
    radius,
    depth,
    mat,
    vertices=16,
    rotation=(0.0, 0.0, 0.0),
):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=depth,
        location=loc,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    return finish_mesh(obj, mat)


def cone(
    name,
    loc,
    radius1,
    radius2,
    depth,
    mat,
    vertices=16,
    rotation=(0.0, 0.0, 0.0),
):
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radius1,
        radius2=radius2,
        depth=depth,
        location=loc,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    return finish_mesh(obj, mat)


def torus(
    name,
    loc,
    major_radius,
    minor_radius,
    mat,
    rotation=(0.0, 0.0, 0.0),
    major_segments=32,
    minor_segments=8,
):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major_radius,
        minor_radius=minor_radius,
        major_segments=major_segments,
        minor_segments=minor_segments,
        location=loc,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    return finish_mesh(obj, mat, smooth=True)


def arc_prism(name, inner, outer, start, end, height, mat, segments=28):
    """Create a solid horizontal arc centered at the origin."""

    vertices = []
    for index in range(segments + 1):
        angle = start + (end - start) * index / segments
        sin_angle = math.sin(angle)
        cos_angle = math.cos(angle)
        for z, radius in ((0.0, inner), (0.0, outer), (height, inner), (height, outer)):
            vertices.append((sin_angle * radius, -cos_angle * radius, z))

    faces = []
    for index in range(segments):
        current = index * 4
        following = (index + 1) * 4
        # bottom, top, inner wall, outer wall
        faces.extend(
            [
                (current, following, following + 1, current + 1),
                (current + 2, current + 3, following + 3, following + 2),
                (current, current + 2, following + 2, following),
                (current + 1, following + 1, following + 3, current + 3),
            ]
        )
    faces.append((0, 1, 3, 2))
    last = segments * 4
    faces.append((last, last + 2, last + 3, last + 1))

    mesh = bpy.data.meshes.new(f"{name} mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    return finish_mesh(obj, mat)


def empty(name, loc, display="PLAIN_AXES", size=0.12, properties=None):
    obj = bpy.data.objects.new(name, None)
    bpy.context.collection.objects.link(obj)
    obj.location = loc
    obj.empty_display_type = display
    obj.empty_display_size = size
    obj.hide_render = True
    if properties:
        for key, value in properties.items():
            obj[key] = value
    return obj


def add_hit_rig(scale=1.0, weapon_side=1.0, height=1.9):
    """Add the repository's stable hit-bone and contact-zone contract."""

    armature = bpy.data.armatures.new("hit_bones")
    armature.display_type = "STICK"
    rig = bpy.data.objects.new("hit_bones", armature)
    bpy.context.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")

    bones = {
        "hit_head": (
            (0.0, 0.0, height * 0.77),
            (0.0, 0.0, height),
        ),
        "hit_chest": (
            (0.0, 0.0, height * 0.40),
            (0.0, 0.0, height * 0.70),
        ),
        "hit_weapon": (
            (0.50 * weapon_side * scale, -0.12 * scale, height * 0.24),
            (0.88 * weapon_side * scale, -0.20 * scale, height * 0.72),
        ),
        "hit_left": (
            (-0.38 * scale, 0.0, height * 0.38),
            (-0.56 * scale, -0.02, height * 0.70),
        ),
        "hit_right": (
            (0.38 * scale, 0.0, height * 0.38),
            (0.56 * scale, -0.02, height * 0.70),
        ),
    }
    for bone_name, (head, tail) in bones.items():
        bone = armature.edit_bones.new(bone_name)
        bone.head = head
        bone.tail = tail
        bone.use_deform = False
    bpy.ops.object.mode_set(mode="OBJECT")

    radii = {
        "hit_head": 0.18 * scale,
        "hit_chest": 0.28 * scale,
        "hit_weapon": 0.18 * scale,
        "hit_left": 0.16 * scale,
        "hit_right": 0.16 * scale,
    }
    for bone_name, (_, tail) in bones.items():
        empty(
            f"{bone_name}_marker",
            tail,
            display="SPHERE",
            size=0.055 * scale,
            properties={"semantic": "vfx_marker", "hit_bone": bone_name},
        )
        empty(
            f"{bone_name}_contact_zone",
            tail,
            display="CIRCLE",
            size=radii[bone_name],
            properties={
                "semantic": "contact_zone",
                "hit_bone": bone_name,
                "radius": radii[bone_name],
            },
        )

    empty("socket_weapon", bones["hit_weapon"][1], properties={"semantic": "socket"})
    empty(
        "socket_cast",
        (0.0, -0.30 * scale, height * 0.67),
        properties={"semantic": "socket"},
    )
    empty(
        "socket_head_fx",
        bones["hit_head"][1],
        properties={"semantic": "socket"},
    )
    empty(
        "socket_chest_fx",
        bones["hit_chest"][1],
        properties={"semantic": "socket"},
    )
    empty(
        "socket_ground_fx",
        (0.0, 0.0, 0.04),
        properties={"semantic": "socket"},
    )


def export(name):
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(OUT / f"{name}.glb"),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_yup=True,
        export_animations=False,
        export_cameras=False,
        export_lights=False,
        export_extras=True,
    )


def tapered_arc(name, radius, width, start, end, height, mat, segments=48):
    """A tapered blade ribbon; tips close to a point, no filled pie sector."""
    vertices = []
    for index in range(segments + 1):
        t = index / segments
        angle = start + (end - start) * t
        half = width * math.sin(math.pi * t) ** 0.75 / 2
        for r in (radius - half, radius + half):
            vertices.append((math.sin(angle) * r, -math.cos(angle) * r, height))
    faces = [(i * 2, i * 2 + 1, i * 2 + 3, i * 2 + 2) for i in range(segments)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    return finish_mesh(obj, mat)


def wire(name, points, mat, thickness=0.018, cyclic=False):
    curve = bpy.data.curves.new(name, "CURVE")
    curve.dimensions = "3D"
    curve.bevel_depth = thickness
    curve.bevel_resolution = 1
    spline = curve.splines.new("POLY")
    spline.points.add(len(points) - 1)
    for point, xyz in zip(spline.points, points):
        point.co = (*xyz, 1.0)
    spline.use_cyclic_u = cyclic
    obj = bpy.data.objects.new(name, curve)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(mat)
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.convert(target="MESH")
    return bpy.context.object


def boss_material(name, color, glow=None):
    return material(name, color, metallic=0.45 if glow is None else 0.0,
                    roughness=0.38, emission=glow, strength=0.85)


def keeper_cleaver_wave():
    reset()
    ember = boss_material("cleaver smolder edge", (0.62, 0.16, 0.045), (0.75, 0.14, 0.025))
    steel = boss_material("cleaver ash edge", (0.40, 0.36, 0.27))
    tapered_arc("keeper cleaver wave crescent", 2.30, 0.15, -1.06, 1.06, 0.055, ember)
    tapered_arc("cleaver fine leading edge", 2.38, 0.028, -1.0, 1.0, 0.06, steel)
    # One restrained trailing fracture, leaving the attack area unobscured.
    tapered_arc("cleaver cooling wake", 2.05, 0.045, -0.70, 0.65, 0.035, steel)
    empty("socket_ground_fx", (0.0, 0.0, 0.0), properties={"semantic": "socket"})
    merge_render_meshes("keeper cleaver wave crescent")
    export("keeper_cleaver_wave")


def keeper_chain_cage():
    reset()
    iron = boss_material("cage tempered iron", (0.22, 0.20, 0.17))
    ember = boss_material("cage binding etch", (0.60, 0.22, 0.06), (0.60, 0.13, 0.025))
    for pillar in range(4):
        angle = pillar * math.tau / 4 + math.pi / 4
        tapered_arc("keeper interlocking chain cage" if pillar == 0 else f"cage broken seal {pillar}",
                    1.28, 0.035, angle - 0.45, angle + 0.45, 0.035, ember)
        # Alternating elongated links interlock vertically; no upright glowing rods.
        for link in range(6):
            orientation = angle + (math.pi / 2 if link % 2 else 0)
            cx = math.cos(angle) * (1.15 - 0.035 * link)
            cy = math.sin(angle) * (1.15 - 0.035 * link)
            cz = 0.16 + link * 0.255
            points = []
            for step in range(20):
                theta = step * math.tau / 20
                side = math.cos(theta) * 0.07
                points.append((cx + math.cos(orientation) * side,
                               cy + math.sin(orientation) * side,
                               cz + math.sin(theta) * 0.17))
            wire(f"interlocked iron link {pillar}-{link}", points, iron, 0.017, True)
        # Curved inward hook at the top of each chain.
        wire(f"cage hook {pillar}", [
            (math.cos(angle) * 0.98, math.sin(angle) * 0.98, 1.62),
            (math.cos(angle) * 0.93, math.sin(angle) * 0.93, 1.76),
            (math.cos(angle) * 0.82, math.sin(angle) * 0.82, 1.79),
            (math.cos(angle) * 0.78, math.sin(angle) * 0.78, 1.68),
        ], iron, 0.024)
    empty("socket_ground_fx", (0.0, 0.0, 0.0), properties={"semantic": "socket"})
    merge_render_meshes("keeper interlocking chain cage")
    export("keeper_chain_cage")


def matron_soul_lance():
    reset()
    soul = boss_material("soul lance cold edge", (0.12, 0.48, 0.52), (0.08, 0.56, 0.62))
    silver = boss_material("soul lance silver spine", (0.38, 0.44, 0.46))
    # Four faceted edges taper into a spear point. Forward is Blender -Y.
    vertices = [(0, -3.45, 0.28), (0, 0, 0.28),
                (-0.105, -2.58, 0.28), (0.105, -2.58, 0.28),
                (0, -2.58, 0.37), (0, -2.58, 0.19)]
    faces = [(0, 2, 4), (0, 4, 3), (0, 3, 5), (0, 5, 2),
             (1, 4, 2), (1, 3, 4), (1, 5, 3), (1, 2, 5)]
    mesh = bpy.data.meshes.new("soul lance faceted spear")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new("matron tapered soul spear", mesh)
    bpy.context.collection.objects.link(obj)
    finish_mesh(obj, soul)
    for side in (-1, 1):
        wire(f"soul lance split wake {side}",
             [(0, -1.45, 0.28), (side * 0.09, -0.8, 0.28),
              (side * 0.12, -0.25, 0.28)], silver, 0.012)
    empty("socket_cast", (0.0, 0.0, 0.28), properties={"semantic": "socket"})
    merge_render_meshes("matron tapered soul spear")
    export("matron_soul_lance")


def matron_crescendo():
    reset()
    violet = boss_material("crescendo dusk resonance", (0.30, 0.15, 0.42), (0.32, 0.09, 0.47))
    soul = boss_material("crescendo pale resonance", (0.16, 0.48, 0.52), (0.08, 0.48, 0.55))
    # Six broken wavefronts read as a expanding sonic ring, without ball/spikes.
    for index in range(6):
        angle = index * math.tau / 6
        tapered_arc("matron broken resonance wave" if index == 0 else f"resonance arc {index}",
                    1.72, 0.065, angle - 0.40, angle + 0.40, 0.055, soul)
        tapered_arc(f"resonance aftertone {index}", 1.52, 0.025,
                    angle - 0.25, angle + 0.25, 0.035, violet)
    empty("socket_ground_fx", (0.0, 0.0, 0.0), properties={"semantic": "socket"})
    merge_render_meshes("matron broken resonance wave")
    export("matron_crescendo")


def main():
    import sys
    generators = (keeper_cleaver_wave, keeper_chain_cage, matron_soul_lance, matron_crescendo)
    if "--preview-only" in sys.argv:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import generate_reliquary_props as art
        art.PREVIEW = ROOT / "target" / "validation" / "boss-vfx-audit"
        art.PREVIEW.mkdir(parents=True, exist_ok=True)
        for generator in generators:
            art.preview(generator.__name__)
    else:
        for generator in generators:
            generator()


if __name__ == "__main__":
    main()
