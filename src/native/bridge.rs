use std::collections::HashMap;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::error::{Result, WinedroidError};

use super::guest_call::abort_active_call;
use super::guest_regs::Regs;
use super::link_resolution::{ImportKind, SystemLibraryProvider};
use super::stub_table::{initialize, stub_address};
use super::system_libraries::is_system_soname;
use super::trampoline::STUB_COUNT;
use super::va_list::GuestVaList;

pub type Handler = Arc<dyn Fn(&mut CallContext) + Send + Sync>;

enum Binding {
    Handler(Handler),
    Missing { importer: String },
    Overflow,
}

pub(super) struct Entry {
    name: String,
    binding: Binding,
}

struct Names {
    ids: HashMap<String, usize>,
    data: HashMap<String, usize>,
}

static ENTRIES: [AtomicPtr<Entry>; STUB_COUNT] = [const { AtomicPtr::new(std::ptr::null_mut()) }; STUB_COUNT];
static NAMES: OnceLock<Mutex<Names>> = OnceLock::new();

fn names() -> &'static Mutex<Names> {
    NAMES.get_or_init(|| Mutex::new(Names { ids: HashMap::new(), data: HashMap::new() }))
}

pub struct CallContext<'a> {
    regs: &'a mut Regs,
    symbol: &'a str,
    failure: Option<String>,
    errno: Option<i32>,
}

impl<'a> CallContext<'a> {
    pub fn new(regs: &'a mut Regs, symbol: &'a str) -> Self {
        Self { regs, symbol, failure: None, errno: None }
    }

    pub fn symbol(&self) -> &str {
        self.symbol
    }

    pub fn regs(&self) -> &Regs {
        self.regs
    }

    pub fn return_address(&self) -> u64 {
        self.regs.lr
    }

    pub fn stack_pointer(&self) -> u64 {
        self.regs.sp
    }

    pub fn arg_u64(&self, index: usize) -> u64 {
        self.regs.arg_u64(index)
    }

    pub fn arg_i32(&self, index: usize) -> i32 {
        self.regs.arg_i32(index)
    }

    pub fn arg_u32(&self, index: usize) -> u32 {
        self.regs.arg_u32(index)
    }

    pub fn arg_ptr(&self, index: usize) -> usize {
        self.regs.arg_ptr(index)
    }

    pub fn arg_f64(&self, index: usize) -> f64 {
        self.regs.arg_f64(index)
    }

    pub fn arg_f32(&self, index: usize) -> f32 {
        self.regs.arg_f32(index)
    }

    pub fn arg_quad(&self, index: usize) -> u128 {
        self.regs.arg_quad(index)
    }

    pub fn stack_u64(&self, index: usize) -> u64 {
        unsafe { std::ptr::read_unaligned((self.regs.sp as *const u64).wrapping_add(index)) }
    }

    pub fn varargs(&self, named_gp: u32, named_fp: u32) -> GuestVaList<'_> {
        GuestVaList::from_regs(self.regs, named_gp, named_fp)
    }

    pub fn va_list_arg(&self, index: usize) -> Result<GuestVaList<'static>> {
        GuestVaList::from_guest(self.regs.x[index])
    }

    pub fn ret_i32(&mut self, value: i32) {
        self.regs.ret_i32(value);
    }

    pub fn ret_u32(&mut self, value: u32) {
        self.regs.ret_u32(value);
    }

    pub fn ret_i64(&mut self, value: i64) {
        self.regs.ret_i64(value);
    }

    pub fn ret_u64(&mut self, value: u64) {
        self.regs.ret_u64(value);
    }

    pub fn ret_ptr(&mut self, value: usize) {
        self.regs.ret_ptr(value);
    }

    pub fn ret_pair(&mut self, first: u64, second: u64) {
        self.regs.ret_pair(first, second);
    }

    pub fn ret_f32(&mut self, value: f32) {
        self.regs.ret_f32(value);
    }

    pub fn ret_f64(&mut self, value: f64) {
        self.regs.ret_f64(value);
    }

    pub fn ret_quad(&mut self, value: u128) {
        self.regs.ret_quad(value);
    }

    pub fn ret_hfa_f32(&mut self, members: &[f32]) {
        self.regs.ret_hfa_f32(members);
    }

    pub fn ret_sret(&mut self, bytes: &[u8]) {
        self.regs.ret_sret(bytes);
    }

    pub fn set_errno(&mut self, linux_code: i32) {
        self.errno = Some(linux_code);
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.failure.get_or_insert_with(|| message.into());
    }
}

fn leak_entry(entry: Entry) -> *mut Entry {
    Box::into_raw(Box::new(entry))
}

fn bind(names: &mut Names, entries: &[AtomicPtr<Entry>], name: &str, binding: Binding) -> std::result::Result<usize, String> {
    let overflow = entries.len() - 1;
    if let Some(&id) = names.ids.get(name) {
        let current = unsafe { &*entries[id].load(Ordering::Acquire) };
        match (&current.binding, &binding) {
            (Binding::Handler(_), Binding::Handler(_)) => return Err(format!("native handler already registered: {name}")),
            (Binding::Missing { .. }, Binding::Handler(_)) => {
                entries[id].store(leak_entry(Entry { name: name.to_string(), binding }), Ordering::Release);
            }
            _ => {}
        }
        return Ok(id);
    }
    let id = names.ids.len();
    if id >= overflow {
        eprintln!("FIXME: native import stub table is full, {name} cannot get a stub; next missing primitive: more native import stub slots");
        if entries[overflow].load(Ordering::Acquire).is_null() {
            entries[overflow].store(leak_entry(Entry { name: "native import stub table is full".to_string(), binding: Binding::Overflow }), Ordering::Release);
        }
        return Ok(overflow);
    }
    entries[id].store(leak_entry(Entry { name: name.to_string(), binding }), Ordering::Release);
    names.ids.insert(name.to_string(), id);
    Ok(id)
}

fn register_binding(name: &str, binding: Binding) -> usize {
    initialize(spk_dispatch);
    let bound = bind(&mut names().lock().expect("bridge names"), &ENTRIES, name, binding);
    match bound {
        Ok(id) => stub_address(id),
        Err(message) => panic!("{message}"),
    }
}

pub fn register_handler(name: &str, handler: impl Fn(&mut CallContext) + Send + Sync + 'static) -> usize {
    register_binding(name, Binding::Handler(Arc::new(handler)))
}

pub fn register_missing(importer: &str, name: &str) -> usize {
    register_binding(name, Binding::Missing { importer: importer.to_string() })
}

pub fn handler_address(name: &str) -> Option<usize> {
    let names = names().lock().expect("bridge names");
    let id = *names.ids.get(name)?;
    let entry = unsafe { &*ENTRIES[id].load(Ordering::Acquire) };
    matches!(entry.binding, Binding::Handler(_)).then(|| stub_address(id))
}

pub fn register_data(name: &str, bytes: usize) -> usize {
    let mut names = names().lock().expect("bridge names");
    *names.data.entry(name.to_string()).or_insert_with(|| {
        let storage = unsafe { libc::calloc(1, bytes.max(16)) };
        assert!(!storage.is_null(), "data object allocation failed");
        storage as usize
    })
}

fn data_address(name: &str) -> Option<usize> {
    names().lock().expect("bridge names").data.get(name).copied()
}

fn unmapped_data_address(importer: &str, name: &str) -> usize {
    let key = format!("unresolved:{name}");
    let mut names = names().lock().expect("bridge names");
    *names.data.entry(key).or_insert_with(|| {
        let page = unsafe { libc::mmap(std::ptr::null_mut(), 16384, libc::PROT_NONE, libc::MAP_PRIVATE | libc::MAP_ANON, -1, 0) };
        assert_ne!(page, libc::MAP_FAILED, "unresolved data page mapping failed for {importer} {name}");
        eprintln!("{}", missing_data_message(importer, name));
        page as usize
    })
}

fn missing_message(importer: &str, name: &str) -> String {
    format!("FIXME: native import not implemented: {importer} {name}; next missing primitive: native import {name}")
}

pub(super) fn missing_data_message(importer: &str, name: &str) -> String {
    format!("FIXME: native data import not implemented: {importer} {name}; next missing primitive: native data object {name}")
}

fn run_handler(entry: &Entry, handler: &Handler, regs: &mut Regs) -> Option<String> {
    let saved_errno = unsafe { *libc::__error() };
    let mut context = CallContext::new(regs, &entry.name);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(&mut context)));
    if outcome.is_err() {
        eprintln!("host handler panicked in {} lr={:#x}", entry.name, context.regs.lr);
        std::process::abort();
    }
    unsafe { *libc::__error() = context.errno.unwrap_or(saved_errno) };
    context.failure.take()
}

extern "C" fn spk_dispatch(id: u64, regs: *mut Regs) {
    let entry = unsafe { ENTRIES[id as usize].load(Ordering::Acquire).as_ref() };
    let Some(entry) = entry else {
        eprintln!("native stub {id} has no binding");
        std::process::abort();
    };
    let regs = unsafe { &mut *regs };
    let failure = match &entry.binding {
        Binding::Handler(handler) => run_handler(entry, handler, regs),
        Binding::Missing { importer } => {
            eprintln!("FIXME: native import not implemented: {importer} {}", entry.name);
            Some(WinedroidError::new(missing_message(importer, &entry.name)).to_string())
        }
        Binding::Overflow => Some(WinedroidError::new("native import stub table is full; next missing primitive: more native import stub slots").to_string()),
    };
    if let Some(message) = failure {
        abort_active_call(message);
    }
}

pub fn native_provider() -> BridgeProvider {
    super::thread_handlers::register_thread_handlers();
    BridgeProvider
}

pub struct BridgeProvider;

impl SystemLibraryProvider for BridgeProvider {
    fn export(&self, soname: &str, name: &str) -> Option<usize> {
        if !is_system_soname(soname) {
            return None;
        }
        data_address(name).or_else(|| handler_address(name))
    }

    fn unresolved(&self, importer: &str, name: &str, kind: ImportKind) -> usize {
        match kind {
            ImportKind::Function => register_missing(importer, name),
            ImportKind::Data => unmapped_data_address(importer, name),
        }
    }
}

#[cfg(test)]
pub(super) fn bind_for_test(entries: &[AtomicPtr<Entry>], names: &mut HashMap<String, usize>, name: &str, handler: bool) -> usize {
    let mut scratch = Names { ids: std::mem::take(names), data: HashMap::new() };
    let binding = if handler { Binding::Handler(Arc::new(|_| {})) } else { Binding::Missing { importer: "test".to_string() } };
    let id = bind(&mut scratch, entries, name, binding);
    *names = scratch.ids;
    id.unwrap_or_else(|message| panic!("{message}"))
}

#[cfg(test)]
pub(super) fn entry_is_overflow(entries: &[AtomicPtr<Entry>], id: usize) -> bool {
    unsafe { entries[id].load(Ordering::Acquire).as_ref() }.is_some_and(|entry| matches!(entry.binding, Binding::Overflow))
}
