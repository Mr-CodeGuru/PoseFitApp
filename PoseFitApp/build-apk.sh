#!/usr/bin/env bash
# ==============================================================================
# POSEFIT — PURE RUST ANDROID APK BUILD SCRIPT
#
# Builds and signs the 100% pure Rust Android application.
# ZERO Java, ZERO Kotlin, ZERO Gradle.
#
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="${SCRIPT_DIR}/target"
BUILD_DIR="${TARGET_DIR}/android-apk"
OUTPUT_APK="${TARGET_DIR}/PoseFit-release.apk"
TARGET_ARCH="${1:-aarch64-linux-android}"

echo "============================================================"
echo "▶ Building PoseFit Pure-Rust Android Application"
echo "  Target: ${TARGET_ARCH}"
echo "============================================================"

# Resolve Android SDK / NDK if available
ANDROID_HOME="${ANDROID_HOME:-${HOME}/Library/Android/sdk}"

# 1. Compile native Rust cdylib
echo "▶ [1/5] Compiling native Rust library (libposefit_android.so)..."
cargo build --package posefit-android --target "${TARGET_ARCH}" --release

# 2. Prepare native lib structure
echo "▶ [2/5] Assembling native library tree..."
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}/lib/arm64-v8a"
cp "${TARGET_DIR}/${TARGET_ARCH}/release/libposefit_android.so" "${BUILD_DIR}/lib/arm64-v8a/"

# 3. Locate aapt2 and android.jar if SDK exists
if [ -d "${ANDROID_HOME}" ]; then
    echo "▶ [3/5] Compiling Binary AndroidManifest.xml via aapt2..."
    ANDROID_JAR=$(find "${ANDROID_HOME}/platforms" -name "android.jar" 2>/dev/null | sort -V | tail -n 1 || true)
    BUILD_TOOLS_DIR=$(find "${ANDROID_HOME}/build-tools" -name "aapt2" 2>/dev/null -exec dirname {} \; | sort -V | tail -n 1 || true)

    if [ -n "${ANDROID_JAR}" ] && [ -n "${BUILD_TOOLS_DIR}" ]; then
        "${BUILD_TOOLS_DIR}/aapt2" link \
            -I "${ANDROID_JAR}" \
            --manifest "${SCRIPT_DIR}/posefit-android/AndroidManifest.xml" \
            -o "${TARGET_DIR}/PoseFit-base.apk"

        cd "${BUILD_DIR}"
        zip -u -r "${TARGET_DIR}/PoseFit-base.apk" lib/
        cd "${SCRIPT_DIR}"

        echo "▶ [4/5] Aligning APK (zipalign)..."
        "${BUILD_TOOLS_DIR}/zipalign" -v -p 4 "${TARGET_DIR}/PoseFit-base.apk" "${TARGET_DIR}/PoseFit-aligned.apk"

        echo "▶ [5/5] Signing APK (apksigner)..."
        keytool -genkey -v -keystore "${TARGET_DIR}/debug.keystore" \
            -storepass android -alias androiddebugkey -keypass android \
            -keyalg RSA -keysize 2048 -validity 10000 \
            -dname "CN=Android Debug,O=Android,C=US" 2>/dev/null || true

        "${BUILD_TOOLS_DIR}/apksigner" sign \
            --ks "${TARGET_DIR}/debug.keystore" \
            --ks-pass pass:android \
            --key-pass pass:android \
            --out "${OUTPUT_APK}" \
            "${TARGET_DIR}/PoseFit-aligned.apk"

        "${BUILD_TOOLS_DIR}/apksigner" verify "${OUTPUT_APK}"
        echo "✅ Valid signed APK created at: ${OUTPUT_APK}"
        exit 0
    fi
fi

# Fallback: simple packaging container
echo "⚠️  Android build-tools not found locally. Packaging fallback zip container..."
cd "${BUILD_DIR}"
cp "${SCRIPT_DIR}/posefit-android/AndroidManifest.xml" .
zip -r -0 "${OUTPUT_APK}" AndroidManifest.xml lib/

echo "============================================================"
echo "ℹ️  For installation on real Android devices, build via GitHub Actions"
echo "   which compiles binary AXML and signs the APK with apksigner."
echo "============================================================"
