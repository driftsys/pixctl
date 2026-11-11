# AD-0001 Font-size estimation with a pure-Rust heuristic

## Context

`inspect`, `check` and `fit` need the font size of the text in an image.

## Options

- An OCR model, for example `ocrs`.
- A pure-Rust heuristic over the heights of the text lines.

## Decision

The MVP uses a heuristic with no OCR model. The estimator returns a size plus a
confidence. `--font-px N` overrides the estimate for every input. `ocrs` stays a
later option behind the same `font::estimate` interface.

## Consequences

- A low confidence is reported, not hidden; the future `check` turns it into
  `needs-review` ([AD-0005](0005-needs-review-not-failure.md)).
- The algorithm is described in [design/inspect.md](../design/inspect.md).

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
