# Test items

Items copied in Path of Exile 2, read by the tests of the parser (`../parser/tests.rs`).

| Files | Source |
|---|---|
| `fr-*.txt`, except `fr-trade-*` | Copied in the French game client by the owner (Ctrl+C, advanced descriptions), October 2026. |
| `en-owner-*.txt`, except `en-owner-trade-*` | Copied in the English game client by the owner (Ctrl+Alt+C), October 2026. |
| `fr-trade-*.txt`, `en-owner-trade-*.txt` | Copied by the owner from the trade of the game (another player's listing), in French and in English: without the `{ … }` headers, their lines marked `(fractured)`, `(mutated)`…, October 2026. |
| `en-*.txt`, except `en-owner-*` and `en-plain-*` | The test items of [Exiled Exchange 2](https://github.com/Kvan7/Exiled-Exchange-2) (`renderer/specs/Parser/items.ts`, MIT license), copied in the English game client. |
| `en-plain-*.txt` | Written by hand: the format without the advanced descriptions, the format of the trade of the game, and cases not met yet (an unidentified jewel with its description, a large stack). |

The texts of the items belong to Grinding Gear Games.
