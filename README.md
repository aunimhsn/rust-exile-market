# rust-exile-market

Price checker for Path of Exile 2 items, written in Rust.

## Status

Early development: the expected behavior is described in [specs.md](specs.md), and the implementation steps in [roadmap.md](roadmap.md).

## Disclaimer

This product isn't affiliated with or endorsed by Grinding Gear Games in any way.

## Requirements

- Rust 1.93 or later (stable toolchain, see `rust-toolchain.toml`)

## Build

```sh
cargo build --release
```

## Development

```sh
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Release

1. Update `version` in `Cargo.toml` (then run `cargo build` to update `Cargo.lock`) and merge it into `main`.
2. Tag the merged commit with the same version and push the tag:

   ```sh
   git switch main && git pull
   git tag v0.1.0
   git push origin v0.1.0
   ```

The [Release workflow](.github/workflows/release.yml) builds the archives for Windows, macOS and Linux and publishes them as a GitHub release. It fails if the tag does not match the version of `Cargo.toml`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

The icons in [assets/](assets/) belong to Grinding Gear Games and are not covered by this license: see [assets/NOTICE.md](assets/NOTICE.md).
