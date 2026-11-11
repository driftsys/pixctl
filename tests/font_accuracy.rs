//! Accuracy of the font-size estimator on synthetic fixtures.

mod support;

use pixctl::font::{estimate, Confidence};
use support::{jpeg_roundtrip, noise, photo_like, render_text, resize, Face, TextSpec};

const FACES: [Face; 3] = [Face::DejaVu, Face::Liberation, Face::Noto];

fn spec(face: Face, em_px: f32, lines: usize, invert: bool) -> TextSpec {
    TextSpec {
        face,
        em_px,
        lines,
        invert,
    }
}

#[test]
fn exact_sizes_within_tolerance() {
    for face in FACES {
        for em_px in [11.0, 14.0, 20.0, 24.0] {
            for invert in [false, true] {
                let est = estimate(&render_text(&spec(face, em_px, 6, invert)));
                let px = est.px.expect("a size is estimated");
                assert!(
                    (px - em_px).abs() <= 1.5,
                    "{face:?} {em_px} invert={invert}: estimated {px}"
                );
                assert_eq!(
                    est.confidence,
                    Confidence::High,
                    "{face:?} {em_px} {invert}"
                );
            }
        }
    }
}

#[test]
fn small_text_is_not_confidently_wrong() {
    for face in FACES {
        for em_px in [8.0, 9.0, 10.0] {
            for invert in [false, true] {
                let est = estimate(&render_text(&spec(face, em_px, 6, invert)));
                let ok = est.confidence == Confidence::Low
                    || est.px.is_some_and(|px| (px - em_px).abs() <= 1.5);
                assert!(ok, "{face:?} {em_px} invert={invert}: {est:?}");
            }
        }
    }
}

#[test]
fn survives_jpeg_and_scaling() {
    let base = render_text(&spec(Face::DejaVu, 14.0, 6, false));
    for (img, expected) in [
        (jpeg_roundtrip(&base, 75), 14.0),
        (resize(&base, 1.5), 21.0),
    ] {
        let est = estimate(&img);
        let ok = est.confidence == Confidence::Low
            || est.px.is_some_and(|px| (px - expected).abs() <= 1.5);
        assert!(ok, "expected {expected}: {est:?}");
    }
}

#[test]
fn noise_is_low_confidence() {
    assert_eq!(estimate(&noise(400, 300, 1)).confidence, Confidence::Low);
}

#[test]
fn photo_like_is_low_confidence() {
    assert_eq!(
        estimate(&photo_like(400, 300, 1)).confidence,
        Confidence::Low
    );
}

#[test]
fn blank_white_and_blank_black_are_low_and_none() {
    for value in [0u8, 255] {
        for (w, h) in [(1, 1), (200, 100)] {
            let est = estimate(&image::GrayImage::from_pixel(w, h, image::Luma([value])));
            assert_eq!(est.px, None);
            assert_eq!(est.confidence, Confidence::Low);
            assert_eq!((est.lines, est.glyphs), (0, 0));
        }
    }
}

#[test]
fn few_lines_is_low_confidence() {
    let est = estimate(&render_text(&spec(Face::DejaVu, 14.0, 2, false)));
    assert_eq!(est.confidence, Confidence::Low);
}
