# AD-0002 Contract constants live in `src/contract.rs`

## Context

pixctl and diagctl share the same display contract (column, font, padding,
frame).

## Options

Alternatives not documented beyond the choice below.

## Decision

The constants live in [`src/contract.rs`](../../src/contract.rs) and are
mirrored by hand with diagctl
([diagctl#22](https://github.com/driftsys/diagctl/issues/22), item 1).
`CONTRACT_VERSION` (currently `1`) is bumped whenever a constant changes, and
`pixctl --version` reports it as `(display contract 1)`. A unit test pins the
number in the version string to the constant.

## Consequences

A change to the contract must be made in both repositories by hand, and the
version bumped in both.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
