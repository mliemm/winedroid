use std::collections::BTreeSet;
use std::path::Path;

use super::bridge::BridgeProvider;
use super::bridge_test_support::register_all;
use super::link_resolution::SystemLibraryProvider;

const SKIPPED_SECTIONS: [&str; 3] = ["LIBC_PRIVATE", "LIBC_PLATFORM", "LIBC_DEPRECATED"];
const OTHER_ARCHITECTURES: [&str; 4] = ["arm", "x86", "x86_64", "riscv64"];

const CORE_LIBC: &[&str] = &[
    "malloc", "free", "calloc", "realloc", "reallocarray", "memalign", "aligned_alloc", "posix_memalign", "malloc_usable_size", "mallinfo", "mallopt",
    "memcpy", "memmove", "memset", "memcmp", "memchr", "memrchr", "memmem", "mempcpy", "strlen", "strnlen", "strcmp", "strncmp", "strcpy", "strncpy", "strcat",
    "strncat", "strchr", "strrchr", "strstr", "strcasecmp", "strncasecmp", "strcasestr", "strspn", "strcspn", "strpbrk", "strdup", "strndup", "stpcpy", "strlcpy",
    "strlcat", "strtok", "strtok_r", "strsep", "strcoll", "strxfrm", "__strlen_chk", "__memcpy_chk", "__memmove_chk", "__memset_chk", "__strcpy_chk", "__strcat_chk",
    "__strncpy_chk", "__strncpy_chk2", "__strchr_chk", "__strrchr_chk", "__vsnprintf_chk", "__vsprintf_chk", "__read_chk", "__FD_SET_chk", "abort", "__assert2",
    "android_set_abort_message", "__stack_chk_fail", "exit", "_exit", "__cxa_atexit", "__cxa_finalize", "__cxa_thread_atexit_impl", "strtol", "strtoll", "strtoul",
    "strtoull", "strtod", "strtof", "atoi", "atol", "atoll", "atof", "abs", "labs", "div", "ldiv", "rand", "srand", "random", "srandom", "lrand48", "srand48",
    "arc4random", "getrandom", "qsort", "bsearch", "getenv", "setenv", "unsetenv", "putenv", "environ", "__progname", "__errno", "strerror", "strerror_r", "isalpha",
    "isspace", "tolower", "toupper", "_ctype_", "setlocale", "newlocale", "uselocale", "localeconv", "sysconf", "getpagesize", "getauxval", "uname", "gethostname",
    "__system_property_get", "__system_property_find", "__system_property_read", "__system_property_read_callback", "snprintf", "sprintf", "vsnprintf", "vsprintf",
    "clock_gettime", "gettimeofday", "time", "nanosleep", "usleep", "sleep", "gmtime_r", "timegm", "isnan", "isinf", "ldexp", "__fpclassifyd",
];

const CORE_LIBM: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "exp", "log", "pow", "sqrt", "floor", "ceil", "round", "fmod", "hypot", "sinf", "cosf", "powf", "sqrtf",
    "ilogb", "nan", "lgamma", "sincos", "fegetround", "fesetround", "fetestexcept", "feraiseexcept", "frexp", "modf", "lrint", "llround",
];

fn architecture_applies(tags: &str) -> bool {
    let words: Vec<&str> = tags.split_whitespace().collect();
    let restricted = words.iter().any(|word| OTHER_ARCHITECTURES.contains(word) || *word == "arm64");
    !restricted || words.contains(&"arm64")
}

fn exported_names(map: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(map).unwrap();
    let mut names = BTreeSet::new();
    let (mut section, mut global) = (String::new(), false);
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_suffix('{').or_else(|| trimmed.split('#').next().and_then(|code| code.trim().strip_suffix('{'))) {
            section = name.trim().to_string();
            global = false;
        } else if trimmed.starts_with("global:") {
            global = true;
        } else if trimmed.starts_with("local:") {
            global = false;
        } else if global && !SKIPPED_SECTIONS.contains(&section.as_str()) {
            let (code, tags) = trimmed.split_once('#').unwrap_or((trimmed, ""));
            if let Some(name) = code.trim().strip_suffix(';') {
                if !name.is_empty() && !name.contains('*') && architecture_applies(tags) {
                    names.insert(name.to_string());
                }
            }
        }
    }
    names
}

fn registered(soname: &str, name: &str) -> bool {
    BridgeProvider.export(soname, name).is_some()
}

#[test]
fn libc_map_txt_exports_that_the_core_shim_registers_are_counted_and_the_core_list_is_complete() {
    register_all();
    let Some(root) = super::bionic_reference::reference_root() else {
        eprintln!("skipped: no android-ref checkout");
        return;
    };
    let exported = exported_names(&root.join("bionic/libc/libc.map.txt"));
    let present: Vec<&String> = exported.iter().filter(|name| registered("libc.so", name)).collect();
    let missing: Vec<&String> = exported.iter().filter(|name| !registered("libc.so", name)).collect();

    eprintln!("libc.map.txt arm64 exports: {}, registered: {}, absent: {}", exported.len(), present.len(), missing.len());
    eprintln!("first absent exports: {:?}", &missing[..missing.len().min(40)]);
    let unmapped: Vec<&&str> = CORE_LIBC.iter().filter(|name| !registered("libc.so", name)).collect();
    assert!(unmapped.is_empty(), "core names without a registration: {unmapped:?}");
    let not_exported: Vec<&&str> = CORE_LIBC.iter().filter(|name| !exported.contains(**name)).collect();
    assert!(not_exported.is_empty(), "core names that libc.map.txt does not export for arm64: {not_exported:?}");
    assert!(present.len() >= CORE_LIBC.len());
}

#[test]
fn registered_libc_names_are_never_exported_by_the_other_system_libraries() {
    register_all();

    for name in ["malloc", "strlen", "snprintf", "__android_log_print", "dlopen", "sin"] {
        let owners: Vec<&str> = ["libc.so", "libm.so", "libdl.so", "liblog.so", "libandroid.so"].into_iter().filter(|soname| registered(soname, name)).collect();
        assert_eq!(owners.len(), 1, "{name} is owned once: {owners:?}");
    }
}

#[test]
fn libm_exports_are_registered_under_libm_and_the_map_is_reported_when_present() {
    register_all();
    let Some(root) = super::bionic_reference::reference_root() else {
        eprintln!("skipped: no android-ref checkout");
        return;
    };
    let map = root.join("bionic/libm/libm.map.txt");
    if map.is_file() {
        let exported = exported_names(&map);
        let present = exported.iter().filter(|name| registered("libm.so", name)).count();
        eprintln!("libm.map.txt arm64 exports: {}, registered: {present}", exported.len());
    } else {
        eprintln!("libm.map.txt is not in the android-ref checkout, so libm coverage is measured against the core list only");
    }
    let unmapped: Vec<&&str> = CORE_LIBM.iter().filter(|name| !registered("libm.so", name)).collect();
    assert!(unmapped.is_empty(), "core libm names without a registration: {unmapped:?}");
}
