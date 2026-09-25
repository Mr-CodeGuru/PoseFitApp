//! Pure-Rust HUD and Skeleton Overlay Rendering Module
//!
//! Provides fast, dependency-free raster rendering directly onto pixel buffers
//! (e.g. Android ANativeWindow_Buffer).

pub mod font;
pub mod hud;
pub mod primitives;

pub use font::EmbeddedFont;
pub use hud::{HudRenderer, SKELETON_CONNECTIONS};
pub use primitives::{
    FrameBuffer, blend_pixel_rgba, draw_filled_circle_rgba, draw_filled_rect_rgba, draw_line_rgba,
};
