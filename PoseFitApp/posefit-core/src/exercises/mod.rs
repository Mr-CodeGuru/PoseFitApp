pub mod bilateral;
pub mod duration;
pub mod standard;

pub use bilateral::BilateralExercise;
pub use duration::DurationExercise;
pub use standard::StandardExercise;

use crate::definitions::schema::{ExerciseConfig, ExerciseKind};
use crate::error::PoseFitError;
use crate::exercise::Exercise;

/// Factory function to instantiate the concrete Exercise implementation based on configuration.
/// Matches Python `exercises/loader.py:load_exercise_from_file`.
pub fn create_exercise(config: ExerciseConfig) -> Result<Box<dyn Exercise>, PoseFitError> {
    if config.exercise_type == ExerciseKind::Duration {
        Ok(Box::new(DurationExercise::new(config)?))
    } else if config.bilateral {
        Ok(Box::new(BilateralExercise::new(config)?))
    } else {
        Ok(Box::new(StandardExercise::new(config)?))
    }
}
