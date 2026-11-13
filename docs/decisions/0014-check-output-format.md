# AD-0014 `check` output matches diagctl, with a `REVIEW` tag

## Context

`check` and diagctl's check are read by the same people and CI jobs. diagctl
prints `TAG  [check] path  detail`. diagctl has only `PASS` and `FAIL`.

## Options

- Pure diagctl format (`PASS` and `FAIL` only). Rejected: a low-confidence font
  estimate would be reported as a pass or a fail, and both are wrong
  ([AD-0005](0005-needs-review-not-failure.md)).
- The diagctl line format with a third tag, `REVIEW`.

## Decision

`check` prints one line per check, `TAG  [check] path  detail`, with `TAG` one
of `PASS`, `FAIL` or `REVIEW`. `--strict` changes the exit code only; the tag
stays `REVIEW`. With `--json` it prints the envelope of
[AD-0016](0016-json-envelope-and-human-output.md): one result per image, with a
lowercase status and a nested array of its checks. That envelope replaces the
array of `{path, check, status, detail}` objects that this decision first set.

## Consequences

A consumer of the diagctl format must accept a third tag. The `fit` command uses
the same line shape with the tag `FIT`, written to stderr.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
