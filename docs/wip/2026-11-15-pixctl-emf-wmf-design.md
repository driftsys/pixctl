# pixctl — EMF/WMF input through an external converter

Scope: [pixctl#9](https://github.com/driftsys/pixctl/issues/9), item 2. The
agent proposed this design on 2026-11-15. The user merged the previous PR
without changing it, which is the approval of record.

## Intent

Legacy requirement documents often embed EMF or WMF vector previews. pixctl
cannot decode these formats itself. When Inkscape or LibreOffice is installed,
pixctl uses it to convert the file to a PNG, then treats that PNG like any other
input. Without a converter, the behaviour stays as it is now.

## Approaches

- **Call an installed converter (chosen).** pixctl stays hermetic: it adds no C
  code, and only calls an external program when the user has one.
- A pure-Rust EMF/WMF renderer. Rejected: no mature crate exists, and writing
  one is out of scope.
- Require the user to convert the files first. Rejected: issue #1 asks for an
  optional converter.

## Decisions

- Conversion applies only to `.emf` and `.wmf` inputs, compared without regard
  to case. It runs inside `load::load`, so `inspect`, `check` and `fit` all
  accept these files.
- Converter lookup, first match wins:
  1. `PIXCTL_CONVERTER`, when it is set and non-empty: the path or name of a
     program. Its kind is chosen from its file name: a name that contains
     `inkscape` is Inkscape, and a name that contains `soffice` or `libreoffice`
     is LibreOffice. Any other name is an error (`convert_failed`) with a hint
     that names the two supported tools.
  2. `inkscape` on `PATH`.
  3. `soffice` on `PATH`, then `libreoffice` on `PATH`.
- Invocation, into a fresh temporary directory that is removed afterwards:
  - Inkscape: `inkscape <in> --export-type=png --export-filename=<tmp>/out.png`.
  - LibreOffice: `soffice --headless --convert-to png --outdir <tmp> <in>`. The
    output is `<tmp>/<stem>.png`.
- Time limit: 60 s per file. When the limit passes, pixctl kills the converter
  and reports `convert_failed`.
- Errors:
  - No converter found: `unsupported_type`, with the hint
    `install Inkscape or LibreOffice, or set PIXCTL_CONVERTER`.
  - The converter exits non-zero, times out, or writes no PNG: `convert_failed`
    (a new code), with the converter's name and exit status in the message.
- The JSON `path` stays the original `.emf` or `.wmf` path. The fit output name
  uses the original stem.
- Dependency: `tempfile` moves from `[dev-dependencies]` to `[dependencies]`. It
  is pure Rust, so it is safe for musl.
- CI: one new job on `ubuntu-latest` installs Inkscape with apt and runs the
  tests marked `#[ignore]` that need a real converter
  (`cargo test -- --ignored converter`).

## Non-goals

- No other vector formats (SVG belongs to diagctl).
- No DPI flag. The converter's default resolution is used.
- No conversion cache.
