use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::feedback::Severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExerciseKind {
    Repetition,
    Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AngleConfig {
    pub points: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    #[serde(default)]
    pub condition: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterConfig {
    pub trigger_state: String,
    pub from_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackConfig {
    #[serde(default)]
    pub condition: String,
    #[serde(default = "default_feedback_message")]
    pub message: String,
    #[serde(default)]
    pub severity: Severity,
}

fn default_feedback_message() -> String {
    "Form warning".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoConfig {
    #[serde(default = "default_tempo_min")]
    pub min: f64,
    #[serde(default = "default_tempo_max")]
    pub max: f64,
}

fn default_tempo_min() -> f64 {
    1.0
}
fn default_tempo_max() -> f64 {
    3.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmoothingConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_window")]
    pub window: usize,
}

fn default_true() -> bool {
    true
}
fn default_window() -> usize {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub reps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseConfig {
    pub name: String,
    pub display_name: Option<String>,
    #[serde(rename = "type")]
    pub exercise_type: ExerciseKind,
    #[serde(default)]
    pub bilateral: bool,
    #[serde(default)]
    pub sides: Vec<String>,
    pub target_duration: Option<f64>,
    pub hold_state: Option<String>,
    #[serde(default)]
    pub angles: HashMap<String, AngleConfig>,
    #[serde(default)]
    pub state_order: Vec<String>,
    #[serde(default)]
    pub states: HashMap<String, StateConfig>,
    pub counter: Option<CounterConfig>,
    pub min_rep_duration: Option<f64>,
    #[serde(default)]
    pub feedback: HashMap<String, FeedbackConfig>,
    pub tempo: Option<TempoConfig>,
    pub ideal_angles: Option<HashMap<String, f64>>,
    pub smoothing: Option<SmoothingConfig>,
    pub calibration: Option<CalibrationConfig>,
    pub visualization: Option<serde_yaml::Value>,
    pub target_muscles: Option<Vec<String>>,
    pub equipment: Option<String>,
    pub difficulty: Option<String>,
    pub default_reps: Option<u32>,
    pub default_sets: Option<u32>,
    pub rest_time: Option<String>,
    pub benefits: Option<Vec<String>>,
    pub description: Option<String>,
}
