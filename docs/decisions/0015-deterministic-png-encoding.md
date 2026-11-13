# AD-0015 Deterministic PNG encoding with oxipng

## Context

`fit` outputs are committed to repositories. Output bytes that change between
runs, machines or thread counts produce noisy diffs.

## Options

Alternatives not documented beyond the settings below.

## Decision

`fit::encode_png` encodes with the `image` PNG encoder, then optimises with
`oxipng` 10.2.1 using preset 2, all metadata chunks stripped and no interlacing.
`oxipng` is built with default features off, which removes multithreading.

## Consequences

The same image and the same pixctl version give the same bytes. A change of the
`oxipng` version or preset can change the bytes, so it is a version-level change
of `fit`. A test pins byte identity across runs and the absence of `tEXt`,
`iTXt`, `zTXt` and `tIME` chunks.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
