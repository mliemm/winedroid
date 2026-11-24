use super::guest_call::call_constructor;
use super::loader::{Guard, Loader};
use super::module::{ModuleId, State};

impl Loader {
    pub(super) fn call_constructors(&self, guard: &Guard, root: ModuleId) {
        // cite: bionic linker_soinfo.cpp#call_constructors, call_array
        let mut calls = Vec::new();
        collect_constructors(&mut guard.borrow_mut(), root, &mut calls);
        for address in calls {
            call_constructor(address, &self.args);
        }
    }
}

fn collect_constructors(state: &mut State, id: ModuleId, calls: &mut Vec<usize>) {
    // cite: bionic linker_soinfo.cpp#call_constructors
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

pub(super) fn destructor_addresses(state: &State, id: ModuleId) -> Vec<usize> {
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
