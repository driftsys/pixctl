# pixctl

A hermetic, cross-platform CLI that makes raster images display-correct in
Markdown docs (GitLab, GitHub, mdbook): it inspects images, fits them to a fixed
display contract (800 px column, 11 px minimum displayed font, opaque white
background with padding and frame) and gates them in CI.

> **Status: early development.** `inspect` is implemented. `fit` and `check` are
> stubbed and exit `2` with "not yet implemented". Scope and rules are tracked
> in **[#1](https://github.com/driftsys/pixctl/issues/1)**.

Siblings: [`diagctl`](https://github.com/driftsys/diagctl) (SVG diagrams) and
[`tblctl`](https://github.com/driftsys/tblctl) (tables).

Exit codes: `0` clean · `1` ≥1 check failed · `2` operational error.

## Install

From the latest GitHub release (macOS, Linux; checksum-verified):

```sh
curl -fsSL https://raw.githubusercontent.com/driftsys/pixctl/main/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/driftsys/pixctl/main/install.ps1 | iex
```

`PIXCTL_INSTALL_DIR` (default `~/.local/bin`) and `PIXCTL_VERSION` (default
latest) override the defaults.

Manual download:

```sh
gh release download -R driftsys/pixctl -p 'pixctl-<target>*'
```

From source:

```sh
cargo install pixctl
```

## Usage

```sh
pixctl inspect <image>... [--json] [--font-px N]
```

`inspect` reports, for each image, its size, whether it has transparent pixels,
an estimated font size of the text in it, and the rule that `fit` would apply.
`--font-px N` replaces the estimate with a given font size in pixels.

Without `--json`, it prints one line per image. With `--json`, it prints a
pretty-printed array with one object per image that loaded:

```console
$ pixctl inspect --json blank.png
[
  {
    "path": "blank.png",
    "width": 1200,
    "height": 400,
    "alpha": false,
    "font": {
      "px": null,
      "confidence": "low",
      "lines": 0,
      "glyphs": 0,
      "source": "estimate"
    },
    "rule": "oversize",
    "factor": 1.0,
    "output_width": 1234
  }
]
```

- `font.px` is `null` when no text line is found. `font.confidence` is `high` or
  `low`; a `low` estimate never selects `upscale`. `font.source` is `estimate`,
  or `override` when `--font-px` is given.
- `rule` is one of `icon`, `unchanged`, `upscale`, `too-small` or `oversize`.
  `factor` is the scale factor and `output_width` the width in pixels of the
  fitted image.
- An image that fails to load is reported on stderr and left out of the output.
  The exit code is `2` when any image fails.

## License

MIT

<!-- git-std:bootstrap -->

## Post-clone setup

Run `./bootstrap` after `git clone` or `git worktree add`.
