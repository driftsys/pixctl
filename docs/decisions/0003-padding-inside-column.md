# AD-0003 Padding and frame count inside the column

## Context

`fit` adds 16 px padding and a 1 px frame on each side. The column is 800 px.

## Options

Alternatives not documented.

## Decision

The padding and frame count inside the column. The content width budget is
`800 − 2 × (16 + 1) = 766 px`.

## Consequences

`oversize` and the upscale limit use 766 px, not 800 px.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
