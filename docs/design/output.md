# Output convention — design

Requirements: [specification/output.md](../specification/output.md). Decision:
[AD-0016](../decisions/0016-json-envelope-and-human-output.md).

## Components

[`output.rs`](../../src/output.rs) holds the shared types and helpers. Each
command builds its results, summary and errors, then prints them in one place.

## Interfaces

- `output::Failure { code, message, file, hint, with }` is `Serialize` and
  `std::error::Error`. `code` is a `&'static str`; `file`, `hint` and `with` are
  skipped in JSON when `None`. `Display` is the message alone, and the `pixctl:`
  prefix is added where the line is printed.
- `output::Envelope<R, S> { schema, results, summary, errors }`. The field order
  is the key order.
- `output::color_enabled(is_tty, no_color, clicolor_force) -> bool` is pure.
  `main.rs` reads the terminal state and the environment, and passes `color` to
  `check::run`.
- `output::paint(tag, enabled)` wraps `PASS` (32), `FAIL` (31) or `REVIEW` (33)
  in ANSI codes. The two spaces after the tag stay outside the codes.
- `output::count(n, noun)` gives `1 image` or `3 images`.
- `load::load(path) -> Result<Loaded, Failure>` gives `read_failed` and
  `unsupported_type`, so the codes do not depend on message text.
- `inspect::run`, `check::run` and `fit_cmd::run` take `json` and the writers;
  `check::run` also takes `color`.

## Flow

1. A command collects results and `Failure` values while it processes the
   inputs.
2. Each `Failure` is written to `err` as `pixctl: <message>`.
3. The exit code is `2` when there is a failure; `check` otherwise uses `1` for
   a failed check (a review under `--strict`).
4. With `json`, the envelope is written to `out` with a trailing newline.
   Without it, `inspect` and `check` print their lines and then a summary line
   when at least one image loaded. `fit` writes its `FIT` lines to `err`.
5. A failure to write the envelope or the text output gives exit `2`.

`fit` messages name the input path, except `out_dir_failed`, which names the
output directory. A `name_collision` keeps a map from each output name to the
input that claimed it first, which gives `with`. The name is claimed before the
write, so it stays claimed if that write fails. `same_file` and `name_collision`
carry a `hint`.

## Testing

Unit tests in `output.rs` cover all colour combinations (an empty `NO_COLOR`
counts as unset), the exact escape bytes, `count` and the omission of absent
`Failure` fields. `tests/inspect.rs`, `tests/check.rs` and `tests/fit.rs` cover
the envelope key order and schema, a mixed run, a run where every input fails,
`--strict` with a review image, the summary lines, colour with `CLICOLOR_FORCE`
and `NO_COLOR`, an empty stdout for `fit` in text mode, and the codes
`read_failed`, `unsupported_type`, `out_dir_failed`, `same_file`,
`name_collision` and `write_failed` (a blocked target, and a broken stderr or
stdout writer). `tests/cli.rs` covers the `Examples:` help sections.
