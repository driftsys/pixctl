# Diagram corpus

This corpus contains 96 source diagrams and 288 labelled images: one PNG,
one 75% PNG, and one quality-80 JPEG per source. It includes Graphviz block,
flow, state and architecture diagrams; PlantUML sequence, state, activity and
component diagrams; and D2 block, flow, state and architecture diagrams. The
sources include short two-line images, multiline labels, borders, arrows,
titles and legends. The body text is the majority of the text lines. The body
sizes are 8, 10, 11, 12, 13, 14, 16 and 20 px. Variant labels are multiplied
by 0.75 for the reduced images and unchanged for JPEGs.

Run `just corpus` on macOS with Graphviz, PlantUML, D2, Java, librsvg,
ImageMagick, Homebrew and shellcheck installed. The script isolates fontconfig
and registers the bundled Noto Sans font with Java. D2 produces SVG, and
librsvg rasterizes it at one output pixel per SVG pixel; the script replaces
D2's embedded font names with the same local font for librsvg. ImageMagick
creates the variants. The committed bytes were generated with Graphviz 13.1.2,
PlantUML 1.2025.7 and D2 0.7.1. Exact bytes may vary with renderer, font and
image-processing versions; repeated runs with the same tools must match. CI
does not regenerate the corpus. draw.io is omitted because its CLI export
crashes in the sandbox.

The font is `tests/fonts/NotoSans-Regular.ttf`, distributed under the SIL Open
Font License in `tests/fonts/LICENSE-NotoSans.txt`. The font and this corpus
are excluded from the published crate.

## Font size calibration

The `calibration/` directory contains the single-line `HHHHHHHH` samples. The
font's OS/2 `sCapHeight` is 714 and `head.unitsPerEm` is 1000, as printed by
`python3 tests/fixtures/diagrams/calibration/font_metrics.py`. Its cap-height
ratio is therefore 0.714. Each sample was rendered using the same font and
output path as the corpus, then converted to greyscale and thresholded at 50%.
The capital H occupies 10 dark pixel rows in each PNG. At the target em size
of 14 px, the font metrics predict 9.996 px; the measured 10 rows imply
10 / 0.714 = 14.006 px. This is within 0.3 px of the label. Pixel-row counts
are quantized; this measurement verifies the mapping at the calibration size,
not a separate subpixel measurement at every size.

| Tool | Source setting | Output settings | Em label | Dark rows |
| ---- | -------------- | --------------- | -------- | --------- |
| Graphviz | 10.5 pt | 96 dpi PNG | 14 px | 10 |
| PlantUML | 14 | 96 dpi PNG, registered font | 14 px | 10 |
| D2 | 14 | SVG, librsvg at 1:1 | 14 px | 10 |

Graphviz settings are three quarters of the intended pixel size because 96 dpi
maps points to pixels by 96/72. PlantUML and D2 settings equal the label in
these output paths. The labels do not use pixctl's estimate.

## Evaluation baseline

The corpus was copied outside the repository before running `just eval`. The
current evaluator reports the following totals and exits 1 because its existing
confidently-wrong threshold is exceeded. This is a baseline, not a new gate.

```text
images: 288 (confident 36, review 252 = 87.5 %)
error on confident: mean 8.7 px, p95 146.1 px
confidently wrong (> 2 px): 2 of 36 = 5.6 % (limit 2 %)  FAIL
K interval for ±1.5 px on all labelled images (n = 288): none (no single K fits)
```
