use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    image: Vec<Entry>,
}

// Each entry has exactly `file` and `px`, a subset of what the eval manifest accepts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    file: String,
    px: f32,
}

/// Splits `<tool>-<kind>-<N>[-small].<ext>` into N and the `-small` flag.
fn parse_name(file: &str) -> (u32, bool) {
    let stem = Path::new(file).file_stem().unwrap().to_str().unwrap();
    let (stem, small) = match stem.strip_suffix("-small") {
        Some(s) => (s, true),
        None => (stem, false),
    };
    let n = stem.rsplit('-').next().unwrap();
    (
        n.parse().unwrap_or_else(|_| panic!("size in {file}")),
        small,
    )
}

fn dims(root: &Path, file: &str) -> (u32, u32) {
    let image = image::open(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
    (image.width(), image.height())
}

#[test]
fn corpus_manifest_matches_decodable_images() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/diagrams");
    let manifest: Manifest =
        toml::from_str(&std::fs::read_to_string(root.join("manifest.toml")).unwrap()).unwrap();
    assert!(!manifest.image.is_empty());
    let mut listed = BTreeSet::new();
    for entry in &manifest.image {
        assert!((6.0..=24.0).contains(&entry.px), "{}", entry.file);
        let (n, small) = parse_name(&entry.file);
        let expected = if small { n as f32 * 0.75 } else { n as f32 };
        assert!(
            (entry.px - expected).abs() < 0.005,
            "{}: px {} expected {expected}",
            entry.file,
            entry.px
        );
        let path = Path::new(&entry.file);
        assert_eq!(path.parent(), Some(Path::new("png")));
        assert!(
            listed.insert(entry.file.clone()),
            "duplicate: {}",
            entry.file
        );
        image::open(root.join(path)).unwrap_or_else(|e| panic!("{}: {e}", entry.file));
    }
    let actual: BTreeSet<_> = std::fs::read_dir(root.join("png"))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            format!("png/{}", entry.file_name().to_str().unwrap())
        })
        .collect();
    assert_eq!(listed, actual);

    // Every source has its three outputs.
    let mut sources = 0;
    for source in std::fs::read_dir(root.join("src")).unwrap() {
        let name = source.unwrap().file_name().into_string().unwrap();
        if name.starts_with('.') {
            continue; // render.sh skips dotfiles too
        }
        let base = name.rsplit_once('.').unwrap().0;
        for output in [
            format!("png/{base}.png"),
            format!("png/{base}-small.png"),
            format!("png/{base}.jpg"),
        ] {
            assert!(listed.contains(&output), "missing in manifest: {output}");
        }
        sources += 1;
        // Variant dimensions follow the base PNG.
        let (w, h) = dims(&root, &format!("png/{base}.png"));
        let (sw, sh) = dims(&root, &format!("png/{base}-small.png"));
        let near = |a: u32, b: f32| (a as f32 - b).abs() <= 1.0;
        assert!(
            near(sw, w as f32 * 0.75) && near(sh, h as f32 * 0.75),
            "{base}: small {sw}x{sh} vs base {w}x{h}"
        );
        assert_eq!(
            dims(&root, &format!("png/{base}.jpg")),
            (w, h),
            "{base}.jpg"
        );
    }
    assert_eq!(manifest.image.len(), sources * 3);
}
