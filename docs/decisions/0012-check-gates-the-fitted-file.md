# AD-0012 `check` gates the fitted file

## Context

The contract has a 800 px column and a 766 px content width (800 − 2 × 17).
`check` must say which one a committed image is measured against.

## Options

- Gate the file as committed: `width` passes at 800 px or less, and the frame is
  part of the file.
- Gate the content width (766 px or less). Rejected: a fitted image is 800 px
  wide, so the output of `fit` would fail its own check.

## Decision

`check` gates the file as committed. `width` passes when the width is at most
`COLUMN` (800 px). `framed` requires the exact frame and padding, and the font
estimate runs on the content inside the frame when the image is framed.

## Consequences

`check` expects a fitted image. An unfitted image fails `framed`. `fit` output
passes `width`, `opaque` and `framed` for the `icon`, `unchanged` and `upscale`
rules; an integration test pins this. The `oversize` flag that
[AD-0004](0004-upscale-rule.md) deferred to `check` is the `width` check on the
committed file.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
