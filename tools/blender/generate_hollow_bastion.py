import math
from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "models"
OUT.mkdir(parents=True, exist_ok=True)


def reset():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)


def material(name, color, metallic=0.0, roughness=0.7, emission=None, strength=1.0):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    if emission is not None:
        bsdf.inputs["Emission Color"].default_value = emission
        bsdf.inputs["Emission Strength"].default_value = strength
    return mat


OBSIDIAN = material("hollow obsidian", (0.025, 0.035, 0.045, 1), 0.55, 0.32)
IRON = material("bastion iron", (0.18, 0.24, 0.28, 1), 0.72, 0.35)
BONE = material("hollow bone", (0.50, 0.58, 0.57, 1), 0.0, 0.82)
CLOTH = material("choir cloth", (0.08, 0.035, 0.12, 1), 0.0, 0.88)
SOUL = material(
    "hollow soul",
    (0.08, 0.72, 0.78, 1),
    0.05,
    0.24,
    (0.02, 0.85, 1.0, 1),
    2.4,
)
VIOLET = material(
    "choir violet",
    (0.42, 0.08, 0.62, 1),
    0.08,
    0.30,
    (0.52, 0.05, 0.86, 1),
    1.7,
)
SILVER = material("matron silver", (0.48, 0.56, 0.62, 1), 0.82, 0.24)


def cube(name, loc, scale, mat, rotation=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cube_add(size=1, location=loc, rotation=rotation)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(mat)
    return obj


def sphere(name, loc, scale, mat, segments=20):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments,
        ring_count=max(8, segments // 2),
        location=loc,
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(mat)
    return obj


def cylinder(name, loc, radius, depth, mat, vertices=16, rotation=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=depth,
        location=loc,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    return obj


def torus(name, loc, major_radius, minor_radius, mat, rotation=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major_radius,
        minor_radius=minor_radius,
        major_segments=32,
        minor_segments=8,
        location=loc,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    return obj


def add_hit_bones(scale=1.0, weapon_side=1.0):
    armature = bpy.data.armatures.new("hit_bones")
    armature.display_type = "STICK"
    obj = bpy.data.objects.new("hit_bones", armature)
    bpy.context.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bones = {
        "hit_head": ((0, 0, 1.55 * scale), (0, 0, 1.92 * scale)),
        "hit_chest": ((0, 0, 0.72 * scale), (0, 0, 1.38 * scale)),
        "hit_left": ((-0.46 * scale, 0, 0.68 * scale), (-0.62 * scale, 0, 1.30 * scale)),
        "hit_right": ((0.46 * scale, 0, 0.68 * scale), (0.62 * scale, 0, 1.30 * scale)),
        "hit_weapon": (
            (0.58 * weapon_side * scale, -0.10 * scale, 0.50 * scale),
            (0.98 * weapon_side * scale, -0.16 * scale, 1.52 * scale),
        ),
    }
    for name, (head, tail) in bones.items():
        bone = armature.edit_bones.new(name)
        bone.head = head
        bone.tail = tail
    bpy.ops.object.mode_set(mode="OBJECT")


def export(name):
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(OUT / f"{name}.glb"),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_yup=True,
    )


def hollow_thrall():
    reset()
    cube("hollow rib cage", (0, 0, 0.96), (0.50, 0.30, 0.72), BONE)
    sphere("hollow helm", (0, 0, 1.55), (0.31, 0.28, 0.34), IRON)
    cube("empty visor", (0, -0.275, 1.55), (0.36, 0.05, 0.10), SOUL)
    cube("left iron arm", (-0.43, 0, 0.96), (0.15, 0.16, 0.70), IRON)
    cube("right iron arm", (0.43, 0, 0.96), (0.15, 0.16, 0.70), IRON)
    cube("left greave", (-0.16, 0, 0.29), (0.16, 0.20, 0.55), OBSIDIAN)
    cube("right greave", (0.16, 0, 0.29), (0.16, 0.20, 0.55), OBSIDIAN)
    cube(
        "bastion blade",
        (0.70, -0.08, 0.96),
        (0.10, 0.07, 1.15),
        SILVER,
        (0.0, math.radians(-14), 0.0),
    )
    torus("soul cage", (0, -0.17, 1.02), 0.22, 0.035, SOUL, (math.radians(90), 0, 0))
    sphere("soul heart", (0, -0.20, 1.02), (0.09, 0.05, 0.09), SOUL, 14)
    add_hit_bones(0.92)
    export("hollow_thrall")


def choir_acolyte():
    reset()
    cube("split choir robe", (0, 0, 0.80), (0.62, 0.46, 1.42), CLOTH)
    sphere("faceless cowl", (0, 0, 1.55), (0.31, 0.28, 0.32), OBSIDIAN)
    torus("choir halo", (0, 0.04, 1.82), 0.27, 0.035, VIOLET, (math.radians(90), 0, 0))
    cube("soul mouth", (0, -0.285, 1.53), (0.30, 0.045, 0.065), SOUL)
    cylinder("bell staff", (0.55, -0.03, 0.91), 0.045, 1.72, IRON, 10)
    sphere("bell focus", (0.55, -0.03, 1.78), (0.14, 0.14, 0.14), VIOLET, 14)
    for index in range(3):
        torus(
            f"voice ring {index}",
            (0, -0.02, 1.12 + index * 0.18),
            0.26 + index * 0.04,
            0.018,
            SOUL if index % 2 == 0 else VIOLET,
            (math.radians(90), 0, 0),
        )
    add_hit_bones(0.94)
    export("choir_acolyte")


def hollow_reaver():
    reset()
    cube("reaver cuirass", (0, 0, 1.02), (1.02, 0.66, 1.42), OBSIDIAN)
    sphere("reaver mask", (0, 0, 1.82), (0.43, 0.37, 0.42), IRON)
    cube("soul visor", (0, -0.36, 1.83), (0.48, 0.055, 0.12), SOUL)
    cube("left plated arm", (-0.68, 0, 1.04), (0.32, 0.34, 0.96), IRON)
    cube("right plated arm", (0.68, 0, 1.04), (0.34, 0.36, 1.02), IRON)
    cube("left bastion boot", (-0.29, 0, 0.25), (0.31, 0.39, 0.52), OBSIDIAN)
    cube("right bastion boot", (0.29, 0, 0.25), (0.31, 0.39, 0.52), OBSIDIAN)
    cube(
        "crescent cleaver",
        (0.98, -0.08, 1.02),
        (0.28, 0.14, 1.26),
        SILVER,
        (0.0, math.radians(-24), 0.0),
    )
    torus("bound soul ring", (0, -0.34, 1.06), 0.30, 0.045, VIOLET, (math.radians(90), 0, 0))
    add_hit_bones(1.18)
    export("hollow_reaver")


def hollow_matron():
    reset()
    cube("matron armored gown", (0, 0, 1.12), (1.10, 0.72, 1.72), OBSIDIAN)
    cube("matron breastplate", (0, -0.10, 1.34), (0.88, 0.54, 0.78), SILVER)
    sphere("matron death mask", (0, 0, 2.05), (0.44, 0.38, 0.46), IRON)
    cube("matron soul eyes", (0, -0.37, 2.08), (0.42, 0.055, 0.085), SOUL)
    for index in range(7):
        angle = -0.9 + index * 0.3
        cube(
            f"hollow crown tine {index}",
            (math.sin(angle) * 0.38, 0.02, 2.48 + math.cos(angle) * 0.10),
            (0.075, 0.10, 0.54 + 0.05 * (index % 2)),
            VIOLET if index % 2 else SILVER,
            (0.0, -angle * 0.25, -angle * 0.45),
        )
    cube("left matron arm", (-0.74, 0, 1.34), (0.32, 0.34, 1.12), SILVER)
    cube("right matron arm", (0.74, 0, 1.34), (0.34, 0.36, 1.18), SILVER)
    cube(
        "hollow greatblade",
        (1.08, -0.10, 1.34),
        (0.28, 0.13, 1.72),
        SOUL,
        (0.0, math.radians(-20), 0.0),
    )
    torus("matron soul halo", (0, 0.10, 2.10), 0.60, 0.055, VIOLET, (math.radians(90), 0, 0))
    sphere("matron exposed soul", (0, -0.38, 1.36), (0.18, 0.08, 0.18), SOUL, 18)
    add_hit_bones(1.34)
    export("hollow_matron")


def soul_font():
    reset()
    cylinder("font obsidian plinth", (0, 0, 0.18), 0.76, 0.36, OBSIDIAN, 12)
    cylinder("font silver basin", (0, 0, 0.58), 0.58, 0.40, SILVER, 16)
    cylinder("font soul surface", (0, 0, 0.82), 0.48, 0.055, SOUL, 32)
    for index in range(6):
        angle = index * math.tau / 6
        cube(
            f"font claw {index}",
            (math.cos(angle) * 0.56, math.sin(angle) * 0.56, 0.90),
            (0.10, 0.10, 0.72),
            IRON,
            (math.radians(18), math.sin(angle) * 0.25, angle),
        )
        sphere(
            f"font wisp {index}",
            (
                math.cos(angle) * 0.30,
                math.sin(angle) * 0.30,
                1.06 + 0.10 * (index % 3),
            ),
            (0.065, 0.065, 0.11),
            SOUL if index % 2 == 0 else VIOLET,
            10,
        )
    torus("font binding seal", (0, 0, 0.86), 0.67, 0.035, VIOLET)
    export("soul_font")


def soul_fissure():
    reset()
    torus("fissure outer scar", (0, 0, 0.06), 0.52, 0.055, OBSIDIAN)
    torus("fissure soul edge", (0, 0, 0.09), 0.38, 0.038, SOUL)
    for index in range(9):
        angle = index * math.tau / 9
        cube(
            f"fissure shard {index}",
            (
                math.cos(angle) * (0.25 + 0.07 * (index % 2)),
                math.sin(angle) * (0.25 + 0.05 * (index % 3)),
                0.20 + 0.04 * (index % 4),
            ),
            (0.08, 0.06, 0.38 + 0.06 * (index % 3)),
            VIOLET if index % 3 == 0 else SOUL,
            (math.radians(14 + index * 3), 0.0, angle),
        )
    sphere("fissure heart", (0, 0, 0.28), (0.16, 0.16, 0.22), SOUL, 16)
    export("soul_fissure")


if __name__ == "__main__":
    hollow_thrall()
    choir_acolyte()
    hollow_reaver()
    hollow_matron()
    soul_font()
    soul_fissure()
