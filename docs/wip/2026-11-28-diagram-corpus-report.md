# Diagram corpus report

The corpus contains 96 text sources and 288 labelled images: 96 original PNGs,
96 reduced PNGs and 96 quality-80 JPEGs. It covers Graphviz, PlantUML and D2 at
body sizes 8, 10, 11, 12, 13, 14, 16 and 20 px. The sources include block,
architecture, sequence, state and flow diagrams, sparse and dense text, borders,
arrows, titles and legends. The corpus and its integration test are excluded
from the published crate. `just corpus` regenerates it; CI checks the script
with shellcheck but does not render the images.

The Noto Sans font has a cap-height ratio of 714 / 1000 = 0.714. At a target 14
px em size, the calibration images for all three tools have 10 dark rows, which
implies 14.006 px. Graphviz used 10.5 pt at 96 dpi; PlantUML used size 14 at 96
dpi; D2 used size 14 in SVG rasterized 1:1 with librsvg. The corpus README
contains the mappings and calibration details. No label uses pixctl's estimate.

Two complete renders with the installed tools produced identical SHA-256 hashes
for all 288 images and the manifest. The complete corpus directory totals
5,559,127 bytes.

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
integration test. The supplied untracked brief was reflowed by prim for the
formatting gate and remains uncommitted.
