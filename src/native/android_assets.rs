use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::apk::ApkArchive;

use super::bionic::guest_mem::{cstr_bytes, write_bytes};
use super::bridge::{register_handler_in, CallContext};

const LIBANDROID: &str = "libandroid.so";

struct Asset {
    data: Vec<u8>,
    position: usize,
}

static APK_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static ASSETS: OnceLock<Mutex<HashMap<u64, Asset>>> = OnceLock::new();
static NEXT_ASSET: AtomicU64 = AtomicU64::new(1);

fn apk_path_slot() -> &'static Mutex<Option<PathBuf>> {
    APK_PATH.get_or_init(|| Mutex::new(None))
}

fn assets() -> &'static Mutex<HashMap<u64, Asset>> {
    ASSETS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn set_apk_path(path: &Path) {
    *apk_path_slot().lock().expect("APK path slot") = Some(path.to_path_buf());
}

// cite: frameworks-base AssetManager2.cpp#AssetManager2::Open
// cite: frameworks-base AssetsProvider.cpp#ZipAssetsProvider::OpenInternal
fn open_asset(name: &str) -> Option<Vec<u8>> {
    let path = apk_path_slot().lock().expect("APK path slot").clone()?;
    let mut apk = ApkArchive::open(&path).ok()?;

    apk.read_optional_file(&format!("assets/{name}")).ok()?
}

pub(super) fn register() {
    // cite: frameworks-base asset_manager.cpp#AAssetManager_fromJava
    register_handler_in(LIBANDROID, "AAssetManager_fromJava", |context| {
        let manager = context.arg_ptr(1);
        context.ret_ptr(usize::from(manager != 0));
    });

    register_handler_in(LIBANDROID, "AAssetManager_open", |context| {
        let name_pointer = context.arg_ptr(1);
        if name_pointer == 0 {
            context.ret_ptr(0);
            return;
        }
        let name = String::from_utf8_lossy(cstr_bytes(name_pointer)).into_owned();
        let name = name.strip_prefix("assets/").unwrap_or(&name);
        let Some(data) = open_asset(name) else {
            context.ret_ptr(0);
            return;
        };
        let id = NEXT_ASSET.fetch_add(1, Ordering::Relaxed);
        assets()
            .lock()
            .expect("asset table")
            .insert(id, Asset { data, position: 0 });
        context.ret_ptr(id as usize);
    });

    register_handler_in(LIBANDROID, "AAsset_getLength", |context| {
        let id = context.arg_ptr(0) as u64;
        let length = assets()
            .lock()
            .expect("asset table")
            .get(&id)
            .map(|asset| asset.data.len())
            .unwrap_or(0);
        context.ret_i64(i64::try_from(length).unwrap_or(i64::MAX));
    });

    register_handler_in(LIBANDROID, "AAsset_getRemainingLength", |context| {
        let id = context.arg_ptr(0) as u64;
        let length = assets()
            .lock()
            .expect("asset table")
            .get(&id)
            .map(|asset| asset.data.len().saturating_sub(asset.position))
            .unwrap_or(0);
        context.ret_i64(i64::try_from(length).unwrap_or(i64::MAX));
    });

    // cite: frameworks-base asset_manager.cpp#AAsset_read
    register_handler_in(LIBANDROID, "AAsset_read", |context| {
        let id = context.arg_ptr(0) as u64;
        let buffer = context.arg_ptr(1);
        let count = context.arg_usize(2);
        if buffer == 0 {
            context.ret_i32(0);
            return;
        }
        let mut asset_table = assets().lock().expect("asset table");
        let Some(asset) = asset_table.get_mut(&id) else {
            context.ret_i32(-1);
            return;
        };
        let available = asset.data.len().saturating_sub(asset.position);
        let copied = available.min(count);
        if copied > 0 {
            write_bytes(buffer, &asset.data[asset.position..asset.position + copied]);
            asset.position += copied;
        }
        context.ret_i32(i32::try_from(copied).unwrap_or(i32::MAX));
    });

    register_handler_in(LIBANDROID, "AAsset_close", |context| {
        let id = context.arg_ptr(0) as u64;
        assets().lock().expect("asset table").remove(&id);
    });
}

trait ArgUsize {
    fn arg_usize(&self, index: usize) -> usize;
}

impl ArgUsize for CallContext<'_> {
    fn arg_usize(&self, index: usize) -> usize {
        self.arg_u64(index) as usize
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn asset_open_reads_the_apk_assets_entry() {
        let path =
            std::env::temp_dir().join(format!("winedroid-assets-{}.apk", std::process::id()));
        {
            let file = std::fs::File::create(&path).expect("create APK");
            let mut archive = zip::ZipWriter::new(file);
            archive
                .start_file(
                    "assets/fonts/test.ttf",
                    zip::write::SimpleFileOptions::default(),
                )
                .expect("start asset");
            archive.write_all(b"font data").expect("write asset");
            archive.finish().expect("finish APK");
        }

        set_apk_path(&path);
        assert_eq!(open_asset("fonts/test.ttf"), Some(b"font data".to_vec()));
        assert_eq!(open_asset("fonts/missing.ttf"), None);
        let _ = std::fs::remove_file(path);
    }
}
