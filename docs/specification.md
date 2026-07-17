# ColorCode Version 1 Specification

## Matrix

Version 1 is a 32x32 grid. Each module stores one nibble, rendered as exactly
one RGB color. Rendered images use a white quiet zone by default.

## Palette

| Value | Hex |
| ----: | --- |
| 0 | #000000 |
| 1 | #FFFFFF |
| 2 | #FF0000 |
| 3 | #00FF00 |
| 4 | #0000FF |
| 5 | #FFFF00 |
| 6 | #00FFFF |
| 7 | #FF00FF |
| 8 | #FF8000 |
| 9 | #8000FF |
| 10 | #80FF00 |
| 11 | #FF80C0 |
| 12 | #8B4513 |
| 13 | #808080 |
| 14 | #000080 |
| 15 | #006400 |

## Function Patterns

The four corners contain a 5x5 chromatic finder pattern. The pattern is not
rotationally symmetric, so the decoder can test four orientations and select
the one with the lowest function-pattern mismatch score.

A custom timing pattern occupies row 5 and column 5 between the finder regions.
The timing pattern uses deterministic color sequences rather than a black/white
alternation.

## Metadata

Metadata is 16 bytes, stored as 32 nibbles and written twice in the first data
slots.

| Bytes | Field |
| ----: | ----- |
| 0..4 | Magic `C16M` |
| 4 | Version |
| 5 | Flags |
| 6 | Compression method |
| 7 | ECC level |
| 8..12 | Stored payload length, big endian |
| 12..16 | CRC32 of original uncompressed bytes |

## Payload

Input bytes are optionally compressed, split into high/low nibbles, protected
with GF(16) ECC blocks, and placed row-major in non-reserved modules. Remaining
unused modules are deterministic padding.

## ECC

Every ECC block has 15 symbols. Levels use systematic data-plus-parity blocks:

| ECC | Data | Parity | Unknown symbol errors per block |
| --- | ---: | -----: | ------------------------------: |
| Low | 13 | 2 | 1 |
| Medium | 11 | 4 | 2 |
| High | 9 | 6 | 3 |
| Maximum | 7 | 8 | 4 |

The decoder also accepts known erasure positions from the color classifier.
