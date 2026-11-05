# Windows test: the window over the game on a second screen

A hand test left to step 21 (Polish) of [roadmap.md](../roadmap.md), from the second code review of step 14b3a (2026-11-02). It needs a Windows player that the owner has to find: neither the owner nor Claude Code has Windows.

## Why

On Windows, each screen has its own scale (100 %, 125 %, 150 %…). The window over the game is placed with the scale of its own screen as egui read it at the previous frame, the one of the main window until the window exists (`show_overlay` in `src/app.rs`). When the main window of Exile Market is on a screen of another scale than the game, the window over the game may be placed wrong at its first opening: over the inventory, or with a jump, for one frame or more.

Two fixes are possible, and only this test tells which is right:

- **the scale of the game read at the hotkey** (`GetDpiForWindow` of `winsafe`, in `src/hotkey/windows.rs`): right only if a hidden window moved to another screen takes the scale of that screen at once, since egui-winit converts the position with the scale of the screen the window is on at that moment;
- **the window kept hidden one more frame** when the scale it reads changes: always right, at the cost of a slightly later opening on that screen.

## Who

A Windows player of Path of Exile 2 with **two screens of different scales**: most often a laptop (150 %) plugged into a screen (100 %). Check it in Settings > System > Display: select each screen, and read "Scale" under "Scale & layout". Two screens at the same scale don't show the case: the test then has nothing to see.

## Before

1. Close Exiled Exchange 2 if it runs: it takes Ctrl+D too.
2. Download the Windows archive of the latest Release workflow (`exile-market-test-x86_64-pc-windows-msvc.zip`; a GitHub account is needed to download it, or the owner sends it), or the release once it exists. Unzip it and start `exile-market.exe`. If Windows SmartScreen blocks it: "More info", then "Run anyway".
3. Write down the scale of each screen.
4. Put the main window of Exile Market on **screen A**.
5. Start Path of Exile 2 on **screen B**, in **windowed fullscreen**, not as administrator, and go to the hideout.

## The test

1. Open the inventory, and press **Ctrl+D** on an item. Watch the window that opens, closely, at the very moment it appears.
2. Click a checkbox in that window.
3. Click in the game, outside the window, to close it.
4. Repeat steps 1 to 3 on two other items, then once in the stash (the window then opens right of the stash).
5. Move the main window of Exile Market to screen B (with the game), and repeat steps 1 to 3 once.

## What to report

| Question | Answer |
|---|---|
| Scale of screen A (main window) and of screen B (game) |  |
| At the first opening, does the window appear at once right beside the inventory, at the right size? |  |
| Does it jump from one place to another, or cover the inventory for a moment? Where, how long (a flash, or it stays)? |  |
| Does it stay wrong (wrong place, wrong size, cut, partly on the other screen)? |  |
| Does a click in it close it (it shouldn't)? |  |
| At the next openings, same thing or right at once? |  |
| With the main window on screen B (step 5), right at once? |  |

A screen recording of the opening helps most (Windows+Alt+R records with the Xbox Game Bar), and the log: `%LOCALAPPDATA%\exile-market\data\exile-market.log`.

## Then

- **Right at once, everywhere**: nothing to fix; write it in `docs/technical-choices.md` (Window over the game, Windows) and close the point of step 21.
- **A jump or an overlap for a moment at the first opening only**: the delay of one frame; keep the window hidden one more frame when the scale it reads changes, or use the scale of the game if the report shows the window takes the scale of its screen at once while hidden.
- **Wrong for good**: the conversion of the position is wrong in another way; compare the places in the log and the recording with `Target::place` (`src/overlay.rs`), then fix and ask the player to test again.
