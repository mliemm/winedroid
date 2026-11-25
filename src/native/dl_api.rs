use std::cell::RefCell;
use std::sync::Arc;

use crate::error::{Result, WinedroidError};

use super::elf_image::TlsSegment;
use super::link_resolution::lookup_in_module;
use super::loader::{Guard, Loader, MAIN_HANDLE};
use super::module::{GuestImage, ModuleId, State, RTLD_GLOBAL};
use super::symbol_table::{STT_GNU_IFUNC, STT_TLS};
use super::tls_modules;

const ERROR_BUFFER_LEN: usize = 511;
const ADDRESS_TAG_MASK: usize = 0x00ff_ffff_ffff_ffff;

thread_local! {
    static DL_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(super) fn set_dl_error<T>(prefix: &str, result: Result<T>) -> Result<T> {
    // cite: bionic dlfcn.cpp#__bionic_format_dlerror
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            let mut message = format!("{prefix}{error}");
            message.truncate(ERROR_BUFFER_LEN);
            DL_ERROR.with(|slot| *slot.borrow_mut() = Some(message.clone()));
            Err(WinedroidError::new(message))
        }
    }
}

pub fn dlerror() -> Option<String> {
    // cite: bionic dlfcn.cpp#__loader_dlerror
    DL_ERROR.with(|slot| slot.borrow_mut().take())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlHandle {
    Default,
    Next,
    Module(u64),
}

impl DlHandle {
    pub fn from_raw(raw: u64) -> Self {
        // cite: bionic dlfcn.h#RTLD_DEFAULT, RTLD_NEXT
        match raw {
            0 => Self::Default,
            u64::MAX => Self::Next,
            other => Self::Module(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlInfo {
    pub path: String,
    pub base: usize,
    pub symbol: Option<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhdrInfo {
    pub addr: usize,
    pub name: String,
    pub phdr: usize,
    pub phnum: u16,
    pub adds: u64,
    pub subs: u64,
    pub tls_modid: usize,
    pub tls_data: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfo {
    pub path: String,
    pub soname: String,
    pub region: usize,
    pub region_len: usize,
    pub load_bias: usize,
    pub shift: u64,
    pub ref_count: usize,
    pub linked: bool,
    pub tls: Option<TlsSegment>,
    pub tls_module: u32,
    pub children: Vec<String>,
    pub phdr_count: u16,
    pub eh_frame_hdr_vaddr: u64,
}

fn find_containing_library(state: &State, address: usize) -> Option<ModuleId> {
    // cite: bionic linker.cpp#find_containing_library
    let address = address & ADDRESS_TAG_MASK;
    state.modules.values().find_map(|module| {
        let guest = module.guest.as_ref()?;
        let region = guest.mapped.region();
        if address < region || address - region >= guest.mapped.region_len() {
            return None;
        }
        let vaddr = address.wrapping_sub(guest.mapped.load_bias()) as u64;
        guest.elf.loads().any(|load| vaddr >= load.vaddr && vaddr - load.vaddr < load.memsz).then_some(module.id)
    })
}

impl Loader {
    pub fn dlsym(&self, handle: DlHandle, name: &str, caller: Option<usize>) -> Result<usize> {
        // cite: bionic dlfcn.cpp#__loader_dlsym
        let guard = self.lock.lock();
        let result = self.dlsym_locked(&guard, handle, name, caller);
        set_dl_error("", result)
    }

    fn dlsym_locked(&self, guard: &Guard, handle: DlHandle, name: &str, caller: Option<usize>) -> Result<usize> {
        // cite: bionic linker.cpp#do_dlsym, dlsym_linear_lookup, dlsym_handle_lookup_impl
        let state = guard.borrow();
        let caller_module = caller.and_then(|address| find_containing_library(&state, address));
        let found = match handle {
            DlHandle::Default | DlHandle::Module(MAIN_HANDLE) => self.linear_lookup(&state, name, caller_module, false)?,
            DlHandle::Next => self.linear_lookup(&state, name, caller_module, true)?,
            DlHandle::Module(raw) => {
                let id = state
                    .handles
                    .get(&raw)
                    .copied()
                    .ok_or_else(|| WinedroidError::new(format!("dlsym failed: invalid handle: {raw:#x}")))?;
                self.handle_lookup(&state, id, None, name)?
            }
        };
        let Some((id, resolved)) = found else {
            return Err(WinedroidError::new(format!("undefined symbol: {name}")));
        };
        match resolved.kind {
            STT_TLS => {
                if resolved.tls_module == 0 {
                    return Err(WinedroidError::new(format!("TLS symbol \"{name}\" in solib \"{}\" with no TLS segment", state.module(id).path)));
                }
                Ok(tls_modules::block_address(resolved.tls_module) + resolved.value as usize)
            }
            STT_GNU_IFUNC => Err(WinedroidError::new(format!(
                "STT_GNU_IFUNC symbol \"{name}\"; next missing primitive: STT_GNU_IFUNC symbol resolution"
            ))),
            _ => Ok(resolved.address),
        }
    }

    fn linear_lookup(&self, state: &State, name: &str, caller: Option<ModuleId>, next: bool) -> Result<Option<(ModuleId, super::relocate::Resolved)>> {
        let after = match (next, caller) {
            (true, None) => return Ok(None),
            (true, Some(caller)) => caller,
            (false, _) => 0,
        };
        for module in state.modules.values().filter(|module| module.id > after && module.rtld_flags & RTLD_GLOBAL != 0) {
            if let Some(resolved) = lookup_in_module(state, &*self.provider, module.id, name)? {
                return Ok(Some((module.id, resolved)));
            }
        }
        match caller {
            Some(caller) => self.handle_lookup(state, state.module(caller).group_root, next.then_some(caller), name),
            None => Ok(None),
        }
    }

    fn handle_lookup(&self, state: &State, root: ModuleId, skip_until: Option<ModuleId>, name: &str) -> Result<Option<(ModuleId, super::relocate::Resolved)>> {
        let mut skipping = skip_until.is_some();
        for id in state.local_group(root) {
            if skipping {
                skipping = Some(id) != skip_until;
                continue;
            }
            if let Some(resolved) = lookup_in_module(state, &*self.provider, id, name)? {
                return Ok(Some((id, resolved)));
            }
        }
        Ok(None)
    }

    pub fn dlvsym(&self, _handle: DlHandle, name: &str, version: &str) -> Result<usize> {
        // cite: bionic dlfcn.cpp#__loader_dlvsym
        set_dl_error(
            "",
            Err(WinedroidError::new(format!("dlvsym({name}, {version}) is not implemented; next missing primitive: dlvsym"))),
        )
    }

    pub fn android_dlopen_ext(&self, name: &str) -> Result<u64> {
        // cite: bionic dlfcn.cpp#__loader_android_dlopen_ext
        set_dl_error(
            "dlopen failed: ",
            Err(WinedroidError::new(format!("android_dlopen_ext({name}) is not implemented; next missing primitive: android_dlopen_ext"))),
        )
    }

    pub fn dladdr(&self, address: usize) -> Option<DlInfo> {
        // cite: bionic linker.cpp#do_dladdr
        let guard = self.lock.lock();
        let state = guard.borrow();
        let id = find_containing_library(&state, address)?;
        let module = state.module(id);
        let guest = module.guest.as_ref()?;
        let bias = guest.mapped.load_bias();
        let soaddr = (address & ADDRESS_TAG_MASK).wrapping_sub(bias) as u64;
        let symbol = guest.symbols.find_by_address(&guest.mapped, soaddr).ok().flatten().and_then(|sym| {
            let name = guest.symbols.name(&guest.mapped, &sym).ok()?;
            Some((name, bias.wrapping_add(sym.value as usize)))
        });
        Some(DlInfo { path: module.path.clone(), base: guest.mapped.region(), symbol })
    }

    pub fn dl_iterate_phdr(&self, callback: &mut dyn FnMut(&PhdrInfo) -> i32) -> i32 {
        // cite: bionic linker.cpp#do_dl_iterate_phdr
        let snapshot: Vec<(PhdrInfo, Option<Arc<GuestImage>>)> = {
            let guard = self.lock.lock();
            let state = guard.borrow();
            state
                .modules
                .values()
                .map(|module| {
                    let mut info = PhdrInfo {
                        addr: 0,
                        name: module.path.clone(),
                        phdr: 0,
                        phnum: 0,
                        adds: state.load_counter,
                        subs: state.unload_counter,
                        tls_modid: 0,
                        tls_data: 0,
                    };
                    if let Some(guest) = &module.guest {
                        info.addr = guest.mapped.load_bias();
                        info.phdr = guest.elf.phdr_vaddr().map_or(0, |vaddr| guest.mapped.address(vaddr));
                        info.phnum = guest.elf.phdrs.len() as u16;
                        info.tls_modid = guest.tls_module as usize;
                        info.tls_data = tls_modules::existing_block_address(guest.tls_module).unwrap_or(0);
                    }
                    (info, module.guest.clone())
                })
                .collect()
        };
        for (info, _keep_mapped) in &snapshot {
            let result = callback(info);
            if result != 0 {
                return result;
            }
        }
        0
    }

    pub fn module_images(&self) -> Vec<(ModuleId, Arc<GuestImage>, String)> {
        let guard = self.lock.lock();
        let state = guard.borrow();
        state
            .modules
            .values()
            .filter_map(|module| module.guest.clone().map(|guest| (module.id, guest, module.path.clone())))
            .collect()
    }

    pub fn module_info(&self, handle: u64) -> Option<ModuleInfo> {
        let guard = self.lock.lock();
        let state = guard.borrow();
        let module = state.module(*state.handles.get(&handle)?);
        let guest = module.guest.as_ref()?;
        Some(ModuleInfo {
            path: module.path.clone(),
            soname: module.soname.clone(),
            region: guest.mapped.region(),
            region_len: guest.mapped.region_len(),
            load_bias: guest.mapped.load_bias(),
            shift: guest.mapped.layout().shift,
            ref_count: module.ref_count,
            linked: module.linked,
            tls: guest.elf.tls(),
            tls_module: guest.tls_module,
            children: module.children.iter().map(|child| state.module(*child).soname.clone()).collect(),
            phdr_count: guest.elf.phdrs.len() as u16,
            eh_frame_hdr_vaddr: guest.elf.eh_frame_hdr().map_or(0, |header| header.vaddr),
        })
    }

    pub fn loaded_module_count(&self) -> usize {
        self.lock.lock().borrow().modules.len()
    }
}
