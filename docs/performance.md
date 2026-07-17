# Performance Notes

The encoder performs a single compression pass, one CRC32 pass, nibble packing,
small GF(16) ECC blocks, and direct RGB rendering. Version 1 payloads are small,
so avoiding excess allocations matters more than parallelism.

The decoder has two paths:

- Exact grid sampling for images produced by the renderer.
- Coarse quadrilateral sampling for cropped or camera-like images.

Exact PNG decode should be dominated by image loading. Camera-style decoding is
more variable because white-balance estimation and grid detection scan pixels.
The code keeps ECC blocks short and bounded, so correction work is predictable.
