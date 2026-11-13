# AD-0007 Libraries

## Context

pixctl decodes images, thresholds them, finds connected components and writes
optimised PNG files (`fit`).

## Options

Alternatives not documented.

## Decision

- `image` 0.25 for decoding, with default features off and only the PNG, JPEG,
  BMP, GIF, TIFF and WebP decoders enabled.
- `imageproc`, with default features off, for connected components. Its
  `otsu_level` is replaced by a local threshold
  ([AD-0010](0010-local-otsu-threshold.md)).
- `oxipng` 10.2.1, with default features off, for PNG output
  ([AD-0015](0015-deterministic-png-encoding.md)).
- `ab_glyph` as a dev-dependency for the synthetic fixtures.
- Flattening onto white, greyscale conversion, line grouping, median height and
  confidence are written in pixctl.

The `imageproc` version is the release that depends on the same `image` major
version.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
