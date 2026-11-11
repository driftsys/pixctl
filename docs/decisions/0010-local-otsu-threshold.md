# AD-0010 Local, overflow-safe Otsu threshold

## Context

`imageproc::contrast::otsu_level` overflows its pixel counts on large images,
which panics in debug and test builds. Large white screenshots are a normal
input for pixctl.

## Options

- Keep `imageproc::otsu_level`.
- A local Otsu implementation over a `u64` histogram.

## Decision

`font.rs` uses a local Otsu threshold that is overflow-safe. A test checks that
it matches `imageproc` on normal images and handles counts above `u32`.

Satisfies: [AD-0001](0001-font-estimate-heuristic.md)
