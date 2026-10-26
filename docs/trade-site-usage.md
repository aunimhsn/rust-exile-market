# Trade site usage

The trade site API is not in the [Grinding Gear Games developer documentation](https://www.pathofexile.com/developer/docs), and their Terms of Use (section 7i) forbid reverse-engineering undocumented endpoints; the sanction is the closing of the accounts of the developer and of the users. The whole community uses it (Awakened PoE Trade, Exiled Exchange 2) and Grinding Gear Games tolerates it, but it is not allowed. The owner accepted this risk on 2026-10-23, with the guardrails below.

The trade site announces its limits per IP address in the `X-Rate-Limit-*` headers of each response (`hits:period:penalty`, in seconds). Measured on 2026-10-23:

| Policy | Limits |
|---|---|
| Search (`trade-search-request-limit`) | 5 per 10 s (60 s penalty), 15 per 60 s (300 s), 30 per 300 s (1,800 s), 600 per 6 h (3,600 s) |
| Fetch (`trade-fetch-request-limit`), 10 offers per request | 12 per 4 s (10 s penalty), 16 per 12 s (300 s), 50 per 300 s (300 s), 1,000 per 6 h (1,800 s) |

Guardrails:

- A search only runs when the player clicks the search or refresh block: copying an item never searches by itself (sorting the loot copies many items in a row). One click makes one search and one fetch; the double chevron makes one more fetch. Nothing runs in the background.
- After each search, the waiting block lasts at least 10 seconds, and longer until every window of the limits allows a new search. The limits are read from the headers of every response (the IP address may share them with a browser or another tool), never from constants; after a refusal (HTTP 429), the block waits for the `Retry-After` delay, and the application never retries by itself.
- Never send a `POESESSID` session cookie: the searches work without an account. So no direct whisper and no travel to a hideout; **Copy whisper** only copies the whisper text of the listing to the clipboard.
- Send a `User-Agent` with the application name, its version and the repository address.
- The README states which services the application calls, and this risk.
