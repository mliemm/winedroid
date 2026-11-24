pub const SYSTEM_SONAMES: [&str; 13] = [
    "libc.so",
    "libdl.so",
    "libm.so",
    "liblog.so",
    "libstdc++.so",
    "libandroid.so",
    "libz.so",
    "libjnigraphics.so",
    "libEGL.so",
    "libGLESv1_CM.so",
    "libGLESv2.so",
    "libGLESv3.so",
    "libOpenSLES.so",
];

pub fn is_system_soname(name: &str) -> bool {
    SYSTEM_SONAMES.contains(&name)
}

pub fn system_library_path(soname: &str) -> String {
    // cite: bionic linker_main.cpp#solist_add_soinfo
    let directory = match soname {
        "libc.so" | "libm.so" | "libdl.so" => "/apex/com.android.runtime/lib64/bionic",
        _ => "/system/lib64",
    };
    format!("{directory}/{soname}")
}
