use crate::definitions::schema::ExerciseConfig;
use crate::error::PoseFitError;
use crate::fsm::ConditionParser;
use crate::landmarks::landmark_index_by_name;

/// Parses and validates an exercise configuration from YAML string.
pub fn load_from_str(yaml_str: &str) -> Result<ExerciseConfig, PoseFitError> {
    let config: ExerciseConfig =
        serde_yaml::from_str(yaml_str).map_err(|e| PoseFitError::YamlError(e.to_string()))?;
    validate_config(&config)?;
    Ok(config)
}

/// Validates the exercise definition for semantic consistency.
pub fn validate_config(config: &ExerciseConfig) -> Result<(), PoseFitError> {
    // 1. Validate angle points map to known MediaPipe landmarks
    for (angle_name, angle_def) in &config.angles {
        if angle_def.points.len() != 3 {
            return Err(PoseFitError::ConfigError(format!(
                "Angle '{angle_name}' must define exactly 3 points, got {}",
                angle_def.points.len()
            )));
        }
        for point_name in &angle_def.points {
            if landmark_index_by_name(point_name).is_none() {
                return Err(PoseFitError::UnknownLandmarkName(format!(
                    "Angle '{angle_name}' references unknown landmark '{point_name}'"
                )));
            }
        }
    }

    // 2. Validate state_order references existing states
    for state_name in &config.state_order {
        if !config.states.contains_key(state_name) {
            return Err(PoseFitError::ConfigError(format!(
                "state_order references undefined state '{state_name}'"
            )));
        }
    }

    // 3. Validate counter configuration
    if let Some(ref counter) = config.counter {
        if !config.states.contains_key(&counter.trigger_state) {
            return Err(PoseFitError::ConfigError(format!(
                "counter trigger_state '{}' not found in states",
                counter.trigger_state
            )));
        }
        if let Some(ref from_state) = counter.from_state
            && !config.states.contains_key(from_state)
        {
            return Err(PoseFitError::ConfigError(format!(
                "counter from_state '{from_state}' not found in states"
            )));
        }
    }

    // 4. Validate conditions in states parse cleanly
    for (state_name, state_def) in &config.states {
        ConditionParser::parse_str(&state_def.condition).map_err(|e| {
            PoseFitError::ConfigError(format!("State '{state_name}' condition parse error: {e}"))
        })?;
    }

    // 5. Validate feedback conditions parse cleanly
    for (feedback_name, fb_def) in &config.feedback {
        ConditionParser::parse_str(&fb_def.condition).map_err(|e| {
            PoseFitError::ConfigError(format!(
                "Feedback '{feedback_name}' condition parse error: {e}"
            ))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUAT_YAML: &str = include_str!("../../assets/exercises/squat.yaml");
    const HAMMER_CURL_YAML: &str = include_str!("../../assets/exercises/hammer_curl.yaml");
    const PLANK_YAML: &str = include_str!("../../assets/exercises/plank.yaml");

    #[test]
    fn test_load_squat_yaml() {
        let config = load_from_str(SQUAT_YAML).expect("squat.yaml should load and validate");
        assert_eq!(config.name, "squat");
        assert_eq!(config.angles.len(), 2);
        assert_eq!(config.state_order.len(), 3);
        assert_eq!(config.counter.unwrap().trigger_state, "ascent");
    }

    #[test]
    fn test_load_hammer_curl_yaml() {
        let config =
            load_from_str(HAMMER_CURL_YAML).expect("hammer_curl.yaml should load and validate");
        assert_eq!(config.name, "hammer_curl");
        assert!(config.bilateral);
        assert_eq!(config.sides, vec!["left", "right"]);
    }

    #[test]
    fn test_load_plank_yaml() {
        let config = load_from_str(PLANK_YAML).expect("plank.yaml should load and validate");
        assert_eq!(config.name, "plank");
        assert_eq!(config.hold_state.as_deref(), Some("hold"));
        assert_eq!(config.target_duration, Some(30.0));
    }
}
