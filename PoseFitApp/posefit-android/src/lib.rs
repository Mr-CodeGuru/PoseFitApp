//! PoseFit Android Native Application
//!
//! Pure Rust implementation using Android's `NativeActivity` (`android-activity`).
//! ZERO Kotlin, ZERO Java, ZERO JVM business logic.

#[cfg(target_os = "android")]
use log::error;
use log::info;
use posefit_core::engine::PoseFitEngine;
use posefit_core::landmarks::Landmark;

#[cfg(target_os = "android")]
use android_activity::{AndroidApp, MainEvent, PollEvent};

/// Native application state managed entirely in Rust.
pub struct AndroidPoseFitApp {
    pub engine: PoseFitEngine,
    pub is_active: bool,
    pub current_exercise: String,
}

impl AndroidPoseFitApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut engine = PoseFitEngine::new();
        engine.register_all_bundled_exercises()?;
        engine.start_exercise("hammer_curl")?;

        Ok(Self {
            engine,
            is_active: false,
            current_exercise: "hammer_curl".to_string(),
        })
    }

    pub fn process_camera_frame(
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

    pub fn switch_exercise(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.engine.stop_exercise()?;
        self.engine.start_exercise(name)?;
        self.current_exercise = name.to_string();
        info!("Switched active exercise to: {name}");
        Ok(())
    }
}

/// Pure Rust entrypoint invoked directly by Android OS via `NativeActivity`.
#[cfg(target_os = "android")]
#[no_mangle]
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

    info!("PoseFit Core initialized with all 18 bundled exercises.");

    let mut quit = false;
    while !quit {
        app.poll_events(Some(std::time::Duration::from_millis(16)), |event| {
            match event {
                PollEvent::Wake => {}
                PollEvent::Timeout => {}
                PollEvent::Main(main_event) => match main_event {
                    MainEvent::InitWindow { .. } => {
                        info!("Native ANativeWindow initialized. Ready for direct GPU/Surface rendering.");
                        state.is_active = true;
                    }
                    MainEvent::TermWindow { .. } => {
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
                    MainEvent::Resume => {
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
    }

    info!("PoseFit Pure Rust Android Activity exited cleanly.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_app_initialization() {
        let app = AndroidPoseFitApp::new().expect("Should initialize without JVM");
        assert_eq!(app.current_exercise, "hammer_curl");
        assert_eq!(app.engine.available_exercises().len(), 18);
    }
}
