# AD-0005 `needs-review` is not a failure

## Context

The font estimate can have low confidence. An unknown font size must not break
CI when pixctl is first adopted.

## Options

Alternatives not documented.

## Decision

`needs-review` is the status that `check` (M2) produces for a `low` confidence.
`check` prints `REVIEW` and exits `0`; `--strict` turns REVIEW into FAIL (exit
`1`). `inspect` reports only the confidence and exits `0` for it.

## Consequences

`check` and `--strict` are not implemented yet (M2).

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
