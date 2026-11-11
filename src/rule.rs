//! The fit rule: decides how `fit` treats one image.

use crate::contract::{CONTENT_WIDTH, FRAME, ICON_MAX, MIN_FONT_PX, PADDING, TARGET_FONT_PX};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    Icon,
    Unchanged,
    Upscale,
    TooSmall,
    Oversize,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Plan {
    pub rule: Rule,
    pub factor: f32,
    pub output_width: u32,
}

/// Width that `fit` adds around the content: padding and frame on both sides.
const BORDER: u32 = 2 * (PADDING + FRAME);

/// Decides the rule for a `w` x `h` image whose text is `font_px` pixels.
///
/// The caller passes `None` when the font size is unknown or not reliable;
/// `None` never produces an upscale.
pub fn plan(w: u32, h: u32, font_px: Option<f32>) -> Plan {
    let keep = |rule| Plan {
        rule,
        factor: 1.0,
        output_width: w + BORDER,
    };
    if w.max(h) <= ICON_MAX {
        return Plan {
            rule: Rule::Icon,
            factor: 1.0,
            output_width: w,
        };
    }
    if w > CONTENT_WIDTH {
        return keep(Rule::Oversize);
    }
    let font = match font_px {
        Some(font) if font < MIN_FONT_PX as f32 => font,
        _ => return keep(Rule::Unchanged),
    };
    let fits = |factor: f32| w as f32 * factor <= CONTENT_WIDTH as f32;
    let needed = (CONTENT_WIDTH as f32 / w as f32).min(TARGET_FONT_PX as f32 / font);
    let mut chosen = if needed <= 1.5 { 1.5 } else { 2.0 };
    if !fits(chosen) {
        chosen = 1.5;
    }
    if !fits(chosen) || font * chosen < MIN_FONT_PX as f32 {
        return keep(Rule::TooSmall);
    }
    Plan {
        rule: Rule::Upscale,
        factor: chosen,
        output_width: (w as f32 * chosen).round() as u32 + BORDER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_64_is_icon_65_is_not() {
        let p = plan(64, 10, None);
        assert_eq!(p.rule, Rule::Icon);
        assert_eq!(p.factor, 1.0);
        assert_eq!(p.output_width, 64);
        assert_ne!(plan(65, 10, None).rule, Rule::Icon);
    }

    #[test]
    fn width_767_is_oversize_766_is_not() {
        let p = plan(767, 100, Some(6.0));
        assert_eq!(p.rule, Rule::Oversize);
        assert_eq!(p.factor, 1.0);
        assert_ne!(plan(766, 100, None).rule, Rule::Oversize);
    }

    #[test]
    fn font_none_never_upscales() {
        let p = plan(300, 200, None);
        assert_eq!(p.rule, Rule::Unchanged);
        assert_eq!(p.factor, 1.0);
    }

    #[test]
    fn font_12_or_more_is_unchanged() {
        let p = plan(300, 200, Some(12.0));
        assert_eq!(p.rule, Rule::Unchanged);
        assert_eq!(p.factor, 1.0);
    }

    #[test]
    fn font_11_5_wide_is_unchanged() {
        let p = plan(600, 200, Some(11.5));
        assert_eq!(p.rule, Rule::Unchanged);
        assert_eq!(p.factor, 1.0);
    }

    #[test]
    fn font_5_gives_too_small() {
        let p = plan(300, 200, Some(5.0));
        assert_eq!(p.rule, Rule::TooSmall);
        assert_eq!(p.factor, 1.0);
    }

    #[test]
    fn font_8_at_300_wide_gives_1_5x() {
        let p = plan(300, 200, Some(8.0));
        assert_eq!(p.rule, Rule::Upscale);
        assert_eq!(p.factor, 1.5);
        assert_eq!(p.output_width, 450 + 34);
    }

    #[test]
    fn font_6_at_300_wide_gives_2x() {
        let p = plan(300, 200, Some(6.0));
        assert_eq!(p.rule, Rule::Upscale);
        assert_eq!(p.factor, 2.0);
        assert_eq!(p.output_width, 600 + 34);
    }

    #[test]
    fn two_x_falls_back_to_1_5() {
        // needed = min(1.91, 1.6) picks 2x, but 2x = 800 > 766, so 1.5x = 600.
        // A font of 7.5 px keeps 1.5x at 11.25 px, above the minimum.
        let p = plan(400, 200, Some(7.5));
        assert_eq!(p.rule, Rule::Upscale);
        assert_eq!(p.factor, 1.5);
        assert_eq!(p.output_width, 600 + 34);
    }

    #[test]
    fn fallback_below_minimum_is_too_small() {
        // The brief's original fallback case: 6 px x 1.5 = 9 px < 11 px.
        let p = plan(400, 200, Some(6.0));
        assert_eq!(p.rule, Rule::TooSmall);
        assert_eq!(p.factor, 1.0);
    }

    #[test]
    fn even_1_5_overflows_is_too_small() {
        let p = plan(600, 200, Some(6.0));
        assert_eq!(p.rule, Rule::TooSmall);
        assert_eq!(p.factor, 1.0);
    }
}
