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
pub struct BilateralExercise {
    pub config: ExerciseConfig,
    angles_def: HashMap<String, AngleDef>,
    pub smoother: TemporalSmoother,
    fsm_left: StateMachine,
    fsm_right: StateMachine,
    trigger_state: String,
    from_state: Option<String>,
    min_rep_duration: f64,
    feedback_rules: Vec<FeedbackRule>,
    score_calc: FormScoreCalculator,

    // Runtime state
    pub counter_left: u32,
    pub counter_right: u32,
    pub last_count_time_left: f64,
    pub last_count_time_right: f64,
    pub current_angles: HashMap<String, f64>,
    pub current_feedback: Vec<FeedbackAlert>,
    pub current_score: FormScoreBreakdown,
    pub rep_completed: bool,
}

impl BilateralExercise {
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

        let mut states_left = HashMap::new();
        let mut states_right = HashMap::new();
        for (state_name, state_cfg) in &config.states {
            let cond_left = ConditionParser::parse_str(&state_cfg.condition)?;
            let cond_right = ConditionParser::parse_str(&state_cfg.condition)?;
            states_left.insert(
                state_name.clone(),
                StateDefinition {
                    name: state_name.clone(),
                    condition_str: state_cfg.condition.clone(),
                    condition: cond_left,
                },
            );
            states_right.insert(
                state_name.clone(),
                StateDefinition {
                    name: state_name.clone(),
                    condition_str: state_cfg.condition.clone(),
                    condition: cond_right,
                },
            );
        }

        let fsm_left = StateMachine::new(config.state_order.clone(), states_left);
        let fsm_right = StateMachine::new(config.state_order.clone(), states_right);

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

        let trigger_state = config
            .counter
            .as_ref()
            .map(|c| c.trigger_state.clone())
            .unwrap_or_else(|| "down".to_string());
        let from_state = config.counter.as_ref().and_then(|c| c.from_state.clone());

        let min_rep_duration = config.min_rep_duration.unwrap_or(0.8);
        let tempo_min = config.tempo.as_ref().map(|t| t.min).unwrap_or(1.0);
        let tempo_max = config.tempo.as_ref().map(|t| t.max).unwrap_or(3.0);
        let ideal_angles = config.ideal_angles.clone().unwrap_or_default();
        let score_calc = FormScoreCalculator::new(ideal_angles, tempo_min, tempo_max);

        Ok(Self {
            config,
            angles_def,
            smoother,
            fsm_left,
            fsm_right,
            trigger_state,
            from_state,
            min_rep_duration,
            feedback_rules,
            score_calc,
            counter_left: 0,
            counter_right: 0,
            last_count_time_left: 0.0,
            last_count_time_right: 0.0,
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

impl Exercise for BilateralExercise {
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

        // 1. Build Left Context
        let mut left_ctx = EvaluationContext::new();
        for (k, &v) in &angles {
            left_ctx.angles.insert(k.clone(), v);
        }
        for (k, &v) in &coords {
            left_ctx.coords.insert(k.clone(), v);
        }
        let left_angle = angles
            .get("left")
            .or_else(|| angles.get("left_angle"))
            .or_else(|| angles.get("primary"))
            .copied()
            .unwrap_or(0.0);
        left_ctx.angles.insert("angle".to_string(), left_angle);
        left_ctx.angles.insert("left_angle".to_string(), left_angle);

        // 2. Build Right Context
        let mut right_ctx = EvaluationContext::new();
        for (k, &v) in &angles {
            right_ctx.angles.insert(k.clone(), v);
        }
        for (k, &v) in &coords {
            right_ctx.coords.insert(k.clone(), v);
        }
        let right_angle = angles
            .get("right")
            .or_else(|| angles.get("right_angle"))
            .or_else(|| angles.get("right_arm"))
            .copied()
            .unwrap_or(0.0);
        right_ctx.angles.insert("angle".to_string(), right_angle);
        right_ctx
            .angles
            .insert("right_angle".to_string(), right_angle);

        // 3. Update State Machines
        self.fsm_left.update(&left_ctx)?;
        self.fsm_right.update(&right_ctx)?;

        // 4. Update Bilateral Counters
        let mut left_counted = false;
        let mut right_counted = false;

        let left_from_valid = match self.from_state {
            Some(ref fs) => self.fsm_left.prev_state.as_deref() == Some(fs.as_str()),
            None => true,
        };

        if self.fsm_left.prev_state != self.fsm_left.current_state
            && self.fsm_left.current_state.as_deref() == Some(&self.trigger_state)
            && left_from_valid
            && (timestamp_sec - self.last_count_time_left) >= self.min_rep_duration
        {
            self.counter_left += 1;
            self.last_count_time_left = timestamp_sec;
            left_counted = true;
        }

        let right_from_valid = match self.from_state {
            Some(ref fs) => self.fsm_right.prev_state.as_deref() == Some(fs.as_str()),
            None => true,
        };

        if self.fsm_right.prev_state != self.fsm_right.current_state
            && self.fsm_right.current_state.as_deref() == Some(&self.trigger_state)
            && right_from_valid
            && (timestamp_sec - self.last_count_time_right) >= self.min_rep_duration
        {
            self.counter_right += 1;
            self.last_count_time_right = timestamp_sec;
            right_counted = true;
        }

        self.rep_completed = left_counted || right_counted;

        // 5. Feedback
        let mut active_feedback = Vec::new();
        for rule in &self.feedback_rules {
            if let Some(alert) = rule.evaluate(&left_ctx)? {
                active_feedback.push(alert);
            }
        }
        self.current_feedback = active_feedback.clone();

        // 6. Form Scoring
        self.current_score =
            self.score_calc
                .calculate(&left_ctx, None, self.current_feedback.len());

        Ok(self.get_status())
    }

    fn reset(&mut self) {
        self.smoother.reset();
        self.fsm_left.reset();
        self.fsm_right.reset();
        self.counter_left = 0;
        self.counter_right = 0;
        self.last_count_time_left = 0.0;
        self.last_count_time_right = 0.0;
        self.current_angles.clear();
        self.current_feedback.clear();
        self.rep_completed = false;
    }

    fn get_status(&self) -> ExerciseResult {
        ExerciseResult {
            exercise_name: self.config.name.clone(),
            counter: self.counter_left + self.counter_right,
            counter_left: self.counter_left,
            counter_right: self.counter_right,
            current_state: self.fsm_left.current_state.clone(),
            prev_state: self.fsm_left.prev_state.clone(),
            state_left: self.fsm_left.current_state.clone(),
            state_right: self.fsm_right.current_state.clone(),
            current_duration: 0.0,
            target_duration: None,
            is_holding: false,
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

    const HAMMER_CURL_YAML: &str = include_str!("../../assets/exercises/hammer_curl.yaml");

    #[test]
    fn test_bilateral_hammer_curl_counting() {
        let config = load_from_str(HAMMER_CURL_YAML).unwrap();
        let mut curl = BilateralExercise::new(config).unwrap();
        curl.smoother.set_enabled(false);

        let mut landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
        landmarks[11] = Landmark::new(0.4, 0.2, 0.0, 1.0, 1.0); // left shoulder
        landmarks[13] = Landmark::new(0.4, 0.5, 0.0, 1.0, 1.0); // left elbow
        landmarks[15] = Landmark::new(0.4, 0.8, 0.0, 1.0, 1.0); // left wrist (straight down)

        landmarks[12] = Landmark::new(0.6, 0.2, 0.0, 1.0, 1.0); // right shoulder
        landmarks[14] = Landmark::new(0.6, 0.5, 0.0, 1.0, 1.0); // right elbow
        landmarks[16] = Landmark::new(0.6, 0.8, 0.0, 1.0, 1.0); // right wrist (straight down)

        let res = curl.process_frame(&landmarks, 1000, 1000, 0.0).unwrap();
        assert_eq!(res.counter_left, 0);
        assert_eq!(res.counter, 0);

        // Move left arm to 'up' state (angle ~90 deg)
        landmarks[15] = Landmark::new(0.7, 0.5, 0.0, 1.0, 1.0);
        let res2 = curl.process_frame(&landmarks, 1000, 1000, 0.5).unwrap();
        assert_eq!(res2.counter_left, 0);

        // Move left arm to 'down' state (angle <= 47 deg)
        landmarks[15] = Landmark::new(0.45, 0.25, 0.0, 1.0, 1.0);
        let res3 = curl.process_frame(&landmarks, 1000, 1000, 1.5).unwrap();
        assert_eq!(res3.counter_left, 1);
        assert_eq!(res3.counter, 1);
    }
}
