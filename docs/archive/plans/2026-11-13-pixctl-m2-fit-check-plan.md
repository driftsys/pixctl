# pixctl M2 `check` + `fit` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `pixctl check` and `pixctl fit` (PNG) and the reported
display-contract version, so pixctl can be released as v0.1.0.

**Architecture:** Pure logic in `src/checks.rs` and `src/fit.rs`. The command
glue is in `src/check.rs` and `src/fit_cmd.rs`, and follows the pattern of
`inspect::run` (writers injected, per-file errors to `err`, return the exit
code). Both commands reuse `load::load`, `font::estimate` and `rule::plan`.

**Tech Stack:** Rust 2021, `clap` derive, `image` 0.25, `imageproc`, `oxipng`
(new), `serde_json`, `assert_cmd`.

**Spec:** `docs/wip/2026-11-13-pixctl-m2-fit-check-design.md`

## Global Constraints

- Exit codes: `0` clean, `1` at least one check failed, `2` operational error. A
  run returns the maximum over its files.
- Line format: `TAG  [name] path  detail`, with two spaces after the tag and
  after the path. TAG is `PASS`, `FAIL`, `REVIEW` or `FIT`.
- Contract constants come only from `src/contract.rs`. Do not repeat the
  literals 800, 766, 16, 1, 11 or `#d0d7de` anywhere else.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` stay
  clean. Run `just build` before each commit.
- Commits: Conventional Commits with scopes from `.git-std.toml` (`check`,
  `fit`, `cli`, `docs`, `deps`). End each message with
  `Co-Authored-By: Claude <noreply@anthropic.com>`.
- Code comments and docs are in plain, literal English.

## Review Focus

1. A JPEG input to `fit` has no alpha. It must still produce a framed, opaque
   PNG. Pinned in Task 3 (`fit_jpeg_input`).
2. Two inputs with the same stem (`a.png`, `a.jpg`) in one `fit` run: the second
   is an error (exit 2), and the first file must not be overwritten. Pinned in
   Task 3.
3. `--out` equal to the input directory with a `.png` input of the same name:
   refused, exit 2, and the input stays unchanged. Pinned in Task 3.
4. An image smaller than 35 px but not an icon (for example 200 × 20): `framed`
   FAILs with a clear detail and does not panic. Pinned in Task 2.
5. `check` on its own `fit` output reports PASS for width, opaque and framed.
   Pinned in Task 3 (`fit_output_passes_check`).

---

### Task 1: Contract version and CLI shape

**Files:**

- Modify: `src/contract.rs`, `src/cli.rs`, `src/main.rs`, `tests/cli.rs`

**Interfaces:**

- Produces: `contract::CONTRACT_VERSION: u32 = 1`.
- Produces:
  `Command::Check { images: Vec<PathBuf>, target: Target, strict: bool, font_px: Option<f32>, json: bool }`.
- Produces:
  `Command::Fit { images: Vec<PathBuf>, out: PathBuf, font_px: Option<f32> }`.
  The `--format` flag and the `Format` enum are removed.

- [ ] **Step 1: Write the failing tests.** In `tests/cli.rs`, add
      `version_reports_contract`: `pixctl --version` stdout contains
      `(display contract 1)`. Add `fit_has_no_format_flag`:
      `pixctl fit a.png --out o --format png` exits 2, and its stderr contains
      `unexpected argument`. Add `check_accepts_strict_and_json`:
      `pixctl check missing.png --strict --json --font-px 12` exits 2 (missing
      file), and its stderr does not contain `unexpected argument`.
- [ ] **Step 2: Run** `cargo test --test cli`. Expected: the three new tests
      FAIL.
- [ ] **Step 3: Implement.** Add the constant with a doc comment that says it is
      bumped whenever a contract constant changes, and that diagctl mirrors it.
      In `cli.rs`, set
      `#[command(version = concat!(env!("CARGO_PKG_VERSION"), " (display contract 1)"))]`,
      and add a unit test asserting that the `1` in that string equals
      `CONTRACT_VERSION`. Give the new flags doc comments in the style of the
      existing ones. Remove "(not yet implemented)" from the two `about` lines.
      `main.rs` keeps the stubs for now.
- [ ] **Step 4: Run** `just build`. Expected: green.
- [ ] **Step 5: Commit**
      `feat(cli): report the contract version and shape the check and fit flags`.

### Task 2: `check`

**Files:**

- Create: `src/checks.rs` (pure), `src/check.rs` (command), `tests/check.rs`
- Modify: `src/lib.rs`, `src/main.rs`, `src/inspect.rs`, `src/load.rs`

**Interfaces:**

- Consumes: `load::load(&Path) -> Result<Loaded>`,
  `font::estimate(&GrayImage) -> FontEstimate`, and the Task 1 CLI.
- Produces: in `load.rs`, `pub fn flatten_to_gray(&RgbaImage) -> GrayImage`.
  Move it from `inspect.rs` and update `inspect` to call it.
- Produces: `checks::Status { Pass, Fail, Review }` (serde lowercase).
- Produces:
  `checks::Outcome { name: &'static str, status: Status, detail: String }`.
- Produces:
  `checks::run_all(rgba: &RgbaImage, font_px: Option<f32>) -> Vec<Outcome>`.
  `font_px` is the `--font-px` override. The four outcomes are in the order
  `width`, `min-font-size`, `opaque`, `framed`.
- Produces: `checks::is_framed(rgba: &RgbaImage) -> bool`.
- Produces:
  `check::run(images: &[PathBuf], strict: bool, font_px: Option<f32>, json: bool, out: &mut impl Write, err: &mut impl Write) -> i32`.

Semantics are in the spec, under "Decisions (agent)" 3 and 4. Rules for the
estimate:

- If `is_framed`, estimate on the inner crop: inset by `PADDING + FRAME` on each
  side.
- Displayed font = `px × min(1, COLUMN ÷ width)`, where `width` is the full
  image width.
- An icon is an image whose larger side is at most `ICON_MAX`.

- [ ] **Step 1: Write the failing unit tests** in `checks.rs`, on synthetic
      `RgbaImage`s:
  - `width_800_passes_801_fails`
  - `opaque_fails_on_one_transparent_pixel`
  - `framed_detects_exact_border`: build a 100 × 60 white image with a 1 px
    `FRAME_COLOUR` ring, and expect `true`.
  - `framed_rejects_wrong_ring_colour`: the same image with one ring pixel
    changed, and expect `false`.
  - `framed_fails_small_non_icon`: a 200 × 20 image gives `framed` FAIL with no
    panic.
  - `icon_skips_font_and_frame`: a 32 × 32 image gives PASS with detail `icon`
    for `min-font-size` and `framed`.
  - `font_override_scales_by_width`: a 1600 px wide image with `font_px` 20
    gives a displayed 10 px, so FAIL. With `font_px` 22 it gives 11 px, so PASS.
  - `low_confidence_is_review`: a blank 300 × 200 white image with no override
    gives `min-font-size` REVIEW.
- [ ] **Step 2: Run** `cargo test --lib checks`. Expected: FAIL (the module is
      missing).
- [ ] **Step 3: Implement** `checks.rs` and the `flatten_to_gray` move.
- [ ] **Step 4: Write the failing CLI tests** in `tests/check.rs`, using
      `assert_cmd` and fixture PNGs written to a `tempfile` dir:
  - `clean_framed_image_exits_0`: a framed 300 × 200 image with `--font-px 12`
    prints four lines that start with `PASS  [`, and exits 0.
  - `fail_exits_1`: an unframed 300 × 200 image prints `FAIL  [framed]` and
    exits 1.
  - `review_exits_0_strict_exits_1`: a blank framed image without `--font-px`
    exits 0 with `REVIEW  [min-font-size]`. With `--strict`, it exits 1, and the
    line still reads `REVIEW`.
  - `json_is_array_of_outcomes`: `--json` parses as an array whose items have
    the keys `path`, `check`, `status` and `detail`, and the status is
    lowercase.
  - `missing_file_exits_2_others_reported`: one valid image and one missing path
    exit 2, and stdout still holds the lines for the valid image.
- [ ] **Step 5: Implement** `check.rs` and wire `main.rs`. In JSON, the
      serialised key is `check` (rename the `name` field). The path is lossy, as
      in `inspect`. Exit code: 2 if any file errored or a write failed;
      otherwise 1 if any FAIL, or any REVIEW under `--strict`; otherwise 0.
- [ ] **Step 6: Run** `just build`. Expected: green.
- [ ] **Step 7: Commit**
      `feat(check): gate width, font size, opacity and frame`.

### Task 3: `fit` (PNG)

**Files:**

- Create: `src/fit.rs` (pure), `src/fit_cmd.rs` (command), `tests/fit.rs`
- Modify: `src/lib.rs`, `src/main.rs`, `Cargo.toml`, `Cargo.lock`

**Interfaces:**

- Consumes: `rule::plan`, `load::load`, `load::flatten_to_gray`,
  `font::estimate`, and `checks::run_all` (in tests only).
- Produces: `fit::render(rgba: &RgbaImage, plan: &rule::Plan) -> RgbImage`:
  flatten onto white, scale, then pad and frame unless the rule is `Icon`.
- Produces: `fit::encode_png(img: &RgbImage) -> anyhow::Result<Vec<u8>>`:
  `image` PNG encoder, then `oxipng::optimize_from_memory` with
  `Options::from_preset(2)`, `strip = StripChunks::All` and an interlace of
  `None`.
- Produces:
  `fit_cmd::run(images: &[PathBuf], out_dir: &Path, font_px: Option<f32>, out: &mut impl Write, err: &mut impl Write) -> i32`.

Dependency: `oxipng = { version = "9", default-features = false }`. Check the
current major version on crates.io first. Disabling the default features removes
the `parallel` and binary features. The font that drives the rule is chosen the
same way as in `inspect`: the override, else the estimate if its confidence is
high, else `None`.

Scaling: factor 2.0 uses `imageops::resize` with `FilterType::Nearest`. Factor
1.5 uses `FilterType::Lanczos3`. The target size is `round(w × f)` ×
`round(h × f)`.

- [ ] **Step 1: Write the failing unit tests** in `fit.rs`:
  - `unchanged_adds_border`: a 300 × 200 image becomes 334 × 234, the pixel
    (0,0) is `FRAME_COLOUR`, and the pixel (1,1) is white.
  - `upscale_2x_nearest`: a 300 × 200 image at factor 2 becomes 634 × 434. A
    black source pixel at (0,0) maps to black at (17,17), (18,17), (17,18) and
    (18,18).
  - `icon_only_flattens`: a 32 × 32 image with a transparent pixel becomes 32 ×
    32, and that pixel is white.
  - `alpha_flattens_onto_white`: a half-transparent black pixel gives a value of
    127 or 128.
  - `encode_is_deterministic`: encoding the same image twice gives equal bytes.
    The bytes decode back to the same pixels and contain no `tEXt`, `iTXt`,
    `zTXt` or `tIME` chunk.
- [ ] **Step 2: Run** `cargo test --lib fit`. Expected: FAIL.
- [ ] **Step 3: Implement** `fit.rs` and add the dependency.
- [ ] **Step 4: Write the failing CLI tests** in `tests/fit.rs`:
  - `fit_writes_png_and_line`: the output is `<out>/<stem>.png`, stdout matches
    `FIT  [unchanged] <path>  <out-path> (334×234)`, and the exit code is 0.
  - `fit_creates_out_dir`.
  - `fit_jpeg_input`: the input is a JPEG and the output is an opaque PNG.
  - `fit_twice_is_byte_identical`: two runs into two directories give equal
    bytes.
  - `same_stem_collision_exits_2`: with inputs `a.png` and `a.jpg`, the output
    `a.png` comes from the first input, and the exit code is 2.
  - `refuses_to_overwrite_input`: `--out` equals the input's directory, the
    input bytes are unchanged, and the exit code is 2.
  - `fit_output_passes_check`: for an unchanged image and an upscaled image
    (`--font-px 6` on 300 × 200), `checks::run_all` on the output gives PASS for
    `width`, `opaque` and `framed`.
- [ ] **Step 5: Implement** `fit_cmd.rs` and wire `main.rs`. Compare canonical
      paths to detect an overwrite of an input. Keep a set of the output paths
      written so far to detect a collision.
- [ ] **Step 6: Run** `just build`. Expected: green.
- [ ] **Step 7: Commit**
      `feat(fit): write padded, framed PNGs following the fit rule` (plus
      `build(deps): add oxipng` if it is a separate commit).

### Task 4: User docs

**Files:**

- Modify: `README.md`, `AGENTS.md`

- [ ] **Step 1:** README: remove the "stubbed" status. Document `check` (flags,
      the four checks, the line format, the tags including REVIEW, `--strict`,
      the JSON shape, exit codes) and `fit` (`--out`, naming, rules, the `FIT`
      line, determinism), and say that `--version` reports the contract version.
      Every example must be produced by the built binary: run it and paste the
      real output.
- [ ] **Step 2:** AGENTS.md: update the module list (`checks.rs`, `check.rs`,
      `fit.rs`, `fit_cmd.rs`), and say that `main.rs` no longer has stubs.
- [ ] **Step 3: Run** `just build`. Expected: green.
- [ ] **Step 4: Commit** `docs(docs): document check and fit`.
