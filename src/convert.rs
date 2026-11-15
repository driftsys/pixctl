//! Converts EMF and WMF files to PNG through an installed Inkscape or
//! LibreOffice.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::output::Failure;

/// The hint for a missing converter.
pub const MISSING_HINT: &str = "install Inkscape or LibreOffice, or set PIXCTL_CONVERTER";

/// The default time limit for one conversion, in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// The supported converter programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Inkscape,
    LibreOffice,
}

/// A converter program and its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converter {
    pub kind: Kind,
    pub program: PathBuf,
}

fn convert_failed(input: &Path, message: String, hint: Option<&str>) -> Failure {
    Failure {
        code: "convert_failed",
        message,
        file: Some(input.to_string_lossy().into_owned()),
        hint: hint.map(str::to_string),
        with: None,
    }
}

/// Chooses the kind from the file name of `program`.
fn kind_of(program: &Path) -> Option<Kind> {
    let name = program.file_name()?.to_string_lossy().to_ascii_lowercase();
    if name.contains("inkscape") {
        Some(Kind::Inkscape)
    } else if name.contains("soffice") || name.contains("libreoffice") {
        Some(Kind::LibreOffice)
    } else {
        None
    }
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn on_path(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path_var).find_map(|dir| {
        let plain = dir.join(name);
        if is_executable(&plain) {
            return Some(plain);
        }
        let exe = dir.join(format!("{name}.exe"));
        is_executable(&exe).then_some(exe)
    })
}

/// Finds a converter from the values of `PIXCTL_CONVERTER` and `PATH`.
///
/// A non-empty `explicit` value wins. Its name must be a supported tool, or
/// the result is `convert_failed`. Otherwise `inkscape`, `soffice` and
/// `libreoffice` are looked up on `path_var`, in that order. `Ok(None)` means
/// that no converter is installed.
pub fn find_in(
    explicit: Option<&str>,
    path_var: Option<&OsStr>,
) -> Result<Option<Converter>, Failure> {
    if let Some(value) = explicit.filter(|v| !v.is_empty()) {
        let program = PathBuf::from(value);
        return match kind_of(&program) {
            Some(kind) => Ok(Some(Converter { kind, program })),
            None => Err(Failure {
                code: "convert_failed",
                message: format!("unknown converter in PIXCTL_CONVERTER: {value}"),
                file: None,
                hint: Some("use Inkscape or LibreOffice (soffice)".to_string()),
                with: None,
            }),
        };
    }
    let Some(path_var) = path_var else {
        return Ok(None);
    };
    for (name, kind) in [
        ("inkscape", Kind::Inkscape),
        ("soffice", Kind::LibreOffice),
        ("libreoffice", Kind::LibreOffice),
    ] {
        if let Some(program) = on_path(name, path_var) {
            return Ok(Some(Converter { kind, program }));
        }
    }
    Ok(None)
}

/// Parses `PIXCTL_CONVERT_TIMEOUT_SECS`. A missing value, or one that is not a
/// positive integer, gives the default of 60 s.
pub fn parse_timeout(value: Option<&str>) -> Duration {
    let secs = value
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// Converts `input` to a PNG in a temporary directory and decodes it.
///
/// The converter is killed when it runs longer than `limit`. The temporary
/// directory is removed on every path.
pub fn to_png(
    conv: &Converter,
    input: &Path,
    limit: Duration,
) -> Result<image::DynamicImage, Failure> {
    let name = conv
        .program
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| conv.program.display().to_string());
    let fail = |msg: String| convert_failed(input, format!("{name}: {msg}"), None);

    let tmp = tempfile::Builder::new()
        .prefix("pixctl-convert")
        .tempdir()
        .map_err(|e| fail(format!("cannot create a temporary directory: {e}")))?;

    let mut cmd = Command::new(&conv.program);
    let produced = match conv.kind {
        Kind::Inkscape => {
            let out = tmp.path().join("out.png");
            let mut export = std::ffi::OsString::from("--export-filename=");
            export.push(&out);
            cmd.arg(input).arg("--export-type=png").arg(export);
            out
        }
        Kind::LibreOffice => {
            let stem = input.file_stem().unwrap_or_default();
            cmd.args(["--headless", "--convert-to", "png", "--outdir"])
                .arg(tmp.path())
                .arg(input);
            tmp.path().join(Path::new(stem).with_extension("png"))
        }
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|e| fail(format!("cannot start the converter: {e}")))?;

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(fail(format!("cannot wait for the converter: {e}")));
            }
        }
        if start.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(fail(format!(
                "timed out after {} s and was killed",
                limit.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    if !status.success() {
        return Err(fail(format!("failed with {status}")));
    }
    if !produced.is_file() {
        return Err(fail("exited successfully but wrote no PNG".to_string()));
    }
    image::open(&produced).map_err(|e| fail(format!("wrote a PNG that cannot be decoded: {e}")))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn stub(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn explicit_value_picks_each_kind() {
        let c = find_in(Some("/x/my-Inkscape"), None).unwrap().unwrap();
        assert_eq!(c.kind, Kind::Inkscape);
        assert_eq!(c.program, PathBuf::from("/x/my-Inkscape"));
        for n in ["soffice", "/o/libreoffice7"] {
            assert_eq!(
                find_in(Some(n), None).unwrap().unwrap().kind,
                Kind::LibreOffice
            );
        }
    }

    #[test]
    fn unknown_explicit_name_is_convert_failed_with_hint() {
        let f = find_in(Some("gimp"), None).unwrap_err();
        assert_eq!(f.code, "convert_failed");
        assert!(f.hint.unwrap().contains("Inkscape"));
    }

    #[test]
    fn empty_explicit_value_falls_back_to_path() {
        let d = tempfile::tempdir().unwrap();
        stub(d.path(), "soffice");
        let c = find_in(Some(""), Some(d.path().as_os_str()))
            .unwrap()
            .unwrap();
        assert_eq!(c.kind, Kind::LibreOffice);
    }

    #[test]
    fn path_order_is_inkscape_soffice_libreoffice() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().as_os_str();
        assert_eq!(find_in(None, Some(path)).unwrap(), None);
        stub(d.path(), "libreoffice");
        let c = find_in(None, Some(path)).unwrap().unwrap();
        assert_eq!(c.program, d.path().join("libreoffice"));
        stub(d.path(), "soffice");
        let c = find_in(None, Some(path)).unwrap().unwrap();
        assert_eq!(c.program, d.path().join("soffice"));
        stub(d.path(), "inkscape");
        let c = find_in(None, Some(path)).unwrap().unwrap();
        assert_eq!(
            (c.kind, c.program),
            (Kind::Inkscape, d.path().join("inkscape"))
        );
    }

    #[test]
    fn non_executable_file_on_path_is_ignored() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("inkscape"), "").unwrap();
        assert_eq!(find_in(None, Some(d.path().as_os_str())).unwrap(), None);
    }

    #[test]
    fn timeout_parsing_falls_back_to_60() {
        assert_eq!(parse_timeout(None), Duration::from_secs(60));
        assert_eq!(parse_timeout(Some("5")), Duration::from_secs(5));
        for bad in ["0", "-3", "abc", "", "1.5"] {
            assert_eq!(parse_timeout(Some(bad)), Duration::from_secs(60), "{bad}");
        }
    }
}
