# pixctl

A hermetic, cross-platform CLI that makes raster images display-correct in
Markdown docs (GitLab, GitHub, mdbook): it inspects images, fits them to a fixed
display contract (800 px column, 11 px minimum displayed font, opaque white
background with padding and frame) and gates them in CI.

> **Status: early development.** `inspect`, `check` and `fit` are implemented.
> Scope and rules are tracked in
> **[#1](https://github.com/driftsys/pixctl/issues/1)**.

Siblings: [`diagctl`](https://github.com/driftsys/diagctl) (SVG diagrams) and
[`tblctl`](https://github.com/driftsys/tblctl) (tables).

Exit codes: `0` clean · `1` ≥1 check failed · `2` operational error.

`pixctl --version` reports the display contract version next to the program
version. The contract version changes whenever a constant of the display
contract changes:

```console
$ pixctl --version
pixctl 0.0.1 (display contract 1)
```

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

## Output

### Envelope

With `--json`, every command writes exactly one pretty-printed JSON object to
stdout, also when there are errors. The keys come in this order:

```json
{"schema": "pixctl/<command>/v1", "results": [], "summary": {}, "errors": []}
```

`schema` is `pixctl/inspect/v1`, `pixctl/check/v1` or `pixctl/fit/v1`. `results`
holds the objects of the command, `summary` its totals, and `errors` the coded
errors. When every input fails, the envelope is still written, with empty
`results`, and the exit code is `2`. The format matches tblctl AD-0005; diagctl
is asked to adopt it.

Each error object has `code` and `message`. It has `file` when a file is
involved, `hint` when there is one, and `with` for a `name_collision`. Each
error is also written to stderr as one line `pixctl: <message>`, with and
without `--json`. Argument errors from the command line parser stay as plain
text with exit code `2` and no envelope.

### Error codes

| Code               | Meaning                                                              | `file`               |
| ------------------ | -------------------------------------------------------------------- | -------------------- |
| `read_failed`      | The file cannot be opened or decoded, or has no stem.                | the input            |
| `unsupported_type` | The file is EMF or WMF.                                              | the input            |
| `out_dir_failed`   | `fit` cannot create the `--out` directory.                           | the output directory |
| `same_file`        | The output would overwrite an input.                                 | the input            |
| `name_collision`   | Two inputs map to one output name.                                   | the later input      |
| `write_failed`     | `fit` cannot write the output or a status line, or a link blocks it. | the input            |

For `name_collision`, `with` is the earlier input, which owns the output name.
The following is the `errors` entry of
`pixctl check --json blank.png missing.png`; the `results` hold the checks of
`blank.png`:

```json
{
  "code": "read_failed",
  "message": "cannot decode image missing.png: No such file or directory (os error 2)",
  "file": "missing.png"
}
```

### Summary lines

Without `--json`, `inspect` and `check` end their stdout with one summary line.
One image is written `1 image`. When every input fails, the line is left out.

- `inspect`: `N images: a icon, b unchanged, c upscale, d too-small, e oversize`
- `check`: `N images: P passed, F failed, R review`, counted per image

`fit` has no summary line. Its `FIT` lines go to stderr, so stdout stays empty
in text mode. Before this change they went to stdout; read them from stderr, or
use `--json`.

### Colour

In text mode, the tags `PASS` (green), `FAIL` (red) and `REVIEW` (yellow) are
coloured when stdout is a terminal. A non-empty `NO_COLOR` turns colour off. A
non-empty `CLICOLOR_FORCE` turns it on when stdout is not a terminal; `NO_COLOR`
wins when both are set. The visible text is the same with and without colour,
and JSON output never contains colour codes.

```console
$ CLICOLOR_FORCE=1 pixctl check dot.png | head -n 1 | cat -v
^[[32mPASS^[[0m  [width] dot.png  32 px <= 800 px
```

## Usage

```sh
pixctl inspect <image>... [--json] [--font-px F]
```

`inspect` reports, for each image, its size, whether it has transparent pixels,
an estimated font size of the text in it, and the rule that `fit` would apply.
`--font-px F` replaces the estimate with a given font size in pixels. F is a
positive number and may be a decimal, such as `10.5`.

Without `--json`, it prints one line per image that loaded, then a summary line
(see [Output](#output)):

```console
$ pixctl inspect blank.png wide.png dot.png
blank.png  300x200  alpha=false  font=?px (low)  rule=unchanged x1 -> 334px
wide.png  1200x400  alpha=false  font=?px (low)  rule=oversize x1 -> 1234px
dot.png  32x32  alpha=false  font=?px (low)  rule=icon x1 -> 32px
3 images: 1 icon, 1 unchanged, 0 upscale, 0 too-small, 1 oversize
```

With `--json`, it prints the [envelope](#output). `results` holds one object per
image that loaded, and `summary` counts the images per rule:

```console
$ pixctl inspect --json wide.png
{
  "schema": "pixctl/inspect/v1",
  "results": [
    {
      "path": "wide.png",
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
  ],
  "summary": {
    "images": 1,
    "icon": 0,
    "unchanged": 0,
    "upscale": 0,
    "too_small": 0,
    "oversize": 1
  },
  "errors": []
}
```

- `font.px` is `null` when no text line is found. `font.confidence` is `high` or
  `low`; a `low` estimate never selects `upscale`. `font.source` is `estimate`,
  or `override` when `--font-px` is given.
- `rule` is one of `icon`, `unchanged`, `upscale`, `too-small` or `oversize`.
  `factor` is the scale factor and `output_width` the width in pixels of the
  fitted image. `oversize` images are not scaled, so their `output_width` is
  wider than the 800 px column.
- An image that fails to load is reported as a coded error and left out of
  `results`. The exit code is `2` when any image fails or the output cannot be
  written.

### check

```sh
pixctl check <image>... [--target markdown] [--strict] [--font-px F] [--json]
```

`check` gates images as they are committed, so it expects fitted images: the
padding and the frame must already be present. It runs four checks on each
image, in this order:

- `width`: passes when the image is at most 800 px wide.
- `min-font-size`: the displayed font size is the estimated font size times
  `min(1, 800 / width)`. It passes when the result is at least 11 px. When the
  image is framed, the estimate uses the content inside the frame.
- `opaque`: fails when any pixel has an alpha value below 255.
- `framed`: passes when the outer 1 px ring is exactly `#d0d7de` and the 16 px
  band inside it is exactly white. An image with a side below 35 px is never
  framed.

An icon is an image whose larger side is at most 64 px. It passes
`min-font-size` and `framed` with the detail `icon`.

`--target` selects the display target. `markdown` is the only target and the
default. `--font-px F` replaces the font estimate with a given font size in
pixels; F is a positive number and may be a decimal.

Without `--json`, `check` prints one line per check and image, in the form
`TAG  [check] path  detail`, then a summary line that counts the images (see
[Output](#output)):

```console
$ pixctl check blank.png wide.png clear.png dot.png
PASS  [width] blank.png  300 px <= 800 px
REVIEW  [min-font-size] blank.png  font size estimate has low confidence
PASS  [opaque] blank.png  no transparent pixel
FAIL  [framed] blank.png  frame or padding missing
FAIL  [width] wide.png  1200 px > 800 px
REVIEW  [min-font-size] wide.png  font size estimate has low confidence
PASS  [opaque] wide.png  no transparent pixel
FAIL  [framed] wide.png  frame or padding missing
PASS  [width] clear.png  300 px <= 800 px
REVIEW  [min-font-size] clear.png  font size estimate has low confidence
FAIL  [opaque] clear.png  60000 transparent pixels
FAIL  [framed] clear.png  frame or padding missing
PASS  [width] dot.png  32 px <= 800 px
PASS  [min-font-size] dot.png  icon
PASS  [opaque] dot.png  no transparent pixel
PASS  [framed] dot.png  icon
4 images: 1 passed, 3 failed, 0 review
```

The tag is one of:

- `PASS`: the image meets the check.
- `FAIL`: the image does not meet the check.
- `REVIEW`: the check cannot decide, because the font estimate has low
  confidence and `--font-px` is not given. A person has to look at the image.

`REVIEW` does not fail the run, unless `--strict` is given. With `--strict`,
every `REVIEW` counts as a failure and the exit code is `1`.

With `--json`, `check` prints the [envelope](#output). `results` holds one
object per image, with its checks. The image `status` is the worst of its checks
in the order `fail`, `review`, `pass`; each check `status` is `pass`, `fail` or
`review`. `summary` counts the images by status. Under `--strict`, the image
`status` stays `review`, and only the exit code changes:

```console
$ pixctl check --json clear.png
{
  "schema": "pixctl/check/v1",
  "results": [
    {
      "path": "clear.png",
      "status": "fail",
      "checks": [
        {
          "check": "width",
          "status": "pass",
          "detail": "300 px <= 800 px"
        },
        {
          "check": "min-font-size",
          "status": "review",
          "detail": "font size estimate has low confidence"
        },
        {
          "check": "opaque",
          "status": "fail",
          "detail": "60000 transparent pixels"
        },
        {
          "check": "framed",
          "status": "fail",
          "detail": "frame or padding missing"
        }
      ]
    }
  ],
  "summary": {
    "images": 1,
    "pass": 0,
    "fail": 1,
    "review": 0
  },
  "errors": []
}
```

The exit code is `0` when no check failed, `1` when at least one check failed
(or needs review under `--strict`), and `2` when an image cannot be loaded or
the output cannot be written. An image that fails to load is reported as a coded
error and the other images are still checked:

```console
$ pixctl check missing.png
pixctl: cannot decode image missing.png: No such file or directory (os error 2)
```

### fit

```sh
pixctl fit <image>... --out <dir> [--font-px F] [--json]
```

`fit` writes one PNG per input image to the `--out` directory. The output is
named `<file stem>.png`. `fit` creates the directory when it is missing. For
each image, it:

1. flattens transparent pixels onto white;
2. scales the image by the factor of the fit rule: `upscale` multiplies the size
   by 2 (nearest neighbour) or by 1.5 (Lanczos3), and every other rule keeps the
   size;
3. adds 16 px of white padding and a 1 px `#d0d7de` frame, except for `icon`
   images, which stay unframed.

The rule is the one that `inspect` reports. `--font-px F` replaces the font
estimate, as in `inspect`. `oversize` and `too-small` images are written at
their own scale, so `check` still reports them. Use `check` to gate them.

Without `--json`, `fit` prints one line per image written, in the form
`FIT  [rule] path  output (W×H)`. The lines go to stderr, and stdout stays
empty. `fit` has no summary line:

```console
$ pixctl fit blank.png wide.png clear.png dot.png --out fitted
FIT  [unchanged] blank.png  fitted/blank.png (334×234)
FIT  [oversize] wide.png  fitted/wide.png (1234×434)
FIT  [unchanged] clear.png  fitted/clear.png (334×234)
FIT  [icon] dot.png  fitted/dot.png (32×32)
```

With `--json`, `fit` prints the [envelope](#output) and writes no `FIT` line.
`results` holds one object per image written, and `summary` has the number of
images written, as `written`:

```console
$ pixctl fit --json blank.png dot.png --out j
{
  "schema": "pixctl/fit/v1",
  "results": [
    {
      "path": "blank.png",
      "out": "j/blank.png",
      "rule": "unchanged",
      "factor": 1.0,
      "width": 334,
      "height": 234
    },
    {
      "path": "dot.png",
      "out": "j/dot.png",
      "rule": "icon",
      "factor": 1.0,
      "width": 32,
      "height": 32
    }
  ],
  "summary": {
    "written": 2
  },
  "errors": []
}
```

A font size below 11 px selects `upscale`, when the scaled image still fits in
the column:

```console
$ pixctl fit --font-px 8 blank.png --out up
FIT  [upscale] blank.png  up/blank.png (484×334)
```

The output of an `icon`, `unchanged` or `upscale` image passes the `width`,
`opaque` and `framed` checks. `min-font-size` still depends on the text in the
image: the test images `blank.png` and `clear.png` have no text, so they get
`REVIEW`, which fails under `--strict`; the icon `dot.png` passes.

```console
$ pixctl check fitted/blank.png fitted/dot.png fitted/clear.png
PASS  [width] fitted/blank.png  334 px <= 800 px
REVIEW  [min-font-size] fitted/blank.png  font size estimate has low confidence
PASS  [opaque] fitted/blank.png  no transparent pixel
PASS  [framed] fitted/blank.png  frame and padding present
PASS  [width] fitted/dot.png  32 px <= 800 px
PASS  [min-font-size] fitted/dot.png  icon
PASS  [opaque] fitted/dot.png  no transparent pixel
PASS  [framed] fitted/dot.png  icon
PASS  [width] fitted/clear.png  334 px <= 800 px
REVIEW  [min-font-size] fitted/clear.png  font size estimate has low confidence
PASS  [opaque] fitted/clear.png  no transparent pixel
PASS  [framed] fitted/clear.png  frame and padding present
3 images: 1 passed, 0 failed, 2 review
```

`fit` is deterministic: the same image and the same version give the same output
bytes on every run. The PNG encoder uses fixed settings, writes no metadata and
no interlacing. The files are not optimised for size.

Two cases are errors for the image that causes them, with the codes
`name_collision` and `same_file`:

- Two inputs that map to the same output name. Output names that differ only in
  case, such as `blank.png` and `BLANK.PNG`, count as the same name, because
  macOS and Windows file systems store them as one file.
- An output that would overwrite an input, including an input that is reached
  through a hard link or a symbolic link. This test compares the files
  themselves, so it follows the case rules of the file system.

```console
$ pixctl fit blank.png BLANK.PNG --out o2
FIT  [unchanged] blank.png  o2/blank.png (334×234)
pixctl: BLANK.PNG: the output o2/BLANK.png is already claimed by another input
```

```console
$ pixctl fit same/blank.png --out same
pixctl: same/blank.png: the output same/blank.png would overwrite an input
```

With `--json`, the collision names the earlier input in `with`:

```console
$ pixctl fit --json blank.png BLANK.PNG --out o3
pixctl: BLANK.PNG: the output o3/BLANK.png is already claimed by another input
{
  "schema": "pixctl/fit/v1",
  "results": [
    {
      "path": "blank.png",
      "out": "o3/blank.png",
      "rule": "unchanged",
      "factor": 1.0,
      "width": 334,
      "height": 234
    }
  ],
  "summary": {
    "written": 1
  },
  "errors": [
    {
      "code": "name_collision",
      "message": "BLANK.PNG: the output o3/BLANK.png is already claimed by another input",
      "file": "BLANK.PNG",
      "hint": "rename one of the inputs",
      "with": "blank.png"
    }
  ]
}
```

The other images are still written, and the exit code is `2`. The exit code is
also `2` when an image cannot be loaded or written, when the output cannot be
written, or when the output directory cannot be created; in the last case no
image is written, and the envelope has empty `results` and one `out_dir_failed`
error. It is `0` otherwise.

## License

MIT

<!-- git-std:bootstrap -->

## Post-clone setup

Run `./bootstrap` after `git clone` or `git worktree add`.
