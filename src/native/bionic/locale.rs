use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use super::abi_types::{EINVAL, ENOENT, LCONV_SIZE};
use super::export::in_libc;
use super::guest_mem::{optional_cstr, read_value, write_value};

const LC_ALL: i32 = 6;
const LC_LAST: i32 = 12;
const LC_ALL_MASK: i32 = (1 << (LC_LAST + 1)) - 1;
const GLOBAL_LOCALE: usize = usize::MAX;
const UTF8_MAX: u64 = 4;
const BYTE_MAX: u64 = 1;
const CHAR_MAX: u8 = 255;

static UTF8: AtomicBool = AtomicBool::new(true);
static LCONV: OnceLock<usize> = OnceLock::new();
static NAMES: [&[u8]; 2] = [b"C.UTF-8\0", b"C\0"];

thread_local! {
    static CURRENT: Cell<usize> = const { Cell::new(GLOBAL_LOCALE) };
}

fn supported(name: &[u8]) -> bool {
    // cite: bionic locale.cpp#__is_supported_locale
    matches!(name, b"" | b"C" | b"C.UTF-8" | b"en_US.UTF-8" | b"POSIX")
}

fn is_utf8(name: &[u8]) -> bool {
    name.is_empty() || name.windows(5).any(|window| window == b"UTF-8")
}

fn setlocale(category: i32, name: Option<&[u8]>) -> Result<usize, i32> {
    // cite: bionic locale.cpp#setlocale
    if !(0..=LC_LAST).contains(&category) {
        return Err(EINVAL);
    }
    if let Some(name) = name {
        if !supported(name) {
            return Err(ENOENT);
        }
        UTF8.store(is_utf8(name), Ordering::SeqCst);
    }
    Ok(NAMES[usize::from(!UTF8.load(Ordering::SeqCst))].as_ptr() as usize)
}

fn allocate_locale(utf8: bool) -> usize {
    let address = unsafe { libc::malloc(8) } as usize;
    write_value::<u64>(address, if utf8 { UTF8_MAX } else { BYTE_MAX });
    address
}

fn max_bytes(locale: usize) -> u64 {
    if locale == GLOBAL_LOCALE {
        if UTF8.load(Ordering::SeqCst) {
            UTF8_MAX
        } else {
            BYTE_MAX
        }
    } else {
        read_value::<u64>(locale)
    }
}

pub fn current_max_bytes() -> u64 {
    max_bytes(CURRENT.with(|current| current.get()))
}

fn lconv() -> usize {
    // cite: bionic locale.cpp#localeconv
    *LCONV.get_or_init(|| {
        let address = Box::leak(vec![0u8; LCONV_SIZE].into_boxed_slice()).as_mut_ptr() as usize;
        let empty = NAMES_EMPTY.as_ptr() as usize;
        let point = b".\0".as_ptr() as usize;
        for index in 0..10 {
            write_value::<usize>(address + index * 8, if index == 0 { point } else { empty });
        }
        for index in 0..14 {
            write_value::<u8>(address + 80 + index, CHAR_MAX);
        }
        address
    })
}

static NAMES_EMPTY: &[u8] = b"\0";

pub fn register() {
    in_libc("setlocale", |context| {
        let name = optional_cstr(context.arg_ptr(1));
        match setlocale(context.arg_i32(0), name) {
            Ok(address) => context.ret_ptr(address),
            Err(code) => {
                context.set_errno(code);
                context.ret_ptr(0);
            }
        }
    });
    in_libc("newlocale", |context| {
        // cite: bionic locale.cpp#newlocale
        let (mask, name) = (context.arg_i32(0), optional_cstr(context.arg_ptr(1)));
        let Some(name) = name.filter(|_| mask & !LC_ALL_MASK == 0) else {
            context.set_errno(EINVAL);
            context.ret_ptr(0);
            return;
        };
        if !supported(name) {
            context.set_errno(ENOENT);
            context.ret_ptr(0);
            return;
        }
        context.ret_ptr(allocate_locale(is_utf8(name)));
    });
    in_libc("duplocale", |context| {
        context.ret_ptr(allocate_locale(max_bytes(context.arg_ptr(0)) == UTF8_MAX))
    });
    in_libc("freelocale", |context| {
        if context.arg_ptr(0) != GLOBAL_LOCALE {
            unsafe { libc::free(context.arg_ptr(0) as *mut libc::c_void) };
        }
    });
    in_libc("uselocale", |context| {
        // cite: bionic locale.cpp#uselocale
        let requested = context.arg_ptr(0);
        let previous = CURRENT.with(|current| current.get());
        if requested != 0 {
            CURRENT.with(|current| current.set(requested));
        }
        context.ret_ptr(previous);
    });
    in_libc("__ctype_get_mb_cur_max", |context| {
        // cite: bionic locale.cpp#__ctype_get_mb_cur_max
        context.ret_u64(current_max_bytes());
    });
    in_libc("localeconv", |context| context.ret_ptr(lconv()));
    let _ = LC_ALL;
}
