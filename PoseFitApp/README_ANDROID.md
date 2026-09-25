# POSEFIT — PURE RUST ANDROID ARCHITECTURE & DEPLOYMENT GUIDE

> **100% Rust. Zero Kotlin. Zero Java. Zero Jetpack Compose. Zero Gradle.**

---

## 1. Architectural Overview

```text
┌────────────────────────────────────────────────────────────────────────┐
│ Android OS (Linux Kernel 5.x / 6.x + SurfaceFlinger + HAL)             │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Launches NativeActivity directly
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│               libposefit_android.so (100% Compiled Rust)               │
│                                                                        │
│  #[no_mangle]                                                          │
│  fn android_main(app: AndroidApp) {                                    │
│      ├── 1. Pure Rust Native Event Loop (android-activity)             │
│      ├── 2. Direct Camera Ingestion (camera2-ndk / AImageReader)       │
│      ├── 3. On-Device Neural Network Inference (BlazePoseEstimator)    │
│      │      ├─ Letterbox Preprocessing & Normalization                 │
│      │      └─ 33 3D Landmark Coordinate Tensor Inversion              │
│      ├── 4. Exercise State Machine & Counting Engine (PoseFitEngine)   │
│      │      ├─ 2D Cosine Angle Math & Temporal Smoothing Filters       │
│      │      ├─ Safe AST Evaluator (60 verified YAML conditions)        │
│      │      ├─ Repetition Debounce & Bilateral Isolation               │
│      │      └─ Form Scoring (0-100) & Real-Time Feedback Rules         │
│      └── 5. On-Device HUD & Skeleton Overlay (HudRenderer)             │
│             ├─ Anti-aliased bone vectors & colored joint dots          │
│             ├─ Real-time HUD banner, rep counts, and grade pill        │
│             └─ Zero-dependency embedded bitmap typography (8x16 font)  │
│  }                                                                     │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Zero-Java / Zero-Kotlin Enforcement

1. **`android:hasCode="false"`**:
   Configured in [`AndroidManifest.xml`](file:///Users/aman/Developer/PoseFit/PoseFitApp/posefit-android/AndroidManifest.xml), instructing the Android runtime that this application package contains **zero Java bytecode or classes.dex**.
2. **Direct OS Native Entrypoint**:
   The application entrypoint is:
   ```rust
   #[no_mangle]
   fn android_main(app: AndroidApp)
   ```
   invoked directly by the Android platform's `android.app.NativeActivity` C-runtime.
3. **No JVM Garbage Collection or JNI Overhead**:
   Every frame is processed in native memory without cross-boundary JNI object allocation or bridge overhead.

---

## 3. Workspace Layout

| Crate | Role |
| :--- | :--- |
| [`posefit-core`](file:///Users/aman/Developer/PoseFit/PoseFitApp/posefit-core) | Core fitness engine: 18 exercises, AST condition parser, vector math, temporal smoother, form scoring, BlazePose pre/postprocessing, and pure-Rust HUD renderer. |
| [`posefit-android`](file:///Users/aman/Developer/PoseFit/PoseFitApp/posefit-android) | NativeActivity entrypoint, native event pump, Android logcat integration, and raw frame dispatch. |
| [`posefit-app`](file:///Users/aman/Developer/PoseFit/PoseFitApp/posefit-app) | Desktop/CLI simulation and video annotation streaming binary. |

---

## 4. Building and Deploying to Android

### Prerequisites
1. **Rust target for Android**:
   ```bash
   rustup target add aarch64-linux-android
   ```
2. **Android NDK**:
   Set `ANDROID_NDK_HOME` to your Android NDK installation (e.g., `~/Library/Android/sdk/ndk/<version>`).
3. **cargo-ndk** (Recommended):
   ```bash
   cargo install cargo-ndk
   ```

### One-Command APK Build
```bash
./build-apk.sh aarch64-linux-android
```

### Install and Run on Device via ADB
```bash
adb install -r target/PoseFit-release.apk
adb shell am start -n com.posefit.app/android.app.NativeActivity
```

### Stream Live Android Logcat
```bash
adb logcat -s PoseFitRust:V
```
