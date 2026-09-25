//! Complete Workout HUD and Skeleton Overlay Renderer
//!
//! Renders skeleton lines, joint angles, rep counters, form score badges,
//! and feedback banners directly into raw frame buffers (matching Android ANativeWindow_Buffer).

use super::font::EmbeddedFont;
use super::primitives::{
    FrameBuffer, draw_filled_circle_rgba, draw_filled_rect_rgba, draw_line_rgba,
};
use crate::engine::WorkoutSummary;
use crate::exercise::ExerciseResult;
use crate::landmarks::Landmark;
use crate::scoring::Grade;

/// MediaPipe skeleton connection pairs (point_a, point_b).
pub const SKELETON_CONNECTIONS: &[(usize, usize)] = &[
    (11, 12), // Shoulders
    (11, 13), // Left upper arm
    (13, 15), // Left forearm
    (12, 14), // Right upper arm
    (14, 16), // Right forearm
    (11, 23), // Left torso
    (12, 24), // Right torso
    (23, 24), // Hips
    (23, 25), // Left thigh
    (25, 27), // Left shin
    (24, 26), // Right thigh
    (26, 28), // Right shin
];

pub struct HudRenderer;

impl HudRenderer {
    /// Returns RGBA color for a form grade badge.
    pub fn grade_color(grade: Grade) -> [u8; 4] {
        match grade {
            Grade::A => [46, 204, 113, 255], // Bright Green
            Grade::B => [52, 152, 219, 255], // Sky Blue
            Grade::C => [241, 196, 15, 255], // Yellow
            Grade::D => [230, 126, 34, 255], // Orange
            Grade::F => [231, 76, 60, 255],  // Crimson Red
        }
    }

    /// Renders skeleton lines and joint dots onto an RGBA image buffer.
    pub fn render_skeleton(fb: &mut FrameBuffer, landmarks: &[Landmark], min_visibility: f32) {
        if landmarks.len() < 33 {
            return;
        }

        // Draw connections
        for &(i1, i2) in SKELETON_CONNECTIONS {
            let lm1 = &landmarks[i1];
            let lm2 = &landmarks[i2];

            if lm1.visibility < min_visibility || lm2.visibility < min_visibility {
                continue;
            }

            let (x1, y1) = lm1.to_pixel_coords(fb.width, fb.height);
            let (x2, y2) = lm2.to_pixel_coords(fb.width, fb.height);

            // Color-code arms differently for bilateral visibility
            let color = if (i1 == 11 && i2 == 13) || (i1 == 13 && i2 == 15) {
                [0, 200, 255, 220] // Cyan for Left Arm
            } else if (i1 == 12 && i2 == 14) || (i1 == 14 && i2 == 16) {
                [255, 170, 0, 220] // Orange for Right Arm
            } else {
                [240, 240, 240, 180] // Off-white for torso & legs
            };

            draw_line_rgba(fb, x1, y1, x2, y2, 4, color);
        }

        // Draw key joint dots
        let key_joints = [11, 12, 13, 14, 15, 16, 23, 24, 25, 26, 27, 28];
        for &idx in &key_joints {
            let lm = &landmarks[idx];
            if lm.visibility >= min_visibility {
                let (cx, cy) = lm.to_pixel_coords(fb.width, fb.height);
                draw_filled_circle_rgba(fb, cx, cy, 5, [255, 255, 255, 255]);
                draw_filled_circle_rgba(fb, cx, cy, 3, [40, 40, 40, 255]);
            }
        }
    }

    /// Renders the complete HUD overlay (top panel, rep counters, score badge, feedback).
    pub fn render_hud(fb: &mut FrameBuffer, result: &ExerciseResult) {
        let hud_h = 100u32.min(fb.height / 4);

        // 1. Top dark translucent HUD bar
        draw_filled_rect_rgba(fb, 0, 0, fb.width, hud_h, [15, 18, 25, 210]);

        // 2. Exercise Title & Rep Counter
        let title_text = format!(
            "EXERCISE: {} [TAP TO CYCLE]",
            result.exercise_name.to_uppercase()
        );
        EmbeddedFont::draw_text_rgba(fb, 20, 15, &title_text, 2, [255, 255, 255, 255]);

        // Total Reps
        let reps_text = if result.counter_left > 0 || result.counter_right > 0 {
            format!(
                "REPS: {} (L:{} | R:{})",
                result.counter, result.counter_left, result.counter_right
            )
        } else {
            format!("TOTAL REPS: {}", result.counter)
        };
        EmbeddedFont::draw_text_rgba(fb, 20, 55, &reps_text, 2, [0, 230, 255, 255]);

        // 3. States
        let state_text = if let (Some(l), Some(r)) = (&result.state_left, &result.state_right) {
            format!("L:[{}] R:[{}]", l.to_uppercase(), r.to_uppercase())
        } else if let Some(s) = &result.current_state {
            format!("STATE: [{}]", s.to_uppercase())
        } else {
            "STATE: [IDLE]".to_string()
        };
        let state_x = (fb.width as i32 / 2) - 50;
        EmbeddedFont::draw_text_rgba(fb, state_x, 55, &state_text, 2, [255, 255, 255, 230]);

        // 4. Form Grade Pill Badge (Top Right)
        let grade_col = Self::grade_color(result.form_grade);
        let badge_w = 200u32;
        let badge_x = (fb.width as i32) - (badge_w as i32) - 20;

        draw_filled_rect_rgba(
            fb,
            badge_x,
            20,
            badge_w,
            50,
            [grade_col[0] / 4, grade_col[1] / 4, grade_col[2] / 4, 220],
        );

        let score_text = format!(
            "FORM: {}% ({})",
            result.form_score,
            result.form_grade.as_char()
        );
        EmbeddedFont::draw_text_rgba(fb, badge_x + 15, 33, &score_text, 2, grade_col);

        // 5. Active Feedback Alerts (Bottom Banner)
        if let Some(first_alert) = result.feedback_alerts.first() {
            let banner_h = 55u32;
            let banner_y = (fb.height as i32) - (banner_h as i32) - 20;

            draw_filled_rect_rgba(
                fb,
                30,
                banner_y,
                fb.width - 60,
                banner_h,
                [220, 30, 30, 220],
            );

            let alert_msg = format!("! FORM ALERT: {}", first_alert.message.to_uppercase());
            EmbeddedFont::draw_text_rgba(
                fb,
                50,
                banner_y + 18,
                &alert_msg,
                2,
                [255, 255, 255, 255],
            );
        }
    }

    /// Renders a full-screen workout summary scorecard directly into the FrameBuffer.
    pub fn render_summary_card(fb: &mut FrameBuffer, summary: &WorkoutSummary) {
        // Dim the entire background
        draw_filled_rect_rgba(fb, 0, 0, fb.width, fb.height, [10, 14, 20, 230]);

        let card_w = (fb.width as i32 - 80).clamp(280, 600) as u32;
        let card_h = (fb.height as i32 - 100).clamp(320, 480) as u32;
        let card_x = (fb.width as i32 - card_w as i32) / 2;
        let card_y = (fb.height as i32 - card_h as i32) / 2;

        // Draw card background
        draw_filled_rect_rgba(fb, card_x, card_y, card_w, card_h, [25, 32, 45, 255]);

        // Card Title
        EmbeddedFont::draw_text_rgba(
            fb,
            card_x + 30,
            card_y + 30,
            "WORKOUT SUMMARY",
            3,
            [0, 230, 255, 255],
        );

        // Exercise Name
        let ex_str = format!("EXERCISE: {}", summary.exercise_name.to_uppercase());
        EmbeddedFont::draw_text_rgba(
            fb,
            card_x + 30,
            card_y + 90,
            &ex_str,
            2,
            [255, 255, 255, 255],
        );

        // Reps
        let reps_str = if summary.counter_left > 0 || summary.counter_right > 0 {
            format!(
                "TOTAL REPS: {} (L:{} | R:{})",
                summary.total_reps, summary.counter_left, summary.counter_right
            )
        } else {
            format!("TOTAL REPS: {}", summary.total_reps)
        };
        EmbeddedFont::draw_text_rgba(
            fb,
            card_x + 30,
            card_y + 130,
            &reps_str,
            2,
            [46, 204, 113, 255],
        );

        // Duration
        let dur_str = format!("DURATION: {:03.1} SECONDS", summary.total_duration_sec);
        EmbeddedFont::draw_text_rgba(
            fb,
            card_x + 30,
            card_y + 170,
            &dur_str,
            2,
            [241, 196, 15, 255],
        );

        // Form Score & Grade
        let score_str = format!(
            "AVG FORM SCORE: {}% (GRADE {})",
            summary.avg_form_score, summary.final_grade
        );
        let grade_color = match summary.final_grade {
            'A' => [46, 204, 113, 255],
            'B' => [52, 152, 219, 255],
            'C' => [241, 196, 15, 255],
            'D' => [230, 126, 34, 255],
            _ => [231, 76, 60, 255],
        };
        EmbeddedFont::draw_text_rgba(fb, card_x + 30, card_y + 210, &score_str, 2, grade_color);

        // Dismiss CTA
        EmbeddedFont::draw_text_rgba(
            fb,
            card_x + 30,
            card_y + card_h as i32 - 50,
            ">> TAP SCREEN TO START NEXT WORKOUT <<",
            2,
            [180, 190, 200, 255],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_render_skeleton_and_hud() {
        let (w, h) = (640, 480);
        let stride = w * 4;
        let mut buffer = vec![0u8; (w * h * 4) as usize];
        let mut fb = FrameBuffer::new(&mut buffer, w, h, stride);

        let landmarks = vec![Landmark::new(0.5, 0.5, 0.0, 0.9, 0.9); 33];
        HudRenderer::render_skeleton(&mut fb, &landmarks, 0.5);

        let res = ExerciseResult {
            exercise_name: "hammer_curl".to_string(),
            counter: 5,
            counter_left: 2,
            counter_right: 3,
            current_state: Some("up".to_string()),
            prev_state: Some("start".to_string()),
            state_left: Some("up".to_string()),
            state_right: Some("flex".to_string()),
            angles: HashMap::new(),
            form_score: 95,
            avg_form_score: 93,
            form_grade: Grade::A,
            rep_completed: true,
            feedback_alerts: vec![],
            is_holding: false,
            current_duration: 0.0,
            target_duration: None,
        };

        HudRenderer::render_hud(&mut fb, &res);

        let non_zero_pixels = fb.buffer.iter().filter(|&&b| b > 0).count();
        assert!(
            non_zero_pixels > 1000,
            "HUD should draw noticeable visual content"
        );
    }

    #[test]
    fn test_render_summary_card() {
        let (w, h) = (640, 480);
        let stride = w * 4;
        let mut buffer = vec![0u8; (w * h * 4) as usize];
        let mut fb = FrameBuffer::new(&mut buffer, w, h, stride);

        let summary = WorkoutSummary {
            exercise_name: "squat".to_string(),
            total_reps: 12,
            counter_left: 0,
            counter_right: 0,
            total_duration_sec: 45.0,
            avg_form_score: 92,
            final_grade: 'A',
        };

        HudRenderer::render_summary_card(&mut fb, &summary);
        let non_zero = fb.buffer.iter().filter(|&&b| b > 0).count();
        assert!(non_zero > 5000, "Summary card should render content");
    }
}
