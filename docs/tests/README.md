# Test guides

The step-by-step guides of the tests by hand, in the game, kept to be run again: before a release, after Path of Exile 2 1.0, after a change of the code they test. Each one is a blank template: its results are given in the conversation or in the pull request, never written here.

- `linux/`: the tests of the owner, on Linux (Bazzite KDE).
- `windows/`: the tests of the Windows tester, who downloads the archive of the release workflow of the pull request. Not at hand step after step: they run every Windows guide at once before the first release (step 17 of `roadmap.md`).

They may be written in French, the language of the testers (an exception to the English of the project, see `CLAUDE.md`).

## Linux

- [Window over the game](linux/overlay-window.md) (14b1; a single screen since the fix after 17a)
- [Keyboard in the window over the game](linux/overlay-keyboard.md) (14b2)
- [Filters of the last items remembered](linux/remembered-filters.md) (#155)
- [Prices in other currencies](linux/prices-other-currencies.md) (#156)
- [Ctrl+D in the game only](linux/hotkey-game-only.md) (14c1)
- [Hotkey turned on or off, its key taken](linux/hotkey-setting.md) (14c3a)
- [Keys of the hotkey from the settings file](linux/hotkey-keys.md) (14c3b1)
- [Key of the hotkey chosen from a list](linux/hotkey-list.md) (14c3b2)
- [Update notice](linux/update-notice.md) (15, once a first release is out)
- [Bug report](linux/bug-report.md) (16b)
- [Executable: its name, its icon, a window that can't open](linux/executable.md) (17a)
- [Another desktop than KDE: the keyboard and the pointer, only checked on KWin](linux/other-desktops.md) (17e3b2, run in 17f)

## Windows

- [Window over the game](windows/overlay-window.md) (14b3a)
- [Keyboard in the window over the game](windows/overlay-keyboard.md) (14b3b)
- [Window over the game on a second screen](windows/second-screen.md) (left to step 21)
- [Ctrl+D in the game only](windows/hotkey-game-only.md) (14c2)
- [Hotkey turned on or off, its key taken](windows/hotkey-setting.md) (14c3a)
- [Keys of the hotkey from the settings file](windows/hotkey-keys.md) (14c3b1)
- [Key of the hotkey chosen from a list](windows/hotkey-list.md) (14c3b2)
- [Bug report](windows/bug-report.md) (16b)
- [Executable: its name and its icon](windows/executable.md) (17a)
- [A copy refused by the system](windows/copy-failure.md)
- [Three cases never tried on Windows](windows/unchecked-assumptions.md) (17e3b2)
