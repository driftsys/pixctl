# pixctl — JSON envelope and human output (clig.dev)

## Intent

On 2026-11-13 the user decided that tblctl, pixctl and diagctl share one output
convention. It is aligned with [clig.dev](https://clig.dev) and recorded in
tblctl as AD-0005 (`docs/decisions/0005-json-envelope-and-human-output.md`,
branch `feat/clig-output` in driftsys/tblctl). pixctl adopts it before the
v0.1.0 tag, so the first release does not ship the bare-array JSON. Agents and
scripts must get results, totals and coded errors from one JSON document.

## Approaches

- **Mirror tblctl AD-0005 in pixctl (chosen).** The three tools agree, and a
  consumer learns one shape.
- A pixctl-specific shape. Rejected: it defeats the cross-tool decision.
- NDJSON, or results on stdout with errors only on stderr. Rejected in tblctl
  AD-0005 for the reasons that record gives.

## Decisions

Fixed by the user (from tblctl AD-0005):

- With `--json`, every command (`inspect`, `check`, `fit`) writes exactly one
  pretty-printed object to stdout, also on errors:
  `{"schema":"pixctl/<command>/v1","results":[…],"summary":{…},"errors":[…]}`.
- Each error object has `code`, `message`, and `file` when a file is involved,
  plus `hint` when there is one. Each error is also written to stderr as one
  line `pixctl: <message>`, in JSON and in text mode.
- Exit codes are unchanged: 2 if there is any error, else 1 if any check failed
  (or a review under `--strict`), else 0.
- `clap` argument errors stay as `clap` text, with exit 2 and no envelope.
- Every command's `--help` has an `Examples:` section, and the top-level help
  ends with a link to the README.
- Colour only when stdout is a terminal: PASS green, FAIL red, REVIEW yellow. A
  non-empty `NO_COLOR` turns colour off. A non-empty `CLICOLOR_FORCE` turns it
  on when stdout is not a terminal. `NO_COLOR` wins. Alignment is the same with
  and without colour.
- `check` text output ends with a summary line. `fit` status lines go to stderr,
  so stdout stays empty in text mode.

pixctl specifics (agent calls, delegated by the user):

- Error codes: `read_failed` (the file cannot be opened or decoded),
  `unsupported_type` (EMF/WMF), `out_dir_failed` (`--out` cannot be created),
  `same_file` (the output would overwrite an input), `name_collision` (two
  inputs map to one output name, with a `with` field naming the earlier input)
  and `write_failed` (the output file cannot be written).
- `inspect`: `results` holds the current report objects, unchanged. `summary` is
  `{images, icon, unchanged, upscale, too_small, oversize}`. The text output
  ends with
  `N images: a icon, b unchanged, c upscale, d too-small,
  e oversize`.
- `check`: `results` holds one object per image,
  `{path, status, checks:[{check, status, detail}]}`. The image status is the
  worst of its checks, ordered fail > review > pass. `summary` is
  `{images, pass, fail, review}`, counted per image. The text output ends with
  `N images: P passed, F failed, R review`. The text lines keep the format
  `TAG  [check] path  detail`.
- `fit`: `results` holds one object per image written,
  `{path, out, rule, factor, width, height}`. `summary` is `{written}`. In text
  mode the `FIT  [rule] path  out (W×H)` line goes to stderr.
- One image is written `1 image`. When every input fails, the summary line is
  left out.
- Colour is hand-rolled with `std::io::IsTerminal`. No new dependency.

## Non-goals

- No change to the checks, the fit rule or the PNG encoding.
- No change to the exit codes or to the text line format.
