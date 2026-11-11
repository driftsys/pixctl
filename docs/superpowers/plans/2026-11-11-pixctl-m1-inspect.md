# pixctl M1 `inspect` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `pixctl inspect <img…> [--json] [--font-px N]` reports width, height,
alpha, the estimated font size with its confidence, the display rule, the scale
factor and the output width.

**Architecture:** One crate with a `lib.rs` (modules `contract`, `load`, `font`,
`rule`, `inspect`) and a thin `main.rs`. `font::estimate` and `rule::plan` are
pure functions. A test-only fixture generator renders known text with
`ab_glyph`, so the true font size is known exactly and `k` can be calibrated.

**Tech Stack:** Rust 2021 (toolchain pinned in `rust-toolchain.toml`), `clap`,
`anyhow`, `image`, `imageproc`, `serde` + `serde_json`; dev: `ab_glyph`,
`tempfile`, `assert_cmd`, `predicates`. `oxipng` is not needed before M3.

**Spec:** `docs/superpowers/specs/2026-11-11-pixctl-mvp-design.md` (decisions
1–8, "Font heuristic", "Commands", "Testing", "Corpus"). Issue:
[pixctl#1](https://github.com/driftsys/pixctl/issues/1).

## Global Constraints

- Column `C = 800` px; minimum displayed font `F = 11` px; padding 16 px; frame
  1 px `#d0d7de`; icon limit 64 px; content budget `C − 2 × (16 + 1) = 766` px.
- Upscale factor is 1.5 or 2; needed factor is `min(766 ÷ w, 12 ÷ font)`; 2× is
  nearest-neighbour, 1.5× is Lanczos3 (applied in M3, only chosen here).
- `needs-review` is not a failure: `inspect` exits `0` for it. Exit `2` when any
  file cannot be processed; the other files are still processed.
- EMF/WMF are reported as unsupported (exit `2` for that file).
- Estimator accuracy: within ±1.5 px at ≥ 11 px on generated fixtures;
  `needs-review` (low confidence) on noise and photo-like images.
- `--font-px N` overrides the estimate for every input.
- Commits: Conventional Commits with the scopes in `.git-std.toml` (`inspect`,
  `deps`, `cli`, `docs`, `repo`). No `Co-authored-by` trailer on driftsys
  commits. Run `just fmt` before committing (it formats Markdown with `prim`).
- `just build` and `just verify` must pass before the PR is opened.

## Decisions this plan adds to the spec

The spec leaves these open. The plan fixes them; the reviewer should check them.

1. **Rule values:** `icon` (max(w,h) ≤ 64), `oversize` (w > 766, or an upscale
   that cannot fit), `upscale` (factor 1.5 or 2), `unchanged`.
2. **Unknown font:** when no font size is known, no font-driven upscale is
   chosen. The rule comes from the width alone (`unchanged` or `oversize`).
3. **Output width:** `round(w × factor) + 2 × (16 + 1)`; for `icon` it is `w`
   (icons are only flattened, never padded).
4. **Font px means em size** in CSS terms, not ascent + descent. The fixture
   generator converts accordingly (see Task 3).
5. **Fixtures for load tests** are generated into a temp directory at test time
   instead of committed binaries (the spec says "small committed fixtures"; M2
   commits the real corpus). Generation keeps the repo free of binaries and
   keeps each test self-describing.
6. **JSON** is an array with one object per successfully inspected image;
   failures go to stderr only.

## Review Focus

Most likely to bite first; each line is pinned by a test in the named task.

1. Blank uniform image (all white or all black): no components. Expect
   `px: null`, low confidence, no panic. (Task 4)
2. Fully transparent PNG: after flattening onto white it is blank. Expect
   `alpha: true` and the same result as case 1. (Task 5)
3. Corrupt, truncated, missing or EMF file among valid files: that file is
   reported on stderr, the others are printed, exit `2`. (Task 5)
4. 16-bit PNG, palette GIF and greyscale JPEG decode to the same
   `width`/`height`/`alpha` they declare. (Task 2)
5. `--font-px 0`, negative or `abc` is rejected by the CLI with exit `2`.
   (Task 5)

---

### Task 1: Library skeleton, contract constants, dependencies

**Files:**

- Create: `src/lib.rs`, `src/contract.rs`
- Modify: `Cargo.toml`, `src/main.rs` (use the library; behaviour unchanged)
- Test: unit test in `src/contract.rs`

**Interfaces:**

- Produces:
  `pixctl::contract::{COLUMN: u32 = 800, MIN_FONT_PX: u32 = 11,
TARGET_FONT_PX: u32 = 12, PADDING: u32 = 16, FRAME: u32 = 1, FRAME_COLOUR:
[u8; 3] = [0xd0, 0xd7, 0xde], ICON_MAX: u32 = 64, CONTENT_WIDTH: u32}`
  where `CONTENT_WIDTH = COLUMN - 2 * (PADDING + FRAME)`.

- [ ] **Step 1: Failing test** `content_width_is_766` in `src/contract.rs`:
      `assert_eq!(CONTENT_WIDTH, 766)` and
      `assert_eq!(FRAME_COLOUR, [208, 215,
222])`.
- [ ] **Step 2:** Run `cargo test --lib contract` → fails (module missing).
- [ ] **Step 3:** Add `src/lib.rs` declaring `pub mod contract;` (later tasks
      add their modules there), move the CLI module into the library
      (`pub mod cli;`), and have `main.rs` call `pixctl::cli`. Add dependencies
      with exact major versions: `image = "0.25"`, `imageproc = "0.27"`,
      `serde = { version = "1",
features = ["derive"] }`, `serde_json = "1"`;
      dev: `ab_glyph = "0.2"`, `tempfile = "3"`. Commit `Cargo.lock`.
- [ ] **Step 4:** `cargo test --locked` → all pass, including the existing
      `tests/cli.rs`.
- [ ] **Step 5: Commit**
      `feat(inspect): add contract constants and library
skeleton` (deps in the
      same commit; scope `inspect`).

### Task 2: `load.rs` — decode to RGBA8

**Files:**

- Create: `src/load.rs`
- Modify: `src/lib.rs` (`pub mod load;`)
- Test: unit tests in `src/load.rs` using `tempfile`

**Interfaces:**

- Produces:
  `pub struct Loaded { pub rgba: image::RgbaImage, pub has_alpha: bool }`,
  `pub fn load(path: &std::path::Path) -> anyhow::Result<Loaded>`. `has_alpha`
  is true when any pixel has alpha < 255 (a declared alpha channel with
  all-opaque pixels is `false`: only visible transparency matters). Paths ending
  in `.emf`/`.wmf` (case-insensitive) return an error whose message contains
  `unsupported`. Errors carry the path in their context.

- [ ] **Step 1: Failing tests** (generate each file with the `image` crate into
      a `tempfile::tempdir()`):
  - `loads_png_with_alpha`: 4×3 RGBA PNG with one pixel alpha 0 →
    `(4, 3, true)`.
  - `opaque_rgba_png_has_no_alpha`: all alpha 255 → `has_alpha == false`.
  - `loads_jpeg`, `loads_tiff`, `loads_bmp`, `loads_webp_lossless`: size
    matches.
  - Review Focus 4: `loads_16bit_png` (`ImageBuffer<Rgba<u16>>`),
    `loads_palette_gif` (2-colour GIF), `loads_greyscale_jpeg` (`GrayImage`
    saved as `.jpg`): size matches, no error.
  - `emf_is_unsupported`: an empty file named `a.emf` → error containing
    `unsupported`.
  - `missing_file_error_names_the_path`, `truncated_png_errors` (first 20 bytes
    of a valid PNG).
- [ ] **Step 2:** `cargo test --lib load` → fails.
- [ ] **Step 3:** Implement with `image::open` → `to_rgba8()`; check the
      extension before opening.
- [ ] **Step 4:** `cargo test --lib load` → pass.
- [ ] **Step 5: Commit** `feat(inspect): decode images to RGBA8`.

### Task 3: Test fixture generator (`tests/support`)

**Files:**

- Create: `tests/support/mod.rs`, `tests/fonts/README.md`,
  `tests/fonts/{DejaVuSans.ttf,LiberationSans-Regular.ttf,NotoSans-Regular.ttf}`
  plus each font's licence file, `tests/support_selftest.rs`
- Modify: `Cargo.toml` (`exclude = ["tests/fonts"]` so the crate package stays
  small)

**Interfaces:**

- Produces (all in `tests/support/mod.rs`, `pub`):
  - `enum Face { DejaVu, Liberation, Noto }`
  - `struct TextSpec { face: Face, em_px: f32, lines: usize, invert: bool }`
  - `fn render_text(spec: &TextSpec) -> image::GrayImage` — renders `lines`
    lines of the pangram
    `"The quick brown fox jumps over the lazy dog 0123456789"` (identical on
    each line) in black on white (white on black when `invert`), with a margin
    of 2 × `em_px` and a line pitch of `1.4 × em_px`.
  - `fn jpeg_roundtrip(img: &GrayImage, quality: u8) -> GrayImage`
  - `fn resize(img: &GrayImage, factor: f32) -> GrayImage` (Lanczos3)
  - `fn noise(w: u32, h: u32, seed: u64) -> GrayImage` — deterministic LCG, no
    `rand` dependency.
  - `fn photo_like(w: u32, h: u32, seed: u64) -> GrayImage` — sum of a few
    low-frequency sine gradients plus mild noise (no text-like components).
- Em conversion (decision 4):
  `PxScale::from(em_px * font.height_unscaled() /
font.units_per_em().unwrap())`,
  so `em_px` is the CSS-like em size.

- [ ] **Step 1:** Download the three open-licence fonts from their upstream
      releases (DejaVu: Bitstream Vera licence; Liberation and Noto: SIL OFL
      1.1). Store each licence text beside the font. Write
      `tests/fonts/README.md` with source URL, version and `sha256` per file.
- [ ] **Step 2: Failing tests** in `tests/support_selftest.rs`:
  - `render_text_has_ink_and_is_deterministic`: two renders of the same spec are
    byte-equal and contain both black and white pixels.
  - `invert_swaps_background`: the corner pixel is 255 normally and 0 inverted.
  - `noise_is_deterministic`: same seed, equal; different seed, different.
- [ ] **Step 3:** Run `cargo test --test support_selftest` → fails (no module).
- [ ] **Step 4:** Implement the generator with `ab_glyph` (`include_bytes!` the
      fonts; draw glyph coverage into the buffer with `outline_glyph` + `draw`).
- [ ] **Step 5:** Run → pass.
- [ ] **Step 6: Commit** `test(inspect): add synthetic text fixture generator`.

### Task 4: `font.rs` — estimator and calibration

**Files:**

- Create: `src/font.rs`, `tests/font_accuracy.rs`
- Modify: `src/lib.rs` (`pub mod font;`)

**Interfaces:**

- Consumes: `tests/support` (Task 3).
- Produces: `pub enum Confidence { High, Low }` (serde `lowercase`),
  `pub struct FontEstimate { pub px: Option<f32>, pub confidence: Confidence,
pub lines: usize, pub glyphs: usize }`,
  `pub fn estimate(img: &image::GrayImage) -> FontEstimate`, `pub const K: f32`
  (calibrated), plus the confidence thresholds as consts (`MIN_LINES = 3`,
  `MIN_GLYPHS = 20`, `MAX_CV = 0.35` start values). No component →
  `px: None, confidence: Low, lines: 0, glyphs: 0`.

Algorithm (spec "Font heuristic"): greyscale → Otsu (`imageproc`) → majority
class is background → 8-connected components of the foreground class → keep
glyph-like ones (height 4–200 px, aspect 0.1–3, area ≥ 6 px) → group by vertical
overlap into lines (line height = min top to max bottom of its members) →
`px = median(line heights) × K` → confidence per the spec.

- [ ] **Step 1: Failing tests** in `tests/font_accuracy.rs`:
  - `exact_sizes_within_tolerance`: for each face in {DejaVu, Liberation, Noto}
    and `em_px` in {11, 14, 20, 24}, 6 lines, normal and inverted:
    `|px − em_px| ≤ 1.5` and `confidence == High`.
  - `small_text_is_not_confidently_wrong`: `em_px` in {8, 9, 10}: either `Low`,
    or `|px − em_px| ≤ 1.5`.
  - `survives_jpeg_and_scaling`: DejaVu 14 px through `jpeg_roundtrip(…, 75)`
    and through `resize(…, 1.5)` (expected 21 px) stay within ±1.5 px or are
    `Low`.
  - `noise_is_low_confidence` and `photo_like_is_low_confidence`: 400×300,
    seed 1.
  - Review Focus 1: `blank_white_and_blank_black_are_low_and_none`.
  - `few_lines_is_low_confidence`: 2 lines → `Low`.
- [ ] **Step 2:** `cargo test --test font_accuracy` → fails.
- [ ] **Step 3:** Implement `estimate`; start with `K = 1.0`.
- [ ] **Step 4: Calibrate.** Run the suite with a temporary print of
      `median line height ÷ em_px` over all passing-size cases; set `K` to its
      median and tighten the thresholds only if a noise/photo case is `High`.
      Record the calibration table (min / median / max ratio per face) in a
      comment on `K`.
- [ ] **Step 5:** Suite passes with `K` and thresholds fixed.
- [ ] **Step 6: Commit** `feat(inspect): estimate font size from text lines`.

### Task 5: `rule.rs`, `inspect` command, JSON, CLI wiring

**Files:**

- Create: `src/rule.rs`, `src/inspect.rs`, `tests/inspect.rs`
- Modify: `src/lib.rs`, `src/cli.rs` (`--font-px`), `src/main.rs`,
  `tests/cli.rs` (the `inspect` row leaves `stubs_exit_2_with_tracking_link`;
  `fit` and `check` stay)

**Interfaces:**

- Consumes: `load::load`, `font::estimate`, `contract::*`.
- Produces:
  - `rule::Rule { Icon, Unchanged, Upscale, Oversize }` (serde `lowercase`),
    `rule::Plan { pub rule: Rule, pub factor: f32, pub output_width: u32 }`,
    `rule::plan(w: u32, h: u32, font_px: Option<f32>) -> Plan` (pure; alpha does
    not influence the rule in M1, so it is not a parameter).
  - `inspect::Report` (serde `Serialize`): `path`, `width`, `height`, `alpha`,
    `font { px: Option<f32>, confidence, lines, glyphs, source: "estimate" |
"override" }`,
    `rule`, `factor`, `output_width`.
  - `inspect::run(images: &[PathBuf], json: bool, font_px: Option<f32>, out:
&mut impl Write, err: &mut impl Write) -> i32`
    — returns `0`, or `2` if any file failed.
  - CLI: `--font-px <F>` accepts a finite `f32` > 0 only (clap `value_parser`).

`plan` rules, in order: `max(w,h) ≤ 64` → `Icon`, factor 1, width `w`. `w > 766`
→ `Oversize`, factor 1. Font `None` or `needed ≤ 1` → `Unchanged`, factor 1.
Otherwise `needed = min(766 ÷ w, 12 ÷ font)`; choose the smallest of {1.5, 2}
that is ≥ `needed` (2 when `needed > 2`); if `w × chosen > 766` fall back to
1.5; if `w × 1.5 > 766` → `Oversize`, factor 1; else `Upscale`.

- [ ] **Step 1: Failing unit tests for `plan`** (in `src/rule.rs`):
  - `icon_64_is_icon_65_is_not`: `(64, 10, None)` → `Icon`; `(65, 10, None)` is
    not.
  - `width_767_is_oversize_766_is_not`.
  - `font_none_never_upscales`: `(300, 200, None)` → `Unchanged`, factor 1.
  - `font_12_or_more_is_unchanged`: `(300, 200, Some(12.0))`.
  - `font_8_at_300_wide_gives_1_5x`: needed `min(2.55, 1.5)` = 1.5 → factor 1.5,
    `output_width == 450 + 34`. And `font_6_at_300_wide_gives_2x`: needed
    `min(2.55, 2)` = 2 → factor 2, width `600 + 34`.
  - `two_x_falls_back_to_1_5`: `(400, 200, Some(6.0))`: 2× = 800 > 766, 1.5× =
    600 → factor 1.5.
  - `even_1_5_overflows_is_oversize`: `(600, 200, Some(6.0))` → `Oversize`,
    factor 1.
- [ ] **Step 2:** Run → fails. **Step 3:** implement `plan`. **Step 4:** pass.
- [ ] **Step 5: Failing CLI tests** in `tests/inspect.rs` (generate images into
      a temp dir with `tests/support`):
  - `inspect_json_reports_all_fields`: DejaVu 14 px, 800 × ~400 canvas, `--json`
    → parsed JSON array of 1 with the keys above, `font.source == "estimate"`,
    `font.px` within ±1.5 of 14, `rule == "unchanged"`.
  - `font_px_override`: `--font-px 8` on a 300-wide image →
    `source ==
"override"`, `rule == "upscale"`, `factor == 1.5`.
  - `plain_output_has_one_line_per_image`: without `--json`, the line contains
    the path, `WxH`, and the rule.
  - Review Focus 2: `fully_transparent_png`: `alpha == true`, `font.px ==
null`,
    `confidence == "low"`, exit `0`.
  - Review Focus 3: `bad_file_among_good_ones`: args
    `good.png corrupt.png
missing.png a.emf good2.png` → exit `2`; stdout JSON
    has 2 entries; stderr names each of the 3 bad files.
  - Review Focus 5: `font_px_rejects_zero_negative_and_text`: `0`, `-3`, `abc` →
    exit `2`, stderr non-empty.
  - `needs_review_still_exits_zero`: noise image → exit `0`.
- [ ] **Step 6:** Run `cargo test --test inspect` → fails.
- [ ] **Step 7:** Implement `inspect::run` and wire `main.rs`. Flatten RGBA onto
      white before converting to greyscale. With `--font-px N`, `confidence` is
      `High`, `lines`/`glyphs` are `0` and `source` is `override`. Print JSON
      with `serde_json::to_writer_pretty`.
- [ ] **Step 8:** `cargo test --locked` → all pass.
- [ ] **Step 9: Commit** `feat(inspect): implement the inspect command`.

### Task 6: Docs and final gate

**Files:**

- Modify: `README.md` (status line: `inspect` implemented, `fit`/`check` still
  stubbed; one usage example with the JSON fields), `AGENTS.md` (module map in
  one short list)

- [ ] **Step 1:** Update both files; as-built facts only, no promises about M2+.
- [ ] **Step 2:** `just fmt`, then `just build` and `just verify` → both pass.
- [ ] **Step 3: Commit** `docs(docs): document the inspect command`.
- [ ] **Step 4:** Push the branch and open a non-draft PR to `main` (title
      `feat(inspect): M1 inspect command`). The PR body states that it carries
      the MVP design commits, so pixctl#3 can be closed once this PR merges. Ask
      the user before any `--admin` merge.
