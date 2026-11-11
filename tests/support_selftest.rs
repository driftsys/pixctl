mod support;

use support::{noise, render_text, Face, TextSpec};

fn spec(invert: bool) -> TextSpec {
    TextSpec {
        face: Face::DejaVu,
        em_px: 16.0,
        lines: 2,
        invert,
    }
}

#[test]
fn render_text_has_ink_and_is_deterministic() {
    let a = render_text(&spec(false));
    let b = render_text(&spec(false));
    assert_eq!(a.as_raw(), b.as_raw());
    assert!(a.pixels().any(|p| p.0[0] == 0));
    assert!(a.pixels().any(|p| p.0[0] == 255));
}

#[test]
fn invert_swaps_background() {
    assert_eq!(render_text(&spec(false)).get_pixel(0, 0).0[0], 255);
    assert_eq!(render_text(&spec(true)).get_pixel(0, 0).0[0], 0);
}

#[test]
fn noise_is_deterministic() {
    assert_eq!(noise(16, 16, 7).as_raw(), noise(16, 16, 7).as_raw());
    assert_ne!(noise(16, 16, 7).as_raw(), noise(16, 16, 8).as_raw());
}
