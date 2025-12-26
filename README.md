# A WIP re-write of [Lapsus](https://github.com/margooey/Lapsus) in rust using various crates for Apple framework bindings.

## Crates used:
- cidre
- core-graphics
- objc2-app-kit
- objc2-core-foundation
- objc2
- macos-multitouch
- log
- env_logger

## Build
```shell
cargo build --release
```

## Debug
```shell
cargo run RUST_LOG=DEBUG
```