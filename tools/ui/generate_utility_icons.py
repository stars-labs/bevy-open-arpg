#!/usr/bin/env python3
"""Render the authored utility medallions. Requires ImageMagick (magick)."""

from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).with_name("utility-icons-reliquary.svg")
OUTPUT = ROOT / "assets/images/generated/ui/utility-icons-reliquary.png"

subprocess.run(
    ["magick", "-background", "none", "-density", "384", str(SOURCE),
     "-resize", "1024x256", f"PNG32:{OUTPUT}"],
    check=True,
)
print(OUTPUT.relative_to(ROOT))
