use std::collections::BTreeSet;
use std::sync::Mutex;

use crate::error::Result;

use super::module::{ModuleId, State};
use super::relocate::{Resolved, SymbolResolution};
use super::symbol_table::{Sym, STB_WEAK};

const STT_OBJECT: u8 = 1;
const STT_FUNC: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    Function,
    Data,
}

pub trait SystemLibraryProvider: Send + Sync {
    fn export(&self, soname: &str, name: &str) -> Option<usize>;
    fn unresolved(&self, importer: &str, name: &str, kind: ImportKind) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingDependency {
    pub library: String,
    pub needed_by: String,
}

#[derive(Debug, Clone, Default)]
pub struct LoadReport {
    pub missing_dependencies: Vec<MissingDependency>,
    pub unresolved_imports: BTreeSet<(String, String)>,
    pub x18_writes: Vec<(String, usize)>,
}

pub fn lookup_in_module(state: &State, provider: &dyn SystemLibraryProvider, id: ModuleId, name: &str) -> Result<Option<Resolved>> {
    // cite: bionic linker_soinfo.cpp#soinfo_do_lookup_impl
    let module = state.module(id);
    let Some(guest) = &module.guest else {
        return Ok(provider.export(&module.soname, name).map(|address| Resolved { address, kind: STT_FUNC, value: 0, tls_module: 0 }));
    };
    let Some((_, sym)) = guest.symbols.lookup(&guest.mapped, name)? else {
        return Ok(None);
    };
    Ok(Some(Resolved {
        address: guest.mapped.load_bias().wrapping_add(sym.value as usize),
        kind: sym.kind(),
        value: sym.value,
        tls_module: guest.tls_module,
    }))
}

pub struct LinkResolution<'a> {
    pub state: &'a State,
    pub provider: &'a dyn SystemLibraryProvider,
    pub importer: ModuleId,
    pub group: &'a [ModuleId],
    pub report: &'a Mutex<LoadReport>,
}

impl SymbolResolution for LinkResolution<'_> {
    fn resolve(&self, name: &str, sym: &Sym) -> Result<Option<Resolved>> {
        // cite: bionic linker.cpp#find_libraries
        let importer = self.state.module(self.importer);
        let symbolic = importer.guest.as_ref().is_some_and(|guest| guest.dynamic.has_symbolic);
        let order = symbolic.then_some(self.importer).into_iter().chain(self.state.global_group()).chain(self.group.iter().copied());
        for id in order {
            if let Some(resolved) = lookup_in_module(self.state, self.provider, id, name)? {
                return Ok(Some(resolved));
            }
        }
        if sym.bind() == STB_WEAK {
            return Ok(None);
        }
        let kind = if sym.kind() == STT_OBJECT { ImportKind::Data } else { ImportKind::Function };
        let fresh = self.report.lock().expect("load report").unresolved_imports.insert((importer.path.clone(), name.to_string()));
        if fresh {
            crate::missing::record_from_error_message(&format!(
                "unresolved import {name} in {}; next missing primitive: unresolved import {name}",
                importer.path
            ));
        }
        Ok(Some(Resolved {
            address: self.provider.unresolved(&importer.path, name, kind),
            kind: if kind == ImportKind::Data { STT_OBJECT } else { STT_FUNC },
            value: 0,
            tls_module: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::loader_test_support::{host_function_address, unresolved_import_address, TestProvider};

    const IMPORTER: &str = "/guest/libimporter.so";
    const STB_GLOBAL: u8 = 1;

    fn symbol(bind: u8, kind: u8) -> Sym {
        Sym { name: 0, info: (bind << 4) | kind, shndx: 0, value: 0, size: 0 }
    }

    fn resolve(provider: &TestProvider, name: &str, sym: &Sym) -> (Option<Resolved>, LoadReport) {
        let mut state = State::with_system_modules();
        let importer = state.allocate(IMPORTER.to_string(), "libimporter.so".to_string(), 0, None, None);
        let report = Mutex::new(LoadReport::default());
        let group = [importer];
        let resolution = LinkResolution { state: &state, provider, importer, group: &group, report: &report };
        let resolved = resolution.resolve(name, sym).unwrap();
        (resolved, report.into_inner().unwrap())
    }

    #[test]
    fn bionic_lookup_symbol_failure_hands_an_unresolved_function_import_to_the_provider() {
        let provider = TestProvider::new();
        let (resolved, report) = resolve(&provider, "absent_function", &symbol(STB_GLOBAL, STT_FUNC));

        let resolved = resolved.unwrap();
        assert_eq!(resolved.address, unresolved_import_address());
        assert_eq!(resolved.kind, STT_FUNC);
        assert_eq!(*provider.unresolved_calls.lock().unwrap(), vec![(IMPORTER.to_string(), "absent_function".to_string(), ImportKind::Function)]);
        assert!(report.unresolved_imports.contains(&(IMPORTER.to_string(), "absent_function".to_string())));
    }

    #[test]
    fn bionic_lookup_symbol_failure_hands_an_unresolved_data_import_to_the_provider_as_data() {
        let provider = TestProvider::new();
        let (resolved, _) = resolve(&provider, "absent_object", &symbol(STB_GLOBAL, STT_OBJECT));

        assert_eq!(resolved.unwrap().kind, STT_OBJECT);
        assert_eq!(provider.unresolved_calls.lock().unwrap()[0].2, ImportKind::Data);
    }

    #[test]
    fn bionic_process_relocation_general_leaves_an_unresolved_weak_import_null_without_asking_the_provider() {
        let provider = TestProvider::new();
        let (resolved, report) = resolve(&provider, "absent_weak", &symbol(STB_WEAK, STT_FUNC));

        assert!(resolved.is_none());
        assert!(provider.unresolved_calls.lock().unwrap().is_empty());
        assert!(report.unresolved_imports.is_empty());
    }

    #[test]
    fn bionic_global_group_serves_a_symbol_from_the_system_module_without_the_unresolved_path() {
        let provider = TestProvider::new();
        let (resolved, report) = resolve(&provider, "strlen", &symbol(STB_GLOBAL, STT_FUNC));

        assert_eq!(resolved.unwrap().address, host_function_address("strlen"));
        assert!(provider.unresolved_calls.lock().unwrap().is_empty());
        assert!(report.unresolved_imports.is_empty());
    }
}
