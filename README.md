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
| `unsupported_type` | The file is EMF or WMF and no converter is installed.                | the input            |
| `convert_failed`   | The EMF/WMF converter fails, times out or writes no usable PNG.      | the input            |
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

### EMF and WMF input

`inspect`, `check` and `fit` accept `.emf` and `.wmf` files (the extension may
be in any case) when Inkscape or LibreOffice is installed. pixctl converts the
file to a PNG in a temporary directory, removes the directory afterwards, and
then handles the PNG like any other input. The `path` in the output stays the
original EMF or WMF path, and `fit` names its output from the original file
name.

pixctl looks for a converter in this order, and the first match wins:

1. `PIXCTL_CONVERTER`, when set and not empty: the path or name of a program. A
   name that contains `inkscape` is run as Inkscape, and a name that contains
   `soffice` or `libreoffice` is run as LibreOffice. Any other name gives
   `convert_failed`.
2. `inkscape` on `PATH`.
3. `soffice` on `PATH`, then `libreoffice` on `PATH`.

Each conversion has a limit of 60 s. Set `PIXCTL_CONVERT_TIMEOUT_SECS` to a
positive whole number to change it; any other value keeps the default. When the
limit passes, pixctl kills the converter and reports `convert_failed`, then
continues with the next file. On Unix the converter runs in its own process
group and a timeout kills the whole group with the `kill` utility. Without
`kill`, a process that the converter started can outlive the limit.

Two errors apply (see [Error codes](#error-codes)):

- With no converter installed, the file gets `unsupported_type`, with the hint
  `install Inkscape or LibreOffice, or set PIXCTL_CONVERTER`.
- When the converter exits with a non-zero status, times out, or exits without
  writing a PNG that can be decoded, the file gets `convert_failed`. The message
  names the converter and says what happened.

`inspect` names the converter it used: the JSON report has a `converter` field
with the file name of the program (for example `inkscape`), and the plain line
ends with `via=<name>`. The field is absent for other formats.

The result depends on the converter. Inkscape and LibreOffice, and different
versions of each, can render one file at different sizes and with different
text, so the width, the font estimate and the check result can differ between
machines. To get results that do not depend on the machine:

- Convert each EMF or WMF file once with `pixctl fit`, commit the PNG, and
  reference and `check` the PNG, not the original file.
- When CI must read EMF or WMF files, set `PIXCTL_CONVERTER=inkscape` so that
  the same program is used every time and LibreOffice is never chosen instead.

The examples use Inkscape, found on `PATH`. The output was produced with a
script that stands in for Inkscape and returns a 300x200 white PNG, because no
converter was installed on the machine that wrote this page.

```console
$ pixctl inspect diagram.wmf
diagram.wmf  300x200  alpha=false  font=?px (low)  rule=unchanged x1 -> 334px  via=inkscape
1 image: 0 icon, 1 unchanged, 0 upscale, 0 too-small, 0 oversize
$ pixctl fit diagram.wmf --out o
FIT  [unchanged] diagram.wmf  o/diagram.png (334×234)
```

When Inkscape exits with status 3, the `errors` entry of
`pixctl inspect --json diagram.wmf` is:

```json
{
  "code": "convert_failed",
  "message": "inkscape: failed with exit status: 3",
  "file": "diagram.wmf"
}
```

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
pixctl fit <image>... --out <dir> [--format png|webp] [--font-px F] [--json]
```

`fit` writes one image per input image to the `--out` directory. The output is
named `<file stem>.<ext>`, where the extension is `png` (the default) or `webp`.
`fit` creates the directory when it is missing. For each image, it:

1. flattens transparent pixels onto white;
2. scales the image by the factor of the fit rule: `upscale` multiplies the size
   by 2 (nearest neighbour) or by 1.5 (Lanczos3), and every other rule keeps the
   size;
3. adds 16 px of white padding and a 1 px `#d0d7de` frame, except for `icon`
   images, which stay unframed.

`--format webp` writes a lossless WebP instead of a PNG. The pixels are the same
as in the PNG output, and the file has no metadata. `check` accepts the WebP
output:

```console
$ pixctl fit blank.png --out fitted --format webp
FIT  [unchanged] blank.png  fitted/blank.webp (334×234)
$ pixctl check fitted/blank.webp
PASS  [width] fitted/blank.webp  334 px <= 800 px
REVIEW  [min-font-size] fitted/blank.webp  font size estimate has low confidence
PASS  [opaque] fitted/blank.webp  no transparent pixel
PASS  [framed] fitted/blank.webp  frame and padding present
1 image: 0 passed, 0 failed, 1 review
```

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
bytes on every run, for both formats. For an EMF or WMF input this holds only
with the same converter and converter version. The PNG encoder uses fixed
settings, writes no metadata and no interlacing. The WebP encoder is lossless
and writes no metadata. The files are not optimised for size.

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

## Evaluating the font estimate

`just eval` measures how well the font size estimate works on a folder of your
own images. It is a private tool for people who work on pixctl: it is a Cargo
example, it is not part of the released `pixctl` binary, and
`cargo install
pixctl` does not install it. It never prints a file name, a path,
image content or a line per image. Its output holds totals only, so the result
can be shared when the images cannot.

`DIR` must be outside the pixctl repository. The tool refuses a `DIR` that is
the repository or a folder inside it, prints
`pixctl eval: DIR is inside the pixctl repository; keep private images outside it`
and exits with `2`, before it reads anything. It also exits with `2` when `DIR`
cannot be read.

### Labelled run

```sh
just eval DIR MANIFEST
```

The manifest is a TOML file with one `[[image]]` table per labelled image. Each
entry has a `file`, relative to `DIR`, and the label: the font size of the text
in the image, in pixels. Give the label in one of two forms:

```toml
[[image]]
file = "login.png"
px = 14

[[image]]
file = "sub/settings.png"
pt = 10.5
scale = 1.0
```

- `px`: the font size in pixels, used as it is.
- `pt` with `scale`: the font size in points and the factor by which the image
  was scaled when it was taken. The label is `px = pt × 96 ÷ 72 × scale`. The
  second entry above is `10.5 × 96 ÷ 72 × 1.0 = 14` px.

An entry must have exactly one of the two forms. All values must be positive
numbers. An absolute `file` is an error. Keys that the tool does not know are
ignored. A manifest with no entry is an error.

The tool estimates the font size of every labelled image and prints four lines:

```console
$ just eval ~/shots ~/shots-labels.toml
images: 4 (confident 4, review 0 = 0.0 %)
error on confident: mean 0.6 px, p95 0.7 px
confidently wrong (> 2 px): 0 of 4 = 0.0 % (limit 2 %)  PASS
K interval for ±1.5 px on all labelled images (n = 4): 0.906–1.094 (current K 1.045, inside)
```

- An image is confident when the estimate has `high` confidence and a font size.
  The other images are counted as `review`.
- An image is confidently wrong when it is confident and the estimate differs
  from the label by more than 2 px. The limit is that fewer than 2 % of the
  confident images are confidently wrong; 1 of 50 is exactly 2 % and fails.
- The `K` line gives the range of the calibration constant `K` for which every
  labelled image that has an estimate has an error of at most 1.5 px, and
  whether the current `K` (see
  [AD-0011](docs/decisions/0011-k-calibration-rule.md)) is `inside` or `outside`
  that range. `n` is the number of images that have an estimate. The range reads
  `none (no single K fits)` when no value of `K` fits all of those images, and
  `none (no image has an estimate)` when `n` is 0.
- When no image is confident, the error line and the wrong line print `n/a`, and
  the wrong line ends with `FAIL`.

The exit code is `0` when the limit is met, `1` when it is not met, and `2` for
an operational error: an unreadable `DIR` or manifest, an invalid manifest, or
an image that cannot be read. The error text is fixed text and the number of the
entry (counted from 1), for example `pixctl eval: entry 2: cannot read image`.
For invalid TOML, the text gives only the line number. It never gives the file
name or the text of a lower-level error.

### Survey

```sh
just eval DIR
```

Without a manifest, the tool surveys every image below `DIR`, including
subfolders and without following symbolic links. It reads the extensions `png`,
`jpg`, `jpeg`, `gif`, `bmp`, `tif`, `tiff`, `webp`, `emf` and `wmf`, in any
case, and counts `.svg` files without reading them. The survey needs no labels:
it counts how many images have a confident estimate and how many images fall
under each fit rule. The progress count `done/total` goes to stderr, and the
report goes to stdout:

```console
$ just eval ~/shots
5/5
images: 5 (skipped SVG: 1)
confident: 4 (80.0 %)
rules: icon 0, unchanged 4, upscale 0, too-small 0, oversize 0
failed: read_failed 1
```

The `failed` line counts the images that could not be loaded, by the error codes
in [Error codes](#error-codes). It adds `panicked` for an image that made a
decoder panic; the survey continues with the next image. It reads `failed: 0`
when every image loaded. The survey exits with `0` unless an operational error
stops it, such as an unreadable `DIR`; images that fail to load do not change
the exit code.

## License

MIT

<!-- git-std:bootstrap -->

## Post-clone setup

Run `./bootstrap` after `git clone` or `git worktree add`.
