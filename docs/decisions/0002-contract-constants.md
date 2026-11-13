# AD-0002 Contract constants live in `src/contract.rs`

## Context

pixctl and diagctl share the same display contract (column, font, padding,
frame).

## Options

Alternatives not documented beyond the choice below.

## Decision

The constants live in [`src/contract.rs`](../../src/contract.rs) and are
mirrored by hand with diagctl until
[diagctl#22](https://github.com/driftsys/diagctl/issues/22) chooses the shared
mechanism. A `CONTRACT_VERSION` constant is deferred until that choice too.

## Consequences

A change to the contract must be made in both repositories by hand.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
