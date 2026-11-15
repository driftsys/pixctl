# AD-0008 EMF/WMF input is deferred

## Context

Some document images are EMF or WMF vector files.

## Options

Alternatives not documented.

## Decision

In the MVP, EMF and WMF files are reported as unsupported (exit `2` for that
file; the other files are still processed). M4 adds an external converter when
one is installed. WebP output is not deferred: `fit --format webp` writes a
lossless WebP with the `image` crate, which needs no C code.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
