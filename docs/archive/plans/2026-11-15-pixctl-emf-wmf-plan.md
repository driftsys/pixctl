# pixctl EMF/WMF Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development.

**Goal:** `.emf` and `.wmf` inputs are converted with Inkscape or LibreOffice
when one is available.

**Spec:** `docs/wip/2026-11-15-pixctl-emf-wmf-design.md`

## Global Constraints

- No C code. `tempfile` is the only dependency change (it moves to
  `[dependencies]`).
- New error code `convert_failed`. `unsupported_type` keeps its meaning (no
  converter found) and gains the hint from the spec.
- 60 s limit per conversion. The child process is killed on timeout and never
  left running.
- Exit codes and envelope shapes are unchanged.
- `just build` is green. Commits use Conventional Commits (`inspect`, `fit`,
  `ci`, `docs`, `deps`) and end with
  `Co-Authored-By: Claude <noreply@anthropic.com>`.
- Docs are in plain, literal English.

## Review Focus

1. A converter that hangs is killed after the limit, and pixctl continues with
   the next file.
2. A converter that exits 0 but writes no PNG gives `convert_failed`, not a
   panic or `read_failed`.
3. The temporary directory is removed on success and on every failure.
4. `PIXCTL_CONVERTER` set to an unknown program name gives `convert_failed` with
   a hint.
5. An uppercase `.EMF` extension is converted too.

### Task 1: The convert module, used by `load`

**Files:** create `src/convert.rs`; modify `src/load.rs`, `src/lib.rs`,
`Cargo.toml`, `Cargo.lock`, `tests/inspect.rs` (or a new `tests/convert.rs`).

**Interfaces:**

- `convert::find() -> Option<Converter>`, where
  `Converter { kind: Kind, program: PathBuf }` and
  `Kind { Inkscape, LibreOffice }`. It reads `PIXCTL_CONVERTER` and `PATH`.
  Write a pure helper that takes the environment values as parameters, so it can
  be tested.
- `convert::to_png(conv: &Converter, input: &Path, limit: Duration) -> Result<image::DynamicImage, Failure>`.
- In `load::load`, EMF/WMF goes through `find`: none gives `unsupported_type`
  with the hint; otherwise `to_png`, then the normal RGBA path.

Steps:

- [ ] Unit tests for the lookup: the env var with each kind, an unknown name, an
      empty env var falling back to PATH, and the PATH order (inkscape, soffice,
      libreoffice). Use temporary directories with executable stubs
      (`#[cfg(unix)]` for the PATH and stub tests; Windows uses `.exe`/`.cmd`
      names, so keep those tests Unix-only).
- [ ] Integration tests with fake converter scripts (Unix only) written into a
      temp dir and pointed at by `PIXCTL_CONVERTER`, with names such as
      `fake-inkscape` and `fake-soffice`. Each script copies a known PNG fixture
      to the output path that the real tool would write:
  - `inspect` on `a.emf` reports the fixture's size, and the JSON `path` is
    `a.emf`;
  - `fit` on `a.EMF` writes `<out>/a.png`;
  - a script that exits 3 gives `convert_failed`, and the message contains the
    exit status;
  - a script that exits 0 and writes nothing gives `convert_failed`;
  - a script that sleeps beyond a short limit is killed. Make the limit
    injectable for the test (for example `PIXCTL_CONVERT_TIMEOUT_SECS`, read
    only in `load`, and documented), and use 1 s in the test;
  - with no converter (empty PATH and no env var), the result is
    `unsupported_type` with the hint;
  - the temp dir is removed: check that no `pixctl-convert*` entries remain
    under a `TMPDIR` set for the test.
- [ ] Implement it. Poll `try_wait` every 50 ms until the limit, then `kill` and
      `wait`. Send the converter's stdout and stderr to null, or keep the last
      lines of stderr for the message (agent's choice).
- [ ] `just build` is green, and `cargo tree -e build | grep -w cc` prints
      nothing. Commit.

### Task 2: A real-converter test, the CI job, and docs

**Files:** `tests/convert.rs`, `.github/workflows/ci.yml`, `justfile`,
`README.md`, `AGENTS.md`

- [ ] Add a test marked `#[ignore]`, named `converter_real_inkscape_*`. It
      writes a tiny WMF fixture (commit one small `.wmf` under
      `tests/fixtures/`, less than 2 KB, created with a tool or hand-built from
      the WMF spec) and runs `inspect` with the real `inkscape`. It asserts that
      the width and height are above 0.
- [ ] Add a `just test-converter` recipe that runs
      `cargo test -- --ignored converter`.
- [ ] Add a CI job `converter` on `ubuntu-latest`:
      `apt-get install -y inkscape`, then the same toolchain setup as the
      existing test job, then `just test-converter`. CI runs only `just`
      recipes.
- [ ] README: a section on EMF/WMF input (lookup order, `PIXCTL_CONVERTER`,
      `PIXCTL_CONVERT_TIMEOUT_SECS`, the 60 s default, and the errors), plus the
      `convert_failed` row in the error table. Add `convert.rs` to the AGENTS.md
      module list.
- [ ] `just build` is green. Commit.
