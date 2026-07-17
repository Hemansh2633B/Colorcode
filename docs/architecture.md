# Architecture

The crate is split into narrow modules:

- `encoder`: compression, CRC, ECC, layout, and matrix construction.
- `decoder`: image/matrix decode orchestration, metadata selection, ECC, CRC.
- `palette` and `classifier`: exact palette, CIELAB conversion, Delta-E match.
- `layout`, `finder`, `timing`, `metadata`: the wire format.
- `renderer`: RGB PNG rendering and matrix loading helpers.
- `vision`, `geometry`, `homography`: grid sampling and orientation support.
- `ecc`: GF(16) systematic block code with erasure and symbol repair.

The Version 1 layout is fixed, but the public `Version` enum and module
boundaries leave room for future 64x64, 96x96, and 128x128 formats.

## Decode Strategy

Generated images are sampled exactly when their dimensions match a rendered
ColorCode grid. Otherwise, the sampler estimates a non-white quadrilateral and
samples cell centers through a bilinear mapper. The resulting matrix is tested
in four rotations. For each plausible orientation, metadata copies are parsed,
the payload length determines the ECC stream size, and CRC32 selects the valid
decode.
