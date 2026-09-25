#!/usr/bin/env python3
"""
PoseFit Video Annotation Tool using Rust Core Application Logic.

1. Reads frames from input video.
2. Extracts MediaPipe 33-point pose landmarks.
3. Streams landmarks into the compiled Rust `posefit-app` binary (`process-stream`).
4. Receives deterministic state, bilateral rep counters, angles, form score, and feedback from Rust.
5. Renders high-visibility fitness telemetry overlay on every frame.
6. Encodes and muxes original audio into `/Users/aman/Developer/PoseFit/data/annotage_dumbel-workout.mp4`.
"""

import os
import sys
import json
import subprocess
import cv2
import mediapipe as mp
import numpy as np

RUST_BIN = "/Users/aman/Developer/PoseFit/PoseFitApp/target/release/posefit-app"
INPUT_VIDEO = "/Users/aman/Developer/PoseFit/data/dumbel-workout.mp4"
OUTPUT_VIDEO = "/Users/aman/Developer/PoseFit/data/annotage_dumbel-workout.mp4"
TEMP_NO_AUDIO = "/Users/aman/Developer/PoseFit/data/temp_annotated_no_audio.mp4"

# Color constants (BGR)
COLOR_BG_DARK = (20, 20, 24)
COLOR_TEXT_WHITE = (255, 255, 255)
COLOR_ACCENT_CYAN = (240, 200, 0)
COLOR_ACCENT_GREEN = (50, 220, 50)
COLOR_ACCENT_YELLOW = (0, 215, 255)
COLOR_ACCENT_ORANGE = (0, 140, 255)
COLOR_ACCENT_RED = (50, 50, 240)
COLOR_ARM_LEFT = (255, 160, 50)   # Blueish/Orange BGR
COLOR_ARM_RIGHT = (50, 200, 255)  # Cyan BGR

# MediaPipe landmark connections for arms and upper body
BODY_CONNECTIONS = [
    (11, 12), # shoulders
    (11, 13), (13, 15), # left arm
    (12, 14), (14, 16), # right arm
    (11, 23), (12, 24), # torso
    (23, 24), # hips
    (23, 25), (25, 27), # left leg
    (24, 26), (26, 28)  # right leg
]


def get_grade_color(grade: str):
    if grade == 'A':
        return COLOR_ACCENT_GREEN
    elif grade == 'B':
        return (0, 230, 200)
    elif grade == 'C':
        return COLOR_ACCENT_YELLOW
    elif grade == 'D':
        return COLOR_ACCENT_ORANGE
    else:
        return COLOR_ACCENT_RED


def draw_hud(frame, telemetry, width, height, rep_flash_timer):
    """Draws sleek fitness overlay with Rust telemetry."""
    overlay = frame.copy()

    # 1. Top HUD Header panel
    panel_h = 130
    cv2.rectangle(overlay, (0, 0), (width, panel_h), (15, 15, 20), -1)
    # Blend overlay with transparency
    cv2.addWeighted(overlay, 0.78, frame, 0.22, 0, frame)

    # Accent top border line
    cv2.line(frame, (0, panel_h), (width, panel_h), (60, 60, 80), 2)

    # Title & Engine badge
    cv2.putText(frame, "POSEFIT CORE", (30, 36), cv2.FONT_HERSHEY_DUPLEX, 0.95, (255, 255, 255), 2, cv2.LINE_AA)
    cv2.putText(frame, "RUST ENGINE v0.1.0", (260, 36), cv2.FONT_HERSHEY_SIMPLEX, 0.55, (180, 220, 255), 1, cv2.LINE_AA)
    cv2.putText(frame, "EXERCISE: HAMMER CURL (BILATERAL)", (30, 65), cv2.FONT_HERSHEY_SIMPLEX, 0.65, (200, 200, 200), 1, cv2.LINE_AA)

    # Rep Counter - Center Box
    center_x = width // 2
    counter_val = telemetry.get("counter", 0)
    counter_text = f"TOTAL REPS: {counter_val}"
    
    rep_color = COLOR_ACCENT_GREEN if rep_flash_timer > 0 else (255, 255, 255)
    cv2.putText(frame, counter_text, (center_x - 140, 52), cv2.FONT_HERSHEY_DUPLEX, 1.25, rep_color, 2, cv2.LINE_AA)

    # Subtext for Total Reps
    rep_sub = f"LEFT: {telemetry.get('counter_left', 0)}  |  RIGHT: {telemetry.get('counter_right', 0)}"
    cv2.putText(frame, rep_sub, (center_x - 120, 85), cv2.FONT_HERSHEY_SIMPLEX, 0.75, (220, 220, 100), 2, cv2.LINE_AA)

    # Left Arm Box
    left_state = (telemetry.get("state_left") or "init").upper()
    left_angle = telemetry.get("angles", {}).get("left", 0.0)
    cv2.putText(frame, f"LEFT ARM: [{left_state}]", (30, 105), cv2.FONT_HERSHEY_SIMPLEX, 0.68, COLOR_ARM_LEFT, 2, cv2.LINE_AA)
    cv2.putText(frame, f"Angle: {left_angle:.1f}°", (220, 105), cv2.FONT_HERSHEY_SIMPLEX, 0.68, (240, 240, 240), 1, cv2.LINE_AA)

    # Right Arm Box
    right_state = (telemetry.get("state_right") or "init").upper()
    right_angle = telemetry.get("angles", {}).get("right", 0.0)
    cv2.putText(frame, f"RIGHT ARM: [{right_state}]", (center_x + 180, 105), cv2.FONT_HERSHEY_SIMPLEX, 0.68, COLOR_ARM_RIGHT, 2, cv2.LINE_AA)
    cv2.putText(frame, f"Angle: {right_angle:.1f}°", (center_x + 400, 105), cv2.FONT_HERSHEY_SIMPLEX, 0.68, (240, 240, 240), 1, cv2.LINE_AA)

    # Form Score Badge (Top Right)
    score_val = telemetry.get("form_score", 100)
    grade_val = telemetry.get("form_grade", "A")
    score_color = get_grade_color(grade_val)

    cv2.rectangle(frame, (width - 250, 20), (width - 30, 80), (30, 30, 40), -1)
    cv2.rectangle(frame, (width - 250, 20), (width - 30, 80), score_color, 2)
    cv2.putText(frame, f"FORM: {score_val}%", (width - 235, 50), cv2.FONT_HERSHEY_DUPLEX, 0.75, (255, 255, 255), 2, cv2.LINE_AA)
    cv2.putText(frame, f"GRADE {grade_val}", (width - 235, 73), cv2.FONT_HERSHEY_SIMPLEX, 0.65, score_color, 2, cv2.LINE_AA)

    # Active Feedback Banner (Bottom)
    alerts = telemetry.get("feedback_alerts", [])
    if alerts:
        msg = alerts[0].get("message", "")
        fb_h = 60
        fb_overlay = frame.copy()
        cv2.rectangle(fb_overlay, (0, height - fb_h), (width, height), (15, 25, 60), -1)
        cv2.addWeighted(fb_overlay, 0.85, frame, 0.15, 0, frame)
        cv2.putText(frame, f"⚠️  FEEDBACK: {msg}", (50, height - 22), cv2.FONT_HERSHEY_DUPLEX, 0.85, (0, 220, 255), 2, cv2.LINE_AA)


def draw_skeleton(frame, landmarks, width, height, angles):
    """Draws MediaPipe body connections and angles calculated by Rust."""
    if not landmarks or len(landmarks) < 33:
        return

    pts = {}
    for i, lm in enumerate(landmarks):
        px = int(lm["x"] * width)
        py = int(lm["y"] * height)
        pts[i] = (px, py)

    # Draw body lines
    for p1_idx, p2_idx in BODY_CONNECTIONS:
        if p1_idx in pts and p2_idx in pts:
            # Color arm lines specifically
            if p1_idx in [11, 13, 15] and p2_idx in [11, 13, 15]:
                color = COLOR_ARM_LEFT
                thick = 5
            elif p1_idx in [12, 14, 16] and p2_idx in [12, 14, 16]:
                color = COLOR_ARM_RIGHT
                thick = 5
            else:
                color = (180, 180, 180)
                thick = 2
            cv2.line(frame, pts[p1_idx], pts[p2_idx], color, thick, cv2.LINE_AA)

    # Draw landmark joints
    for idx, (px, py) in pts.items():
        if idx in [11, 12, 13, 14, 15, 16]: # Arm joints
            radius = 8
            color = (0, 255, 255)
        else:
            radius = 4
            color = (200, 200, 200)
        cv2.circle(frame, (px, py), radius, color, -1, cv2.LINE_AA)
        cv2.circle(frame, (px, py), radius + 2, (0, 0, 0), 1, cv2.LINE_AA)

    # Draw angle tags on elbows
    if 13 in pts: # Left elbow
        left_angle = angles.get("left", 0.0)
        ex, ey = pts[13]
        cv2.putText(frame, f"{left_angle:.0f}°", (ex - 60, ey - 10), cv2.FONT_HERSHEY_DUPLEX, 0.7, COLOR_ARM_LEFT, 2, cv2.LINE_AA)

    if 14 in pts: # Right elbow
        right_angle = angles.get("right", 0.0)
        ex, ey = pts[14]
        cv2.putText(frame, f"{right_angle:.0f}°", (ex + 15, ey - 10), cv2.FONT_HERSHEY_DUPLEX, 0.7, COLOR_ARM_RIGHT, 2, cv2.LINE_AA)


def main():
    print(f"▶ Initializing PoseFit Video Annotation Pipeline...")
    print(f"  Input: {INPUT_VIDEO}")
    print(f"  Rust binary: {RUST_BIN}")

    if not os.path.exists(RUST_BIN):
        print(f"Error: Rust binary not found at {RUST_BIN}. Run `cargo build --release` first.")
        sys.exit(1)

    cap = cv2.VideoCapture(INPUT_VIDEO)
    if not cap.isOpened():
        print(f"Error: Failed to open input video {INPUT_VIDEO}")
        sys.exit(1)

    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
    fps = cap.get(cv2.CAP_PROP_FPS) or 25.0
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    print(f"  Video Specs: {width}x{height} @ {fps:.1f} FPS, {total_frames} total frames")

    # Step 1: Extract landmarks from all frames with MediaPipe Pose
    print("\n[Step 1/3] Extracting pose landmarks with MediaPipe...")
    mp_pose = mp.solutions.pose
    pose = mp_pose.Pose(
        min_detection_confidence=0.5,
        min_tracking_confidence=0.5,
        model_complexity=1
    )

    frame_landmarks_list = []
    frame_idx = 0

    while True:
        ret, frame = cap.read()
        if not ret:
            break

        timestamp_sec = frame_idx / fps
        rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        results = pose.process(rgb)

        landmarks = []
        if results.pose_landmarks:
            for lm in results.pose_landmarks.landmark:
                landmarks.append({
                    "x": float(lm.x),
                    "y": float(lm.y),
                    "z": float(lm.z),
                    "visibility": float(lm.visibility),
                    "presence": float(getattr(lm, "presence", 1.0))
                })
        else:
            # Fallback zero landmarks
            for _ in range(33):
                landmarks.append({"x": 0.0, "y": 0.0, "z": 0.0, "visibility": 0.0, "presence": 0.0})

        frame_data = {
            "frame_idx": frame_idx,
            "timestamp_sec": timestamp_sec,
            "width": width,
            "height": height,
            "landmarks": landmarks
        }
        frame_landmarks_list.append(frame_data)
        frame_idx += 1
        if frame_idx % 100 == 0 or frame_idx == total_frames:
            print(f"  Extracted landmarks: {frame_idx}/{total_frames} frames ({frame_idx*100//total_frames}%)")

    cap.release()
    pose.close()

    # Step 2: Feed landmarks to the compiled Rust engine via process-stream
    print("\n[Step 2/3] Processing landmarks through Rust PoseFitEngine...")
    rust_proc = subprocess.Popen(
        [RUST_BIN, "process-stream", "--exercise", "hammer_curl"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    rust_input = "\n".join(json.dumps(f) for f in frame_landmarks_list) + "\n"
    stdout_data, stderr_data = rust_proc.communicate(input=rust_input)

    if rust_proc.returncode != 0:
        print(f"Rust engine error (code {rust_proc.returncode}):\n{stderr_data}")
        sys.exit(1)

    print(f"Rust Engine Log:\n{stderr_data.strip()}")

    # Parse Rust telemetry lines
    telemetry_by_frame = {}
    for line in stdout_data.strip().splitlines():
        if not line.strip():
            continue
        obj = json.loads(line)
        telemetry_by_frame[obj["frame_idx"]] = obj

    print(f"  Received Rust telemetry for {len(telemetry_by_frame)} frames.")

    # Step 3: Render and annotate video
    print(f"\n[Step 3/3] Annotating video frames and encoding to {TEMP_NO_AUDIO}...")
    cap = cv2.VideoCapture(INPUT_VIDEO)
    fourcc = cv2.VideoWriter_fourcc(*'mp4v')
    out = cv2.VideoWriter(TEMP_NO_AUDIO, fourcc, fps, (width, height))

    frame_idx = 0
    rep_flash_timer = 0

    while True:
        ret, frame = cap.read()
        if not ret:
            break

        telemetry = telemetry_by_frame.get(frame_idx, {})
        landmarks = frame_landmarks_list[frame_idx]["landmarks"] if frame_idx < len(frame_landmarks_list) else []

        if telemetry.get("rep_completed", False):
            rep_flash_timer = 12 # flash for ~0.5 sec

        # Draw skeleton
        draw_skeleton(frame, landmarks, width, height, telemetry.get("angles", {}))

        # Draw HUD overlay
        draw_hud(frame, telemetry, width, height, rep_flash_timer)

        if rep_flash_timer > 0:
            rep_flash_timer -= 1

        out.write(frame)
        frame_idx += 1
        if frame_idx % 100 == 0 or frame_idx == total_frames:
            print(f"  Rendered frame: {frame_idx}/{total_frames} ({frame_idx*100//total_frames}%)")

    cap.release()
    out.release()

    # Step 4: Mux audio from original video using ffmpeg
    print(f"\nMuxing original audio into final annotated video: {OUTPUT_VIDEO}")
    cmd = [
        "ffmpeg", "-y",
        "-i", TEMP_NO_AUDIO,
        "-i", INPUT_VIDEO,
        "-c:v", "libx264",
        "-pix_fmt", "yuv420p",
        "-c:a", "aac",
        "-map", "0:v:0",
        "-map", "1:a:0?",
        OUTPUT_VIDEO
    ]
    mux_res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if mux_res.returncode == 0:
        print(f"✅ Successfully created annotated video: {OUTPUT_VIDEO}")
        if os.path.exists(TEMP_NO_AUDIO):
            os.remove(TEMP_NO_AUDIO)
    else:
        print(f"FFmpeg mux warning: {mux_res.stderr.decode()}")
        # Fallback to TEMP_NO_AUDIO if audio muxing failed
        os.rename(TEMP_NO_AUDIO, OUTPUT_VIDEO)
        print(f"Saved video to {OUTPUT_VIDEO}")

    print("\nAnnotation complete!")


if __name__ == "__main__":
    main()
