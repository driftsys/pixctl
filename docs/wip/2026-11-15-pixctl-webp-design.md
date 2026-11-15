# pixctl — WebP output (`fit --format webp`)

Scope: [pixctl#9](https://github.com/driftsys/pixctl/issues/9), item 1. The user
approved this design on 2026-11-15.

## Intent

Some docs pipelines prefer WebP, which gives smaller files than PNG. `fit` gains
a lossless WebP output next to PNG.

## Approaches

- **The `image` crate's lossless WebP encoder (chosen).** `image` already
  depends on the pure-Rust `image-webp`, so no new dependency is needed and the
  musl build stays free of C code.
- The `webp` crate (libwebp bindings). Rejected: it adds C code, which breaks
  the zig musl build.
- Lossy WebP. Rejected: lossy compression blurs text, which the display contract
  exists to keep readable.

## Decisions

- `fit --format png|webp` uses `png` by default.
- The output is `<out>/<stem>.<ext>`, with the extension `png` or `webp`. A name
  collision compares the full file name, extension included, folded to
  lowercase.
- The rendering (flatten, scale, pad, frame) does not change. Only the encoder
  does.
- WebP output is lossless and has no metadata. The bytes are identical across
  runs.
- The JSON envelope does not change. `out` already carries the path.
- The `fit` help Examples show `--format webp`. AD-0008, the README and the
  records are updated.

## Non-goals

- No lossy WebP, no quality flag, no AVIF.
