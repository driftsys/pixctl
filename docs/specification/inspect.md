# `inspect` — requirements

Scope source: [pixctl#1](https://github.com/driftsys/pixctl/issues/1).
Implemented in milestone M1. Decisions:
[AD-0001](../decisions/0001-font-estimate-heuristic.md) to
[AD-0011](../decisions/0011-k-calibration-rule.md).

## Display contract

- The display column is 800 px wide. The minimum displayed font size is 11 px.
- `fit` adds 16 px padding and a 1 px `#d0d7de` frame on each side, so the
  content width budget is 766 px.
- An image whose longer side is at most 64 px is an icon.

## Command

`pixctl inspect <image>... [--json] [--font-px F]`

- For each image, `inspect` reports the width, the height, whether any pixel is
  transparent (alpha < 255), the font estimate with its confidence, the rule,
  the scale factor and the output width.
- Supported input formats are PNG, JPEG, BMP, GIF, TIFF and WebP. EMF and WMF
  files are reported as unsupported.
- `--font-px F` replaces the estimate for every input. `F` is a positive number
  and may be a decimal. The CLI rejects `0`, negative values and non-numbers
  with exit `2`.
- Without `--json`, `inspect` prints one line per image. With `--json`, it
  prints an array with one object per image that loaded.
- An image that fails to load is reported on stderr with its path. The other
  images are still reported. The exit code is `2` when any image fails, and `0`
  otherwise.
- A low confidence is not a failure: `inspect` exits `0` for it.

## Rules

- `icon`: the longer side is at most 64 px.
- `oversize`: the width is above 766 px.
- `upscale`: the font is below 11 px and is known with high confidence (or set
  by `--font-px`), and a factor of 1.5 or 2 fits in 766 px and brings the font
  to at least 11 px.
- `too-small`: the font is below 11 px, is known with high confidence, and no
  allowed factor meets both conditions above.
- `unchanged`: every other case, including an unknown or low-confidence font.

## Estimator accuracy

- On the generated fixtures at 11 px and above, the estimate is within ±1.5 px
  of the em size.
- On noise and blank images, the estimate has low confidence.

## Terminology

`needs-review` is the status that the future `check` command (M2) produces for a
`low` confidence. `inspect` reports only the confidence.
