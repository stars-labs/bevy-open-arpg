"""Regenerate the canonical authored assets; no prototype fallbacks."""
from pathlib import Path
import runpy
import bpy

for script in (
    "generate_hero.py",
    "generate_reliquary_props.py",
    "generate_reliquary_enemies.py",
    "generate_reliquary_effects.py",
    "generate_combat_contacts.py",
    "generate_enemy_roster.py",
    "generate_reliquary_environment.py",
):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    runpy.run_path(str(Path(__file__).with_name(script)), run_name="__main__")
