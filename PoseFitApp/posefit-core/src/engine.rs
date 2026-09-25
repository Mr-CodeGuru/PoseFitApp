use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::definitions::{ExerciseConfig, load_from_str};
use crate::error::PoseFitError;
use crate::exercise::{Exercise, ExerciseResult};
use crate::exercises::create_exercise;
use crate::landmarks::Landmark;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutSummary {
    pub exercise_name: String,
    pub total_reps: u32,
    pub counter_left: u32,
    pub counter_right: u32,
    pub total_duration_sec: f64,
    pub avg_form_score: u32,
    pub final_grade: char,
}

/// Abstract pose estimator trait for BlazePose or mock inference.
pub trait PoseEstimator: Send + Sync {
    fn estimate(
        &mut self,
        frame_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<Landmark>, PoseFitError>;
}

/// Mock pose estimator for testing and headless execution without MediaPipe.
pub struct MockPoseEstimator {
    pub next_landmarks: Vec<Landmark>,
}

impl MockPoseEstimator {
    pub fn new() -> Self {
        Self {
            next_landmarks: vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33],
        }
    }

    pub fn set_landmarks(&mut self, landmarks: Vec<Landmark>) {
        self.next_landmarks = landmarks;
    }
}

impl Default for MockPoseEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl PoseEstimator for MockPoseEstimator {
    fn estimate(
        &mut self,
        _frame_data: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<Vec<Landmark>, PoseFitError> {
        Ok(self.next_landmarks.clone())
    }
}

/// Central application engine coordinating exercise execution, state management,
/// and landmark processing. Matches Python `ExerciseEngine`.
pub struct PoseFitEngine {
    configs: HashMap<String, ExerciseConfig>,
    active_exercise: Option<Box<dyn Exercise>>,
    workout_start_time: Option<f64>,
    workout_last_time: Option<f64>,
}

impl Default for PoseFitEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PoseFitEngine {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            active_exercise: None,
            workout_start_time: None,
            workout_last_time: None,
        }
    }

    /// Registers a single exercise from a YAML configuration string.
    pub fn register_exercise_yaml(&mut self, yaml_str: &str) -> Result<(), PoseFitError> {
        let config = load_from_str(yaml_str)?;
        self.configs.insert(config.name.clone(), config);
        Ok(())
    }

    /// Pre-loads and registers all 18 bundled exercise definitions.
    pub fn register_all_bundled_exercises(&mut self) -> Result<(), PoseFitError> {
        let bundled = [
            include_str!("../assets/exercises/bicep_curl.yaml"),
            include_str!("../assets/exercises/calf_raise.yaml"),
            include_str!("../assets/exercises/deadlift.yaml"),
            include_str!("../assets/exercises/glute_bridge.yaml"),
            include_str!("../assets/exercises/hammer_curl.yaml"),
            include_str!("../assets/exercises/high_knees.yaml"),
            include_str!("../assets/exercises/jumping_jack.yaml"),
            include_str!("../assets/exercises/lateral_raise.yaml"),
            include_str!("../assets/exercises/leg_raise.yaml"),
            include_str!("../assets/exercises/lunge.yaml"),
            include_str!("../assets/exercises/mountain_climber.yaml"),
            include_str!("../assets/exercises/plank.yaml"),
            include_str!("../assets/exercises/push_up.yaml"),
            include_str!("../assets/exercises/shoulder_press.yaml"),
            include_str!("../assets/exercises/side_lunge.yaml"),
            include_str!("../assets/exercises/squat.yaml"),
            include_str!("../assets/exercises/tricep_dip.yaml"),
            include_str!("../assets/exercises/wall_sit.yaml"),
        ];

        for yaml_str in bundled {
            self.register_exercise_yaml(yaml_str)?;
        }

        Ok(())
    }

    /// Returns list of all registered exercise names.
    pub fn available_exercises(&self) -> Vec<String> {
        let mut list: Vec<String> = self.configs.keys().cloned().collect();
        list.sort();
        list
    }

    /// Starts tracking the specified exercise.
    pub fn start_exercise(&mut self, exercise_name: &str) -> Result<(), PoseFitError> {
        let config = self
            .configs
            .get(exercise_name)
            .cloned()
            .ok_or_else(|| PoseFitError::ExerciseNotFound(exercise_name.to_string()))?;

        let exercise = create_exercise(config)?;
        self.active_exercise = Some(exercise);
        self.workout_start_time = None;
        self.workout_last_time = None;
        Ok(())
    }

    /// Ingests a frame's landmarks, processes FSM, counting, and scoring.
    pub fn process_landmarks(
        &mut self,
        landmarks: &[Landmark],
        width: u32,
        height: u32,
        timestamp_sec: f64,
    ) -> Result<ExerciseResult, PoseFitError> {
        if self.workout_start_time.is_none() {
            self.workout_start_time = Some(timestamp_sec);
        }
        self.workout_last_time = Some(timestamp_sec);

        let exercise = self
            .active_exercise
            .as_mut()
            .ok_or_else(|| PoseFitError::InvalidState("No active exercise running".into()))?;

        exercise.process_frame(landmarks, width, height, timestamp_sec)
    }

    /// Stops tracking the exercise and generates a complete workout summary.
    pub fn stop_exercise(&mut self) -> Result<WorkoutSummary, PoseFitError> {
        let exercise = self
            .active_exercise
            .take()
            .ok_or_else(|| PoseFitError::InvalidState("No active exercise running".into()))?;

        let status = exercise.get_status();
        let start_t = self.workout_start_time.unwrap_or(0.0);
        let end_t = self.workout_last_time.unwrap_or(start_t);
        let total_duration = (end_t - start_t).max(0.0);

        Ok(WorkoutSummary {
            exercise_name: status.exercise_name,
            total_reps: status.counter,
            counter_left: status.counter_left,
            counter_right: status.counter_right,
            total_duration_sec: total_duration,
            avg_form_score: status.avg_form_score,
            final_grade: status.form_grade.as_char(),
        })
    }

    /// Resets the currently active exercise.
    pub fn reset(&mut self) {
        if let Some(ref mut exercise) = self.active_exercise {
            exercise.reset();
        }
        self.workout_start_time = None;
        self.workout_last_time = None;
    }

    /// Current status of the active exercise.
    pub fn status(&self) -> Option<ExerciseResult> {
        self.active_exercise.as_ref().map(|ex| ex.get_status())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_bundled_initialization() {
        let mut engine = PoseFitEngine::new();
        engine.register_all_bundled_exercises().unwrap();
        let available = engine.available_exercises();
        assert_eq!(available.len(), 18);
        assert!(available.contains(&"squat".to_string()));
        assert!(available.contains(&"hammer_curl".to_string()));
        assert!(available.contains(&"plank".to_string()));
    }

    #[test]
    fn test_engine_lifecycle_flow() {
        let mut engine = PoseFitEngine::new();
        engine.register_all_bundled_exercises().unwrap();
        engine.start_exercise("squat").unwrap();

        let landmarks = vec![Landmark::new(0.5, 0.5, 0.0, 1.0, 1.0); 33];
        let res = engine
            .process_landmarks(&landmarks, 1280, 720, 1.0)
            .unwrap();
        assert_eq!(res.exercise_name, "squat");

        let summary = engine.stop_exercise().unwrap();
        assert_eq!(summary.exercise_name, "squat");
    }
}
