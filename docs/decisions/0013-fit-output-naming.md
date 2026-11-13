# AD-0013 `fit` output naming and overwrite protection

## Context

`fit` writes `<out>/<file stem>.png`, so two inputs can map to one output, and
an output can equal an input. Output names must also stay valid when the
repository is checked out on case-insensitive file systems (macOS, Windows).

## Options

- Compare the real file names (exact case). Rejected: `A.png` and `a.png` are
  distinct on Linux but one file on macOS and Windows, so outputs would not be
  portable.
- Compare names case-insensitively, on every file system.

## Decision

`fit` compares output names case-insensitively on every file system. An output
is an error (exit `2`) when its resolved path equals an input path, or equals an
output already written in the same run. The error applies to that image only;
the other images are still processed, and the first writer keeps the file.

## Consequences

Two outputs that differ only in case are refused even on a case-sensitive file
system. An existing target is canonicalised first, so a link that points to an
input is also refused.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
