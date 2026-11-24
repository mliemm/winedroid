use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use super::elf_image::TlsSegment;

#[repr(C)]
#[derive(Debug)]
pub struct TlsDescArg {
    pub generation: u64,
    pub module_id: u64,
    pub offset: u64,
}

#[derive(Debug, Clone)]
struct TlsModule {
    segment: TlsSegment,
    init_address: usize,
    generation: u64,
}

static REGISTRY: Mutex<Vec<Option<TlsModule>>> = Mutex::new(Vec::new());
static GENERATION: AtomicU64 = AtomicU64::new(0);
static EPOCH: AtomicU64 = AtomicU64::new(0);

pub fn register(segment: TlsSegment, init_address: usize) -> u32 {
    // cite: bionic linker_tls.cpp#register_soinfo_tls
    let mut registry = REGISTRY.lock().expect("tls registry");
    let module = Some(TlsModule { segment, init_address, generation: GENERATION.fetch_add(1, Ordering::SeqCst) + 1 });
    match registry.iter().position(Option::is_none) {
        Some(index) => {
            registry[index] = module;
            index as u32 + 1
        }
        None => {
            registry.push(module);
            registry.len() as u32
        }
    }
}

pub fn unregister(module_id: u32) {
    // cite: bionic linker_tls.cpp#unregister_soinfo_tls
    let mut registry = REGISTRY.lock().expect("tls registry");
    if let Some(slot) = registry.get_mut(module_id as usize - 1) {
        *slot = None;
    }
    EPOCH.fetch_add(1, Ordering::SeqCst);
}

pub fn generation_of(module_id: u32) -> Option<u64> {
    REGISTRY.lock().expect("tls registry").get(module_id as usize - 1).and_then(|slot| slot.as_ref().map(|module| module.generation))
}

struct Block {
    pointer: *mut u8,
    generation: u64,
}

#[derive(Default)]
struct Blocks {
    epoch: u64,
    entries: Vec<Option<Block>>,
}

impl Blocks {
    fn drop_stale(&mut self) {
        let epoch = EPOCH.load(Ordering::SeqCst);
        if self.epoch == epoch {
            return;
        }
        let registry = REGISTRY.lock().expect("tls registry");
        for (index, slot) in self.entries.iter_mut().enumerate() {
            let live = registry.get(index).and_then(|module| module.as_ref()).map(|module| module.generation);
            if slot.as_ref().is_some_and(|block| Some(block.generation) != live) {
                *slot = None;
            }
        }
        self.epoch = epoch;
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        unsafe { libc::free(self.pointer as *mut _) };
    }
}

thread_local! {
    static BLOCKS: RefCell<Blocks> = RefCell::new(Blocks::default());
}

pub fn existing_block_address(module_id: u32) -> Option<usize> {
    BLOCKS
        .try_with(|blocks| {
            let mut blocks = blocks.borrow_mut();
            blocks.drop_stale();
            blocks.entries.get(module_id.checked_sub(1)? as usize).and_then(|block| block.as_ref().map(|block| block.pointer as usize))
        })
        .ok()
        .flatten()
}

pub fn block_address(module_id: u32) -> usize {
    // cite: bionic bionic_elf_tls.cpp#__tls_get_addr
    if let Some(address) = existing_block_address(module_id) {
        return address;
    }
    let module = REGISTRY
        .lock()
        .expect("tls registry")
        .get(module_id as usize - 1)
        .cloned()
        .flatten()
        .unwrap_or_else(|| panic!("TLS module {module_id} is not registered"));
    let alignment = (module.segment.alignment as usize).max(16);
    let mut pointer: *mut libc::c_void = std::ptr::null_mut();
    let size = (module.segment.size as usize).max(1);
    let status = unsafe { libc::posix_memalign(&mut pointer, alignment, size) };
    assert_eq!(status, 0, "posix_memalign for TLS block failed");
    unsafe {
        std::ptr::write_bytes(pointer as *mut u8, 0, size);
        std::ptr::copy_nonoverlapping(module.init_address as *const u8, pointer as *mut u8, module.segment.init_size as usize);
    }
    BLOCKS.with(|blocks| {
        let mut blocks = blocks.borrow_mut();
        if blocks.entries.len() < module_id as usize {
            blocks.entries.resize_with(module_id as usize, || None);
        }
        blocks.entries[module_id as usize - 1] = Some(Block { pointer: pointer as *mut u8, generation: module.generation });
    });
    pointer as usize
}

#[repr(C)]
pub struct TlsIndex {
    pub module_id: u64,
    pub offset: u64,
}

pub extern "C" fn __tls_get_addr(index: *const TlsIndex) -> usize {
    let index = unsafe { &*index };
    block_address(index.module_id as u32) + index.offset as usize
}

extern "C" fn tls_block_address(arg: *const TlsDescArg) -> usize {
    let arg = unsafe { &*arg };
    std::panic::catch_unwind(|| block_address(arg.module_id as u32) + arg.offset as usize).unwrap_or_else(|_| std::process::abort())
}

std::arch::global_asm!(
    ".text",
    ".p2align 2",
    ".globl _winedroid_tlsdesc_resolver_dynamic",
    "_winedroid_tlsdesc_resolver_dynamic:",
    "ldr x0, [x0, #8]",
    "sub sp, sp, #672",
    "stp x29, x30, [sp, #0]",
    "mov x29, sp",
    "stp x1, x2, [sp, #16]",
    "stp x3, x4, [sp, #32]",
    "stp x5, x6, [sp, #48]",
    "stp x7, x8, [sp, #64]",
    "stp x9, x10, [sp, #80]",
    "stp x11, x12, [sp, #96]",
    "stp x13, x14, [sp, #112]",
    "stp x15, x16, [sp, #128]",
    "str x17, [sp, #144]",
    "stp q0, q1, [sp, #160]",
    "stp q2, q3, [sp, #192]",
    "stp q4, q5, [sp, #224]",
    "stp q6, q7, [sp, #256]",
    "stp q8, q9, [sp, #288]",
    "stp q10, q11, [sp, #320]",
    "stp q12, q13, [sp, #352]",
    "stp q14, q15, [sp, #384]",
    "stp q16, q17, [sp, #416]",
    "stp q18, q19, [sp, #448]",
    "stp q20, q21, [sp, #480]",
    "stp q22, q23, [sp, #512]",
    "stp q24, q25, [sp, #544]",
    "stp q26, q27, [sp, #576]",
    "stp q28, q29, [sp, #608]",
    "stp q30, q31, [sp, #640]",
    "bl {block}",
    "mrs x1, tpidrro_el0",
    "sub x0, x0, x1",
    "ldp q0, q1, [sp, #160]",
    "ldp q2, q3, [sp, #192]",
    "ldp q4, q5, [sp, #224]",
    "ldp q6, q7, [sp, #256]",
    "ldp q8, q9, [sp, #288]",
    "ldp q10, q11, [sp, #320]",
    "ldp q12, q13, [sp, #352]",
    "ldp q14, q15, [sp, #384]",
    "ldp q16, q17, [sp, #416]",
    "ldp q18, q19, [sp, #448]",
    "ldp q20, q21, [sp, #480]",
    "ldp q22, q23, [sp, #512]",
    "ldp q24, q25, [sp, #544]",
    "ldp q26, q27, [sp, #576]",
    "ldp q28, q29, [sp, #608]",
    "ldp q30, q31, [sp, #640]",
    "ldp x1, x2, [sp, #16]",
    "ldp x3, x4, [sp, #32]",
    "ldp x5, x6, [sp, #48]",
    "ldp x7, x8, [sp, #64]",
    "ldp x9, x10, [sp, #80]",
    "ldp x11, x12, [sp, #96]",
    "ldp x13, x14, [sp, #112]",
    "ldp x15, x16, [sp, #128]",
    "ldr x17, [sp, #144]",
    "ldp x29, x30, [sp, #0]",
    "add sp, sp, #672",
    "ret",
    ".p2align 2",
    ".globl _winedroid_tlsdesc_resolver_unresolved_weak",
    "_winedroid_tlsdesc_resolver_unresolved_weak:",
    "ldr x0, [x0, #8]",
    "str x1, [sp, #-16]!",
    "mrs x1, tpidrro_el0",
    "sub x0, x0, x1",
    "ldr x1, [sp], #16",
    "ret",
    block = sym tls_block_address,
);

extern "C" {
    fn winedroid_tlsdesc_resolver_dynamic();
    fn winedroid_tlsdesc_resolver_unresolved_weak();
}

pub fn dynamic_resolver_address() -> usize {
    winedroid_tlsdesc_resolver_dynamic as *const () as usize
}

pub fn unresolved_weak_resolver_address() -> usize {
    winedroid_tlsdesc_resolver_unresolved_weak as *const () as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bionic_register_tls_module_reuses_the_lowest_free_module_id() {
        let segment = TlsSegment { size: 8, alignment: 8, init_vaddr: 0, init_size: 0 };
        let first = register(segment.clone(), 0);
        let second = register(segment.clone(), 0);
        unregister(first);

        assert_eq!(register(segment, 0), first);
        assert_ne!(first, second);
    }

    #[test]
    fn bionic_unregister_soinfo_tls_drops_the_blocks_other_modules_ids_would_reuse() {
        static TEMPLATE: [u8; 4] = [7, 7, 7, 7];
        let segment = TlsSegment { size: 4, alignment: 4, init_vaddr: 0, init_size: 4 };
        let module = register(segment, TEMPLATE.as_ptr() as usize);
        let block = block_address(module);
        assert_eq!(unsafe { *(block as *const u8) }, 7);
        assert_eq!(existing_block_address(module), Some(block));
        let generation = generation_of(module).unwrap();

        unregister(module);

        assert_eq!(existing_block_address(module), None);
        assert_ne!(generation_of(module), Some(generation));
    }
}
