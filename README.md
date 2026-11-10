# pixctl

A hermetic, cross-platform CLI that makes raster images display-correct in
Markdown docs (GitLab, GitHub, mdbook): it inspects images, fits them to a
fixed display contract (800 px column, 11 px minimum displayed font, opaque
white background with padding and frame) and gates them in CI.

> **Status: early development.** Commands (`inspect`, `fit`, `check`) are
> stubbed and exit `2` with "not yet implemented". Scope and rules are tracked
> in **[#1](https://github.com/driftsys/pixctl/issues/1)**.

Siblings: [`diagctl`](https://github.com/driftsys/diagctl) (SVG diagrams) and
[`tblctl`](https://github.com/driftsys/tblctl) (tables).

Exit codes: `0` clean · `1` ≥1 check failed · `2` operational error.

## License

MIT
