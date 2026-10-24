---
name: excalidraw-wireframes
description: Draw editable Excalidraw wireframes of a user interface (grayscale, one group per block, one frame per screen, size or state) in docs/wireframes/, and iterate on them with the owner, who edits the file in VS Code or on excalidraw.com. Use it before any HTML mock-up or interface code, when the owner asks for wireframes, a layout, a sketch of the screens, or to rework the layout of an existing wireframe.
---

# Excalidraw wireframes

Goal: agree with the owner on the **layout** of the interface, without colors or code, in a file they can edit themselves. A coded mock-up made too early is generic and never exactly what the owner wants; a wireframe they draw on becomes their own.

Only the layout is decided here: the style (colors, fonts) comes after, in the HTML mock-up (`html-mockups` skill). The agreed layout ends up in `specs.md`.

## 1. Before drawing

- Read `specs.md` (what the interface shows) and ask the owner what is unclear, a few questions at a time: the screens or windows, the sizes (window width, phone, desktop), what matters most at a glance, the panels that open over the content.
- For a responsive interface, start with the **narrowest layout** (mobile first): it forces to choose the essential, and the wider layouts only rearrange the same blocks.

## 2. Generate the file

Write a Python generator in the scratchpad that uses `scripts/excalidraw.py` (read its docstring), and save to `docs/wireframes/<name>.excalidraw`:

- **Grayscale only** (`INK`, `WEAK`, `FILL`), the hand-drawn font: it reads as a draft, so the discussion stays on the layout.
- **One frame per screen, window size or state**, named after it ("Narrow · 420 px", "Settings open", "Error · offline").
- **One group per block** (header, item, list, status bar…), so that the owner moves a block in one click.
- **Sample data close to real ones**: real names, plausible numbers, long and short texts. Lorem ipsum hides the real constraints (a long name, a large number).
- A title and a short legend at the top of the canvas: what the file shows, the breakpoints, "Layout only, no colors".
- Keep the generator: the next iteration reuses it.

## 3. Check before handing over

Render a preview with `scripts/preview.py` (SVG, then a PNG to look at it) and check overlaps, texts that overflow their block, alignment, and that every block is grouped. The preview checks positions, not the look.

## 4. Hand it over

Tell the owner how to open it:

- In VS Code with the **Excalidraw** extension (`pomdtr.excalidraw-editor`), or on excalidraw.com (**Open**, then **Save to…**).
- Double-click a block to edit one of its elements; **Ctrl+Shift+G** ungroups it.
- The VS Code extension **neither autosaves nor reloads** a file changed on disk: press **Ctrl+S** before asking Claude Code to read the file; before Claude Code rewrites it, save and close the tab, then reopen it afterwards, and never save an old tab over the new file.

## 5. Iterate

The best results came from this loop: **the owner draws, Claude Code rearranges and completes**, the owner draws the next version, and so on.

1. Read the owner's version (load it with `Drawing.load`), and list what changed before touching anything.
2. **Commit the owner's version first** (on the wireframes branch), so that every iteration stays in the history.
3. Rearrange and complete it with a new id prefix per iteration (`v2`, `v3`…): the owner's elements are kept, the new ones never collide with them. Replace a whole frame (`remove_frames`) rather than patching elements one by one.
4. One question per iteration: layout first; content and wording later.
5. Once the main frames are agreed, draw the rest under them: the other kinds of content, the empty, loading and waiting states, the errors, the other language, the panels that open over the content.

## 6. Finish

- Write the agreed layout in `specs.md` (window layout section: blocks, order, what each shows, breakpoints).
- Then build the HTML mock-up from the frames (`html-mockups` skill). From then on, the mock-up is the reference; the wireframe only fixed the first layout, and `specs.md` says so.
