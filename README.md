# pixctl

A hermetic, cross-platform CLI that makes raster images display-correct in
Markdown docs (GitLab, GitHub, mdbook): it inspects images, fits them to a fixed
display contract (800 px column, 11 px minimum displayed font, opaque white
background with padding and frame) and gates them in CI.

> **Status: early development.** Commands (`inspect`, `fit`, `check`) are
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

## License

MIT

<!-- git-std:bootstrap -->

## Post-clone setup

Run `./bootstrap` after `git clone` or `git worktree add`.
