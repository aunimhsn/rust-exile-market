"""Write Excalidraw wireframes from Python: grayscale, hand-drawn, one group per block.

Import it from a generator script kept in the scratchpad:

    import sys
    sys.path.insert(0, "<skill directory>/scripts")
    from excalidraw import Drawing, WEAK, FILL

    drawing = Drawing(prefix="v1")
    frame = drawing.frame(0, 0, 420, 720, "Narrow · 420 px")
    header = drawing.group()
    drawing.rect(0, 0, 420, 48, group=header, frame=frame, rounded=False)
    drawing.text(14, 14, "My application", 18, group=header, frame=frame)
    drawing.save("docs/wireframes/window-layouts.excalidraw")

To change a file the owner edited, load it first: their elements are kept, and the new
elements get ids with another prefix, so that they never collide with the owner's.

    drawing = Drawing.load("docs/wireframes/window-layouts.excalidraw", prefix="v2")
    drawing.remove_frames(lambda name: name.startswith("Narrow"))
"""

import json
import random
from pathlib import Path

INK = "#1e1e1e"
WEAK = "#868e96"
FILL = "#e9ecef"
DARK_FILL = "#ced4da"
# Virgil, the hand-drawn font: the file reads as a draft, not as a design.
HAND_DRAWN = 1
# A fixed date: the generated file doesn't change when nothing changed.
UPDATED = 1761231600000


class Drawing:
    """The elements of an Excalidraw file, with helpers to add the usual shapes."""

    def __init__(self, prefix="w", seed=7, elements=None, app_state=None):
        self.prefix = prefix
        self.elements = list(elements or [])
        self.app_state = app_state or {"viewBackgroundColor": "#ffffff", "gridSize": 20}
        self._random = random.Random(seed)
        self._counter = 0

    @classmethod
    def load(cls, path, prefix):
        """The drawing of an existing file, without its deleted elements."""
        document = json.loads(Path(path).read_text(encoding="utf-8"))
        elements = [e for e in document["elements"] if not e.get("isDeleted")]
        return cls(prefix=prefix, elements=elements, app_state=document.get("appState"))

    def save(self, path):
        document = {
            "type": "excalidraw",
            "version": 2,
            "source": "https://excalidraw.com",
            "elements": self.elements,
            "appState": self.app_state,
            "files": {},
        }
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        Path(path).write_text(json.dumps(document, ensure_ascii=False, indent=1), encoding="utf-8")
        return len(self.elements)

    # Structure

    def new_id(self, kind):
        self._counter += 1
        return f"{self.prefix}-{kind}-{self._counter}"

    def group(self):
        """A new group id: give it to every element of a block, so that the block moves in
        one click (double-click to edit one of its elements, Ctrl+Shift+G to ungroup)."""
        return self.new_id("group")

    def frame(self, x, y, width, height, name):
        """A named frame, one per screen, window size or state; returns its id."""
        return self._add("frame", x, y, width, height, None, None, name=name)["id"]

    def frames(self):
        return {e["id"]: e for e in self.elements if e["type"] == "frame"}

    def remove_frames(self, matches):
        """Removes the frames whose name `matches` accepts, with everything inside them."""
        removed = {i for i, f in self.frames().items() if matches(f["name"])}
        self.elements = [
            e for e in self.elements if e["id"] not in removed and e.get("frameId") not in removed
        ]

    def send_to_back(self, start):
        """Moves the elements added since index `start` behind all the others of the drawing,
        e.g. a band background drawn after its content."""
        added = self.elements[start:]
        del self.elements[start:]
        self.elements[0:0] = added

    # Shapes

    def rect(self, x, y, width, height, group=None, frame=None, fill=None, dashed=False,
             rounded=True, stroke=INK):
        return self._add(
            "rectangle", x, y, width, height, group, frame,
            backgroundColor=fill or "transparent",
            strokeStyle="dashed" if dashed else "solid",
            roundness={"type": 3} if rounded else None,
            strokeColor=stroke,
        )

    def ellipse(self, x, y, width, height, group=None, frame=None, fill=FILL):
        return self._add("ellipse", x, y, width, height, group, frame, backgroundColor=fill)

    def text(self, x, y, value, size=16, group=None, frame=None, color=INK, align="left",
             width=None):
        """A text; `align="right"` puts its right edge at `x`. The width is an estimate:
        check the preview."""
        lines = value.split("\n")
        width = width or max(len(line) for line in lines) * size * 0.55
        height = len(lines) * size * 1.25
        if align == "right":
            x -= width
        elif align == "center":
            x -= width / 2
        return self._add(
            "text", x, y, width, height, group, frame,
            strokeColor=color,
            text=value,
            originalText=value,
            fontSize=size,
            fontFamily=HAND_DRAWN,
            textAlign=align,
            verticalAlign="top",
            containerId=None,
            lineHeight=1.25,
            autoResize=True,
        )

    def line(self, points, x, y, group=None, frame=None, color=INK, dashed=False, arrow=False):
        """A line through `points`, relative to (`x`, `y`); with an arrowhead at the end when
        `arrow` is true."""
        xs = [p[0] for p in points]
        ys = [p[1] for p in points]
        return self._add(
            "arrow" if arrow else "line", x, y, max(xs) - min(xs), max(ys) - min(ys), group, frame,
            points=[list(p) for p in points],
            strokeColor=color,
            strokeStyle="dashed" if dashed else "solid",
            lastCommittedPoint=None,
            startBinding=None,
            endBinding=None,
            startArrowhead=None,
            endArrowhead="arrow" if arrow else None,
        )

    def _add(self, kind, x, y, width, height, group, frame, **extra):
        element = {
            "id": self.new_id(kind),
            "type": kind,
            "x": x,
            "y": y,
            "width": width,
            "height": height,
            "angle": 0,
            "strokeColor": INK,
            "backgroundColor": "transparent",
            "fillStyle": "solid",
            "strokeWidth": 1,
            "strokeStyle": "solid",
            "roughness": 1,
            "opacity": 100,
            "groupIds": [group] if group else [],
            "frameId": frame,
            "roundness": None,
            "seed": self._random.randint(1, 2**31),
            "version": 1,
            "versionNonce": self._random.randint(1, 2**31),
            "isDeleted": False,
            "boundElements": None,
            "updated": UPDATED,
            "link": None,
            "locked": False,
        }
        element.update(extra)
        self.elements.append(element)
        return element
