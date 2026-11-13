# `inspect` — design

Requirements: [specification/inspect.md](../specification/inspect.md).

## Components

One crate. `src/lib.rs` declares the modules; `src/main.rs` is thin glue that
parses the arguments, calls the commands. `check` and `fit` are in
[check-fit.md](check-fit.md).

| Module                                 | Purpose                                                                         |
| -------------------------------------- | ------------------------------------------------------------------------------- |
| [`cli.rs`](../../src/cli.rs)           | `clap` definitions; validates `--font-px` (positive, finite)                    |
| [`contract.rs`](../../src/contract.rs) | display contract constants ([AD-0002](../decisions/0002-contract-constants.md)) |
| [`load.rs`](../../src/load.rs)         | decodes to RGBA8; `has_alpha` is true when any pixel has alpha < 255            |
| [`font.rs`](../../src/font.rs)         | pure `estimate(&GrayImage) -> FontEstimate`                                     |
| [`rule.rs`](../../src/rule.rs)         | pure `plan(w, h, font_px: Option<f32>) -> Plan`                                 |
| [`inspect.rs`](../../src/inspect.rs)   | builds one `Report` per image, prints plain lines or JSON                       |

## Interfaces

- `contract`: `COLUMN = 800`, `MIN_FONT_PX = 11`, `TARGET_FONT_PX = 12`,
  `PADDING = 16`, `FRAME = 1`, `FRAME_COLOUR = #d0d7de`, `ICON_MAX = 64`,
  `CONTENT_WIDTH = COLUMN − 2 × (PADDING + FRAME) = 766`.
- `load::load(path) -> Result<Loaded { rgba, has_alpha }>`. Paths ending in
  `.emf`/`.wmf` (any case) return an error that contains `unsupported`. Errors
  carry the path.
- `font::estimate` returns `px: Option<f32>`, `confidence: High | Low`, and the
  `lines` and `glyphs` counts.
- `rule::plan` returns `Plan { rule, factor, output_width }`. `None` as the font
  never produces an upscale.
- `inspect::run(images, json, font_px, out, err) -> i32`.

## Font estimator

1. `inspect` flattens the RGBA image onto white and converts it to greyscale.
2. A local Otsu threshold splits the pixels; the majority class is the
   background, so light-on-dark text also works
   ([AD-0010](../decisions/0010-local-otsu-threshold.md)).
3. 8-connected components are kept as glyphs when their height is 4–200 px,
   their aspect is 0.1–3 and their area is at least 6 px.
4. Glyphs are grouped into text lines by vertical overlap. A line height spans
   from the ascender top to the descender bottom.
5. `px = median line height × K`, with `K = 1.045`
   ([AD-0011](../decisions/0011-k-calibration-rule.md)). Font px means the CSS
   em size.
6. Confidence is `high` when there are at least 3 lines, at least 20 glyphs, and
   the coefficient of variation of the line heights is below 0.35. Otherwise it
   is `low`. With no line, `px` is `null`.

## Rule

`plan` checks, in order: icon, oversize, then the font. See
[AD-0004](../decisions/0004-upscale-rule.md) for the upscale rule and
[AD-0009](../decisions/0009-output-width.md) for the output width. `inspect`
passes `None` to `plan` when the estimate has low confidence and no `--font-px`
is given.

## Output

- JSON: an array of objects with `path`, `width`, `height`, `alpha`,
  `font { px, confidence, lines, glyphs, source }`, `rule`, `factor`,
  `output_width`. `source` is `estimate` or `override`. With `--font-px`,
  confidence is `high` and `lines`/`glyphs` are `0`. Rules are kebab-case.
  `path` is written with invalid UTF-8 replaced by U+FFFD, so one path never
  stops the output.
- Plain: one line per image, with the font shown as `<px>px` or `?px`.
- Load failures go to stderr only (`pixctl: <error>`).

## Testing

- `tests/support/mod.rs` renders known text with `ab_glyph` and three open fonts
  committed in [`tests/fonts/`](../../tests/fonts/README.md) (DejaVu Sans,
  Liberation Sans, Noto Sans), so the em size is exact.
- Load-test images are generated into a temp directory at test time instead of
  committed binaries.
- `tests/font_accuracy.rs` checks the ±1.5 px acceptance; `tests/inspect.rs`
  covers the command, the JSON shape, blank and transparent images, and mixed
  valid and failing inputs.
