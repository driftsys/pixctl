# AD-0011 Calibration rule for `K`

## Context

The estimate is the median line height × `K`. The plan proposed taking `K` from
the median ratio, but that value fails the ±1.5 px acceptance on some fixtures.

## Options

- `K` from the median ratio of line height to em size (rejected: fails the
  acceptance).
- `K` from the interval of values that meets the acceptance.

## Decision

`K` is the midpoint of the interval of `K` values that keeps every exact-size
fixture within ±1.5 px of its em size, rounded to 3 decimals. On the synthetic
fixtures (DejaVu, Liberation and Noto Sans; 11, 14, 20 and 24 px; normal and
inverted) the interval is 1.0278–1.0625, so `K = 1.045`. The calibration table
is kept in the `K` doc comment in [`src/font.rs`](../../src/font.rs).

## Consequences

`K` is fitted to three synthetic faces. It must be rechecked on the M2 real
corpus and may need recalibration.

Satisfies: [AD-0001](0001-font-estimate-heuristic.md)
