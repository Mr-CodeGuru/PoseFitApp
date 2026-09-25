//! Image Preprocessing for On-Device Pose Estimation
//!
//! Handles RGB buffer scaling, letterboxing/aspect-ratio preservation,
//! and normalization into neural network tensor format.

/// Metadata required to reverse letterboxing in post-processing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LetterboxInfo {
    pub orig_width: u32,
    pub orig_height: u32,
    pub target_width: u32,
    pub target_height: u32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub scale: f32,
}

/// Preprocesses raw RGB image bytes (3 bytes per pixel: [R, G, B, R, G, B...])
/// into a flat normalized float buffer in NCHW or NHWC format (range: [0.0, 1.0]).
pub fn preprocess_rgb_letterbox(
    raw_rgb: &[u8],
    orig_width: u32,
    orig_height: u32,
    target_width: u32,
    target_height: u32,
) -> (Vec<f32>, LetterboxInfo) {
    let scale_x = target_width as f32 / orig_width as f32;
    let scale_y = target_height as f32 / orig_height as f32;
    let scale = scale_x.min(scale_y);

    let new_w = (orig_width as f32 * scale).round() as u32;
    let new_h = (orig_height as f32 * scale).round() as u32;

    let pad_x = ((target_width - new_w) as f32) / 2.0;
    let pad_y = ((target_height - new_h) as f32) / 2.0;

    let info = LetterboxInfo {
        orig_width,
        orig_height,
        target_width,
        target_height,
        pad_x,
        pad_y,
        scale,
    };

    // Target buffer filled with black (0.0) padding
    let total_pixels = (target_width * target_height) as usize;
    let mut normalized = vec![0.0f32; total_pixels * 3];

    // Bilinear or nearest-neighbor sampling for scaled region
    for ty in 0..target_height {
        let ty_f = ty as f32;
        if ty_f < pad_y || ty_f >= pad_y + (new_h as f32) {
            continue;
        }

        let sy_f = (ty_f - pad_y) / scale;
        let sy = (sy_f.floor() as u32).min(orig_height - 1);

        for tx in 0..target_width {
            let tx_f = tx as f32;
            if tx_f < pad_x || tx_f >= pad_x + (new_w as f32) {
                continue;
            }

            let sx_f = (tx_f - pad_x) / scale;
            let sx = (sx_f.floor() as u32).min(orig_width - 1);

            let src_idx = ((sy * orig_width + sx) * 3) as usize;
            let dst_idx = ((ty * target_width + tx) * 3) as usize;

            if src_idx + 2 < raw_rgb.len() && dst_idx + 2 < normalized.len() {
                normalized[dst_idx] = raw_rgb[src_idx] as f32 / 255.0;
                normalized[dst_idx + 1] = raw_rgb[src_idx + 1] as f32 / 255.0;
                normalized[dst_idx + 2] = raw_rgb[src_idx + 2] as f32 / 255.0;
            }
        }
    }

    (normalized, info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letterbox_padding_calculation() {
        let (orig_w, orig_h) = (1920, 1080);
        let (target_w, target_h) = (256, 256);

        let dummy_rgb = vec![128u8; (orig_w * orig_h * 3) as usize];
        let (buf, info) = preprocess_rgb_letterbox(&dummy_rgb, orig_w, orig_h, target_w, target_h);

        assert_eq!(buf.len(), (256 * 256 * 3) as usize);
        assert_eq!(info.pad_x, 0.0);
        assert!(info.pad_y > 0.0); // Widescreen image has top/bottom letterbox padding
        assert!((info.scale - (256.0 / 1920.0)).abs() < 1e-4);
    }
}
