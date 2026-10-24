"""A rough SVG preview of an Excalidraw file, to check a layout before handing it over.

    python3 preview.py <file.excalidraw> <preview.svg> [x0,y0,x1,y1]

The optional box keeps only the elements inside it (one frame, for a closer look). Render the
SVG to PNG to look at it (headless browser screenshot, `rsvg-convert`, or Pillow + cairosvg).
The fonts and the hand-drawn strokes differ from Excalidraw: the preview checks positions,
sizes and overlaps, not the look.
"""

import html
import json
import sys

FRAME_COLOR = "#228be6"


def preview(document, box=None):
    elements = [e for e in document["elements"] if not e.get("isDeleted")]
    if box:
        x0, y0, x1, y1 = box
        elements = [
            e for e in elements
            if e["x"] < x1 and e["y"] < y1 and e["x"] + e["width"] > x0 and e["y"] + e["height"] > y0
        ]
    if not elements:
        raise SystemExit("nothing to preview")
    left = min(e["x"] for e in elements) - 20
    top = min(e["y"] for e in elements) - 30
    right = max(e["x"] + e["width"] for e in elements) + 20
    bottom = max(e["y"] + e["height"] for e in elements) + 20
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{left} {top} {right - left} {bottom - top}"'
        f' width="{right - left}" height="{bottom - top}" style="background:#fff;font-family:sans-serif">',
        f'<rect x="{left}" y="{top}" width="{right - left}" height="{bottom - top}" fill="#fff"/>',
    ]
    for e in elements:
        out.extend(draw(e))
    out.append("</svg>")
    return "\n".join(out)


def draw(e):
    stroke = e["strokeColor"]
    fill = e["backgroundColor"] if e["backgroundColor"] != "transparent" else "none"
    dash = ' stroke-dasharray="6 4"' if e["strokeStyle"] == "dashed" else ""
    kind = e["type"]
    if kind == "frame":
        name = html.escape(e.get("name") or "")
        return [
            f'<rect x="{e["x"]}" y="{e["y"]}" width="{e["width"]}" height="{e["height"]}"'
            f' fill="none" stroke="{FRAME_COLOR}" stroke-width="2"/>',
            f'<text x="{e["x"]}" y="{e["y"] - 6}" font-size="14" fill="{FRAME_COLOR}">{name}</text>',
        ]
    if kind == "rectangle":
        radius = 4 if e.get("roundness") else 0
        return [
            f'<rect x="{e["x"]}" y="{e["y"]}" width="{e["width"]}" height="{e["height"]}"'
            f' rx="{radius}" fill="{fill}" stroke="{stroke}"{dash}/>'
        ]
    if kind in ("ellipse", "diamond"):
        rx, ry = e["width"] / 2, e["height"] / 2
        return [
            f'<ellipse cx="{e["x"] + rx}" cy="{e["y"] + ry}" rx="{rx}" ry="{ry}"'
            f' fill="{fill}" stroke="{stroke}"{dash}/>'
        ]
    if kind in ("line", "arrow", "freedraw"):
        points = " ".join(f'{e["x"] + p[0]},{e["y"] + p[1]}' for p in e["points"])
        return [f'<polyline points="{points}" fill="none" stroke="{stroke}"{dash}/>']
    if kind == "text":
        anchor = {"right": "end", "center": "middle"}.get(e["textAlign"], "start")
        x = e["x"] + {"end": e["width"], "middle": e["width"] / 2}.get(anchor, 0)
        size = e["fontSize"]
        return [
            f'<text x="{x}" y="{e["y"] + size * (1 + 1.25 * i)}" font-size="{size}"'
            f' fill="{stroke}" text-anchor="{anchor}">{html.escape(line)}</text>'
            for i, line in enumerate(e["text"].split("\n"))
        ]
    return []


if __name__ == "__main__":
    if len(sys.argv) not in (3, 4):
        raise SystemExit(__doc__)
    with open(sys.argv[1], encoding="utf-8") as source:
        document = json.load(source)
    box = tuple(map(float, sys.argv[3].split(","))) if len(sys.argv) == 4 else None
    with open(sys.argv[2], "w", encoding="utf-8") as target:
        target.write(preview(document, box))
