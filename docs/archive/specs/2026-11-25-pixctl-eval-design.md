# pixctl font-estimate evaluation (`just eval`)

## Intent

The font-size estimate drives every `upscale`, `too-small` and `REVIEW` result,
but it is tuned only on synthetic text (AD-0011: `K = 1.045`, fitted to DejaVu,
Liberation and Noto Sans rendered by the tests). The images pixctl will see are
screenshots of engineering tools from the user's work. Those images cannot be
committed. This work gives the user numbers that say whether the estimate can be
trusted on those images, and whether `K` must change.

## Goals

- `just eval DIR` without a manifest runs the label-free mode: it walks `DIR`
  recursively, runs `inspect` logic on every image, and prints totals only: the
  number of images, the share with a confident estimate, the share in each rule
  (`icon`, `unchanged`, `upscale`, `too-small`, `oversize`), and the number that
  fail to load, by error code. It tells how often pixctl would ask for review or
  flag an image across thousands of real images, not whether the estimate is
  right. The user's corpus is about 37,000 doc attachments (PNG, JPEG, GIF, EMF,
  WMF; SVG is out of scope and is counted as skipped).

- `just eval DIR MANIFEST` measures the estimate on a labelled sample (about 30
  to 50 images, labelled by hand) and prints totals only. `file` entries are
  relative to `DIR`, so the manifest can live outside the image repository.
- The totals are the metrics from the MVP design: mean and p95 absolute error on
  confident results, the share of results that are not confident (sent to
  review), and the confidently wrong rate (confident and off by more than 2 px),
  which must stay below 2 %.
- The eval also prints the interval of `K` values that would keep every labelled
  image within ±1.5 px (the AD-0011 rule), so that `K` can be rechecked on real
  images.

## Non-goals

- No change to the estimator or to `K` in this work. The eval reports; a
  recalibration, if the numbers ask for one, is a separate change with its own
  decision record.
- No `pixctl eval` command. The released binary does not change.
- No labelling tool or GUI.
- No public corpus in this work (the user's choice). A public corpus in
  `tests/fixtures/real/` with a CI gate is a later change; issue #9 keeps it
  open.

## Anti-goals

- The eval must never print a file name, a path, image content or any per-image
  line for a private folder. The output must be safe to paste into a public
  issue.
- The private folder must never enter the repository. The eval refuses a `DIR`
  that is inside the git work tree of the repository (exit 2), so the images
  cannot be committed by accident.
- Labels must not come from pixctl's own estimates. Reviewing the tool's guesses
  biases the confidently-wrong rate.

## Constraints

- No C dependencies (the musl release build). The TOML parser is a
  `[dev-dependencies]` entry only.
- Runs on macOS, Linux and Windows wherever the repository builds.
- CI runs only `just` recipes.
- Docs in plain, literal English.

## Approaches

1. **Rust example, called by `just eval DIR` (chosen).** `examples/eval.rs`
   reads `DIR/manifest.toml`, decodes each image with `pixctl::load`, calls
   `pixctl::font::estimate`, and prints the totals. Testable, cross-platform, no
   change to the shipped binary. Cost: a work machine needs a Rust toolchain and
   a checkout.
2. A shell or Python script around `pixctl inspect --json`: rejected. It adds
   Python or `jq` as a requirement that a work machine or Windows may not have,
   and its arithmetic has no tests.
3. A hidden `pixctl eval` command: rejected. It ships evaluation code and a TOML
   parser in the released tool for a feature only the maintainer uses.

The user picked approach 1.

## Labels (decided)

Each image gets a true em size in pixels, from one of two sources:

- **Known render settings**, for screenshots taken on purpose: the UI font size
  in points and the display scale. The eval computes
  `px = pt × 96 ÷ 72 × scale`. Example: 9 pt at 125 % is 15 px.
- **Measured by hand**, for existing screenshots: the em size in pixels.

## Manifest

`MANIFEST`, a TOML file given as the second argument, one table per image:

```toml
[[image]]
file = "editor-dark.png"
pt = 9.0
scale = 1.25

[[image]]
file = "old-dialog.png"
px = 13.0
```

- Exactly one of `px`, or `pt` with `scale`, per entry. Anything else is an
  error that names the entry by its index, not its file name.
- `file` is relative to `DIR`. An absolute path is an error.
- `px`, `pt` and `scale` must be finite and above 0. `px` together with `pt` or
  `scale`, `pt` without `scale`, or `scale` without `pt` is an error.
- Unknown keys are allowed.
- A manifest with no entries is an error (exit 2).

## Metrics (builder-check answers, the designer's calls)

- Error is `|estimate − label|` on the unrounded values. Confidently wrong is an
  error strictly above 2.0 px.
- Confident means `Confidence::High` with a `px` value. Every other result,
  including no detected line, counts as review. Review share = review ÷ total.
- p95 uses the nearest-rank method.
- With no confident result, the error and rate lines print `n/a` and the exit
  code is `1`: the run cannot show that the limit is met.
- The `K` interval is the intersection of
  `[(label − 1.5) ÷ h, (label + 1.5) ÷
  h]` over images with a detected line,
  where `h = px ÷ K`. The tolerance is fixed at 1.5 px (AD-0011). An empty
  intersection prints `none (no single K
  fits)`. The line also prints how many
  images it used. It never changes the exit code.

## Output

Totals only, for example:

```text
images: 24 (confident 19, review 5 = 20.8 %)
error on confident: mean 0.6 px, p95 1.4 px
confidently wrong (> 2 px): 0 of 19 = 0.0 % (limit 2 %)  PASS
K interval for ±1.5 px on all labelled images: 1.031–1.072 (current K 1.045, inside)
```

- The `K` interval uses every image that has at least one detected line,
  confident or not, from `px ÷ K` (the median line height).
- Exit `0` when the confidently wrong rate is below 2 %, `1` when it is not, `2`
  for an operational error (missing manifest, unreadable image, bad entry). This
  is the repository's exit contract.
- `--json` is not needed.
- Every error message is built by the eval and names an entry by its index and a
  fixed kind (for example `entry 3: cannot decode image`). The text of a
  `pixctl::output::Failure`, of a TOML parse error, or of an I/O error is never
  printed, because it can contain a path or a manifest line. A TOML error prints
  only its line number.

## Left to the builder

- Which TOML crate (`toml` is the default choice; it is pure Rust).
- How the example shares decoding with the crate (`pixctl::load::load` and
  `pixctl::load::flatten_to_gray` are already public through `src/lib.rs`).
- `just eval DIR [MANIFEST]`, with `DIR` required.
- The label-free mode uses the same rule logic as `inspect` (`pixctl::rule`),
  with `--font-px` unsupported. Its exit code is `0` unless an operational error
  stops the run; images that fail to load are counted, not fatal.
- The label-free mode should process images in parallel if that is cheap; 37,000
  images including EMF/WMF conversions must finish in a reasonable time.
  Progress goes to stderr as a count only.

## Packaging (the designer's calls)

- `examples/eval.rs` is excluded from the published crate package. It must pass
  `clippy --all-targets -D warnings`, because `just build` compiles it.
- Unit tests cover the manifest rules, the metrics and the redaction of errors,
  on small images rendered at test time (the existing synthetic approach). No
  real screenshot is committed.

## Trace

Satisfies: issue #9 (`just eval DIR`; the public corpus stays open); AD-0011
consequence (recheck `K` on a real corpus).
