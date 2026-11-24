# Native test fixtures

Small aarch64-linux-android shared objects that the native loader tests use as
their oracle ("the fixture source and Bionic ABI"). Each binary is generated
from the matching file in `src/`; nothing is hand edited.

## Provenance

- Sources: `src/fx_add.c`, `src/fx_reloc.c`, `src/fx_dep.c`, `src/fx_jni.c`.
- Headers: Bionic from the android-ref checkout (`bionic/libc/include` and the
  kernel uapi directories) plus `include/jni.h`. No Android NDK is installed.
- `include/jni.h` is written from ART's `JNINativeInterface` table in
  `art/runtime/jni/jni_internal.cc` (`gJniNativeInterface`). The function slot
  order matches that table entry for entry (233 slots, checked when the header
  was written). The android-ref checkout carries no `libnativehelper/jni.h`.
- Compiler: Apple clang with `--target=aarch64-linux-android29`, linker:
  `ld.lld` from the Rust toolchain (`rustlib/*/bin/gcc-ld/ld.lld`).
- Import stubs: the script builds throwaway `libc.so`, `libdl.so` and
  `liblog.so` with empty functions so the linker records `DT_NEEDED` and the
  undefined symbols stay imports. The stubs are not checked in.

## Build

    ANDROID_REF_ROOT=<android-ref checkout> RUSTUP_TOOLCHAIN=1.100.0 scripts/build_native_fixtures.sh

`ANDROID_REF_ROOT` defaults to `$HOME/Dev/android-ref`; `CLANG` and `LD_LLD`
override the tool lookup, and `FIXTURE_OUT_ROOT` redirects the output away from
`bin/`. Rebuilding with the same toolchain reproduces the
checked in binaries byte for byte.

## Variants

- `bin/16k/`: `-z max-page-size=16384`, `p_align` 0x4000 on every PT_LOAD, as
  current NDK builds (r28 and later) and the TikTok libraries produce.
- `bin/4k/`: `-z max-page-size=4096`, `p_align` 0x1000, as older NDK builds
  produce.

## Contents

- `libfx_add.so`: exports `add` and the data symbol `fx_data_value`.
- `libfx_reloc.so`: RELATIVE, ABS64, GLOB_DAT, JUMP_SLOT and TLSDESC
  relocations, `DT_INIT_ARRAY` with a constructor, `DT_FINI_ARRAY` with a
  destructor, a `__cxa_atexit` registration, a PT_TLS segment (`__thread`
  variables), `-fstack-protector-strong`, and imports `strlen`, `memcpy`,
  `malloc`, `free`, `snprintf`, `pthread_mutex_lock`, `pthread_mutex_unlock`,
  `__android_log_print`, `__cxa_atexit`, `__stack_chk_fail`.
- `libfx_dep.so`: `DT_NEEDED` on `libfx_add.so` (calls `add`, reads
  `fx_data_value`) and `dlopen`/`dlsym`/`dlclose` of `libfx_add.so`.
- `libfx_jni.so`: `JNI_OnLoad` with `RegisterNatives`, and `Java_fx_Native_*`
  natives that call `GetStringUTFChars`, `ReleaseStringUTFChars`,
  `NewStringUTF`, `FindClass`, `GetMethodID`, `GetStaticMethodID`,
  `CallIntMethod`, `CallStaticVoidMethod`, `GetArrayLength` and `ThrowNew`.
