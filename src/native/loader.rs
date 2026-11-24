use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::error::{Result, WinedroidError};

use super::dl_api::set_dl_error;
use super::dl_lock::{RecursiveGuard, RecursiveLock};
use super::dynamic_table::DynamicInfo;
use super::elf_image::ElfImage;
use super::guest_call::{call_constructor, call_destructor, GuestProcessArgs};
use super::image_map::MappedImage;
use super::library_search::{expand_runpath, open_library};
use super::link_resolution::{LinkResolution, LoadReport, MissingDependency, SystemLibraryProvider};
use super::module::{GuestImage, ModuleId, State, RTLD_FLAGS_MASK, RTLD_NOLOAD};
use super::relocate::{relocate_all, RelocationContext};
use super::symbol_table::SymbolTable;
use super::system_libraries::is_system_soname;
use super::tls_modules;

pub const MAIN_HANDLE: u64 = 1;

pub trait ImagePatcher: Send + Sync {
    fn patch(&self, image: &MappedImage, elf: &ElfImage) -> Result<()>;
}

pub struct NoPatcher;

impl ImagePatcher for NoPatcher {
    fn patch(&self, _image: &MappedImage, _elf: &ElfImage) -> Result<()> {
        Ok(())
    }
}

pub struct Loader {
    pub(super) lock: RecursiveLock<RefCell<State>>,
    pub(super) provider: Box<dyn SystemLibraryProvider>,
    patcher: Box<dyn ImagePatcher>,
    library_dirs: Vec<PathBuf>,
    strict: bool,
    args: GuestProcessArgs,
    pub(super) report: Mutex<LoadReport>,
}

pub(super) type Guard<'a> = RecursiveGuard<'a, RefCell<State>>;

#[derive(Clone)]
struct Task {
    name: String,
    needed_by: Option<ModuleId>,
}

impl Loader {
    pub fn new(library_dirs: Vec<PathBuf>, provider: Box<dyn SystemLibraryProvider>) -> Self {
        Self {
            lock: RecursiveLock::new(RefCell::new(State::with_system_modules())),
            provider,
            patcher: Box::new(NoPatcher),
            library_dirs,
            strict: crate::missing::strict_enabled(),
            args: GuestProcessArgs::new(),
            report: Mutex::new(LoadReport::default()),
        }
    }

    pub fn with_patcher(mut self, patcher: Box<dyn ImagePatcher>) -> Self {
        self.patcher = patcher;
        self
    }

    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    pub fn report(&self) -> LoadReport {
        self.report.lock().expect("load report").clone()
    }

    pub fn dlopen(&self, name: Option<&str>, flags: i32, caller: Option<usize>) -> Result<u64> {
        // cite: bionic dlfcn.cpp#__loader_dlopen
        let guard = self.lock.lock();
        let result = self.dlopen_locked(&guard, name, flags, caller);
        set_dl_error("dlopen failed: ", result)
    }

    fn dlopen_locked(&self, guard: &Guard, name: Option<&str>, flags: i32, _caller: Option<usize>) -> Result<u64> {
        // cite: bionic linker.cpp#do_dlopen, find_library
        if flags & !RTLD_FLAGS_MASK != 0 {
            return Err(WinedroidError::new(format!("invalid flags to dlopen: {flags:x}")));
        }
        let Some(name) = name else {
            return Ok(MAIN_HANDLE);
        };
        let root = self.find_libraries(guard, name, flags)?;
        let handle = {
            let mut state = guard.borrow_mut();
            let group_root = state.module(root).group_root;
            let module = state.module_mut(group_root);
            module.ref_count = module.ref_count.wrapping_add(1);
            state.module(root).handle
        };
        if flags & RTLD_NOLOAD == 0 {
            self.call_constructors(guard, root);
        }
        Ok(handle)
    }

    fn find_libraries(&self, guard: &Guard, name: &str, flags: i32) -> Result<ModuleId> {
        let mut created = Vec::new();
        let result = self.find_libraries_inner(guard, name, flags, &mut created);
        if result.is_err() {
            guard.borrow_mut().remove_modules(&created);
        }
        result
    }

    fn find_libraries_inner(&self, guard: &Guard, name: &str, flags: i32, created: &mut Vec<ModuleId>) -> Result<ModuleId> {
        // cite: bionic linker.cpp#find_libraries
        let mut tasks = vec![Task { name: name.to_string(), needed_by: None }];
        let mut found: Vec<Option<ModuleId>> = Vec::new();
        let mut index = 0;
        while index < tasks.len() {
            let task = tasks[index].clone();
            let loaded = self.find_or_load(guard, &task, flags, created)?;
            match loaded {
                None if index == 0 => return Err(WinedroidError::new(format!("library \"{}\" not found", task.name))),
                None => {
                    let needed_by = guard.borrow().module(task.needed_by.expect("dependency task")).path.clone();
                    if self.strict {
                        return Err(WinedroidError::new(format!(
                            "library \"{}\" not found: needed by {needed_by} in namespace clns-1",
                            task.name
                        )));
                    }
                    let message = format!("library \"{}\" not found: needed by {needed_by}", task.name);
                    crate::missing::record_from_error_message(&format!("{message}; next missing primitive: library \"{}\" not found", task.name));
                    self.report
                        .lock()
                        .expect("load report")
                        .missing_dependencies
                        .push(MissingDependency { library: task.name.clone(), needed_by });
                    found.push(None);
                }
                Some((id, newly_loaded)) => {
                    let mut state = guard.borrow_mut();
                    if let Some(parent) = task.needed_by {
                        state.add_child(parent, id);
                    }
                    if newly_loaded {
                        if let Some(guest) = state.module(id).guest.clone() {
                            tasks.extend(guest.dynamic.needed.iter().map(|needed| Task { name: needed.clone(), needed_by: Some(id) }));
                        }
                    }
                    found.push(Some(id));
                }
            }
            index += 1;
        }
        let root = found[0].expect("root task resolved");
        if flags & RTLD_NOLOAD != 0 {
            return Ok(root);
        }
        self.link_group(guard, root)?;
        let mut state = guard.borrow_mut();
        for (task, id) in tasks.iter().zip(&found) {
            let (Some(needed_by), Some(id)) = (task.needed_by, id) else {
                continue;
            };
            let dependency_root = state.module(*id).group_root;
            if state.module(needed_by).group_root != dependency_root {
                let module = state.module_mut(dependency_root);
                module.ref_count = module.ref_count.wrapping_add(1);
            }
        }
        Ok(root)
    }

    fn find_or_load(&self, guard: &Guard, task: &Task, flags: i32, created: &mut Vec<ModuleId>) -> Result<Option<(ModuleId, bool)>> {
        // cite: bionic linker.cpp#find_library_internal, load_library
        if let Some(id) = guard.borrow().find_by_soname(&task.name) {
            return Ok(Some((id, false)));
        }
        let runpath = match task.needed_by {
            Some(parent) => {
                let state = guard.borrow();
                let module = state.module(parent);
                module
                    .guest
                    .as_ref()
                    .and_then(|guest| guest.dynamic.runpath.as_deref())
                    .map(|runpath| expand_runpath(runpath, &module.path))
                    .unwrap_or_default()
            }
            None => Vec::new(),
        };
        let Some(opened) = open_library(&task.name, &self.library_dirs, &runpath) else {
            let basename = task.name.rsplit('/').next().unwrap_or(&task.name);
            if task.name.contains('/') && is_system_soname(basename) {
                return Ok(guard.borrow().find_by_soname(basename).map(|id| (id, false)));
            }
            return Ok(None);
        };
        if let Some(id) = guard.borrow().find_by_identity(opened.identity) {
            return Ok(Some((id, false)));
        }
        if flags & RTLD_NOLOAD != 0 {
            return Err(WinedroidError::new(format!("library \"{}\" wasn't loaded and RTLD_NOLOAD prevented it", task.name)));
        }
        let path = opened.path.to_string_lossy().into_owned();
        let data = fs::read(&opened.path)?;
        let guest = Self::load_image(&path, &data)?;
        let mut state = guard.borrow_mut();
        let soname = guest.dynamic.soname.clone().unwrap_or_default();
        let flags_1 = guest.dynamic.flags_1;
        let id = state.allocate(path, soname, flags & RTLD_FLAGS_MASK, Some(opened.identity), Some(Arc::new(guest)));
        state.set_dt_flags_1(id, flags_1);
        created.push(id);
        Ok(Some((id, true)))
    }

    pub(super) fn load_image(path: &str, data: &[u8]) -> Result<GuestImage> {
        // cite: bionic linker_phdr.cpp#ElfReader
        let elf = ElfImage::parse(path, data)?;
        let mapped = MappedImage::map(path, data, &elf)?;
        let dynamic_header = elf.dynamic();
        let dynamic = DynamicInfo::read(path, &mapped, dynamic_header.vaddr, dynamic_header.filesz)?;
        let symbols = SymbolTable::new(path, &mapped, &dynamic)?;
        let tls_module = elf.tls().map_or(0, |segment| {
            let init_address = mapped.address(segment.init_vaddr);
            tls_modules::register(segment, init_address)
        });
        Ok(GuestImage { elf, mapped, dynamic, symbols, tls_module })
    }

    fn link_group(&self, guard: &Guard, root: ModuleId) -> Result<()> {
        // cite: bionic linker.cpp#find_libraries, soinfo::link_image
        let group = guard.borrow().local_group(root);
        for id in &group {
            let (guest, path) = {
                let state = guard.borrow();
                let module = state.module(*id);
                if module.linked {
                    continue;
                }
                match &module.guest {
                    Some(guest) => (guest.clone(), module.path.clone()),
                    None => continue,
                }
            };
            let mut descriptors = Vec::new();
            {
                let state = guard.borrow();
                let resolution = LinkResolution { state: &state, provider: &*self.provider, importer: *id, group: &group, report: &self.report };
                let mut context = RelocationContext {
                    name: &path,
                    image: &guest.mapped,
                    symbols: &guest.symbols,
                    own_tls_module: guest.tls_module,
                    resolution: &resolution,
                    descriptors: &mut descriptors,
                };
                relocate_all(&mut context, &guest.dynamic)?;
            }
            self.patcher.patch(&guest.mapped, &guest.elf)?;
            guest.mapped.protect_final(&guest.elf)?;
            guest.mapped.protect_relro(&guest.elf)?;
            guard.borrow_mut().module_mut(*id).descriptors = descriptors;
        }
        let mut state = guard.borrow_mut();
        for id in group {
            let module = state.module_mut(id);
            if !module.linked {
                module.linked = true;
                module.group_root = root;
                state.load_counter += 1;
            }
        }
        Ok(())
    }

    fn call_constructors(&self, guard: &Guard, root: ModuleId) {
        // cite: bionic linker_soinfo.cpp#call_constructors, call_array
        let mut calls = Vec::new();
        collect_constructors(&mut guard.borrow_mut(), root, &mut calls);
        for address in calls {
            call_constructor(address, &self.args);
        }
    }

    pub fn dlclose(&self, handle: u64) -> Result<()> {
        // cite: bionic linker.cpp#do_dlclose
        let guard = self.lock.lock();
        let id = guard.borrow().handles.get(&handle).copied();
        let result = match id {
            Some(id) => {
                self.unload(&guard, id);
                Ok(())
            }
            None => Err(WinedroidError::new(format!("invalid handle: {handle:#x}"))),
        };
        set_dl_error("dlclose failed: ", result)
    }

    fn unload(&self, guard: &Guard, id: ModuleId) {
        // cite: bionic linker.cpp#soinfo_unload
        let root = {
            let mut state = guard.borrow_mut();
            let module = state.module(id);
            let root = if module.linked { module.group_root } else { id };
            if module.linked {
                let root_module = state.module_mut(root);
                root_module.ref_count = root_module.ref_count.wrapping_sub(1);
                if root_module.ref_count > 0 {
                    return;
                }
            }
            root
        };
        self.unload_group(guard, root);
    }

    fn unload_group(&self, guard: &Guard, root: ModuleId) {
        // cite: bionic linker.cpp#soinfo_unload_impl
        let (local, external, destructors) = {
            let mut state = guard.borrow_mut();
            if !state.module(root).can_unload() {
                return;
            }
            let was_linked = state.module(root).linked;
            let mut unload_list = vec![root];
            let mut local: Vec<ModuleId> = Vec::new();
            let mut external: Vec<ModuleId> = Vec::new();
            while !unload_list.is_empty() {
                let id = unload_list.remove(0);
                if local.contains(&id) {
                    continue;
                }
                local.push(id);
                let children = std::mem::take(&mut state.module_mut(id).children);
                for child in children {
                    state.module_mut(child).parents.retain(|parent| *parent != id);
                    if local.contains(&child) {
                        continue;
                    }
                    let module = state.module(child);
                    if module.linked && module.group_root != root {
                        external.push(child);
                    } else if module.parents.is_empty() {
                        unload_list.push(child);
                    }
                }
            }
            if !was_linked {
                external.clear();
            }
            let destructors: Vec<usize> = local.iter().flat_map(|id| destructor_addresses(&state, *id)).collect();
            (local, external, destructors)
        };
        for address in destructors {
            call_destructor(address);
        }
        {
            let mut state = guard.borrow_mut();
            state.unload_counter += local.len() as u64;
            state.remove_modules(&local);
        }
        for child in external {
            self.unload(guard, child);
        }
    }
}

fn collect_constructors(state: &mut State, id: ModuleId, calls: &mut Vec<usize>) {
    if state.module(id).constructors_called {
        return;
    }
    state.module_mut(id).constructors_called = true;
    for child in state.module(id).children.clone() {
        collect_constructors(state, child, calls);
    }
    let Some(guest) = state.module(id).guest.clone() else {
        return;
    };
    let dynamic = &guest.dynamic;
    if dynamic.init != 0 && dynamic.init != u64::MAX {
        calls.push(guest.mapped.address(dynamic.init));
    }
    for index in 0..dynamic.init_array.size / 8 {
        if let Ok(entry) = guest.mapped.read_u64(dynamic.init_array.vaddr + index * 8) {
            if entry != 0 && entry != u64::MAX {
                calls.push(entry as usize);
            }
        }
    }
}

fn destructor_addresses(state: &State, id: ModuleId) -> Vec<usize> {
    // cite: bionic linker_soinfo.cpp#call_destructors
    let module = state.module(id);
    let Some(guest) = module.guest.as_ref().filter(|_| module.constructors_called) else {
        return Vec::new();
    };
    let dynamic = &guest.dynamic;
    let mut addresses = Vec::new();
    for index in (0..dynamic.fini_array.size / 8).rev() {
        if let Ok(entry) = guest.mapped.read_u64(dynamic.fini_array.vaddr + index * 8) {
            if entry != 0 && entry != u64::MAX {
                addresses.push(entry as usize);
            }
        }
    }
    if dynamic.fini != 0 && dynamic.fini != u64::MAX {
        addresses.push(guest.mapped.address(dynamic.fini));
    }
    addresses
}

impl Loader {
    pub fn dlopen_without_constructors(&self, name: &str) -> Result<u64> {
        let guard = self.lock.lock();
        let result = self.find_libraries(&guard, name, 2).map(|root| guard.borrow().module(root).handle);
        set_dl_error("dlopen failed: ", result)
    }
}
