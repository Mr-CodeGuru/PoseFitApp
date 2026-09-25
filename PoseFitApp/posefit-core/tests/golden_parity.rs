use posefit_core::definitions::schema::ExerciseKind;
use posefit_core::definitions::{ExerciseConfig, load_from_str};
use posefit_core::engine::PoseFitEngine;
use posefit_core::exercise::Exercise;
use posefit_core::exercises::{BilateralExercise, DurationExercise, StandardExercise};
use posefit_core::feedback::Severity;
use posefit_core::landmarks::Landmark;

/// Golden Test 1: YAML Loading for all 18 exercises.
/// Parity target: Python `test_yaml_loading`.
#[test]
fn test_golden_yaml_loading_all_18() {
    let mut engine = PoseFitEngine::new();
    engine
        .register_all_bundled_exercises()
        .expect("All 18 bundled exercises must register without error");

    let exercises = engine.available_exercises();
    assert_eq!(exercises.len(), 18, "Expected 18 exercises registered");

    let expected = [
        "bicep_curl",
        "calf_raise",
        "deadlift",
        "glute_bridge",
        "hammer_curl",
        "high_knees",
        "jumping_jack",
        "lateral_raise",
        "leg_raise",
        "lunge",
        "mountain_climber",
        "plank",
        "push_up",
        "shoulder_press",
        "side_lunge",
        "squat",
        "tricep_dip",
        "wall_sit",
    ];

    for name in expected {
        assert!(
            exercises.contains(&name.to_string()),
            "Missing exercise {name}"
        );
    }
}

/// Golden Test 2: Exercise Info & Metadata integrity.
/// Parity target: Python `test_exercise_info`.
#[test]
fn test_golden_exercise_metadata() {
    let squat_yaml = include_str!("../assets/exercises/squat.yaml");
    let squat: ExerciseConfig = load_from_str(squat_yaml).unwrap();
    assert_eq!(squat.display_name.as_deref(), Some("Squat"));
    assert_eq!(squat.exercise_type, ExerciseKind::Repetition);
    assert_eq!(squat.default_reps, Some(10));
    assert_eq!(squat.difficulty.as_deref(), Some("beginner"));
    let muscles = squat.target_muscles.unwrap();
    assert!(muscles.contains(&"Quadriceps".to_string()));
    assert!(muscles.contains(&"Gluteus".to_string()));
    assert!(muscles.contains(&"Hamstrings".to_string()));
}

/// Golden Test 3: State Machine Logic with simulated angle sequence.
/// Parity target: Python `test_state_machine`.
/// Sequence: [175, 160, 130, 100, 85, 95, 140, 170, 175] -> exactly 1 rep.
#[test]
fn test_golden_squat_state_machine_trajectory() {
    let squat_yaml = include_str!("../assets/exercises/squat.yaml");
    let config = load_from_str(squat_yaml).unwrap();
    let mut squat = StandardExercise::new(config).unwrap();
    squat.smoother.set_enabled(false);

    let test_angles = [175.0, 160.0, 130.0, 100.0, 85.0, 95.0, 140.0, 170.0, 175.0];
    let timestamps = [0.0, 0.3, 0.6, 0.9, 1.2, 1.5, 1.8, 2.1, 2.4];

    for (&angle, &ts) in test_angles.iter().zip(timestamps.iter()) {
        // Construct landmarks to yield exactly `angle` for (left_shoulder, left_hip, left_knee)
        let mut landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
        landmarks[11] = Landmark::new(0.5, 0.2, 0.0, 1.0, 1.0); // left shoulder
        landmarks[23] = Landmark::new(0.5, 0.5, 0.0, 1.0, 1.0); // left hip

        let rad = (180.0f64 - angle).to_radians();
        let knee_x = 0.5 + 0.3 * rad.sin() as f32;
        let knee_y = 0.5 + 0.3 * rad.cos() as f32;
        landmarks[25] = Landmark::new(knee_x, knee_y, 0.0, 1.0, 1.0); // left knee

        squat
            .process_frame(&landmarks, 1000, 1000, ts)
            .expect("Frame processing should succeed");
    }

    assert_eq!(
        squat.counter, 1,
        "Trajectory [175 -> 85 -> 175] must complete exactly 1 rep"
    );
}

/// Golden Test 4: Bilateral Exercise (Hammer Curl).
/// Parity target: Python `test_bilateral_exercise`.
#[test]
fn test_golden_bilateral_hammer_curl() {
    let curl_yaml = include_str!("../assets/exercises/hammer_curl.yaml");
    let config = load_from_str(curl_yaml).unwrap();
    assert!(config.bilateral, "hammer_curl must be bilateral");
    assert_eq!(config.sides, vec!["left", "right"]);

    let mut exercise = BilateralExercise::new(config).unwrap();
    exercise.smoother.set_enabled(false);

    // Initial frame
    let landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
    let res = exercise.process_frame(&landmarks, 1000, 1000, 0.0).unwrap();
    assert_eq!(res.counter, 0);
    assert_eq!(res.counter_left, 0);
    assert_eq!(res.counter_right, 0);
}

/// Golden Test 5: Duration Exercise (Plank).
/// Parity target: Python `test_duration_exercise`.
#[test]
fn test_golden_duration_plank() {
    let plank_yaml = include_str!("../assets/exercises/plank.yaml");
    let config = load_from_str(plank_yaml).unwrap();
    assert_eq!(config.exercise_type, ExerciseKind::Duration);
    assert_eq!(config.hold_state.as_deref(), Some("hold"));
    assert_eq!(config.target_duration, Some(30.0));

    let mut plank = DurationExercise::new(config).unwrap();
    plank.smoother.set_enabled(false);

    // Initial frame
    let landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
    let res = plank.process_frame(&landmarks, 1000, 1000, 0.0).unwrap();
    assert_eq!(res.counter, 0);
    assert_eq!(res.target_duration, Some(30.0));
}

/// Golden Test 6: Feedback Rules (Knee caving / knees_caving).
/// Parity target: Python `test_feedback_rules`.
/// Context: left_knee_x < left_ankle_x - 20 (knee is 50px inward).
#[test]
fn test_golden_feedback_knees_caving() {
    let squat_yaml = include_str!("../assets/exercises/squat.yaml");
    let config = load_from_str(squat_yaml).unwrap();
    let mut squat = StandardExercise::new(config).unwrap();
    squat.smoother.set_enabled(false);

    let mut landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
    landmarks[11] = Landmark::new(0.5, 0.2, 0.0, 1.0, 1.0); // left shoulder
    landmarks[23] = Landmark::new(0.5, 0.5, 0.0, 1.0, 1.0); // left hip
    landmarks[25] = Landmark::new(0.20, 0.7, 0.0, 1.0, 1.0); // left knee at x=200px (in 1000px frame)
    landmarks[27] = Landmark::new(0.25, 0.9, 0.0, 1.0, 1.0); // left ankle at x=250px (in 1000px frame)

    let res = squat.process_frame(&landmarks, 1000, 1000, 0.0).unwrap();

    let knees_caving_alert = res
        .feedback_alerts
        .iter()
        .find(|a| a.name == "knees_caving");
    assert!(
        knees_caving_alert.is_some(),
        "Knee caving alert should be triggered"
    );
    let alert = knees_caving_alert.unwrap();
    assert_eq!(alert.severity, Severity::Warning);
}

/// Golden Test 7: Config Validation (Valid vs Invalid configs).
/// Parity target: Python `test_config_validation`.
#[test]
fn test_golden_config_validation() {
    let valid_yaml = r#"
name: test_exercise
type: repetition
angles:
  primary:
    points: [left_shoulder, left_hip, left_knee]
states:
  start:
    condition: "angle > 160"
  down:
    condition: "angle < 100"
counter:
  trigger_state: down
"#;
    let res = load_from_str(valid_yaml);
    assert!(res.is_ok(), "Valid config should validate cleanly");

    // Invalid config referencing unknown landmark
    let invalid_landmark_yaml = r#"
name: bad_landmark
type: repetition
angles:
  primary:
    points: [left_shoulder, invalid_joint_xyz, left_knee]
states:
  start:
    condition: "angle > 160"
"#;
    let res_bad = load_from_str(invalid_landmark_yaml);
    assert!(res_bad.is_err(), "Invalid joint name must fail validation");

    // Invalid state reference in state_order
    let invalid_state_yaml = r#"
name: bad_state
type: repetition
angles:
  primary:
    points: [left_shoulder, left_hip, left_knee]
state_order:
  - nonexistent_state
states:
  start:
    condition: "angle > 160"
"#;
    let res_bad_state = load_from_str(invalid_state_yaml);
    assert!(
        res_bad_state.is_err(),
        "Undefined state in state_order must fail validation"
    );
}
