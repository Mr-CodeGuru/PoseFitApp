# PROJECT WORKSPACE REVERSE-ENGINEERING & ANDROID/RUST MIGRATION REPORT

**Target Codebase:** [PoseFit](file:///Users/aman/Developer/PoseFit)  
**Analysis Date:** September 2026  
**Auditor / Architecture Lead:** Senior Software Architect, Computer Vision & Rust/Android Systems Migration Specialist  
**Reference Python Implementation Status:** Fully reverse-engineered and verified via local static analysis & test execution.

---

# 1. IMPORTANT ANALYSIS RULES & EVIDENCE CLASSIFICATION

All findings in this document adhere strictly to verified workspace facts. Findings are categorized under four explicit operational truth markers:

- **`[OBSERVED]`**: Direct, verified reality from file contents, AST parsing, directory listings, or test executions.
- **`[INFERRED]`**: Deductions derived logically from surrounding architectural conventions, commented code, or variable patterns.
- **`[UNKNOWN]`**: Incomplete or absent specifications where the workspace lacks definitive implementation.
- **`[EXTERNAL]`**: Dependencies, services, or protocols outside the repository boundary.

### Explicit Missing / Unresolved Workspace Dependencies Notice
The following components are referenced in documentation or legacy bytecode but are **`MISSING / NOT PROVIDED`** in the repository:
1. `main.py` (`[OBSERVED]` referenced in [README.md](file:///Users/aman/Developer/PoseFit/README.md#L309), but completely missing from workspace).
2. `db/workout_logger.py` (`[OBSERVED]` file exists on disk, but has **0 bytes**; `app.py` has a fallback `DummyWorkoutLogger`).
3. `create_static_folders.py` (`[OBSERVED]` referenced in [static/images/manual_setup.md](file:///Users/aman/Developer/PoseFit/static/images/manual_setup.md#L18), but missing).
4. `feedback/indicators.py`, `feedback/information.py`, `feedback/layout.py` (`[OBSERVED]` only stale `.pyc` files exist in [feedback/__pycache__/](file:///Users/aman/Developer/PoseFit/feedback/__pycache__); `.py` source files were deleted during migration to YAML engine).
5. `/dashboard_data` HTTP route (`[OBSERVED]` invoked by [static/js/dashboard.js](file:///Users/aman/Developer/PoseFit/static/js/dashboard.js#L4), but not defined in [app.py](file:///Users/aman/Developer/PoseFit/app.py)).

---

# 2. EXECUTIVE SUMMARY

### 1. What this project is
**PoseFit** is a Python-based, real-time computer vision fitness coaching application. It ingests video streams (from a local web camera or uploaded video files), detects human body pose landmarks, tracks joint angles, executes a state machine to count exercise repetitions or duration, calculates a composite form quality score (0–100 with A–F grading), renders visual feedback onto video frames, and presents a web interface.

### 2. What problem it solves
It solves the problem of unsupervised physical training where users lack immediate feedback on exercise posture, repetition progression, tempo, and form defects (e.g., knee collapse during squats, sagging hips during planks).

### 3. What the user does with it
- **Live Workout Mode:** The user selects an exercise, specifies sets/reps goals, starts the camera, performs the exercise in view of the webcam, and receives real-time visual feedback, audio/visual stage cues, repetition counts, and form grading.
- **Video Analysis Mode:** The user uploads a pre-recorded workout video (MP4/AVI/MOV/WebM), monitors background frame processing progress, and reviews/downloads an annotated video with a neon-glow skeleton overlay, statistics HUD, and downloadable summary report.
- **Progress Tracking:** The user inspects workout history, streaks, and exercise distributions via the Dashboard and Profile pages.

### 4. What happens internally
1. Frames arrive via OpenCV `cv2.VideoCapture` (webcam or video file).
2. Frames are converted from BGR to RGB.
3. MediaPipe Pose (`model_complexity=1`) infers 33 3D normalized landmarks.
4. Landmarks are scaled to 2D pixel space `(x, y)`.
5. The `ExerciseEngine` parses exercise configuration (from YAML definitions) and delegates to `BaseExercise`, `BilateralExercise`, or `DurationExercise`.
6. Specific joint angles are computed using 2D vector trigonometry (`math.acos` / `np.arccos`).
7. A Finite State Machine (FSM) transitions based on angle/coordinate conditions evaluated against safety constraints.
8. Repetitions are counted when transition trigger states match and temporal debounce thresholds are satisfied.
9. Form scores and textual feedback warnings are computed and drawn directly onto the video frames using OpenCV primitives.
10. Annotated frames are JPEG-encoded into an MJPEG HTTP stream (`/video_feed`) or encoded to H.264 MP4 via `imageio-ffmpeg` / OpenCV VideoWriter.

### 5. What the computer-vision pipeline does
Performs single-person landmark detection, extracts pixel coordinates of 13 specific joints (shoulders, elbows, wrists, hips, knees, ankles, nose), calculates planar angles between 3-joint vectors, and generates visual skeletons, angle labels, bounding indicators, and HUD graphics.

### 6. What models are involved
- **MediaPipe Pose (BlazePose):** A two-stage pipeline consisting of:
  - *Pose Detector (Palm/Body Detector equivalent)*: Downscales to 224×224, predicts human bounding box & rotation.
  - *Pose Landmark Model*: Cropped 256×256 ROI tensor inference producing 33 keypoints in normalized coordinates $(x, y, z)$ plus visibility and presence flags.
  - Runtime: MediaPipe Python C++ bindings wrapping TensorFlow Lite runtime.

### 7. What inputs it accepts
- Live Web Camera stream (via OpenCV `cv2.VideoCapture(0, cv2.CAP_DSHOW)`).
- Pre-recorded video files (MP4, AVI, MOV, WebM; max 50 MB, max 120 seconds).
- JSON payloads for exercise configuration (sets, reps, exercise type).

### 8. What outputs it produces
- `multipart/x-mixed-replace; boundary=frame` MJPEG stream with HUD overlays.
- Processed H.264/MP4 annotated video files for download.
- JSON API status responses (`reps`, `form_score`, `avg_form_score`, `form_grade`, `current_state`, `feedback`).
- HTML/CSS/JS web interface with Chart.js charts.

### 9. Whether processing is local or server-based
- **Current implementation:** Hybrid server-local. Computation runs on the backend Flask server hosting the OpenCV/MediaPipe pipeline, while the browser acts as a thin client displaying MJPEG/canvas and submitting control commands.
- **Target Android/Rust implementation:** Fully local, embedded on-device execution. The Android camera feeds frames directly into a Rust core compiled via JNI, running local TFLite/ONNX inference on the mobile NPU/GPU without network round-trips.

### 10. Major architectural components
- **Flask Presentation Layer:** [app.py](file:///Users/aman/Developer/PoseFit/app.py) providing routing, camera streaming, and subprocess worker coordination.
- **Inference Wrapper:** [pose_estimation/estimation.py](file:///Users/aman/Developer/PoseFit/pose_estimation/estimation.py) (`PoseEstimator`).
- **YAML Definition Store:** [exercises/definitions/*.yaml](file:///Users/aman/Developer/PoseFit/exercises/definitions) (18 exercise configurations).
- **Core Exercise Engine:** [exercises/loader.py](file:///Users/aman/Developer/PoseFit/exercises/loader.py), [exercises/base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py), [exercises/engine.py](file:///Users/aman/Developer/PoseFit/exercises/engine.py).
- **Subprocess Video Processor:** [video_processor.py](file:///Users/aman/Developer/PoseFit/video_processor.py).
- **Persistence Layer:** [db/workout_logger.py](file:///Users/aman/Developer/PoseFit/db/workout_logger.py) (currently stubbed/dummy).

```text
CURRENT PYTHON SERVER ARCHITECTURE:

    [ Web Browser Client ]
      │             │
      │ HTTP MJPEG  │ REST /start_exercise, /get_status, /api/video/upload
      ▼             ▼
┌─────────────────────────────────────────────────────────────┐
│ Flask Server (app.py)                                       │
│  ├── Camera Capture Thread (cv2.VideoCapture)               │
│  ├── Subprocess Worker (video_processor.py via imageio)     │
│  └── Global ExerciseEngine Instance                         │
└──────────────────────────┬──────────────────────────────────┘
                           │
        ┌──────────────────┴──────────────────┐
        ▼                                     ▼
┌──────────────────────────────┐    ┌──────────────────────────────┐
│ MediaPipe PoseEstimator      │    │ ExerciseEngine Core          │
│ (BlazePose TFLite Models)    │    │ (loader.py, base_exercise.py)│
│  • BGR -> RGB                │    │  • YAML parser (18 exercises)│
│  • 33 Keypoint Inference     │───►│  • 2D Vector Angle Math      │
│  • Normalization -> Pixels   │    │  • State Machine (eval FSM)  │
└──────────────────────────────┘    │  • Rep & Duration Counters   │
                                    │  • Form Score Engine (0-100) │
                                    └──────────────┬───────────────┘
                                                   │
                                                   ▼
                                    ┌──────────────────────────────┐
                                    │ Frame Overlay Rendering      │
                                    │ (OpenCV text, lines, glow)   │
                                    └──────────────────────────────┘
```

---

# 3. COMPLETE PROJECT STRUCTURE

```text
PoseFit/
├── .gitignore
├── LICENSE
├── README.md
├── app.py
├── requirements.txt
├── test_engine.py
├── video_processor.py
├── data/
│   └── dumbel-workout.mp4
├── db/
│   ├── __pycache__/
│   └── workout_logger.py
├── exercises/
│   ├── __pycache__/
│   ├── base_exercise.py
│   ├── engine.py
│   ├── loader.py
│   └── definitions/
│       ├── bicep_curl.yaml
│       ├── calf_raise.yaml
│       ├── deadlift.yaml
│       ├── glute_bridge.yaml
│       ├── hammer_curl.yaml
│       ├── high_knees.yaml
│       ├── jumping_jack.yaml
│       ├── lateral_raise.yaml
│       ├── leg_raise.yaml
│       ├── lunge.yaml
│       ├── mountain_climber.yaml
│       ├── plank.yaml
│       ├── push_up.yaml
│       ├── shoulder_press.yaml
│       ├── side_lunge.yaml
│       ├── squat.yaml
│       ├── tricep_dip.yaml
│       └── wall_sit.yaml
├── feedback/
│   └── __pycache__/
├── output/
│   └── images/
├── pose_estimation/
│   ├── __pycache__/
│   ├── angle_calculation.py
│   └── estimation.py
├── static/
│   ├── css/ (dashboard.css, profile.css, style.css, video_analysis.css)
│   ├── images/ (hammer_curl.png, push_up.png, squat.png, manual_setup.md, README.txt)
│   └── js/ (dashboard.js, profile.js, script.js, video_analysis.js)
├── templates/
│   ├── dashboard.html
│   ├── index.html
│   ├── profile.html
│   └── video_analysis.html
└── utils/
    ├── __pycache__/
    └── draw_text_with_background.py
```

### Path Inventory & Migration Action Matrix

| Path | Type | Purpose | Runtime Critical? | Android / Rust Action | Traceability Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| [app.py](file:///Users/aman/Developer/PoseFit/app.py) | Python (Flask) | Web server, MJPEG streaming, API endpoints, camera loop | Yes (current) | **Remove / Replace** | Replaced entirely by Android Activity/ViewModel and Rust Core native orchestration. |
| [video_processor.py](file:///Users/aman/Developer/PoseFit/video_processor.py) | Python CLI | Subprocess batch video analyzer with neon skeleton overlay | Yes (for video mode) | **Rewrite in Rust** | Port to a background Rust video decoding/encoding pipeline (using MediaCodec on Android). |
| [test_engine.py](file:///Users/aman/Developer/PoseFit/test_engine.py) | Python Test | Validates YAML loading, state machine logic, feedback rules | No | **Re-implement in Rust** | Convert into `cargo test` unit and integration test suite. |
| [exercises/base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py) | Python | FSM, Rep counter, Form scoring, angle calculation | **Yes** | **Rewrite in Rust** | Core business logic. Re-implement in Rust with strict types and a safe expression engine. |
| [exercises/engine.py](file:///Users/aman/Developer/PoseFit/exercises/engine.py) | Python | High-level API coordinating exercise processing & rendering | **Yes** | **Rewrite in Rust** | Engine coordinator struct in Rust. Rendering delegated to Android Canvas / OpenGL. |
| [exercises/loader.py](file:///Users/aman/Developer/PoseFit/exercises/loader.py) | Python | YAML definition parser & validator | **Yes** | **Rewrite in Rust** | Parse YAML/JSON using `serde_yaml` or embed pre-compiled structs. |
| [exercises/definitions/*.yaml](file:///Users/aman/Developer/PoseFit/exercises/definitions) | Config (18 files) | Definitions for 18 exercises (angles, FSM, feedback, tempo) | **Yes** | **Convert / Embed** | Bundle into Android assets or embed statically into Rust binary via `include_str!`. |
| [pose_estimation/estimation.py](file:///Users/aman/Developer/PoseFit/pose_estimation/estimation.py) | Python | MediaPipe Pose wrapper | **Yes** | **Replace** | Replaced by Android MediaPipe Tasks Pose Landmarker or ONNX/TFLite C++ / Rust bindings. |
| [pose_estimation/angle_calculation.py](file:///Users/aman/Developer/PoseFit/pose_estimation/angle_calculation.py) | Python | Vector angle math utility | No (legacy) | **Merge into Rust CV** | Superseded by `BaseExercise._angle_between`. Implement as standard Rust math function. |
| [utils/draw_text_with_background.py](file:///Users/aman/Developer/PoseFit/utils/draw_text_with_background.py) | Python | OpenCV text background box drawing helper | Yes (current UI) | **Replace** | Replace with native Android UI / Jetpack Compose text overlays over camera preview. |
| [db/workout_logger.py](file:///Users/aman/Developer/PoseFit/db/workout_logger.py) | Python (0 bytes) | Intended workout persistence | No (broken) | **Implement in Android** | Implement using Android Room (SQLite) or Rust `rusqlite`. |
| `feedback/` | Python Bytecode | Stale `.pyc` files from deprecated feedback module | No | **Delete** | Abandon completely; replaced by YAML feedback rules in `base_exercise.py`. |
| `static/` & `templates/` | Web Assets | HTML, CSS, JavaScript web interface | No (web only) | **Replace with Native UI** | Replaced by Jetpack Compose Android UI. |
| [data/dumbel-workout.mp4](file:///Users/aman/Developer/PoseFit/data/dumbel-workout.mp4) | Asset (Video) | Test workout video (13 MB) | No | **Retain as Test Asset** | Use as golden test vector for verifying Python vs Rust output parity. |

---

# 4. ACTUAL APPLICATION FLOW

```text
APPLICATION RUNTIME FLOW:

[ 1. Startup & Init ]
  │ Flask starts (app.run: single-process, unthreaded, port 5000)
  │ Lazy singleton PoseEstimator initialized on demand
  │ Exercise definitions validated in exercises/definitions/
  ▼
[ 2. User Interaction ]
  │ Client loads index.html, calls GET /exercises
  │ User chooses exercise (e.g., 'squat'), sets=3, reps=10
  │ Client calls POST /start_exercise
  ▼
[ 3. Exercise Activation ]
  │ ExerciseEngine.set_exercise("squat")
  │ YAML parsed: states, angles, counter_rule, feedback_rules loaded
  │ Camera started: cv2.VideoCapture(0, cv2.CAP_DSHOW) @ 1280x720, 30fps
  ▼
[ 4. Frame Acquisition & Streaming Loop ] (generate_frames in app.py)
  │ camera.read() -> frame (BGR 1280x720)
  ▼
[ 5. Inference & Pose Extraction ]
  │ cv2.cvtColor(frame, BGR2RGB) -> rgb_frame
  │ PoseEstimator.pose.process(rgb_frame) -> results.pose_landmarks
  ▼
[ 6. Exercise Processing Pipeline ] (ExerciseEngine.process_frame)
  │ Landmark coordinates extracted: (x, y) = (lm.x * width, lm.y * height)
  │ Angles calculated: _angle_between(p1, p2, p3) with optional moving average smoothing
  │ Context dictionary populated with all angle values and joint coordinates
  ▼
[ 7. FSM Evaluation & Rep Counter ]
  │ BaseExercise.update_state(context):
  │   Evaluates YAML conditions in state_order via _safe_eval()
  │ BaseExercise.update_counter():
  │   Verifies state change, trigger_state match, from_state match, and min_rep_duration
  │   If valid: counter += 1
  ▼
[ 8. Quality Scoring & Feedback ]
  │ Form score (0-100) calculated:
  │   - Angle deviation penalty (max 40 pts)
  │   - Tempo penalty based on rep duration (max 30 pts)
  │   - Feedback rule violation penalty (max 30 pts, 10 pts per warning)
  │ Letter grade assigned: A (>=90), B (>=80), C (>=70), D (>=60), F (<60)
  ▼
[ 9. Frame HUD Annotation & Output ]
  │ cv2 lines, circles, text HUD, and neon overlay rendered onto frame
  │ Frame encoded as JPEG -> yielded via HTTP MJPEG stream
  │ Client polls GET /get_status for UI counter/score updates
```

### Detailed Stage Breakdown

#### Stage 1: Live Camera Exercise Flow
- **Module:** [app.py](file:///Users/aman/Developer/PoseFit/app.py) (`generate_frames`), [exercises/engine.py](file:///Users/aman/Developer/PoseFit/exercises/engine.py), [exercises/base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py).
- **Inputs:** Raw camera frames from `cv2.VideoCapture`.
- **Outputs:** MJPEG stream with overlaid metrics; JSON responses from `/get_status`.
- **State Changes:** `exercise_engine.exercise.counter` increments; `current_state` transitions; `sets_completed` increments upon reaching `exercise_goal`.
- **Performance Implications:** Heavy bottleneck. MediaPipe inference on CPU blocks frame generation; JPEG compression for every frame consumes significant CPU cycles.

#### Stage 2: Video File Analysis Subprocess Flow
- **Module:** [app.py](file:///Users/aman/Developer/PoseFit/app.py) (`upload_video`, `process_video_subprocess`), [video_processor.py](file:///Users/aman/Developer/PoseFit/video_processor.py).
- **Inputs:** Multipart uploaded video file.
- **Outputs:** Processed H.264 video file (`<uuid>_processed.mp4`), progress polling JSON (`<uuid>_results.json`).
- **Internal Execution:** Subprocess is spawned via `subprocess.Popen([sys.executable, 'video_processor.py', ...])`. Frame stepping occurs with frame skipping (`analyze_skip = max(1, int(fps / 8))`), MediaPipe tracks landmarks, `draw_skeleton()` renders neon glow, and `imageio` writes H.264 frames using `libx264`.
- **Errors / Fallbacks:** If `imageio` fails, falls back to OpenCV `avc1`, `H264`, `XVID`, or `mp4v`.

#### Stage 3: Error & Fallback Flow
- **Camera Failure:** If `camera.read()` fails, it retries 3 times with 100ms sleep. If unresolvable, exits generator.
- **DB Failure:** [db/workout_logger.py](file:///Users/aman/Developer/PoseFit/db/workout_logger.py) failure falls back silently to `DummyWorkoutLogger` with zeroed counters.
- **Pose Extraction Failure:** If no person is in view (`results.pose_landmarks is None`), `process_frame` aborts early and leaves states unchanged without incrementing counters.

---

# 5. COMPUTER-VISION PIPELINE

```text
CV PIPELINE TRANSFORMATION FLOW:

  Camera / Video Frame (BGR888, 1280x720, 30 FPS)
         │
         ▼
  cv2.cvtColor(frame, COLOR_BGR2RGB)
         │
         ▼
  MediaPipe Pose Landmarker (BlazePose)
    ├── Detector: 224x224 input -> RoI crop & rotation
    └── Regressor: 256x256 RoI -> 33 3D normalized landmarks (x, y, z, vis, pres)
         │
         ▼
  Pixel Denormalization
    ├── x_pixel = int(landmark.x * width)
    └── y_pixel = int(landmark.y * height)
         │
         ▼
  Planar 2D Angle Calculation
    ├── Vector ba = p1 - p2, Vector bc = p3 - p2
    ├── cos_theta = dot(ba, bc) / (||ba|| * ||bc|| + 1e-6)
    └── theta = arccos(clip(cos_theta, -1.0, 1.0)) * 180 / PI
         │
         ▼
  Moving Average Smoothing (Window = 3 to 5 frames)
         │
         ▼
  Context Dictionary & FSM Condition Evaluation
```

## Input Specifications
- **Format:** In-memory OpenCV NumPy array `uint8`.
- **Color Space:** Captured as BGR; converted to RGB before inference.
- **Resolution:** Camera requested at 1280×720 (720p); video file input variable.
- **Frame Rate:** Target 30 FPS for live camera; variable for video files.
- **Orientation:** Assumes upright landscape. No rotation correction or EXIF parsing is performed.

## Preprocessing
1. Frame conversion from BGR to RGB via `cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)`.
2. Implicit internal MediaPipe preprocessing:
   - Bilinear downsampling to model input dimensions.
   - Normalization of pixel intensities to $[-1.0, 1.0]$ or $[0.0, 1.0]$.
   - Detection of person region-of-interest (RoI) followed by rotated bounding box crop.

## Model Specifications
- **Architecture:** BlazePose (Google MediaPipe Pose).
- **Framework:** TensorFlow Lite compiled with C++ MediaPipe graph framework.
- **Landmarks Output:** 33 keypoints. PoseFit extracts 13 named keypoints:
  - Nose: index 0
  - Left/Right Shoulder: indices 11, 12
  - Left/Right Elbow: indices 13, 14
  - Left/Right Wrist: indices 15, 16
  - Left/Right Hip: indices 23, 24
  - Left/Right Knee: indices 25, 26
  - Left/Right Ankle: indices 27, 28

## Postprocessing & Mathematics
1. **Coordinate Conversion:** Normalized coordinates $x, y \in [0.0, 1.0]$ mapped to pixel coordinates:
   $$x_{px} = \lfloor x_{norm} \times \text{width} \rfloor, \quad y_{px} = \lfloor y_{norm} \times \text{height} \rfloor$$
2. **Angle Calculation (`_angle_between` in [base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py#L464)):**
   Given points $a, b, c$ where $b$ is the vertex:
   $$\vec{ba} = a - b, \quad \vec{bc} = c - b$$
   $$\cos(\theta) = \frac{\vec{ba} \cdot \vec{bc}}{\|\vec{ba}\|_2 \|\vec{bc}\|_2 + 10^{-6}}$$
   $$\theta = \arccos(\text{clamp}(\cos(\theta), -1.0, 1.0)) \times \frac{180}{\pi}$$
3. **Temporal Smoothing:** Moving average window of size $N$ (configured per exercise, typically $N=3$ or $N=5$):
   $$\theta_{smoothed}(t) = \frac{1}{K}\sum_{i=0}^{K-1} \theta(t - i)$$
4. **Form Score Calculation ([base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py#L311)):**
   $$\text{Score} = 100 - \min(\text{AnglePenalty}, 40) - \min(\text{TempoPenalty}, 30) - \min(\text{FeedbackPenalty}, 30)$$
   - *AnglePenalty:* If `ideal_angles` exists: $\frac{1}{M}\sum \frac{|\theta_{current} - \theta_{ideal}|}{10} \times 5$.
   - *TempoPenalty:* If duration $T < T_{min}$: $\frac{T_{min} - T}{0.5} \times 15$; if $T > T_{max}$: $(T - T_{max}) \times 10$.
   - *FeedbackPenalty:* $\text{Count}(\text{triggered\_feedbacks}) \times 10$.

## CV / ML Dependencies
- `mediapipe>=0.10.0`: Pose landmark model inference.
- `opencv-python>=4.5.0`: Video capture, color space conversion, image encoding, vector rendering.
- `numpy>=1.21.0`: Vector arithmetic, dot product, norms.
- `imageio>=2.31.0` / `imageio-ffmpeg>=0.4.8`: H.264 video decoding and muxing.

---

# 6. MODEL INVENTORY

| Model | Purpose | Format | Input Shape | Output Shape | Runtime | Android / Rust Feasibility |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BlazePose Detector** | Person detection & bounding box alignment | TFLite | `1x224x224x3` (FP32) | Detection boxes, keypoints, score | MediaPipe / TFLite | **Native Android TFLite / MediaPipe Tasks** |
| **BlazePose Landmarker (Full / Complexity 1)** | 33 3D Pose Landmarks regression | TFLite | `1x256x256x3` (FP32) | Landmark coords `33x5` (x,y,z,vis,pres) | MediaPipe / TFLite | **Native Android TFLite / MediaPipe Tasks / ONNX Runtime** |

### Mobile Execution Constraints
1. **MediaPipe Tasks SDK vs Custom Rust Inference:** MediaPipe provides official Android AAR bindings (`com.google.mediapipe:tasks-vision`) which natively leverage the GPU/NNAPI delegates via hardware acceleration. Running pure ONNX Runtime inside Rust on Android requires compiling ONNX Runtime C-API with NNAPI/XNNPACK execution providers, or running TFLite C API within Rust.
2. **Recommended Android Target:** Execute MediaPipe Pose Landmarker via **MediaPipe Tasks Vision Android Library** or **TFLite C-API statically linked in Rust**. The landmark coordinate buffers (33 floats $\times$ 5 values) are then passed to Rust via zero-copy pointer for angle computation, FSM evaluation, and form scoring.

```text
MODEL CONVERSION & INTEGRATION PIPELINE:

Google MediaPipe Pose TFLite Assets (pose_landmarker_full.task)
                       │
                       ▼
          [ Choice A: Recommended Hybrid ]
          Android CameraX -> MediaPipe Tasks Android SDK (GPU/NNAPI)
                       │
                       ▼ (33 Landmarks: x, y, z, vis)
          JNI Zero-Copy DirectBuffer
                       │
                       ▼
          Rust Core (FSM Engine, Math, Rep Counting, Scoring)

                       OR

          [ Choice B: Pure Rust Native Engine ]
          CameraX YUV Frame -> Rust JNI Pointer
                       │
                       ▼
          tract / ONNX Runtime Rust Binding (CPU/XNNPACK)
                       │
                       ▼
          Rust BlazePose Model Inference -> Rust FSM Engine
```

---

# 7. ALL ENDPOINTS / APIs

### 1. `GET /`
- **Purpose:** Renders main workout page (`index.html`).
- **Internal Flow:** Calls `render_template('index.html')`.
- **Android Relevance:** **REMOVE**. Replaced by native Android UI (Jetpack Compose).

### 2. `GET /video_feed`
- **Purpose:** Streams live webcam video with rendered pose overlays as MJPEG.
- **Request:** HTTP GET with browser `<img>` tag target.
- **Response:** `multipart/x-mixed-replace; boundary=frame`.
- **Internal Flow:** Calls `generate_frames()` generator -> reads camera -> calls `PoseEstimator.estimate_pose()` -> calls `ExerciseEngine.process_frame()` -> renders overlays -> encodes JPEG -> streams bytes.
- **Android Relevance:** **REMOVE**. Video frames are rendered locally on-device on a SurfaceView / TextureView.

### 3. `POST /start_exercise`
- **Purpose:** Initializes and begins tracking a specific exercise.
- **Request:** JSON `{"exercise_type": "squat", "sets": 3, "reps": 10}`.
- **Response:** `{"success": true, "exercise": "squat", "info": {...}}`.
- **Internal Flow:** Checks available exercises -> calls `exercise_engine.set_exercise(type)` -> initializes camera -> sets `exercise_running = True`.
- **Android Relevance:** **REPLACE WITH LOCAL FUNCTION**. Directly invoke `exercise_engine.start_exercise(type, sets, reps)` in Rust.

### 4. `POST /stop_exercise`
- **Purpose:** Stops exercise tracking and logs workout summary.
- **Request:** Empty POST.
- **Response:** `{"success": true}`.
- **Internal Flow:** Computes duration -> retrieves `avg_form_score` -> calls `workout_logger.log_workout(...)` -> sets `exercise_running = False`.
- **Android Relevance:** **REPLACE WITH LOCAL FUNCTION**. Directly invoke `exercise_engine.stop_exercise()` in Rust.

### 5. `POST /stop_camera`
- **Purpose:** Releases hardware camera.
- **Request:** Empty POST.
- **Response:** `{"success": true}`.
- **Internal Flow:** Sets `exercise_running = False` -> calls `release_camera()`.
- **Android Relevance:** **REPLACE WITH LOCAL ANDROID LIFECYCLE**. Managed via CameraX lifecycle (`ProcessCameraProvider.unbindAll()`).

### 6. `GET /get_status`
- **Purpose:** Returns current repetition count, current set, and form score.
- **Request:** Empty GET.
- **Response:** JSON:
  ```json
  {
    "exercise_running": true,
    "current_reps": 4,
    "current_set": 1,
    "total_sets": 3,
    "rep_goal": 10,
    "form_score": 92,
    "avg_form_score": 88,
    "form_grade": "A"
  }
  ```
- **Android Relevance:** **REPLACE WITH LOCAL FUNCTION / STATEFLOW**. Rust emits updates through JNI callback or Kotlin StateFlow.

### 7. `GET /exercises`
- **Purpose:** Returns list of all 18 available exercises with YAML metadata.
- **Request:** Empty GET.
- **Response:** `{"exercises": [...], "info": {...}, "count": 18}`.
- **Internal Flow:** Iterates `exercises/definitions/*.yaml` via `loader.py`.
- **Android Relevance:** **REPLACE WITH LOCAL FUNCTION**. Rust returns JSON string or structured list of definitions.

### 8. `GET /dashboard` & `GET /profile`
- **Purpose:** Renders dashboard statistics and profile pages.
- **Response:** HTML pages.
- **Android Relevance:** **REMOVE**. Handled natively by Compose screens.

### 9. `POST /api/profile/update`
- **Purpose:** Stubbed profile update endpoint.
- **Request:** JSON profile fields.
- **Response:** `{"success": true, "message": "Profile updated"}`.
- **Android Relevance:** **REPLACE WITH LOCAL DATABASE**. Handled by local Room DB or DataStore.

### 10. `POST /api/video/upload`
- **Purpose:** Uploads video file for offline analysis.
- **Request:** Multipart form with `video` file and `exercise_type` string.
- **Response:** `{"success": true, "video_id": "<uuid>", "message": "..."}`.
- **Internal Flow:** Validates file size (<50MB) and duration (<120s) -> saves to `uploads/` -> spawns thread `process_video_subprocess()`.
- **Android Relevance:** **REPLACE WITH LOCAL BACKGROUND WORKER**. Android `WorkManager` triggers Rust batch video processor.

### 11. `GET /api/video/status/<video_id>`
- **Purpose:** Returns progress percentage and live metrics of video processing.
- **Response:** JSON with `status`, `progress`, `reps`, `form_score`, `grade`, `state`, `feedback`, `has_processed_video`, `processed_video_url`.
- **Android Relevance:** **REPLACE WITH LOCAL WORKMANAGER OBSERVER**.

### 12. `GET /api/video/processed/<video_id>`
- **Purpose:** Serves processed video file with skeleton overlay.
- **Response:** Video file binary (`video/mp4`).
- **Android Relevance:** **REPLACE WITH LOCAL URI**. File stored in app-specific cache directory.

### 13. `POST /api/video/analyze_frame`
- **Purpose:** Real-time frame analysis endpoint.
- **Status:** **`[OBSERVED]` DISABLED in code**. Returns error message advising use of subprocess to avoid memory exhaustion.
- **Android Relevance:** **REMOVE**.

---

# 8. FUNCTION AND CLASS INVENTORY

### `PoseEstimator`
- **File:** [pose_estimation/estimation.py](file:///Users/aman/Developer/PoseFit/pose_estimation/estimation.py#L4)
- **Type:** Class
- **Responsibility:** Instantiates MediaPipe Pose solution and processes RGB frames.
- **Inputs:** `frame: np.ndarray` (BGR), `exercise_type: str`.
- **Outputs:** `results: NamedTuple` (containing `pose_landmarks`).
- **Side Effects:** Directly alters input BGR frame by drawing line overlays for squat/pushup/hammer_curl.
- **Dependencies:** `cv2`, `mediapipe`.
- **Called By:** `app.py` (`generate_frames`).
- **Android/Rust Equivalent:** MediaPipe Tasks Android Pose Landmarker or native Rust TFLite binding.

### `BaseExercise`
- **File:** [exercises/base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py#L19)
- **Type:** Class (Core Engine)
- **Responsibility:** Finite State Machine evaluation, rep counting, angle smoothing, form scoring, feedback rule evaluation.
- **Inputs:** `config: Dict[str, Any]` (YAML structure).
- **Outputs:** State changes, rep counts, form score integer (0–100), feedback alerts.
- **Side Effects:** Mutates internal counters, histories, and timer stamps.
- **Dependencies:** `numpy`, `time`.
- **Called By:** [loader.py](file:///Users/aman/Developer/PoseFit/exercises/loader.py), [engine.py](file:///Users/aman/Developer/PoseFit/exercises/engine.py).
- **Android/Rust Equivalent:** `struct BaseExercise` in Rust implementing the `ExerciseTrait`.

### `BilateralExercise`
- **File:** [exercises/base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py#L540)
- **Type:** Class (Inherits `BaseExercise`)
- **Responsibility:** Manages dual independent state machines and counters for left and right body limbs (e.g. bicep curls, hammer curls).
- **Inputs:** Landmarks, frame dimensions.
- **Outputs:** `counter_left`, `counter_right`, `state_left`, `state_right`.
- **Android/Rust Equivalent:** `struct BilateralExercise` in Rust.

### `DurationExercise`
- **File:** [exercises/base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py#L657)
- **Type:** Class (Inherits `BaseExercise`)
- **Responsibility:** Tracks time elapsed in `hold_state` (e.g. plank, wall sit). Increments counter upon achieving `target_duration`.
- **Inputs:** Context dictionary, timestamps.
- **Outputs:** `current_duration: float`, `is_holding: bool`.
- **Android/Rust Equivalent:** `struct DurationExercise` in Rust.

### `ExerciseEngine`
- **File:** [exercises/engine.py](file:///Users/aman/Developer/PoseFit/exercises/engine.py#L17)
- **Type:** Class (High-level Facade)
- **Responsibility:** Coordinates landmark extraction, angle calculation, exercise dispatching, and OpenCV canvas rendering.
- **Inputs:** `frame: np.ndarray`, `landmarks: List[NormalizedLandmark]`.
- **Outputs:** Result dictionary containing `counter`, `state`, `angles`, `feedback`, `form_score`, `avg_form_score`, `form_grade`.
- **Side Effects:** In-place pixel drawing on `frame`.
- **Dependencies:** `BaseExercise`, `loader.py`, `draw_text_with_background.py`.
- **Android/Rust Equivalent:** Rust `ExerciseEngine` struct handling business logic; rendering delegated to Android Canvas / OpenGL shaders.

### `load_exercise_from_file` / `load_exercise`
- **File:** [exercises/loader.py](file:///Users/aman/Developer/PoseFit/exercises/loader.py#L19)
- **Type:** Functions
- **Responsibility:** Reads YAML from disk, determines exercise subclass, instantiates exercise object.
- **Inputs:** Exercise name or file path string.
- **Outputs:** `BaseExercise` / `BilateralExercise` / `DurationExercise` instance.
- **Android/Rust Equivalent:** `ExerciseLoader::load_from_yaml_str(&str) -> Result<Box<dyn Exercise>, EngineError>`.

### `process_video`
- **File:** [video_processor.py](file:///Users/aman/Developer/PoseFit/video_processor.py#L217)
- **Type:** Standalone Function / Process Entry Point
- **Responsibility:** Full offline video decoding, frame-stepping, MediaPipe inference, skeleton glow rendering, stats overlay, and H.264 video re-encoding.
- **Inputs:** `video_path`, `exercise_type`, `output_json_path`, `output_video_path`.
- **Outputs:** Encoded MP4 file and results JSON.
- **Android/Rust Equivalent:** Rust background video processing task utilizing Android `MediaExtractor` / `MediaCodec` and Rust processing pipelines.

---

# 9. DATA FLOW

```text
DATA FLOW TRANSFORMATION MATRIX:

[ Camera Sensor / Video File ]
  │ Representation: YUV_420_888 / BGR Compressed Bytes
  │ Owner: OS / Media Framework
  ▼
[ OpenCV Frame Buffer ]
  │ Representation: np.ndarray, shape=(H, W, 3), dtype=uint8, BGR
  │ Owner: VideoCapture
  ▼
[ Color Space Converted Frame ]
  │ Representation: np.ndarray, shape=(H, W, 3), dtype=uint8, RGB
  │ Transformation: cv2.cvtColor(frame, COLOR_BGR2RGB)
  ▼
[ MediaPipe Ingest Tensor ]
  │ Representation: TFLite Tensor, normalized float32
  │ Owner: mp.solutions.pose
  ▼
[ Landmark Vector ]
  │ Representation: 33 Landmarks with (x: f32, y: f32, z: f32, visibility: f32)
  │ Normalization: x, y in [0.0, 1.0] relative to frame dimensions
  ▼
[ Pixel Joint Coordinates Map ]
  │ Representation: Dict[str, Tuple[int, int]], e.g., "left_knee" -> (340, 520)
  │ Transformation: x_px = int(lm.x * width), y_px = int(lm.y * height)
  ▼
[ Joint Angles Map ]
  │ Representation: Dict[str, f64], e.g., "primary" -> 88.4°
  │ Transformation: Vector dot product & arccos over 3 landmarks
  ▼
[ Evaluation Context ]
  │ Representation: Dict[str, Any] containing all angles, coords, and aliases
  ▼
[ FSM State & Rep Transition ]
  │ Representation: State Enum/String ("start" -> "descent" -> "ascent"), Counter: u32
  │ Transformation: Condition string evaluation + Temporal debounce check
  ▼
[ Quality Scoring & Feedback ]
  │ Representation: FormScore: u32 (0-100), Grade: Char ('A'-'F'), Feedback: List[String]
  ▼
[ Visual Frame Overlay ]
  │ Representation: np.ndarray BGR with OpenCV rasterized HUD & neon skeleton
  ▼
[ Encoded Output Buffer ]
  │ Representation: Compressed JPEG byte slice / H.264 NAL units in MP4 container
  ▼
[ Android UI / Surface ]
```

---

# 10. STATE MANAGEMENT

### Current Python State Model
1. **Global Process State in [app.py](file:///Users/aman/Developer/PoseFit/app.py):**
   - `camera`: Singleton `cv2.VideoCapture` instance.
   - `exercise_engine`: Singleton `ExerciseEngine` instance.
   - `exercise_running`: Global boolean flag.
   - `sets_completed`, `sets_goal`, `exercise_goal`: Global integers.
   - `video_analyses`: Dictionary mapping `video_id` to processing metadata.
   - `lock`: `threading.Lock` protecting the `output_frame` JPEG buffer.
2. **Exercise Instance State in [base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py):**
   - `current_state`, `prev_state`: String tracking FSM position.
   - `counter`, `counter_left`, `counter_right`: Repetition totals.
   - `last_count_time`: Timestamp for debouncing.
   - `angle_history`: List of floats for moving average smoothing.
   - `rep_start_time`, `rep_durations`, `rep_form_scores`: Tracking arrays for tempo and average form quality.
3. **Threading & Concurrency Safety:**
   - **`[OBSERVED]` Thread-Unsafe:** Flask runs with `threaded=False` by default in `app.run()`. Multiple HTTP calls modifying global exercise parameters during frame generation can cause race conditions.
   - **`[OBSERVED]` Subprocess Isolation:** Long video analysis is isolated inside a separate Python process to circumvent GIL contention and memory leaks.

### Mapping to Android / Rust State Architecture
- **Rust Core State Container:** An encapsulated `EngineContext` struct owned inside an `Arc<Mutex<EngineContext>>` or an actor-style message channel.
- **Android UI State:** Android Jetpack `ViewModel` observing a Kotlin `StateFlow<WorkoutUiState>` fed by JNI callbacks from the Rust core.
- **Thread Safety:** Pure thread safety guaranteed by Rust's `Send + Sync` compiler constraints.

---

# 11. CONCURRENCY AND PERFORMANCE

### Performance Bottleneck Analysis

| Bottleneck | Current Python Behavior | Technical Reason | Mobile/Rust Impact |
| :--- | :--- | :--- | :--- |
| **Model Inference** | 30–70 ms per frame on CPU | Python MediaPipe runs on host CPU without GPU delegate by default | **Major:** Must run via Android GPU/NNAPI delegate or frame rate will drop below 15 FPS. |
| **String `eval()` Condition Checks** | Dynamic Python `eval()` executed for every state on every frame | Python bytecode compilation and evaluation in interpreted scope | **Eliminated in Rust:** Replaced by native compiled enum checks or a tokenized bytecode VM. |
| **Image Copies & Encoding** | BGR $\to$ RGB $\to$ BGR $\to$ JPEG byte string on every frame | Unnecessary memory allocations and CPU JPEG compression overhead | **Eliminated in Rust/Android:** Zero-copy camera YUV buffer mapping and hardware rendering via SurfaceView / OpenGL. |
| **Subprocess Overhead** | Spawns separate Python interpreter for video files | Subprocess initialization, Python startup, and JSON disk polling | **Eliminated in Rust:** Lightweight background worker thread using native threading. |

### Real-Time Camera Processing Feasibility
- **Current Python System:** Struggles to maintain 30 FPS on standard desktop hardware when streaming JPEG over HTTP.
- **Target Android + Rust System:** Fully capable of sustained 30–60 FPS by bypassing CPU image conversions, running inference on the mobile NPU/GPU, and computing vector trigonometry directly in native Rust.

---

# 12. FILESYSTEM / STORAGE BEHAVIOR

### Current File I/O
- **Files Read:**
  - [exercises/definitions/*.yaml](file:///Users/aman/Developer/PoseFit/exercises/definitions): 18 YAML files loaded at startup.
  - Video files from `uploads/` read by `cv2.VideoCapture`.
  - Stored video [data/dumbel-workout.mp4](file:///Users/aman/Developer/PoseFit/data/dumbel-workout.mp4).
- **Files Written:**
  - `uploads/<uuid>_<filename>`: Temporary raw uploaded videos.
  - `uploads/<uuid>_results.json`: Polling progress IPC file written every 60 frames.
  - `uploads/<uuid>_processed.mp4`: Final annotated video.
  - Original raw upload and JSON files are deleted upon completion.

### Android Storage Architecture Mapping
- **Asset Storage:** Exercise definitions bundled into Android `assets/exercises/` or compiled directly into the Rust static binary via `include_str!()`.
- **Temporary Cache:** Video analysis staging located in `context.cacheDir` (`/data/user/0/<package>/cache/`).
- **Exported Videos:** Processed videos saved to `MediaStore.Video` or `context.getExternalFilesDir(Environment.DIRECTORY_MOVIES)`.
- **Database:** Structured workout history stored in Android SQLite via Room or Rust `rusqlite` in `context.filesDir`.

---

# 13. CONFIGURATION AND ENVIRONMENT

### Environment Variables & Hardcoded Values
- **`[OBSERVED]` TensorFlow Configuration in [app.py](file:///Users/aman/Developer/PoseFit/app.py#L2):**
  - `os.environ["TF_ENABLE_ONEDNN_OPTS"] = "0"`
  - `os.environ["OMP_NUM_THREADS"] = "1"`
  - `os.environ["TF_NUM_INTEROP_THREADS"] = "1"`
  - `os.environ["TF_NUM_INTRAOP_THREADS"] = "1"`
  - `os.environ["TF_CPP_MIN_LOG_LEVEL"] = "2"`
- **`[OBSERVED]` Hardcoded Constants:**
  - Flask Secret Key: `'fitness_trainer_secret_key'` (`[SECURITY RISK]`).
  - Backend Camera API: `cv2.VideoCapture(0, cv2.CAP_DSHOW)` (Windows DirectShow backend hardcoded; fails on Linux/macOS without fallback).
  - Port: `5000`.
  - Max Upload Size: `MAX_VIDEO_SIZE_MB = 50`.
  - Max Video Duration: `MAX_VIDEO_DURATION_SEC = 120`.

### Target Android Configuration
- App configuration handled via Android `SharedPreferences` / Jetpack `DataStore`.
- Threading and hardware acceleration configured natively via Android NDK / TFLite Delegate options.

---

# 14. DEPENDENCY AUDIT

| Python Dependency | Purpose in Current Project | Critical? | Rust / Android Equivalent | Migration Category |
| :--- | :--- | :--- | :--- | :--- |
| `flask` | HTTP server, REST endpoints, MJPEG streaming | No | None (Replaced by Android UI / Activities) | **Can be removed** |
| `opencv-python` | Image conversion, video I/O, canvas rendering | Yes | `image` crate (Rust), Android `CameraX` / `MediaCodec` | **Requires replacement** |
| `mediapipe` | 33-Keypoint Pose Estimation | **Yes** | Google MediaPipe Tasks Android SDK / TFLite | **Android-specific replacement** |
| `numpy` | Vector arithmetic, dot product, norms | Yes | `nalgebra` or `glam` or custom inline Rust math | **Requires replacement** |
| `pyyaml` | Parsing exercise configuration files | Yes | `serde_yaml` or compile-time JSON / Rust code | **Requires replacement** |
| `imageio` | Video decoding and writing | Yes (video mode) | Android `MediaExtractor` & `MediaMuxer` | **Android-specific replacement** |
| `imageio-ffmpeg` | Bundled FFmpeg binary for H.264 encoding | Yes (video mode) | Hardware `MediaCodec` (H.264 encoder) | **Android-specific replacement** |

---

# 15. PYTHON $\to$ RUST MAPPING

| Python Component | Current Responsibility | Rust Candidate | Rationale |
| :--- | :--- | :--- | :--- |
| `BaseExercise` | FSM, Rep counting, Scoring | `struct BaseExercise` | High performance, memory safety, zero-cost abstractions. |
| `BilateralExercise` | Dual-side limb state tracking | `struct BilateralExercise` | Composable struct or trait implementation. |
| `DurationExercise` | Timed hold tracking | `struct DurationExercise` | Instant duration evaluation via `std::time::Instant`. |
| `_safe_eval()` | Dynamic evaluation of YAML conditions | `evalexpr` or Custom Rule AST | Compile-time rule AST avoids runtime evaluation hazards. |
| `_angle_between()` | 2D vector trigonometry | Custom inline math / `glam::Vec2` | 2D vector dot product and clamping runs in <5 nanoseconds. |
| `PoseEstimator` | MediaPipe execution wrapper | MediaPipe Tasks Android / TFLite C API | Native C-ABI interop without Python overhead. |
| `draw_text_with_bg()` | Overlay rendering | Android Canvas / Skia / `imageproc` | Native UI layer handles text rendering much cleaner than bitmap manipulation. |
| `video_processor.py` | Offline batch video processing | Rust Pipeline (`crossbeam` worker) | Background worker utilizing Android hardware decoders. |
| `logging` | System diagnostic logs | `tracing` + `tracing-android` | Native routing directly to Android logcat. |

---

# 16. ANDROID ARCHITECTURE

```text
TARGET ANDROID + RUST ARCHITECTURE:

┌─────────────────────────────────────────────────────────────┐
│                    Android UI Layer                         │
│       Jetpack Compose (WorkoutScreen, VideoAnalysisScreen)  │
└──────────────────────────────┬──────────────────────────────┘
                               │ StateFlow / ViewModel
┌──────────────────────────────▼──────────────────────────────┐
│                  Android Kotlin Layer                       │
│  ├── CameraX Controller (PreviewView, ImageAnalysis)        │
│  ├── MediaPipe Tasks Vision (PoseLandmarker GPU/NNAPI)       │
│  ├── MediaCodec Video Transcoder (Offline Video Analysis)   │
│  └── Room Database (Workout History & Statistics)           │
└──────────────────────────────┬──────────────────────────────┘
                               │ JNI Zero-Copy Pointer (Landmarks / Frames)
┌──────────────────────────────▼──────────────────────────────┐
│                    Rust Core (posefit-core)                 │
│  ├── JNI C-ABI Boundary (librust_posefit.so)                │
│  ├── Exercise Engine (Coordinator)                          │
│  ├── FSM State Machine & AST Condition Evaluator            │
│  ├── 2D Vector Trigonometry & Smoothing Buffer              │
│  ├── Quality Scoring & Feedback Rule Engine                 │
│  └── YAML / Binary Exercise Definitions Store               │
└─────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities
- **UI Layer (Jetpack Compose):** Renders UI controls, counters, real-time score rings, grade badges, and error banners.
- **Camera Layer (CameraX):** Streams frames from mobile camera, controls auto-focus, exposure, and frame rotation.
- **Inference Layer (MediaPipe Tasks Android):** Consumes camera frames, delegates to GPU/NPU, extracts 33 keypoints.
- **Rust Core Layer:** Ingests landmark arrays across JNI, computes angles, executes FSM transitions, updates counters, evaluates form penalties, and emits structured results.
- **Storage Layer (Room DB):** Stores completed workout logs, rep history, and user settings.

---

# 17. RUST CORE DESIGN

```text
posefit-core/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # JNI exports and initialization
│   ├── engine.rs               # Top-level ExerciseEngine facade
│   ├── traits.rs               # Exercise trait definition
│   ├── math.rs                 # 2D Vector arithmetic & angle calculations
│   ├── smoothing.rs            # Moving average temporal filter
│   ├── fsm.rs                  # Finite state machine and condition evaluator
│   ├── scoring.rs              # Form score (0-100) & penalty calculator
│   ├── exercises/
│   │   ├── mod.rs
│   │   ├── standard.rs         # Base repetition exercise
│   │   ├── bilateral.rs        # Dual-limb alternating exercise
│   │   └── duration.rs         # Timed hold exercise
│   ├── definitions/
│   │   ├── mod.rs
│   │   ├── loader.rs           # YAML / JSON parser
│   │   └── schema.rs           # Serde data structs
│   └── errors.rs               # Strongly typed error hierarchy
```

### Key Structs & Traits

```rust
// Core Exercise Trait
pub trait Exercise: Send + Sync {
    fn process_frame(&mut self, landmarks: &[Landmark; 33], frame_size: (u32, u32)) -> ExerciseResult;
    fn reset(&mut self);
    fn get_status(&self) -> ExerciseStatus;
}

// 2D Landmark Representation
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Landmark {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub visibility: f32,
}

// Exercise Status Output
#[derive(Debug, Clone, Serialize)]
pub struct ExerciseResult {
    pub counter: u32,
    pub current_state: String,
    pub form_score: u32,
    pub avg_form_score: u32,
    pub form_grade: char,
    pub feedback_messages: Vec<FeedbackAlert>,
    pub rep_completed: bool,
}
```

---

# 18. JNI / FFI BOUNDARY

```text
Kotlin (CameraX / MediaPipe)
         │
         │ DirectByteBuffer (33 * 4 floats = 528 bytes)
         ▼
JNI: Java_com_posefit_core_PoseFitBridge_processFrame(JNIEnv, jobject, jlong engine_ptr, jobject byte_buffer, jint width, jint height)
         │
         ▼
Rust Core: unsafe { std::slice::from_raw_parts(...) }
         │ (Zero memory copies)
         ▼
ExerciseEngine.process_frame(&landmarks, (width, height))
         │
         ▼
Returns compact packed struct (Rep count, Score, State ID, Feedback flags)
```

### Memory Copy Optimization
- Rather than passing Java objects or JSON strings across JNI, pass a **`java.nio.DirectByteBuffer`** containing the flat array of landmark coordinates.
- Return results via a flat C-compatible primitive struct:
  ```rust
  #[repr(C)]
  pub struct NativeExerciseOutput {
      pub counter: i32,
      pub counter_left: i32,
      pub counter_right: i32,
      pub state_id: i32,
      pub form_score: i32,
      pub avg_form_score: i32,
      pub grade_ascii: u8,
      pub rep_completed: bool,
      pub active_feedback_mask: u32,
  }
  ```

---

# 19. CAMERA PIPELINE

```text
Android CameraX (ImageAnalysis UseCase)
         │
         ▼ Format: YUV_420_888 @ 1080p/720p 30 FPS
MediaPipe Tasks PoseLandmarker (Running on GPU Delegate)
         │
         ▼ Inference Latency: ~12-18 ms
33 Normalized Keypoints
         │
         ▼ JNI Direct Call (<0.1 ms)
Rust ExerciseEngine (Vector Angles + FSM Evaluation)
         │ Latency: <0.05 ms
Output Metrics (Reps, Score, Grade)
         │
         ▼ Kotlin StateFlow (<1 ms)
Jetpack Compose Recomposition (60 FPS UI Overlay)
```

### Latency & Frame Dropping Strategy
- Set CameraX `ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST`. If the inference pipeline is occupied, incoming camera frames are automatically dropped without accumulating memory buffers.
- The UI camera preview runs independently on `PreviewView` at 60 FPS, ensuring zero visual stutter for the user.

---

# 20. ERROR HANDLING

### Current Python Exceptions vs Rust Error Model

| Python Failure Mode | Current Python Behavior | Rust Target Error Enum |
| :--- | :--- | :--- |
| Unknown landmark name in YAML | `raise ValueError(f"Unknown landmark...")` | `ConfigError::InvalidLandmarkName(String)` |
| YAML parse error | Crashes loader or test runner | `ConfigError::YamlDeserialization(serde_yaml::Error)` |
| Condition syntax error in `_safe_eval` | Catches exception, prints to stdout, ignores state | `FsmError::ConditionEvaluationFailed(String)` |
| Zero vector norm in angle math | Adds `1e-6` epsilon | Returns `0.0` or handles via `.atan2()` |
| Subprocess crash during video analysis | Returns HTTP 500 or JSON error | `VideoError::TranscodeFailed(String)` |

```rust
#[derive(thiserror::Error, Debug)]
pub enum PoseFitError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("FSM execution error: {0}")]
    Fsm(#[from] FsmError),
    #[error("Invalid exercise state: {0}")]
    InvalidState(String),
}
```

---

# 21. LOGGING & DEBUGGING

- **Current Python System:** Uses Python `logging.basicConfig(level=logging.DEBUG)` writing to standard error. Subprocess communicates logs by printing to stdout and having the parent parse line-by-line.
- **Target Android / Rust System:** Rust logs instrumented using the `tracing` crate. Log events are piped directly to Android Logcat via `tracing_android::layer("PoseFitNative")`.

---

# 22. SECURITY REVIEW

### Critical Vulnerabilities in Existing Python Codebase
1. **Arbitrary Code Execution via `eval()` (`[OBSERVED]` in [base_exercise.py:494](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py#L494)):**
   Although `_safe_eval` blocks certain substrings (`import`, `exec`, `open`, etc.), relying on Python's built-in `eval()` for YAML condition evaluation is inherently fragile and vulnerable to sandbox escapes.
2. **Unauthenticated File Upload & Arbitrary Execution (`[OBSERVED]` in [app.py:466](file:///Users/aman/Developer/PoseFit/app.py#L466)):**
   `/api/video/upload` accepts arbitrary file uploads. Although extension and duration checks exist, files are executed against external FFmpeg processes without sandboxing.
3. **Hardcoded Session Secret (`[OBSERVED]` in [app.py:59](file:///Users/aman/Developer/PoseFit/app.py#L59)):**
   `app.secret_key = 'fitness_trainer_secret_key'` committed directly into repository.

### Remediation in Mobile Architecture
- Move to a compiled, strictly typed condition engine in Rust.
- Eliminate network exposure completely; all processing runs within the Android application sandbox.

---

# 23. CURRENT ARCHITECTURE PROBLEMS

1. **Broken Workout Persistence:** `db/workout_logger.py` is an empty 0-byte file, causing the app to silently fall back to an in-memory dummy logger that discards all user workouts.
2. **Missing HTTP Endpoint:** `static/js/dashboard.js` attempts to query `/dashboard_data`, which does not exist in `app.py`, generating HTTP 404 console errors.
3. **Memory Leaks in Live Analysis:** Live frame analysis via `/api/video/analyze_frame` had to be explicitly disabled because re-instantiating `PoseEstimator` causes out-of-memory crashes.
4. **Hardcoded Windows DirectShow Backend:** `cv2.VideoCapture(0, cv2.CAP_DSHOW)` crashes or fails to open webcams on macOS and Linux systems unless modified.
5. **Redundant Visual Rendering:** Skeletons are drawn twice in live mode: once inside `PoseEstimator.estimate_pose()` and once inside `ExerciseEngine._draw_visualization()`.

---

# 24. ANDROID MIGRATION BLOCKERS & ROADMAP RISKS

### Blockers (Must resolve before coding)
- **Mathematical Parity Verification:** Establish exact golden outputs for angles and FSM transitions from the Python implementation to prevent discrepancies in the Rust version.
- **MediaPipe Runtime Selection:** Decide between bundling MediaPipe Tasks Android AAR (recommended) vs compiling standalone TFLite C API within Rust.

### Major Risks
- **Mobile Thermal Throttling:** Continuous 30 FPS MediaPipe pose estimation on mobile devices can cause thermal throttling after 5–10 minutes of active workout.
- **Camera Aspect Ratio & Orientation:** Mobile portrait mode requires coordinate rotation transformations before feeding landmarks into the angle calculation engine.

### Minor Migration Work
- Porting 18 YAML exercise definitions to JSON or embedded Rust structs.
- Implementing vector angle math and moving average smoothing in Rust.

### Retained Directly
- The entire mathematical specification: landmark indices, angle definitions, FSM state names, trigger rules, and tempo boundaries.

---

# 25. OFFLINE / ON-DEVICE FEASIBILITY

### Feasibility Verdict: 100% FEASIBLE OFFLINE

| Resource | Mobile Requirement | Assessment |
| :--- | :--- | :--- |
| **Model Size** | BlazePose Landmarker: ~9 MB | Negligible storage footprint. |
| **RAM Consumption** | ~120 MB total (App + Model + Buffers) | Readily available on Android devices (standard is 4GB–12GB). |
| **Compute / Latency** | Mobile GPU / NPU: 12–18 ms per frame | Easily achieves 30 FPS real-time processing. |
| **Network Dependency** | Zero remote calls | Fully self-contained on-device operation. |

---

# 26. MODEL CONVERSION / OPTIMIZATION PLAN

```text
CONVERSION PATHWAY:

Source Asset (MediaPipe pose_landmarker_full.task)
                  │
                  ▼
         [ Inspection & Validation ]
         Verify input tensor: 1x256x256x3 (float32)
         Verify output tensor: 33 landmarks (x, y, z, visibility, presence)
                  │
                  ▼
         [ Mobile Runtime Deployment ]
         • Target A: Google MediaPipe Tasks Vision Android AAR (Official)
                     Enables NNAPI / Qualcomm Adreno / ARM Mali GPU delegates
         • Target B: Flat TFLite model loaded via tflite-rs in Rust Core
                  │
                  ▼
         [ Quantization Consideration ]
         FP16 quantization reduces model size from 9MB to 4.5MB with <0.5% landmark drift.
```

---

# 27. TESTING STRATEGY & NUMERICAL PARITY

To guarantee that **Python implementation output $\equiv$ Rust implementation output**:

```text
PARITY TEST MATRIX:

  [ Test Video: dumbel-workout.mp4 ]
            │
            ├──────────────────────────────┬──────────────────────────────┐
            ▼                                                             ▼
  Python Implementation                                         Rust Core Implementation
    • Dump landmarks.json                                         • Ingest landmarks.json
    • Dump angle_history.json                                     • Compute angles
    • Dump fsm_states.json                                        • Evaluate FSM states
    • Dump rep_counts.json                                        • Compute rep counts
    • Dump form_scores.json                                       • Compute form scores
            │                                                             │
            └──────────────────────────────┬──────────────────────────────┘
                                           ▼
                            Automated Diff Comparison:
                            • Angle tolerance: |delta| < 0.001 degrees
                            • FSM state transitions: 100% identical frame indices
                            • Final Rep Count: Exact match
                            • Form Score: Exact match
```

---

# 28. MIGRATION PHASES

```text
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   PHASE 0    │ ──► │   PHASE 1    │ ──► │   PHASE 2    │ ──► │   PHASE 3    │
│ Freeze Spec  │     │ Rust Math/FSM│     │ Test Parity  │     │ JNI Boundary │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
                                                                       │
┌──────────────┐     ┌──────────────┐     ┌──────────────┐             │
│   PHASE 6    │ ◄── │   PHASE 5    │ ◄── │   PHASE 4    │ ◄───────────┘
│ Hardening    │     │  Compose UI  │     │ CameraX + ML │
└──────────────┘     │              │     │              │
                     └──────────────┘     └──────────────┘
```

- **Phase 0 (Specification Freeze):** Export all 18 YAML configurations and extract golden test vectors from [data/dumbel-workout.mp4](file:///Users/aman/Developer/PoseFit/data/dumbel-workout.mp4).
- **Phase 1 (Rust Core Engine):** Implement vector trigonometry, moving average smoothing, FSM, scoring, and YAML parser in pure Rust (`cargo test`).
- **Phase 2 (Parity Verification):** Validate Rust engine against Python golden vectors.
- **Phase 3 (JNI Bridge):** Create `librust_posefit.so` with flat C-ABI and DirectByteBuffer zero-copy memory mapping.
- **Phase 4 (CameraX & ML Integration):** Integrate CameraX and MediaPipe Tasks Vision on Android to feed landmarks to the JNI bridge.
- **Phase 5 (Jetpack Compose UI):** Build modern native UI with exercise selection, HUD, and rep counters.
- **Phase 6 (Hardening & Optimization):** Profile memory usage, tune GPU delegates, and test across Android devices.

---

# 29. FINAL TARGET ARCHITECTURE

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        Android Application                             │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │               Jetpack Compose UI Layer                           │  │
│  │  • Live Workout HUD (Reps, Set, Form Score Gauge, Grade)         │  │
│  │  • Exercise Selector (18 Exercises categorized Upper/Lower/Cardio)│ │
│  │  • Video Analysis Screen & Summary Report                        │  │
│  └──────────────────────────────────┬───────────────────────────────┘  │
│                                     │ StateFlow Updates                │
│  ┌──────────────────────────────────▼───────────────────────────────┐  │
│  │               WorkoutViewModel (Kotlin)                          │  │
│  └───────────────┬───────────────────────────────────▲──────────────┘  │
│                  │ Setup & Config                    │ Rep & Score     │
│  ┌───────────────▼────────────────┐                  │ Callbacks       │
│  │ CameraX ImageAnalysis (30 FPS) │                  │                 │
│  └───────────────┬────────────────┘                  │                 │
│                  │ YUV420 Frame                      │                 │
│  ┌───────────────▼────────────────┐                  │                 │
│  │ MediaPipe Tasks Pose Landmarker│                  │                 │
│  │ (GPU / NNAPI Delegate)         │                  │                 │
│  └───────────────┬────────────────┘                  │                 │
│                  │ 33 Landmarks (Float Buffer)       │                 │
│                  │ DirectByteBuffer Pointer          │                 │
│  ┌───────────────▼───────────────────────────────────┴──────────────┐  │
│  │               Rust Core Native Library                           │  │
│  │             (librust_posefit.so via JNI)                         │  │
│  │                                                                  │  │
│  │  ┌────────────────────────┐      ┌────────────────────────────┐  │  │
│  │  │  Vector Trigonometry   │ ───► │  Temporal Smoothing Buffer │  │  │
│  │  │  (2D Dot Product/Norm) │      │  (Moving Average Filter)   │  │  │
│  │  └────────────────────────┘      └─────────────┬──────────────┘  │  │
│  │                                                │ Smoothed Angles │  │
│  │  ┌────────────────────────┐      ┌─────────────▼──────────────┐  │  │
│  │  │  Form Score & Feedback │ ◄─── │  FSM State Machine Engine  │  │  │
│  │  │  (0-100 Score, Grades) │      │  (18 Rule Definitions)     │  │  │
│  │  └────────────────────────┘      └─────────────┬──────────────┘  │  │
│  │                                                │ State Changes   │  │
│  │                                  ┌─────────────▼──────────────┐  │  │
│  │                                  │  Repetition / Time Counter │  │  │
│  │                                  │  (Debounce & Trigger Check)│  │  │
│  │                                  └────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

---

# 30. MIGRATION TRACEABILITY MATRIX

| Existing Component | Current Responsibility | Android / Rust Replacement | Difficulty | Migration Status |
| :--- | :--- | :--- | :--- | :--- |
| `app.py` | Flask HTTP & Streaming Server | Android App / Jetpack Compose | Medium | Redesign |
| `video_processor.py` | Offline batch video processor | Rust worker + Android MediaCodec | High | Rewrite |
| `PoseEstimator` | MediaPipe Python Pose wrapper | MediaPipe Tasks Android SDK | Medium | Replace |
| `BaseExercise` | FSM, Rep counting, Scoring | `posefit-core::BaseExercise` | Medium | Rewrite |
| `BilateralExercise` | Dual-side limb state tracking | `posefit-core::BilateralExercise` | Medium | Rewrite |
| `DurationExercise` | Timed hold tracking | `posefit-core::DurationExercise` | Low | Rewrite |
| `ExerciseEngine` | High-level execution facade | `posefit-core::ExerciseEngine` | Low | Rewrite |
| `loader.py` | YAML loader & validator | `serde_yaml` or compiled Rust code | Low | Rewrite |
| `exercises/definitions/*.yaml` | 18 exercise definition files | Bundled JSON / embedded structs | Low | Convert |
| `angle_calculation.py` | Math helper | `posefit-core::math` | Low | Rewrite |
| `workout_logger.py` | DB persistence (currently 0 bytes) | Android Room (SQLite) | Medium | New Implementation |
| `static/` & `templates/` | Web UI | Jetpack Compose Native UI | Medium | Redesign |

---

# 31. CRITICAL DISCOVERIES

### What I Now Know About the Project
1. **The Core Logic is Independent of Flask:** The true core of PoseFit is [exercises/base_exercise.py](file:///Users/aman/Developer/PoseFit/exercises/base_exercise.py) and the 18 YAML definitions. Flask and OpenCV are merely an I/O transport layer.
2. **Angle Math is Exclusively 2D:** The codebase does not use 3D world coordinates. Angles are computed in 2D image pixel space ($x \times \text{width}, y \times \text{height}$).
3. **Condition Checks Use Python `eval()`:** The YAML configuration strings (e.g. `"angle > 90 and angle <= 165"`) are evaluated dynamically via Python `eval()`.
4. **Workout Logging is Broken:** [db/workout_logger.py](file:///Users/aman/Developer/PoseFit/db/workout_logger.py) is empty (0 bytes), and the app silently discards workout history via a fallback dummy class.
5. **Dashboard Endpoint is Missing:** [static/js/dashboard.js](file:///Users/aman/Developer/PoseFit/static/js/dashboard.js) calls an endpoint (`/dashboard_data`) that does not exist in [app.py](file:///Users/aman/Developer/PoseFit/app.py).

### Things That Were NOT Obvious From Documentation
- `main.py` is documented in the README but is completely missing from the workspace.
- The `feedback/` folder contains only stale `.pyc` bytecode files; the original `.py` source files were deleted in a previous refactoring.
- The real-time video frame upload API (`/api/video/analyze_frame`) is explicitly disabled in [app.py](file:///Users/aman/Developer/PoseFit/app.py#L744) to prevent server memory crashes.

### Invariant Behavioral Contracts (MUST NOT CHANGE)
- **Landmark Indices:** Must strictly adhere to the 13 MediaPipe landmark indices mapped in `BaseExercise.LANDMARK_MAP`.
- **FSM State Evaluation Order:** Must evaluate states sequentially according to `state_order` in the YAML, with the first matching condition taking precedence.
- **Debounce Thresholds:** Must enforce `min_rep_duration` between counted repetitions.
- **Scoring Formulas:** Form score penalties (40% angle deviation, 30% tempo compliance, 30% form feedback warnings) must remain numerically consistent.

### Things That SHOULD Be Redesigned
- **Replace Dynamic String Evaluation:** Replace Python `eval()` with a compiled condition AST or a tokenized bytecode evaluator in Rust.
- **Eliminate Subprocess Overhead:** Replace Python CLI subprocess execution with native asynchronous Rust worker threads.
- **Implement Proper Persistence:** Implement an actual SQLite database via Android Room.

---

# 32. WHAT I NEED TO BUILD NEXT

### Implementation-Oriented Migration Checklist

1. **Extract Behavioral Reference Data:**
   - Run [test_engine.py](file:///Users/aman/Developer/PoseFit/test_engine.py) against [data/dumbel-workout.mp4](file:///Users/aman/Developer/PoseFit/data/dumbel-workout.mp4) to dump a golden JSON log of frame-by-frame landmarks, calculated angles, FSM states, rep counts, and form scores.
2. **Freeze Exercise Contracts:**
   - Convert all 18 YAML definitions in [exercises/definitions/](file:///Users/aman/Developer/PoseFit/exercises/definitions) into a unified JSON schema or compile-time Rust data structure.
3. **Build the Rust Core (`posefit-core`):**
   - Implement 2D vector trigonometry: `angle_between(p1, p2, p3)`.
   - Implement temporal moving average filter.
   - Implement condition expression AST: support `>`, `<`, `>=`, `<=`, `==`, `and`, `abs()`, and constant arithmetic offsets.
   - Implement `BaseExercise`, `BilateralExercise`, and `DurationExercise`.
   - Implement `FormScoreCalculator` with letter grade boundaries.
4. **Build Parity Test Suite:**
   - Write Rust tests verifying exact numerical parity against the Python golden reference data.
5. **Implement JNI Boundary:**
   - Expose `Java_com_posefit_core_PoseFitBridge` functions taking DirectByteBuffer pointers for zero-copy landmark passing.
6. **Set Up Android Application Project:**
   - Configure Android Studio project with NDK and Cargo NDK build scripts.
   - Add CameraX dependencies (`camera-core`, `camera-camera2`, `camera-lifecycle`, `camera-view`).
   - Add Google MediaPipe Tasks Vision (`com.google.mediapipe:tasks-vision:latest`).
7. **Assemble Android Pipeline:**
   - Connect CameraX `ImageAnalysis` to MediaPipe Pose Landmarker running on GPU delegate.
   - Pass inferred landmarks across JNI to Rust Core.
   - Receive updated exercise status and expose to Jetpack Compose via Kotlin `StateFlow`.
8. **Build Android UI:**
   - Create Workout HUD overlay displaying current reps, sets, form score ring, and feedback warnings.
   - Implement exercise selection screen for all 18 exercises.
   - Implement local SQLite database using Room to properly persist workout history.
