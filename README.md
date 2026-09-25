# 🏋️ PoseFit — Pure Rust Computer-Vision Fitness Engine & Android App

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Android](https://img.shields.io/badge/Android-NativeActivity-brightgreen.svg)](https://developer.android.com/ndk)
[![Java/Kotlin Footprint](https://img.shields.io/badge/Java%20%2F%20Kotlin-0%20lines-blue.svg)](#zero-java--zero-kotlin-architecture)
[![CI/CD](https://img.shields.io/badge/CI%2FCD-GitHub%20Actions-blueviolet.svg)](.github/workflows/android-rust-ci.yml)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

An ultra-high-performance, on-device AI fitness coaching application built in **100% pure Rust**. PoseFit tracks human body pose landmarks in real time, executes finite state machines to count repetitions or hold duration, scores exercise form quality (0–100 with A–F grading), delivers instant biomechanical correction cues, and renders a live HUD overlay.

---

## ⚡ Key Highlights

- **100% Pure Rust on Android**: Built on Android's `NativeActivity` (`android-activity`) with `android:hasCode="false"`. **Zero Kotlin, zero Java, zero JVM runtime overhead**.
- **18 Built-in Exercise Definitions**: Evaluated via a custom, sandboxed AST expression engine for 60 mathematical condition rules.
- **On-Device ML Inference Pipeline**: Embedded aspect-ratio letterboxing, normalization, and 33 3D landmark tensor decoding for BlazePose.
- **Dependency-Free Native HUD & Skeleton**: Built-in raster renderer and 8x16 embedded bitmap font that renders skeleton vectors, joint angle gauges, rep counters, and scorecards directly into Android's `ANativeWindow_Buffer`.
- **Zero-Storage Cloud Compilation**: Automated GitHub Actions CI workflow compiles the ARM64 Android `.so` and packages `PoseFit-release.apk` in the cloud without requiring local NDK storage.

---

## 📐 System Architecture

```text
┌────────────────────────────────────────────────────────────────────────┐
│ Android OS (Linux Kernel 5.x / 6.x + SurfaceFlinger + Camera2 HAL)     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Launches android.app.NativeActivity
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│               libposefit_android.so (100% Compiled Rust)               │
│                                                                        │
│  #[unsafe(no_mangle)]                                                  │
│  fn android_main(app: AndroidApp) {                                    │
│      ├── 1. Pure Rust Native Event Loop (android-activity)             │
│      │      └─ Ingests Window, Input, Pause, Resume, Destroy events    │
│      │                                                                 │
│      ├── 2. Direct Camera Ingestion (camera2-ndk / AImageReader)       │
│      │      └─ Zero JNI: Receives raw RGB byte slices directly         │
│      │                                                                 │
│      ├── 3. On-Device Neural Network Inference (BlazePoseEstimator)    │
│      │      ├─ Letterbox Preprocessing & Aspect-Ratio Normalization    │
│      │      └─ 33 3D Landmark Coordinate Tensor Decoding & Inversion   │
│      │                                                                 │
│      ├── 4. Exercise State Machine & Counting Engine (PoseFitEngine)   │
│      │      ├─ 18 Bundled YAML Exercises (Squat, Bicep Curl, Plank...) │
│      │      ├─ 2D Cosine Angle Math & Temporal Smoothing Filters       │
│      │      ├─ Rep Debounce & Independent Bilateral Arm Isolation      │
│      │      ├─ Isometric Hold Duration & Break Tracking (Plank)        │
│      │      └─ Form Scoring (0-100) & Real-Time Feedback Warnings      │
│      │                                                                 │
│      ├── 5. On-Device HUD & Skeleton Overlay (HudRenderer)             │
│      │      ├─ Anti-aliased skeleton lines & colored joint dots        │
│      │      ├─ Real-time HUD banner, rep counts, and grade pill        │
│      │      ├─ Self-contained 8x16 embedded bitmap typography          │
│      │      └─ Renders directly into ANativeWindow_Buffer              │
│      │                                                                 │
│      └── 6. Interactive Touch Input Handler (Native Motion Events)     │
│             ├─ Tap Top-Left: Cycle through exercises                   │
│             ├─ Tap Top-Right: Finish workout & view scorecard modal    │
│             └─ Tap Modal: Dismiss and start next exercise              │
│  }                                                                     │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 📂 Repository Workspace Structure

```text
PoseFit/
├── .github/workflows/
│   └── android-rust-ci.yml       # Cloud CI: Checks, tests, and builds PoseFit.apk
├── data/
│   ├── dumbel-workout.mp4        # Sample workout video
│   └── annotage_dumbel-workout.mp4 # Video annotated with Rust engine & HUD
├── exercises/definitions/        # 18 Canonical exercise YAML definitions
│   ├── squat.yaml, bicep_curl.yaml, hammer_curl.yaml, plank.yaml...
├── PoseFitApp/                   # Pure-Rust Cargo Workspace
│   ├── Cargo.toml                # Multi-crate virtual workspace manifest
│   ├── build-apk.sh              # Local Android packaging script
│   ├── README_ANDROID.md         # In-depth Android NDK deployment guide
│   │
│   ├── posefit-core/             # Core Computer-Vision & Fitness Logic
│   │   ├── src/
│   │   │   ├── definitions/      # YAML loader & 18 embedded configs
│   │   │   ├── engine.rs         # PoseFitEngine coordinating exercise flow
│   │   │   ├── exercises/        # Standard, Bilateral, Duration engines
│   │   │   ├── feedback.rs       # Real-time form defect alerts
│   │   │   ├── fsm.rs            # 60-condition AST expression parser
│   │   │   ├── inference/        # BlazePose pre/postprocessing & estimator
│   │   │   ├── landmarks.rs      # 33 MediaPipe landmark coordinates
│   │   │   ├── math.rs           # 2D vector cosine angle math
│   │   │   ├── rendering/        # Raster primitives, embedded font & HUD
│   │   │   ├── scoring.rs        # 0-100 Form scoring & grade boundaries
│   │   │   └── smoothing.rs      # Temporal moving average window filters
│   │   └── tests/golden_parity.rs# 7 Golden verification tests
│   │
│   ├── posefit-android/          # Android NativeActivity App
│   │   ├── AndroidManifest.xml   # android:hasCode="false"
│   │   └── src/lib.rs            # android_main event loop & touch handler
│   │
│   └── posefit-app/              # Desktop CLI & Video Stream Runner
│       └── src/main.rs           # CLI subcommands (test, info, process-stream)
│
└── SUMMARY.md                    # Complete reverse-engineering & migration audit
```

---

## 🏃 Supported Exercises (18 Bundled)

| Category | Exercises | Tracking Method |
| :--- | :--- | :--- |
| **Standard Repetition** | Squat, Push-up, Lunge, Deadlift, Tricep Dip, Glute Bridge, Jumping Jack, Mountain Climber, High Knees, Leg Raise, Side Lunge | Cyclic FSM state transition & debounce |
| **Bilateral Isolation** | Bicep Curl, Hammer Curl, Shoulder Press, Lateral Raise, Calf Raise | Independent Left/Right arm state machines |
| **Duration & Isometric** | Plank, Wall Sit | Continuous holding time accumulator & break tracking |

---

## 🧪 Local Testing & Verification

All tests compile cleanly on desktop (macOS / Linux / Windows) without requiring an Android device or emulator:

```bash
cd PoseFitApp

# 1. Format check
cargo fmt --all --check

# 2. Workspace compilation check
cargo check --workspace

# 3. Run all 71 unit & golden parity tests
cargo test --workspace

# 4. Strict Clippy linter
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

## 🚀 Building & Deploying the Android App

### Method 1: Cloud Build via GitHub Actions (Recommended)

1. Push your code to GitHub:
   ```bash
   git push origin main
   ```
2. Navigate to the **Actions** tab on your GitHub repository.
3. Once the workflow completes, download the compiled **`PoseFit-release-apk`** artifact.
4. Install directly to your phone:
   ```bash
   adb install -r PoseFit-release.apk
   ```

### Method 2: Local Compilation (Requires Android NDK)

If you have the Android NDK installed:

```bash
# Add ARM64 Rust target
rustup target add aarch64-linux-android

# Build native shared library & package APK
./PoseFitApp/build-apk.sh aarch64-linux-android

# Stream real-time Android logcat logs
adb logcat -s PoseFitRust:V
```

---

## 🕹️ Interactive Android Controls

| Action | Screen Region | Behavior |
| :--- | :--- | :--- |
| **Cycle Exercise** | Tap Top-Left (`EXERCISE: SQUAT [TAP TO CYCLE]`) | Immediately switches to the next bundled exercise |
| **Finish Workout** | Tap Top-Right (`FORM SCORE`) | Concludes session and brings up full-screen scorecard |
| **Dismiss Scorecard**| Tap Anywhere | Returns to live camera tracking for the next exercise |

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
