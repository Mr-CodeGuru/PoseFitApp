pub mod definitions;
pub mod engine;
pub mod error;
pub mod exercise;
pub mod exercises;
pub mod feedback;
pub mod fsm;
pub mod inference;
pub mod landmarks;
pub mod math;
pub mod rendering;
pub mod scoring;
pub mod smoothing;

pub use definitions::{ExerciseConfig, load_from_str};
pub use engine::{MockPoseEstimator, PoseEstimator, PoseFitEngine, WorkoutSummary};
pub use error::PoseFitError;
pub use exercise::{Exercise, ExerciseResult};
pub use feedback::{FeedbackAlert, FeedbackRule, Severity};
pub use fsm::{
    CompOp, Condition, ConditionParser, EvaluationContext, Expr, StateDefinition, StateMachine,
};
pub use inference::{BlazePoseConfig, BlazePoseEstimator};
pub use landmarks::{LANDMARK_NAMES, Landmark};
pub use math::calculate_angle_2d;
pub use rendering::{EmbeddedFont, FrameBuffer, HudRenderer};
pub use scoring::{FormScoreBreakdown, FormScoreCalculator, Grade};
pub use smoothing::TemporalSmoother;
