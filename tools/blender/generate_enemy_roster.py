"""Generate the distinct enemy roster and boss-skill VFX.

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


def ashbone_guard():
    reset()
    # Broad, shield-led Vanguard silhouette for the baseline Ashbone Guard.
    cube("ashbone guard plated ribs", (0.0, 0.04, 0.98), (0.70, 0.46, 1.26), BONE)
    cube("ashbone guard iron tabard", (0.0, -0.16, 0.78), (0.48, 0.18, 0.92), IRON)
    sphere(
        "ashbone guard crowned skull",
        (0.0, 0.0, 1.70),
        (0.33, 0.30, 0.35),
        BONE,
        20,
    )
    cube("ashbone guard visor", (0.0, -0.295, 1.70), (0.40, 0.045, 0.10), OBSIDIAN)
    cube("ashbone guard ember sight", (0.0, -0.325, 1.70), (0.22, 0.025, 0.035), EMBER)
    for side in (-1.0, 1.0):
        cube(
            f"ashbone guard {'left' if side < 0 else 'right'} pauldron",
            (side * 0.46, 0.0, 1.37),
            (0.34, 0.54, 0.25),
            STEEL,
            (0.0, math.radians(side * 5), math.radians(side * 9)),
        )
        cube(
            f"ashbone guard {'left' if side < 0 else 'right'} greave",
            (side * 0.19, 0.0, 0.27),
            (0.22, 0.30, 0.54),
            IRON,
        )
    cone(
        "ashbone guard helm crest",
        (0.0, 0.03, 2.04),
        0.12,
        0.0,
        0.56,
        GOLD,
        10,
    )

    # An oversized octagonal shield sits ahead of the left shoulder. A tapered
    # lower plate gives it a kite profile instead of a generic round buckler.
    cylinder(
        "ashbone guard greatshield",
        (-0.58, -0.31, 1.05),
        0.62,
        0.13,
        IRON,
        8,
        (math.radians(90), 0.0, 0.0),
    )
    cone(
        "ashbone guard shield point",
        (-0.58, -0.31, 0.47),
        0.43,
        0.08,
        0.72,
        IRON,
        8,
    )
    torus(
        "ashbone guard shield ward",
        (-0.58, -0.385, 1.05),
        0.37,
        0.045,
        GOLD,
        (math.radians(90), 0.0, 0.0),
        24,
        7,
    )
    sphere(
        "ashbone guard shield boss",
        (-0.58, -0.42, 1.05),
        (0.15, 0.07, 0.15),
        EMBER,
        14,
    )
    for index in range(4):
        angle = index * math.tau / 4 + math.pi / 4
        sphere(
            f"ashbone guard shield rivet {index}",
            (
                -0.58 + math.cos(angle) * 0.46,
                -0.39,
                1.05 + math.sin(angle) * 0.46,
            ),
            (0.045, 0.025, 0.045),
            GOLD,
            10,
        )

    # Compact one-handed blade keeps the shield, not the weapon, dominant.
    cube(
        "ashbone guard short blade",
        (0.62, -0.18, 1.00),
        (0.11, 0.075, 1.08),
        STEEL,
        (math.radians(5), math.radians(-12), math.radians(-12)),
        0.012,
    )
    cube(
        "ashbone guard sword grip",
        (0.51, -0.10, 0.48),
        (0.15, 0.13, 0.52),
        LEATHER,
        (0.0, 0.0, math.radians(-12)),
    )
    cube(
        "ashbone guard sword guard",
        (0.56, -0.14, 0.69),
        (0.40, 0.10, 0.09),
        GOLD,
        (0.0, 0.0, math.radians(-12)),
    )
    add_hit_rig(scale=1.02, height=2.06)
    export("ashbone_guard")


def bone_stalker():
    reset()
    # Low, forward-leaning silhouette with long hooked claws.
    cube(
        "stalker hunched ribs",
        (0.0, 0.05, 0.88),
        (0.52, 0.42, 0.64),
        BONE,
        (math.radians(-15), 0.0, 0.0),
    )
    sphere("stalker wedge skull", (0.0, -0.18, 1.35), (0.27, 0.31, 0.24), BONE, 18)
    cube("stalker blindfold", (0.0, -0.47, 1.37), (0.38, 0.055, 0.10), CLOTH)
    sphere("stalker ember eye", (0.0, -0.51, 1.37), (0.07, 0.035, 0.045), EMBER, 12)
    for side in (-1.0, 1.0):
        cube(
            f"stalker {'left' if side < 0 else 'right'} upper limb",
            (side * 0.37, -0.08, 0.78),
            (0.14, 0.15, 0.68),
            BONE,
            (math.radians(22), math.radians(side * 18), math.radians(side * 13)),
        )
        cube(
            f"stalker {'left' if side < 0 else 'right'} shin",
            (side * 0.20, 0.04, 0.27),
            (0.13, 0.18, 0.52),
            OBSIDIAN,
            (math.radians(-8), 0.0, math.radians(side * 6)),
        )
        cube(
            f"stalker {'left' if side < 0 else 'right'} sickle",
            (side * 0.58, -0.37, 0.72),
            (0.10, 0.08, 0.92),
            STEEL,
            (math.radians(28), math.radians(side * 16), math.radians(side * 18)),
            0.01,
        )
        cone(
            f"stalker {'left' if side < 0 else 'right'} claw tip",
            (side * 0.68, -0.62, 0.30),
            0.105,
            0.0,
            0.42,
            STEEL,
            10,
            (math.radians(46), 0.0, math.radians(-side * 12)),
        )
        cube(
            f"stalker {'left' if side < 0 else 'right'} rear spine",
            (side * 0.20, 0.25, 1.03),
            (0.075, 0.28, 0.30),
            OBSIDIAN,
            (math.radians(-28), 0.0, math.radians(side * 10)),
            0.01,
        )
    add_hit_rig(scale=0.86, height=1.58)
    export("bone_stalker")


def hollow_stalker():
    reset()
    # Chapter II counterpart: a low soul-fed hunter with a forked silhouette.
    cube(
        "hollow stalker bowed cage",
        (0.0, 0.08, 0.83),
        (0.58, 0.46, 0.62),
        OBSIDIAN,
        (math.radians(-18), 0.0, 0.0),
    )
    torus(
        "hollow stalker soul ribs",
        (0.0, -0.15, 0.91),
        0.26,
        0.037,
        SOUL,
        (math.radians(90), 0.0, 0.0),
        22,
        6,
    )
    sphere(
        "hollow stalker eyeless skull",
        (0.0, -0.22, 1.34),
        (0.28, 0.31, 0.25),
        BONE,
        18,
    )
    cube("hollow stalker soul maw", (0.0, -0.51, 1.30), (0.29, 0.045, 0.075), SOUL)
    for side in (-1.0, 1.0):
        cube(
            f"hollow stalker {'left' if side < 0 else 'right'} long arm",
            (side * 0.40, -0.11, 0.77),
            (0.14, 0.16, 0.76),
            IRON,
            (math.radians(24), math.radians(side * 17), math.radians(side * 16)),
        )
        cube(
            f"hollow stalker {'left' if side < 0 else 'right'} reverse leg",
            (side * 0.22, 0.03, 0.26),
            (0.14, 0.20, 0.52),
            BONE,
            (math.radians(-10), 0.0, math.radians(side * 8)),
        )
        cone(
            f"hollow stalker {'left' if side < 0 else 'right'} soul talon",
            (side * 0.61, -0.54, 0.48),
            0.13,
            0.0,
            0.68,
            SOUL,
            10,
            (math.radians(48), math.radians(side * 8), math.radians(-side * 16)),
        )
        cone(
            f"hollow stalker dorsal spike {0 if side < 0 else 1}",
            (side * 0.20, 0.31, 1.16),
            0.09,
            0.0,
            0.52,
            VIOLET,
            9,
            (math.radians(-45), math.radians(side * 12), 0.0),
        )
    sphere("hollow stalker exposed soul", (0.0, -0.31, 0.88), (0.12, 0.06, 0.14), SOUL, 14)
    add_hit_rig(scale=0.88, height=1.58)
    export("hollow_stalker")


def ashen_marksman():
    reset()
    cube("marksman split coat", (0.0, 0.03, 0.80), (0.54, 0.38, 1.36), CLOTH)
    sphere("marksman hood", (0.0, 0.0, 1.58), (0.30, 0.28, 0.34), OBSIDIAN, 20)
    cube("marksman ember visor", (0.0, -0.275, 1.58), (0.34, 0.045, 0.055), EMBER)
    for side in (-1.0, 1.0):
        cube(
            f"marksman {'left' if side < 0 else 'right'} mantle",
            (side * 0.37, 0.0, 1.30),
            (0.27, 0.44, 0.18),
            LEATHER,
            (0.0, math.radians(side * 8), math.radians(side * 8)),
        )
        cube(
            f"marksman {'left' if side < 0 else 'right'} boot",
            (side * 0.16, -0.02, 0.22),
            (0.18, 0.25, 0.44),
            IRON,
        )
    # Wide crossbow instantly separates this ranged unit at camera distance.
    cube("marksman crossbow stock", (0.22, -0.40, 1.04), (0.12, 0.12, 1.02), LEATHER)
    cube(
        "marksman crossbow bow",
        (0.22, -0.44, 1.31),
        (1.14, 0.08, 0.12),
        STEEL,
        (0.0, math.radians(-5), 0.0),
    )
    cone(
        "marksman loaded bolt",
        (0.22, -0.76, 1.31),
        0.055,
        0.0,
        0.78,
        EMBER,
        10,
        (math.radians(90), 0.0, 0.0),
    )
    cube(
        "marksman back quiver",
        (-0.30, 0.25, 1.02),
        (0.26, 0.22, 0.82),
        LEATHER,
        (math.radians(-14), 0.0, math.radians(-13)),
    )
    for index in range(4):
        cylinder(
            f"marksman quiver bolt {index}",
            (-0.40 + index * 0.07, 0.28, 1.48 + (index % 2) * 0.07),
            0.018,
            0.66,
            STEEL,
            8,
            (math.radians(-12), 0.0, math.radians(-10 + index * 5)),
        )
    add_hit_rig(scale=0.94, height=1.94)
    export("ashen_marksman")


def choir_marksman():
    reset()
    cube("choir marksman divided robe", (0.0, 0.04, 0.84), (0.60, 0.44, 1.46), CLOTH)
    sphere("choir marksman listening cowl", (0.0, 0.0, 1.63), (0.31, 0.29, 0.35), OBSIDIAN, 20)
    cube("choir marksman voice slit", (0.0, -0.285, 1.62), (0.32, 0.045, 0.065), SOUL)
    for side in (-1.0, 1.0):
        cube(
            f"choir marksman {'left' if side < 0 else 'right'} sound mantle",
            (side * 0.39, 0.08, 1.31),
            (0.26, 0.44, 0.18),
            VIOLET,
            (0.0, math.radians(side * 9), math.radians(side * 9)),
        )
        cube(
            f"choir marksman {'left' if side < 0 else 'right'} foot",
            (side * 0.17, -0.02, 0.22),
            (0.19, 0.27, 0.44),
            IRON,
        )
    # Long-range tuning-fork launcher, readable from the isometric camera.
    cube(
        "choir marksman resonance stock",
        (0.25, -0.34, 1.05),
        (0.12, 0.12, 1.24),
        IRON,
        (math.radians(7), 0.0, 0.0),
    )
    cube(
        "choir marksman fork crossbar",
        (0.25, -0.43, 1.40),
        (1.20, 0.09, 0.11),
        STEEL,
    )
    for side in (-1.0, 1.0):
        cube(
            f"choir marksman fork tine {0 if side < 0 else 1}",
            (0.25 + side * 0.52, -0.58, 1.40),
            (0.10, 0.44, 0.10),
            SOUL,
            (0.0, 0.0, math.radians(side * 4)),
        )
        torus(
            f"choir marksman tuning halo {0 if side < 0 else 1}",
            (0.25 + side * 0.50, -0.43, 1.40),
            0.17,
            0.025,
            VIOLET,
            (math.radians(90), 0.0, 0.0),
            18,
            6,
        )
    cylinder(
        "choir marksman loaded tone",
        (0.25, -0.89, 1.40),
        0.055,
        1.08,
        SOUL,
        10,
        (math.radians(90), 0.0, 0.0),
    )
    torus(
        "choir marksman back antenna",
        (-0.28, 0.20, 1.56),
        0.31,
        0.030,
        VIOLET,
        (math.radians(90), 0.0, math.radians(-12)),
    )
    add_hit_rig(scale=0.96, height=1.99)
    export("choir_marksman")


def treasure_imp():
    reset()
    sphere("imp hunched body", (0.0, 0.02, 0.53), (0.34, 0.29, 0.45), LEATHER, 18)
    sphere("imp masked head", (0.0, -0.14, 0.98), (0.25, 0.25, 0.25), OBSIDIAN, 18)
    cube("imp gold grin", (0.0, -0.38, 0.93), (0.28, 0.045, 0.07), GOLD)
    for side in (-1.0, 1.0):
        cone(
            f"imp {'left' if side < 0 else 'right'} horn",
            (side * 0.18, -0.01, 1.22),
            0.085,
            0.0,
            0.38,
            BONE,
            10,
            (0.0, math.radians(side * 28), math.radians(-side * 22)),
        )
        cube(
            f"imp {'left' if side < 0 else 'right'} foot",
            (side * 0.18, -0.09, 0.10),
            (0.22, 0.34, 0.19),
            IRON,
        )
        sphere(
            f"imp coin pouch {0 if side < 0 else 1}",
            (side * 0.38, 0.03, 0.48),
            (0.18, 0.14, 0.22),
            GOLD,
            14,
        )
    cube("imp reliquary pack", (0.0, 0.32, 0.73), (0.78, 0.48, 0.82), LEATHER)
    cube("imp pack lid", (0.0, 0.32, 1.16), (0.84, 0.52, 0.18), GOLD)
    cube("imp pack lock", (0.0, 0.055, 0.75), (0.18, 0.045, 0.22), EMBER)
    for index in range(5):
        cylinder(
            f"imp exposed coin {index}",
            (-0.24 + index * 0.12, 0.04, 1.29 + 0.03 * (index % 2)),
            0.07,
            0.025,
            GOLD,
            14,
        )
    add_hit_rig(scale=0.64, height=1.28)
    export("treasure_imp")


def seal_warden():
    reset()
    cube("warden layered vestment", (0.0, 0.05, 0.96), (0.76, 0.50, 1.62), CLOTH)
    cube("warden iron cuirass", (0.0, -0.09, 1.25), (0.68, 0.48, 0.66), IRON)
    sphere("warden sealed helm", (0.0, 0.0, 1.92), (0.36, 0.32, 0.38), IRON, 20)
    cube("warden rune slit", (0.0, -0.315, 1.92), (0.40, 0.045, 0.075), VIOLET)
    torus(
        "warden crown seal",
        (0.0, 0.10, 2.14),
        0.43,
        0.045,
        GOLD,
        (math.radians(90), 0.0, 0.0),
    )
    for side in (-1.0, 1.0):
        cube(
            f"warden {'left' if side < 0 else 'right'} ward plate",
            (side * 0.51, 0.0, 1.34),
            (0.34, 0.54, 0.27),
            GOLD,
            (0.0, 0.0, math.radians(side * 10)),
        )
        cube(
            f"warden {'left' if side < 0 else 'right'} sabaton",
            (side * 0.22, -0.03, 0.24),
            (0.24, 0.34, 0.48),
            OBSIDIAN,
        )
    cylinder("warden seal staff", (0.68, -0.03, 1.08), 0.055, 2.06, STEEL, 12)
    torus(
        "warden staff focus",
        (0.68, -0.03, 2.13),
        0.24,
        0.045,
        VIOLET,
        (math.radians(90), 0.0, 0.0),
    )
    sphere("warden staff heart", (0.68, -0.03, 2.13), (0.10, 0.10, 0.10), EMBER, 14)
    torus(
        "warden floating ward",
        (-0.55, -0.30, 1.18),
        0.44,
        0.07,
        VIOLET,
        (math.radians(90), 0.0, 0.0),
    )
    cylinder(
        "warden ward face",
        (-0.55, -0.30, 1.18),
        0.32,
        0.035,
        GOLD,
        10,
        (math.radians(90), 0.0, 0.0),
    )
    add_hit_rig(scale=1.05, height=2.24)
    export("seal_warden")


def keeper():
    reset()
    cube("keeper furnace torso", (0.0, 0.05, 1.24), (1.22, 0.76, 1.54), OBSIDIAN)
    cube("keeper ribbed breastplate", (0.0, -0.23, 1.42), (1.06, 0.40, 0.82), IRON)
    sphere("keeper execution helm", (0.0, 0.0, 2.20), (0.48, 0.42, 0.47), IRON, 22)
    cube("keeper ash visor", (0.0, -0.415, 2.18), (0.54, 0.055, 0.11), EMBER)
    sphere("keeper furnace heart", (0.0, -0.45, 1.45), (0.20, 0.08, 0.20), EMBER, 18)
    for side in (-1.0, 1.0):
        cube(
            f"keeper {'left' if side < 0 else 'right'} anvil pauldron",
            (side * 0.79, 0.0, 1.72),
            (0.54, 0.76, 0.36),
            STEEL,
            (0.0, math.radians(side * 5), math.radians(side * 12)),
        )
        cube(
            f"keeper {'left' if side < 0 else 'right'} arm",
            (side * 0.73, -0.03, 1.18),
            (0.36, 0.38, 1.02),
            IRON,
            (0.0, 0.0, math.radians(side * 5)),
        )
        cube(
            f"keeper {'left' if side < 0 else 'right'} greave",
            (side * 0.34, 0.0, 0.38),
            (0.38, 0.47, 0.76),
            OBSIDIAN,
        )
        cone(
            f"keeper crown spike {0 if side < 0 else 1}",
            (side * 0.30, 0.02, 2.67),
            0.11,
            0.0,
            0.55,
            GOLD,
            10,
            (0.0, math.radians(side * 8), math.radians(-side * 8)),
        )
    cone("keeper crown center", (0.0, 0.02, 2.73), 0.13, 0.0, 0.64, GOLD, 10)
    # Oversized hooked cleaver on the right.
    cube(
        "keeper cleaver slab",
        (1.12, -0.12, 1.32),
        (0.42, 0.18, 1.78),
        STEEL,
        (0.0, math.radians(-21), math.radians(-7)),
    )
    cube(
        "keeper cleaver ember edge",
        (1.29, -0.22, 1.32),
        (0.075, 0.06, 1.60),
        EMBER,
        (0.0, math.radians(-21), math.radians(-7)),
        0.008,
    )
    cube("keeper cleaver grip", (0.86, -0.02, 0.58), (0.16, 0.15, 0.92), LEATHER)
    for index in range(5):
        torus(
            f"keeper hanging chain link {index}",
            (-0.76, 0.02, 1.40 - index * 0.22),
            0.115,
            0.028,
            GOLD,
            (math.radians(90 if index % 2 == 0 else 0), 0.0, 0.0),
            16,
            6,
        )
    add_hit_rig(scale=1.30, height=2.78)
    export("keeper")


def nemesis():
    reset()
    cube(
        "nemesis torn cuirass",
        (0.0, 0.03, 0.97),
        (0.76, 0.46, 1.48),
        OBSIDIAN,
        (0.0, 0.0, math.radians(-3)),
    )
    sphere("nemesis faceless mask", (0.0, -0.02, 1.80), (0.34, 0.31, 0.38), BONE, 20)
    cube("nemesis blood glyph", (0.0, -0.315, 1.80), (0.08, 0.04, 0.34), BLOOD)
    sphere("nemesis bound heart", (0.0, -0.30, 1.13), (0.16, 0.075, 0.16), BLOOD, 16)
    for side in (-1.0, 1.0):
        cube(
            f"nemesis {'left' if side < 0 else 'right'} ragged mantle",
            (side * 0.47, 0.11, 1.36),
            (0.34, 0.50, 0.26),
            CLOTH if side < 0 else IRON,
            (math.radians(-8), math.radians(side * 10), math.radians(side * 12)),
        )
        cube(
            f"nemesis {'left' if side < 0 else 'right'} boot",
            (side * 0.20, -0.02, 0.24),
            (0.22, 0.30, 0.48),
            IRON,
        )
        cube(
            f"nemesis {'left' if side < 0 else 'right'} blade",
            (side * 0.58, -0.22, 0.97),
            (0.11, 0.08, 1.18 if side > 0 else 0.88),
            STEEL,
            (
                math.radians(14),
                math.radians(side * 20),
                math.radians(side * 22),
            ),
            0.012,
        )
        cone(
            f"nemesis back thorn {0 if side < 0 else 1}",
            (side * 0.30, 0.34, 1.46),
            0.10,
            0.0,
            0.62,
            BLOOD,
            10,
            (math.radians(-48), math.radians(side * 14), 0.0),
        )
    torus(
        "nemesis broken halo",
        (0.0, 0.12, 2.02),
        0.43,
        0.035,
        BLOOD,
        (math.radians(90), 0.0, 0.0),
    )
    add_hit_rig(scale=1.02, height=2.08)
    export("nemesis")


def bell_warden():
    reset()
    cube("bell warden choir robe", (0.0, 0.04, 0.96), (0.88, 0.56, 1.72), CLOTH)
    cube("bell warden rib armor", (0.0, -0.18, 1.30), (0.72, 0.35, 0.76), IRON)
    sphere("bell warden cowl", (0.0, 0.0, 1.95), (0.38, 0.34, 0.41), OBSIDIAN, 20)
    cube("bell warden voice slit", (0.0, -0.335, 1.92), (0.38, 0.045, 0.08), SOUL)
    for side in (-1.0, 1.0):
        cube(
            f"bell warden {'left' if side < 0 else 'right'} choir wing",
            (side * 0.52, 0.22, 1.38),
            (0.30, 0.28, 1.12),
            VIOLET,
            (math.radians(-16), math.radians(side * 15), math.radians(side * 12)),
        )
        cube(
            f"bell warden {'left' if side < 0 else 'right'} foot",
            (side * 0.23, -0.02, 0.22),
            (0.25, 0.34, 0.44),
            OBSIDIAN,
        )
    # A large shoulder-slung bell and fork make this warden distinct from the acolyte.
    cone(
        "bell warden great bell",
        (0.60, -0.02, 1.45),
        0.40,
        0.24,
        0.72,
        GOLD,
        24,
    )
    sphere("bell warden clapper", (0.60, -0.02, 1.02), (0.095, 0.095, 0.12), SOUL, 12)
    cube("bell warden fork stem", (-0.60, -0.03, 1.18), (0.10, 0.10, 1.72), STEEL)
    cube("bell warden fork left tine", (-0.74, -0.03, 2.10), (0.10, 0.10, 0.56), STEEL)
    cube("bell warden fork right tine", (-0.46, -0.03, 2.10), (0.10, 0.10, 0.56), STEEL)
    for index in range(3):
        torus(
            f"bell warden resonance ring {index}",
            (0.0, 0.10, 1.25 + index * 0.32),
            0.42 + index * 0.12,
            0.030,
            SOUL if index % 2 == 0 else VIOLET,
            (math.radians(90), 0.0, 0.0),
        )
    add_hit_rig(scale=1.08, weapon_side=-1.0, height=2.34)
    export("bell_warden")


def keeper_cleaver_wave():
    reset()
    wave = arc_prism(
        "keeper cleaver wave core",
        inner=0.72,
        outer=2.35,
        start=-1.05,
        end=1.05,
        height=0.075,
        mat=EMBER,
        segments=34,
    )
    wave.location.z = 0.035
    arc_prism(
        "keeper cleaver wave iron edge",
        inner=2.28,
        outer=2.48,
        start=-1.08,
        end=1.08,
        height=0.11,
        mat=STEEL,
        segments=34,
    )
    for index in range(7):
        angle = -0.84 + index * 0.28
        cone(
            f"keeper cleaver wave cinder {index}",
            (math.sin(angle) * 1.72, -math.cos(angle) * 1.72, 0.24 + 0.05 * (index % 2)),
            0.09,
            0.0,
            0.46,
            EMBER,
            8,
            (math.radians(10 + index * 3), angle, 0.0),
        )
    empty("socket_ground_fx", (0.0, 0.0, 0.0), properties={"semantic": "socket"})
    merge_render_meshes("keeper cleaver wave core")
    export("keeper_cleaver_wave")


def keeper_chain_cage():
    reset()
    torus("keeper chain cage ring", (0.0, 0.0, 0.08), 1.15, 0.09, EMBER)
    torus("keeper chain cage outer seal", (0.0, 0.0, 0.06), 1.43, 0.035, GOLD)
    for pillar in range(8):
        angle = pillar * math.tau / 8
        x = math.cos(angle) * 1.15
        y = math.sin(angle) * 1.15
        for link in range(6):
            torus(
                f"keeper chain cage link {pillar}-{link}",
                (x, y, 0.20 + link * 0.31),
                0.105,
                0.027,
                GOLD if link % 2 == 0 else IRON,
                (
                    math.radians(90 if link % 2 == 0 else 0),
                    angle,
                    0.0,
                ),
                14,
                6,
            )
        cone(
            f"keeper chain cage hook {pillar}",
            (x, y, 2.08),
            0.11,
            0.0,
            0.46,
            EMBER,
            9,
        )
    empty("socket_ground_fx", (0.0, 0.0, 0.0), properties={"semantic": "socket"})
    merge_render_meshes("keeper chain cage ring")
    export("keeper_chain_cage")


def matron_soul_lance():
    reset()
    # The asset points along Blender -Y / Bevy +Z.
    cylinder(
        "matron soul lance core",
        (0.0, -1.55, 0.28),
        0.11,
        3.10,
        SOUL,
        16,
        (math.radians(90), 0.0, 0.0),
    )
    cone(
        "matron soul lance spearhead",
        (0.0, -3.22, 0.28),
        0.34,
        0.0,
        0.78,
        VIOLET,
        12,
        (math.radians(90), 0.0, 0.0),
    )
    for index in range(4):
        torus(
            f"matron soul lance focus ring {index}",
            (0.0, -0.45 - index * 0.72, 0.28),
            0.28 + index * 0.035,
            0.030,
            VIOLET if index % 2 else SOUL,
            (math.radians(90), 0.0, 0.0),
            20,
            6,
        )
    empty("socket_cast", (0.0, 0.0, 0.28), properties={"semantic": "socket"})
    merge_render_meshes("matron soul lance core")
    export("matron_soul_lance")


def matron_crescendo():
    reset()
    torus("matron crescendo crown", (0.0, 0.0, 0.09), 1.72, 0.075, VIOLET)
    for ring in range(3):
        torus(
            f"matron crescendo resonance ring {ring}",
            (0.0, 0.0, 0.12 + ring * 0.10),
            0.66 + ring * 0.43,
            0.040,
            SOUL if ring % 2 == 0 else VIOLET,
        )
    for index in range(12):
        angle = index * math.tau / 12
        cone(
            f"matron crescendo voice spike {index}",
            (
                math.cos(angle) * 1.72,
                math.sin(angle) * 1.72,
                0.42 + 0.08 * (index % 3),
            ),
            0.12,
            0.0,
            0.82 + 0.09 * (index % 2),
            SOUL if index % 2 == 0 else VIOLET,
            10,
            (
                math.radians(18),
                math.sin(angle) * 0.18,
                angle,
            ),
        )
    sphere("matron crescendo heart", (0.0, 0.0, 0.48), (0.24, 0.24, 0.34), SOUL, 18)
    empty("socket_ground_fx", (0.0, 0.0, 0.0), properties={"semantic": "socket"})
    merge_render_meshes("matron crescendo crown")
    export("matron_crescendo")


def main():
    ashbone_guard()
    bone_stalker()
    hollow_stalker()
    ashen_marksman()
    choir_marksman()
    treasure_imp()
    seal_warden()
    keeper()
    nemesis()
    bell_warden()
    keeper_cleaver_wave()
    keeper_chain_cage()
    matron_soul_lance()
    matron_crescendo()


if __name__ == "__main__":
    main()
