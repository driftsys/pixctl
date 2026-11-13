# AD-0009 Output width

## Context

`inspect` reports the width of the image that `fit` would write.

## Options

Alternatives not documented.

## Decision

`output_width = round(w × factor) + 2 × (16 + 1)` for every rule except `icon`.
The factor is 1 for `unchanged`, `too-small` and `oversize`. For `icon`,
`output_width = w`, because icons are only flattened, never padded.

## Consequences

An `oversize` image is not scaled, so its `output_width` is wider than the 800
px column. `fit` produces exactly this width.

Satisfies: [AD-0004](0004-upscale-rule.md)
