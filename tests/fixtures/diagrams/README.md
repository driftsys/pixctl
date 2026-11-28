# Diagram corpus

This corpus contains 96 source diagrams and 288 labelled images: one PNG,
one 75% PNG, and one quality-80 JPEG per source. It includes Graphviz block,
flow, state and architecture diagrams; PlantUML sequence, state, activity and
component diagrams; and D2 block, flow, state and architecture diagrams. The
sources include short two-line images, multiline labels, borders, arrows,
titles and legends. The body text is the majority of the text lines. The body
sizes are 8, 10, 11, 12, 13, 14, 16 and 20 px. Variant labels are multiplied
by 0.75 for the reduced images and unchanged for JPEGs.

Run `just corpus` on macOS with Graphviz, PlantUML, D2, librsvg, ImageMagick,
Homebrew and a Java JDK (javac) installed. The script isolates fontconfig
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

The labels give the em size in pixels. The calibration shows that each tool
setting in `render.sh` produces that em size. `calibration/calibrate.sh`
renders the sources in `calibration/` (a single line `HHHHHHHH`) at a nominal
em of 100 px with the same font and pipeline as `render.sh`: Graphviz
`fontsize` 75 pt at `dpi=96`, PlantUML `defaultFontSize` 100, and D2
`style.font-size` 100. It flattens each PNG on white, thresholds it at 50 %,
and counts the rows of dark pixels, which is the cap height of the H. The
font's cap height is 714 of 1000 units, so the expected value is 71.4 px
(`calibration/font_metrics.py` prints the font metrics). The script prints the
measured cap height, the implied em (cap height / 0.714) and the error.

```text
graphviz cap height  72 px  implied em 100.84 px  error  +0.8 %
plantuml cap height  71 px  implied em  99.44 px  error  -0.6 %
d2       cap height  72 px  implied em 100.84 px  error  +0.8 %
```

The measurement counts whole pixel rows, so one row in 71 (about 1.4 %) is the
resolution. At a body size of 14 px that is about 0.2 px. The three tools are
within one row of the nominal size, so the labels are correct to about that
precision. They do not use pixctl's estimate. The calibration measures the
mapping at 100 px and assumes that the tools scale linearly down to the body
sizes of the corpus; it does not measure each size separately.

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
