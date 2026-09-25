//! Landmark Postprocessing
//!
//! Transforms raw neural network output coordinates (e.g. BlazePose 33 landmarks)
//! back into unpadded normalized screen coordinates [0.0, 1.0].

use super::preprocessing::LetterboxInfo;
use crate::landmarks::Landmark;

/// Sigmoid activation function for logit confidence scores.
#[inline]
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Decodes raw BlazePose landmark tensor into 33 `Landmark` structs.
///
/// BlazePose output format:
/// - 33 landmarks * 5 floats = 165 floats (x, y, z, visibility, presence)
/// - or 33 landmarks * 3 floats = 99 floats (x, y, z) + auxiliary confidence tensors
pub fn decode_blazepose_landmarks(
    raw_landmarks: &[f32],
    info: &LetterboxInfo,
    is_logits: bool,
) -> Vec<Landmark> {
    let mut landmarks = Vec::with_capacity(33);

    // Determine stride: 5 floats per landmark (x, y, z, vis, pres) or 3 (x, y, z)
    let stride = if raw_landmarks.len() >= 165 { 5 } else { 3 };

    let active_w = info.target_width as f32 - (2.0 * info.pad_x);
    let active_h = info.target_height as f32 - (2.0 * info.pad_y);

    for i in 0..33 {
        let base = i * stride;
        if base + 2 >= raw_landmarks.len() {
            // Fill default fallback if truncated
            landmarks.push(Landmark::new(0.0, 0.0, 0.0, 0.0, 0.0));
            continue;
        }

        let raw_x = raw_landmarks[base];
        let raw_y = raw_landmarks[base + 1];
        let raw_z = raw_landmarks[base + 2];

        // Convert from model target coordinates to unpadded [0.0, 1.0] normalized space
        let norm_x = if active_w > 0.0 {
            let px = if raw_x <= 1.0 {
                raw_x * info.target_width as f32
            } else {
                raw_x
            };
            ((px - info.pad_x) / active_w).clamp(0.0, 1.0)
        } else {
            raw_x.clamp(0.0, 1.0)
        };

        let norm_y = if active_h > 0.0 {
            let py = if raw_y <= 1.0 {
                raw_y * info.target_height as f32
            } else {
                raw_y
            };
            ((py - info.pad_y) / active_h).clamp(0.0, 1.0)
        } else {
            raw_y.clamp(0.0, 1.0)
        };

        let (visibility, presence) = if stride >= 5 {
            let mut vis = raw_landmarks[base + 3];
            let mut pres = raw_landmarks[base + 4];
            if is_logits {
                vis = sigmoid(vis);
                pres = sigmoid(pres);
            }
            (vis.clamp(0.0, 1.0), pres.clamp(0.0, 1.0))
        } else {
            (1.0, 1.0)
        };

        landmarks.push(Landmark::new(norm_x, norm_y, raw_z, visibility, presence));
    }

    landmarks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_blazepose_landmarks_inversion() {
        let info = LetterboxInfo {
            orig_width: 1920,
            orig_height: 1080,
            target_width: 256,
            target_height: 256,
            pad_x: 0.0,
            pad_y: 56.0,
            scale: 256.0 / 1920.0,
        };

        // Center point in target coordinate space
        let mut raw = vec![0.0f32; 165];
        raw[0] = 128.0; // x center
        raw[1] = 128.0; // y center
        raw[2] = 0.0;
        raw[3] = 0.95;
        raw[4] = 0.99;

        let decoded = decode_blazepose_landmarks(&raw, &info, false);
        assert_eq!(decoded.len(), 33);
        assert!((decoded[0].x - 0.5).abs() < 1e-3);
        assert!((decoded[0].y - 0.5).abs() < 1e-3);
        assert_eq!(decoded[0].visibility, 0.95);
        assert_eq!(decoded[0].presence, 0.99);
    }
}
