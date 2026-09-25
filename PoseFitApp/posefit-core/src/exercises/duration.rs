use std::collections::HashMap;

use crate::definitions::schema::ExerciseConfig;
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
pub struct DurationExercise {
    pub config: ExerciseConfig,
    angles_def: HashMap<String, AngleDef>,
    pub smoother: TemporalSmoother,
    fsm: StateMachine,
    hold_state: String,
    target_duration: f64,
    feedback_rules: Vec<FeedbackRule>,
    score_calc: FormScoreCalculator,

    // Runtime state
    pub counter: u32,
    pub current_duration: f64,
    pub hold_start_time: Option<f64>,
    pub is_holding: bool,
    pub current_angles: HashMap<String, f64>,
    pub current_feedback: Vec<FeedbackAlert>,
    pub current_score: FormScoreBreakdown,
    pub rep_completed: bool,
}

impl DurationExercise {
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

        let hold_state = config
            .hold_state
            .clone()
            .unwrap_or_else(|| "hold".to_string());
        let target_duration = config.target_duration.unwrap_or(30.0);

        let tempo_min = config.tempo.as_ref().map(|t| t.min).unwrap_or(1.0);
        let tempo_max = config.tempo.as_ref().map(|t| t.max).unwrap_or(3.0);
        let ideal_angles = config.ideal_angles.clone().unwrap_or_default();
        let score_calc = FormScoreCalculator::new(ideal_angles, tempo_min, tempo_max);

        Ok(Self {
            config,
            angles_def,
            smoother,
            fsm,
            hold_state,
            target_duration,
            feedback_rules,
            score_calc,
            counter: 0,
            current_duration: 0.0,
            hold_start_time: None,
            is_holding: false,
            current_angles: HashMap::new(),
            current_feedback: Vec::new(),
            current_score: FormScoreBreakdown {
                score: 100,
                grade: Grade::A,
                angle_penalty: 0,
                tempo_penalty: 0,
                feedback_penalty: 0,
            },
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
}

impl Exercise for DurationExercise {
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

        let mut ctx = EvaluationContext::new();
        for (k, &v) in &angles {
            ctx.angles.insert(k.clone(), v);
        }
        for (k, &v) in &coords {
            ctx.coords.insert(k.clone(), v);
        }

        // 1. Update State Machine
        self.fsm.update(&ctx)?;

        // 2. Duration tracking
        self.rep_completed = false;
        if self.fsm.current_state.as_deref() == Some(&self.hold_state) {
            if !self.is_holding {
                self.hold_start_time = Some(timestamp_sec);
                self.is_holding = true;
                self.current_duration = 0.0;
            } else if let Some(start_t) = self.hold_start_time {
                self.current_duration = timestamp_sec - start_t;
            }
        } else {
            self.is_holding = false;
            if self.current_duration >= self.target_duration {
                self.counter += 1;
                self.rep_completed = true;
            }
            self.current_duration = 0.0;
            self.hold_start_time = None;
        }

        // 3. Feedback
        let mut active_feedback = Vec::new();
        for rule in &self.feedback_rules {
            if let Some(alert) = rule.evaluate(&ctx)? {
                active_feedback.push(alert);
            }
        }
        self.current_feedback = active_feedback.clone();

        // 4. Form Scoring
        self.current_score = self
            .score_calc
            .calculate(&ctx, None, self.current_feedback.len());

        Ok(self.get_status())
    }

    fn reset(&mut self) {
        self.smoother.reset();
        self.fsm.reset();
        self.counter = 0;
        self.current_duration = 0.0;
        self.hold_start_time = None;
        self.is_holding = false;
        self.current_angles.clear();
        self.current_feedback.clear();
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
            current_duration: self.current_duration,
            target_duration: Some(self.target_duration),
            is_holding: self.is_holding,
            angles: self.current_angles.clone(),
            form_score: self.current_score.score,
            avg_form_score: self.current_score.score,
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

    const PLANK_YAML: &str = include_str!("../../assets/exercises/plank.yaml");

    #[test]
    fn test_duration_plank_holding_and_completion() {
        let mut config = load_from_str(PLANK_YAML).unwrap();
        config.target_duration = Some(5.0);
        let mut plank = DurationExercise::new(config).unwrap();
        plank.smoother.set_enabled(false);

        let mut landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
        landmarks[11] = Landmark::new(0.2, 0.5, 0.0, 1.0, 1.0); // shoulder
        landmarks[23] = Landmark::new(0.5, 0.5, 0.0, 1.0, 1.0); // hip
        landmarks[27] = Landmark::new(0.8, 0.5, 0.0, 1.0, 1.0); // ankle

        let res0 = plank.process_frame(&landmarks, 1000, 1000, 0.0).unwrap();
        assert_eq!(res0.current_state.as_deref(), Some("hold"));
        assert!(res0.is_holding);
        assert_eq!(res0.current_duration, 0.0);

        let res1 = plank.process_frame(&landmarks, 1000, 1000, 3.0).unwrap();
        assert!(res1.is_holding);
        assert_eq!(res1.current_duration, 3.0);
        assert_eq!(res1.counter, 0);

        let res2 = plank.process_frame(&landmarks, 1000, 1000, 6.0).unwrap();
        assert!(res2.is_holding);
        assert_eq!(res2.current_duration, 6.0);

        // Break hold
        landmarks[23] = Landmark::new(0.5, 0.2, 0.0, 1.0, 1.0);
        let res3 = plank.process_frame(&landmarks, 1000, 1000, 6.5).unwrap();
        assert!(!res3.is_holding);
        assert_eq!(
            res3.counter, 1,
            "Completed 1 rep after target duration achieved"
        );
        assert!(res3.rep_completed);
    }
}
