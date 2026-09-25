use serde::{Deserialize, Serialize};

/// Represents a single 3D pose landmark.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub visibility: f32,
    pub presence: f32,
}

impl Landmark {
    pub const fn new(x: f32, y: f32, z: f32, visibility: f32, presence: f32) -> Self {
        Self {
            x,
            y,
            z,
            visibility,
            presence,
        }
    }

    /// Converts normalized landmark coordinates [0.0, 1.0] to pixel space (x, y)
    /// exactly matching Python: `(int(lm.x * w), int(lm.y * h))`.
    pub fn to_pixel_coords(&self, width: u32, height: u32) -> (i32, i32) {
        let px = (self.x * width as f32) as i32;
        let py = (self.y * height as f32) as i32;
        (px, py)
    }
}

// MediaPipe landmark indices used in PoseFit
pub const NOSE: usize = 0;
pub const LEFT_SHOULDER: usize = 11;
pub const RIGHT_SHOULDER: usize = 12;
pub const LEFT_ELBOW: usize = 13;
pub const RIGHT_ELBOW: usize = 14;
pub const LEFT_WRIST: usize = 15;
pub const RIGHT_WRIST: usize = 16;
pub const LEFT_HIP: usize = 23;
pub const RIGHT_HIP: usize = 24;
pub const LEFT_KNEE: usize = 25;
pub const RIGHT_KNEE: usize = 26;
pub const LEFT_ANKLE: usize = 27;
pub const RIGHT_ANKLE: usize = 28;

pub const LANDMARK_NAMES: [(&str, usize); 13] = [
    ("nose", NOSE),
    ("left_shoulder", LEFT_SHOULDER),
    ("right_shoulder", RIGHT_SHOULDER),
    ("left_elbow", LEFT_ELBOW),
    ("right_elbow", RIGHT_ELBOW),
    ("left_wrist", LEFT_WRIST),
    ("right_wrist", RIGHT_WRIST),
    ("left_hip", LEFT_HIP),
    ("right_hip", RIGHT_HIP),
    ("left_knee", LEFT_KNEE),
    ("right_knee", RIGHT_KNEE),
    ("left_ankle", LEFT_ANKLE),
    ("right_ankle", RIGHT_ANKLE),
];

pub fn landmark_index_by_name(name: &str) -> Option<usize> {
    LANDMARK_NAMES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, idx)| *idx)
}

pub fn landmark_name_by_index(index: usize) -> Option<&'static str> {
    LANDMARK_NAMES
        .iter()
        .find(|(_, idx)| *idx == index)
        .map(|(name, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_landmark_indices() {
        assert_eq!(landmark_index_by_name("nose"), Some(0));
        assert_eq!(landmark_index_by_name("left_shoulder"), Some(11));
        assert_eq!(landmark_index_by_name("right_shoulder"), Some(12));
        assert_eq!(landmark_index_by_name("left_elbow"), Some(13));
        assert_eq!(landmark_index_by_name("right_elbow"), Some(14));
        assert_eq!(landmark_index_by_name("left_wrist"), Some(15));
        assert_eq!(landmark_index_by_name("right_wrist"), Some(16));
        assert_eq!(landmark_index_by_name("left_hip"), Some(23));
        assert_eq!(landmark_index_by_name("right_hip"), Some(24));
        assert_eq!(landmark_index_by_name("left_knee"), Some(25));
        assert_eq!(landmark_index_by_name("right_knee"), Some(26));
        assert_eq!(landmark_index_by_name("left_ankle"), Some(27));
        assert_eq!(landmark_index_by_name("right_ankle"), Some(28));
        assert_eq!(landmark_index_by_name("unknown"), None);
    }

    #[test]
    fn test_to_pixel_coords() {
        let lm = Landmark::new(0.5, 0.25, 0.0, 1.0, 1.0);
        let (px, py) = lm.to_pixel_coords(1280, 720);
        assert_eq!(px, 640);
        assert_eq!(py, 180);
    }
}
