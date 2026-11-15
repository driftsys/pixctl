# AD-0008 EMF/WMF input uses an external converter

## Context

Some document images are EMF or WMF vector files.

## Options

- A pure-Rust EMF/WMF renderer: rejected, no mature crate exists.
- Require users to convert the files first: rejected, issue #1 asks for an
  optional converter.

## Decision

`load::load` converts `.emf` and `.wmf` files (any case) to PNG with an external
converter, so `inspect`, `check` and `fit` accept them. pixctl adds no C code
and calls a program only when one is installed.

The converter is chosen in this order: `PIXCTL_CONVERTER` (kind chosen from the
file name: `inkscape`, or `soffice`/`libreoffice`; any other name is
`convert_failed`), then `inkscape`, `soffice` and `libreoffice` on `PATH`.
Without a converter the file fails with `unsupported_type` and a hint; the other
files are still processed (exit `2`). A converter that exits non-zero, times out
(60 s, or `PIXCTL_CONVERT_TIMEOUT_SECS`) or writes no usable PNG gives
`convert_failed`. On Unix a timeout kills the converter's process group with the
`kill` utility.

WebP output was never deferred: `fit --format webp` writes a lossless WebP with
the `image` crate, which needs no C code.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
