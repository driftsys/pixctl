# EMF/WMF conversion — design

Requirements: [specification/inspect.md](../specification/inspect.md) (input
formats). Decision: [AD-0008](../decisions/0008-emf-wmf-external-converter.md).
The call site, `load::load`, is in [inspect.md](inspect.md). Error codes are in
[output.md](output.md).

## Components

`src/convert.rs` finds an installed converter and runs it. It has no state and
reads no environment itself: `load::load` passes in the values of
`PIXCTL_CONVERTER`, `PATH` and `PIXCTL_CONVERT_TIMEOUT_SECS`, so the lookup is
testable with plain parameters.

## Interfaces

- `convert::Converter { kind: Kind, program: PathBuf }`, with
  `Kind { Inkscape, LibreOffice }`.
- `convert::find_in(explicit, path_var) -> Result<Option<Converter>, Failure>`.
  `Ok(None)` means that no converter is installed. A non-empty `explicit` value
  wins; its kind comes from its file name (case-insensitive: `inkscape`, or
  `soffice`/`libreoffice`), and any other name is `convert_failed` with a hint.
  An empty value falls back to `PATH`, searched for `inkscape`, `soffice`, then
  `libreoffice`. Only executable files match.
- `convert::to_png(&Converter, input, limit) -> Result<DynamicImage, Failure>`.
- `convert::parse_timeout(Option<&str>) -> Duration`. A missing value, or one
  that is not a positive integer, gives `DEFAULT_TIMEOUT_SECS` (60).
- `convert::MISSING_HINT` is the hint of `unsupported_type`:
  `install Inkscape or LibreOffice, or set PIXCTL_CONVERTER`.

## Conversion

1. `load::load` converts only paths that end in `.emf` or `.wmf`, in any case.
2. `to_png` creates a temporary directory with the prefix `pixctl-convert`. The
   directory is removed on every path, success or failure.
3. A relative input path is prefixed with `./`, so a name that starts with `-`
   is not read as an option.
4. The invocation depends on the kind:
   - Inkscape: `<in> --export-type=png --export-filename=<tmp>/out.png`.
   - LibreOffice: `--headless --convert-to png --outdir <tmp> <in>`. The output
     is `<tmp>/<stem>.png`.
5. The converter's stdin, stdout and stderr are null.
6. pixctl polls `try_wait` every 50 ms. When the limit passes, it kills the
   converter and reports `convert_failed`. On Unix the converter starts in its
   own process group, and the whole group is killed with the `kill` utility
   (`/bin/kill` or `/usr/bin/kill`), so no `libc` dependency is needed. Without
   `kill`, a child process of the converter can outlive the limit. Other
   platforms kill only the converter process.
7. The result is `convert_failed`, with the program file name in the message,
   when the converter cannot start, exits non-zero, times out, writes no file,
   or writes a file that does not decode as an image.
8. The decoded image then follows the normal RGBA path. The JSON `path` stays
   the original `.emf` or `.wmf` path, and `fit` names its output from the
   original stem. `Loaded.converter` carries the program file name, which
   `inspect` prints ([inspect.md](inspect.md)).

## Dependencies

`tempfile` is a normal dependency (pure Rust, safe for musl). No C code is
added.

## Testing

- Unit tests in `src/convert.rs` (Unix): lookup with each kind, an unknown name,
  an empty variable, `PATH` order, empty `PATH` entries, a non-executable file,
  and timeout parsing.
- `tests/convert.rs` uses fake converter scripts (Unix) for the success,
  non-zero exit, no output, undecodable output, timeout, no converter, process
  group kill, argument safety and temporary directory cleanup cases.
- `converter_real_inkscape_converts_a_wmf` is `#[ignore]`d and needs Inkscape.
  `just test-converter` runs it and fails when it does not run; the `converter`
  CI job installs Inkscape and runs that recipe. The fixture
  `tests/fixtures/rect.wmf` is built by hand from the WMF specification.
