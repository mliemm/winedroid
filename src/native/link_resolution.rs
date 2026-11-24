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
