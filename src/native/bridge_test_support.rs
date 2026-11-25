use std::cell::RefCell;
use std::sync::{Arc, Once};

use super::bridge::{register_handler, BridgeProvider, CallContext};
use super::call_assign::CallBuilder;
use super::dl_api::DlHandle;
use super::handler_registry::register_all_groups;
use super::loader::Loader;
use super::loader_test_support::fixture_dir;
use super::quad::f64_to_quad;
use super::va_list::GuestVaList;

#[derive(Debug, Clone, PartialEq)]
pub enum Seen {
    Int(i64),
    Real(f64),
    Quad(u128),
}

thread_local! {
    static PROBE: RefCell<Vec<Seen>> = const { RefCell::new(Vec::new()) };
    static LOGGED: RefCell<Vec<(i32, String, String)>> = const { RefCell::new(Vec::new()) };
    static ATEXIT: RefCell<Vec<(u64, u64, u64)>> = const { RefCell::new(Vec::new()) };
    static STACK_SEEN: RefCell<(u64, Vec<u64>)> = const { RefCell::new((0, Vec::new())) };
    static SHAPES: RefCell<(Vec<Vec<u8>>, Vec<(i32, i32)>)> = const { RefCell::new((Vec::new(), Vec::new())) };
}

pub fn take_stack_seen() -> (u64, Vec<u64>) {
    STACK_SEEN.with(|seen| std::mem::take(&mut *seen.borrow_mut()))
}

pub fn take_shapes() -> (Vec<Vec<u8>>, Vec<(i32, i32)>) {
    SHAPES.with(|shapes| std::mem::take(&mut *shapes.borrow_mut()))
}

pub fn take_probe() -> Vec<Seen> {
    PROBE.with(|probe| std::mem::take(&mut *probe.borrow_mut()))
}

pub fn take_logged() -> Vec<(i32, String, String)> {
    LOGGED.with(|logged| std::mem::take(&mut *logged.borrow_mut()))
}

pub fn take_atexit() -> Vec<(u64, u64, u64)> {
    ATEXIT.with(|registered| std::mem::take(&mut *registered.borrow_mut()))
}

fn c_string(pointer: usize) -> String {
    unsafe { std::ffi::CStr::from_ptr(pointer as *const _) }.to_string_lossy().into_owned()
}

pub fn format_c(format: &str, va: &mut GuestVaList) -> String {
    let mut out = String::new();
    let mut chars = format.chars().peekable();
    while let Some(letter) = chars.next() {
        if letter != '%' {
            out.push(letter);
            continue;
        }
        let mut long = false;
        while chars.peek() == Some(&'l') {
            chars.next();
            long = true;
        }
        match chars.next() {
            Some('d') => out.push_str(&if long { va.next_i64().to_string() } else { va.next_i32().to_string() }),
            Some('x') => out.push_str(&format!("{:x}", va.next_u32())),
            Some('c') => out.push(char::from(va.next_u32() as u8)),
            Some('s') => out.push_str(&c_string(va.next_ptr())),
            Some('f') => out.push_str(&format!("{:.6}", va.next_f64())),
            Some('%') => out.push('%'),
            other => panic!("test formatter does not know %{other:?}"),
        }
    }
    out
}

fn register_libc_handlers() {
    register_handler("strlen", |context| {
        let length = unsafe { libc::strlen(context.arg_ptr(0) as *const _) };
        context.ret_u64(length as u64);
    });
    register_handler("memcpy", |context| {
        unsafe { std::ptr::copy_nonoverlapping(context.arg_ptr(1) as *const u8, context.arg_ptr(0) as *mut u8, context.arg_u64(2) as usize) };
        context.ret_ptr(context.arg_ptr(0));
    });
    register_handler("malloc", |context| {
        let pointer = unsafe { libc::malloc(context.arg_u64(0) as usize) };
        context.ret_ptr(pointer as usize);
    });
    register_handler("free", |context| unsafe { libc::free(context.arg_ptr(0) as *mut _) });
    register_handler("snprintf", |context| {
        let (buffer, capacity, format) = (context.arg_ptr(0), context.arg_u64(1) as usize, c_string(context.arg_ptr(2)));
        let text = format_c(&format, &mut context.varargs(3, 0));
        let writable = capacity.saturating_sub(1).min(text.len());
        if capacity > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(text.as_ptr(), buffer as *mut u8, writable);
                *(buffer as *mut u8).add(writable) = 0;
            }
        }
        context.ret_i32(text.len() as i32);
    });
    register_handler("__android_log_print", |context| {
        let (priority, tag, format) = (context.arg_i32(0), c_string(context.arg_ptr(1)), c_string(context.arg_ptr(2)));
        let text = format_c(&format, &mut context.varargs(3, 0));
        LOGGED.with(|logged| logged.borrow_mut().push((priority, tag, text)));
        context.ret_i32(1);
    });
    register_handler("pthread_mutex_lock", |context| context.ret_i32(0));
    register_handler("pthread_mutex_unlock", |context| context.ret_i32(0));
    register_handler("__cxa_atexit", |context| {
        ATEXIT.with(|registered| registered.borrow_mut().push((context.arg_u64(0), context.arg_u64(1), context.arg_u64(2))));
        context.ret_i32(0);
    });
}

fn probe_value_list(context: &mut CallContext) {
    let mut va = context.varargs(1, 0);
    let mut seen = vec![Seen::Int(va.next_i64()), Seen::Real(va.next_f64()), Seen::Int(i64::from(va.next_i32()))];
    seen.push(Seen::Real(va.next_f64()));
    seen.push(Seen::Quad(va.next_quad()));
    for _ in 0..13 {
        seen.push(Seen::Int(i64::from(va.next_i32())));
    }
    for _ in 0..10 {
        seen.push(Seen::Real(va.next_f64()));
    }
    seen.push(Seen::Quad(va.next_quad()));
    PROBE.with(|probe| *probe.borrow_mut() = seen);
    let first = context.arg_i32(0);
    context.ret_i32(first);
}

fn register_fixture_handlers() {
    register_handler("fx_probe", probe_value_list);
    register_handler("fx_vprobe", |context| {
        let count = context.arg_i32(0);
        match context.va_list_arg(1) {
            Ok(mut va) => {
                let sum: i64 = (0..count).map(|_| i64::from(va.next_i32())).sum();
                context.ret_i64(sum);
            }
            Err(error) => context.fail(error.to_string()),
        }
    });
    register_handler("fx_vshapes", |context| {
        let shapes = c_string(context.arg_ptr(0));
        let mut va = match context.va_list_arg(1) {
            Ok(va) => va,
            Err(error) => return context.fail(error.to_string()),
        };
        let (mut values, mut offsets) = (Vec::new(), Vec::new());
        for shape in shapes.chars() {
            let bytes = match shape {
                'i' => va.next_i64().to_le_bytes().to_vec(),
                'A' => va.next_composite(16, 16, None),
                'S' => va.next_composite(16, 8, None),
                'H' => va.next_composite(16, 8, Some((8, 2))),
                'F' => va.next_composite(12, 4, Some((4, 3))),
                'B' => va.next_composite(32, 8, None),
                other => panic!("test handler does not know shape {other}"),
            };
            values.push(bytes);
            offsets.push((va.raw().gr_offs, va.raw().vr_offs));
        }
        SHAPES.with(|record| *record.borrow_mut() = (values, offsets));
        context.ret_i64(shapes.len() as i64);
    });
    register_handler("fx_get_s24", |context| {
        let bytes: Vec<u8> = [1i64, 2, 3].iter().flat_map(|value| value.to_le_bytes()).collect();
        context.ret_sret(&bytes);
    });
    register_handler("fx_get_hfa4", |context| context.ret_hfa_f32(&[1.0, 2.0, 3.0, 4.0]));
    register_handler("fx_get_ld", |context| context.ret_quad(f64_to_quad(2.5)));
    register_handler("fx_get_pair", |context| context.ret_pair(7, 9));
    register_handler("fx_take_narrow", |context| {
        let character = context.arg_u32(0) as u8 as i8;
        let short = context.arg_u32(1) as u16 as i16;
        let int = context.arg_i32(2);
        let boolean = context.arg_u32(3) as u8 != 0;
        let byte = context.arg_u32(4) as u8;
        let sum = i32::from(character) + i32::from(short) + int + i32::from(byte) + if boolean { 1000 } else { 0 };
        context.ret_i32(sum);
    });
    register_handler("fx_nest_host", |context| {
        let (function, depth) = (context.arg_ptr(0), context.arg_u64(1));
        match CallBuilder::new(function).int(depth).invoke() {
            Ok(result) => context.ret_u64(result.u64()),
            Err(error) => context.fail(error.to_string()),
        }
    });
    register_handler("fx_sp_check", |context| {
        let aligned = context.stack_pointer() % 16 == 0;
        let words = (0..10).map(|index| context.stack_u64(index)).collect();
        STACK_SEEN.with(|seen| *seen.borrow_mut() = (context.stack_pointer(), words));
        context.ret_i32(i32::from(aligned));
    });
    register_handler("fx_touch_errno", |context| match context.arg_i32(0) {
        0 => unsafe { *libc::__error() = 99 },
        1 => context.set_errno(34),
        2 => {
            let mut ends = [0i32; 2];
            unsafe {
                assert_eq!(libc::pipe(ends.as_mut_ptr()), 0);
                libc::fcntl(ends[0], libc::F_SETFL, libc::O_NONBLOCK);
                let mut byte = 0u8;
                assert_eq!(libc::read(ends[0], &mut byte as *mut u8 as *mut _, 1), -1);
                let host_errno = *libc::__error();
                libc::close(ends[0]);
                libc::close(ends[1]);
                context.set_errno_from_host(host_errno);
            }
        }
        3 => context.set_errno_from_host(libc::EPROCLIM),
        _ => context.set_errno(1000),
    });
    register_handler("fx_clobber_probe", |_| {
        let mut accumulator = [0u64; 32];
        for round in 0..64u64 {
            for (index, slot) in accumulator.iter_mut().enumerate() {
                *slot = slot.wrapping_mul(31).wrapping_add(round + index as u64);
            }
        }
        std::hint::black_box(&accumulator);
        std::hint::black_box(accumulator.iter().map(|value| *value as f64).sum::<f64>());
    });
}

pub fn register_all() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_libc_handlers();
        register_fixture_handlers();
        register_all_groups();
    });
}

pub struct Bridged {
    pub loader: Arc<Loader>,
    pub handle: u64,
}

impl Bridged {
    pub fn load(variant: &str, library: &str) -> Self {
        register_all();
        let loader = Arc::new(Loader::new(vec![fixture_dir(variant)], Box::new(BridgeProvider)).with_strict(false));
        let handle = loader.dlopen(Some(library), 2, None).unwrap_or_else(|error| panic!("{variant}/{library}: {error}"));
        Self { loader, handle }
    }

    pub fn symbol(&self, name: &str) -> usize {
        self.loader.dlsym(DlHandle::Module(self.handle), name, None).unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    pub fn call(&self, name: &str) -> CallBuilder {
        CallBuilder::new(self.symbol(name))
    }
}
