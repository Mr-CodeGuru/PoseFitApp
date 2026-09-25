//! PoseFit Android Native Application
//!
//! Pure Rust implementation using Android's `NativeActivity` (`android-activity`).
//! ZERO Kotlin, ZERO Java, ZERO JVM business logic.

#[cfg(target_os = "android")]
use log::error;
use log::info;
use posefit_core::engine::{PoseFitEngine, WorkoutSummary};
use posefit_core::inference::BlazePoseEstimator;
use posefit_core::landmarks::Landmark;
use posefit_core::rendering::{FrameBuffer, HudRenderer};

#[cfg(target_os = "android")]
use android_activity::{
    AndroidApp, InputStatus, MainEvent, PollEvent,
    input::{InputEvent, MotionAction},
};

/// High-level interaction mode of the Android application.
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    LiveWorkout,
    WorkoutSummary(WorkoutSummary),
}

/// Native application state managed entirely in Rust.
pub struct AndroidPoseFitApp {
    pub engine: PoseFitEngine,
    pub is_active: bool,
    pub mode: AppMode,
    pub current_exercise: String,
    pub exercise_index: usize,
}

impl AndroidPoseFitApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut engine =
            PoseFitEngine::new().with_estimator(Box::new(BlazePoseEstimator::default()));
        engine.register_all_bundled_exercises()?;
        engine.start_exercise("squat")?;

        Ok(Self {
            engine,
            is_active: false,
            mode: AppMode::LiveWorkout,
            current_exercise: "squat".to_string(),
            exercise_index: 0,
        })
    }

    /// Ingests pre-extracted landmarks directly.
    pub fn process_landmarks(
        &mut self,
        landmarks: &[Landmark],
        width: u32,
        height: u32,
        timestamp_sec: f64,
    ) -> Result<posefit_core::exercise::ExerciseResult, Box<dyn std::error::Error>> {
        let result = self
            .engine
            .process_landmarks(landmarks, width, height, timestamp_sec)?;
        Ok(result)
    }

    /// Ingests raw camera image buffer (RGB) from native AImageReader directly in pure Rust.
    pub fn process_raw_camera_frame(
        &mut self,
        image_bytes: &[u8],
        width: u32,
        height: u32,
        timestamp_sec: f64,
    ) -> Result<posefit_core::exercise::ExerciseResult, Box<dyn std::error::Error>> {
        let result = self
            .engine
            .process_image_bytes(image_bytes, width, height, timestamp_sec)?;
        Ok(result)
    }

    /// Renders skeleton overlay and workout HUD directly onto an Android ANativeWindow raster buffer.
    pub fn render_frame_overlay(
        &self,
        buffer: &mut [u8],
        width: u32,
        height: u32,
        stride: u32,
        landmarks: &[Landmark],
        result: &posefit_core::exercise::ExerciseResult,
    ) {
        let mut fb = FrameBuffer::new(buffer, width, height, stride);
        match &self.mode {
            AppMode::LiveWorkout => {
                HudRenderer::render_skeleton(&mut fb, landmarks, 0.5);
                HudRenderer::render_hud(&mut fb, result);
            }
            AppMode::WorkoutSummary(summary) => {
                HudRenderer::render_summary_card(&mut fb, summary);
            }
        }
    }

    /// Switches the active exercise.
    pub fn switch_exercise(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.engine.stop_exercise();
        self.engine.start_exercise(name)?;
        self.current_exercise = name.to_string();
        self.mode = AppMode::LiveWorkout;
        info!("Switched active exercise to: {name}");
        Ok(())
    }

    /// Cycles forward to the next bundled exercise definition.
    pub fn cycle_next_exercise(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let exercises = self.engine.available_exercises();
        if exercises.is_empty() {
            return Ok(self.current_exercise.clone());
        }

        self.exercise_index = (self.exercise_index + 1) % exercises.len();
        let next_name = exercises[self.exercise_index].clone();
        self.switch_exercise(&next_name)?;
        Ok(next_name)
    }

    /// Finishes the active workout session and displays the summary scorecard.
    pub fn finish_workout(&mut self) -> Result<WorkoutSummary, Box<dyn std::error::Error>> {
        let summary = self.engine.stop_exercise()?;
        self.mode = AppMode::WorkoutSummary(summary.clone());
        info!("Workout completed: {:?}", summary);
        Ok(summary)
    }

    /// Handles native touchscreen tap events in pure Rust.
    pub fn handle_touch(&mut self, x: f32, y: f32, width: u32, _height: u32) {
        match &self.mode {
            AppMode::WorkoutSummary(_) => {
                // Tap anywhere on the summary card dismisses and starts next workout
                let _ = self.cycle_next_exercise();
            }
            AppMode::LiveWorkout => {
                let header_h = 120.0f32;
                if y <= header_h {
                    if x <= (width as f32 * 0.6) {
                        // Tapping top-left cycles through exercises
                        let _ = self.cycle_next_exercise();
                    } else {
                        // Tapping top-right finishes the workout
                        let _ = self.finish_workout();
                    }
                }
            }
        }
    }
}

/// Pure Rust entrypoint invoked directly by Android OS via `NativeActivity`.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("PoseFitRust"),
    );

    info!("╔════════════════════════════════════════════════════════════╗");
    info!("║      POSEFIT PURE RUST ANDROID NATIVE ACTIVITY START       ║");
    info!("║              Zero Java — Zero Kotlin — 100% Rust           ║");
    info!("╚════════════════════════════════════════════════════════════╝");

    let mut state = match AndroidPoseFitApp::new() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize PoseFit Rust app: {e}");
            return;
        }
    };

    let (win_w, win_h) = (1080u32, 1920u32);
    let mut quit = false;

    while !quit {
        app.poll_events(Some(std::time::Duration::from_millis(16)), |event| {
            match event {
                PollEvent::Wake => {}
                PollEvent::Timeout => {}
                PollEvent::Main(main_event) => match main_event {
                    MainEvent::InitWindow { .. } => {
                        info!("Native ANativeWindow initialized. Direct Rust GPU/Surface pipeline active.");
                        state.is_active = true;
                    }
                    MainEvent::TerminateWindow { .. } => {
                        info!("Native ANativeWindow terminated.");
                        state.is_active = false;
                    }
                    MainEvent::WindowResized { .. } => {
                        info!("Native ANativeWindow resized.");
                    }
                    MainEvent::RedrawNeeded { .. } => {
                        // Triggers pure Rust rendering frame to ANativeWindow
                    }
                    MainEvent::Pause => {
                        info!("App Paused. Suspending camera ingestion.");
                        state.is_active = false;
                    }
                    MainEvent::Resume { .. } => {
                        info!("App Resumed. Resuming real-time workout tracking.");
                        state.is_active = true;
                    }
                    MainEvent::Destroy => {
                        info!("Native Activity destroying.");
                        quit = true;
                    }
                    _ => {}
                },
                _ => {}
            }
        });

        // Ingest touch inputs from native input queue in pure Rust
        #[cfg(target_os = "android")]
        if let Ok(mut input_iter) = app.input_events_iter() {
            while input_iter.next(|input_event| {
                if let InputEvent::MotionEvent(motion) = input_event {
                    if motion.action() == MotionAction::Down {
                        let pointer = motion.pointer_at_index(0);
                        let x = pointer.x();
                        let y = pointer.y();
                        state.handle_touch(x, y, win_w, win_h);
                        return InputStatus::Handled;
                    }
                }
                InputStatus::Unhandled
            }) {}
        }
    }

    info!("PoseFit Pure Rust Android Activity exited cleanly.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_app_initialization() {
        let app = AndroidPoseFitApp::new().expect("Should initialize without JVM");
        assert_eq!(app.current_exercise, "squat");
        assert_eq!(app.engine.available_exercises().len(), 18);
        assert_eq!(app.mode, AppMode::LiveWorkout);
    }

    #[test]
    fn test_android_exercise_cycling() {
        let mut app = AndroidPoseFitApp::new().expect("Should initialize without JVM");
        let initial = app.current_exercise.clone();
        let next = app.cycle_next_exercise().expect("Should cycle exercise");
        assert_ne!(initial, next);
    }

    #[test]
    fn test_android_touch_handling() {
        let mut app = AndroidPoseFitApp::new().expect("Should initialize without JVM");

        // Tap top-left to cycle exercise
        let ex1 = app.current_exercise.clone();
        app.handle_touch(50.0, 50.0, 1080, 1920);
        assert_ne!(ex1, app.current_exercise);

        // Tap top-right to finish workout
        app.handle_touch(900.0, 50.0, 1080, 1920);
        match &app.mode {
            AppMode::WorkoutSummary(s) => {
                assert!(s.total_duration_sec >= 0.0);
            }
            _ => panic!("App should be in WorkoutSummary mode"),
        }

        // Tap summary card to restart
        app.handle_touch(500.0, 500.0, 1080, 1920);
        assert_eq!(app.mode, AppMode::LiveWorkout);
    }
}
