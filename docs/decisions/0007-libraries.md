# AD-0007 Libraries

## Context

pixctl decodes images, thresholds them, finds connected components and, from M3,
writes optimised PNG files.

## Options

Alternatives not documented.

## Decision

- `image` 0.25 for decoding and greyscale.
- `imageproc` for connected components. Its `otsu_level` is replaced by a local
  threshold ([AD-0010](0010-local-otsu-threshold.md)).
- `oxipng` for PNG output, added in M3.
- `ab_glyph` as a dev-dependency for the synthetic fixtures.
- Line grouping, median height and confidence are written in pixctl.

The `imageproc` version is the release that depends on the same `image` major
version.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
