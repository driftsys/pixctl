# AD-0004 Upscale rule

## Context

Images with small text must be upscaled so the text is readable, without
exceeding the 766 px content width ([AD-0003](0003-padding-inside-column.md)).

## Options

Alternatives not documented beyond the trigger below. The plan first sized the
trigger on the 12 px target; it was changed to the 11 px minimum so that a font
in [11, 12) px stays `unchanged`.

## Decision

- An upscale is considered only when the font is below 11 px and the font is
  known with high confidence or set by `--font-px`. A low-confidence or unknown
  font never upscales; the rule then comes from the size alone: `icon` uses the
  larger of width and height, `oversize` uses the width.
- The needed factor is `min(766 ÷ w, 12 ÷ font)`. The tool picks the smallest of
  {1.5, 2} that is at least the needed factor, capped at 2, and drops to 1.5
  when 2 would exceed 766 px.
- If 1.5 also exceeds 766 px, or the chosen factor leaves the font below 11 px,
  there is no upscale and the rule is `too-small`.
- `oversize` means only that the width is above 766 px. `plan` returns
  `oversize` before it looks at the font, so an oversize image with a font below
  11 px is not flagged by `inspect`. Issue #1 asks to flag it; that flag is
  deferred to `check`.
- 2× uses nearest-neighbour and 1.5× uses Lanczos3 (applied by `fit`, M3).

## Consequences

Implemented in [`src/rule.rs`](../../src/rule.rs).

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
