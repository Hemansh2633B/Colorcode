# Developer Guide

## Common Commands

```sh
cargo fmt
cargo test
cargo run -- encode examples/hello.txt -o examples/hello.c16.png
cargo run -- decode examples/hello.c16.png -o /tmp/hello.out
```

## Adding a Version

Add a new `Version` variant, introduce version-specific dimensions and reserved
areas, then route layout capacity and function-pattern placement through the
version. Keep existing metadata parsing unchanged for compatibility.

## Adding Compression

Add a `Compression` variant, assign a stable metadata ID, and implement
`compress`/`decompress`. Avoid methods that require external dictionaries or
runtime services.

## Testing Notes

Integration tests cover matrix, PNG, JPEG, rotation, compression, and ECC
repair. Additional camera tests should feed real captured images through
`Decoder::decode_path` and keep the fixtures lossless when possible.