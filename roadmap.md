# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Technical choices

| Topic | Choice | Reason |
|---|---|---|
| Releases | GitHub Actions workflow triggered by version tags | Builds every system on its own runner and publishes the archives with `gh release`, without extra tooling. |

## Steps

<!--
One line per step, small enough to be reviewed in one pull request, for example:
- [ ] **1. Config and local files**: read and write `config.toml`.
-->
