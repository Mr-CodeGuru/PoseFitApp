//! 2D Vector trigonometry and angle calculations matching Python PoseFit behavior.

/// Calculates the 2D planar angle in degrees formed by three points (p1, p2, p3),
/// where `p2` is the vertex (joint center).
///
/// Mathematical contract matches `BaseExercise._angle_between` in Python:
/// - Vector BA = p1 - p2
/// - Vector BC = p3 - p2
/// - dot = BA . BC
/// - norm1 = ||BA||, norm2 = ||BC||
/// - if norm1 == 0 or norm2 == 0 -> returns 0.0
/// - cos_angle = dot / (norm1 * norm2) clamped to [-1.0, 1.0]
/// - angle = acos(cos_angle) in degrees
pub fn calculate_angle_2d(p1: (i32, i32), p2: (i32, i32), p3: (i32, i32)) -> f64 {
    calculate_angle_2d_f64(
        (p1.0 as f64, p1.1 as f64),
        (p2.0 as f64, p2.1 as f64),
        (p3.0 as f64, p3.1 as f64),
    )
}

/// Floating-point version of `calculate_angle_2d`.
pub fn calculate_angle_2d_f64(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> f64 {
    let v1_x = p1.0 - p2.0;
    let v1_y = p1.1 - p2.1;
    let v2_x = p3.0 - p2.0;
    let v2_y = p3.1 - p2.1;

    let dot = v1_x * v2_x + v1_y * v2_y;
    let norm1 = (v1_x * v1_x + v1_y * v1_y).sqrt();
    let norm2 = (v2_x * v2_x + v2_y * v2_y).sqrt();

    if norm1 == 0.0 || norm2 == 0.0 {
        return 0.0;
    }

    let cos_angle = (dot / (norm1 * norm2)).clamp(-1.0, 1.0);
    cos_angle.acos().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-4;

    #[test]
    fn test_angle_90_degrees() {
        // Right angle: p1=(0, 10), p2=(0, 0), p3=(10, 0)
        let angle = calculate_angle_2d((0, 10), (0, 0), (10, 0));
        assert!((angle - 90.0).abs() < EPSILON, "Expected 90.0, got {angle}");
    }

    #[test]
    fn test_angle_180_degrees() {
        // Straight line: p1=(-10, 0), p2=(0, 0), p3=(10, 0)
        let angle = calculate_angle_2d((-10, 0), (0, 0), (10, 0));
        assert!(
            (angle - 180.0).abs() < EPSILON,
            "Expected 180.0, got {angle}"
        );
    }

    #[test]
    fn test_angle_0_degrees() {
        // Coincident rays: p1=(10, 0), p2=(0, 0), p3=(10, 0)
        let angle = calculate_angle_2d((10, 0), (0, 0), (10, 0));
        assert!((angle - 0.0).abs() < EPSILON, "Expected 0.0, got {angle}");
    }

    #[test]
    fn test_angle_45_degrees() {
        // 45 degrees: p1=(10, 0), p2=(0, 0), p3=(10, 10)
        let angle = calculate_angle_2d((10, 0), (0, 0), (10, 10));
        assert!((angle - 45.0).abs() < EPSILON, "Expected 45.0, got {angle}");
    }

    #[test]
    fn test_angle_zero_vectors() {
        // Degenerate point: p1 == p2
        let angle1 = calculate_angle_2d((0, 0), (0, 0), (10, 0));
        assert_eq!(angle1, 0.0);

        // Degenerate point: p3 == p2
        let angle2 = calculate_angle_2d((10, 0), (0, 0), (0, 0));
        assert_eq!(angle2, 0.0);

        // All points identical
        let angle3 = calculate_angle_2d((5, 5), (5, 5), (5, 5));
        assert_eq!(angle3, 0.0);
    }

    #[test]
    fn test_angle_nearly_zero_vectors() {
        let angle = calculate_angle_2d_f64((1e-12, 0.0), (0.0, 0.0), (0.0, 1e-12));
        assert!((angle - 90.0).abs() < 1e-2, "Expected ~90.0, got {angle}");
    }

    #[test]
    fn test_floating_point_edge_cases() {
        // Parallel vectors with slight precision noise that could make cos > 1.0 without clamp
        let p1 = (1000.0, 1000.0);
        let p2 = (0.0, 0.0);
        let p3 = (1000.000000001, 1000.000000001);
        let angle = calculate_angle_2d_f64(p1, p2, p3);
        assert!((angle - 0.0).abs() < EPSILON, "Expected ~0.0, got {angle}");
    }
}
