use super::*;
use crate::native::{
    call_assign::CallBuilder, guest_context, loader::Loader, runtime::NativeRuntime,
};

mod env;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct NativeMethod {
    class: String,
    name: String,
    signature: String,
}

impl From<&DexMethodId> for NativeMethod {
    fn from(method: &DexMethodId) -> Self {
        Self {
            class: method.class_descriptor.clone(),
            name: method.name.clone(),
            signature: method.proto.descriptor(),
        }
    }
}

#[derive(Clone)]
enum LibraryState {
    Loading,
    Ready,
    Failed(String),
}

#[derive(Default)]
pub(super) struct JniState {
    libraries: RefCell<HashMap<PathBuf, LibraryState>>,
    bindings: RefCell<HashMap<NativeMethod, usize>>,
    method_ids: RefCell<HashMap<NativeMethod, usize>>,
    methods_by_id: RefCell<HashMap<usize, DexMethodId>>,
    field_ids: RefCell<Vec<DexFieldId>>,
    globals: RefCell<HashMap<usize, DexValue>>,
}

impl DexProgram {
    pub(super) fn load_jni_library(&self, soname: &str) -> Option<Result<()>> {
        let apk = self
            .apk_path
            .as_ref()
            .filter(|path| path.ends_with("source.apk"))?;
        let package = self.package_name.as_deref()?;
        let app = apk.parent()?;
        let library_dir = app.join("native/arm64-v8a");
        let path = library_dir.join(soname);
        if !path.is_file() {
            return None;
        }
        Some((|| {
            let prefix = app
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| WinedroidError::new("installed app has no native runtime prefix"))?;
            let runtime = NativeRuntime::initialize(vec![library_dir])?;
            if guest_context::current().is_none() {
                let policy = std::sync::Arc::new(std::sync::RwLock::new(
                    self.external_storage_policy.clone(),
                ));
                guest_context::install(guest_context::GuestContext::open(
                    prefix, package, policy,
                )?)?;
            }
            self.load_native_image(runtime.loader(), &path)
        })())
    }

    fn load_native_image(&self, loader: &Loader, path: &Path) -> Result<()> {
        // cite: art java_vm_ext.cc#LoadNativeLibrary, CheckOnLoadResult
        let existing = self.jni.libraries.borrow().get(path).cloned();
        match existing {
            Some(LibraryState::Loading | LibraryState::Ready) => return Ok(()),
            Some(LibraryState::Failed(message)) => return Err(WinedroidError::new(message)),
            None => {}
        }
        let name = path
            .to_str()
            .ok_or_else(|| WinedroidError::new("native library path is not UTF-8"))?;
        let handle = loader.dlopen(Some(name), crate::native::module::RTLD_NOW, None)?;
        self.jni
            .libraries
            .borrow_mut()
            .insert(path.to_owned(), LibraryState::Loading);

        let result = (|| {
            let Some(address) = loader.lookup_export(handle, "JNI_OnLoad")? else {
                return Ok(());
            };
            let frame = env::Frame::new(self, 0);
            let version = frame
                .call(CallBuilder::new(address).ptr(env::vm_pointer()).ptr(0))?
                .i32();
            if let Some(exception) = frame.exception() {
                return Err(WinedroidError::new(format!(
                    "JNI_OnLoad in {name} raised {exception}"
                )));
            }
            if !env::valid_version(version) {
                return Err(WinedroidError::new(format!(
                    "JNI_OnLoad in {name} returned {version:#x}"
                )));
            }
            self.jni.bindings.borrow_mut().extend(frame.take_bindings());
            Ok(())
        })();

        let state = match &result {
            Ok(()) => {
                trace::event("jni", format!("loaded {name}"));
                LibraryState::Ready
            }
            Err(error) => LibraryState::Failed(error.to_string()),
        };
        self.jni
            .libraries
            .borrow_mut()
            .insert(path.to_owned(), state);
        result
    }

    fn find_jni_method(
        &self,
        class: &str,
        name: &str,
        signature: &str,
        static_method: Option<bool>,
    ) -> Result<Option<(DexMethodId, u32)>> {
        // cite: art jni_internal.cc#FindMethod, RegisterNatives, GetMethodID
        let mut class = class.to_owned();
        let mut visited = HashSet::new();
        while visited.insert(class.clone()) {
            let Some(loaded) = self.load_class(&class) else {
                break;
            };
            let dex = &self.dex_files[loaded.dex_file_index];
            let mut non_native = None;
            for encoded in loaded
                .class_def
                .direct_methods
                .iter()
                .chain(&loaded.class_def.virtual_methods)
            {
                let method = dex.method(encoded.method_index)?;
                let flags = encoded.access_flags;
                if method.name != name || !method.proto.descriptor_is(signature) {
                    continue;
                }
                if let Some(expect_static) = static_method {
                    if (flags & DEX_ACC_STATIC != 0) == expect_static {
                        return Ok(Some((method.clone(), flags)));
                    }
                } else if flags & DEX_ACC_NATIVE != 0 {
                    return Ok(Some((method.clone(), flags)));
                } else {
                    non_native = Some((method.clone(), flags));
                }
            }
            if non_native.is_some() {
                return Ok(non_native);
            }
            if name == "<init>" {
                break;
            }
            let Some(parent) = loaded.class_def.superclass_descriptor.as_ref() else {
                break;
            };
            class.clone_from(parent);
        }
        Ok(None)
    }

    fn find_jni_static_field(
        &self,
        class: &str,
        name: &str,
        signature: &str,
    ) -> Result<Option<DexFieldId>> {
        // cite: art jni_internal.cc#FindFieldJNI
        // cite: art class.cc#FindStaticField, FindDeclaredStaticField
        let mut unknown = None;
        let mut pending = vec![class.to_owned()];
        let mut visited = HashSet::new();
        while let Some(class) = pending.pop() {
            if !visited.insert(class.clone()) {
                continue;
            }
            let field = DexFieldId {
                class_descriptor: class.clone(),
                type_descriptor: signature.to_owned(),
                name: name.to_owned(),
            };
            if let Some(loaded) = self.load_class(&class) {
                for encoded in &loaded.class_def.static_fields {
                    if *self.field_id(loaded.dex_file_index, encoded.field_index)? == field {
                        return Ok(Some(field));
                    }
                }
            } else if bootstrap_static_field(&field).is_some() {
                return Ok(Some(field));
            } else if bootstrap_declared_fields(&class).is_none_or(|fields| {
                fields
                    .iter()
                    .any(|field| field.name == name && field.access_flags & DEX_ACC_STATIC != 0)
            }) {
                unknown.get_or_insert_with(|| class.clone());
            }
            pending.extend(self.superclass_descriptor(&class));
            pending.extend(self.class_interface_descriptors(&class).into_iter().rev());
        }
        match unknown {
            Some(class) => Err(WinedroidError::new(format!(
                "next missing primitive: JNI static field {name}:{signature} of framework class {class}"
            ))),
            None => Ok(None),
        }
    }

    pub(super) fn execute_registered_native(
        &self,
        resolved: &ResolvedMethod,
        arguments: &[DexValue],
        depth: usize,
    ) -> Result<DexExecutionResult> {
        // cite: art art_method.h#GetEntryPointFromJni
        let method = &resolved.method;
        let key = method.into();
        let address = self.jni.bindings.borrow().get(&key).copied().ok_or_else(|| {
            WinedroidError::new(format!(
                "JNI method has no registered entry point\nnext missing primitive: JNI binding {}.{}",
                method.class_descriptor, method.display_signature()
            ))
        })?;
        let frame = env::Frame::new(self, depth);
        let is_static = resolved.encoded_method.access_flags & DEX_ACC_STATIC != 0;
        let (receiver, arguments) = if is_static {
            (frame.class_reference(&method.class_descriptor), arguments)
        } else {
            let (receiver, arguments) = arguments
                .split_first()
                .ok_or_else(|| WinedroidError::new("missing JNI receiver"))?;
            (frame.reference(receiver.clone()), arguments)
        };
        let mut call = CallBuilder::new(address)
            .ptr(env::env_pointer())
            .ptr(receiver);
        let expected_words: usize = method
            .proto
            .parameters
            .iter()
            .map(|descriptor| {
                if matches!(descriptor.as_str(), "J" | "D") {
                    2
                } else {
                    1
                }
            })
            .sum();
        if arguments.len() != expected_words {
            return Err(WinedroidError::new(format!(
                "JNI argument count mismatch for {}: {}",
                method.display_signature(),
                arguments.len()
            )));
        }
        let mut word = 0;
        for parameter in &method.proto.parameters {
            let value = &arguments[word];
            let invalid_argument =
                || WinedroidError::new(format!("invalid JNI argument for {parameter}: {value}"));
            call = match (parameter.as_str(), value) {
                ("Z", DexValue::Int(value)) => call.u8((*value != 0) as u8),
                ("B", DexValue::Int(value)) => call.i8(*value as i8),
                ("C", DexValue::Int(value)) => call.u16(*value as u16),
                ("S", DexValue::Int(value)) => call.i16(*value as i16),
                ("I", DexValue::Int(value)) => call.i32(*value),
                ("J", DexValue::Long(value)) => call.int(*value as u64),
                ("D", DexValue::Long(value)) => call.f64(f64::from_bits(*value as u64)),
                // cite: art shadow_frame.h#GetVRegFloat
                ("F", value) => call.f32(read_float_argument(value).ok_or_else(invalid_argument)?),
                (
                    descriptor,
                    value @ (DexValue::Null
                    | DexValue::Int(0)
                    | DexValue::String(_)
                    | DexValue::Object(_)
                    | DexValue::Reference(_)),
                ) if descriptor.starts_with(['L', '[']) => call.ptr(frame.reference(value.clone())),
                _ => return Err(invalid_argument()),
            };
            word += if matches!(parameter.as_str(), "J" | "D") {
                2
            } else {
                1
            };
        }
        let result = frame.call(call)?;
        self.jni.bindings.borrow_mut().extend(frame.take_bindings());
        if let Some(exception) = frame.exception() {
            return Ok(DexExecutionResult::Throw(exception));
        }
        let value = match method.proto.return_type.as_str() {
            "V" => return Ok(DexExecutionResult::Void),
            "Z" => DexValue::Int((result.u64() as u8 != 0) as i32),
            "B" => DexValue::Int(result.u64() as i8 as i32),
            "C" => DexValue::Int(result.u64() as u16 as i32),
            "S" => DexValue::Int(result.u64() as i16 as i32),
            "I" => DexValue::Int(result.i32()),
            "J" => DexValue::Long(result.u64() as i64),
            "D" => DexValue::Long(result.f64().to_bits() as i64),
            "F" => DexValue::Float(result.f32()),
            descriptor if descriptor.starts_with(['L', '[']) => {
                frame.value(result.u64() as usize)?
            }
            _ => return Err(WinedroidError::new("invalid JNI return descriptor")),
        };
        Ok(DexExecutionResult::Value(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bridge;

    #[test]
    fn art_jni_float_argument_preserves_dex_register_bits() {
        // cite: art shadow_frame.h#GetVRegFloat
        extern "C" fn echo_float(_env: usize, _class: usize, value: f32) -> f32 {
            value
        }

        let dex = dex::parse_dex_file("classes.dex", fixture_dex()).unwrap();
        let program = DexProgram::new(vec![dex]);
        let mut method = program
            .resolve_named_method("Lfx/Native;", "registeredAdd")
            .unwrap();
        method.method.proto.shorty = "FF".to_owned();
        method.method.proto.parameters = vec!["F".to_owned()];
        method.method.proto.return_type = "F".to_owned();
        program
            .jni
            .bindings
            .borrow_mut()
            .insert((&method.method).into(), echo_float as *const () as usize);

        for input in [
            DexValue::Int((-1.0f32).to_bits() as i32),
            DexValue::Float(-0.0),
        ] {
            let expected = read_float_argument(&input).unwrap().to_bits();
            let result = program
                .execute_registered_native(&method, &[input], 0)
                .unwrap();
            let DexExecutionResult::Value(DexValue::Float(value)) = result else {
                panic!("expected a JNI float return");
            };
            assert_eq!(value.to_bits(), expected);
        }
        assert!(program
            .execute_registered_native(&method, &[DexValue::Null], 0)
            .is_err());
    }

    #[test]
    fn art_jni_on_load_registers_and_calls_fixture_method() {
        // cite: art jni_internal.cc#RegisterNatives
        // cite: art java_vm_ext.cc#LoadNativeLibrary
        let dex = dex::parse_dex_file("classes.dex", fixture_dex()).unwrap();
        let program = DexProgram::new(vec![dex]);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/native_fixtures/bin/16k");
        let loader =
            Loader::new(vec![dir.clone()], Box::new(bridge::native_provider())).with_strict(true);
        let path = dir.join("libfx_jni.so");

        program.load_native_image(&loader, &path).unwrap();
        program.load_native_image(&loader, &path).unwrap();
        assert_eq!(program.jni.bindings.borrow().len(), 1);
        let method = program
            .resolve_named_method("Lfx/Native;", "registeredAdd")
            .unwrap();
        assert_eq!(
            program
                .execute_method(
                    None,
                    &method,
                    Some(&[DexValue::Int(19), DexValue::Int(23)]),
                    0
                )
                .unwrap(),
            DexExecutionResult::Value(DexValue::Int(42)),
        );

        let wrong_program = DexProgram::new(Vec::new());
        for _ in 0..2 {
            let error = wrong_program.load_native_image(&loader, &path).unwrap_err();
            assert!(error.to_string().contains("JNI_OnLoad"));
        }
        assert!(wrong_program.jni.bindings.borrow().is_empty());
    }

    fn fixture_dex() -> Vec<u8> {
        let strings = ["I", "III", "Lfx/Native;", "registeredAdd"];
        let strings_off = 112;
        let types_off = strings_off + 4 * strings.len();
        let proto_off = types_off + 8;
        let method_off = proto_off + 12;
        let class_off = method_off + 8;
        let args_off = class_off + 32;
        let mut dex = vec![0; args_off];
        dex.extend_from_slice(&[2, 0, 0, 0, 0, 0, 0, 0]);
        let class_data_off = dex.len();
        dex.extend_from_slice(&[0, 0, 1, 0, 0, 0x89, 0x02, 0]);
        for (index, string) in strings.into_iter().enumerate() {
            let offset = dex.len();
            put_u32(&mut dex, strings_off + 4 * index, offset);
            dex.push(string.len() as u8);
            dex.extend_from_slice(string.as_bytes());
            dex.push(0);
        }
        dex[..8].copy_from_slice(b"dex\n035\0");
        let len = dex.len();
        for (offset, value) in [
            (32, len),
            (36, 112),
            (40, 0x12345678),
            (56, strings.len()),
            (60, strings_off),
            (64, 2),
            (68, types_off),
            (72, 1),
            (76, proto_off),
            (88, 1),
            (92, method_off),
            (96, 1),
            (100, class_off),
            (104, len - args_off),
            (108, args_off),
            (types_off + 4, 2),
            (proto_off, 1),
            (proto_off + 8, args_off),
            (method_off + 4, 3),
            (class_off, 1),
            (class_off + 4, 1),
            (class_off + 8, u32::MAX as usize),
            (class_off + 24, class_data_off),
        ] {
            put_u32(&mut dex, offset, value);
        }
        dex[method_off] = 1;
        dex
    }

    fn put_u32(dex: &mut [u8], offset: usize, value: usize) {
        dex[offset..offset + 4].copy_from_slice(&(value as u32).to_le_bytes());
    }
}
