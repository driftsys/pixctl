//! The display checks that `check` runs on one decoded image.

use crate::contract::{COLUMN, FRAME, FRAME_COLOUR, ICON_MAX, MIN_FONT_PX, PADDING};
use crate::font::{self, Confidence};
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
            match est.confidence {
                Confidence::High => est.px,
                Confidence::Low => None,
            }
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
        img.put_pixel(50, 0, Rgba([0xd0, 0xd7, 0xdf, 255]));
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
    fn status_serialises_lowercase() {
        assert_eq!(
            serde_json::to_string(&Status::Review).unwrap(),
            "\"review\""
        );
    }
}
