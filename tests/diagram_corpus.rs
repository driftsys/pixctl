use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    image: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    file: String,
    px: f32,
}

#[test]
fn corpus_manifest_matches_decodable_images() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/diagrams");
    let manifest: Manifest =
        toml::from_str(&std::fs::read_to_string(root.join("manifest.toml")).unwrap()).unwrap();
    assert!(!manifest.image.is_empty());
    let mut listed = BTreeSet::new();
    for entry in manifest.image {
        assert!((6.0..=24.0).contains(&entry.px), "{}", entry.file);
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
}
