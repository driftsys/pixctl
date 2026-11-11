//! Rendering contract shared by `inspect`, `fit` and `check`.
//!
//! All sizes are in pixels.

/// Width of the display column that images are fitted to.
pub const COLUMN: u32 = 800;
/// Smallest estimated font size that is still readable.
pub const MIN_FONT_PX: u32 = 11;
/// Font size that upscaling aims for.
pub const TARGET_FONT_PX: u32 = 12;
/// Padding between the frame and the image content, on each side.
pub const PADDING: u32 = 16;
/// Thickness of the frame drawn around an image, on each side.
pub const FRAME: u32 = 1;
/// RGB colour of the frame.
pub const FRAME_COLOUR: [u8; 3] = [0xd0, 0xd7, 0xde];
/// Largest side length of an image that is treated as an icon.
pub const ICON_MAX: u32 = 64;
/// Width available to the image content inside the padded frame.
pub const CONTENT_WIDTH: u32 = COLUMN - 2 * (PADDING + FRAME);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_width_is_766() {
        assert_eq!(CONTENT_WIDTH, 766);
        assert_eq!(FRAME_COLOUR, [208, 215, 222]);
    }
}
