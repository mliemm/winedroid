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
        case "$name" in
            +*) printf 'char %s[16];\n' "${name#+}" >>"$source" ;;
            *) printf 'void %s(void) {}\n' "$name" >>"$source" ;;
        esac
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
    "$LLD" "${flags[@]}" -soname "lib$name.so" -L "${STUB_DIR:-$work/stubs}" -L "$out" "$@" \
        -o "$out/lib$name.so" "$work/$object.o"
}

build_variant() {
    local page="$1" label="$2"
    local out="$OUT_ROOT/$label"
    local work
    work="$(mktemp -d)"
    mkdir -p "$out" "$work/stubs" "$work/stubs_bridge" "$work/stubs_core" "$work/stubs_threads" "$work/stubs_signals" "$work/stubs_io" "$work/stubs_net"
    rm -f "$out"/*.so

    make_stub libc.so "$work/stubs" strlen memcpy malloc free snprintf \
        pthread_mutex_lock pthread_mutex_unlock __cxa_atexit __stack_chk_fail __tls_get_addr __fx_record
    make_stub libc.so "$work/stubs_io" \
        __errno __fgets_chk __fread_chk __fprintf_chk __fwrite_chk __snprintf_chk __sprintf_chk __stack_chk_fail __swprintf_chk __vsnprintf_chk __vsprintf_chk \
        close fclose feof fflush fgetc fopen fprintf fputs fread fscanf fseek ftell fwrite getline malloc free memcmp memcpy memset mmap mprotect munmap \
        open opendir read readdir rewind snprintf sscanf stat strcmp strlen strstr swprintf tmpfile ungetc closedir
    make_stub libc.so "$work/stubs_net" \
        __FD_SET_chk __errno __snprintf_chk __stack_chk_fail accept bind close connect epoll_create1 epoll_ctl epoll_wait eventfd \
        freeaddrinfo getaddrinfo getnameinfo getsockname getsockopt listen memset pipe poll read recv select send setsockopt shutdown socket socketpair strcmp timerfd_create timerfd_settime write
    make_stub libc.so "$work/stubs_bridge" strlen malloc free __stack_chk_fail \
        fx_probe fx_vprobe fx_get_s24 fx_get_hfa4 fx_get_ld fx_get_pair fx_nest_host fx_not_registered \
        fx_sp_check fx_touch_errno fx_clobber_probe fx_vshapes +fx_missing_object pthread_self gettid getpid __errno
    make_stub libc.so "$work/stubs_threads" \
        __cxa_thread_atexit_impl __errno __pthread_cleanup_pop __pthread_cleanup_push __stack_chk_fail clock_gettime gettid memset nanosleep \
        pthread_attr_destroy pthread_attr_getstack pthread_attr_init pthread_attr_setdetachstate pthread_attr_setstacksize \
        pthread_barrier_destroy pthread_barrier_init pthread_barrier_wait pthread_cond_broadcast pthread_cond_init pthread_cond_signal \
        pthread_cond_timedwait pthread_cond_wait pthread_condattr_init pthread_condattr_setclock pthread_create pthread_detach \
        pthread_getattr_np pthread_getname_np pthread_getspecific pthread_join pthread_key_create pthread_key_delete pthread_mutex_destroy \
        pthread_mutex_init pthread_mutex_lock pthread_mutex_trylock pthread_mutex_unlock pthread_mutexattr_destroy pthread_mutexattr_init \
        pthread_mutexattr_setprotocol pthread_mutexattr_settype pthread_once pthread_rwlock_init pthread_rwlock_rdlock pthread_rwlock_timedwrlock \
        pthread_rwlock_tryrdlock pthread_rwlock_trywrlock pthread_rwlock_unlock pthread_rwlock_wrlock pthread_rwlockattr_init \
        pthread_rwlockattr_setkind_np pthread_self pthread_setname_np pthread_setspecific pthread_spin_destroy pthread_spin_init \
        pthread_spin_lock pthread_spin_trylock pthread_spin_unlock sched_yield sem_clockwait sem_init sem_post sem_timedwait sem_trywait sem_wait \
        strcmp syscall pthread_exit
    make_stub libc.so "$work/stubs_signals" \
        __errno __stack_chk_fail abort getpid gettid memset pthread_create pthread_join pthread_kill pthread_self raise sched_yield \
        sigaction sigaddset sigaltstack sigemptyset sigismember signal sigprocmask tgkill fx_emit
    make_stub libdl.so "$work/stubs" dlopen dlsym dlclose dlerror
    make_stub libc.so "$work/stubs_core" \
        __FD_SET_chk __assert2 __cxa_atexit __errno __memcpy_chk __memset_chk __read_chk __snprintf_chk __stack_chk_fail \
        __strcat_chk __strchr_chk __strcpy_chk __strlen_chk __strncpy_chk2 __system_property_find __system_property_get \
        __system_property_read __system_property_serial __vsprintf_chk abort android_set_abort_message asctime_r atoi \
        bsearch calloc clock_gettime exit free getenv getpagesize gettimeofday gmtime_r lrand48 mallinfo malloc \
        malloc_usable_size mallopt memalign memrchr memset putenv qsort rand realloc reallocarray setenv sleep snprintf \
        sprintf srand srand48 strcasecmp strcasestr strcat strchr strcmp strcpy strerror strerror_r strlcat strlcpy strlen \
        strncasecmp strstr strtod strtok_r strtol strtoul strxfrm sysconf tolower toupper isalnum ispunct isspace isxdigit isalpha isdigit setlocale newlocale uselocale freelocale __ctype_get_mb_cur_max localeconv mbrtowc wcrtomb btowc wctob mbsinit mbtowc wctomb wcslen wcscmp wcsncmp wcschr wcsrchr wcscpy wcscat wmemcmp wmemchr wmemset wcsncpy tzset localtime_r mktime strftime ctime_r +_ctype_ +tzname +timezone +daylight timegm unsetenv usleep vsnprintf vsprintf
    make_stub libm.so "$work/stubs_core" atan2 copysignl fabsl feclearexcept fegetenv fegetround feraiseexcept fesetenv fesetround \
        fetestexcept fma fmax frexp hypot ilogb ilogbf ldexp lgamma lgamma_r llround lrint modf nan nanf nearbyint pow \
        powf round scalbln sin sincos sqrtf +signgam
    make_stub liblog.so "$work/stubs_core" __android_log_buf_write __android_log_print __android_log_set_default_tag \
        __android_log_set_minimum_priority __android_log_write __android_log_assert __android_log_set_logger
    make_stub libdl.so "$work/stubs_core" dl_iterate_phdr dladdr dlclose dlerror dlopen dlsym
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

    compile fx_bridge "$work"
    STUB_DIR="$work/stubs_bridge" link fx_bridge "$page" "$work" "$out" -lc

    compile fx_relro "$work"
    link fx_relro "$page" "$work" "$out"

    compile fx_libc "$work"
    STUB_DIR="$work/stubs_core" link fx_libc "$page" "$work" "$out" -lc -llog

    compile fx_math "$work"
    STUB_DIR="$work/stubs_core" link fx_math "$page" "$work" "$out" -lc -lm

    compile fx_threads "$work" -mno-outline-atomics
    STUB_DIR="$work/stubs_threads" link fx_threads "$page" "$work" "$out" -lc

    compile fx_signals "$work"
    STUB_DIR="$work/stubs_signals" link fx_signals "$page" "$work" "$out" -lc

    compile fx_dlmain "$work"
    STUB_DIR="$work/stubs_core" link fx_dlmain "$page" "$work" "$out" -lc -ldl

    compile fx_io "$work"
    STUB_DIR="$work/stubs_io" link fx_io "$page" "$work" "$out" -lc

    compile fx_net "$work"
    STUB_DIR="$work/stubs_net" link fx_net "$page" "$work" "$out" -lc

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
