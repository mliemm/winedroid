use std::cell::RefCell;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use crate::error::{Result, WinedroidError};
use crate::external_storage::ExternalStoragePolicy;
use crate::guest_app_paths;
use crate::guest_filesystem::GuestFilesystem;

const NATIVE_ABI: &str = "arm64-v8a";

pub struct AppCodeMount {
    pub guest: String,
    pub host: PathBuf,
}

pub struct GuestContext {
    pub package: String,
    pub filesystem: GuestFilesystem,
    pub policy: Arc<RwLock<ExternalStoragePolicy>>,
    pub prefix: PathBuf,
    pub temp_dir: PathBuf,
    pub app_code: Vec<AppCodeMount>,
}

static CONTEXT: OnceLock<Arc<GuestContext>> = OnceLock::new();

thread_local! {
    static OVERRIDE: RefCell<Option<Arc<GuestContext>>> = const { RefCell::new(None) };
}

impl GuestContext {
    pub fn open(
        prefix: &Path,
        package: &str,
        policy: Arc<RwLock<ExternalStoragePolicy>>,
    ) -> Result<Self> {
        let app = prefix.join("apps").join(package);
        let filesystem = GuestFilesystem::new(
            package,
            app.join("data"),
            app.join("data_de"),
            prefix.join("storage"),
        )?;
        let temp_dir = prefix.join("tmp");
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&temp_dir)
            .map_err(|error| {
                WinedroidError::with_source(
                    format!("cannot create guest temp directory {}", temp_dir.display()),
                    error,
                )
            })?;
        // cite: frameworks-base ApplicationInfo.java#sourceDir, nativeLibraryDir
        let app_code = vec![
            AppCodeMount {
                guest: guest_app_paths::base_apk(package),
                host: app.join("source.apk"),
            },
            AppCodeMount {
                guest: guest_app_paths::native_library_dir(package),
                host: app.join("native").join(NATIVE_ABI),
            },
        ];
        Ok(Self {
            package: package.to_string(),
            filesystem,
            policy,
            prefix: prefix.to_path_buf(),
            temp_dir,
            app_code,
        })
    }
}

pub fn install(context: GuestContext) -> Result<()> {
    CONTEXT.set(Arc::new(context)).map_err(|_| WinedroidError::new("guest context is already installed; next missing primitive: replacing the guest context"))
}

pub fn current() -> Option<Arc<GuestContext>> {
    OVERRIDE
        .with(|slot| slot.borrow().clone())
        .or_else(|| CONTEXT.get().cloned())
}

pub fn use_on_this_thread(context: Arc<GuestContext>) {
    OVERRIDE.with(|slot| *slot.borrow_mut() = Some(context));
}
