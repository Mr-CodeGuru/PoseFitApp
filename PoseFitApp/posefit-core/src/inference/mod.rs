//! On-Device Neural Network Pose Inference Module
//!
//! Provides pure-Rust image preprocessing, BlazePose tensor decoding,
//! and inference coordination conforming to `PoseEstimator`.

pub mod blazepose;
pub mod postprocessing;
pub mod preprocessing;

pub use blazepose::{BlazePoseConfig, BlazePoseEstimator};
pub use postprocessing::decode_blazepose_landmarks;
pub use preprocessing::{LetterboxInfo, preprocess_rgb_letterbox};
