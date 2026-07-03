import math
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "models"
OUT.mkdir(parents=True, exist_ok=True)


def reset():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def material(name, color, metallic=0.0, roughness=0.75, emission=None):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    if emission:
        bsdf.inputs["Emission Color"].default_value = emission
        bsdf.inputs["Emission Strength"].default_value = 0.8
    return mat


STEEL = material("cold worn steel", (0.42, 0.44, 0.48, 1), 0.45, 0.38)
DARK = material("charcoal cloth", (0.035, 0.032, 0.04, 1), 0.0, 0.85)
BONE = material("aged bone", (0.72, 0.68, 0.55, 1), 0.0, 0.82)
EMBER = material("ember glow", (1.0, 0.23, 0.04, 1), 0.0, 0.4, (1.0, 0.18, 0.02, 1))
LEATHER = material("old leather", (0.23, 0.12, 0.055, 1), 0.0, 0.75)
STONE = material("reliquary stone", (0.22, 0.21, 0.24, 1), 0.0, 0.9)
GOLD = material("dull relic gold", (0.92, 0.58, 0.16, 1), 0.65, 0.4)
ARCANE = material("arcane blue glow", (0.18, 0.52, 1.0, 0.72), 0.0, 0.28, (0.12, 0.42, 1.0, 1))
BLOOD = material("dark blood spray", (0.45, 0.02, 0.015, 0.82), 0.0, 0.52, (0.5, 0.02, 0.0, 1))
HOLY = material("reliquary gold glow", (1.0, 0.82, 0.25, 0.72), 0.15, 0.32, (1.0, 0.66, 0.12, 1))
FROST = material("frost ward glow", (0.42, 0.78, 1.0, 0.70), 0.0, 0.30, (0.22, 0.62, 1.0, 1))


def cube(name, loc, scale, mat):
    bpy.ops.mesh.primitive_cube_add(size=1, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(mat)
    return obj


def sphere(name, loc, scale, mat, segments=24):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=segments, ring_count=12, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(mat)
    return obj


def cylinder(name, loc, radius, depth, mat, vertices=16):
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    return obj


def bone_marker(name, loc, radius=0.045):
    obj = sphere(name, loc, (radius, radius, radius), HOLY, 12)
    obj.display_type = "WIRE"
    obj.hide_render = True
    return obj


def hit_contact_zone(name, loc, radius, mat):
    zone = cylinder(f"{name}_contact_zone", loc, radius, 0.018, mat, 24)
    zone.display_type = "WIRE"
    zone.hide_render = True
    return zone


def add_hit_bones(scale=1.0):
    armature = bpy.data.armatures.new("hit_bones")
    armature.display_type = "STICK"
    obj = bpy.data.objects.new("hit_bones", armature)
    bpy.context.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bones = {
        "hit_head": ((0, 0, 1.65 * scale), (0, 0, 1.92 * scale)),
        "hit_chest": ((0, 0, 0.78 * scale), (0, 0, 1.35 * scale)),
        "hit_weapon": ((0.58 * scale, -0.14 * scale, 0.42 * scale), (0.9 * scale, -0.22 * scale, 1.32 * scale)),
        "hit_left": ((-0.42 * scale, 0, 0.72 * scale), (-0.58 * scale, 0, 1.32 * scale)),
        "hit_right": ((0.42 * scale, 0, 0.72 * scale), (0.58 * scale, 0, 1.32 * scale)),
    }
    for name, (head, tail) in bones.items():
        bone = armature.edit_bones.new(name)
        bone.head = head
        bone.tail = tail
    bpy.ops.object.mode_set(mode="OBJECT")
    for name, (_, tail) in bones.items():
        bone_marker(f"{name}_marker", tail)
        hit_contact_zone(name, tail, 0.11 * scale, HOLY)
    return obj


def export(name):
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(OUT / f"{name}.glb"),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
    )


def hero():
    reset()
    cube("tabard", (0, 0, 0.95), (0.62, 0.38, 1.25), DARK)
    sphere("helm", (0, 0, 1.72), (0.34, 0.31, 0.34), STEEL)
    cube("visor", (0, -0.30, 1.73), (0.42, 0.08, 0.11), DARK)
    cube("left pauldron", (-0.44, 0, 1.33), (0.28, 0.45, 0.22), STEEL)
    cube("right pauldron", (0.44, 0, 1.33), (0.28, 0.45, 0.22), STEEL)
    cube("sword blade", (0.78, -0.18, 0.9), (0.09, 0.08, 1.25), STEEL).rotation_euler[1] = math.radians(18)
    cube("sword hilt", (0.62, -0.18, 0.44), (0.32, 0.10, 0.08), GOLD)
    cube("left boot", (-0.18, 0, 0.18), (0.22, 0.28, 0.36), LEATHER)
    cube("right boot", (0.18, 0, 0.18), (0.22, 0.28, 0.36), LEATHER)
    add_hit_bones()
    export("hero")


def skeleton():
    reset()
    sphere("skull", (0, 0, 1.45), (0.28, 0.24, 0.30), BONE)
    cube("rib cage", (0, 0, 0.92), (0.42, 0.24, 0.55), BONE)
    cube("spine cloth", (0, -0.02, 0.55), (0.28, 0.18, 0.55), DARK)
    cube("left arm bone", (-0.42, 0, 0.93), (0.12, 0.12, 0.65), BONE)
    cube("right arm bone", (0.42, 0, 0.93), (0.12, 0.12, 0.65), BONE)
    cube("rust blade", (0.67, -0.08, 0.86), (0.07, 0.06, 0.9), STEEL)
    cube("left leg bone", (-0.15, 0, 0.25), (0.12, 0.12, 0.5), BONE)
    cube("right leg bone", (0.15, 0, 0.25), (0.12, 0.12, 0.5), BONE)
    add_hit_bones(0.86)
    export("skeleton")


def cultist():
    reset()
    cube("hooded robe", (0, 0, 0.82), (0.58, 0.44, 1.35), DARK)
    sphere("hidden face", (0, -0.04, 1.52), (0.28, 0.25, 0.28), DARK)
    cube("ember mask slit", (0, -0.27, 1.54), (0.30, 0.05, 0.06), EMBER)
    cylinder("staff", (0.52, -0.08, 0.9), 0.04, 1.65, LEATHER, 10).rotation_euler[0] = math.radians(8)
    sphere("staff ember", (0.62, -0.10, 1.76), (0.12, 0.12, 0.12), EMBER)
    add_hit_bones(0.92)
    export("cultist")


def butcher():
    reset()
    cube("brute torso", (0, 0, 1.0), (0.95, 0.62, 1.35), LEATHER)
    sphere("iron head", (0, 0, 1.82), (0.42, 0.36, 0.38), STEEL)
    cube("cleaver blade", (0.9, -0.08, 1.04), (0.22, 0.12, 1.1), STEEL).rotation_euler[1] = math.radians(-22)
    cube("left arm", (-0.66, 0, 1.02), (0.28, 0.28, 0.9), LEATHER)
    cube("right arm", (0.66, 0, 1.02), (0.32, 0.32, 1.0), LEATHER)
    cube("left boot", (-0.28, 0, 0.25), (0.28, 0.36, 0.5), DARK)
    cube("right boot", (0.28, 0, 0.25), (0.28, 0.36, 0.5), DARK)
    add_hit_bones(1.18)
    export("butcher")


def effects():
    reset()
    cylinder("slash crescent core", (0, 0, 0.04), 0.82, 0.035, ARCANE, 48)
    bpy.context.object.scale.y = 0.18
    cube("slash leading edge", (0.52, -0.04, 0.05), (0.55, 0.035, 0.045), HOLY).rotation_euler[2] = math.radians(24)
    cube("slash trailing edge", (-0.48, 0.04, 0.04), (0.40, 0.025, 0.035), ARCANE).rotation_euler[2] = math.radians(24)
    export("slash_arc")

    reset()
    sphere("hit spark center", (0, 0, 0.18), (0.13, 0.13, 0.13), HOLY, 16)
    for i in range(8):
        angle = i * math.tau / 8
        shard = cube(
            f"hit spark shard {i}",
            (math.cos(angle) * 0.22, math.sin(angle) * 0.22, 0.18),
            (0.28, 0.035, 0.035),
            HOLY if i % 2 == 0 else EMBER,
        )
        shard.rotation_euler[2] = angle
    export("hit_spark")

    reset()
    sphere("bone shatter core", (0, 0, 0.22), (0.12, 0.10, 0.12), BONE, 12)
    for i in range(11):
        angle = i * math.tau / 11
        shard = cube(
            f"bone shard {i}",
            (math.cos(angle) * 0.24, math.sin(angle) * 0.24, 0.2 + 0.035 * (i % 3)),
            (0.26 + 0.04 * (i % 2), 0.035, 0.055),
            BONE if i % 3 else HOLY,
        )
        shard.rotation_euler[2] = angle
        shard.rotation_euler[1] = math.radians(-18 + i % 5 * 9)
    export("bone_shatter")

    reset()
    cylinder("bone impact hit bone ring", (0, 0, 0.12), 0.36, 0.028, BONE, 36)
    bpy.context.object.scale.y = 0.36
    sphere("bone impact marrow flash", (0, 0, 0.28), (0.11, 0.10, 0.11), HOLY, 12)
    for i in range(16):
        angle = i * math.tau / 16
        shard = cube(
            f"bone impact splinter {i}",
            (
                math.cos(angle) * (0.19 + 0.025 * (i % 4)),
                math.sin(angle) * (0.13 + 0.02 * (i % 3)),
                0.22 + 0.026 * (i % 5),
            ),
            (0.20 + 0.035 * (i % 3), 0.026, 0.044),
            BONE if i % 4 else HOLY,
        )
        shard.rotation_euler[2] = angle
        shard.rotation_euler[1] = math.radians(-26 + i % 7 * 8)
    export("bone_impact")

    reset()
    sphere("blood spray heart", (0, 0, 0.24), (0.12, 0.08, 0.12), BLOOD, 12)
    for i in range(13):
        angle = i * math.tau / 13
        droplet = sphere(
            f"blood droplet {i}",
            (math.cos(angle) * (0.15 + 0.03 * (i % 4)), math.sin(angle) * 0.18, 0.18 + 0.05 * (i % 5)),
            (0.035 + 0.008 * (i % 3), 0.022, 0.035 + 0.006 * (i % 2)),
            BLOOD,
            8,
        )
        droplet.rotation_euler[2] = angle
    export("blood_spray")

    reset()
    cylinder("execution blood seal", (0, 0, 0.08), 0.42, 0.032, BLOOD, 48)
    bpy.context.object.scale.y = 0.42
    sphere("execution burst heart", (0, 0, 0.30), (0.16, 0.11, 0.16), BLOOD, 14)
    for i in range(14):
        angle = i * math.tau / 14
        lash = cube(
            f"execution burst lash {i}",
            (math.cos(angle) * 0.26, math.sin(angle) * 0.20, 0.26 + 0.02 * (i % 4)),
            (0.42 - 0.01 * (i % 5), 0.034, 0.055),
            BLOOD if i % 3 else EMBER,
        )
        lash.rotation_euler[2] = angle
        lash.rotation_euler[1] = math.radians(10 + i % 5 * 4)
    export("execution_burst")

    reset()
    cylinder("arcane impact ring", (0, 0, 0.06), 0.42, 0.035, ARCANE, 48)
    bpy.context.object.scale.y = 0.42
    sphere("arcane impact core", (0, 0, 0.28), (0.13, 0.13, 0.13), ARCANE, 16)
    for i in range(8):
        angle = i * math.tau / 8
        bolt = cube(
            f"arcane fork {i}",
            (math.cos(angle) * 0.28, math.sin(angle) * 0.28, 0.26),
            (0.33, 0.026, 0.045),
            ARCANE if i % 2 == 0 else HOLY,
        )
        bolt.rotation_euler[2] = angle
        bolt.rotation_euler[1] = math.radians(10)
    export("arcane_impact")

    reset()
    sphere("holy impact core", (0, 0, 0.26), (0.16, 0.16, 0.16), HOLY, 18)
    for i in range(12):
        angle = i * math.tau / 12
        ray = cube(
            f"holy impact ray {i}",
            (math.cos(angle) * 0.26, math.sin(angle) * 0.26, 0.27),
            (0.34, 0.032, 0.052),
            HOLY if i % 2 == 0 else GOLD,
        )
        ray.rotation_euler[2] = angle
        ray.rotation_euler[1] = math.radians(14)
    cylinder("holy impact seal", (0, 0, 0.06), 0.34, 0.026, HOLY, 48)
    bpy.context.object.scale.y = 0.34
    export("holy_impact")

    reset()
    cylinder("ember impact scorched hit ring", (0, 0, 0.08), 0.44, 0.04, EMBER, 48)
    bpy.context.object.scale.y = 0.34
    sphere("ember impact furnace heart", (0, 0, 0.28), (0.13, 0.10, 0.13), EMBER, 14)
    for i in range(12):
        angle = i * math.tau / 12
        flame = cube(
            f"ember impact flame lash {i}",
            (math.cos(angle) * 0.26, math.sin(angle) * 0.20, 0.26 + 0.025 * (i % 4)),
            (0.36 + 0.04 * (i % 3), 0.030, 0.065),
            EMBER if i % 3 else HOLY,
        )
        flame.rotation_euler[2] = angle
        flame.rotation_euler[1] = math.radians(12 + i % 4 * 7)
    export("ember_impact")

    reset()
    cylinder("frost impact fractured hit ring", (0, 0, 0.07), 0.40, 0.032, FROST, 48)
    bpy.context.object.scale.y = 0.40
    sphere("frost impact frozen core", (0, 0, 0.27), (0.13, 0.13, 0.13), FROST, 16)
    for i in range(14):
        angle = i * math.tau / 14
        shard = cube(
            f"frost impact ice shard {i}",
            (math.cos(angle) * 0.26, math.sin(angle) * 0.24, 0.25 + 0.025 * (i % 5)),
            (0.08, 0.028, 0.34 + 0.04 * (i % 3)),
            FROST if i % 2 else ARCANE,
        )
        shard.rotation_euler[2] = angle
        shard.rotation_euler[1] = math.radians(20 + i % 5 * 6)
    export("frost_impact")

    reset()
    cylinder("void impact siphon seal", (0, 0, 0.075), 0.42, 0.034, BLOOD, 48)
    bpy.context.object.scale.y = 0.42
    sphere("void impact shadow heart", (0, 0, 0.28), (0.14, 0.11, 0.14), DARK, 14)
    for i in range(9):
        angle = i * math.tau / 9
        veil = cube(
            f"void impact torn veil {i}",
            (math.cos(angle) * 0.24, math.sin(angle) * 0.22, 0.25 + 0.024 * (i % 4)),
            (0.38 - 0.012 * i, 0.032, 0.075),
            BLOOD if i % 2 else ARCANE,
        )
        veil.rotation_euler[2] = angle + math.radians(20)
        veil.rotation_euler[1] = math.radians(-14 + i % 4 * 6)
    export("void_impact")

    reset()
    cylinder("frenzy impact torn hit ring", (0, 0, 0.085), 0.40, 0.030, EMBER, 42)
    bpy.context.object.scale.y = 0.26
    sphere("frenzy impact rage core", (0, 0, 0.26), (0.12, 0.095, 0.12), BLOOD, 12)
    for i in range(10):
        angle = i * math.tau / 10
        slash = cube(
            f"frenzy impact claw slash {i}",
            (
                math.cos(angle) * (0.22 + 0.018 * (i % 3)),
                math.sin(angle) * 0.17,
                0.24 + 0.024 * (i % 4),
            ),
            (0.34 + 0.03 * (i % 2), 0.024, 0.050),
            EMBER if i % 2 == 0 else BLOOD,
        )
        slash.rotation_euler[2] = angle + math.radians(24)
        slash.rotation_euler[1] = math.radians(18 + i % 4 * 5)
    export("frenzy_impact")

    reset()
    cylinder("vampiric siphon blood halo", (0, 0, 0.075), 0.46, 0.030, BLOOD, 48)
    bpy.context.object.scale.y = 0.34
    cylinder("vampiric siphon inner pull", (0, 0, 0.12), 0.24, 0.024, ARCANE, 32)
    bpy.context.object.scale.y = 0.18
    sphere("vampiric siphon heart", (0, 0, 0.30), (0.12, 0.095, 0.12), BLOOD, 14)
    for i in range(9):
        angle = i * math.tau / 9
        stream = cube(
            f"vampiric siphon stream {i}",
            (math.cos(angle) * 0.27, math.sin(angle) * 0.20, 0.24 + 0.018 * (i % 4)),
            (0.30 - 0.010 * (i % 4), 0.025, 0.065),
            BLOOD if i % 3 else ARCANE,
        )
        stream.rotation_euler[2] = angle - math.radians(18)
        stream.rotation_euler[1] = math.radians(-12 + i % 5 * 6)
    export("vampiric_siphon")

    reset()
    cylinder("desecrator burst corrupted ground seal", (0, 0, 0.055), 0.50, 0.034, BLOOD, 52)
    bpy.context.object.scale.y = 0.38
    sphere("desecrator burst bile core", (0, 0, 0.26), (0.14, 0.10, 0.14), DARK, 14)
    for i in range(13):
        angle = i * math.tau / 13
        plume = cube(
            f"desecrator burst corrupted plume {i}",
            (math.cos(angle) * 0.30, math.sin(angle) * 0.22, 0.22 + 0.026 * (i % 5)),
            (0.11, 0.030, 0.34 + 0.035 * (i % 3)),
            BLOOD if i % 2 == 0 else EMBER,
        )
        plume.rotation_euler[2] = angle + math.radians(10)
        plume.rotation_euler[1] = math.radians(20 + i % 5 * 5)
    export("desecrator_burst")

    reset()
    cylinder("guard clash hit bone disc", (0, 0, 0.12), 0.34, 0.026, STEEL, 36)
    bpy.context.object.scale.y = 0.18
    cube("guard clash parry cross", (0, 0, 0.22), (0.60, 0.035, 0.070), HOLY).rotation_euler[2] = math.radians(18)
    cube("guard clash counter cross", (0, 0, 0.22), (0.54, 0.032, 0.060), FROST).rotation_euler[2] = math.radians(-22)
    for i in range(10):
        angle = i * math.tau / 10
        spark = cube(
            f"guard clash spark {i}",
            (math.cos(angle) * 0.24, math.sin(angle) * 0.16, 0.25 + 0.018 * (i % 4)),
            (0.22, 0.024, 0.036),
            HOLY if i % 2 else STEEL,
        )
        spark.rotation_euler[2] = angle
    export("guard_clash")

    reset()
    cylinder("armor break cracked plate", (0, 0, 0.12), 0.38, 0.030, STEEL, 36)
    bpy.context.object.scale.y = 0.22
    for i in range(9):
        angle = i * math.tau / 9
        shard = cube(
            f"armor break shard {i}",
            (math.cos(angle) * 0.22, math.sin(angle) * 0.16, 0.24 + 0.018 * (i % 4)),
            (0.18 + 0.04 * (i % 3), 0.030, 0.060),
            STEEL if i % 2 == 0 else HOLY,
        )
        shard.rotation_euler[2] = angle + math.radians(12)
        shard.rotation_euler[1] = math.radians(-18 + i % 5 * 8)
    cube("armor break bright fracture", (0, 0, 0.25), (0.56, 0.030, 0.048), HOLY).rotation_euler[2] = math.radians(-18)
    export("armor_break")

    reset()
    cylinder("soul ward reflected halo", (0, 0, 0.11), 0.44, 0.030, ARCANE, 48)
    bpy.context.object.scale.y = 0.30
    cylinder("soul ward inner backlash", (0, 0, 0.15), 0.26, 0.026, HOLY, 36)
    bpy.context.object.scale.y = 0.18
    for i in range(8):
        angle = i * math.tau / 8
        lash = cube(
            f"soul ward backlash lash {i}",
            (math.cos(angle) * 0.28, math.sin(angle) * 0.20, 0.27 + 0.020 * (i % 3)),
            (0.34, 0.028, 0.058),
            ARCANE if i % 2 == 0 else BLOOD,
        )
        lash.rotation_euler[2] = angle + math.radians(24)
        lash.rotation_euler[1] = math.radians(12 + i % 4 * 5)
    sphere("soul ward reflected heart", (0, 0, 0.31), (0.11, 0.09, 0.11), ARCANE, 12)
    export("soul_ward_hit")

    reset()
    cylinder("hit bone rune contact ring", (0, 0, 0.055), 0.34, 0.022, HOLY, 40)
    bpy.context.object.scale.y = 0.24
    cylinder("hit bone rune inner mark", (0, 0, 0.078), 0.18, 0.018, ARCANE, 6)
    bpy.context.object.scale.y = 0.14
    for i in range(5):
        angle = i * math.tau / 5
        tick = cube(
            f"hit bone rune tick {i}",
            (math.cos(angle) * 0.25, math.sin(angle) * 0.18, 0.12 + 0.012 * (i % 2)),
            (0.11, 0.022, 0.045),
            HOLY if i % 2 else ARCANE,
        )
        tick.rotation_euler[2] = angle
        tick.rotation_euler[1] = math.radians(10)
    export("hit_bone_rune")

    reset()
    cylinder("hit bone lock targeting ring", (0, 0, 0.050), 0.28, 0.018, ARCANE, 36)
    bpy.context.object.scale.y = 0.19
    cylinder("hit bone lock inner pin", (0, 0, 0.082), 0.075, 0.030, HOLY, 5)
    bpy.context.object.scale.y = 0.055
    for i in range(4):
        angle = i * math.tau / 4
        bracket = cube(
            f"hit bone lock bracket {i}",
            (math.cos(angle) * 0.22, math.sin(angle) * 0.15, 0.12),
            (0.13, 0.020, 0.044),
            HOLY if i % 2 == 0 else ARCANE,
        )
        bracket.rotation_euler[2] = angle
        bracket.rotation_euler[1] = math.radians(8)
    export("hit_bone_lock")

    reset()
    sphere("marrow flash core", (0, 0, 0.20), (0.10, 0.085, 0.10), HOLY, 12)
    for i in range(9):
        angle = i * math.tau / 9
        splinter = cube(
            f"marrow flash splinter {i}",
            (math.cos(angle) * 0.18, math.sin(angle) * 0.15, 0.18 + 0.018 * (i % 4)),
            (0.17 + 0.025 * (i % 3), 0.022, 0.040),
            BONE if i % 2 else HOLY,
        )
        splinter.rotation_euler[2] = angle
        splinter.rotation_euler[1] = math.radians(-16 + i % 5 * 8)
    export("marrow_flash")

    reset()
    cylinder("bone fracture echo cracked halo", (0, 0, 0.090), 0.42, 0.024, BONE, 42)
    bpy.context.object.scale.y = 0.28
    for i in range(10):
        angle = i * math.tau / 10
        crack = cube(
            f"bone fracture echo crack {i}",
            (math.cos(angle) * 0.25, math.sin(angle) * 0.18, 0.18 + 0.017 * (i % 4)),
            (0.20 + 0.026 * (i % 3), 0.021, 0.052),
            BONE if i % 3 else HOLY,
        )
        crack.rotation_euler[2] = angle + math.radians(9)
        crack.rotation_euler[1] = math.radians(-20 + i % 5 * 9)
    sphere("bone fracture echo marrow spark", (0, 0, 0.28), (0.075, 0.062, 0.075), HOLY, 10)
    export("bone_fracture_echo")

    reset()
    cylinder("elite affix break outer seal", (0, 0, 0.070), 0.48, 0.030, EMBER, 48)
    bpy.context.object.scale.y = 0.32
    cylinder("elite affix break inner seal", (0, 0, 0.105), 0.30, 0.025, ARCANE, 36)
    bpy.context.object.scale.y = 0.22
    for i in range(12):
        angle = i * math.tau / 12
        lash = cube(
            f"elite affix break lash {i}",
            (math.cos(angle) * 0.30, math.sin(angle) * 0.22, 0.22 + 0.018 * (i % 4)),
            (0.25 + 0.025 * (i % 4), 0.026, 0.052),
            EMBER if i % 3 else ARCANE,
        )
        lash.rotation_euler[2] = angle + math.radians(16)
        lash.rotation_euler[1] = math.radians(12 + i % 4 * 5)
    sphere("elite affix break heart", (0, 0, 0.30), (0.115, 0.095, 0.115), HOLY, 12)
    export("elite_affix_break")

    reset()
    for i in range(7):
        angle = i * math.tau / 7
        veil = cube(
            f"shadow burst veil {i}",
            (math.cos(angle) * 0.2, math.sin(angle) * 0.2, 0.22 + 0.02 * (i % 3)),
            (0.40 - 0.025 * i, 0.032, 0.08),
            DARK if i % 2 == 0 else ARCANE,
        )
        veil.rotation_euler[2] = angle + math.radians(18)
        veil.rotation_euler[1] = math.radians(-12)
    sphere("shadow burst ember", (0, 0, 0.25), (0.10, 0.10, 0.10), EMBER, 12)
    export("shadow_burst")

    reset()
    cylinder("headshot hit bone crown", (0, 0, 0.14), 0.30, 0.026, HOLY, 36)
    bpy.context.object.scale.y = 0.24
    sphere("headshot burst core", (0, 0, 0.28), (0.15, 0.15, 0.15), EMBER, 16)
    for i in range(12):
        angle = i * math.tau / 12
        ray = cube(
            f"headshot crown ray {i}",
            (math.cos(angle) * 0.27, math.sin(angle) * 0.20, 0.30 + 0.018 * (i % 4)),
            (0.38, 0.034, 0.054),
            HOLY if i % 3 else EMBER,
        )
        ray.rotation_euler[2] = angle
        ray.rotation_euler[1] = math.radians(18 + i % 4 * 6)
    export("headshot_burst")

    reset()
    cylinder("crit bone crown halo", (0, 0, 0.13), 0.36, 0.024, HOLY, 44)
    bpy.context.object.scale.y = 0.27
    cylinder("crit bone crown inner fracture", (0, 0, 0.18), 0.18, 0.020, EMBER, 7)
    bpy.context.object.scale.y = 0.13
    for i in range(14):
        angle = i * math.tau / 14
        spike = cube(
            f"crit bone crown spike {i}",
            (math.cos(angle) * 0.30, math.sin(angle) * 0.22, 0.26 + 0.018 * (i % 4)),
            (0.08, 0.025, 0.28 + 0.030 * (i % 3)),
            HOLY if i % 2 == 0 else EMBER,
        )
        spike.rotation_euler[2] = angle
        spike.rotation_euler[1] = math.radians(20 + i % 5 * 5)
    sphere("crit bone crown impact heart", (0, 0, 0.32), (0.11, 0.10, 0.11), EMBER, 12)
    export("crit_bone_crown")

    reset()
    sphere("crit burst core", (0, 0, 0.2), (0.18, 0.18, 0.18), EMBER, 16)
    for i in range(10):
        angle = i * math.tau / 10
        ray = cube(
            f"crit burst ray {i}",
            (math.cos(angle) * 0.3, math.sin(angle) * 0.3, 0.2),
            (0.46, 0.045, 0.045),
            EMBER if i % 2 == 0 else BLOOD,
        )
        ray.rotation_euler[2] = angle
    export("crit_burst")

    reset()
    cylinder("stagger shock ring", (0, 0, 0.04), 0.68, 0.035, HOLY, 48)
    bpy.context.object.scale.y = 0.68
    for i in range(12):
        angle = i * math.tau / 12
        spike = cube(
            f"stagger shard {i}",
            (math.cos(angle) * 0.46, math.sin(angle) * 0.46, 0.18),
            (0.30, 0.035, 0.09),
            HOLY if i % 2 == 0 else ARCANE,
        )
        spike.rotation_euler[2] = angle
        spike.rotation_euler[1] = math.radians(18)
    sphere("stagger exposed core", (0, 0, 0.32), (0.16, 0.16, 0.16), HOLY, 16)
    export("stagger_burst")

    reset()
    for i in range(5):
        alpha_scale = 1.0 - i * 0.12
        trail = cube(
            f"shadow dash trail {i}",
            (-0.18 * i, 0, 0.34 + 0.02 * i),
            (0.92 - 0.1 * i, 0.045, 0.10),
            ARCANE if i % 2 == 0 else DARK,
        )
        trail.rotation_euler[2] = math.radians(8 + i * 4)
        trail.scale.z *= alpha_scale
    sphere("dash ember tip", (0.46, 0, 0.38), (0.11, 0.11, 0.11), EMBER, 12)
    export("shadow_trail")

    reset()
    cylinder("loot prism beam", (0, 0, 0.5), 0.08, 1.0, HOLY, 6)
    sphere("loot prism heart", (0, 0, 1.05), (0.2, 0.2, 0.2), GOLD, 16)
    for i in range(6):
        angle = i * math.tau / 6
        shard = cube(
            f"loot prism shard {i}",
            (math.cos(angle) * 0.24, math.sin(angle) * 0.24, 0.82),
            (0.08, 0.035, 0.38),
            HOLY if i % 2 == 0 else GOLD,
        )
        shard.rotation_euler[2] = angle
        shard.rotation_euler[1] = math.radians(20)
    export("loot_prism")

    reset()
    cylinder("objective sigil ground ring", (0, 0, 0.035), 0.62, 0.035, ARCANE, 48)
    bpy.context.object.scale.y = 0.62
    for i in range(4):
        angle = i * math.tau / 4
        shard = cube(
            f"objective sigil shard {i}",
            (math.cos(angle) * 0.18, math.sin(angle) * 0.18, 0.34),
            (0.10, 0.035, 0.44),
            HOLY if i % 2 == 0 else ARCANE,
        )
        shard.rotation_euler[2] = angle
        shard.rotation_euler[1] = math.radians(16)
    sphere("objective sigil beacon", (0, 0, 0.68), (0.13, 0.13, 0.13), HOLY, 16)
    export("objective_sigil")

    reset()
    cylinder("ember vent molten ring", (0, 0, 0.04), 0.74, 0.06, EMBER, 48)
    bpy.context.object.scale.y = 0.74
    cylinder("ember vent cracked core", (0, 0, 0.07), 0.42, 0.045, BLOOD, 32)
    for i in range(10):
        angle = i * math.tau / 10
        flame = cube(
            f"ember vent flame tongue {i}",
            (math.cos(angle) * 0.31, math.sin(angle) * 0.31, 0.28 + 0.025 * (i % 3)),
            (0.10, 0.035, 0.46 + 0.08 * (i % 2)),
            EMBER if i % 2 == 0 else HOLY,
        )
        flame.rotation_euler[2] = angle
        flame.rotation_euler[1] = math.radians(14 + i % 3 * 6)
    export("ember_vent")

    reset()
    cylinder("boss portal outer seal", (0, 0, 0.05), 1.25, 0.05, BLOOD, 64)
    bpy.context.object.scale.y = 1.25
    cylinder("boss portal inner seal", (0, 0, 0.09), 0.78, 0.045, EMBER, 64)
    bpy.context.object.scale.y = 0.78
    for i in range(12):
        angle = i * math.tau / 12
        rune = cube(
            f"boss portal rune {i}",
            (math.cos(angle) * 1.02, math.sin(angle) * 1.02, 0.18),
            (0.20, 0.045, 0.08),
            HOLY if i % 3 == 0 else EMBER,
        )
        rune.rotation_euler[2] = angle
    for i in range(7):
        angle = i * math.tau / 7
        flame = cube(
            f"boss portal flame pillar {i}",
            (math.cos(angle) * 0.48, math.sin(angle) * 0.48, 0.48),
            (0.14, 0.05, 0.9),
            EMBER if i % 2 == 0 else BLOOD,
        )
        flame.rotation_euler[2] = angle
        flame.rotation_euler[1] = math.radians(11)
    sphere("boss portal ash heart", (0, 0, 0.72), (0.24, 0.24, 0.24), EMBER, 18)
    export("boss_summon_portal")

    reset()
    cylinder("affix ember aura ground rune", (0, 0, 0.045), 0.74, 0.035, EMBER, 48)
    bpy.context.object.scale.y = 0.74
    for i in range(8):
        angle = i * math.tau / 8
        flame = cube(
            f"affix ember flame marker {i}",
            (math.cos(angle) * 0.54, math.sin(angle) * 0.54, 0.23),
            (0.08, 0.035, 0.36),
            EMBER if i % 2 == 0 else BLOOD,
        )
        flame.rotation_euler[2] = angle
        flame.rotation_euler[1] = math.radians(16)
    export("affix_ember_aura")

    reset()
    cylinder("affix arcane aura rotating sigil", (0, 0, 0.05), 0.78, 0.035, ARCANE, 48)
    bpy.context.object.scale.y = 0.78
    for i in range(6):
        angle = i * math.tau / 6
        shard = cube(
            f"affix arcane shard {i}",
            (math.cos(angle) * 0.46, math.sin(angle) * 0.46, 0.34),
            (0.12, 0.035, 0.42),
            ARCANE if i % 2 == 0 else HOLY,
        )
        shard.rotation_euler[2] = angle
        shard.rotation_euler[1] = math.radians(22)
    sphere("affix arcane core", (0, 0, 0.72), (0.11, 0.11, 0.11), ARCANE, 14)
    export("affix_arcane_aura")

    reset()
    cylinder("affix frost aura control ring", (0, 0, 0.05), 0.72, 0.035, FROST, 48)
    bpy.context.object.scale.y = 0.72
    for i in range(10):
        angle = i * math.tau / 10
        spike = cube(
            f"affix frost spike {i}",
            (math.cos(angle) * 0.48, math.sin(angle) * 0.48, 0.24),
            (0.08, 0.03, 0.30),
            FROST if i % 2 == 0 else ARCANE,
        )
        spike.rotation_euler[2] = angle
        spike.rotation_euler[1] = math.radians(18)
    export("affix_frost_aura")

    reset()
    cylinder("affix blood aura siphon ring", (0, 0, 0.05), 0.70, 0.035, BLOOD, 48)
    bpy.context.object.scale.y = 0.70
    for i in range(7):
        angle = i * math.tau / 7
        droplet = sphere(
            f"affix blood siphon drop {i}",
            (math.cos(angle) * 0.45, math.sin(angle) * 0.45, 0.30 + 0.03 * (i % 3)),
            (0.055, 0.035, 0.070),
            BLOOD if i % 2 == 0 else EMBER,
            10,
        )
        droplet.rotation_euler[2] = angle
    export("affix_blood_aura")

    reset()
    cylinder("affix ward aura shield ring", (0, 0, 0.05), 0.76, 0.035, HOLY, 48)
    bpy.context.object.scale.y = 0.76
    for i in range(8):
        angle = i * math.tau / 8
        plate = cube(
            f"affix ward shield plate {i}",
            (math.cos(angle) * 0.52, math.sin(angle) * 0.52, 0.34),
            (0.16, 0.035, 0.22),
            HOLY if i % 2 == 0 else STEEL,
        )
        plate.rotation_euler[2] = angle
        plate.rotation_euler[1] = math.radians(12)
    export("affix_ward_aura")


def props():
    reset()
    cube("sword blade", (0, 0, 0.72), (0.12, 0.08, 1.18), STEEL)
    cube("sword crossguard", (0, 0, 0.18), (0.5, 0.12, 0.10), GOLD)
    cylinder("sword grip", (0, 0, -0.05), 0.06, 0.38, LEATHER, 12)
    export("sword")

    reset()
    cube("chest body", (0, 0, 0.35), (1.05, 0.64, 0.55), LEATHER)
    cube("chest lid", (0, 0, 0.72), (1.12, 0.70, 0.28), GOLD)
    cube("chest lock", (0, -0.36, 0.48), (0.18, 0.05, 0.18), GOLD)
    export("chest")

    reset()
    cylinder("altar base", (0, 0, 0.18), 0.75, 0.36, STONE, 8)
    cylinder("altar plinth", (0, 0, 0.62), 0.48, 0.72, STONE, 8)
    sphere("ember relic", (0, 0, 1.1), (0.18, 0.18, 0.18), EMBER)
    export("altar")

    reset()
    cube("quartermaster robe", (0, 0, 0.72), (0.52, 0.40, 1.15), DARK)
    sphere("quartermaster helm", (0, 0, 1.42), (0.24, 0.22, 0.24), STEEL, 16)
    cube("quartermaster pack", (0.0, 0.22, 0.86), (0.74, 0.18, 0.56), LEATHER)
    cube("trade ledger", (-0.38, -0.16, 0.78), (0.20, 0.08, 0.28), GOLD)
    cylinder("soul ward lantern", (0.42, -0.12, 0.96), 0.09, 0.32, ARCANE, 12)
    sphere("lantern glow", (0.42, -0.12, 1.18), (0.12, 0.12, 0.12), ARCANE, 12)
    export("quartermaster")

    reset()
    cylinder("fortune shrine base", (0, 0, 0.16), 0.58, 0.32, STONE, 10)
    cylinder("fortune shrine column", (0, 0, 0.68), 0.34, 0.82, GOLD, 10)
    sphere("fortune shrine coin halo", (0, 0, 1.24), (0.24, 0.24, 0.08), HOLY, 20)
    for i in range(6):
        angle = i * math.tau / 6
        coin = cube(
            f"fortune coin {i}",
            (math.cos(angle) * 0.36, math.sin(angle) * 0.36, 1.02),
            (0.08, 0.025, 0.08),
            GOLD,
        )
        coin.rotation_euler[2] = angle
    export("fortune_shrine")

    reset()
    cylinder("storm shrine base", (0, 0, 0.14), 0.54, 0.28, STONE, 10)
    cylinder("storm shrine conduit", (0, 0, 0.72), 0.22, 1.08, ARCANE, 12)
    for i in range(4):
        angle = i * math.tau / 4
        tine = cube(
            f"storm tine {i}",
            (math.cos(angle) * 0.28, math.sin(angle) * 0.28, 1.24),
            (0.08, 0.035, 0.44),
            STEEL if i % 2 == 0 else ARCANE,
        )
        tine.rotation_euler[2] = angle
        tine.rotation_euler[1] = math.radians(18)
    sphere("storm core", (0, 0, 1.34), (0.16, 0.16, 0.16), ARCANE, 16)
    export("storm_shrine")

    reset()
    cylinder("renewal well basin", (0, 0, 0.28), 0.66, 0.34, STONE, 16)
    cylinder("renewal water", (0, 0, 0.49), 0.48, 0.05, ARCANE, 24)
    for i in range(5):
        angle = i * math.tau / 5
        wisp = sphere(
            f"renewal wisp {i}",
            (math.cos(angle) * 0.28, math.sin(angle) * 0.28, 0.72 + 0.04 * (i % 2)),
            (0.055, 0.055, 0.055),
            HOLY if i % 2 == 0 else ARCANE,
            10,
        )
        wisp.rotation_euler[2] = angle
    export("healing_well")

    reset()
    cylinder("cursed shrine base", (0, 0, 0.18), 0.62, 0.36, STONE, 9)
    cube("cursed shrine tablet", (0, 0, 0.74), (0.42, 0.18, 0.9), DARK)
    sphere("cursed blood eye", (0, -0.11, 1.05), (0.15, 0.06, 0.15), BLOOD, 16)
    for i in range(5):
        angle = i * math.tau / 5
        claw = cube(
            f"cursed claw {i}",
            (math.cos(angle) * 0.38, math.sin(angle) * 0.38, 0.42),
            (0.10, 0.035, 0.38),
            BLOOD if i % 2 == 0 else DARK,
        )
        claw.rotation_euler[2] = angle
        claw.rotation_euler[1] = math.radians(24)
    export("cursed_shrine")

    reset()
    cylinder("blood obelisk base", (0, 0, 0.18), 0.48, 0.36, STONE, 8)
    cylinder("blood obelisk spire", (0, 0, 0.9), 0.24, 1.42, BLOOD, 6)
    cube("blood rune cut", (0, -0.22, 1.0), (0.16, 0.035, 0.54), EMBER)
    sphere("blood obelisk crown", (0, 0, 1.68), (0.16, 0.16, 0.16), BLOOD, 12)
    export("blood_obelisk")

    reset()
    cylinder("vault plinth", (0, 0, 0.16), 0.76, 0.32, STONE, 12)
    cube("vault coffer body", (0, 0, 0.58), (1.12, 0.70, 0.58), LEATHER)
    cube("vault gilded lid", (0, 0, 0.98), (1.22, 0.78, 0.26), GOLD)
    cube("vault reliquary lock", (0, -0.40, 0.70), (0.22, 0.055, 0.24), HOLY)
    for i in range(4):
        angle = i * math.tau / 4
        shard = cube(
            f"vault corner ward {i}",
            (math.cos(angle) * 0.54, math.sin(angle) * 0.34, 0.98),
            (0.10, 0.05, 0.22),
            HOLY,
        )
        shard.rotation_euler[2] = angle
    export("reliquary_vault")

    reset()
    cylinder("ember rift ring", (0, 0, 0.62), 0.62, 0.08, EMBER, 48)
    bpy.context.object.rotation_euler[0] = math.radians(90)
    cylinder("ember rift inner tear", (0, 0, 0.62), 0.34, 0.06, BLOOD, 32)
    bpy.context.object.rotation_euler[0] = math.radians(90)
    for i in range(8):
        angle = i * math.tau / 8
        ember = cube(
            f"rift ember {i}",
            (math.cos(angle) * 0.42, 0.0, 0.62 + math.sin(angle) * 0.42),
            (0.08, 0.04, 0.18),
            EMBER if i % 2 == 0 else HOLY,
        )
        ember.rotation_euler[1] = angle
    export("ember_rift_prop")

    reset()
    cylinder("ashen pylon base", (0, 0, 0.18), 0.52, 0.36, STONE, 8)
    cylinder("ashen pylon core", (0, 0, 0.82), 0.24, 1.16, EMBER, 8)
    for i in range(4):
        angle = i * math.tau / 4
        brace = cube(
            f"pylon brace {i}",
            (math.cos(angle) * 0.28, math.sin(angle) * 0.28, 0.82),
            (0.08, 0.045, 1.02),
            STEEL,
        )
        brace.rotation_euler[2] = angle
    sphere("pylon overload heart", (0, 0, 1.48), (0.18, 0.18, 0.18), HOLY, 16)
    export("ashen_pylon")

    reset()
    cube("lore page parchment", (0, 0, 0.06), (0.58, 0.08, 0.38), LEATHER)
    cube("lore page gold seal", (0.20, -0.045, 0.08), (0.12, 0.025, 0.12), GOLD)
    cube("lore page ink line a", (-0.10, -0.047, 0.10), (0.28, 0.02, 0.025), DARK)
    cube("lore page ink line b", (-0.06, -0.047, 0.03), (0.34, 0.02, 0.025), DARK)
    export("lore_page")

    reset()
    cylinder("breakable urn belly", (0, 0, 0.34), 0.30, 0.56, LEATHER, 12)
    cylinder("breakable urn neck", (0, 0, 0.72), 0.18, 0.24, LEATHER, 12)
    cube("breakable urn bone seal", (0, -0.26, 0.48), (0.18, 0.04, 0.16), BONE)
    export("breakable_urn")

    reset()
    cube("breakable coffer body", (0, 0, 0.30), (0.62, 0.46, 0.42), LEATHER)
    cube("breakable coffer lid", (0, 0, 0.58), (0.68, 0.50, 0.18), STONE)
    cube("breakable coffer clasp", (0, -0.27, 0.40), (0.14, 0.035, 0.16), GOLD)
    export("breakable_coffer")


hero()
skeleton()
cultist()
butcher()
props()
effects()
