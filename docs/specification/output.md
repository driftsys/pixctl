# Output convention — requirements

Scope source: [pixctl#1](https://github.com/driftsys/pixctl/issues/1). The
format matches tblctl AD-0005, and diagctl is asked to adopt it. It follows
[clig.dev](https://clig.dev). Decision:
[AD-0016](../decisions/0016-json-envelope-and-human-output.md), which mirrors
tblctl AD-0005. It applies to `inspect`, `check` and `fit`; the commands are in
[inspect.md](inspect.md) and [check-fit.md](check-fit.md).

## Intent

An agent or a script gets the results, the totals and the coded errors of a run
from one JSON document. A person gets a short text output.

## JSON envelope

- With `--json`, a command writes exactly one pretty-printed object to stdout,
  also when there are errors:
  `{"schema":"pixctl/<command>/v1","results":[…],"summary":{…},"errors":[…]}`.
  The keys come in this order.
- When every input fails, `results` is empty and the object is still written.
- A mixed run has both `results` and `errors`.

## Errors

- An error object has `code`, `message`, `file` when a file is involved, `hint`
  when there is one, and `with` for a `name_collision`. A field that does not
  apply is left out.
- Each error is also written to stderr as one line `pixctl: <message>`, with and
  without `--json`.
- The codes:

| Code               | Meaning                                                      |
| ------------------ | ------------------------------------------------------------ |
| `read_failed`      | The file cannot be opened or decoded, or has no file stem.   |
| `unsupported_type` | The file is EMF or WMF.                                      |
| `out_dir_failed`   | `fit` cannot create the `--out` directory.                   |
| `same_file`        | The `fit` output would overwrite an input.                   |
| `name_collision`   | Two inputs map to one output name; `with` is the earlier one |
| `write_failed`     | `fit` cannot write the output or a status line.              |

A dangling symbolic link at the `fit` target is also `write_failed`.

- The exit code is `2` when there is any error. Otherwise it is `1` when a check
  failed, or a review under `--strict`. Otherwise it is `0`.
- Argument errors from `clap` stay as `clap` text with exit `2` and no envelope.

## Human output

- Colour applies only to the `PASS`, `FAIL` and `REVIEW` tags of `check`: green,
  red and yellow. It is on when stdout is a terminal, or when `CLICOLOR_FORCE`
  is non-empty. A non-empty `NO_COLOR` turns it off and wins. The visible text
  is the same with and without colour. ANSI codes never appear in JSON.
- A count of one is written `1 image`.
- Every command's `--help` has an `Examples:` section. The top-level help ends
  with a link to the README.

## Non-goals

No change to the checks, the fit rule, the PNG encoding, the exit codes or the
text line format.

## Alternatives considered

- A pixctl-specific shape, a bare array, results on stdout with errors only on
  stderr, and NDJSON: rejected, see
  [AD-0016](../decisions/0016-json-envelope-and-human-output.md).
