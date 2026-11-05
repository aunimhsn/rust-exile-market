#!/usr/bin/env python3
"""Renders the application icon, assets/logo/icon.svg, at each size of the systems.

Writes assets/logo/icon.png (256 px), the icon of the window, and assets/logo/icon.ico (16 to
256 px), the icon of the Windows executable (build.rs). Each size is rendered on its own, its dark
edge kept half a pixel wide, rather than scaled down from the largest one.

Needs rsvg-convert (librsvg) and Pillow. Run it again after a change of the icon:
    python3 scripts/app-icon.py
"""

import io
import re
import subprocess
import sys
from pathlib import Path

from PIL import Image

LOGO = Path(__file__).resolve().parent.parent / "assets" / "logo"
SIZES = [16, 24, 32, 48, 64, 128, 256]
# The width of the view box of the SVG, in its units.
VIEW_BOX = 76
# The blur of the dark edge, in pixels at every size: CSS drop-shadow(0 0 .5px) of the mock-up.
EDGE = 0.25


def render(svg: str, size: int) -> Image.Image:
    blur = EDGE * VIEW_BOX / size
    svg = re.sub(r'stdDeviation="[^"]*"', f'stdDeviation="{blur:.4f}"', svg, count=1)
    png = subprocess.run(
        ["rsvg-convert", "--width", str(size), "--height", str(size), "--format", "png"],
        input=svg.encode(),
        capture_output=True,
        check=True,
    ).stdout
    return Image.open(io.BytesIO(png)).convert("RGBA")


def main() -> int:
    svg = (LOGO / "icon.svg").read_text()
    images = [render(svg, size) for size in SIZES]
    largest = images[-1]
    largest.save(LOGO / "icon.png", optimize=True)
    largest.save(LOGO / "icon.ico", sizes=[(size, size) for size in SIZES], append_images=images[:-1])
    print(f"Wrote {LOGO / 'icon.png'} and {LOGO / 'icon.ico'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
