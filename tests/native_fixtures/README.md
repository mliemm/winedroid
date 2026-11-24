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
  `ld.lld` from the Rust toolchain (`rustlib/*/bin/gcc-ld/ld.lld`). The
  versions that produced the checked in binaries are the first two lines of
  `bin/manifest.txt` (Apple clang 21.0.0 clang-2100.4.1.1, LLD 23.1.5 from
  Rust 1.100.0).
- `bin/manifest.txt` also lists the POSIX `cksum` of every source, `jni.h` and
  binary. A unit test fails when a source or binary changes without a rebuild.
  The byte for byte rebuild test needs `ANDROID_REF_ROOT` and the recorded
  toolchain, and says so when it skips (the build script exits 127 when it
  finds no clang or ld.lld, which the test treats as a skip).
- The build script is also covered without the android-ref checkout: a test
  runs it with recording stand-ins for clang and ld.lld and checks the page
  size, hardening, packing and include flags of every compile and link line.
- The JNI slot indices of every function `fx_jni.c` calls are pinned in a unit
  test from ART's `gJniNativeInterface` and `gJniInvokeInterface`, and compared
  with the header and with the table offsets loaded by the binaries.
- `libfx_add.so` has no relocations, so a unit test maps it and runs `add`
  natively on Apple Silicon. The other fixtures need the loader and are only
  checked structurally.
- `include/jni.h` parameter and return types are compared with the ART
  `JNIImpl` and `JII` definitions slot by slot when `ANDROID_REF_ROOT` is set.
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
  destructor, a `__cxa_atexit` registration with a hidden `__dso_handle` as crtbegin_so.o
  defines it, a PT_TLS segment (`__thread`
  variables), `-fstack-protector-strong`, and imports `strlen`, `memcpy`,
  `malloc`, `free`, `snprintf`, `pthread_mutex_lock`, `pthread_mutex_unlock`,
  `__android_log_print`, `__cxa_atexit`, `__stack_chk_fail`.
- `libfx_aps2.so` and `libfx_relr.so`: the `fx_reloc.c` source linked with
  `--pack-dyn-relocs=android` (`DT_ANDROID_RELA` with the APS2 payload) and
  `--pack-dyn-relocs=relr` (`DT_RELR`).
- `libfx_dep.so`: `DT_NEEDED` on `libfx_add.so` (calls `add`, reads
  `fx_data_value`) and `dlopen`/`dlsym`/`dlclose` of `libfx_add.so`.
- `libfx_jni.so`: `JNI_OnLoad` with `RegisterNatives`, and `Java_fx_Native_*`
  natives that call `GetStringUTFChars`, `ReleaseStringUTFChars`,
  `NewStringUTF`, `FindClass`, `GetMethodID`, `GetStaticMethodID`,
  `CallIntMethod`, `CallStaticVoidMethod`, `GetArrayLength` and `ThrowNew`.
