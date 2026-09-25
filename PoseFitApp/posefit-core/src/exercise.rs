use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::PoseFitError;
use crate::feedback::FeedbackAlert;
use crate::landmarks::Landmark;
use crate::scoring::Grade;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExerciseResult {
    pub exercise_name: String,
    pub counter: u32,
    pub counter_left: u32,
    pub counter_right: u32,
    pub current_state: Option<String>,
    pub prev_state: Option<String>,
    pub state_left: Option<String>,
    pub state_right: Option<String>,
    pub current_duration: f64,
    pub target_duration: Option<f64>,
    pub is_holding: bool,
    pub angles: HashMap<String, f64>,
    pub form_score: u32,
    pub avg_form_score: u32,
    pub form_grade: Grade,
    pub feedback_alerts: Vec<FeedbackAlert>,
    pub rep_completed: bool,
}

pub trait Exercise: Send + Sync {
    fn name(&self) -> &str;
    fn process_frame(
        &mut self,
        landmarks: &[Landmark],
        frame_width: u32,
        frame_height: u32,
        timestamp_sec: f64,
    ) -> Result<ExerciseResult, PoseFitError>;
    fn reset(&mut self);
    fn get_status(&self) -> ExerciseResult;
}
