use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::fsm::EvaluationContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grade {
    A,
    B,
    C,
    D,
    F,
}

impl Grade {
    pub fn from_score(score: u32) -> Self {
        match score {
            90..=100 => Grade::A,
            80..=89 => Grade::B,
            70..=79 => Grade::C,
            60..=69 => Grade::D,
            _ => Grade::F,
        }
    }

    pub fn as_char(&self) -> char {
        match self {
            Grade::A => 'A',
            Grade::B => 'B',
            Grade::C => 'C',
            Grade::D => 'D',
            Grade::F => 'F',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormScoreBreakdown {
    pub score: u32,
    pub grade: Grade,
    pub angle_penalty: u32,
    pub tempo_penalty: u32,
    pub feedback_penalty: u32,
}

#[derive(Debug, Clone)]
pub struct FormScoreCalculator {
    pub ideal_angles: HashMap<String, f64>,
    pub tempo_min: f64,
    pub tempo_max: f64,
}

impl Default for FormScoreCalculator {
    fn default() -> Self {
        Self {
            ideal_angles: HashMap::new(),
            tempo_min: 1.0,
            tempo_max: 3.0,
        }
    }
}

impl FormScoreCalculator {
    pub fn new(ideal_angles: HashMap<String, f64>, tempo_min: f64, tempo_max: f64) -> Self {
        Self {
            ideal_angles,
            tempo_min,
            tempo_max,
        }
    }

    /// Calculates form score matching Python `BaseExercise.calculate_form_score`:
    /// - Max score = 100
    /// - Angle penalty: max 40 points
    /// - Tempo penalty: max 30 points
    /// - Feedback penalty: 10 points per active feedback, max 30 points
    /// - Clamped to 0..=100
    pub fn calculate(
        &self,
        ctx: &EvaluationContext,
        last_rep_duration: Option<f64>,
        active_feedback_count: usize,
    ) -> FormScoreBreakdown {
        let mut score = 100i32;

        // 1. Angle Penalty (max 40)
        let angle_penalty = self.calculate_angle_penalty(ctx);
        let capped_angle_penalty = angle_penalty.min(40);
        score -= capped_angle_penalty as i32;

        // 2. Tempo Penalty (max 30)
        let tempo_penalty = self.calculate_tempo_penalty(last_rep_duration);
        let capped_tempo_penalty = tempo_penalty.min(30);
        score -= capped_tempo_penalty as i32;

        // 3. Feedback Penalty (max 30)
        let feedback_penalty = (active_feedback_count as u32) * 10;
        let capped_feedback_penalty = feedback_penalty.min(30);
        score -= capped_feedback_penalty as i32;

        let final_score = score.clamp(0, 100) as u32;
        let grade = Grade::from_score(final_score);

        FormScoreBreakdown {
            score: final_score,
            grade,
            angle_penalty: capped_angle_penalty,
            tempo_penalty: capped_tempo_penalty,
            feedback_penalty: capped_feedback_penalty,
        }
    }

    fn calculate_angle_penalty(&self, ctx: &EvaluationContext) -> u32 {
        if self.ideal_angles.is_empty() {
            return 0;
        }

        let mut total_deviation = 0.0;
        let mut count = 0;

        for (angle_name, &ideal_value) in &self.ideal_angles {
            let current_value = ctx.get_var(angle_name).or_else(|| ctx.get_var("angle"));
            if let Some(val) = current_value {
                let deviation = (val - ideal_value).abs();
                total_deviation += (deviation / 10.0) * 5.0;
                count += 1;
            }
        }

        if count > 0 {
            (total_deviation / count as f64) as u32
        } else {
            0
        }
    }

    fn calculate_tempo_penalty(&self, last_duration: Option<f64>) -> u32 {
        let Some(duration) = last_duration else {
            return 0;
        };

        if duration < self.tempo_min {
            // Too fast: each 0.5s under min = 15 points
            (((self.tempo_min - duration) / 0.5) * 15.0) as u32
        } else if duration > self.tempo_max {
            // Too slow: each 1s over max = 10 points
            ((duration - self.tempo_max) * 10.0) as u32
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_score() {
        let calc = FormScoreCalculator::default();
        let ctx = EvaluationContext::new();
        let res = calc.calculate(&ctx, Some(2.0), 0);
        assert_eq!(res.score, 100);
        assert_eq!(res.grade, Grade::A);
        assert_eq!(res.angle_penalty, 0);
        assert_eq!(res.tempo_penalty, 0);
        assert_eq!(res.feedback_penalty, 0);
    }

    #[test]
    fn test_angle_penalty_and_cap() {
        let mut ideal = HashMap::new();
        ideal.insert("knee".to_string(), 90.0);
        let calc = FormScoreCalculator::new(ideal, 1.0, 3.0);

        // Deviation of 20 degrees: (20 / 10) * 5 = 10 penalty
        let ctx = EvaluationContext::new().with_angle("knee", 70.0);
        let res = calc.calculate(&ctx, Some(2.0), 0);
        assert_eq!(res.angle_penalty, 10);
        assert_eq!(res.score, 90);
        assert_eq!(res.grade, Grade::A);

        // Huge deviation of 120 degrees: (120 / 10) * 5 = 60 penalty -> capped at 40
        let ctx_bad = EvaluationContext::new().with_angle("knee", 210.0);
        let res_bad = calc.calculate(&ctx_bad, Some(2.0), 0);
        assert_eq!(res_bad.angle_penalty, 40);
        assert_eq!(res_bad.score, 60);
        assert_eq!(res_bad.grade, Grade::D);
    }

    #[test]
    fn test_tempo_penalty_fast_and_slow() {
        let calc = FormScoreCalculator::new(HashMap::new(), 1.0, 3.0);
        let ctx = EvaluationContext::new();

        // 0.5s under 1.0s: (1.0 - 0.5) / 0.5 * 15 = 15 points
        let res_fast = calc.calculate(&ctx, Some(0.5), 0);
        assert_eq!(res_fast.tempo_penalty, 15);
        assert_eq!(res_fast.score, 85);
        assert_eq!(res_fast.grade, Grade::B);

        // 2s over 3.0s: (5.0 - 3.0) * 10 = 20 points
        let res_slow = calc.calculate(&ctx, Some(5.0), 0);
        assert_eq!(res_slow.tempo_penalty, 20);
        assert_eq!(res_slow.score, 80);
        assert_eq!(res_slow.grade, Grade::B);

        // Extremely slow: 10.0s -> (10.0 - 3.0) * 10 = 70 points -> capped at 30
        let res_too_slow = calc.calculate(&ctx, Some(10.0), 0);
        assert_eq!(res_too_slow.tempo_penalty, 30);
        assert_eq!(res_too_slow.score, 70);
        assert_eq!(res_too_slow.grade, Grade::C);
    }

    #[test]
    fn test_feedback_penalty_and_cap() {
        let calc = FormScoreCalculator::default();
        let ctx = EvaluationContext::new();

        // 2 feedbacks: 20 points
        let res2 = calc.calculate(&ctx, Some(2.0), 2);
        assert_eq!(res2.feedback_penalty, 20);
        assert_eq!(res2.score, 80);

        // 5 feedbacks: 50 points -> capped at 30
        let res5 = calc.calculate(&ctx, Some(2.0), 5);
        assert_eq!(res5.feedback_penalty, 30);
        assert_eq!(res5.score, 70);
    }

    #[test]
    fn test_score_clamped_to_zero() {
        let mut ideal = HashMap::new();
        ideal.insert("knee".to_string(), 90.0);
        let calc = FormScoreCalculator::new(ideal, 1.0, 3.0);

        // Max penalties across all 3: 40 + 30 + 30 = 100 penalty -> score = 0
        let ctx = EvaluationContext::new().with_angle("knee", 200.0);
        let res = calc.calculate(&ctx, Some(0.0), 5);
        assert_eq!(res.score, 0);
        assert_eq!(res.grade, Grade::F);
    }
}
