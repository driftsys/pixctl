# AD-0016 One JSON envelope, coded errors and clig.dev human output

## Context

The user decided on 2026-11-13 that tblctl, pixctl and diagctl share one output
convention, aligned with [clig.dev](https://clig.dev). It is recorded in tblctl
as AD-0005 (driftsys/tblctl,
`docs/decisions/0005-json-envelope-and-human-output.md`). pixctl adopts it
before the v0.1.0 tag, so the first release does not ship the bare-array JSON of
[AD-0014](0014-check-output-format.md). Agents and scripts need results, totals
and coded errors from one JSON document.

## Options

- Mirror tblctl AD-0005 in pixctl. A consumer learns one shape for the three
  tools.
- A pixctl-specific shape. Rejected: it defeats the cross-tool decision.
- A bare array, results on stdout with errors only on stderr, or NDJSON.
  Rejected in tblctl AD-0005 for the reasons that record gives.

## Decision

pixctl mirrors tblctl AD-0005.

- With `--json`, `inspect`, `check` and `fit` each write exactly one
  pretty-printed object to stdout, also when there are errors:
  `{"schema":"pixctl/<command>/v1","results":[…],"summary":{…},"errors":[…]}`.
  When every input fails, `results` is empty and the object is still written.
- An error object has `code` and `message`, plus `file`, `hint` and `with` when
  they apply. Each error is also written to stderr as `pixctl: <message>`, in
  JSON mode and in text mode.
- The error codes are `read_failed`, `unsupported_type`, `out_dir_failed`,
  `same_file`, `name_collision` (`with` names the earlier input) and
  `write_failed`. Three calls fix edge cases: a dangling symbolic link at the
  `fit` target is `write_failed`; an input without a file stem is `read_failed`;
  a failed write of a `FIT` status line is `write_failed`.
- `check` results are one object per image with a nested `checks` array. The
  image status is the worst of its checks (fail over review over pass). Under
  `--strict` the status stays `review` and only the exit code changes.
- `summary` counts images that produced a result: `inspect` by rule, `check` by
  image status, `fit` as `written`. The text output of `inspect` and `check`
  ends with a summary line, which is left out when no image loaded.
- `fit` writes its `FIT` lines to stderr, so stdout is empty in text mode.
- Colour applies to the `check` tags only when stdout is a terminal, or when
  `CLICOLOR_FORCE` is non-empty. A non-empty `NO_COLOR` turns it off and wins.
  Colour is hand-rolled with `std::io::IsTerminal`; no dependency is added.
- Exit codes are unchanged. `clap` argument errors stay as `clap` text with exit
  `2` and no envelope.
- Each command's `--help` has an `Examples:` section, and the top-level help
  links to the README.

## Consequences

Consumers of the earlier bare array, and of `FIT` lines on stdout, must change.
Any change to the envelope is made together with the other two tools.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1), tblctl
AD-0005
