# `check` and `fit` — design

Requirements: [specification/check-fit.md](../specification/check-fit.md).
Shared parts (loading, font estimate, rule): [inspect.md](inspect.md).

## Components

The pure logic is in `checks.rs` and `fit.rs`. The command modules follow the
pattern of `inspect::run`: the writers are injected, per-file errors are coded
`Failure` values that go to `err` and into the JSON envelope, and the function
returns the exit code. The shared types are in [output.md](output.md).

| Module                                 | Purpose                                                             |
| -------------------------------------- | ------------------------------------------------------------------- |
| [`checks.rs`](../../src/checks.rs)     | pure checks on one RGBA image                                       |
| [`check.rs`](../../src/check.rs)       | the `check` command: loads, runs the checks, prints lines or JSON   |
| [`fit.rs`](../../src/fit.rs)           | pure `render` and `encode_png`                                      |
| [`fit_cmd.rs`](../../src/fit_cmd.rs)   | the `fit` command: names, collision guards, writes files            |
| [`contract.rs`](../../src/contract.rs) | adds `CONTRACT_VERSION = 1`                                         |
| [`cli.rs`](../../src/cli.rs)           | `check` and `fit` flags; the version string carries the contract    |
| [`load.rs`](../../src/load.rs)         | `flatten_to_gray` moved here from `inspect.rs`, shared by all three |
| [`output.rs`](../../src/output.rs)     | envelope, `Failure`, colour and counts ([output.md](output.md))     |

Contract constants come only from `contract.rs`. A unit test in `cli.rs` pins
the number in the version string to `CONTRACT_VERSION`.

## Interfaces

- `checks::run_all(rgba, font_px) -> Vec<Outcome>` returns the outcomes in the
  order `width`, `min-font-size`, `opaque`, `framed`. `Outcome` holds `name`
  (serialised as `check`), `status` (`Pass`, `Fail`, `Review`; lowercase in
  JSON) and `detail`.
- `checks::is_framed(rgba) -> bool`.
- `check::run(images, strict, font_px, json, color, out, err) -> i32`.
- `fit::render(rgba, plan) -> RgbImage`: flatten onto white, scale, then pad and
  frame unless the rule is `icon`. Factor 2.0 uses nearest neighbour; any other
  factor uses Lanczos3. The target size is `round(w × f)` by `round(h × f)`.
- `fit::encode_png(img) -> Result<Vec<u8>>`
  ([AD-0015](../decisions/0015-deterministic-png-encoding.md)).
- `fit_cmd::run(images, out_dir, font_px, json, out, err) -> i32`.

## `check` flow

1. An image whose larger side is at most `ICON_MAX` skips `min-font-size` and
   `framed` (detail `icon`).
2. `is_framed` tests each pixel by its distance to the nearest edge: the ring
   must be the frame colour, and the band up to `PADDING + FRAME` must be white.
   The image must be at least `2 × (PADDING + FRAME) + 1` px on each side.
3. When framed, the font estimate runs on the crop inset by `PADDING + FRAME`.
   The displayed font uses the full image width.
4. The checks of one image become one `ImageResult` with the image status (the
   worst of its checks) and a `checks` array. The JSON `path` is written
   lossily, as in `inspect`.
5. The summary counts the results by image status. The text output prints one
   line per check, with the tag painted when `color` is set, then the summary
   line when at least one image loaded.
6. The exit code is `2` when there is a load failure or a write failure on
   stdout. Otherwise it is `1` when an image has status fail, or review under
   `--strict`, and `0` if not.

## `fit` flow

1. The font that drives the rule is the override, else a high-confidence
   estimate, else `None`, as in `inspect`.
2. The output directory is created once, before the first image. A failure is
   reported once as `out_dir_failed` and gives `2`.
3. An existing target is compared by file identity with every input, so a hard
   link or symbolic link to an input is detected; a symbolic link to a missing
   file is refused. The resolved name is compared case-insensitively against the
   outputs claimed so far ([AD-0013](../decisions/0013-fit-output-naming.md)). A
   match is an error for that image only (`same_file`, `name_collision` with the
   earlier input in `with`, or `write_failed` for a dangling link).
4. The image is rendered, encoded and written. A render, encode or write error
   is `write_failed`. Without `--json` the `FIT` line is written to `err`; with
   it, the image becomes a result `{path, out, rule, factor, width, height}`. A
   failed write of the `FIT` line is `write_failed`. A file name without a stem
   is `read_failed`.

## Testing

- Unit tests in `checks.rs` and `fit.rs` use synthetic images (width boundary,
  one transparent pixel, ring colour, small non-icon image, icon, override
  scaling, low confidence, nearest-neighbour mapping, alpha flattening,
  deterministic encoding without metadata chunks).
- `tests/check.rs` and `tests/fit.rs` cover the commands: exit codes, `REVIEW`
  with and without `--strict`, the JSON envelope, a missing file among valid
  ones, colour, JPEG input, output-name collisions, refusal to overwrite an
  input (also through a hard link, a symbolic link or a dangling link),
  byte-identical runs, an empty stdout for `fit` in text mode, and that `fit`
  output passes `check`. `tests/cli.rs` covers the version string and the flag
  shapes.
