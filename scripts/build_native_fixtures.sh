#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
REF_ROOT="${ANDROID_REF_ROOT:-$HOME/Dev/android-ref}"
FIXTURES="$REPO_ROOT/tests/native_fixtures"
OUT_ROOT="${FIXTURE_OUT_ROOT:-$FIXTURES/bin}"
BIONIC="$REF_ROOT/bionic/libc"
CLANG="${CLANG:-clang}"
TARGET="aarch64-linux-android29"
TOOL_MISSING_STATUS=127

find_lld() {
    if [ -n "${LD_LLD:-}" ]; then
        printf '%s\n' "$LD_LLD"
        return
    fi
    if command -v ld.lld >/dev/null 2>&1; then
        command -v ld.lld
        return
    fi
    local sysroot
    sysroot="$(rustc --print sysroot 2>/dev/null)" || sysroot=""
    local candidate
    for candidate in "$sysroot"/lib/rustlib/*/bin/gcc-ld/ld.lld; do
        if [ -x "$candidate" ]; then
            printf '%s\n' "$candidate"
            return
        fi
    done
    echo "ld.lld not found: set LD_LLD" >&2
    exit "$TOOL_MISSING_STATUS"
}

if ! command -v "$CLANG" >/dev/null 2>&1; then
    echo "clang not found: set CLANG" >&2
    exit "$TOOL_MISSING_STATUS"
fi

LLD="$(find_lld)"
RESOURCE_INCLUDE="$("$CLANG" -print-resource-dir)/include"

COMPILE_FLAGS=(
    "--target=$TARGET"
    -ffreestanding -nostdinc -fPIC -O1 -fno-builtin
    -fstack-protector-strong -fno-emulated-tls
    -fasynchronous-unwind-tables -fno-ident
    -I "$FIXTURES/include"
    -isystem "$RESOURCE_INCLUDE"
    -isystem "$BIONIC/include"
    -isystem "$BIONIC/kernel/uapi"
    -isystem "$BIONIC/kernel/uapi/asm-arm64"
    -isystem "$BIONIC/kernel/android/uapi"
)

link_flags() {
    printf '%s\n' --eh-frame-hdr -shared -z noexecstack -z relro -z now \
        --hash-style=gnu --build-id=none --as-needed -z "max-page-size=$1"
}

make_stub() {
    local soname="$1" out_dir="$2"
    shift 2
    local source="$out_dir/${soname%.so}.c"
    : >"$source"
    local name
    for name in "$@"; do
        printf 'void %s(void) {}\n' "$name" >>"$source"
    done
    "$CLANG" "--target=$TARGET" -ffreestanding -nostdinc -fPIC -O1 -c "$source" -o "$source.o"
    "$LLD" -shared --build-id=none -soname "$soname" -o "$out_dir/$soname" "$source.o"
}

compile() {
    local name="$1" work="$2"
    shift 2
    "$CLANG" "${COMPILE_FLAGS[@]}" "$@" -c "$FIXTURES/src/$name.c" -o "$work/$name.o"
}

link() {
    local name="$1" page="$2" work="$3" out="$4"
    shift 4
    link_as "$name" "$name" "$page" "$work" "$out" "$@"
}

link_as() {
    local object="$1" name="$2" page="$3" work="$4" out="$5"
    shift 5
    local flags=()
    while IFS= read -r flag; do
        flags+=("$flag")
    done < <(link_flags "$page")
    "$LLD" "${flags[@]}" -soname "lib$name.so" -L "$work/stubs" -L "$out" "$@" \
        -o "$out/lib$name.so" "$work/$object.o"
}

build_variant() {
    local page="$1" label="$2"
    local out="$OUT_ROOT/$label"
    local work
    work="$(mktemp -d)"
    mkdir -p "$out" "$work/stubs"
    rm -f "$out"/*.so

    make_stub libc.so "$work/stubs" strlen memcpy malloc free snprintf \
        pthread_mutex_lock pthread_mutex_unlock __cxa_atexit __stack_chk_fail __tls_get_addr __fx_record
    make_stub libdl.so "$work/stubs" dlopen dlsym dlclose dlerror
    make_stub liblog.so "$work/stubs" __android_log_print

    compile fx_add "$work"
    link fx_add "$page" "$work" "$out"

    compile fx_reloc "$work"
    link fx_reloc "$page" "$work" "$out" -lc -llog

    link_as fx_reloc fx_aps2 "$page" "$work" "$out" -lc -llog --pack-dyn-relocs=android
    link_as fx_reloc fx_relr "$page" "$work" "$out" -lc -llog --pack-dyn-relocs=relr

    compile fx_dep "$work"
    link fx_dep "$page" "$work" "$out" -lfx_add -ldl

    compile fx_order_child "$work"
    link fx_order_child "$page" "$work" "$out" -lc

    compile fx_order_parent "$work"
    link fx_order_parent "$page" "$work" "$out" -lfx_order_child -lc -ldl -rpath '$ORIGIN/dep'

    compile fx_jni "$work"
    link fx_jni "$page" "$work" "$out" -lc

    rm -rf "$work"
}

write_manifest() {
    local manifest="$OUT_ROOT/manifest.txt"
    {
        printf 'clang %s\n' "$("$CLANG" --version | head -n 1)"
        printf 'ld.lld %s\n' "$("$LLD" --version | head -n 1)"
        local file
        for file in "$FIXTURES"/src/*.c "$FIXTURES"/include/jni.h; do
            printf 'source %s %s\n' "${file#"$FIXTURES"/}" "$(cksum <"$file" | cut -d' ' -f1,2)"
        done
        for file in "$OUT_ROOT"/16k/*.so "$OUT_ROOT"/4k/*.so; do
            printf 'binary %s %s\n' "${file#"$OUT_ROOT"/}" "$(cksum <"$file" | cut -d' ' -f1,2)"
        done
    } >"$manifest"
}

build_variant 16384 16k
build_variant 4096 4k
write_manifest
