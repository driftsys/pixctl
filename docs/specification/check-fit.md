# `check` and `fit` — requirements

Scope source: [pixctl#1](https://github.com/driftsys/pixctl/issues/1).
Implemented in the first version, which merges the MVP milestones M2 (`check`)
and M3 (`fit`, PNG output). Decisions:
[AD-0002](../decisions/0002-contract-constants.md),
[AD-0005](../decisions/0005-needs-review-not-failure.md) and
[AD-0012](../decisions/0012-check-gates-the-fitted-file.md) to
[AD-0015](../decisions/0015-deterministic-png-encoding.md). The display contract
and the rules are in [inspect.md](inspect.md).

## Intent

A docs author, or CI, can turn a raster image into a display-correct PNG (`fit`)
and gate committed images against the Markdown display contract (`check`).

## Contract version

- `pixctl --version` prints `pixctl <version> (display contract 1)`.
- The contract version is bumped whenever a contract constant changes. The JSON
  output of `inspect` does not carry it.

## `check`

`pixctl check <image>... [--target markdown] [--strict] [--font-px F] [--json]`

- `check` gates the image as committed, so it expects a fitted image with its
  frame included ([AD-0012](../decisions/0012-check-gates-the-fitted-file.md)).
- For each image, `check` runs four checks in this order:
  - `width`: pass when the width is at most 800 px.
  - `min-font-size`: the displayed font is `font × min(1, 800 ÷ width)`. Pass
    when it is at least 11 px, fail when it is below. The result is `REVIEW`
    when the estimate has low confidence and no `--font-px` is given.
  - `opaque`: fail when any pixel has alpha below 255.
  - `framed`: pass when the outer 1 px ring is exactly `#d0d7de` and the next 16
    px ring is exactly `#ffffff`. An image with a side below 35 px is never
    framed.
- An icon (longer side at most 64 px) passes `min-font-size` and `framed` with
  the detail `icon`.
- When the image is framed, the font estimate uses the content inside the frame.
  Otherwise it uses the whole image.
- `--target markdown` is the only target. `--font-px F` replaces the estimate.
  `--strict` turns every `REVIEW` into a failure for the exit code only; the
  line still reads `REVIEW`.
- Without `--json`, `check` prints one line per check:
  `TAG  [check] path  detail`, with `TAG` one of `PASS`, `FAIL` or `REVIEW`.
  With `--json`, it prints an array of `{path, check, status, detail}` objects,
  with the status in lowercase
  ([AD-0014](../decisions/0014-check-output-format.md)).
- An image that fails to load is reported on stderr with its path. The other
  images are still checked.
- The exit code is `2` when any image fails to load or the output cannot be
  written. Otherwise it is `1` when any check failed, or any check is `REVIEW`
  under `--strict`. Otherwise it is `0`.

## `fit`

`pixctl fit <image>... --out DIR [--font-px F]`

- `fit` writes `<DIR>/<file stem>.png` for each input and creates `DIR` when it
  is missing.
- `fit` flattens the image onto white. It then applies the rule from
  [inspect.md](inspect.md): `upscale` scales by 2× (nearest neighbour) or 1.5×
  (Lanczos3).
- Every rule except `icon` adds 16 px of white padding and a 1 px `#d0d7de`
  frame. `oversize` and `too-small` images are written at 1×. `fit` exits `0`
  for them; gating them is the job of `check`.
- `fit` prints one line per image: `FIT  [rule] path  out-path (W×H)`.
- `--font-px F` replaces the estimate. There is no `--format` flag until WebP
  output exists (M4).
- The output bytes are the same on every run for the same input and the same
  version ([AD-0015](../decisions/0015-deterministic-png-encoding.md)).
- An input that fails to load, an output that would overwrite an input, and an
  output name already written for another input in the same run are errors. Each
  is reported on stderr with its path, and the other images are still processed
  ([AD-0013](../decisions/0013-fit-output-naming.md)). The exit code is `2` if
  any image failed, and `0` otherwise.
- The output of `fit` passes the `width`, `opaque` and `framed` checks whenever
  the rule is `icon`, `unchanged` or `upscale`.

## Deferred

WebP output, EMF/WMF input, and the public corpus with `just eval` are
follow-ups and are not built.

## Alternatives considered

- Output with `PASS` and `FAIL` only, as in diagctl: rejected, see
  [AD-0014](../decisions/0014-check-output-format.md).
- `check` measuring the content width (at most 766 px): rejected, see
  [AD-0012](../decisions/0012-check-gates-the-fitted-file.md).
