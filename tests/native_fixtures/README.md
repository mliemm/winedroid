# Native test fixtures

Small aarch64-linux-android shared objects that the native loader tests use as
their oracle ("the fixture source and Bionic ABI"). Each binary is generated
from the matching file in `src/`; nothing is hand edited.

## Provenance

- Sources: `src/fx_add.c`, `src/fx_reloc.c`, `src/fx_dep.c`, `src/fx_jni.c`,
  `src/fx_order_child.c`, `src/fx_order_parent.c`, `src/fx_relro.c`,
  `src/fx_bridge.c`, `src/fx_libc.c`, `src/fx_math.c`, `src/fx_dlmain.c`,
  `src/fx_threads.c`, `src/fx_signals.c`.
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
- `libfx_order_child.so`: constructors and destructors with explicit priorities
  101 and 102 that call the imported `__fx_record`, a read only literal pool
  of `mrs tpidr_el0` and shadow stack words in an executable section without
  an FDE (`fx_pool_words`), and a function containing `mov x18, x0`.
- `libfx_order_parent.so`: `DT_NEEDED` on `libfx_order_child.so`, `DT_RUNPATH`
  `$ORIGIN/dep`, priority 101 and 102 constructors and destructors, and a
  constructor that calls `dlopen`, `dlsym` and `dlclose` while the loader
  holds its lock.
- `libfx_bridge.so`: guest variadic callees (`va_arg` over int, long, double,
  long double, 16 byte, 24 byte, homogeneous float and `__int128` arguments),
  guest callers of variadic and `va_list` imports (`fx_probe`, `fx_vprobe`),
  sret, HFA, pair and long double returning imports, a naked `sp` probe, a
  callee saved register loop, recursion through a host handler, a call of an
  unregistered import, `pthread_self`, `gettid`, `getpid`, `errno`, and a
  stack guard overflow, a variadic guest function that forwards its `va_list`
  to a host import (`fx_call_vshapes`, used with `int`, 16 byte aligned,
  homogeneous float aggregate and 32 byte struct arguments), a read of an
  unresolved data import (`fx_read_missing_object`), raw thread pointer reads
  of Bionic TLS slots (`fx_tp_*`), and an errno probe (`fx_errno_after`). It
  links against a throwaway `libc.so` stub that also names the `fx_*` imports;
  names written with a leading `+` in the build script become data objects.
- `libfx_relro.so`: a 32 KB `.data.rel.ro` table of function pointers (4096
  RELATIVE relocations) so `PT_GNU_RELRO` covers at least one whole 16 KB host
  page in both variants, plus a plain writable `fx_relro_plain` in a segment
  after it.
- `libfx_libc.so`: calls the Bionic libc core through its imports (printf
  family, FORTIFY `_chk` entry points, strto*, qsort, bsearch, strerror, env,
  random, sysconf, properties, time, locale, ctype, liblog). Several functions
  return values the tests compare with an oracle named in the test.
- `libfx_math.so`: libm and fenv calls (`ilogb`, `nan`, `lgamma`, rounding
  modes, exception flags, `fabsl`, `copysignl`).
- `libfx_dlmain.so`: guest calls of `dlopen`, `dlsym`, `dlerror`, `dladdr`,
  `dlclose` and `dl_iterate_phdr` against the other fixtures.
