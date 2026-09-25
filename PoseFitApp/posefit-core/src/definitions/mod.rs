pub mod loader;
pub mod schema;

pub use loader::{load_from_str, validate_config};
pub use schema::ExerciseConfig;

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_yaml {
        ($name:ident, $path:expr) => {
            #[test]
            fn $name() {
                let yaml_str = include_str!($path);
                let cfg = load_from_str(yaml_str)
                    .unwrap_or_else(|e| panic!("Failed to load {}: {:?}", $path, e));
                assert!(!cfg.name.is_empty());
            }
        };
    }

    test_yaml!(test_bicep_curl, "../../assets/exercises/bicep_curl.yaml");
    test_yaml!(test_calf_raise, "../../assets/exercises/calf_raise.yaml");
    test_yaml!(test_deadlift, "../../assets/exercises/deadlift.yaml");
    test_yaml!(
        test_glute_bridge,
        "../../assets/exercises/glute_bridge.yaml"
    );
    test_yaml!(test_hammer_curl, "../../assets/exercises/hammer_curl.yaml");
    test_yaml!(test_high_knees, "../../assets/exercises/high_knees.yaml");
    test_yaml!(
        test_jumping_jack,
        "../../assets/exercises/jumping_jack.yaml"
    );
    test_yaml!(
        test_lateral_raise,
        "../../assets/exercises/lateral_raise.yaml"
    );
    test_yaml!(test_leg_raise, "../../assets/exercises/leg_raise.yaml");
    test_yaml!(test_lunge, "../../assets/exercises/lunge.yaml");
    test_yaml!(
        test_mountain_climber,
        "../../assets/exercises/mountain_climber.yaml"
    );
    test_yaml!(test_plank, "../../assets/exercises/plank.yaml");
    test_yaml!(test_push_up, "../../assets/exercises/push_up.yaml");
    test_yaml!(
        test_shoulder_press,
        "../../assets/exercises/shoulder_press.yaml"
    );
    test_yaml!(test_side_lunge, "../../assets/exercises/side_lunge.yaml");
    test_yaml!(test_squat, "../../assets/exercises/squat.yaml");
    test_yaml!(test_tricep_dip, "../../assets/exercises/tricep_dip.yaml");
    test_yaml!(test_wall_sit, "../../assets/exercises/wall_sit.yaml");

    #[test]
    fn test_all_18_exercises_verified() {
        let all_yaml = [
            include_str!("../../assets/exercises/bicep_curl.yaml"),
            include_str!("../../assets/exercises/calf_raise.yaml"),
            include_str!("../../assets/exercises/deadlift.yaml"),
            include_str!("../../assets/exercises/glute_bridge.yaml"),
            include_str!("../../assets/exercises/hammer_curl.yaml"),
            include_str!("../../assets/exercises/high_knees.yaml"),
            include_str!("../../assets/exercises/jumping_jack.yaml"),
            include_str!("../../assets/exercises/lateral_raise.yaml"),
            include_str!("../../assets/exercises/leg_raise.yaml"),
            include_str!("../../assets/exercises/lunge.yaml"),
            include_str!("../../assets/exercises/mountain_climber.yaml"),
            include_str!("../../assets/exercises/plank.yaml"),
            include_str!("../../assets/exercises/push_up.yaml"),
            include_str!("../../assets/exercises/shoulder_press.yaml"),
            include_str!("../../assets/exercises/side_lunge.yaml"),
            include_str!("../../assets/exercises/squat.yaml"),
            include_str!("../../assets/exercises/tricep_dip.yaml"),
            include_str!("../../assets/exercises/wall_sit.yaml"),
        ];
        assert_eq!(all_yaml.len(), 18);
        for y in all_yaml {
            let res = load_from_str(y);
            assert!(res.is_ok(), "Validation failed: {:?}", res.err());
        }
    }
}
