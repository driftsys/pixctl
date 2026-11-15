# pixctl WebP output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development.

**Goal:** `fit --format webp` writes lossless, deterministic WebP files.

**Spec:** `docs/wip/2026-11-15-pixctl-webp-design.md`

## Global Constraints

- No new dependency, and no C code. Use the `webp` feature of `image`, which is
  already enabled. `cargo tree -e build` must show no `cc`.
- Lossless only. The output is byte-identical across runs.
- Exit codes, the envelope shape and the line formats are unchanged.
- `just build` is green. Commits use Conventional Commits (`fit`, `cli`, `docs`)
  and end with `Co-Authored-By: Claude <noreply@anthropic.com>`.
- Docs are in plain, literal English.

## Review Focus

1. `a.png` and `a.webp` as outputs of `a.jpg` in two runs into the same `--out`
   directory are both kept, because the extensions differ.
2. A WebP output decodes to exactly the pixels of the PNG output of the same
   input.
3. `check` on a fitted WebP passes `width`, `opaque` and `framed`.

### Task 1: WebP encoding, the `--format` flag, and docs

**Files:** `src/cli.rs`, `src/fit.rs`, `src/fit_cmd.rs`, `src/main.rs`,
`tests/fit.rs`, `tests/cli.rs`, `README.md`

- [ ] `cli.rs`: add `enum Format { Png, Webp }` (`ValueEnum`) and
      `--format <png|webp>` on `fit`, with the default `png`. Add a help example
      `pixctl fit shot.png --out docs/img --format webp`.
- [ ] `fit.rs`: add `encode_webp(img: &RgbImage) -> anyhow::Result<Vec<u8>>`,
      using `image::codecs::webp::WebPEncoder::new_lossless`.
- [ ] `fit_cmd.rs`: pass the format through, choose the extension, and use the
      full file name for the collision key.
- [ ] Tests:
  - the WebP output is deterministic (two runs give equal bytes);
  - it decodes to the same pixels as the PNG output;
  - `check` passes `width`, `opaque` and `framed` on it;
  - `--format webp` writes `<stem>.webp`, and the FIT line and the JSON `out`
    show it;
  - outputs `a.png` and `a.webp` coexist;
  - the CLI rejects `--format gif`.
- [ ] README: document `--format`, using real output from the binary.
- [ ] `just build` is green. Commit.
