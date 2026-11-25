use std::path::PathBuf;
use std::sync::OnceLock;

use crate::error::{Result, WinedroidError};

use super::bridge::native_provider;
use super::guest_call::{call_guest, GuestResult};
use super::loader::Loader;

pub struct NativeRuntime {
    loader: Loader,
    library_dirs: Vec<PathBuf>,
}

static RUNTIME: OnceLock<NativeRuntime> = OnceLock::new();

impl NativeRuntime {
    pub fn initialize(library_dirs: Vec<PathBuf>) -> Result<&'static NativeRuntime> {
        let runtime = RUNTIME.get_or_init(|| NativeRuntime {
            loader: Loader::new(library_dirs.clone(), Box::new(native_provider())),
            library_dirs: library_dirs.clone(),
        });
        if runtime.library_dirs != library_dirs {
            return Err(WinedroidError::new(format!(
                "native runtime is already initialized with library directories {:?}, not {library_dirs:?}; next missing primitive: reinitializing the native runtime",
                runtime.library_dirs
            )));
        }
        Ok(runtime)
    }

    pub fn current() -> Option<&'static NativeRuntime> {
        RUNTIME.get()
    }

    pub fn library_dirs(&self) -> &[PathBuf] {
        &self.library_dirs
    }

    pub fn loader(&self) -> &Loader {
        &self.loader
    }

    pub fn call_guest(&self, address: usize, args: &[u64]) -> Result<GuestResult> {
        call_guest(address, args)
    }
}
