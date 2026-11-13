# AD-0002 Contract constants live in `src/contract.rs`

## Context

pixctl and diagctl share the same display contract: the column width and the
minimum displayed font.

## Options

Alternatives not documented beyond the choice below.

## Decision

The constants live in [`src/contract.rs`](../../src/contract.rs) and are
mirrored by hand with diagctl
([diagctl#22](https://github.com/driftsys/diagctl/issues/22), item 1).
`CONTRACT_VERSION` (currently `1`) is bumped whenever a constant changes, and
`pixctl --version` reports it as `(display contract 1)`. A unit test pins the
number in the version string to the constant.

The padding and the frame belong to pixctl only. Diagram SVGs from diagctl carry
no padding and no frame. Both tools apply one shared rule: the image the reader
sees is at most 800 px wide. The user decided this on 2026-11-13, and
[diagctl PR #31](https://github.com/driftsys/diagctl/pull/31) records it.

## Consequences

A change to a shared constant (the column width or the minimum font) must be
made in both repositories by hand, and the version bumped in both. Nothing is
shared as code between the repositories.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
