# Architecture

## Overview

ColorCode is a 16-color, nibble-addressed matrix code format. Version 2 uses a
1280x1280 module grid with a capacity of ~708 KB at Low ECC.

## Module Layout

The matrix is fixed at 1280x1280. Reserved regions include four 5x5 finder
patterns at the corners and a custom timing pattern at row/column 5.

## Pipeline

```
Input bytes
    → Compression (optional: LZ4, Zstd)
    → CRC32
    → Split into nibbles
    → GF(16) ECC (15-symbol blocks, 4 levels)
    → Matrix placement (finder, timing, metadata, payload)
    → RGB rendering (PNG)
```

Decoding reverses the pipeline: image → sampled matrix → orientation search →
metadata candidates → ECC decode → CRC32 verify → decompress → output.

## Decode Strategy

Generated images are sampled exactly when their dimensions match a rendered
ColorCode grid. Otherwise, the sampler estimates a non-white quadrilateral and
samples cell centers through a bilinear mapper. The resulting matrix is tested
in four rotations. For each plausible orientation, metadata copies are parsed,
the payload length determines the ECC stream size, and CRC32 selects the valid
decode.