# pixctl M2 — `check` and `fit` (first version, v0.1.0)

Scope source: [pixctl#1](https://github.com/driftsys/pixctl/issues/1) and the
archived MVP design (`docs/archive/specs/2026-11-11-pixctl-mvp-design.md`). The
user asked for a first version, M1 `inspect` plus `check` and `fit`, released as
v0.1.0. The user delegated the open design calls to the agent on 2026-11-13.
Calls marked "(agent)" were made by the agent.

## Intent

A docs author, or CI, can turn a raster image into a display-correct PNG (`fit`)
and gate committed images against the Markdown display contract (`check`). This
merges the MVP's M2 (`check`) and M3 (`fit` PNG) into one release.

## Decisions taken by the user

1. Contract mechanism: hand-mirrored constants plus a reported
   `CONTRACT_VERSION` (diagctl#22 item 1).
2. Output matches diagctl: one line per check, `TAG  [check] path  detail`, and
   `--json` prints an array.

## Decisions (agent)

1. **REVIEW is kept.** diagctl has only PASS and FAIL. pixctl adds a third tag,
   `REVIEW`, in the same line format, for a low-confidence font estimate
   (AD-0005). `--strict` turns REVIEW into FAIL.
2. **`CONTRACT_VERSION = 1`** in `src/contract.rs`. `pixctl --version` prints
   `pixctl <ver> (display contract 1)`. The JSON outputs of `inspect` stay
   unchanged.
3. **`check` gates the file as committed**, so it expects a fitted image, frame
   included. Checks in this order:
   - `width`: PASS when the width is ≤ `COLUMN` (800 px). This resolves open
     item 3: a fitted image is content ≤ 766 px plus a 34 px border.
   - `min-font-size`: the displayed font is `font × min(1, 800 ÷ width)`. PASS
     when ≥ 11 px, FAIL when below. REVIEW when the estimate has low confidence
     and no `--font-px` is given.
   - `opaque`: FAIL when any pixel has alpha < 255.
   - `framed`: PASS when the outer 1 px ring is exactly `#d0d7de` and the next
     16 px ring is exactly `#ffffff`. The image must be at least 35 × 35 px.
   - Icons (both sides ≤ 64 px) pass `min-font-size` and `framed` with the
     detail `icon`.
   - The font estimate runs on the content inside the frame when the image is
     framed, and on the whole image otherwise.
4. **`check` flags:** `--target markdown` (the only target), `--strict`,
   `--font-px F`, `--json`. JSON: an array of objects
   `{path, check, status, detail}`, with the status in lowercase.
5. **`fit`** writes `<out>/<file stem>.png` for each input:
   - flatten onto white;
   - `upscale`: 2× nearest-neighbour, or 1.5× Lanczos3;
   - every rule except `icon` adds 16 px of white padding and a 1 px `#d0d7de`
     frame. `oversize` and `too-small` are written at 1×. `fit` prints one line
     per image, `FIT  [rule] path  out-path (W×H)`, and exits 0. Gating these
     cases is the job of `check`.
   - Encoding: `image` PNG encoder, then `oxipng` with a fixed preset, metadata
     stripped, and default features off (no multithreading), so the output is
     byte-identical across runs.
   - `--out DIR` is created when it is missing. If two inputs map to the same
     output name, or an output would overwrite an input, that image is an error
     (exit 2) and the rest are still processed.
   - `--font-px F` is supported.
   - `--format` is removed until WebP lands (M4). A flag that only errors is
     worse than no flag.
6. **Errors:** the same as `inspect`. A per-file error goes to stderr with the
   path, and the other files are still processed. The exit code is the maximum
   over the files (2 > 1 > 0).
7. **Deferred, unchanged from the MVP:** WebP output, EMF/WMF, and the real
   public corpus and `just eval`. These are recorded as follow-ups, not built.

## Approaches considered

- **Diagctl lines with a REVIEW tag (chosen).** One line format for both tools,
  and the uncertain case stays visible.
- **Pure diagctl (PASS/FAIL only).** Rejected: a low-confidence estimate would
  be reported as a PASS or a FAIL, and both are wrong.
- **`check` measures the content width (≤ 766), not the file width.** Rejected:
  a fitted image is 800 px wide, so its own output would fail.

## Non-goals and anti-goals

- No downscaling, cropping or OCR.
- `fit` must never produce bytes that differ between runs on the same input and
  version.
- `fit` output must pass `check` (`width`, `opaque` and `framed`) whenever the
  rule is `icon`, `unchanged` or `upscale`. An integration test pins this.

## Left to the builder

- The module split (`checks.rs`, `fit.rs`, `commands/`) follows the MVP table.
- The detail-string wording.
