# pixctl MVP — design

Scope source: [pixctl#1](https://github.com/driftsys/pixctl/issues/1). This
document fixes the decisions that issue leaves open and orders the work.

## Decisions

1. **Font-size estimation: pure-Rust heuristic.** No OCR model in the MVP. The
   estimator returns a size plus a confidence; low confidence gives
   `needs-review`. `--font-px N` overrides the estimate for every input. `ocrs`
   stays a later option behind the same interface.
2. **Contract constants live in `src/contract.rs`.** They are mirrored by hand
   with diagctl until
   [diagctl#22](https://github.com/driftsys/diagctl/issues/22) picks the shared
   mechanism.
3. **The padding and frame count inside the column.** The content width budget
   is `C − 2 × (16 + 1) = 766 px`.
4. **The upscale factor is 1.5 or 2.** The needed factor is
   `min(766 ÷ w, 12 ÷ font)`. The tool picks the smallest of {1.5, 2} that is ≥
   the needed factor, caps it at 2, and drops to 1.5 when 2 would exceed 766 px.
   If even 1.5 overflows, it does not upscale, and the image is flagged. 2× uses
   nearest-neighbour and 1.5× uses Lanczos3.
5. **`needs-review` is not a failure.** `check` prints `REVIEW` and still exits
   `0`, while `--strict` turns REVIEW into FAIL (exit `1`). Unknown font size
   must not break CI on day one.
6. **Single crate for now.** Modules keep pure interfaces (notably
   `font::estimate`) so the contract and the estimator can be extracted into
   crates later, when diagctl or tblctl consume them.
7. **Libraries.** `image` (decode, greyscale), `imageproc` (Otsu threshold,
   connected components), `oxipng`; `ab_glyph` as a dev-dependency. Line
   grouping, median height and confidence are hand-rolled.
8. **EMF/WMF are deferred.** In the MVP they are reported as unsupported (exit
   `2` for that file). WebP output is also deferred to the last milestone.

## Units

| Module        | Purpose                                      | Depends on |
| ------------- | -------------------------------------------- | ---------- |
| `contract.rs` | C, F, padding, frame colour, icon limit      | —          |
| `load.rs`     | decode PNG/JPEG/BMP/GIF/TIFF/WebP to RGBA8   | `image`    |
| `font.rs`     | `estimate(&GrayImage) -> FontEstimate`       | —          |
| `rule.rs`     | pure: (w, h, alpha, font) → `Rule` + factor  | contract   |
| `fit.rs`      | scale, flatten onto white, pad + frame, PNG  | rule       |
| `checks.rs`   | `width`, `min-font-size`, `opaque`, `framed` | rule, font |
| `commands/*`  | thin CLI glue, output, exit codes            | all        |

`rule.rs` and `font.rs` are pure functions and carry most of the tests.

### Font heuristic (`font.rs`)

1. Convert to greyscale, apply an Otsu threshold, and take the majority class as
   the background (so light-on-dark text also works).
2. Find 8-connected components and keep the glyph-like ones: height 4–200 px,
   aspect 0.1–3, area ≥ 6 px.
3. Group the components into text lines by vertical overlap. Use the line
   heights, from ascender top to descender bottom.
4. font px ≈ median line height × k, with `k` calibrated on the fixtures.
5. Confidence is high when there are ≥ 3 lines and ≥ 20 glyphs, and the
   coefficient of variation of the line heights is < 0.35. Otherwise the result
   is `needs-review`.

## Commands

- `inspect <img…> [--json] [--font-px N]` reports width, height, alpha, the font
  estimate with its confidence, the rule, the factor and the output width.
- `check <img…> [--target markdown] [--strict] [--font-px N]` prints one
  `PASS|FAIL|REVIEW` line per check and image. Exit codes are `0`/`1`/`2`.
- `fit <img…> --out DIR [--font-px N]` applies the rule, flattens onto
  `#ffffff`, adds 16 px padding and a 1 px `#d0d7de` frame, and writes a PNG
  (oxipng, metadata stripped). Icons ≤ 64 px are only flattened.

Determinism: PNG output has no timestamps or text chunks, uses a fixed oxipng
preset, and is pinned through `Cargo.lock`. A test fits the same input twice and
compares the bytes.

## Testing

- The text fixtures are generated in tests: `ab_glyph` plus the embedded DejaVu
  font render known strings at 9, 11, 14 and 20 px, so the expected font size is
  exact and no large binaries are committed.
- Small committed fixtures (`tests/fixtures/`) cover an icon, an alpha PNG, an
  oversize screenshot-like image, a JPEG and a TIFF.
- The estimator must land within ±1.5 px on the generated fixtures at 11 px and
  above, and produce `needs-review` on noise and photos.
- Strict TDD per milestone.

## Corpus

- **Synthetic (M1):** generated at test time with `ab_glyph` and open fonts
  (DejaVu, Liberation, Noto) at 8–24 px. It varies layout, inverted colours,
  alpha, JPEG artefacts and 0.5×/1.5× scaling. It calibrates `k` and the
  confidence thresholds.
- **Real public (M2):** about 20 open-licence screenshots in
  `tests/fixtures/real/`. A `manifest.toml` holds each file's source, licence
  and expected font range, or `review`.
- **Private:** `just eval DIR` runs against a local, uncommitted corpus and
  prints aggregate metrics only.
- **Metrics:** mean and p95 error on confident results, the `needs-review`
  share, and the confidently wrong rate (off by more than 2 px), which must stay
  below 2%.

## Milestones (one PR each)

1. **M1 `inspect`:** contract, load, font, rule, and the JSON output.
2. **M2 `check`:** the four checks, `--strict`, and the exit codes.
3. **M3 `fit` (PNG):** pad, frame and flatten, with the determinism test.
4. **M4:** WebP output, plus an EMF/WMF external converter when one is
   installed.

## Out of scope

OCR transcription, crop/redraw tooling for oversize images, dark/light variants.
