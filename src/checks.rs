//! The display checks that `check` runs on one decoded image.

use crate::contract::{COLUMN, FRAME, FRAME_COLOUR, ICON_MAX, MIN_FONT_PX, PADDING};
use crate::font;
use crate::load::flatten_to_gray;
use image::{imageops, RgbaImage};
use serde::Serialize;

/// Result of one check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Fail,
    /// The check could not decide; a person has to look at the image.
    Review,
}

/// One check result for one image.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Outcome {
    #[serde(rename = "check")]
    pub name: &'static str,
    pub status: Status,
    pub detail: String,
}

/// Width of the band (frame plus padding) on each side of a framed image.
const BAND: u32 = PADDING + FRAME;

/// Runs the four checks, in the order `width`, `min-font-size`, `opaque`,
/// `framed`. `font_px` replaces the font estimate.
pub fn run_all(rgba: &RgbaImage, font_px: Option<f32>) -> Vec<Outcome> {
    let icon = rgba.width().max(rgba.height()) <= ICON_MAX;
    let framed = is_framed(rgba);
    vec![
        width(rgba),
        if icon {
            pass("min-font-size", "icon")
        } else {
            min_font_size(rgba, font_px, framed)
        },
        opaque(rgba),
        if icon {
            pass("framed", "icon")
        } else if framed {
            pass("framed", "frame and padding present")
        } else {
            fail("framed", "frame or padding missing")
        },
    ]
}

fn pass(name: &'static str, detail: &str) -> Outcome {
    Outcome {
        name,
        status: Status::Pass,
        detail: detail.to_string(),
    }
}

fn fail(name: &'static str, detail: &str) -> Outcome {
    Outcome {
        name,
        status: Status::Fail,
        detail: detail.to_string(),
    }
}

fn width(rgba: &RgbaImage) -> Outcome {
    let w = rgba.width();
    let (status, op) = if w <= COLUMN {
        (Status::Pass, "<=")
    } else {
        (Status::Fail, ">")
    };
    Outcome {
        name: "width",
        status,
        detail: format!("{w} px {op} {COLUMN} px"),
    }
}

fn opaque(rgba: &RgbaImage) -> Outcome {
    let n = rgba.pixels().filter(|p| p.0[3] < 255).count();
    if n == 0 {
        pass("opaque", "no transparent pixel")
    } else {
        Outcome {
            name: "opaque",
            status: Status::Fail,
            detail: format!("{n} transparent pixels"),
        }
    }
}

fn min_font_size(rgba: &RgbaImage, font_px: Option<f32>, framed: bool) -> Outcome {
    let name = "min-font-size";
    let px = match font_px {
        Some(px) => Some(px),
        None => {
            // A frame is not text, so the estimate looks inside it.
            let est = if framed {
                let inner = imageops::crop_imm(
                    rgba,
                    BAND,
                    BAND,
                    rgba.width() - 2 * BAND,
                    rgba.height() - 2 * BAND,
                )
                .to_image();
                font::estimate(&flatten_to_gray(&inner))
            } else {
                font::estimate(&flatten_to_gray(rgba))
            };
            font::usable_px(est.px, est.confidence)
        }
    };
    let Some(px) = px else {
        return Outcome {
            name,
            status: Status::Review,
            detail: "font size estimate has low confidence".to_string(),
        };
    };
    let displayed = px * (COLUMN as f32 / rgba.width() as f32).min(1.0);
    let (status, op) = if displayed >= MIN_FONT_PX as f32 {
        (Status::Pass, ">=")
    } else {
        (Status::Fail, "<")
    };
    Outcome {
        name,
        status,
        detail: format!("{displayed:.1} px {op} {MIN_FONT_PX} px"),
    }
}

/// True when the outer ring is exactly the frame colour and the padding band
/// inside it is exactly white. An image smaller than the two bands plus one
/// pixel is never framed.
pub fn is_framed(rgba: &RgbaImage) -> bool {
    let (w, h) = rgba.dimensions();
    if w < 2 * BAND + 1 || h < 2 * BAND + 1 {
        return false;
    }
    let [fr, fg, fb] = FRAME_COLOUR;
    rgba.enumerate_pixels().all(|(x, y, p)| {
        // Distance to the nearest edge: 0 on the ring, 1..=PADDING in the band.
        let d = x.min(y).min(w - 1 - x).min(h - 1 - y);
        if d < FRAME {
            p.0 == [fr, fg, fb, 255]
        } else if d < BAND {
            p.0 == [255, 255, 255, 255]
        } else {
            true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::Confidence;
    use image::Rgba;

    fn white(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]))
    }

    /// White image with a one pixel frame ring.
    fn ring(w: u32, h: u32) -> RgbaImage {
        let [r, g, b] = FRAME_COLOUR;
        let mut img = white(w, h);
        for (x, y, p) in img.enumerate_pixels_mut() {
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                *p = Rgba([r, g, b, 255]);
            }
        }
        img
    }

    fn find(outcomes: &[Outcome], name: &str) -> Outcome {
        outcomes.iter().find(|o| o.name == name).unwrap().clone()
    }

    #[test]
    fn width_800_passes_801_fails() {
        assert_eq!(width(&white(800, 40)).status, Status::Pass);
        assert_eq!(width(&white(801, 40)).status, Status::Fail);
    }

    #[test]
    fn opaque_fails_on_one_transparent_pixel() {
        let mut img = white(10, 10);
        assert_eq!(opaque(&img).status, Status::Pass);
        img.put_pixel(3, 3, Rgba([255, 255, 255, 254]));
        assert_eq!(opaque(&img).status, Status::Fail);
    }

    #[test]
    fn framed_detects_exact_border() {
        assert!(is_framed(&ring(100, 60)));
    }

    #[test]
    fn framed_rejects_wrong_ring_colour() {
        let mut img = ring(100, 60);
        let [r, g, b] = FRAME_COLOUR;
        img.put_pixel(50, 0, Rgba([r, g, b.wrapping_add(1), 255]));
        assert!(!is_framed(&img));
    }

    #[test]
    fn framed_rejects_non_white_padding() {
        let mut img = ring(100, 60);
        img.put_pixel(10, 10, Rgba([250, 255, 255, 255]));
        assert!(!is_framed(&img));
    }

    #[test]
    fn framed_fails_small_non_icon() {
        let outcomes = run_all(&ring(200, 20), Some(12.0));
        assert_eq!(find(&outcomes, "framed").status, Status::Fail);
    }

    #[test]
    fn icon_skips_font_and_frame() {
        let outcomes = run_all(&white(32, 32), None);
        for name in ["min-font-size", "framed"] {
            let o = find(&outcomes, name);
            assert_eq!((o.status, o.detail.as_str()), (Status::Pass, "icon"));
        }
    }

    #[test]
    fn icon_boundary_is_64_on_the_larger_side() {
        let icon = |w, h| find(&run_all(&white(w, h), None), "framed").detail == "icon";
        assert!(icon(64, 64));
        assert!(!icon(64, 65));
        assert!(!icon(65, 10));
    }

    #[test]
    fn framed_estimate_uses_the_inner_crop() {
        // White blocks on a black inner area: 3 lines of 7 glyphs, each
        // 6 x 11 px, so the estimate is 11 * K = 11.5 px.
        let (iw, ih) = (78, 68);
        let mut img = white(iw + 2 * BAND, ih + 2 * BAND);
        for y in 0..ih {
            for x in 0..iw {
                img.put_pixel(BAND + x, BAND + y, Rgba([0, 0, 0, 255]));
            }
        }
        for row in 0..3 {
            for col in 0..7 {
                for y in 0..11 {
                    for x in 0..6 {
                        let (px, py) = (BAND + 4 + col * 10 + x, BAND + 4 + row * 20 + y);
                        img.put_pixel(px, py, Rgba([255, 255, 255, 255]));
                    }
                }
            }
        }
        let [r, g, b] = FRAME_COLOUR;
        let (w, h) = img.dimensions();
        for (x, y, p) in img.enumerate_pixels_mut() {
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                *p = Rgba([r, g, b, 255]);
            }
        }
        assert!(is_framed(&img));
        // On the whole image the padding hides the lines: low confidence.
        let whole = font::estimate(&flatten_to_gray(&img));
        assert_eq!(whole.confidence, Confidence::Low);
        let o = find(&run_all(&img, None), "min-font-size");
        assert_eq!(
            (o.status, o.detail.as_str()),
            (Status::Pass, "11.5 px >= 11 px")
        );
    }

    #[test]
    fn outcomes_come_in_the_documented_order() {
        let names: Vec<_> = run_all(&white(100, 100), Some(12.0))
            .iter()
            .map(|o| o.name)
            .collect();
        assert_eq!(names, ["width", "min-font-size", "opaque", "framed"]);
    }

    #[test]
    fn font_override_scales_by_width() {
        let img = white(1600, 100);
        let low = run_all(&img, Some(20.0));
        assert_eq!(find(&low, "min-font-size").status, Status::Fail);
        let ok = run_all(&img, Some(22.0));
        assert_eq!(find(&ok, "min-font-size").status, Status::Pass);
    }

    #[test]
    fn low_confidence_is_review() {
        let outcomes = run_all(&white(300, 200), None);
        assert_eq!(find(&outcomes, "min-font-size").status, Status::Review);
    }

    #[test]
    fn low_confidence_with_a_size_is_review() {
        // Two lines of 6 x 5 px blocks: the estimate is about 5 px, which
        // would fail the check, but two lines give only low confidence.
        let mut img = white(128, 68);
        for row in 0..2 {
            for col in 0..12 {
                for y in 0..5 {
                    for x in 0..6 {
                        img.put_pixel(4 + col * 10 + x, 4 + row * 30 + y, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
        let est = font::estimate(&flatten_to_gray(&img));
        assert!(est.px.is_some(), "{est:?}");
        assert_eq!(est.confidence, Confidence::Low);
        let o = find(&run_all(&img, None), "min-font-size");
        assert_eq!(o.status, Status::Review);
    }

    #[test]
    fn framed_needs_35_px_on_both_sides() {
        assert!(is_framed(&ring(35, 100)));
        assert!(is_framed(&ring(100, 35)));
        assert!(!is_framed(&ring(34, 100)));
        assert!(!is_framed(&ring(100, 34)));
    }

    #[test]
    fn padding_band_is_16_px_wide() {
        // The pixel at distance 16 from the edge is padding; 17 is content.
        let mut img = ring(100, 60);
        img.put_pixel(16, 30, Rgba([0, 0, 0, 255]));
        assert!(!is_framed(&img));
        let mut img = ring(100, 60);
        img.put_pixel(17, 30, Rgba([0, 0, 0, 255]));
        assert!(is_framed(&img));
    }

    #[test]
    fn status_serialises_lowercase() {
        assert_eq!(
            serde_json::to_string(&Status::Review).unwrap(),
            "\"review\""
        );
    }
}
