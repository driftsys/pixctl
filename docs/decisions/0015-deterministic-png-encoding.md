# AD-0015 Deterministic PNG and WebP encoding

## Context

`fit` outputs are committed to repositories. Output bytes that change between
runs, machines or thread counts produce noisy diffs. The release build is a
static musl binary built with `cargo zigbuild`.

## Options

- Optimise with `oxipng` after the `image` encoder. Rejected: `oxipng` 10 always
  compiles the C library libdeflate, whose AVX-512 code does not compile under
  `zig cc`, so the static musl release build fails.
- Encode with the pure Rust PNG encoder of `image`, with fixed settings.
- WebP with the `webp` crate (libwebp bindings). Rejected: it is C code, which
  breaks the static musl release build.
- Lossy WebP. Rejected: lossy compression blurs text, which the display contract
  needs to stay readable.
- Lossless WebP with the pure Rust encoder of `image-webp` (through `image`).

## Decision

`fit::encode_png` encodes with `image::codecs::png::PngEncoder`, with
`CompressionType::Best` and `FilterType::Adaptive`. The encoder writes no
metadata chunks and no interlacing. No C library is involved.

`fit::encode_webp` encodes with
`image::codecs::webp::WebPEncoder::new_lossless`, which is the lossless pure
Rust encoder of `image-webp`. It writes no metadata chunks (no EXIF, XMP or ICC
profile). The `image-webp` version is pinned through `Cargo.lock`, so the same
image and the same pixctl version give the same bytes in builds that use the
shipped `Cargo.lock` (the release binaries and `cargo install --locked`). A
build that resolves a newer `image-webp` can produce different bytes.

## Consequences

The same image and the same pixctl version give the same bytes. A change of the
`image` version or of the two settings can change the bytes, so it is a
version-level change of `fit`. The PNG files are larger than the output of an
optimiser such as `oxipng`. Tests pin byte identity across runs, the absence of
`tEXt`, `iTXt`, `zTXt` and `tIME` chunks, and the two encoder settings. For
WebP, tests pin byte identity and the absence of `EXIF`, `XMP` and `ICCP`
chunks. A change of the `image-webp` version can change the WebP bytes.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
