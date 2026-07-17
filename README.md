# ColorCode

**ColorCode** is a high-capacity, 16-color matrix code format written in Rust. Unlike QR codes, ColorCode uses a 16-color palette to store data more densely — a single 1280×1280 module code can hold **up to ~700 KB** of data.

## Features

- **High capacity**: ~708 KB per code (Low ECC), far exceeding QR codes
- **16-color palette**: Exact RGB colors, no gradients or anti-aliasing
- **Robust error correction**: 4 ECC levels (Low/Medium/High/Maximum) using GF(16) Reed-Solomon
- **Optional compression**: LZ4 or Zstd compression before encoding
- **CRC32 verification**: Automatic integrity checking
- **Rotation-aware decoding**: Works with rotated/camera-captured images
- **Cross-platform**: CLI tool, Rust library, and Android JNI bindings

---

## Quick Start

### Install (from source)

```bash
# Requires Rust 1.70+
git clone https://github.com/Hemansh2633B/Colorcode.git
cd Colorcode
cargo build --release
```

The binary will be at `target/release/colorcode`.

---

### Basic Usage

**Encode a file:**
```bash
colorcode encode myfile.txt
# Creates myfile.c16.png in the same directory
```

**Encode with options:**
```bash
colorcode encode myfile.txt --ecc high --module-size 24 --compression zstd -o output.png
```

**Decode a ColorCode image:**
```bash
colorcode decode myfile.c16.png -o decoded.txt
# Or to stdout:
colorcode decode myfile.c16.png > decoded.txt
```

**Check capacity:**
```bash
colorcode capacity --ecc low
```

---

## Understanding the Options

### ECC (Error Correction Code) Levels

ECC adds redundancy so the code can be recovered even if partially damaged, dirty, or misread.

| Level | Data Shards | Parity Shards | Correctable Errors/Block | Capacity (bytes) | Best For |
|-------|-------------|---------------|--------------------------|------------------|----------|
| **Low** | 13 | 2 | 1 | **708,799** | Clean prints, digital display |
| **Medium** | 11 | 4 | 2 | 599,811 | General use, slight wear |
| **High** | 9 | 6 | 3 | 490,823 | Outdoor, handling, lower quality print |
| **Maximum** | 7 | 8 | 4 | 381,835 | Harsh environments, camera capture |

**How it works**: Data is split into 15-symbol blocks. Each block has `data_shards` actual data + `parity_shards` recovery symbols. The decoder can reconstruct up to `parity_shards/2` corrupted symbols per block.

> **Recommendation**: Start with `Low`. Increase if your codes will be printed on rough surfaces, photographed at angles, or exposed to wear.

### Module Size

Size of each colored square in pixels.

| Size | Use Case | PNG Size (approx) |
|------|----------|-------------------|
| 2–4 | Digital display, small prints | ~5 MB |
| 8–16 | Standard prints, documents | ~5–10 MB |
| 24+ | Large posters, high-res scanning | 10+ MB |

> **Tip**: For camera/phone scanning, use **8–16**. Too small = hard to resolve; too large = huge files.

### Compression

Applied *before* ECC, so repetitive data shrinks dramatically.

| Method | ID | Best For |
|--------|-----|----------|
| **None** | 0 | Already-compressed files (zip, jpg, mp4, encrypted) |
| **LZ4** | 1 | Text, JSON, logs, source code — very fast |
| **Zstd** | 2 | Maximum compression for text/structured data |

> **Note**: Compression only helps if data has redundancy. Random/binary data won't compress.

---

## Library Usage (Rust)

Add to `Cargo.toml`:
```toml
[dependencies]
colorcode = { git = "https://github.com/Hemansh2633B/Colorcode.git" }
```

```rust
use colorcode::{Encoder, Decoder, EncoderOptions, EccLevel, Compression};

// Encode
let options = EncoderOptions {
    ecc_level: EccLevel::Low,
    compression: Compression::Zstd,
    module_size: 16,
    quiet_zone: 4,
    version: None,
};
let code = Encoder::new(options).encode_bytes(b"Hello, ColorCode!")?;
code.matrix.save_png("hello.c16.png", 16)?;

// Decode
let decoded = Decoder::decode("hello.c16.png")?;
assert_eq!(decoded.data, b"Hello, ColorCode!");
```

---

## File Format

- **Extension**: `.c16.png`
- **Magic bytes**: `C16M` (identifies ColorCode files)
- **Matrix**: 1280×1280 modules (4-bit each)
- **Finder patterns**: 4 chromatic 5×5 corners (rotation detection)
- **Timing pattern**: Row/column 5 between finders
- **Metadata**: 16 bytes × 2 copies (version, flags, compression, ECC, payload length, CRC32)
- **Payload**: Nibble-stream with GF(16) ECC blocks

---

## Performance

| Operation | Typical Time |
|-----------|--------------|
| Encode 100 KB (Low ECC) | ~50 ms |
| Encode 700 KB (Low ECC) | ~200 ms |
| Decode exact PNG | ~30 ms |
| Decode camera image | ~100–500 ms |

---

## Requirements

- **Rust**: 1.70+ (2024 edition)
- **Dependencies**: `image`, `clap`, `lz4_flex`, `zstd`, `crc32fast`

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

## Contributing

1. Fork the repository
2. Create a feature branch
3. Run `cargo fmt && cargo test`
4. Submit a PR

---

## Related

- [Specification](docs/specification.md) — Wire format details
- [Architecture](docs/architecture.md) — Module overview
- [Developer Guide](docs/developer-guide.md) — Adding versions/compression