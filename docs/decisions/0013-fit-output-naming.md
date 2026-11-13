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

`fit` compares the names of the outputs of one run case-insensitively on every
file system. An output is an error (exit `2`) when its target is the same file
as an input, or equals an output already claimed in the same run. An existing
target is the same file as an input when both have the same identity, so a hard
link or a symbolic link to an input is refused. That comparison uses the
`same-file` crate and follows the file system, so on a case-sensitive file
system `Foo.PNG` and `Foo.png` are different files. A symbolic link to a missing
file is also refused. The error applies to that image only; the other images are
still processed, and the first writer keeps the file. The errors use the codes
`same_file`, `name_collision` (with the earlier input in `with`) and, for a
symbolic link to a missing file, `write_failed`
([AD-0016](0016-json-envelope-and-human-output.md)).

## Consequences

Two outputs that differ only in case are refused even on a case-sensitive file
system. The identity of an existing target is compared with every input, so a
link that points to an input is also refused.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)
