# Diagram corpus with exact font-size labels — brief

## Why

pixctl estimates the body-text size of an image (`src/font.rs`, median line
height × `K`) and marks the estimate High or Low confidence. On a private corpus
of engineering diagrams, 1,348 of 2,026 images get Low confidence: 758 because
line heights vary too much (coefficient of variation ≥ 0.35) and 525 because
fewer than 3 lines are found. The next change will tune the confidence rules.
Before that, we need a public set of diagram images whose true body-text size is
known exactly, so that tuning cannot make wrong sizes confident. This brief is
only the corpus. Do not change `src/`.

## What to build

1. `tests/fixtures/diagrams/src/`: diagram source files, as text, for the tools
   installed on this Mac: Graphviz (`dot`), PlantUML (`plantuml`) and D2 (`d2`).
   draw.io is optional: its CLI export crashes in the sandbox, so skip it and
   say so in the README. Diagram kinds: block and architecture diagrams,
   sequence diagrams, state machines, flowcharts. Include what confuses a
   line-height rule: box borders, arrows, a title in a larger size, notes or
   legends in a smaller size, short labels (1 or 2 lines), and dense labels.
2. Body-text sizes from 8 to 20 px (at least 8, 10, 11, 12, 13, 14, 16, 20). The
   label of an image is its body-text size in pixels: the size used by most of
   the text in the image. Titles and notes in other sizes are allowed, but body
   text must be the clear majority of the text lines.
3. A generator, `tests/fixtures/diagrams/render.sh` (POSIX shell, passes
   `shellcheck`), that renders every source to PNG in
   `tests/fixtures/diagrams/png/` and writes
   `tests/fixtures/diagrams/manifest.toml` in the format `just eval` reads:

   ```toml
   [[image]]
   file = "png/dot-block-12.png"
   px = 12.0
   ```

   The generator also writes two variants of a subset: shrunk to 75 % (label ×
   0.75) and JPEG at quality 80 (same label). Use a pure tool already installed
   (`sips` on macOS is fine; note in the script that it needs macOS). Rendering
   must give the same bytes on every run.
4. A `just corpus` recipe that runs the generator. CI does not run it. Commit
   the sources, the PNGs, the JPEGs and the manifest. Keep the set to about 300
   images and under 10 MB in total.
5. Add the folder to the `exclude` list in `Cargo.toml` so it is not packaged.

## Labels must be exact — verify each tool

The whole value of the corpus is that the labels are right. Each tool maps its
font-size setting to pixels differently (points versus pixels, 72 versus 96 DPI,
output scale factors). For each tool:

- Render a calibration sample: a single line of capital "H" characters at a
  known setting, in the same font and the same output settings the corpus uses.
- Measure the cap height of the "H" in the PNG (count the rows of dark pixels).
- Compare it with the font's cap-height ratio (read the font's metrics, for
  example the OS/2 `sCapHeight` ÷ `unitsPerEm`, with a small script). This gives
  the em size in pixels.
- Fix the mapping from setting to label so that the label equals that em size
  within ±0.3 px.

Record the mapping and the measurements per tool in
`tests/fixtures/diagrams/README.md`. Never derive a label from pixctl's own
estimate.

## Test and gate

- Add an integration test (for example `tests/diagram_corpus.rs`) that checks
  every manifest entry exists and decodes, that labels are between 6 and 24 px,
  and that the manifest lists every image in `png/`.
- Run `just eval tests/fixtures/diagrams tests/fixtures/diagrams/manifest.toml`.
  It refuses a folder inside the repository by design, so copy the folder to a
  temporary directory outside the repository first. Record the current output in
  the README as the baseline. Do not add an accuracy gate yet; the confidence
  change adds it.

## Rules

- Rust 2021; `just build` must stay green (prim formats Markdown, clippy runs
  with `-D warnings`). Before `git`, `just` or `cargo`, run
  `export PATH=$HOME/.local/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH`.
- Conventional Commits with the scopes in `.git-std.toml`, for example
  `test(inspect): add the diagram corpus`. End each commit message with
  `Co-Authored-By: Codex <noreply@openai.com>`.
- Docs and comments in plain, literal English, no idioms.
- Only free fonts that may be redistributed in rendered images (DejaVu,
  Liberation, Noto, or the fonts in `tests/fonts`).
- Work only inside this worktree. Do not push.

## Report

When done, write `docs/wip/2026-11-28-diagram-corpus-report.md`: what you built,
the per-tool label verification results, the image count and total size, the
baseline `just eval` output, and anything you could not do.
