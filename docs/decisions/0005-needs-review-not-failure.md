# AD-0005 `needs-review` is not a failure

## Context

The font estimate can have low confidence. An unknown font size must not break
CI when pixctl is first adopted.

## Options

Alternatives not documented.

## Decision

The status that `check` produces for a `low` confidence is named `needs-review`
in this record and `review` in the output. `check` prints `REVIEW` (`review` in
JSON) and exits `0`; `--strict` turns REVIEW into FAIL (exit `1`). `inspect`
reports only the confidence and exits `0` for it. Under `--strict` the status in
JSON stays `review`; only the exit code changes.

## Consequences

`check` and `--strict` are implemented; the line format is in
[AD-0014](0014-check-output-format.md) and the JSON format in
[AD-0016](0016-json-envelope-and-human-output.md).

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
