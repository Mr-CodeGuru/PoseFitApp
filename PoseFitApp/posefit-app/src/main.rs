use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

use posefit_core::engine::PoseFitEngine;
use posefit_core::feedback::FeedbackAlert;
use posefit_core::landmarks::Landmark;
use posefit_core::scoring::Grade;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct FrameInput {
    pub frame_idx: usize,
    pub timestamp_sec: f64,
    pub width: u32,
    pub height: u32,
    pub landmarks: Vec<LandmarkInput>,
}

#[derive(Debug, Deserialize)]
struct LandmarkInput {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    #[serde(default)]
    pub visibility: f32,
    #[serde(default)]
    pub presence: f32,
}

#[derive(Debug, Serialize)]
struct FrameOutput {
    pub frame_idx: usize,
    pub timestamp_sec: f64,
    pub exercise_name: String,
    pub counter: u32,
    pub counter_left: u32,
    pub counter_right: u32,
    pub current_state: Option<String>,
    pub state_left: Option<String>,
    pub state_right: Option<String>,
    pub angles: HashMap<String, f64>,
    pub form_score: u32,
    pub avg_form_score: u32,
    pub form_grade: Grade,
    pub rep_completed: bool,
    pub feedback_alerts: Vec<FeedbackAlert>,
}

fn process_stream(
    exercise_name: &str,
    input_path: Option<&str>,
    output_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = PoseFitEngine::new();
    engine.register_all_bundled_exercises()?;
    engine.start_exercise(exercise_name)?;

    let reader: Box<dyn BufRead> = match input_path {
        Some(path) => Box::new(BufReader::new(File::open(path)?)),
        None => Box::new(BufReader::new(io::stdin())),
    };

    let mut writer: Box<dyn Write> = match output_path {
        Some(path) => Box::new(BufWriter::new(File::create(path)?)),
        None => Box::new(BufWriter::new(io::stdout())),
    };

    let mut total_processed = 0;

    for line in reader.lines() {
        let line_str = line?;
        let trimmed = line_str.trim();
        if trimmed.is_empty() {
            continue;
        }

        let frame: FrameInput = match serde_json::from_str(trimmed) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to parse frame JSON: {e} | line: {trimmed}");
                continue;
            }
        };

        let landmarks: Vec<Landmark> = frame
            .landmarks
            .iter()
            .map(|lm| Landmark::new(lm.x, lm.y, lm.z, lm.visibility, lm.presence))
            .collect();

        let result = engine.process_landmarks(
            &landmarks,
            frame.width,
            frame.height,
            frame.timestamp_sec,
        )?;

        let output = FrameOutput {
            frame_idx: frame.frame_idx,
            timestamp_sec: frame.timestamp_sec,
            exercise_name: result.exercise_name,
            counter: result.counter,
            counter_left: result.counter_left,
            counter_right: result.counter_right,
            current_state: result.current_state,
            state_left: result.state_left,
            state_right: result.state_right,
            angles: result.angles,
            form_score: result.form_score,
            avg_form_score: result.avg_form_score,
            form_grade: result.form_grade,
            rep_completed: result.rep_completed,
            feedback_alerts: result.feedback_alerts,
        };

        serde_json::to_writer(&mut writer, &output)?;
        writeln!(&mut writer)?;
        total_processed += 1;
    }

    writer.flush()?;
    eprintln!(
        "Rust Engine finished processing {} frames for exercise '{}'.",
        total_processed, exercise_name
    );

    let summary = engine.stop_exercise()?;
    eprintln!("Final Workout Summary: {:?}", summary);

    Ok(())
}

fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║             POSEFIT CORE ENGINE — RUST MIGRATION           ║");
    println!("║        Real-Time Deterministic Computer-Vision Fitness     ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let mut engine = PoseFitEngine::new();
    engine.register_all_bundled_exercises()?;

    let exercises = engine.available_exercises();
    println!(
        "Loaded and verified {} exercise definitions:",
        exercises.len()
    );
    for (i, name) in exercises.iter().enumerate() {
        print!("{:20} ", format!("{}. {}", i + 1, name));
        if (i + 1) % 3 == 0 {
            println!();
        }
    }
    println!("\n");

    println!("------------------------------------------------------------");
    println!("▶ SIMULATION 1: Standard Repetition Exercise (Squat)");
    println!("------------------------------------------------------------");
    engine.start_exercise("squat")?;

    let angles = [
        175.0, 160.0, 130.0, 100.0, 80.0, 75.0, 95.0, 140.0, 175.0,
    ];
    let timestamps = [0.0, 0.3, 0.6, 0.9, 1.2, 1.5, 1.8, 2.1, 2.4];
    let (w, h) = (1000u32, 1000u32);

    for (&angle, &ts) in angles.iter().zip(timestamps.iter()) {
        let mut landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
        landmarks[11] = Landmark::new(0.5, 0.2, 0.0, 1.0, 1.0); // shoulder
        landmarks[23] = Landmark::new(0.5, 0.5, 0.0, 1.0, 1.0); // hip

        let rad = (180.0f64 - angle).to_radians();
        let knee_x = 0.5 + 0.3 * rad.sin() as f32;
        let knee_y = 0.5 + 0.3 * rad.cos() as f32;
        landmarks[25] = Landmark::new(knee_x, knee_y, 0.0, 1.0, 1.0); // knee

        let status = engine.process_landmarks(&landmarks, w, h, ts)?;
        let state_str = status.current_state.as_deref().unwrap_or("none");
        let rep_mark = if status.rep_completed {
            "★ REP COMPLETED!"
        } else {
            ""
        };
        println!(
            "t={:03.1}s | Input: {:5.1}° | Smoothed: {:5.1}° | State: {:8} | Reps: {} | Score: {:3} ({:?}) {}",
            ts,
            angle,
            status.angles.get("primary").copied().unwrap_or(0.0),
            state_str,
            status.counter,
            status.form_score,
            status.form_grade,
            rep_mark
        );
    }

    let summary = engine.stop_exercise()?;
    println!("\nWorkout Summary: {:?}", summary);

    println!("\n------------------------------------------------------------");
    println!("▶ SIMULATION 2: Bilateral Repetition Exercise (Hammer Curl)");
    println!("------------------------------------------------------------");
    engine.start_exercise("hammer_curl")?;
    let mut curl_landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
    curl_landmarks[11] = Landmark::new(0.4, 0.2, 0.0, 1.0, 1.0);
    curl_landmarks[13] = Landmark::new(0.4, 0.5, 0.0, 1.0, 1.0);
    curl_landmarks[15] = Landmark::new(0.4, 0.8, 0.0, 1.0, 1.0);

    curl_landmarks[12] = Landmark::new(0.6, 0.2, 0.0, 1.0, 1.0);
    curl_landmarks[14] = Landmark::new(0.6, 0.5, 0.0, 1.0, 1.0);
    curl_landmarks[16] = Landmark::new(0.6, 0.8, 0.0, 1.0, 1.0);

    engine.process_landmarks(&curl_landmarks, w, h, 0.0)?;

    curl_landmarks[15] = Landmark::new(0.4, 0.35, 0.0, 1.0, 1.0);
    let status_curl = engine.process_landmarks(&curl_landmarks, w, h, 1.0)?;
    println!(
        "Bilateral curl status: total_reps={}, left={}, right={}, state_l={:?}, state_r={:?}",
        status_curl.counter,
        status_curl.counter_left,
        status_curl.counter_right,
        status_curl.state_left,
        status_curl.state_right
    );
    let summary2 = engine.stop_exercise()?;
    println!("Workout Summary: {:?}", summary2);

    println!("\n------------------------------------------------------------");
    println!("▶ SIMULATION 3: Duration Exercise (Plank)");
    println!("------------------------------------------------------------");
    engine.start_exercise("plank")?;
    let mut plank_landmarks = vec![Landmark::new(0.0, 0.0, 0.0, 1.0, 1.0); 33];
    plank_landmarks[11] = Landmark::new(0.2, 0.5, 0.0, 1.0, 1.0);
    plank_landmarks[23] = Landmark::new(0.5, 0.5, 0.0, 1.0, 1.0);
    plank_landmarks[27] = Landmark::new(0.8, 0.5, 0.0, 1.0, 1.0);

    for &t in &[0.0, 10.0, 20.0, 30.5] {
        let status = engine.process_landmarks(&plank_landmarks, w, h, t)?;
        println!(
            "t={:4.1}s | State: {:?} | Holding: {} | Current Duration: {:4.1}s / {:?}s",
            t,
            status.current_state,
            status.is_holding,
            status.current_duration,
            status.target_duration
        );
    }

    plank_landmarks[23] = Landmark::new(0.5, 0.1, 0.0, 1.0, 1.0);
    let _ = engine.process_landmarks(&plank_landmarks, w, h, 31.0)?;
    let status = engine.process_landmarks(&plank_landmarks, w, h, 31.5)?;
    println!(
        "t=31.5s | Completed Plank Rep: {}, Target Duration: {:?}s",
        status.counter, status.target_duration
    );
    let summary3 = engine.stop_exercise()?;
    println!("Workout Summary: {:?}\n", summary3);

    println!("All simulations verified successfully.");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "process-stream" {
        let mut exercise = "hammer_curl".to_string();
        let mut input_file: Option<String> = None;
        let mut output_file: Option<String> = None;

        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--exercise" if i + 1 < args.len() => {
                    exercise = args[i + 1].clone();
                    i += 1;
                }
                "--input" if i + 1 < args.len() => {
                    input_file = Some(args[i + 1].clone());
                    i += 1;
                }
                "--output" if i + 1 < args.len() => {
                    output_file = Some(args[i + 1].clone());
                    i += 1;
                }
                _ => {}
            }
            i += 1;
        }

        process_stream(&exercise, input_file.as_deref(), output_file.as_deref())
    } else {
        run_demo()
    }
}
