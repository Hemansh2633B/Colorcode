# ColorCode

ColorCode is a Rust implementation of a custom 16-color matrix code. It is not
a QR-code implementation and does not reuse the QR layout. Version 1 uses a
32x32 matrix where every module stores one 4-bit symbol from a fixed palette.

## Features

- Encoder and decoder library API.
- `colorcode` CLI for files and images.
- Exact 16-color RGB palette with no gradients, alpha, or antialiasing.
- PNG renderer with configurable module size and quiet zone.
- Metadata, CRC32 validation, optional LZ4/Zstd compression.
- GF(16) Reed-Solomon style block ECC for nibble streams.
- Delta-E color classification in CIELAB with white-balance estimation.
- Rotation-aware decoding for generated PNG/JPEG images.

## CLI

```sh
cargo run -- encode hello.txt
cargo run -- encode hello.txt --ecc high --module-size 24
cargo run -- encode archive.zip --compression zstd -o archive.c16.png
cargo run -- decode hello.c16.png -o hello.decoded.txt
cargo run -- capacity --ecc low
```

Without `-o`, `decode` writes the recovered bytes to stdout.

## Library

```rust
use colorcode::{Decoder, Encoder};

let code = Encoder::encode(b"hello")?;
code.matrix.save_png("hello.c16.png", 16)?;
let decoded = Decoder::decode("hello.c16.png")?;
assert_eq!(decoded.data, b"hello");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Version 1 Capacity

Version 1 reserves modules for finder patterns, timing, and two metadata copies.
The stored payload capacity depends on ECC level:

| ECC | Stored bytes |
| --- | -----------: |
| Low | 351 |
| Medium | 297 |
| High | 243 |
| Maximum | 189 |

Compression applies before ECC, so repetitive text can exceed those original
input sizes. Binary data that does not compress must fit within the selected
stored-byte capacity.

## Documentation

- [Specification](docs/specification.md)
- [Architecture](docs/architecture.md)
- [Developer guide](docs/developer-guide.md)
- [Performance notes](docs/performance.md)
