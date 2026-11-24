use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::Arc;

use super::dynamic_table::{DynamicInfo, DF_1_GLOBAL, DF_1_NODELETE};
use super::elf_image::ElfImage;
use super::image_map::MappedImage;
use super::symbol_table::SymbolTable;
use super::system_libraries::{system_library_path, SYSTEM_SONAMES};
use super::tls_modules::TlsDescArg;

pub const RTLD_NOW: i32 = 2;
pub const RTLD_NOLOAD: i32 = 4;
pub const RTLD_GLOBAL: i32 = 0x100;
pub const RTLD_NODELETE: i32 = 0x1000;
pub const RTLD_FLAGS_MASK: i32 = 1 | RTLD_NOW | RTLD_NOLOAD | RTLD_GLOBAL | RTLD_NODELETE;

pub type ModuleId = u32;

pub struct GuestImage {
    pub elf: ElfImage,
    pub mapped: MappedImage,
    pub dynamic: DynamicInfo,
    pub symbols: SymbolTable,
    pub tls_module: u32,
}

pub struct Module {
    pub id: ModuleId,
    pub path: String,
    pub soname: String,
    pub rtld_flags: i32,
    pub dt_flags_1: u64,
    pub children: Vec<ModuleId>,
    pub parents: Vec<ModuleId>,
    pub group_root: ModuleId,
    pub ref_count: usize,
    pub linked: bool,
    pub constructors_called: bool,
    pub identity: Option<(u64, u64)>,
    pub handle: u64,
    pub guest: Option<Arc<GuestImage>>,
    pub descriptors: Vec<Box<TlsDescArg>>,
}

impl Module {
    pub fn is_system(&self) -> bool {
        self.guest.is_none()
    }

    pub fn can_unload(&self) -> bool {
        // cite: bionic linker_soinfo.cpp#can_unload
        !self.linked || self.rtld_flags & (RTLD_NODELETE | RTLD_GLOBAL) == 0
    }
}

#[derive(Default)]
pub struct State {
    pub modules: BTreeMap<ModuleId, Module>,
    pub handles: HashMap<u64, ModuleId>,
    pub retained: Vec<Module>,
    pub next_id: ModuleId,
    pub load_counter: u64,
    pub unload_counter: u64,
}

fn generate_handle(state: &State) -> u64 {
    // cite: bionic linker_soinfo.cpp#generate_handle
    loop {
        let value = (u64::from(unsafe { libc::arc4random() }) << 32 | u64::from(unsafe { libc::arc4random() })) | 1;
        if value != u64::MAX && value != 1 && !state.handles.contains_key(&value) {
            return value;
        }
    }
}

impl State {
    pub fn with_system_modules() -> Self {
        let mut state = Self { next_id: 1, ..Self::default() };
        for soname in SYSTEM_SONAMES {
            let id = state.allocate(system_library_path(soname), soname.to_string(), RTLD_GLOBAL | RTLD_NODELETE, None, None);
            let module = state.modules.get_mut(&id).expect("just allocated");
            module.linked = true;
            module.constructors_called = true;
            module.group_root = id;
        }
        state
    }

    pub fn allocate(&mut self, path: String, soname: String, rtld_flags: i32, identity: Option<(u64, u64)>, guest: Option<Arc<GuestImage>>) -> ModuleId {
        // cite: bionic linker.cpp#soinfo_alloc
        let id = self.next_id;
        self.next_id += 1;
        let handle = generate_handle(self);
        self.handles.insert(handle, id);
        self.modules.insert(
            id,
            Module {
                id,
                path,
                soname,
                rtld_flags,
                dt_flags_1: 0,
                children: Vec::new(),
                parents: Vec::new(),
                group_root: id,
                ref_count: 0,
                linked: false,
                constructors_called: false,
                identity,
                handle,
                guest,
                descriptors: Vec::new(),
            },
        );
        id
    }

    pub fn module(&self, id: ModuleId) -> &Module {
        &self.modules[&id]
    }

    pub fn module_mut(&mut self, id: ModuleId) -> &mut Module {
        self.modules.get_mut(&id).expect("module id is live")
    }

    pub fn find_by_soname(&self, name: &str) -> Option<ModuleId> {
        // cite: bionic linker.cpp#find_loaded_library_by_soname
        if name.contains('/') {
            return None;
        }
        self.modules.values().find(|module| !module.soname.is_empty() && module.soname == name).map(|module| module.id)
    }

    pub fn find_by_identity(&self, identity: (u64, u64)) -> Option<ModuleId> {
        // cite: bionic linker.cpp#find_loaded_library_by_inode
        self.modules.values().find(|module| module.identity == Some(identity)).map(|module| module.id)
    }

    pub fn add_child(&mut self, parent: ModuleId, child: ModuleId) {
        // cite: bionic linker_soinfo.cpp#add_child
        self.module_mut(parent).children.push(child);
        self.module_mut(child).parents.push(parent);
    }

    pub fn local_group(&self, root: ModuleId) -> Vec<ModuleId> {
        // cite: bionic linker.cpp#walk_dependencies_tree
        let mut order = Vec::new();
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([root]);
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            order.push(id);
            queue.extend(self.module(id).children.iter().copied());
        }
        order
    }

    pub fn global_group(&self) -> Vec<ModuleId> {
        // cite: bionic linker_namespaces.cpp#get_global_group
        let system = self.modules.values().filter(|module| module.is_system());
        let flagged = self.modules.values().filter(|module| module.guest.is_some() && module.dt_flags_1 & DF_1_GLOBAL != 0);
        system.chain(flagged).map(|module| module.id).collect()
    }

    pub fn remove_modules(&mut self, ids: &[ModuleId]) {
        for id in ids {
            if let Some(module) = self.modules.remove(id) {
                self.handles.remove(&module.handle);
                if let Some(guest) = &module.guest {
                    if guest.tls_module != 0 {
                        self.retained.push(module);
                    }
                }
            }
        }
        for module in self.modules.values_mut() {
            module.children.retain(|child| !ids.contains(child));
            module.parents.retain(|parent| !ids.contains(parent));
        }
    }

    pub fn set_dt_flags_1(&mut self, id: ModuleId, flags: u64) {
        // cite: bionic linker_soinfo.cpp#set_dt_flags_1
        let module = self.module_mut(id);
        module.dt_flags_1 = flags;
        if flags & DF_1_GLOBAL != 0 {
            module.rtld_flags |= RTLD_GLOBAL;
        }
        if flags & DF_1_NODELETE != 0 {
            module.rtld_flags |= RTLD_NODELETE;
        }
    }
}
