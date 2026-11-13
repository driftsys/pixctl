# AD-0015 Deterministic PNG encoding

## Context

`fit` outputs are committed to repositories. Output bytes that change between
runs, machines or thread counts produce noisy diffs. The release build is a
static musl binary built with `cargo zigbuild`.

## Options

- Optimise with `oxipng` after the `image` encoder. Rejected: `oxipng` 10 always
  compiles the C library libdeflate, whose AVX-512 code does not compile under
  `zig cc`, so the static musl release build fails.
- Encode with the pure Rust PNG encoder of `image`, with fixed settings.

## Decision

`fit::encode_png` encodes with `image::codecs::png::PngEncoder`, with
`CompressionType::Best` and `FilterType::Adaptive`. The encoder writes no
metadata chunks and no interlacing. No C library is involved.

## Consequences

The same image and the same pixctl version give the same bytes. A change of the
`image` version or of the two settings can change the bytes, so it is a
version-level change of `fit`. The PNG files are larger than the output of an
optimiser such as `oxipng`. Tests pin byte identity across runs, the absence of
`tEXt`, `iTXt`, `zTXt` and `tIME` chunks, and the two encoder settings.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
