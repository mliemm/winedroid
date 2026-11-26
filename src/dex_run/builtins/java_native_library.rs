use super::super::*;

fn map_library_name(library_name: &str) -> String {
    // cite: libcore System.c#System_mapLibraryName
    // cite: libcore jvm_md.h#JNI_LIB_PREFIX, JNI_LIB_SUFFIX
    format!("lib{library_name}.so")
}

fn unsatisfied_link_error_message(message: String) -> DexGeneratedException {
    DexGeneratedException {
        class_descriptor: "Ljava/lang/UnsatisfiedLinkError;".to_owned(),
        detail_message: Some(message),
    }
}

impl DexProgram {
    pub(super) fn system_load(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        // cite: libcore System.java#load
        // cite: libcore Runtime.java#load0
        let [value] = invocation.arguments.as_slice() else {
            return None;
        };
        if is_android_null_value(value) {
            return Some(self.null_pointer_exception_result());
        }
        let DexValue::String(filename) = value else {
            return None;
        };
        if !filename.starts_with('/') {
            return Some(self.throw_library_error(format!(
                "Expecting an absolute path of the library: {filename}"
            )));
        }
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        if let Some(package) = self.package_name.as_deref() {
            if let Some(soname) = crate::guest_app_paths::native_library_name(package, filename) {
                if let Some(result) = self.load_jni_library(soname) {
                    return Some(self.native_load_result(soname, result));
                }
            }
        }
        Some(self.throw_library_error(format!("dlopen failed: library \"{filename}\" not found")))
    }

    pub(super) fn system_load_library(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: libcore System.java#loadLibrary
        // cite: libcore Runtime.java#loadLibrary0, nativeLoad
        let [library_name] = invocation.arguments.as_slice() else {
            return None;
        };
        if is_android_null_value(library_name) {
            return Some(self.null_pointer_exception_result());
        }
        let library_name = match library_name {
            DexValue::String(library_name) => library_name.as_ref(),
            _ => return None,
        };
        if library_name.contains('/') {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(unsatisfied_link_error_message(format!(
                    "Directory separator should not appear in library name: {library_name}"
                ))),
            ));
        }
        if library_name.encode_utf16().count() > 240 {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
                    detail_message: Some("name too long".to_owned()),
                }),
            ));
        }

        let soname = map_library_name(library_name);
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        if let Some(result) = self.load_jni_library(&soname) {
            return Some(self.native_load_result(&soname, result));
        }
        if self.installed_native_library_exists(&soname) {
            return None;
        }

        // cite: bionic dlfcn.cpp#dlopen_ext, __bionic_format_dlerror
        // cite: bionic linker.cpp#DL_OPEN_ERR
        Some(DexExecutionResult::Throw(
            self.allocate_generated_exception(unsatisfied_link_error_message(format!(
                "dlopen failed: library \"{soname}\" not found"
            ))),
        ))
    }

    fn throw_library_error(&self, message: String) -> DexExecutionResult {
        DexExecutionResult::Throw(
            self.allocate_generated_exception(unsatisfied_link_error_message(message)),
        )
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    fn native_load_result(&self, soname: &str, result: Result<()>) -> DexExecutionResult {
        match result {
            Ok(()) => DexExecutionResult::Void,
            Err(error) => {
                trace::event("jni", format!("failed to load {soname}: {error}"));
                self.throw_library_error(error.to_string())
            }
        }
    }

    fn installed_native_library_exists(&self, soname: &str) -> bool {
        // cite: art java_vm_ext.cc#LoadNativeLibrary
        let Some(native_dir) = self
            .apk_path
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(|app_dir| app_dir.join("native"))
        else {
            return false;
        };
        let Ok(abis) = std::fs::read_dir(native_dir) else {
            return false;
        };

        abis.flatten().any(|abi| abi.path().join(soname).is_file())
    }
}
