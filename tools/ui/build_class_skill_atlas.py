#!/usr/bin/env python3
"""Assemble the class skill icon atlas. Requires ImageMagick (magick).

Layout: 7 columns x 3 rows of 256 px cells (1792x768). Row = class, column = slot.

    build_class_skill_atlas.py                 assemble the atlas
    build_class_skill_atlas.py import RAW NAME normalise a raw render into
                                               skill-icon-sources/NAME.png

Four Warden cells are cut from the original six-icon atlas. Their medallions
sit off-centre there, so they are trimmed and re-centred to line up with the
new icons.
"""

from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
SOURCES = Path(__file__).with_name("skill-icon-sources")
LEGACY_ATLAS = ROOT / "assets/images/generated/ui/skill-icons-reliquary.png"
OUTPUT = ROOT / "assets/images/generated/ui/class-skill-icons-reliquary.png"

CELL = 256
# Visible medallion diameter; the original atlas medallions measure 205x209.
MEDALLION = 206
# Raw renders are masked slightly inside their painted rim to drop the
# chroma-key fringe.
MASK_INSET = 2
# Raw renders paint the stone lighter than the original icons; pull the blacks
# down so both generations sit at the same depth.
STONE_LEVEL = "7%,100%,0.92"
LEGACY_COLUMNS = 3

CLASSES = ["warden", "stalker", "embercaller"]
COLUMNS = ["basic", "q_a", "q_b", "e_a", "e_b", "r_a", "r_b"]
# (class, column) -> cell index in the original atlas.
LEGACY_CELLS = {
    ("warden", "basic"): 0,  # strike
    ("warden", "q_a"): 2,  # dash
    ("warden", "e_a"): 3,  # nova
    ("warden", "r_a"): 1,  # rupture
}


def magick(*args: str) -> str:
    result = subprocess.run(
        ["magick", *args], check=True, capture_output=True, text=True
    )
    return result.stdout


def cell_args(class_name: str, column: str) -> list[str]:
    """ImageMagick arguments that yield one centred 256 px cell."""
    legacy = LEGACY_CELLS.get((class_name, column))
    if legacy is not None:
        x = (legacy % LEGACY_COLUMNS) * CELL
        y = (legacy // LEGACY_COLUMNS) * CELL
        # Gravity is a global setting; reset it so the crop offset is absolute.
        source = [str(LEGACY_ATLAS), "-gravity", "NorthWest",
                  "-crop", f"{CELL}x{CELL}+{x}+{y}", "+repage", "-trim", "+repage"]
    else:
        path = SOURCES / f"{class_name}_{column}.png"
        if not path.is_file():
            sys.exit(f"missing icon source: {path.relative_to(ROOT)}")
        source = [str(path)]
    return ["(", *source, "-background", "none", "-gravity", "center",
            "-extent", f"{CELL}x{CELL}", ")"]


def build() -> None:
    args: list[str] = []
    for class_name in CLASSES:
        args.append("(")
        for column in COLUMNS:
            args += cell_args(class_name, column)
        args += ["+append", ")"]
    args += ["-background", "none", "-append", "-strip",
             "-define", "png:exclude-chunks=date,time", f"PNG32:{OUTPUT}"]
    magick(*args)
    size = magick("identify", "-format", "%wx%h", str(OUTPUT))
    expected = f"{CELL * len(COLUMNS)}x{CELL * len(CLASSES)}"
    if size != expected:
        sys.exit(f"atlas is {size}, expected {expected}")
    print(f"{OUTPUT.relative_to(ROOT)} {size}")


def import_raw(raw: Path, name: str) -> None:
    """Scale a square chroma-key render so its medallion matches the atlas."""
    # Flood the flat background from the corners, then measure what is left.
    width, height = (
        int(v) for v in magick("identify", "-format", "%w %h", str(raw)).split()
    )
    corners = [(0, 0), (width - 1, 0), (0, height - 1), (width - 1, height - 1)]
    floods = []
    for x, y in corners:
        floods += ["-draw", f"alpha {x},{y} floodfill"]
    bbox = magick(str(raw), "-alpha", "set", "-fuzz", "30%", "-fill", "none",
                  *floods, "-alpha", "extract", "-threshold", "50%",
                  "-format", "%@", "info:")
    size, left, top = bbox.replace("+", " ").split()
    disc_w, disc_h = (int(v) for v in size.split("x"))
    diameter = (disc_w + disc_h) / 2
    center_x = int(left) + disc_w / 2
    center_y = int(top) + disc_h / 2

    painted = MEDALLION + 2 * MASK_INSET
    side = round(diameter * CELL / painted)
    x0 = round(center_x - side / 2)
    y0 = round(center_y - side / 2)
    middle = (CELL - 1) / 2
    radius = MEDALLION / 2
    SOURCES.mkdir(exist_ok=True)
    output = SOURCES / f"{name}.png"
    magick(
        str(raw), "-alpha", "off", "-background", "black", "-gravity", "NorthWest",
        "-extent", f"{side}x{side}{x0:+d}{y0:+d}",
        "-filter", "Lanczos", "-resize", f"{CELL}x{CELL}!", "-level", STONE_LEVEL,
        "(", "-size", f"{CELL}x{CELL}", "xc:black", "-fill", "white",
        "-draw", f"circle {middle},{middle} {middle},{middle - radius}", ")",
        "-alpha", "off", "-compose", "CopyOpacity", "-composite", "-strip",
        "-define", "png:exclude-chunks=date,time", f"PNG32:{output}",
    )
    print(f"{output.relative_to(ROOT)} disc {disc_w}x{disc_h} in {width}x{height}")


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "import":
        import_raw(Path(sys.argv[2]), sys.argv[3])
    elif len(sys.argv) == 1:
        build()
    else:
        sys.exit(__doc__)
