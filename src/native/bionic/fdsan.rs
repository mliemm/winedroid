use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use super::errno_map::linux_from_darwin;
use super::export::in_libc;
use super::fatal::fatal;
use super::fd_table;
use super::fs_result::{host_errno, linux_errno_of};
use super::props;
use super::super::host_log;

pub const OWNER_FILE: u64 = 1;
pub const OWNER_DIR: u64 = 2;

const LEVEL_DISABLED: i32 = 0;
const LEVEL_WARN_ONCE: i32 = 1;
const LEVEL_WARN_ALWAYS: i32 = 2;
const LEVEL_FATAL: i32 = 3;
const PROPERTY: &str = "debug.fdsan";
const EBADF: i32 = 9;

// cite: bionic fdsan.cpp#__libc_init_fdsan
static LEVEL: AtomicI32 = AtomicI32::new(LEVEL_FATAL);
static TRACK_HOOK: AtomicU64 = AtomicU64::new(0);

fn initialize_level() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let level = set_level_from_property(LEVEL_FATAL);
        let _ = level;
    });
}

fn parse_property(value: &str) -> Option<i32> {
    // cite: bionic fdsan.cpp#android_fdsan_set_error_level_from_property
    match value {
        "1" | "fatal" => Some(LEVEL_FATAL),
        "warn" => Some(LEVEL_WARN_ALWAYS),
        "warn_once" => Some(LEVEL_WARN_ONCE),
        _ => None,
    }
}

fn set_level_from_property(default_level: i32) -> i32 {
    let level = props::get(PROPERTY).as_deref().and_then(parse_property).unwrap_or(default_level);
    LEVEL.swap(level, Ordering::SeqCst)
}

pub fn create_owner_tag(owner_type: u64, tag: u64) -> u64 {
    // cite: bionic fdsan.cpp#android_fdsan_create_owner_tag
    if tag == 0 {
        return 0;
    }
    if owner_type & 0xff != owner_type {
        fatal(None, &format!("invalid android_fdsan_owner_type value: {owner_type:x}"));
    }
    (owner_type << 56) | (tag & ((1u64 << 56) - 1))
}

pub fn tag_type(tag: u64) -> &'static str {
    // cite: bionic fdsan.cpp#android_fdsan_get_tag_type
    match tag >> 56 {
        1 => "FILE*",
        2 => "DIR*",
        3 => "unique_fd",
        5 => "FileInputStream",
        6 => "FileOutputStream",
        7 => "RandomAccessFile",
        8 => "ParcelFileDescriptor",
        4 => "sqlite",
        9 => "ART FdFile",
        10 => "DatagramSocketImpl",
        11 => "SocketImpl",
        12 => "ZipArchive",
        13 => "native_handle_t",
        255 if tag >> 48 == 0xffff => "native object of unknown type",
        255 => "Java object of unknown type",
        _ => "native object of unknown type",
    }
}

pub fn tag_value(tag: u64) -> u64 {
    // cite: bionic fdsan.cpp#android_fdsan_get_tag_value
    (((tag << 8) as i64) >> 8) as u64
}

fn describe(tag: u64) -> String {
    format!("{} {:#x}", tag_type(tag), tag_value(tag))
}

fn report(message: String) {
    initialize_level();
    // cite: bionic fdsan.cpp#fdsan_error
    match LEVEL.load(Ordering::SeqCst) {
        LEVEL_DISABLED => {}
        LEVEL_FATAL => fatal(Some("fdsan"), &message),
        level => {
            host_log::line(&format!("fdsan: {message}"));
            if level == LEVEL_WARN_ONCE {
                let _ = LEVEL.compare_exchange(LEVEL_WARN_ONCE, LEVEL_DISABLED, Ordering::SeqCst, Ordering::SeqCst);
            }
        }
    }
}

pub fn owner_tag(fd: i32) -> u64 {
    fd_table::get(fd).map_or(0, |entry| entry.owner_tag.load(Ordering::SeqCst))
}

pub fn exchange_owner_tag(fd: i32, expected: u64, new: u64) {
    // cite: bionic fdsan.cpp#android_fdsan_exchange_owner_tag
    let Some(entry) = fd_table::get(fd) else { return };
    if let Err(actual) = entry.owner_tag.compare_exchange(expected, new, Ordering::SeqCst, Ordering::SeqCst) {
        let message = match (expected != 0, actual != 0) {
            (true, true) => format!("failed to exchange ownership of file descriptor: fd {fd} is owned by {}, was expected to be owned by {}", describe(actual), describe(expected)),
            (true, false) => format!("failed to exchange ownership of file descriptor: fd {fd} is unowned, was expected to be owned by {}", describe(expected)),
            (false, true) => format!("failed to exchange ownership of file descriptor: fd {fd} is owned by {}, was expected to be unowned", describe(actual)),
            (false, false) => fatal(None, "fdsan atomic_compare_exchange_strong failed unexpectedly while exchanging owner tag"),
        };
        report(message);
    }
}

pub fn close_with_tag(fd: i32, expected: u64) -> i64 {
    // cite: bionic fdsan.cpp#android_fdsan_close_with_tag
    let Some(entry) = fd_table::get(fd) else {
        if expected != 0 {
            report(format!("double-close of file descriptor {fd} detected"));
        }
        return -i64::from(EBADF);
    };
    if let Err(actual) = entry.owner_tag.compare_exchange(expected, 0, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst) {
        let message = match (expected != 0, actual != 0) {
            (true, true) => format!("attempted to close file descriptor {fd}, expected to be owned by {}, actually owned by {}", describe(expected), describe(actual)),
            (true, false) => format!("attempted to close file descriptor {fd}, expected to be owned by {}, actually unowned", describe(expected)),
            (false, true) => format!("attempted to close file descriptor {fd}, expected to be unowned, actually owned by {}", describe(actual)),
            (false, false) => fatal(None, "fdsan atomic_compare_exchange_strong failed unexpectedly while closing"),
        };
        report(message);
    }
    fd_table::take(fd);
    let status = unsafe { libc::close(fd) };
    if status != 0 {
        let errno = host_errno();
        let linux = linux_from_darwin(errno).unwrap_or_else(|| linux_errno_of(errno));
        // cite: bionic fdsan.cpp#close
        if linux == 4 {
            return 0;
        }
        return -i64::from(linux);
    }
    0
}

pub fn register() {
    in_libc("android_fdsan_create_owner_tag", |context| context.ret_u64(create_owner_tag(context.arg_u64(0), context.arg_u64(1))));
    in_libc("android_fdsan_get_owner_tag", |context| context.ret_u64(owner_tag(context.arg_i32(0))));
    in_libc("android_fdsan_get_tag_type", |context| context.ret_ptr(super::guest_mem::intern(tag_type(context.arg_u64(0)))));
    in_libc("android_fdsan_get_tag_value", |context| context.ret_u64(tag_value(context.arg_u64(0))));
    in_libc("android_fdsan_exchange_owner_tag", |context| exchange_owner_tag(context.arg_i32(0), context.arg_u64(1), context.arg_u64(2)));
    in_libc("android_fdsan_close_with_tag", |context| {
        let result = close_with_tag(context.arg_i32(0), context.arg_u64(1));
        super::fs_result::finish_int(context, result);
    });
    in_libc("android_fdsan_get_error_level", |context| {
        initialize_level();
        context.ret_i32(LEVEL.load(Ordering::SeqCst));
    });
    in_libc("android_fdsan_set_error_level", |context| {
        initialize_level();
        context.ret_i32(LEVEL.swap(context.arg_i32(0), Ordering::SeqCst));
    });
    in_libc("android_fdsan_set_error_level_from_property", |context| {
        initialize_level();
        context.ret_i32(set_level_from_property(context.arg_i32(0)));
    });
    in_libc("android_fdsan_get_fd_table", |context| {
        super::fs_result::missing("android_fdsan_get_fd_table", "the fd table is private to the winedroid shim");
        context.ret_ptr(0);
    });
    in_libc("android_fdtrack_get_enabled", |context| context.ret_i32(0));
    in_libc("android_fdtrack_set_enabled", |context| context.ret_i32(0));
    in_libc("android_fdtrack_set_globally_enabled", |_| {});
    in_libc("android_fdtrack_compare_exchange_hook", |context| {
        // cite: bionic bionic_fdtrack.h#FDTRACK_CREATE
        let expected = context.arg_ptr(0);
        let current = TRACK_HOOK.load(Ordering::SeqCst);
        let wanted = super::guest_mem::read_value::<u64>(expected);
        if current == wanted {
            TRACK_HOOK.store(context.arg_u64(1), Ordering::SeqCst);
            context.ret_i32(1);
        } else {
            super::guest_mem::write_value::<u64>(expected, current);
            context.ret_i32(0);
        }
    });
}
