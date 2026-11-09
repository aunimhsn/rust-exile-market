# Exile Market

A price checker for Path of Exile 2, free and open source, for Windows and Linux. Copy an item in the game and see its price in a small window over the game.

- **Currencies and uniques**: their price at once, in three currencies, from [poe2scout](https://poe2scout.com).
- **Rare, magic and normal items, and jewels**: their stats and tiers, each stat a filter to tick, then the offers of similar items on the official trade site, and a price estimated from them.
- The game in English or French, and the interface in both.

## Download

Take the archive of your system from the [latest release](https://github.com/aunimhsn/rust-exile-market/releases/latest):

| System | Archive |
|---|---|
| Windows | `exile-market-<version>-x86_64-pc-windows-msvc.zip` |
| Linux | `exile-market-<version>-x86_64-unknown-linux-gnu.tar.gz` |

Unpack it and start `exile-market.exe` (Windows) or `exile-market` (Linux). There is nothing to install. The application tells you when a new version is out, and you download it the same way.

The executable isn't signed for now, so Windows may show an alert at the first launch. If it offers "More info", choose it, then "Run anyway". If it only offers OK, confirm and start the application again.

The checksums of the archives are in `SHA256SUMS.txt`, on the page of the release.

The macOS archives are for players who run the Windows client through a compatibility layer, the game having no macOS client: there, only Ctrl+C copies an item, without the window over the game, and macOS refuses the unsigned executable at first (System Settings, Privacy & Security, Open Anyway). They are less tested.

## Set the display mode of the game

Put the game in **windowed fullscreen** or in a **window**. Exclusive fullscreen isn't supported: Ctrl+D copies nothing, and the D reaches the game.

## Copy an item

Exile Market reads the clipboard while it runs, and keeps only what looks like a Path of Exile 2 item. It doesn't need the focus.

- **Ctrl+D**, with the pointer over an item in the game (Windows and Linux): the item is copied and its price shows in a small window beside the panel of the game. A click outside it closes it. If another application already uses Ctrl+D, the window says so: choose another key in the settings.
- **Ctrl+C**, the copy of the game: the price shows in the main window of Exile Market.

A search on the trade site only runs when you click it.

## Troubleshooting

### Wayland

Exile Market is an X11 application: on a Wayland desktop it runs through XWayland, like the game under Proton, so that it can read what the game copies without having the focus. Most desktops start XWayland on their own, and it was tried on KDE Plasma (Bazzite). If the window can't open, a message says what is missing: install XWayland (the package `xwayland` on most distributions), then start the application again.

### "The clipboard can't be read."

The band goes away by itself as soon as the clipboard can be read again.

- On Linux, the clipboard is read through X11: check that XWayland is running (`echo $DISPLAY` prints something in a terminal of your session), then start the application again.
- On Windows, another application may be holding the clipboard, such as a clipboard manager or a remote desktop: close it.

If it stays, report it with the bug icon of the title bar.

## What it connects to

- **[poe2scout](https://poe2scout.com)**: the prices of the selected league, at launch and every hour.
- **The official trade site** (pathofexile.com): the names of the items and of the stats, once a day; the offers, only when you click a search. Never with an account or a cookie.
- **GitHub**: at launch, to see whether a new version is out. The bug report opens in your browser, once you have seen what it holds; nothing is sent without you.

It doesn't modify the game nor read its memory. Ctrl+D sends the game the Ctrl+C of one copy, once per press.

**The risk for your account**: Exile Market looks up offers on the official trade site through an interface Grinding Gear Games doesn't document, which goes against section 7i of their terms of use, as their [developer documentation](https://www.pathofexile.com/developer/docs) says, and these terms allow them to close an account without notice. Using it is at your own risk.

## Files on your computer

| | Linux | Windows | macOS |
|---|---|---|---|
| Settings | `~/.config/exile-market/` | `%APPDATA%\exile-market\config\` | `~/Library/Application Support/exile-market/` |
| Log | `~/.local/state/exile-market/` | `%LOCALAPPDATA%\exile-market\data\` | `~/Library/Logs/exile-market/` |

The log is `exile-market.log`, and `exile-market.previous.log` for the previous session. The texts copied outside the game are never written to it.

## License

Exile Market is free software, under the GNU General Public License, version 3 or (at your option) any later version: see [LICENSE](LICENSE). It comes with no warranty. Its source code is in [this repository](https://github.com/aunimhsn/rust-exile-market).

Some of what the executable holds isn't under this license, and stays under its own terms ([assets/NOTICE.md](assets/NOTICE.md), [assets/game-data/NOTICE.md](assets/game-data/NOTICE.md)):

- the icons, the names of the items and the game data belong to Grinding Gear Games, and are used in a free, non-commercial tool;
- the fonts Manrope and JetBrains Mono are under the SIL Open Font License 1.1;
- the Lucide icons are under the ISC license;
- the libraries are under their licenses, gathered in `THIRD-PARTY-LICENSES` in each release archive;
- the name "Exile Market" and its logo aren't under the GPL either: a modified version must change both.

## Disclaimer

This product isn't affiliated with or endorsed by Grinding Gear Games in any way.

## Build from source

With Rust 1.95 or later (see `rust-toolchain.toml`):

```sh
cargo build --release
```

The executable is `target/release/exile-market`. See [specs.md](specs.md) for the expected behavior, [roadmap.md](roadmap.md) for the steps, and [docs/releases.md](docs/releases.md) to publish a version.
