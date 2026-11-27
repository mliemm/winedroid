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
    handles: RefCell<Vec<u64>>,
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
        self.jni.handles.borrow_mut().push(handle);
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

    fn find_native_method(&self, loader: &Loader, method: &DexMethodId) -> Result<Option<usize>> {
        // cite: art java_vm_ext.cc#FindNativeMethod, FindNativeMethodInternal
        let short_name = jni_short_name(&method.class_descriptor, &method.name);
        let long_name = jni_long_name(&short_name, &method.proto.parameters);
        let handles = self.jni.handles.borrow().clone();
        for handle in handles {
            for name in [&short_name, &long_name] {
                if let Some(address) = loader.lookup_export(handle, name)? {
                    return Ok(Some(address));
                }
            }
        }
        Ok(None)
    }

    pub(super) fn execute_registered_native(
        &self,
        resolved: &ResolvedMethod,
        arguments: &[DexValue],
        depth: usize,
    ) -> Result<DexExecutionResult> {
        // cite: art art_method.h#GetEntryPointFromJni
        let method = &resolved.method;
        let key: NativeMethod = method.into();
        let registered = self.jni.bindings.borrow().get(&key).copied();
        let address = match registered {
            Some(address) => address,
            None => {
                let runtime =
                    NativeRuntime::current().filter(|_| !self.jni.handles.borrow().is_empty());
                let Some(runtime) = runtime else {
                    return Err(WinedroidError::new(format!(
                        "JNI method has no registered entry point\nnext missing primitive: JNI binding {}.{}",
                        method.class_descriptor, method.display_signature()
                    )));
                };
                match self.find_native_method(runtime.loader(), method)? {
                    Some(address) => address,
                    None => {
                        // cite: art jni_entrypoints.cc#artFindNativeMethodRunnable
                        // cite: art java_vm_ext.cc#FindNativeMethod
                        let short_name = jni_short_name(&method.class_descriptor, &method.name);
                        let long_name = jni_long_name(&short_name, &method.proto.parameters);
                        return Ok(DexExecutionResult::Throw(self.allocate_generated_exception(
                            DexGeneratedException {
                                class_descriptor: "Ljava/lang/UnsatisfiedLinkError;".to_owned(),
                                detail_message: Some(format!(
                                    "No implementation found for {} (tried {short_name} and {long_name}) - is the library loaded, e.g. System.loadLibrary?",
                                    pretty_method(method)
                                )),
                            },
                        )));
                    }
                }
            }
        };
        self.jni.bindings.borrow_mut().insert(key, address);
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

// cite: art art_method.cc#PrettyMethod
fn pretty_method(method: &DexMethodId) -> String {
    let pretty = |descriptor: &str| {
        descriptor_to_pretty_name(descriptor).unwrap_or_else(|| descriptor.to_owned())
    };
    let parameters: Vec<String> = method
        .proto
        .parameters
        .iter()
        .map(|parameter| pretty(parameter))
        .collect();
    format!(
        "{} {}.{}({})",
        pretty(&method.proto.return_type),
        pretty(&method.class_descriptor),
        method.name,
        parameters.join(", ")
    )
}

fn jni_short_name(class_descriptor: &str, method: &str) -> String {
    // cite: art descriptors_names.cc#GetJniShortName
    let class_name = class_descriptor
        .strip_prefix('L')
        .and_then(|name| name.strip_suffix(';'))
        .unwrap_or(class_descriptor);
    format!(
        "Java_{}_{}",
        mangle_for_jni(class_name),
        mangle_for_jni(method)
    )
}

fn jni_long_name(short_name: &str, parameters: &[String]) -> String {
    // cite: art art_method.cc#JniLongName
    format!("{short_name}__{}", mangle_for_jni(&parameters.concat()))
}

fn mangle_for_jni(text: &str) -> String {
    // cite: art descriptors_names.cc#MangleForJni
    let mut result = String::new();
    for unit in text.encode_utf16() {
        match unit {
            0x30..=0x39 | 0x41..=0x5a | 0x61..=0x7a => result.push(char::from(unit as u8)),
            0x2e | 0x2f => result.push('_'),
            0x5f => result.push_str("_1"),
            0x3b => result.push_str("_2"),
            0x5b => result.push_str("_3"),
            _ => result.push_str(&format!("_0{unit:04x}")),
        }
    }
    result
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

    #[test]
    fn art_jni_names_match_class_linker_test() {
        // cite: art class_linker_test.cc#JniShortName_JniLongName
        let string = "Ljava/lang/String;";
        let char_at = jni_short_name(string, "charAt");
        assert_eq!(char_at, "Java_java_lang_String_charAt");
        assert_eq!(
            jni_long_name(&char_at, &["I".to_owned()]),
            "Java_java_lang_String_charAt__I"
        );
        let index_of = jni_short_name(string, "indexOf");
        assert_eq!(
            jni_long_name(&index_of, &[string.to_owned(), "I".to_owned()]),
            "Java_java_lang_String_indexOf__Ljava_lang_String_2I"
        );
        let copy_value_of = jni_short_name(string, "copyValueOf");
        assert_eq!(
            jni_long_name(
                &copy_value_of,
                &["[C".to_owned(), "I".to_owned(), "I".to_owned()]
            ),
            "Java_java_lang_String_copyValueOf___3CII"
        );
        assert_eq!(
            jni_short_name("La_b/C$D;", "e\u{e9}"),
            "Java_a_1b_C_00024D_e_000e9"
        );
    }

    #[test]
    fn art_jni_unregistered_native_resolves_by_exported_short_name() {
        // cite: art java_vm_ext.cc#FindNativeMethod
        let dex = dex::parse_dex_file("classes.dex", fixture_dex()).unwrap();
        let program = DexProgram::new(vec![dex]);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/native_fixtures/bin/16k");
        let loader =
            Loader::new(vec![dir.clone()], Box::new(bridge::native_provider())).with_strict(true);
        let mut method = program
            .resolve_named_method("Lfx/Native;", "registeredAdd")
            .unwrap()
            .method;
        assert_eq!(program.find_native_method(&loader, &method).unwrap(), None);

        let path = dir.join("libfx_jni.so");
        program.load_native_image(&loader, &path).unwrap();
        assert_eq!(program.find_native_method(&loader, &method).unwrap(), None);

        let handle = loader
            .dlopen(path.to_str(), crate::native::module::RTLD_NOW, None)
            .unwrap();
        let exported = loader
            .lookup_export(handle, "Java_fx_Native_stringLength")
            .unwrap()
            .unwrap();
        method.name = "stringLength".to_owned();
        method.proto.parameters = vec!["Ljava/lang/String;".to_owned()];
        assert_eq!(
            program.find_native_method(&loader, &method).unwrap(),
            Some(exported)
        );
    }

    #[test]
    fn art_jni_call_static_method_v_upcalls_guest_static_methods() {
        // cite: art jni_internal.cc#CallStaticLongMethodV, CallStaticVoidMethodV, GetStaticMethodID
        // cite: art java_vm_ext.cc#FindNativeMethod
        let dex = dex::parse_dex_file(
            "jni_static_long.dex",
            include_bytes!("tests/fixtures/jni_static_long.dex").to_vec(),
        )
        .unwrap();
        let program = DexProgram::new(vec![dex]);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/native_fixtures/bin/16k");
        let runtime = NativeRuntime::initialize(vec![dir.clone()]).unwrap();
        program
            .load_native_image(runtime.loader(), &dir.join("libfx_jni.so"))
            .unwrap();
        let method = program
            .resolve_named_method("Lfx/Native;", "callStaticLong")
            .unwrap();

        assert_eq!(
            program
                .execute_method(
                    None,
                    &method,
                    Some(&[DexValue::Long(14), DexValue::Long(14)]),
                    0
                )
                .unwrap(),
            DexExecutionResult::Value(DexValue::Long(43)),
        );

        let call_void = program
            .resolve_named_method("Lfx/Native;", "callStaticVoid")
            .unwrap();
        assert_eq!(
            program
                .execute_method(None, &call_void, Some(&[DexValue::Int(41)]), 0)
                .unwrap(),
            DexExecutionResult::Void,
        );
        let recorded = DexFieldId {
            class_descriptor: "Lfx/Native;".to_owned(),
            type_descriptor: "I".to_owned(),
            name: "recorded".to_owned(),
        };
        assert_eq!(
            program
                .runtime
                .borrow()
                .static_fields
                .get(&recorded)
                .cloned(),
            Some(DexValue::Int(42))
        );
    }

    #[test]
    fn art_jni_native_without_binding_or_symbol_throws_unsatisfied_link_error() {
        // cite: art jni_entrypoints.cc#artFindNativeMethodRunnable
        // cite: art java_vm_ext.cc#FindNativeMethod
        let dex = dex::parse_dex_file(
            "jni_static_long.dex",
            include_bytes!("tests/fixtures/jni_static_long.dex").to_vec(),
        )
        .unwrap();
        let program = DexProgram::new(vec![dex]);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/native_fixtures/bin/16k");
        let runtime = NativeRuntime::initialize(vec![dir.clone()]).unwrap();
        program
            .load_native_image(runtime.loader(), &dir.join("libfx_jni.so"))
            .unwrap();
        let method = program
            .resolve_named_method("Lfx/Native;", "unbound")
            .unwrap();

        let DexExecutionResult::Throw(exception) = program
            .execute_method(None, &method, Some(&[DexValue::Null, DexValue::Null]), 0)
            .unwrap()
        else {
            panic!("an unbound native should throw");
        };
        assert!(program.value_matches_type(&exception, "Ljava/lang/UnsatisfiedLinkError;"));
        assert_eq!(
            program.read_stored_instance_field(
                &exception,
                &DexFieldId {
                    class_descriptor: "Ljava/lang/Throwable;".to_owned(),
                    type_descriptor: "Ljava/lang/String;".to_owned(),
                    name: "detailMessage".to_owned(),
                }
            ),
            Some(DexValue::String(std::rc::Rc::from(
                "No implementation found for int fx.Native.unbound(java.lang.String, int[]) (tried Java_fx_Native_unbound and Java_fx_Native_unbound__Ljava_lang_String_2_3I) - is the library loaded, e.g. System.loadLibrary?"
            )))
        );
    }

    #[test]
    fn art_jni_byte_array_elements_and_regions_round_trip_through_native_code() {
        // cite: art jni_internal.cc#GetByteArrayElements, ReleaseByteArrayElements, NewByteArray, SetByteArrayRegion, ThrowAIOOBE
        let dex = dex::parse_dex_file(
            "jni_static_long.dex",
            include_bytes!("tests/fixtures/jni_static_long.dex").to_vec(),
        )
        .unwrap();
        let program = DexProgram::new(vec![dex]);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/native_fixtures/bin/16k");
        let runtime = NativeRuntime::initialize(vec![dir.clone()]).unwrap();
        program
            .load_native_image(runtime.loader(), &dir.join("libfx_jni.so"))
            .unwrap();
        let bytes = |values: &[i32]| {
            program.allocate_filled_array(
                "[B".to_owned(),
                values.iter().map(|value| DexValue::Int(*value)).collect(),
            )
        };
        let contents = |array: &DexValue| {
            program.runtime.borrow().array_entries[&map_identity_key(array)].clone()
        };
        let call = |name: &str, arguments: &[DexValue]| {
            let method = program.resolve_named_method("Lfx/Native;", name).unwrap();
            program
                .execute_method(None, &method, Some(arguments), 0)
                .unwrap()
        };

        let input = bytes(&[1, 2, -128]);
        let DexExecutionResult::Value(output) =
            call("xorBytes", &[input.clone(), DexValue::Int(0x0f)])
        else {
            panic!("xorBytes should return a byte[]");
        };
        assert_eq!(
            contents(&output),
            [DexValue::Int(14), DexValue::Int(13), DexValue::Int(-113)]
        );
        assert_eq!(
            contents(&input),
            [DexValue::Int(1), DexValue::Int(2), DexValue::Int(-128)]
        );

        let data = bytes(&[5]);
        assert_eq!(
            call("incrementInPlace", &[data.clone()]),
            DexExecutionResult::Void
        );
        assert_eq!(contents(&data), [DexValue::Int(6)]);

        let short = bytes(&[0, 0]);
        let DexExecutionResult::Throw(exception) = call("overflowRegion", &[short.clone()]) else {
            panic!("a region past the end should throw");
        };
        assert!(
            program.value_matches_type(&exception, "Ljava/lang/ArrayIndexOutOfBoundsException;")
        );
        assert_eq!(contents(&short), [DexValue::Int(0), DexValue::Int(0)]);
    }

    #[test]
    fn art_jni_get_string_utf_chars_encodes_string_objects_as_modified_utf8() {
        // cite: art jni_internal.cc#GetStringUTFChars, ReleaseStringUTFChars
        let dex = dex::parse_dex_file(
            "jni_static_long.dex",
            include_bytes!("tests/fixtures/jni_static_long.dex").to_vec(),
        )
        .unwrap();
        let program = DexProgram::new(vec![dex]);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/native_fixtures/bin/16k");
        let runtime = NativeRuntime::initialize(vec![dir.clone()]).unwrap();
        program
            .load_native_image(runtime.loader(), &dir.join("libfx_jni.so"))
            .unwrap();
        let method = program
            .resolve_named_method("Lfx/Native;", "stringLength")
            .unwrap();
        let string_object = |text: &str| {
            let value = program.allocate_object("Ljava/lang/String;".to_owned());
            program
                .runtime
                .borrow_mut()
                .string_values
                .insert(map_identity_key(&value), text.to_owned());
            value
        };

        for (text, length) in [("h\u{e9}llo", 6), ("\u{1f600}", 6), ("", 0)] {
            assert_eq!(
                program
                    .execute_method(None, &method, Some(&[string_object(text)]), 0)
                    .unwrap(),
                DexExecutionResult::Value(DexValue::Int(length)),
                "{text:?}"
            );
        }
        assert_eq!(
            program
                .execute_method(None, &method, Some(&[DexValue::String("abc".into())]), 0)
                .unwrap(),
            DexExecutionResult::Value(DexValue::Int(3))
        );
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
