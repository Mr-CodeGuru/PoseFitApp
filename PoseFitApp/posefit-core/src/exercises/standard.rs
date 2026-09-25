use std::collections::HashMap;

use crate::definitions::schema::{CounterConfig, ExerciseConfig};
use crate::error::PoseFitError;
use crate::exercise::{Exercise, ExerciseResult};
use crate::feedback::{FeedbackAlert, FeedbackRule};
use crate::fsm::{ConditionParser, EvaluationContext, StateDefinition, StateMachine};
use crate::landmarks::Landmark;
use crate::math::calculate_angle_2d;
use crate::scoring::{FormScoreBreakdown, FormScoreCalculator, Grade};
use crate::smoothing::TemporalSmoother;

#[derive(Debug, Clone)]
struct AngleDef {
    points: [String; 3],
}

#[derive(Debug)]
pub struct StandardExercise {
    pub config: ExerciseConfig,
    angles_def: HashMap<String, AngleDef>,
    pub smoother: TemporalSmoother,
    fsm: StateMachine,
    counter_rule: Option<CounterConfig>,
    min_rep_duration: f64,
    feedback_rules: Vec<FeedbackRule>,
    score_calc: FormScoreCalculator,

    // Runtime state
    pub counter: u32,
    pub last_count_time: f64,
    pub rep_start_time: Option<f64>,
    pub rep_durations: Vec<f64>,
    pub rep_form_scores: Vec<u32>,
    pub current_angles: HashMap<String, f64>,
    pub current_feedback: Vec<FeedbackAlert>,
    pub current_score: FormScoreBreakdown,
    pub avg_form_score: u32,
    pub rep_completed: bool,
}

impl StandardExercise {
    pub fn new(config: ExerciseConfig) -> Result<Self, PoseFitError> {
        let mut angles_def = HashMap::new();
        for (name, angle_cfg) in &config.angles {
            if angle_cfg.points.len() == 3 {
                angles_def.insert(
                    name.clone(),
                    AngleDef {
                        points: [
                            angle_cfg.points[0].clone(),
                            angle_cfg.points[1].clone(),
                            angle_cfg.points[2].clone(),
                        ],
                    },
                );
            }
        }

        let smoothing_cfg = config.smoothing.as_ref();
        let smoother = TemporalSmoother::new(
            smoothing_cfg.map(|s| s.window).unwrap_or(3),
            smoothing_cfg.map(|s| s.enabled).unwrap_or(true),
        );

        let mut states = HashMap::new();
        for (state_name, state_cfg) in &config.states {
            let cond = ConditionParser::parse_str(&state_cfg.condition)?;
            states.insert(
                state_name.clone(),
                StateDefinition {
                    name: state_name.clone(),
                    condition_str: state_cfg.condition.clone(),
                    condition: cond,
                },
            );
        }

        let fsm = StateMachine::new(config.state_order.clone(), states);

        let mut feedback_rules = Vec::new();
        for (fb_name, fb_cfg) in &config.feedback {
            let cond = ConditionParser::parse_str(&fb_cfg.condition)?;
            feedback_rules.push(FeedbackRule {
                name: fb_name.clone(),
                condition_str: fb_cfg.condition.clone(),
                condition: cond,
                message: fb_cfg.message.clone(),
                severity: fb_cfg.severity,
            });
        }

        let tempo_min = config.tempo.as_ref().map(|t| t.min).unwrap_or(1.0);
        let tempo_max = config.tempo.as_ref().map(|t| t.max).unwrap_or(3.0);
        let ideal_angles = config.ideal_angles.clone().unwrap_or_default();
        let score_calc = FormScoreCalculator::new(ideal_angles, tempo_min, tempo_max);

        let min_rep_duration = config.min_rep_duration.unwrap_or(0.8);
        let counter_rule = config.counter.clone();

        Ok(Self {
            config,
            angles_def,
            smoother,
            fsm,
            counter_rule,
            min_rep_duration,
            feedback_rules,
            score_calc,
            counter: 0,
            last_count_time: 0.0,
            rep_start_time: None,
            rep_durations: Vec::new(),
            rep_form_scores: Vec::new(),
            current_angles: HashMap::new(),
            current_feedback: Vec::new(),
            current_score: FormScoreBreakdown {
                score: 100,
                grade: Grade::A,
                angle_penalty: 0,
                tempo_penalty: 0,
                feedback_penalty: 0,
            },
            avg_form_score: 100,
            rep_completed: false,
        })
    }

    fn extract_coords(
        landmarks: &[Landmark],
        width: u32,
        height: u32,
    ) -> HashMap<String, (i32, i32)> {
        let mut coords = HashMap::new();
        for &(name, idx) in &crate::landmarks::LANDMARK_NAMES {
            if idx < landmarks.len() {
                coords.insert(
                    name.to_string(),
                    landmarks[idx].to_pixel_coords(width, height),
                );
            }
        }
        coords
    }

    fn calculate_angles(&mut self, coords: &HashMap<String, (i32, i32)>) -> HashMap<String, f64> {
        let mut angles = HashMap::new();
        for (angle_name, def) in &self.angles_def {
            if let (Some(&p1), Some(&p2), Some(&p3)) = (
                coords.get(&def.points[0]),
                coords.get(&def.points[1]),
                coords.get(&def.points[2]),
            ) {
                let raw_angle = calculate_angle_2d(p1, p2, p3);
                let smoothed_angle = self.smoother.smooth(angle_name, raw_angle);
                angles.insert(angle_name.clone(), smoothed_angle);
            }
        }
        self.current_angles = angles.clone();
        angles
    }

    fn build_context(
        &self,
        angles: &HashMap<String, f64>,
        coords: &HashMap<String, (i32, i32)>,
    ) -> EvaluationContext {
        let mut ctx = EvaluationContext::new();
        ctx.primary_angle_name = Some("primary".to_string());
        for (k, &v) in angles {
            ctx.angles.insert(k.clone(), v);
        }
        for (k, &v) in coords {
            ctx.coords.insert(k.clone(), v);
        }
        ctx
    }
}

impl Exercise for StandardExercise {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn process_frame(
        &mut self,
        landmarks: &[Landmark],
        frame_width: u32,
        frame_height: u32,
        timestamp_sec: f64,
    ) -> Result<ExerciseResult, PoseFitError> {
        let coords = Self::extract_coords(landmarks, frame_width, frame_height);
        let angles = self.calculate_angles(&coords);
        let ctx = self.build_context(&angles, &coords);

        // 1. Update State Machine
        let (current_state, state_changed) = self.fsm.update(&ctx)?;

        if state_changed
            && let Some(ref rule) = self.counter_rule
            && let Some(ref from_state) = rule.from_state
            && current_state.as_deref() == Some(from_state.as_str())
        {
            self.rep_start_time = Some(timestamp_sec);
        }

        // 2. Feedback evaluation
        let mut active_feedback = Vec::new();
        for rule in &self.feedback_rules {
            if let Some(alert) = rule.evaluate(&ctx)? {
                active_feedback.push(alert);
            }
        }
        self.current_feedback = active_feedback.clone();

        // 3. Form Scoring
        let last_rep_duration = self.rep_durations.last().copied();
        self.current_score =
            self.score_calc
                .calculate(&ctx, last_rep_duration, self.current_feedback.len());

        // 4. Counter evaluation
        self.rep_completed = false;
        if let Some(ref rule) = self.counter_rule {
            let reached_trigger = self.fsm.current_state.as_deref() == Some(&rule.trigger_state);
            let prev_different = self.fsm.prev_state.as_deref() != Some(&rule.trigger_state);

            if reached_trigger && prev_different {
                let from_valid = match rule.from_state {
                    Some(ref fs) => self.fsm.prev_state.as_deref() == Some(fs.as_str()),
                    None => true,
                };

                let time_valid = (timestamp_sec - self.last_count_time) >= self.min_rep_duration;

                if from_valid && time_valid {
                    self.counter += 1;
                    self.last_count_time = timestamp_sec;
                    self.rep_completed = true;

                    if let Some(start_t) = self.rep_start_time {
                        let duration = timestamp_sec - start_t;
                        self.rep_durations.push(duration);
                        self.rep_start_time = None;
                    }

                    self.rep_form_scores.push(self.current_score.score);
                    if !self.rep_form_scores.is_empty() {
                        let sum: u32 = self.rep_form_scores.iter().sum();
                        self.avg_form_score = sum / (self.rep_form_scores.len() as u32);
                    }
                }
            }
        }

        Ok(self.get_status())
    }

    fn reset(&mut self) {
        self.smoother.reset();
        self.fsm.reset();
        self.counter = 0;
        self.last_count_time = 0.0;
        self.rep_start_time = None;
        self.rep_durations.clear();
        self.rep_form_scores.clear();
        self.current_angles.clear();
        self.current_feedback.clear();
        self.current_score = FormScoreBreakdown {
            score: 100,
            grade: Grade::A,
            angle_penalty: 0,
            tempo_penalty: 0,
            feedback_penalty: 0,
        };
        self.avg_form_score = 100;
        self.rep_completed = false;
    }

    fn get_status(&self) -> ExerciseResult {
        ExerciseResult {
            exercise_name: self.config.name.clone(),
            counter: self.counter,
            counter_left: 0,
            counter_right: 0,
            current_state: self.fsm.current_state.clone(),
            prev_state: self.fsm.prev_state.clone(),
            state_left: None,
            state_right: None,
            current_duration: 0.0,
            target_duration: None,
            is_holding: false,
            angles: self.current_angles.clone(),
            form_score: self.current_score.score,
            avg_form_score: self.avg_form_score,
            form_grade: self.current_score.grade,
            feedback_alerts: self.current_feedback.clone(),
            rep_completed: self.rep_completed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::load_from_str;

    const SQUAT_YAML: &str = include_str!("../../assets/exercises/squat.yaml");

    fn create_landmarks_for_squat_angle(angle_deg: f64) -> Vec<Landmark> {
        let mut landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
        landmarks[11] = Landmark::new(0.5, 0.2, 0.0, 1.0, 1.0); // left shoulder
        landmarks[23] = Landmark::new(0.5, 0.5, 0.0, 1.0, 1.0); // left hip

        let rad = (180.0 - angle_deg).to_radians();
        let knee_x = 0.5 + 0.3 * rad.sin() as f32;
        let knee_y = 0.5 + 0.3 * rad.cos() as f32;
        landmarks[25] = Landmark::new(knee_x, knee_y, 0.0, 1.0, 1.0); // left knee
        landmarks[27] = Landmark::new(knee_x, knee_y + 0.3, 0.0, 1.0, 1.0); // left ankle

        landmarks
    }

    #[test]
    fn test_squat_rep_counting_flow() {
        let config = load_from_str(SQUAT_YAML).unwrap();
        let mut squat = StandardExercise::new(config).unwrap();
        squat.smoother.set_enabled(false);

        // 1. Start position (angle 175 > 165 -> state: start)
        let lm_start = create_landmarks_for_squat_angle(175.0);
        let res1 = squat.process_frame(&lm_start, 1000, 1000, 0.0).unwrap();
        assert_eq!(res1.current_state.as_deref(), Some("start"));
        assert_eq!(res1.counter, 0);

        // 2. Descent position (angle 120 -> state: descent)
        let lm_descent = create_landmarks_for_squat_angle(120.0);
        let res2 = squat.process_frame(&lm_descent, 1000, 1000, 0.5).unwrap();
        assert_eq!(res2.current_state.as_deref(), Some("descent"));
        assert_eq!(res2.counter, 0);

        // 3. Ascent / Bottom position (angle 85 <= 90 -> state: ascent, triggers rep count)
        let lm_bottom = create_landmarks_for_squat_angle(85.0);
        let res3 = squat.process_frame(&lm_bottom, 1000, 1000, 1.5).unwrap();
        assert_eq!(res3.current_state.as_deref(), Some("ascent"));
        assert_eq!(res3.counter, 1);
        assert!(res3.rep_completed);

        // 4. Staying in bottom state should not double-count
        let res4 = squat.process_frame(&lm_bottom, 1000, 1000, 1.6).unwrap();
        assert_eq!(res4.counter, 1);
        assert!(!res4.rep_completed);

        // 5. Too-fast rep should be debounced: min_rep_duration is 0.8s
        squat.process_frame(&lm_descent, 1000, 1000, 1.8).unwrap();
        let res_fast = squat.process_frame(&lm_bottom, 1000, 1000, 1.9).unwrap();
        assert_eq!(res_fast.counter, 1, "Should debounce too-fast rep");
        assert!(!res_fast.rep_completed);
    }
}
