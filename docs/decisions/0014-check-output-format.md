# AD-0014 `check` output matches diagctl, with a `REVIEW` tag

## Context

`check` and diagctl's check are read by the same people and CI jobs. diagctl
prints `TAG  [check] path  detail` and, with `--json`, an array. diagctl has
only `PASS` and `FAIL`.

## Options

- Pure diagctl format (`PASS` and `FAIL` only). Rejected: a low-confidence font
  estimate would be reported as a pass or a fail, and both are wrong
  ([AD-0005](0005-needs-review-not-failure.md)).
- The diagctl line format with a third tag, `REVIEW`.

## Decision

`check` prints one line per check, `TAG  [check] path  detail`, with `TAG` one
of `PASS`, `FAIL` or `REVIEW`. With `--json` it prints an array of
`{path, check, status, detail}` objects with a lowercase status. `--strict`
changes the exit code only; the tag stays `REVIEW`.

## Consequences

A consumer of the diagctl format must accept a third tag. The `fit` command uses
the same line shape with the tag `FIT`.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
