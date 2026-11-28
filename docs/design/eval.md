# Font-estimate evaluation — design

Decision: [AD-0017](../decisions/0017-eval-is-a-private-cargo-example.md). The
rule `K` is checked against is in
[AD-0011](../decisions/0011-k-calibration-rule.md). User documentation is the
README section "Evaluating the font estimate".

## Purpose

The font estimate is tuned on synthetic text only. `just eval` measures it on a
folder of private images that cannot be committed, and prints totals that are
safe to share. It reports; it never changes the estimator or `K`.

## Components

`examples/eval/` is a Cargo example (`[[example]] name = "eval"` in
`Cargo.toml`, with `test = true`). It is not part of the released binary and is
excluded from the published package. `toml` and `rayon` are `[dev-dependencies]`
only, so the release build gains no dependency and no C code.

- `main.rs` — argument handling, the repository guard, the labelled mode and the
  exit codes.
- `error.rs` — `EvalError`, whose text is fixed.
- `manifest.rs` — parses the manifest into entries with a label in pixels.
- `metrics.rs` — error totals, the `K` interval and the four report lines.
- `survey.rs` — the unlabelled mode: collects files, measures them in parallel
  and prints the survey lines.

The recipe is `just eval DIR MANIFEST=""`. It runs
`cargo run --quiet --release --locked --manifest-path <repo>/Cargo.toml --example eval`
with `RUSTUP_TOOLCHAIN` set to the channel in `rust-toolchain.toml`, because the
recipe does not change into the repository (`[no-cd]`). It prints nothing apart
from the eval output. A relative `DIR` or `MANIFEST` resolves against the
caller's working directory.

## Modes

With a manifest argument the run is labelled. Without one it is a survey.

### Labelled mode

1. Read and parse the manifest. `file` entries are relative to `DIR`.
2. For each entry, `pixctl::load::load` decodes the image, then
   `pixctl::font::estimate` runs on `flatten_to_gray`.
3. A sample is confident when the confidence is `High` and a size exists. Every
   other sample counts as review, including an image without a detected line.
4. Print the four lines below.

Manifest: a TOML file with one `[[image]]` table per image. An entry has `file`
and one label form: `px`, or `pt` with `scale`, where
`px = pt × 96 ÷ 72 × scale`. `px` with `pt` or `scale`, `pt` alone, `scale`
alone, no label, a value that is not finite and above 0, or an absolute `file`
is an error that names the entry by its 1-based index. Unknown keys are ignored.
A manifest with no entry is an error.

### Survey mode

`survey::collect` walks `DIR` without following symbolic links. It reads the
extensions `png`, `jpg`, `jpeg`, `gif`, `bmp`, `tif`, `tiff`, `webp`, `emf` and
`wmf` in any case, and counts `.svg` files as skipped without reading them. A
subfolder that cannot be read is skipped; an unreadable `DIR` is an error.

`survey::run` measures the files in chunks of 100 with `rayon`. Per image it
loads, estimates and calls `rule::plan` with `font::usable_px`, as `inspect`
does. A load failure is counted under its error code. A panic in a decoder is
caught and counted under the code `panicked`. Progress goes to stderr as
`\r<done>/<total>`.

## Metrics

- Error is `|estimate − label|` on unrounded values, over confident samples.
- Confidently wrong is an error strictly above 2.0 px. The limit is met when
  `wrong × 100 < confident × 2`, in integers, so 1 of 50 fails.
- p95 uses the nearest-rank method: rank `ceil(0.95 × n)`.
- The `K` interval intersects `[(label − 1.5) ÷ h, (label + 1.5) ÷ h]` over
  every sample that has an estimate, confident or not, with `h = estimate ÷ K`.
  The tolerance of 1.5 px is the rule of AD-0011. The interval never changes the
  exit code.

## Output

Labelled mode, on stdout:

```text
images: 24 (confident 19, review 5 = 20.8 %)
error on confident: mean 0.6 px, p95 1.4 px
confidently wrong (> 2 px): 0 of 19 = 0.0 % (limit 2 %)  PASS
K interval for ±1.5 px on all labelled images (n = 24): 1.031–1.072 (current K 1.045, inside)
```

With no confident sample, the second line is `error on confident: n/a` and the
third is `confidently wrong (> 2 px): n/a (limit 2 %)  FAIL`. The `K` line ends
with `none (no single K fits)` for an empty intersection and
`none (no image has an estimate)` when `n = 0`. The place is `inside` or
`outside` the interval.

Survey mode, on stdout:

```text
images: 5 (skipped SVG: 1)
confident: 4 (80.0 %)
rules: icon 0, unchanged 4, upscale 0, too-small 0, oversize 0
failed: read_failed 1
```

`failed:` reads `0` when every image loaded.

## Privacy guarantees

- Only totals are printed. No file name, path, image content or per-image line.
- Every error is built by the eval as fixed text, with an entry index where one
  applies, for example `pixctl eval: entry 3: cannot read image`. The text of a
  `Failure`, a TOML error or an I/O error is never printed. A TOML error prints
  only its line number.
- Repository guard: before anything is read, `DIR` is canonicalized and refused
  when it is the repository root (`CARGO_MANIFEST_DIR`, canonicalized) or inside
  it, with
  `DIR is inside the pixctl repository; keep private images outside
  it`.
- The recipe is silent (`@`), so it does not echo the arguments.
- `main` installs an empty panic hook, because a panic message names a source
  path. Both modes catch panics with `survey::guarded` and report fixed text. In
  labelled mode a panic is `entry N: cannot read image`.

## Exit codes

- `0` — labelled: the confidently wrong rate is below 2 %. Survey: no
  operational error.
- `1` — labelled only: the limit is not met, or there is no confident sample.
- `2` — operational error: missing `DIR` argument (usage text), unreadable
  `DIR`, `DIR` inside the repository, unreadable or invalid manifest, or an
  image that cannot be read in labelled mode, or a report that cannot be
  written. In survey mode an image that fails to load is counted, not fatal.

## Testing

Unit tests in each module cover the manifest rules, the metrics, error
redaction, the guard, the survey walk (nested folders, SVG, symbolic links) and
the panic mapping. Images are rendered at test time through
`tests/support/mod.rs`; no real screenshot is committed.
