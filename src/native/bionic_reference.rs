use std::path::PathBuf;

pub fn reference_root() -> Option<PathBuf> {
    let configured = std::env::var_os("ANDROID_REF_ROOT").map(PathBuf::from);
    let sibling = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../android-ref");
    configured
        .into_iter()
        .chain(std::iter::once(sibling))
        .find(|root| root.join("bionic/libc/libc.map.txt").is_file())
}

pub fn linux_errno_value(root: &std::path::Path, name: &str) -> Option<i32> {
    [
        "bionic/libc/kernel/uapi/asm-generic/errno-base.h",
        "bionic/libc/kernel/uapi/asm-generic/errno.h",
    ]
    .iter()
    .find_map(|path| {
        let source = std::fs::read_to_string(root.join(path)).ok()?;
        source.lines().find_map(|line| {
            let mut words = line.split_whitespace();
            (words.next() == Some("#define") && words.next() == Some(name))
                .then(|| words.next()?.parse().ok())
                .flatten()
        })
    })
}
