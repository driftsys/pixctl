# pixctl clig output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Adopt the cross-tool JSON envelope and the human-output rules in
`inspect`, `check` and `fit`.

**Architecture:** A new `src/output.rs` holds the envelope type, the error type
with stable codes, the colour decision, and the plural helper. Each command
builds its results, summary and errors, then prints them in one place. `load`
returns a typed error, so that the codes are not derived from message text.

**Tech Stack:** Rust, `clap` derive (`after_help`), `serde`, and
`std::io::IsTerminal`. No new dependency.

**Spec:** `docs/wip/2026-11-13-pixctl-clig-output-design.md`. The reference
record is tblctl AD-0005 (quoted in the spec).

## Global Constraints

- JSON: exactly one pretty-printed object on stdout with `--json`, also when
  there are errors. The key order is `schema`, `results`, `summary`, `errors`.
  The schema string is `pixctl/<command>/v1`.
- Error object: `code`, `message`, then `file`, `hint` and `with` only when
  present (skip them when `None`). Each error is also one `pixctl: <message>`
  line on stderr, in both modes.
- Exit codes 0/1/2 are unchanged. The text line format
  `TAG  [check] path  detail` is unchanged.
- Colour rule: on when stdout is a TTY, or when `CLICOLOR_FORCE` is non-empty. A
  non-empty `NO_COLOR` always turns it off.
- `just build` is green. Commits use Conventional Commits with scopes
  `cli|inspect|check|fit|docs` and end with
  `Co-Authored-By: Claude <noreply@anthropic.com>`.
- Docs and comments are in plain, literal English.

## Review Focus

1. A run where every input fails still prints a valid envelope with empty
   `results` and exits 2.
2. `--json` with a mixed run (one error, one success) has both `results` and
   `errors`.
3. ANSI colour codes never appear in JSON output, or when stdout is piped
   without `CLICOLOR_FORCE`.
4. `fit` text mode leaves stdout empty.
5. Under `--strict`, the image status in JSON stays `review` while the exit code
   is 1.

---

### Task 1: The output module, typed load errors, `inspect`, and help examples

**Files:**

- Create: `src/output.rs`
- Modify: `src/lib.rs`, `src/load.rs`, `src/inspect.rs`, `src/cli.rs`,
  `tests/inspect.rs`, `tests/cli.rs`

**Interfaces (produces):**

- `output::Failure { code: &'static str, message: String, file: Option<String>, hint: Option<String>, with: Option<String> }`,
  which is `Serialize` and `std::error::Error`.
- `output::Envelope<R, S> { schema: String, results: Vec<R>, summary: S, errors: Vec<Failure> }`.
- `output::color_enabled(is_tty: bool, no_color: Option<&str>, clicolor_force: Option<&str>) -> bool`,
  which is pure.
- `output::paint(tag: &str, enabled: bool) -> String`. PASS gives green (32),
  FAIL red (31), REVIEW yellow (33). The padding width does not change.
- `output::count(n: usize, noun: &str) -> String`, giving `1 image` or
  `3 images`.
- `load::load(&Path) -> Result<Loaded, Failure>`, with the codes `read_failed`
  and `unsupported_type`.

Steps:

- [ ] Unit tests in `output.rs` cover `color_enabled` (all 8 combinations of
      TTY, NO_COLOR and CLICOLOR_FORCE, plus an empty `NO_COLOR` that counts as
      unset), `paint` (exact escape bytes, and plain text when off), `count`,
      and the serialisation of `Failure` (absent fields are omitted).
- [ ] `inspect` uses the envelope, with
      `summary {images, icon, unchanged, upscale, too_small, oversize}`. Text
      mode ends with
      `N images: a icon, b unchanged, c upscale, d too-small, e oversize`, left
      out when there are no results.
- [ ] Update `tests/inspect.rs`: test the envelope shape; a missing file gives
      `errors[0].code == "read_failed"` and `results` is empty, with exit 2; the
      stderr line is present.
- [ ] `--help` for `inspect`, `check` and `fit` has an `Examples:` section
      (`after_help`) with 2 or 3 real invocations each. The top-level
      `after_help` ends with
      `Documentation: https://github.com/driftsys/pixctl#readme`. Add a CLI test
      for each.
- [ ] `just build` is green. Commit.

### Task 2: `check`

**Files:** `src/check.rs`, `tests/check.rs` (and `src/checks.rs` only if a
helper is needed)

- [ ] Results are one object per image,
      `{path, status, checks:[{check, status, detail}]}`. The image status is
      the worst of its checks (fail > review > pass).
      `summary {images, pass, fail, review}`.
- [ ] Text mode keeps the lines and colours the tag through `output::paint`,
      with `color_enabled` taken from the real stdout TTY state and the
      environment variables. It ends with
      `N images: P passed, F failed, R review`, left out when no image loaded.
- [ ] Tests:
  - the envelope, with key order and the schema `pixctl/check/v1`;
  - a mixed run (missing first, valid second) gives one result and one error,
    with exit 2;
  - all inputs missing gives an empty `results` and exit 2;
  - `--strict` with a review image gives `review` in JSON and exit 1;
  - the summary line text;
  - `CLICOLOR_FORCE=1` gives ANSI codes, `NO_COLOR=1 CLICOLOR_FORCE=1` gives
    none, and the piped default gives none.
- [ ] `just build` is green. Commit.

### Task 3: `fit` and docs

**Files:** `src/fit_cmd.rs`, `src/cli.rs` (`--json` on fit), `tests/fit.rs`,
`README.md`, `AGENTS.md`

- [ ] `fit --json` prints the envelope. Results are
      `{path, out, rule, factor, width, height}`, and the summary is
      `{written}`. The codes are `out_dir_failed`, `same_file`, `name_collision`
      (with `with`, the earlier input) and `write_failed`, plus the load codes.
- [ ] Text mode: the `FIT …` lines go to stderr, and stdout is empty.
- [ ] Tests: update the existing fit tests for stderr; add an envelope test; add
      an error-code test for each code that can be triggered; check that stdout
      is empty in text mode.
- [ ] README: document the envelope, the error codes, colour and the environment
      variables, the summary lines, and the move of the `fit` lines to stderr.
      Use real output from the built binary. Update the AGENTS.md module list.
- [ ] `just build` is green. Commit.
