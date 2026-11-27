#!/usr/bin/env bash
set -euo pipefail

REF_ROOT="${ANDROID_REF_ROOT:-$HOME/Dev/android-ref}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

PLATFORM_TAG="android-14.0.0_r1"

FRAMEWORKS_BASE_PATHS=(core/java core/jni core/res graphics/java libs/androidfw native/android
                       services/core/java telephony/java media/java)

LIBCORE_PATHS=(ojluni/src/main luni/src/main json/src/main)

ART_PATHS=(runtime libdexfile)

DALVIK_PATHS=(docs opcode-gen)

BIONIC_PATHS=(libc linker libm docs)

LOGGING_PATHS=(liblog)

FRAMEWORKS_NATIVE_PATHS=(libs/input include)

CTS_PATHS=(tests/tests/view tests/tests/widget tests/tests/text
           tests/tests/graphics tests/tests/content tests/tests/database)

ANDROIDX_BRANCH="androidx-main"
ANDROIDX_PATHS=(appcompat activity fragment core compose room sqlite datastore lifecycle)

fetch_sparse() {
    local url="$1" dest="$2" ref="$3"
    shift 3
    if [ -d "$dest/.git" ]; then
        git -C "$dest" sparse-checkout set "$@"
        echo "reconciled sparse paths: $dest"
        return
    fi
    git clone --depth 1 --filter=blob:none --sparse --branch "$ref" "$url" "$dest"
    git -C "$dest" sparse-checkout set "$@"
    git -C "$dest" rev-parse HEAD > "$dest/.pin"
    echo "pinned $dest at $(cat "$dest/.pin")"
}

mkdir -p "$REF_ROOT"

fetch_sparse https://android.googlesource.com/platform/frameworks/base \
    "$REF_ROOT/frameworks-base" "$PLATFORM_TAG" "${FRAMEWORKS_BASE_PATHS[@]}"

fetch_sparse https://android.googlesource.com/platform/libcore \
    "$REF_ROOT/libcore" "$PLATFORM_TAG" "${LIBCORE_PATHS[@]}"

fetch_sparse https://android.googlesource.com/platform/art \
    "$REF_ROOT/art" "$PLATFORM_TAG" "${ART_PATHS[@]}"

fetch_sparse https://android.googlesource.com/platform/dalvik \
    "$REF_ROOT/dalvik" "$PLATFORM_TAG" "${DALVIK_PATHS[@]}"

fetch_sparse https://android.googlesource.com/platform/bionic \
    "$REF_ROOT/bionic" "$PLATFORM_TAG" "${BIONIC_PATHS[@]}"

fetch_sparse https://android.googlesource.com/platform/system/logging \
    "$REF_ROOT/logging" "$PLATFORM_TAG" "${LOGGING_PATHS[@]}"

fetch_sparse https://android.googlesource.com/platform/frameworks/native \
    "$REF_ROOT/frameworks-native" "$PLATFORM_TAG" "${FRAMEWORKS_NATIVE_PATHS[@]}"

fetch_sparse https://android.googlesource.com/platform/cts \
    "$REF_ROOT/cts" "$PLATFORM_TAG" "${CTS_PATHS[@]}"

fetch_sparse https://android.googlesource.com/platform/frameworks/support \
    "$REF_ROOT/androidx" "$ANDROIDX_BRANCH" "${ANDROIDX_PATHS[@]}"

"$SCRIPT_DIR/materialize_android_ref_repositories.sh" "$REF_ROOT"

echo
echo "Android reference checkout ready at: $REF_ROOT"
du -sh "$REF_ROOT"/* 2>/dev/null || true
