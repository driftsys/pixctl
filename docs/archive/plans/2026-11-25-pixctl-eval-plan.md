# pixctl `just eval` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `just eval DIR [MANIFEST]` measures the font estimate on a private
image folder and prints totals only: a label-free survey without a manifest,
accuracy against hand labels with one.

**Architecture:** A multi-file Cargo example, `examples/eval/`, that uses the
`pixctl` library (`load`, `font`, `rule`). Pure logic (manifest parsing,
metrics, survey counting, error text) lives in small modules with unit tests;
`main.rs` holds `run(args, out, err) -> i32` so the whole flow is testable
without spawning a process. The released binary does not change.

**Tech Stack:** Rust 2021, `toml` + `serde` (manifest), `rayon` (parallel
survey), both `[dev-dependencies]`; test images from `tests/support`.

**Spec:** `docs/wip/2026-11-25-pixctl-eval-design.md`

## Global Constraints

- No C dependencies. New crates are `[dev-dependencies]` only: `toml`, `rayon`.
  `serde` is already a dependency.
- The released `pixctl` binary does not change. `examples/` is added to the
  `exclude` list in `Cargo.toml`.
- Output never contains a file name, a path, image content, or a per-image line.
  Error text is fixed text plus an entry index; the text of a `Failure`, a TOML
  error or an I/O error is never printed (a TOML error prints only its line
  number).
- `DIR` inside the pixctl repository (canonical path under `CARGO_MANIFEST_DIR`)
  is refused with exit `2`.
- Exit contract: `0` clean, `1` limit not met, `2` operational error.
- Labelled limits: confidently wrong = confident and `|est − label| > 2.0`; must
  be below 2 %. `K` tolerance ±1.5 px (AD-0011). Label from points:
  `px = pt × 96 ÷ 72 × scale`.
- `just build` stays green (it runs `clippy --all-targets -D warnings`, which
  compiles the example). Conventional Commits with scopes from `.git-std.toml`
  (`inspect`, `docs`, `deps`, `repo`). Commit trailer:
  `Co-Authored-By: Claude <noreply@anthropic.com>`.
- Docs in plain, literal English.

## Review Focus

1. A manifest `file` that does not exist or cannot be decoded: exit `2` with
   `entry N: cannot read image`, and the path is not printed (Task 3 test
   `missing_image_error_names_the_index_only`).
2. A survey folder with non-image files, SVG and nested folders: SVG counted as
   skipped, other non-images ignored, nested folders walked (Task 4 test
   `survey_walks_nested_folders_and_skips_svg`).
3. Symbolic links in the survey folder: not followed, so a link loop cannot hang
   the run (Task 4 test `survey_does_not_follow_symlinks`, Unix only).
4. A survey folder with zero images: prints `images: 0` and exits `0`, no
   division by zero (Task 4 test `empty_survey_prints_zero`).
5. A relative `DIR` given from inside the repository (for example `.`): the
   guard uses the canonical path and refuses it (Task 3 test
   `dir_inside_the_repository_is_refused`).

---

### Task 1: Example scaffold and manifest parsing

**Files:**

- Modify: `Cargo.toml` (`[[example]]`, `[dev-dependencies]`, `exclude`)
- Create: `examples/eval/main.rs` (module declarations and a stub `main`)
- Create: `examples/eval/manifest.rs`
- Create: `examples/eval/error.rs`

**Interfaces:**

- Produces:
  - `error::EvalError` — enum with `Display` giving fixed text only:
    `BadEntry { index: usize, reason: &'static str }` →
    `entry {index}:
    {reason}`; `BadToml { line: Option<usize> }` →
    `manifest: invalid TOML at
    line {line}` (or `manifest: invalid TOML`);
    `EmptyManifest` → `manifest: no entries`; `CannotReadManifest` →
    `manifest: cannot be read`; `CannotReadImage { index: usize }` →
    `entry {index}: cannot read image`; `DirInsideRepository` →
    `DIR is inside the pixctl repository; keep
    private images outside it`;
    `CannotReadDir` → `DIR cannot be read`. Entry indexes are 1-based.
  - `manifest::Entry { file: PathBuf, label_px: f32 }`
  - `manifest::parse(text: &str) -> Result<Vec<Entry>, EvalError>`

- [ ] **Step 1: Add the example target and dependencies.** In `Cargo.toml`:
      `[[example]] name = "eval"`, `path = "examples/eval/main.rs"`,
      `test = true`; dev-dependencies `toml` and `rayon`; append `"examples"` to
      `exclude`.

- [ ] **Step 2: Write the failing tests in `examples/eval/manifest.rs`:**

```rust
#[test] fn px_entry_is_used_as_is()      // px = 13.0 -> label_px 13.0
#[test] fn pt_and_scale_give_px()        // pt = 9.0, scale = 1.25 -> 15.0
#[test] fn unknown_keys_are_allowed()    // source = "x" alongside px
#[test] fn px_with_pt_is_an_error()      // BadEntry { index: 1, .. }
#[test] fn pt_without_scale_is_an_error()
#[test] fn scale_without_pt_is_an_error()
#[test] fn non_positive_or_non_finite_is_an_error() // px = 0.0, px = -1.0, pt = nan
#[test] fn absolute_file_is_an_error()   // file = "/etc/x.png" (and "C:\\x.png" on Windows)
#[test] fn empty_manifest_is_an_error()  // "" -> EmptyManifest
#[test] fn bad_toml_reports_the_line_only() // the Display text contains "line 2" and not the bad line's text
#[test] fn second_entry_error_names_index_2()
```

- [ ] **Step 3: Run `cargo test --example eval`.** Expected: compile errors /
      failures for the missing items.

- [ ] **Step 4: Implement `EvalError` and `parse`.** Deserialize
      `{ image: Vec<RawEntry> }` with
      `RawEntry { file: String, px: Option<f32>,
  pt: Option<f32>, scale: Option<f32> }`
      and `#[serde(default)]` for the list; validate per entry; take the line
      from `toml::de::Error::span()` by counting newlines in `text` up to the
      span start.

- [ ] **Step 5: Run `cargo test --example eval` and `just build`.** Expected:
      all pass.

- [ ] **Step 6: Commit** `feat(inspect): parse the eval manifest`.

### Task 2: Accuracy metrics and the `K` interval

**Files:**

- Create: `examples/eval/metrics.rs`

**Interfaces:**

- Consumes: nothing from Task 1.
- Produces:
  - `metrics::Sample { label_px: f32, est_px: Option<f32>, confident: bool }`
  - `metrics::Accuracy { images: usize, confident: usize, mean: Option<f32>,
    p95: Option<f32>, wrong: usize }`
  - `metrics::accuracy(samples: &[Sample]) -> Accuracy`
  - `metrics::KInterval { used: usize, range: Option<(f32, f32)> }`
  - `metrics::k_interval(samples: &[Sample], k: f32) -> KInterval`
  - `metrics::report(a: &Accuracy, ki: &KInterval, k: f32) -> (String, bool)` —
    the four output lines of the design, and `true` when the limit is met.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test] fn error_is_absolute_on_raw_values()   // label 15, est 14.2 and 15.8 -> mean 0.8
#[test] fn wrong_is_strictly_above_two()       // est = label + 2.0 not wrong; + 2.01 wrong
#[test] fn review_results_are_excluded_from_error() // confident: false not in mean
#[test] fn p95_is_nearest_rank()               // errors 1..=20 (px) -> p95 = 19
#[test] fn no_confident_result_gives_none_and_fails() // mean None; report(...).1 == false; text has "n/a"
#[test] fn limit_is_below_two_percent()        // 1 wrong of 50 (2.0 %) fails; 1 of 51 passes
#[test] fn k_interval_is_the_intersection()    // two samples -> expected bounds to 3 decimals
#[test] fn k_interval_skips_samples_without_estimate()
#[test] fn empty_k_interval_prints_none()      // text has "none (no single K fits)"
#[test] fn report_says_inside_or_outside()     // current K 1.045 inside / outside
```

`k_interval` derives `h = est_px / k` per sample with an estimate (confident or
not) and intersects `[(label − 1.5)/h, (label + 1.5)/h]`.

- [ ] **Step 2: Run `cargo test --example eval`.** Expected: failures for the
      missing items.

- [ ] **Step 3: Implement the four functions.** The output lines follow the
      design's Output section; percentages with one decimal, px values with one
      decimal, `K` bounds with three.

- [ ] **Step 4: Run `cargo test --example eval` and `just build`.** Expected:
      all pass.

- [ ] **Step 5: Commit**
      `feat(inspect): compute eval accuracy and the K interval`.

### Task 3: Labelled mode end to end, the guard, and `just eval`

**Files:**

- Modify: `examples/eval/main.rs`
- Modify: `justfile` (recipe `eval`)

**Interfaces:**

- Consumes: `manifest::parse`,
  `metrics::{Sample, accuracy, k_interval,
  report}`, `error::EvalError`;
  `pixctl::load::{load, flatten_to_gray}`,
  `pixctl::font::{estimate, Confidence, K}`.
- Produces:
  `fn run(args: &[OsString], out: &mut impl Write, err: &mut impl
  Write) -> i32`;
  `fn inside_repository(dir: &Path) -> bool`. `main` calls `run` with
  `std::env::args_os().skip(1)` and exits with its code. Task 4 adds the survey
  branch to `run` when no manifest is given.

- [ ] **Step 1: Write the failing tests in `main.rs`.** Test images are rendered
      with `tests/support`
      (`#[path = "../../tests/support/mod.rs"]
  mod support;` inside
      `#[cfg(test)]`) and saved as PNG in a `TempDir` outside the repository;
      the manifest is written to the same `TempDir`.

```rust
#[test] fn labelled_run_prints_totals_and_exits_0()
    // 4 images (DejaVu 14 px, 3 lines) labelled px = 14.0; out contains
    // "images: 4" and "PASS"; out and err contain no "png" and no TempDir path
#[test] fn confidently_wrong_labels_exit_1()     // labels px = 30.0 -> exit 1, "FAIL"
#[test] fn missing_image_error_names_the_index_only()
    // entry 2 file = "nope.png" -> exit 2; err == "pixctl eval: entry 2: cannot read image\n"
#[test] fn bad_manifest_exits_2()
#[test] fn dir_inside_the_repository_is_refused()
    // DIR = CARGO_MANIFEST_DIR and DIR = "." from it -> exit 2, DirInsideRepository text
#[test] fn missing_dir_argument_prints_usage_and_exits_2()
```

- [ ] **Step 2: Run `cargo test --example eval`.** Expected: failures.

- [ ] **Step 3: Implement `run` and `inside_repository`.** Usage text:
      `usage: just eval DIR [MANIFEST]`. Each image: `load` → `flatten_to_gray`
      → `estimate`; confident = `Confidence::High` and `px.is_some()`. Errors
      print as `pixctl eval: {EvalError}`. `inside_repository` compares
      canonical paths with `starts_with`.

- [ ] **Step 4: Add the recipe.** `eval DIR *MANIFEST:` runs
      `cargo run --release --locked --example eval -- {{DIR}} {{MANIFEST}}`,
      with a one-line comment that it prints totals only.

- [ ] **Step 5: Run `cargo test --example eval` and `just build`.** Expected:
      all pass.

- [ ] **Step 6: Commit** `feat(inspect): add the labelled eval and just eval`.

### Task 4: Label-free survey mode

**Files:**

- Create: `examples/eval/survey.rs`
- Modify: `examples/eval/main.rs` (survey branch)

**Interfaces:**

- Consumes: `pixctl::load::{load, flatten_to_gray}`,
  `pixctl::font::{estimate, usable_px}`, `pixctl::rule::{plan, Rule}`.
- Produces:
  - `survey::collect(dir: &Path) -> Result<(Vec<PathBuf>, usize), EvalError>` —
    the image paths (extensions
    `png jpg jpeg gif bmp tif tiff webp emf
    wmf`, any case) and the count of
    skipped `.svg` files; walks recursively without following symbolic links.
  - `survey::Survey { images: usize, confident: usize, rules: [usize; 5],
    failed: BTreeMap<&'static str, usize>, skipped_svg: usize }`
  - `survey::run(paths: &[PathBuf], progress: &mut impl Write) -> Survey` —
    parallel with `rayon`; the rule uses
    `plan(w, h, usable_px(px,
    confidence))`, as `inspect` does; a load
    `Failure` adds 1 to `failed[failure.code]`.
  - `survey::report(s: &Survey) -> String`

- [ ] **Step 1: Write the failing tests:**

```rust
#[test] fn survey_walks_nested_folders_and_skips_svg()
    // a/b/x.png, a/y.PNG, z.svg, notes.txt -> 2 paths, skipped_svg 1
#[cfg(unix)] #[test] fn survey_does_not_follow_symlinks()   // link to parent dir
#[test] fn survey_counts_rules_and_failures()
    // one 14 px text image (confident), one 32x32 icon, one junk .png ->
    // images 3, confident 1, rules[icon] 1, failed["read_failed"] 1
#[test] fn empty_survey_prints_zero()        // report contains "images: 0"
#[test] fn report_has_no_paths()             // report text has no "png" and no "/"
#[test] fn run_without_manifest_surveys_and_exits_0() // via main::run
```

- [ ] **Step 2: Run `cargo test --example eval`.** Expected: failures.

- [ ] **Step 3: Implement `collect`, `run`, `report`, and the `main.rs`
      branch.** Report lines: `images: N (skipped SVG: S)`,
      `confident: C
  (x.x %)`, one line
      `rules: icon a, unchanged b, upscale c, too-small d,
  oversize e`, and
      `failed: read_failed n, unsupported_type m, ...` (or `failed: 0`).
      Progress: `\r{done}/{total}` to `err` every 100 images and at the end,
      then a newline.

- [ ] **Step 4: Run `cargo test --example eval` and `just build`.** Expected:
      all pass.

- [ ] **Step 5: Commit** `feat(inspect): add the label-free eval survey`.

### Task 5: Documentation

**Files:**

- Modify: `README.md` (new section "Evaluating the font estimate")
- Modify: `AGENTS.md` (the `just eval` recipe line, `examples/eval/` in the
  module list)
- Modify: `docs/specification/check-fit.md` (Deferred section: the public corpus
  is still not built; `just eval` is)

- [ ] **Step 1: Write the README section.** Both modes with their commands, the
      manifest format with the two label forms and the `pt → px` formula,
      example output from the design, the exit codes, the 2 % limit, the
      guarantee that output holds totals only, and the refusal of a `DIR` inside
      the repository.

- [ ] **Step 2: Update AGENTS.md and check-fit.md.**

- [ ] **Step 3: Run `just build`.** Expected: green (prim checks the Markdown).

- [ ] **Step 4: Commit** `docs(docs): describe just eval`.
