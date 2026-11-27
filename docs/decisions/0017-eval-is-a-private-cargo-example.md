# AD-0017 The font-estimate evaluation is a private Cargo example

## Context

[AD-0011](0011-k-calibration-rule.md) fits `K` to synthetic text and asks for a
recheck on a real corpus. The real images are screenshots of engineering tools.
They cannot be committed, and their names and content must not leak.

## Options

- A Cargo example run by `just eval`, with the manifest parser as a
  `[dev-dependencies]` entry.
- A shell or Python script around `pixctl inspect --json`: rejected. It needs
  Python or `jq`, which a work machine or Windows may lack, and its arithmetic
  has no tests.
- A `pixctl eval` command: rejected. It ships evaluation code and a TOML parser
  in the released tool for a feature only a maintainer uses.

## Decision

The evaluation is the Cargo example `examples/eval/`, run by
`just eval DIR MANIFEST=""`. It is not part of the released binary. It prints
totals only, built from fixed text, and it refuses a `DIR` that is inside the
repository. It reports the estimate error and the `K` interval; changing `K` is
a separate change with its own record. Details are in
[design/eval.md](../design/eval.md).

## Consequences

Running the evaluation needs a Rust toolchain and a checkout. The release build
gains no dependency. A public corpus with a CI gate stays open as issue #9.

Satisfies: [pixctl#9](https://github.com/driftsys/pixctl/issues/9)
