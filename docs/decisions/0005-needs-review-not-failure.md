# AD-0005 `needs-review` is not a failure

## Context

The font estimate can have low confidence. An unknown font size must not break
CI when pixctl is first adopted.

## Options

Alternatives not documented.

## Decision

`needs-review` is the status that `check` produces for a `low` confidence.
`check` prints `REVIEW` and exits `0`; `--strict` turns REVIEW into FAIL (exit
`1`). `inspect` reports only the confidence and exits `0` for it.

## Consequences

`check` and `--strict` are implemented; the output format is in
[AD-0014](0014-check-output-format.md).

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
