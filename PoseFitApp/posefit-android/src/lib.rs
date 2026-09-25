//! PoseFit Android Native Application
//!
//! Pure Rust implementation using Android's `NativeActivity` (`android-activity`).
//! ZERO Kotlin, ZERO Java, ZERO JVM business logic.

#[cfg(target_os = "android")]
use log::error;
use log::info;
#[cfg(target_os = "android")]
use ndk::hardware_buffer_format::HardwareBufferFormat;
use posefit_core::engine::{PoseFitEngine, WorkoutSummary};
use posefit_core::exercise::ExerciseResult;
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
    pub latest_result: Option<ExerciseResult>,
    pub window_width: u32,
    pub window_height: u32,
}

impl AndroidPoseFitApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut engine =
            PoseFitEngine::new().with_estimator(Box::new(BlazePoseEstimator::default()));
        engine.register_all_bundled_exercises()?;

        let exercises = engine.available_exercises();
        let exercise_index = exercises.iter().position(|e| e == "squat").unwrap_or(0);
        let current_exercise = exercises
            .get(exercise_index)
            .cloned()
            .unwrap_or_else(|| "squat".to_string());
        engine.start_exercise(&current_exercise)?;

        Ok(Self {
            engine,
            is_active: false,
            mode: AppMode::LiveWorkout,
            current_exercise,
            exercise_index,
            latest_result: None,
            window_width: 1080,
            window_height: 2400,
        })
    }

    /// Ingests pre-extracted landmarks directly.
    pub fn process_landmarks(
        &mut self,
        landmarks: &[Landmark],
        width: u32,
        height: u32,
        timestamp_sec: f64,
    ) -> Result<ExerciseResult, Box<dyn std::error::Error>> {
        let result = self
            .engine
            .process_landmarks(landmarks, width, height, timestamp_sec)?;
        self.latest_result = Some(result.clone());
        Ok(result)
    }

    /// Ingests raw camera image buffer (RGB) from native AImageReader directly in pure Rust.
    pub fn process_raw_camera_frame(
        &mut self,
        image_bytes: &[u8],
        width: u32,
        height: u32,
        timestamp_sec: f64,
    ) -> Result<ExerciseResult, Box<dyn std::error::Error>> {
        let result = self
            .engine
            .process_image_bytes(image_bytes, width, height, timestamp_sec)?;
        self.latest_result = Some(result.clone());
        Ok(result)
    }

    /// Renders the complete application UI directly onto an arbitrary raster buffer.
    pub fn draw_screen_to_buffer(
        &self,
        buffer: &mut [u8],
        width: u32,
        height: u32,
        stride_pixels: u32,
    ) {
        let mut fb = FrameBuffer::new(buffer, width, height, stride_pixels);
        match &self.mode {
            AppMode::WorkoutSummary(summary) => {
                HudRenderer::render_summary_card(&mut fb, summary);
            }
            AppMode::LiveWorkout => {
                let total = self.engine.available_exercises().len();
                HudRenderer::render_workout_screen(
                    &mut fb,
                    &self.current_exercise,
                    self.exercise_index,
                    total,
                    self.latest_result.as_ref(),
                );
            }
        }
    }

    /// Renders the complete application UI directly onto an Android ANativeWindow buffer.
    #[cfg(target_os = "android")]
    pub fn draw_current_screen(&mut self, app: &AndroidApp) {
        let window = match app.native_window() {
            Some(w) => w,
            None => return,
        };

        let mut guard = match window.lock(None) {
            Ok(g) => g,
            Err(_) => return,
        };

        let w = guard.width() as u32;
        let h = guard.height() as u32;
        let stride_pixels = guard.stride() as u32;

        self.window_width = w;
        self.window_height = h;

        let num_bytes = (stride_pixels * h * 4) as usize;
        let bits_ptr = guard.bits() as *mut u8;
        if bits_ptr.is_null() {
            return;
        }

        let buffer = unsafe { std::slice::from_raw_parts_mut(bits_ptr, num_bytes) };
        self.draw_screen_to_buffer(buffer, w, h, stride_pixels);
    }

    /// Switches the active exercise.
    pub fn switch_exercise(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.engine.stop_exercise();
        self.engine.start_exercise(name)?;
        self.current_exercise = name.to_string();
        self.latest_result = None;
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

    /// Cycles backward to the previous bundled exercise definition.
    pub fn cycle_prev_exercise(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let exercises = self.engine.available_exercises();
        if exercises.is_empty() {
            return Ok(self.current_exercise.clone());
        }

        if self.exercise_index == 0 {
            self.exercise_index = exercises.len() - 1;
        } else {
            self.exercise_index -= 1;
        }
        let prev_name = exercises[self.exercise_index].clone();
        self.switch_exercise(&prev_name)?;
        Ok(prev_name)
    }

    /// Finishes the active workout session and displays the summary scorecard.
    pub fn finish_workout(&mut self) -> Result<WorkoutSummary, Box<dyn std::error::Error>> {
        let summary = self.engine.stop_exercise()?;
        self.mode = AppMode::WorkoutSummary(summary.clone());
        info!("Workout completed: {:?}", summary);
        Ok(summary)
    }

    /// Handles native touchscreen tap events in pure Rust using actual runtime screen bounds.
    pub fn handle_touch(&mut self, x: f32, y: f32) {
        let width = self.window_width as f32;
        let height = self.window_height as f32;

        match &self.mode {
            AppMode::WorkoutSummary(_) => {
                // Tap anywhere on the summary card dismisses and starts next workout
                let _ = self.cycle_next_exercise();
            }
            AppMode::LiveWorkout => {
                let header_h = 160.0f32;
                let bottom_y = height - 140.0f32;

                if y <= header_h {
                    // Tap top bar -> cycle next exercise
                    let _ = self.cycle_next_exercise();
                } else if y >= bottom_y {
                    let col_w = width / 3.0;
                    if x < col_w {
                        // Left button: Prev
                        let _ = self.cycle_prev_exercise();
                    } else if x < col_w * 2.0 {
                        // Middle button: Finish
                        let _ = self.finish_workout();
                    } else {
                        // Right button: Next
                        let _ = self.cycle_next_exercise();
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

    let mut quit = false;

    while !quit {
        app.poll_events(
            Some(std::time::Duration::from_millis(30)),
            |event| match event {
                PollEvent::Wake => {}
                PollEvent::Timeout => {}
                PollEvent::Main(main_event) => match main_event {
                    MainEvent::InitWindow { .. } => {
                        info!("Native ANativeWindow initialized. Setting 32-bit RGBA geometry.");
                        if let Some(window) = app.native_window() {
                            let _ = window.set_buffers_geometry(
                                0,
                                0,
                                Some(HardwareBufferFormat::R8G8B8A8_UNORM),
                            );
                        }
                        state.is_active = true;
                        state.draw_current_screen(&app);
                    }
                    MainEvent::TerminateWindow { .. } => {
                        info!("Native ANativeWindow terminated.");
                        state.is_active = false;
                    }
                    MainEvent::WindowResized { .. } => {
                        if let Some(window) = app.native_window() {
                            let _ = window.set_buffers_geometry(
                                0,
                                0,
                                Some(HardwareBufferFormat::R8G8B8A8_UNORM),
                            );
                        }
                        state.draw_current_screen(&app);
                    }
                    MainEvent::RedrawNeeded { .. } => {
                        state.draw_current_screen(&app);
                    }
                    MainEvent::Pause => {
                        info!("App Paused. Suspending workout tracking.");
                        state.is_active = false;
                    }
                    MainEvent::Resume { .. } => {
                        info!("App Resumed. Resuming workout UI.");
                        state.is_active = true;
                        state.draw_current_screen(&app);
                    }
                    MainEvent::Destroy => {
                        info!("Native Activity destroying.");
                        quit = true;
                    }
                    _ => {}
                },
                _ => {}
            },
        );

        // Ingest touch inputs from native input queue in pure Rust
        #[cfg(target_os = "android")]
        if let Ok(mut input_iter) = app.input_events_iter() {
            let mut touched = false;
            while input_iter.next(|input_event| {
                if let InputEvent::MotionEvent(motion) = input_event {
                    if motion.action() == MotionAction::Down {
                        let pointer = motion.pointer_at_index(0);
                        state.handle_touch(pointer.x(), pointer.y());
                        touched = true;
                        return InputStatus::Handled;
                    }
                }
                InputStatus::Unhandled
            }) {}

            // Redraw immediately on user interaction
            if touched {
                state.draw_current_screen(&app);
            }
        }

        // Keep screen refreshed if active
        #[cfg(target_os = "android")]
        if state.is_active {
            state.draw_current_screen(&app);
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
    fn test_android_screen_buffer_rendering() {
        let app = AndroidPoseFitApp::new().expect("Should initialize without JVM");
        let (w, h) = (720, 1280);
        let stride_pixels = w;
        let mut buffer = vec![0u8; (w * h * 4) as usize];
        app.draw_screen_to_buffer(&mut buffer, w, h, stride_pixels);

        let painted_pixels = buffer.iter().filter(|&&b| b > 0).count();
        assert!(
            painted_pixels > 10000,
            "Screen buffer should be filled with UI elements"
        );
    }

    #[test]
    fn test_android_exercise_cycling() {
        let mut app = AndroidPoseFitApp::new().expect("Should initialize without JVM");
        let initial = app.current_exercise.clone();
        let next = app.cycle_next_exercise().expect("Should cycle next");
        assert_ne!(initial, next);

        let prev = app.cycle_prev_exercise().expect("Should cycle prev");
        assert_eq!(initial, prev);
    }

    #[test]
    fn test_android_touch_handling() {
        let mut app = AndroidPoseFitApp::new().expect("Should initialize without JVM");
        app.window_width = 1080;
        app.window_height = 2400;

        // Tap top header -> cycle exercise
        let ex1 = app.current_exercise.clone();
        app.handle_touch(100.0, 50.0);
        assert_ne!(ex1, app.current_exercise);

        // Tap bottom-middle (Finish) -> WorkoutSummary
        app.handle_touch(540.0, 2350.0);
        match &app.mode {
            AppMode::WorkoutSummary(s) => {
                assert!(s.total_duration_sec >= 0.0);
            }
            _ => panic!("App should be in WorkoutSummary mode"),
        }

        // Tap summary card to restart
        app.handle_touch(500.0, 500.0);
        assert_eq!(app.mode, AppMode::LiveWorkout);
    }
}
