use crate::error::Result;

use super::guest_call::{call_constructor, call_init};
use super::loader::{Guard, Loader};
use super::module::{ModuleId, State};

impl Loader {
    pub(super) fn call_constructors(&self, guard: &Guard, root: ModuleId) -> Result<()> {
        // cite: bionic linker_soinfo.cpp#call_constructors, call_array
        let mut calls = Vec::new();
        collect_constructors(&mut guard.borrow_mut(), root, &mut calls);
        for call in calls {
            match call {
                Constructor::Init(address) => call_init(address)?,
                Constructor::InitArray(address) => call_constructor(address, &self.args)?,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Constructor {
    Init(usize),
    InitArray(usize),
}

fn collect_constructors(state: &mut State, id: ModuleId, calls: &mut Vec<Constructor>) {
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
        calls.push(Constructor::Init(guest.mapped.address(dynamic.init)));
    }
    for index in 0..dynamic.init_array.size / 8 {
        if let Ok(entry) = guest.mapped.read_u64(dynamic.init_array.vaddr + index * 8) {
            if entry != 0 && entry != u64::MAX {
                calls.push(Constructor::InitArray(entry as usize));
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::native::fixture_elf::{FixtureElf, PT_DYNAMIC};
    use crate::native::loader_test_support::{
        dlopen_without_constructors, fixture_dir, loader_for,
    };

    const DT_INIT: u64 = 12;
    const DT_FLAGS_1: u64 = 0x6fff_fffb;

    #[test]
    fn bionic_call_function_passes_no_arguments_to_dt_init_and_process_arguments_to_init_array() {
        let mut data = fs::read(fixture_dir("16k").join("libfx_order_child.so")).unwrap();
        let (init_vaddr, slot) = {
            let elf = FixtureElf::parse(&data);
            let dynamic = elf
                .segments
                .iter()
                .find(|segment| segment.kind == PT_DYNAMIC)
                .unwrap()
                .offset as usize;
            let index = elf
                .dynamic
                .iter()
                .position(|entry| entry.0 == DT_FLAGS_1)
                .unwrap();
            (elf.export_addresses["fx_write_x18"], dynamic + index * 16)
        };
        data[slot..slot + 8].copy_from_slice(&DT_INIT.to_le_bytes());
        data[slot + 8..slot + 16].copy_from_slice(&init_vaddr.to_le_bytes());
        let dir = std::env::temp_dir().join(format!("winedroid-init-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("libfx_order_child.so"), data).unwrap();
        let loader = loader_for(vec![dir.clone()]);
        let handle = dlopen_without_constructors(&loader, "libfx_order_child.so").unwrap();

        let guard = loader.lock.lock();
        let root = *guard.borrow().handles.get(&handle).unwrap();
        let bias = guard
            .borrow()
            .module(root)
            .guest
            .as_ref()
            .unwrap()
            .mapped
            .load_bias();
        let mut calls = Vec::new();
        collect_constructors(&mut guard.borrow_mut(), root, &mut calls);

        assert_eq!(calls.len(), 4, "{calls:?}");
        assert_eq!(calls[0], Constructor::Init(bias + init_vaddr as usize));
        assert!(
            calls[1..]
                .iter()
                .all(|call| matches!(call, Constructor::InitArray(_))),
            "{calls:?}"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
