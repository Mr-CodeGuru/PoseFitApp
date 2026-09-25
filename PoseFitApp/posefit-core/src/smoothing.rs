use std::collections::{HashMap, VecDeque};

/// Moving average temporal smoothing filter for joint angles.
/// Matches Python `BaseExercise._apply_smoothing` exactly:
/// - Window size $N$ (default 3)
/// - Insufficient history ($< N$) averages all currently available values
/// - Exceeding window drops the oldest sample
/// - History is maintained per angle name
#[derive(Debug, Clone)]
pub struct TemporalSmoother {
    window_size: usize,
    enabled: bool,
    history: HashMap<String, VecDeque<f64>>,
}

impl TemporalSmoother {
    pub fn new(window_size: usize, enabled: bool) -> Self {
        Self {
            window_size: window_size.max(1),
            enabled,
            history: HashMap::new(),
        }
    }

    /// Default configuration with window size 3 and enabled = true.
    pub fn default_config() -> Self {
        Self::new(3, true)
    }

    /// Applies temporal moving average smoothing to `angle` for `angle_name`.
    /// If smoothing is disabled, returns `raw_angle` directly without updating history.
    pub fn smooth(&mut self, angle_name: &str, raw_angle: f64) -> f64 {
        if !self.enabled {
            return raw_angle;
        }

        let queue = self.history.entry(angle_name.to_string()).or_default();
        queue.push_back(raw_angle);

        if queue.len() > self.window_size {
            queue.pop_front();
        }

        let sum: f64 = queue.iter().sum();
        sum / queue.len() as f64
    }

    /// Resets history for all angles. Matches Python `self.angle_history.clear()`.
    pub fn reset(&mut self) {
        self.history.clear();
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn window_size(&self) -> usize {
        self.window_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_smoothing_single_value() {
        let mut smoother = TemporalSmoother::new(3, true);
        let val = smoother.smooth("primary", 100.0);
        assert!((val - 100.0).abs() < EPSILON);
    }

    #[test]
    fn test_smoothing_insufficient_history() {
        let mut smoother = TemporalSmoother::new(3, true);
        smoother.smooth("primary", 100.0);
        let val2 = smoother.smooth("primary", 110.0);
        // Average of 100 and 110 = 105.0
        assert!((val2 - 105.0).abs() < EPSILON);
    }

    #[test]
    fn test_smoothing_window_sliding() {
        let mut smoother = TemporalSmoother::new(3, true);
        smoother.smooth("primary", 90.0);
        smoother.smooth("primary", 100.0);
        smoother.smooth("primary", 110.0); // window: [90, 100, 110], mean = 100

        let val4 = smoother.smooth("primary", 120.0); // window: [100, 110, 120], mean = 110
        assert!((val4 - 110.0).abs() < EPSILON);
    }

    #[test]
    fn test_smoothing_per_angle_independence() {
        let mut smoother = TemporalSmoother::new(3, true);
        smoother.smooth("left", 50.0);
        smoother.smooth("right", 150.0);

        let left = smoother.smooth("left", 70.0); // mean(50, 70) = 60
        let right = smoother.smooth("right", 170.0); // mean(150, 170) = 160

        assert!((left - 60.0).abs() < EPSILON);
        assert!((right - 160.0).abs() < EPSILON);
    }

    #[test]
    fn test_smoothing_reset() {
        let mut smoother = TemporalSmoother::new(3, true);
        smoother.smooth("primary", 100.0);
        smoother.smooth("primary", 120.0);
        smoother.reset();

        let val = smoother.smooth("primary", 60.0);
        assert!((val - 60.0).abs() < EPSILON);
    }

    #[test]
    fn test_smoothing_disabled() {
        let mut smoother = TemporalSmoother::new(3, false);
        smoother.smooth("primary", 100.0);
        let val = smoother.smooth("primary", 120.0);
        assert!((val - 120.0).abs() < EPSILON);
    }
}
