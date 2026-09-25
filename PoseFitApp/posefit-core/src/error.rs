use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum PoseFitError {
    #[error("Missing landmark '{0}'")]
    MissingLandmark(String),

    #[error("Landmark index {0} out of bounds (max {1})")]
    LandmarkIndexOutOfBounds(usize, usize),

    #[error("Unknown landmark name '{0}'")]
    UnknownLandmarkName(String),

    #[error("YAML deserialization error: {0}")]
    YamlError(String),

    #[error("Configuration validation error: {0}")]
    ConfigError(String),

    #[error("Condition evaluation error: {0}")]
    ConditionError(String),

    #[error("Exercise not found: '{0}'")]
    ExerciseNotFound(String),

    #[error("Invalid exercise state: '{0}'")]
    InvalidState(String),

    #[error("Pose estimation error: {0}")]
    EstimationError(String),
}
