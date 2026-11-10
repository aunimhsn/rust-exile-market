# poe2scout usage

poe2scout limits its API to 60 requests per minute per IP address, in a sliding window, without a queue ([source](https://github.com/poe2scout/poe2scout/blob/main/net/Poe2scout.Api/Ioc/ServiceCollectionExtensions.cs)). The limit applies to each player's machine, not to the application as a whole. A full refresh is about 26 requests (the list of the categories, the rates of the currencies, then the 24 categories of 2026-10-25, 250 items per page), once an hour; copying an item makes no request. The application still follows these rules:

- Space the requests of a refresh: one per second at most (1.1 seconds in the code, so that the refreshes of several leagues in a row never fit 61 requests in a minute).
- When the API refuses a request, wait and retry at the next attempt, without insisting: a refresh stops at its first failed request. Without an answer (no network), the next attempt comes a minute later, with one request. When the API answers with an error, or with something the application can't read (after a change of the game, say), the same refresh would fail the same way after as many requests: the next attempt waits 1, 2, 4… minutes, an hour at most (`Backoff` in `src/http.rs`, network review of 2026-11-10).
- Keep the loaded price histories in memory: copying an item again makes no new request.
- Send a `User-Agent` with the application name, its version and the repository address.
- Tell the poe2scout maintainers about the application, on their Discord (linked from [poe2scout.com](https://poe2scout.com)), when the releases reach 100 downloads in total (`gh release view` gives the count of each archive).
