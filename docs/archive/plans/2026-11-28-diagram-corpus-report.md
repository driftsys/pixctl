# Diagram corpus report

The corpus contains 96 text sources and 288 labelled images: 96 original PNGs,
96 reduced PNGs and 96 quality-80 JPEGs. It covers Graphviz, PlantUML and D2 at
body sizes 8, 10, 11, 12, 13, 14, 16 and 20 px. The sources include block,
architecture, sequence, state and flow diagrams, sparse and dense text, borders,
arrows, titles and legends. The corpus and its integration test are excluded
from the published crate. `just corpus` regenerates it; CI checks the script
with shellcheck but does not render the images.

The Noto Sans font has a cap-height ratio of 714 / 1000 = 0.714.
`calibration/calibrate.sh` renders the calibration sources at a nominal 100 px
(Graphviz 75 pt at 96 dpi, PlantUML size 100, D2 font-size 100 rasterized 1:1
with librsvg) and measures the cap height as 72, 71 and 72 rows. The implied em
is 100.84, 99.44 and 100.84 px, within 1 % of nominal. The resolution is one row
in 71 (about 1.4 %, about 0.2 px at 14 px). The corpus README has the output and
the details. No label uses pixctl's estimate.

Two complete renders with the installed tools produced identical SHA-256 hashes
for all 288 images and the manifest. The complete corpus directory totals
5,561,393 bytes.

The evaluation on a copy outside the repository produced:

```text
images: 288 (confident 36, review 252 = 87.5 %)
error on confident: mean 8.7 px, p95 146.1 px
confidently wrong (> 2 px): 2 of 36 = 5.6 % (limit 2 %)  FAIL
K interval for ±1.5 px on all labelled images (n = 288): none (no single K fits)
```

The existing evaluator returned exit code 1 because its current threshold was
exceeded. No new accuracy gate was added. D2's direct PNG export attempted to
install Playwright in this environment; the generator uses local SVG export and
librsvg instead. draw.io was omitted as requested because its CLI crashes in the
sandbox. Reproducibility across different versions of the rendering tools is not
established.

Validation: `just build` passed, including the corpus integration test, clippy,
shellcheck and the install test. The package listing excludes the corpus and its
integration test. The supplied brief was reflowed by prim for the formatting
gate and is committed at
`docs/archive/specs/2026-11-28-diagram-corpus-brief.md`.
