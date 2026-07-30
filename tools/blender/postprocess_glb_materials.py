"""Apply deterministic detail textures and culling policy to runtime GLBs.

Run with Blender 5.1:
    blender --background --factory-startup \
        --python tools/blender/postprocess_glb_materials.py

Optional arguments must follow Blender's ``--`` separator:
    --models-dir PATH   Process another directory (useful for verification).
    --only NAME [...]   Process only the named GLB files.

The exported glTF material graph is the native
``baseColorTexture * baseColorFactor`` representation. The image supplies
grayscale surface variation while the original material color remains the
factor/tint. Pure combat effects are deliberately texture-free.
"""

from __future__ import annotations

import argparse
from functools import cache
import hashlib
import json
import os
from dataclasses import dataclass
from pathlib import Path
import struct
import sys

import bpy


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MODELS_DIR = ROOT / "assets" / "models"
TEXTURE_DIR = ROOT / "assets" / "textures" / "materials"
DETAIL_TEXTURES = {
    "bone": TEXTURE_DIR / "bone-detail.jpg",
    "cloth": TEXTURE_DIR / "cloth-detail.jpg",
    "metal": TEXTURE_DIR / "metal-detail.jpg",
    "stone": TEXTURE_DIR / "stone-detail.jpg",
}
PIPELINE_MARKER_KEY = "bevy_material_pipeline"
DETAIL_CLASS_KEY = "bevy_detail_class"
DETAIL_BY_MATERIAL = {
    "aged bone": "bone",
    "bastion iron": "metal",
    "charcoal cloth": "cloth",
    "choir cloth": "cloth",
    "cold worn steel": "metal",
    "dull relic gold": "metal",
    "hollow bone": "bone",
    "hollow obsidian": "stone",
    "matron silver": "metal",
    "old leather": "cloth",
    "reliquary stone": "stone",
    "roster ash bone": "bone",
    "roster black iron": "metal",
    "roster char cloth": "cloth",
    "roster obsidian": "stone",
    "roster pale steel": "metal",
    "roster reliquary gold": "metal",
    "roster scorched leather": "cloth",
}

# These are deliberately thin, camera-facing, or burst/ring-style effects.
# They stay double-sided and texture-free. Everything else is treated as a
# closed character or prop and gets back-face culling.
VFX_GLBS = frozenset(
    {
        "affix_arcane_aura.glb",
        "affix_blood_aura.glb",
        "affix_ember_aura.glb",
        "affix_frost_aura.glb",
        "affix_ward_aura.glb",
        "arcane_impact.glb",
        "armor_break.glb",
        "blood_spray.glb",
        "bone_fracture_echo.glb",
        "bone_impact.glb",
        "bone_shatter.glb",
        "boss_summon_portal.glb",
        "crit_bone_crown.glb",
        "crit_burst.glb",
        "desecrator_burst.glb",
        "elite_affix_break.glb",
        "ember_impact.glb",
        "ember_vent.glb",
        "execution_burst.glb",
        "frenzy_impact.glb",
        "frost_impact.glb",
        "guard_clash.glb",
        "headshot_burst.glb",
        "hit_bone_lock.glb",
        "hit_bone_rune.glb",
        "hit_spark.glb",
        "holy_impact.glb",
        "keeper_chain_cage.glb",
        "keeper_cleaver_wave.glb",
        "loot_prism.glb",
        "marrow_flash.glb",
        "matron_crescendo.glb",
        "matron_soul_lance.glb",
        "objective_sigil.glb",
        "shadow_burst.glb",
        "shadow_trail.glb",
        "slash_arc.glb",
        "soul_fissure.glb",
        "soul_ward_hit.glb",
        "stagger_burst.glb",
        "vampiric_siphon.glb",
        "void_impact.glb",
    }
)


@dataclass(frozen=True)
class MaterialSpec:
    name: str
    base_color: tuple[float, float, float, float]
    metallic: float
    roughness: float
    emissive: tuple[float, float, float]
    emissive_strength: float

    @property
    def is_emissive(self) -> bool:
        return max(self.emissive) > 1.0e-6 and self.emissive_strength > 0.0


def parse_args() -> argparse.Namespace:
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--models-dir",
        type=Path,
        default=DEFAULT_MODELS_DIR,
        help="directory containing runtime GLBs",
    )
    parser.add_argument(
        "--only",
        nargs="+",
        metavar="GLB",
        help="optional GLB filenames to process",
    )
    return parser.parse_args(args)


@cache
def pipeline_fingerprint() -> str:
    """Hash the implementation and canonical inputs used to make each GLB."""

    digest = hashlib.sha256()
    digest.update(bpy.app.version_string.encode("utf-8"))
    digest.update(Path(__file__).read_bytes())
    for detail, path in sorted(DETAIL_TEXTURES.items()):
        if not path.is_file():
            raise FileNotFoundError(f"missing {detail} detail texture: {path}")
        digest.update(detail.encode("utf-8"))
        digest.update(path.read_bytes())
    return f"glb-detail-v1-{digest.hexdigest()[:20]}"


def read_glb_json(path: Path) -> dict:
    with path.open("rb") as stream:
        header = stream.read(20)
        if len(header) != 20 or header[:4] != b"glTF":
            raise ValueError(f"{path} is not a binary glTF file")
        version, total_length, json_length, chunk_type = struct.unpack(
            "<IIII", header[4:]
        )
        if version != 2 or chunk_type != 0x4E4F534A:
            raise ValueError(f"{path} is not a glTF 2.0 GLB")
        if total_length != path.stat().st_size:
            raise ValueError(f"{path} has an invalid GLB length")
        return json.loads(stream.read(json_length).decode("utf-8").rstrip(" \0"))


def material_specs(document: dict) -> dict[str, MaterialSpec]:
    specs = {}
    for index, source in enumerate(document.get("materials", [])):
        name = source.get("name", f"Material_{index}")
        if name in specs:
            raise ValueError(f"duplicate material name: {name}")
        pbr = source.get("pbrMetallicRoughness", {})
        base_color = tuple(pbr.get("baseColorFactor", (1.0, 1.0, 1.0, 1.0)))
        emissive = tuple(source.get("emissiveFactor", (0.0, 0.0, 0.0)))
        emissive_strength = (
            source.get("extensions", {})
            .get("KHR_materials_emissive_strength", {})
            .get("emissiveStrength", 1.0)
        )
        specs[name] = MaterialSpec(
            name=name,
            base_color=base_color,
            metallic=pbr.get("metallicFactor", 1.0),
            roughness=pbr.get("roughnessFactor", 1.0),
            emissive=emissive,
            emissive_strength=emissive_strength,
        )
    return specs


def detail_class(spec: MaterialSpec) -> str | None:
    """Classify by the stable authored name; emission always wins as VFX."""

    if spec.is_emissive:
        return None
    name = spec.name.casefold()
    try:
        return DETAIL_BY_MATERIAL[name]
    except KeyError as error:
        raise ValueError(
            f"unclassified non-emissive material {spec.name!r}; "
            "add it to DETAIL_BY_MATERIAL"
        ) from error


def reset_and_import(path: Path) -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    result = bpy.ops.import_scene.gltf(filepath=str(path))
    if "FINISHED" not in result:
        raise RuntimeError(f"Blender failed to import {path}")


def clear_material_nodes_and_images() -> None:
    for material in bpy.data.materials:
        material.use_nodes = True
        material.node_tree.nodes.clear()
    for image in list(bpy.data.images):
        bpy.data.images.remove(image)


def load_detail_images(classes: set[str]) -> dict[str, bpy.types.Image]:
    images = {}
    for detail in sorted(classes):
        path = DETAIL_TEXTURES[detail]
        if not path.is_file():
            raise FileNotFoundError(f"missing {detail} detail texture: {path}")
        image = bpy.data.images.load(str(path), check_existing=False)
        image.name = f"{detail}-detail"
        image.colorspace_settings.name = "sRGB"
        images[detail] = image
    return images


def rebuild_material(
    material: bpy.types.Material,
    spec: MaterialSpec,
    image: bpy.types.Image | None,
    double_sided: bool,
) -> None:
    nodes = material.node_tree.nodes
    links = material.node_tree.links

    output = nodes.new("ShaderNodeOutputMaterial")
    output.name = "Material Output"
    output.location = (420.0, 0.0)
    bsdf = nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.name = "Principled BSDF"
    bsdf.location = (80.0, 0.0)
    links.new(bsdf.outputs["BSDF"], output.inputs["Surface"])

    bsdf.inputs["Metallic"].default_value = spec.metallic
    bsdf.inputs["Roughness"].default_value = spec.roughness
    bsdf.inputs["Alpha"].default_value = spec.base_color[3]
    bsdf.inputs["Emission Color"].default_value = (*spec.emissive, 1.0)
    bsdf.inputs["Emission Strength"].default_value = spec.emissive_strength
    material.diffuse_color = spec.base_color
    material.use_backface_culling = not double_sided
    material[PIPELINE_MARKER_KEY] = pipeline_fingerprint()
    material[DETAIL_CLASS_KEY] = image.name.removesuffix("-detail") if image else "none"

    if image is None:
        bsdf.inputs["Base Color"].default_value = spec.base_color
        return

    # This is the same node pattern created by Blender's glTF importer for
    # baseColorTexture * baseColorFactor, and the exporter recognizes it
    # without baking or losing the original tint.
    multiply = nodes.new("ShaderNodeMix")
    multiply.name = "Base Color Factor"
    multiply.label = "Base Color Factor"
    multiply.data_type = "RGBA"
    multiply.blend_type = "MULTIPLY"
    multiply.location = (-160.0, 80.0)
    multiply.inputs["Factor"].default_value = 1.0
    multiply.inputs[7].default_value = spec.base_color

    texture = nodes.new("ShaderNodeTexImage")
    texture.name = f"{image.name} Texture"
    texture.label = "BASE COLOR"
    texture.location = (-460.0, 80.0)
    texture.image = image
    texture.interpolation = "Linear"
    texture.extension = "REPEAT"

    links.new(texture.outputs["Color"], multiply.inputs[6])
    links.new(multiply.outputs[2], bsdf.inputs["Base Color"])


def validate_textured_uvs(classes_by_material: dict[str, str]) -> None:
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        mesh = obj.data
        for slot_index, material in enumerate(mesh.materials):
            if material is None or material.name not in classes_by_material:
                continue
            if not mesh.uv_layers:
                raise ValueError(
                    f"{obj.name}/{material.name} has no UVs for its detail texture"
                )
            uvs = [
                loop.uv
                for polygon in mesh.polygons
                if polygon.material_index == slot_index
                for loop_index in polygon.loop_indices
                for loop in (mesh.uv_layers.active.data[loop_index],)
            ]
            if not uvs:
                continue
            u_span = max(uv.x for uv in uvs) - min(uv.x for uv in uvs)
            v_span = max(uv.y for uv in uvs) - min(uv.y for uv in uvs)
            if u_span <= 1.0e-6 and v_span <= 1.0e-6:
                raise ValueError(
                    f"{obj.name}/{material.name} has degenerate UVs"
                )


def export_glb(path: Path) -> None:
    temporary = path.with_name(f".{path.stem}.postprocess.glb")
    try:
        bpy.ops.object.select_all(action="SELECT")
        result = bpy.ops.export_scene.gltf(
            filepath=str(temporary),
            export_format="GLB",
            use_selection=True,
            export_apply=True,
            export_yup=True,
            export_animations=False,
            export_cameras=False,
            export_lights=False,
            export_extras=True,
            export_image_format="AUTO",
            export_jpeg_quality=90,
        )
        if "FINISHED" not in result:
            raise RuntimeError(f"Blender failed to export {path}")
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def is_current(
    document: dict,
    specs: dict[str, MaterialSpec],
    classes: dict[str, str],
    is_vfx: bool,
) -> bool:
    materials = document.get("materials", [])
    if not materials:
        return False
    marker = pipeline_fingerprint()
    for source in materials:
        name = source.get("name", "")
        extras = source.get("extras", {})
        if extras.get(PIPELINE_MARKER_KEY) != marker:
            return False
        expected_detail = classes.get(name, "none")
        if extras.get(DETAIL_CLASS_KEY) != expected_detail:
            return False
        pbr = source.get("pbrMetallicRoughness", {})
        if ("baseColorTexture" in pbr) != (expected_detail != "none"):
            return False
        if source.get("doubleSided", False) != is_vfx:
            return False

    if classes:
        images = document.get("images", [])
        if not images or any(
            image.get("mimeType") != "image/jpeg" or "bufferView" not in image
            for image in images
        ):
            return False
    return set(specs) == {material.get("name", "") for material in materials}


def process(path: Path) -> tuple[int, int, bool]:
    document = read_glb_json(path)
    specs = material_specs(document)
    is_vfx = path.name in VFX_GLBS
    classes = (
        {}
        if is_vfx
        else {
            name: detail
            for name, spec in specs.items()
            if (detail := detail_class(spec)) is not None
        }
    )
    if is_current(document, specs, classes, is_vfx):
        return len(classes), len(specs), True

    reset_and_import(path)
    imported = {material.name: material for material in bpy.data.materials}
    if imported.keys() != specs.keys():
        raise ValueError(
            f"{path.name} material names changed on import: "
            f"expected {sorted(specs)}, found {sorted(imported)}"
        )

    clear_material_nodes_and_images()
    images = load_detail_images(set(classes.values()))
    for name in sorted(specs):
        detail = classes.get(name)
        rebuild_material(
            imported[name],
            specs[name],
            images.get(detail) if detail is not None else None,
            double_sided=is_vfx,
        )
    validate_textured_uvs(classes)
    export_glb(path)
    return len(classes), len(specs), False


def main() -> None:
    if bpy.app.version[:2] != (5, 1):
        raise RuntimeError(
            f"this deterministic pipeline requires Blender 5.1, found "
            f"{bpy.app.version_string}"
        )
    args = parse_args()
    models_dir = args.models_dir.resolve()
    if not models_dir.is_dir():
        raise FileNotFoundError(f"models directory does not exist: {models_dir}")
    requested = set(args.only or ())
    paths = [
        path
        for path in sorted(models_dir.glob("*.glb"))
        if not requested or path.name in requested
    ]
    missing = requested - {path.name for path in paths}
    if missing:
        raise FileNotFoundError(f"requested GLBs not found: {sorted(missing)}")
    if not paths:
        raise RuntimeError(f"no GLBs found in {models_dir}")

    textured_materials = 0
    material_count = 0
    unchanged_count = 0
    for index, path in enumerate(paths, start=1):
        textured, materials, unchanged = process(path)
        textured_materials += textured
        material_count += materials
        unchanged_count += int(unchanged)
        mode = "VFX/no-detail" if path.name in VFX_GLBS else f"{textured} detailed"
        if unchanged:
            mode += ", up-to-date"
        print(f"[{index:02d}/{len(paths):02d}] {path.name}: {mode}")

    print(
        f"Processed {len(paths)} GLBs, {material_count} materials, "
        f"{textured_materials} detailed materials, {unchanged_count} unchanged."
    )


if __name__ == "__main__":
    main()
