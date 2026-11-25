use crate::error::{Result, WinedroidError};

use super::dl_api::set_dl_error;
use super::guest_call::call_destructor;
use super::init_fini::destructor_addresses;
use super::loader::{Guard, Loader};
use super::module::ModuleId;

impl Loader {
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
            if let Err(error) = call_destructor(address) {
                eprintln!("destructor at {address:#x} failed: {error}");
            }
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
