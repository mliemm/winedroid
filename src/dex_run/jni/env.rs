use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use super::*;
use crate::native::bionic::guest_mem::{cstr_bytes_bounded, read_value, write_value};
use crate::native::bridge::{register_handler, register_missing, CallContext};
use crate::native::guest_call::GuestResult;
use crate::native::va_list::GuestVaList;

const JNI_OK: i32 = 0;
const JNI_ERR: i32 = -1;
const JNI_EDETACHED: i32 = -2;
const JNI_EVERSION: i32 = -3;
const JNI_VERSION_1_6: i32 = 0x10006;
const MAX_NATIVE_METHODS: usize = 4096;
const MAX_NAME_BYTES: usize = 4096;
const MAX_NEW_STRING_UTF_BYTES: usize = 1 << 24;
static NEXT_REF: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static ACTIVE_FRAME: Cell<*const c_void> = const { Cell::new(std::ptr::null()) };
    static ENV: usize = env_table().as_ptr() as usize;
}

struct FrameScope(*const c_void);

impl Drop for FrameScope {
    fn drop(&mut self) {
        ACTIVE_FRAME.with(|active| active.set(self.0));
    }
}

pub(super) struct Frame<'a> {
    program: &'a DexProgram,
    depth: usize,
    refs: RefCell<HashMap<usize, DexValue>>,
    bindings: RefCell<HashMap<NativeMethod, usize>>,
    strings: RefCell<HashMap<usize, (usize, Vec<u8>)>>,
    exception: RefCell<Option<DexValue>>,
}

impl<'a> Frame<'a> {
    pub(super) fn new(program: &'a DexProgram, depth: usize) -> Self {
        Self {
            program,
            depth,
            refs: RefCell::new(HashMap::new()),
            bindings: RefCell::new(HashMap::new()),
            strings: RefCell::new(HashMap::new()),
            exception: RefCell::new(None),
        }
    }

    pub(super) fn call(&self, call: CallBuilder) -> Result<GuestResult> {
        let previous = ACTIVE_FRAME.with(|active| active.replace((self as *const Self).cast()));
        let _scope = FrameScope(previous);
        call.invoke()
    }

    pub(super) fn take_bindings(&self) -> HashMap<NativeMethod, usize> {
        std::mem::take(&mut *self.bindings.borrow_mut())
    }

    pub(super) fn reference(&self, value: DexValue) -> usize {
        if is_android_null_value(&value) {
            return 0;
        }
        let handle = next_reference();
        self.refs.borrow_mut().insert(handle, value);
        handle
    }

    pub(super) fn class_reference(&self, descriptor: &str) -> usize {
        self.reference(DexValue::Reference(Rc::from(format!(
            "Ljava/lang/Class;:{descriptor}"
        ))))
    }

    pub(super) fn value(&self, handle: usize) -> Result<DexValue> {
        if handle == 0 {
            return Ok(DexValue::Null);
        }
        self.refs
            .borrow()
            .get(&handle)
            .cloned()
            .or_else(|| self.program.jni.globals.borrow().get(&handle).cloned())
            .ok_or_else(|| WinedroidError::new(format!("invalid JNI reference {handle:#x}")))
    }

    fn class(&self, handle: usize) -> Result<String> {
        let DexValue::Reference(value) = self.value(handle)? else {
            return Err(WinedroidError::new("JNI reference is not a class"));
        };
        value
            .strip_prefix("Ljava/lang/Class;:")
            .map(str::to_owned)
            .ok_or_else(|| WinedroidError::new("JNI reference is not a class"))
    }

    pub(super) fn exception(&self) -> Option<DexValue> {
        self.exception.borrow().clone()
    }

    fn initialize_class(&self, class: &str) -> Result<bool> {
        match self
            .program
            .ensure_class_initialized(class, self.depth + 1)?
        {
            ClassInitializationResult::Complete => Ok(true),
            ClassInitializationResult::Throw(exception) => {
                *self.exception.borrow_mut() = Some(exception);
                Ok(false)
            }
            ClassInitializationResult::Parked(execution) => {
                self.program.discard_parked_execution(&execution);
                Err(WinedroidError::new(
                    "next missing primitive: JNI class initialization that parks",
                ))
            }
        }
    }

    fn static_field(&self, id: usize, kind: u8) -> Result<DexFieldId> {
        let field = id
            .checked_sub(1)
            .and_then(|index| self.program.jni.field_ids.borrow().get(index).cloned())
            .ok_or_else(|| WinedroidError::new("unknown JNI static field ID"))?;
        let actual = match field.type_descriptor.as_bytes().first() {
            Some(b'[') => Some(b'L'),
            first => first.copied(),
        };
        if actual != Some(kind) {
            return Err(WinedroidError::new(format!(
                "JNI static field {} accessed with the wrong type",
                field.display_signature()
            )));
        }
        Ok(field)
    }

    fn throw(&self, class: &str, detail: String) {
        let exception = self
            .program
            .allocate_generated_exception(DexGeneratedException {
                class_descriptor: class.to_owned(),
                detail_message: Some(detail),
            });
        *self.exception.borrow_mut() = Some(exception);
    }
}

fn next_reference() -> usize {
    let reference = NEXT_REF.fetch_add(1, Ordering::Relaxed);
    assert_ne!(reference, 0, "exhausted JNI references");
    reference
}

fn with_env(
    context: &mut CallContext,
    operation: impl FnOnce(&Frame<'_>, &mut CallContext) -> Result<()>,
) {
    if context.arg_ptr(0) != env_pointer() {
        context.fail("invalid JNI environment for current thread");
        return;
    }
    let address = ACTIVE_FRAME.with(Cell::get);
    if address.is_null() {
        context.fail("no active JNI frame for current thread");
        return;
    }
    let frame = unsafe { &*address.cast::<Frame<'_>>() };
    if let Err(error) = operation(frame, context) {
        context.fail(format!("{}: {error}", context.symbol()));
    }
}

pub(super) fn valid_version(version: i32) -> bool {
    // cite: art java_vm_ext.cc#IsBadJniVersion
    matches!(version, 0x10002 | 0x10004 | JNI_VERSION_1_6)
}

pub(super) fn env_pointer() -> usize {
    ENV.with(|env| std::ptr::from_ref(env) as usize)
}

pub(super) fn vm_pointer() -> usize {
    static VM: OnceLock<usize> = OnceLock::new();
    let vm = VM.get_or_init(|| {
        // cite: art java_vm_ext.cc#gJniInvokeInterface
        let mut table = [0; 8];
        for (index, name) in [
            (3, "DestroyJavaVM"),
            (4, "AttachCurrentThread"),
            (5, "DetachCurrentThread"),
            (7, "AttachCurrentThreadAsDaemon"),
        ] {
            table[index] = register_missing("JavaVM", name);
        }
        table[6] = register_handler("JavaVM.GetEnv", get_env);
        Box::leak(Box::new(table)).as_ptr() as usize
    });
    std::ptr::from_ref(vm) as usize
}

fn env_table() -> &'static [usize; 233] {
    static TABLE: OnceLock<[usize; 233]> = OnceLock::new();
    TABLE.get_or_init(|| {
        // cite: art jni_internal.cc#gJniNativeInterface
        let mut table = [0; 233];
        let handlers: &[(usize, &str, fn(&mut CallContext))] = &[
            (4, "GetVersion", get_version),
            (6, "FindClass", find_class),
            (15, "ExceptionOccurred", exception_occurred),
            (17, "ExceptionClear", exception_clear),
            (21, "NewGlobalRef", new_global_ref),
            (22, "DeleteGlobalRef", delete_global_ref),
            (23, "DeleteLocalRef", delete_local_ref),
            (62, "CallVoidMethodV", call_void_method_v),
            (33, "GetMethodID", get_method_id),
            (113, "GetStaticMethodID", get_static_method_id),
            (133, "CallStaticLongMethodV", call_static_method_v::<b'J'>),
            (142, "CallStaticVoidMethodV", call_static_method_v::<b'V'>),
            (144, "GetStaticFieldID", get_static_field_id),
            (145, "GetStaticObjectField", get_static_field::<b'L'>),
            (146, "GetStaticBooleanField", get_static_field::<b'Z'>),
            (147, "GetStaticByteField", get_static_field::<b'B'>),
            (148, "GetStaticCharField", get_static_field::<b'C'>),
            (149, "GetStaticShortField", get_static_field::<b'S'>),
            (150, "GetStaticIntField", get_static_field::<b'I'>),
            (151, "GetStaticLongField", get_static_field::<b'J'>),
            (152, "GetStaticFloatField", get_static_field::<b'F'>),
            (153, "GetStaticDoubleField", get_static_field::<b'D'>),
            (154, "SetStaticObjectField", set_static_field::<b'L'>),
            (155, "SetStaticBooleanField", set_static_field::<b'Z'>),
            (156, "SetStaticByteField", set_static_field::<b'B'>),
            (157, "SetStaticCharField", set_static_field::<b'C'>),
            (158, "SetStaticShortField", set_static_field::<b'S'>),
            (159, "SetStaticIntField", set_static_field::<b'I'>),
            (160, "SetStaticLongField", set_static_field::<b'J'>),
            (161, "SetStaticFloatField", set_static_field::<b'F'>),
            (162, "SetStaticDoubleField", set_static_field::<b'D'>),
            (167, "NewStringUTF", new_string_utf),
            (169, "GetStringUTFChars", get_string_utf_chars),
            (170, "ReleaseStringUTFChars", release_string_utf_chars),
            (171, "GetArrayLength", get_array_length),
            (173, "GetObjectArrayElement", get_object_array_element),
            (215, "RegisterNatives", register_natives),
            (219, "GetJavaVM", get_java_vm),
            (228, "ExceptionCheck", exception_check),
        ];
        for &(slot, name, handler) in handlers {
            table[slot] = register_handler(&format!("JNIEnv.{name}"), handler);
        }
        for (slot, address) in table.iter_mut().enumerate().skip(4) {
            if *address == 0 {
                *address = register_missing("JNIEnv", &format!("JNIEnv slot {slot}"));
            }
        }
        table
    })
}

fn get_env(context: &mut CallContext) {
    // cite: art java_vm_ext.cc#GetEnv, HandleGetEnv
    let output = context.arg_ptr(1);
    if output == 0 || context.arg_ptr(0) != vm_pointer() {
        context.ret_i32(JNI_ERR);
        return;
    }
    let (status, env) = if ACTIVE_FRAME.with(Cell::get).is_null() {
        (JNI_EDETACHED, 0)
    } else if !valid_version(context.arg_i32(2)) {
        (JNI_EVERSION, 0)
    } else {
        (JNI_OK, env_pointer())
    };
    write_value(output, env);
    context.ret_i32(status);
}

fn bounded_name(address: usize) -> Result<String> {
    if address == 0 {
        return Err(WinedroidError::new("null JNI string"));
    }
    let bytes = cstr_bytes_bounded(address, MAX_NAME_BYTES);
    if bytes.len() == MAX_NAME_BYTES {
        return Err(WinedroidError::new("unterminated JNI string"));
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|error| WinedroidError::with_source("unsupported JNI name encoding", error))
}

fn find_class(context: &mut CallContext) {
    // cite: art jni_internal.cc#FindClass
    with_env(context, |frame, context| {
        let name = bounded_name(context.arg_ptr(1))?;
        let descriptor = if name.starts_with('[') {
            name
        } else {
            format!("L{name};")
        };
        if frame.program.load_class(&descriptor).is_some()
            || super::super::bootstrap::is_bootstrap_class_descriptor(&descriptor)
        {
            context.ret_ptr(frame.class_reference(&descriptor));
        } else {
            frame.throw("Ljava/lang/NoClassDefFoundError;", descriptor);
            context.ret_ptr(0);
        }
        Ok(())
    });
}

fn get_version(context: &mut CallContext) {
    // cite: art jni_internal.cc#GetVersion
    with_env(context, |_, context| {
        context.ret_i32(JNI_VERSION_1_6);
        Ok(())
    });
}

fn get_array_length(context: &mut CallContext) {
    // cite: art jni_internal.cc#GetArrayLength
    with_env(context, |frame, context| {
        let array = frame.value(context.arg_ptr(1))?;
        if is_android_null_value(&array) {
            return Err(WinedroidError::new("GetArrayLength: java_array == null"));
        }
        let length = frame
            .program
            .array_length(&array)
            .ok_or_else(|| WinedroidError::new("GetArrayLength: reference is not an array"))?;
        context.ret_i32(length);
        Ok(())
    });
}

fn get_object_array_element(context: &mut CallContext) {
    // cite: art jni_internal.cc#GetObjectArrayElement
    with_env(context, |frame, context| {
        let array = frame.value(context.arg_ptr(1))?;
        if is_android_null_value(&array) {
            return Err(WinedroidError::new(
                "GetObjectArrayElement: java_array == null",
            ));
        }
        if !value_class_descriptor(&array)
            .is_some_and(|class| class.starts_with("[L") || class.starts_with("[["))
        {
            return Err(WinedroidError::new(
                "GetObjectArrayElement: reference is not an object array",
            ));
        }
        let length = frame.program.array_length(&array).ok_or_else(|| {
            WinedroidError::new("GetObjectArrayElement: array storage is unavailable")
        })?;
        let index = context.arg_i32(2);
        if index < 0 || index >= length {
            frame.throw(
                "Ljava/lang/ArrayIndexOutOfBoundsException;",
                format!("length={length}; index={index}"),
            );
            context.ret_ptr(0);
            return Ok(());
        }
        let value = frame
            .program
            .read_array_value(&array, index)
            .ok_or_else(|| {
                WinedroidError::new("GetObjectArrayElement: array element is unavailable")
            })?;
        context.ret_ptr(frame.reference(value));
        Ok(())
    });
}

fn get_method_id(context: &mut CallContext) {
    method_id(context, false);
}

fn get_static_method_id(context: &mut CallContext) {
    method_id(context, true);
}

fn method_id(context: &mut CallContext, is_static: bool) {
    // cite: art jni_internal.cc#FindMethodID, GetMethodID, GetStaticMethodID
    with_env(context, |frame, context| {
        let class = frame.class(context.arg_ptr(1))?;
        let name = bounded_name(context.arg_ptr(2))?;
        let signature = bounded_name(context.arg_ptr(3))?;
        let Some((method, _)) =
            frame
                .program
                .find_jni_method(&class, &name, &signature, Some(is_static))?
        else {
            frame.throw(
                "Ljava/lang/NoSuchMethodError;",
                format!("{class}.{name}{signature}"),
            );
            context.ret_ptr(0);
            return Ok(());
        };
        if !frame.initialize_class(&class)? {
            context.ret_ptr(0);
            return Ok(());
        }
        let mut ids = frame.program.jni.method_ids.borrow_mut();
        let next = ids.len() + 1;
        let id = *ids.entry(NativeMethod::from(&method)).or_insert(next);
        frame
            .program
            .jni
            .methods_by_id
            .borrow_mut()
            .insert(id, method);
        context.ret_ptr(id);
        Ok(())
    });
}

fn get_static_field_id(context: &mut CallContext) {
    // cite: art jni_internal.cc#GetStaticFieldID, FindFieldJNI
    with_env(context, |frame, context| {
        let class = frame.class(context.arg_ptr(1))?;
        let name = bounded_name(context.arg_ptr(2))?;
        let signature = bounded_name(context.arg_ptr(3))?;
        if !frame.initialize_class(&class)? {
            context.ret_ptr(0);
            return Ok(());
        }
        let Some(field) = frame
            .program
            .find_jni_static_field(&class, &name, &signature)?
        else {
            frame.throw(
                "Ljava/lang/NoSuchFieldError;",
                format!(
                    "no \"{signature}\" field \"{name}\" in class \"{class}\" or its superclasses"
                ),
            );
            context.ret_ptr(0);
            return Ok(());
        };
        let mut fields = frame.program.jni.field_ids.borrow_mut();
        let index = fields
            .iter()
            .position(|known| *known == field)
            .unwrap_or_else(|| {
                fields.push(field);
                fields.len() - 1
            });
        context.ret_ptr(index + 1);
        Ok(())
    });
}

fn get_static_field<const KIND: u8>(context: &mut CallContext) {
    // cite: art jni_internal.cc#GetStaticObjectField, GET_STATIC_PRIMITIVE_FIELD
    with_env(context, |frame, context| {
        let field = frame.static_field(context.arg_ptr(2), KIND)?;
        let value = frame
            .program
            .read_stored_static_field(&field)
            .or_else(|| bootstrap_static_field(&field))
            .or_else(|| default_field_value(&field));
        match (KIND, value) {
            (
                b'L',
                Some(
                    value @ (DexValue::Null
                    | DexValue::Int(0)
                    | DexValue::String(_)
                    | DexValue::Object(_)
                    | DexValue::Reference(_)),
                ),
            ) => context.ret_ptr(frame.reference(value)),
            (b'Z', Some(DexValue::Int(value))) => context.ret_u32(u32::from(value as u8)),
            (b'B', Some(DexValue::Int(value))) => context.ret_i32(i32::from(value as i8)),
            (b'C', Some(DexValue::Int(value))) => context.ret_u32(u32::from(value as u16)),
            (b'S', Some(DexValue::Int(value))) => context.ret_i32(i32::from(value as i16)),
            (b'I', Some(DexValue::Int(value))) => context.ret_i32(value),
            (b'J', Some(DexValue::Long(value))) => context.ret_i64(value),
            (b'F', Some(DexValue::Float(value))) => context.ret_f32(value),
            (b'F', Some(DexValue::Int(bits))) => context.ret_f32(f32::from_bits(bits as u32)),
            (b'D', Some(DexValue::Long(bits))) => context.ret_f64(f64::from_bits(bits as u64)),
            (_, value) => {
                return Err(WinedroidError::new(format!(
                    "next missing primitive: JNI static field {} holding {value:?}",
                    field.display_signature()
                )))
            }
        }
        Ok(())
    });
}

fn set_static_field<const KIND: u8>(context: &mut CallContext) {
    // cite: art jni_internal.cc#SetStaticObjectField, SET_STATIC_PRIMITIVE_FIELD
    with_env(context, |frame, context| {
        let field = frame.static_field(context.arg_ptr(2), KIND)?;
        let bits = context.arg_u64(3);
        let value = match KIND {
            b'Z' => DexValue::Int(i32::from(bits as u8)),
            b'B' => DexValue::Int(i32::from(bits as i8)),
            b'C' => DexValue::Int(i32::from(bits as u16)),
            b'S' => DexValue::Int(i32::from(bits as i16)),
            b'I' => DexValue::Int(bits as i32),
            b'J' => DexValue::Long(bits as i64),
            b'F' => DexValue::Float(context.arg_f32(0)),
            b'D' => DexValue::Long(context.arg_f64(0).to_bits() as i64),
            _ => frame.value(context.arg_ptr(3))?,
        };
        frame.program.write_stored_static_field(&field, value);
        Ok(())
    });
}

fn register_natives(context: &mut CallContext) {
    // cite: art jni_internal.cc#RegisterNatives, ReportInvalidJNINativeMethod
    with_env(context, |frame, context| {
        let class = frame.class(context.arg_ptr(1))?;
        let rows = context.arg_ptr(2);
        let count = context.arg_i32(3);
        if !(0..=MAX_NATIVE_METHODS as i32).contains(&count) || (count > 0 && rows == 0) {
            return Err(WinedroidError::new("invalid JNI native method table"));
        }
        let width = 3 * std::mem::size_of::<usize>();
        rows.checked_add(count as usize * width)
            .ok_or_else(|| WinedroidError::new("JNI native method table overflow"))?;

        for index in 0..count as usize {
            let row = rows + index * width;
            let [name, signature, address] = read_value::<[usize; 3]>(row);
            if name == 0 || signature == 0 || address == 0 {
                frame.throw(
                    "Ljava/lang/NoSuchMethodError;",
                    format!("invalid native method at index {index}"),
                );
                context.ret_i32(JNI_ERR);
                return Ok(());
            }
            let name = bounded_name(name)?;
            let signature = bounded_name(signature)?;
            let signature = signature.strip_prefix('!').unwrap_or(&signature);
            let resolved = frame
                .program
                .find_jni_method(&class, &name, signature, None)?;
            let Some((key, _)) = resolved.filter(|(_, flags)| flags & DEX_ACC_NATIVE != 0) else {
                frame.throw(
                    "Ljava/lang/NoSuchMethodError;",
                    format!("{class}.{name}{signature}"),
                );
                context.ret_i32(JNI_ERR);
                return Ok(());
            };
            frame
                .bindings
                .borrow_mut()
                .insert(NativeMethod::from(&key), address);
        }
        context.ret_i32(JNI_OK);
        Ok(())
    });
}

fn call_void_method_v(context: &mut CallContext) {
    // cite: art jni_internal.cc#CallVoidMethodV
    // cite: art reflection.cc#InvokeVirtualOrInterfaceWithVarArgs, ArgArray, BuildArgArrayFromVarArgs
    with_env(context, |frame, context| {
        let receiver = frame.value(context.arg_ptr(1))?;
        if receiver == DexValue::Null {
            return Err(WinedroidError::new("null JNI receiver"));
        }
        let method = frame
            .program
            .jni
            .methods_by_id
            .borrow()
            .get(&context.arg_ptr(2))
            .cloned()
            .ok_or_else(|| WinedroidError::new("unknown JNI method ID"))?;
        if method.proto.return_type != "V" {
            return Err(WinedroidError::new(format!(
                "CallVoidMethodV on non-void method {}",
                method.display_signature()
            )));
        }
        let mut va_list = context.va_list_arg(3)?;
        let arguments = decode_va_arguments(
            frame,
            receiver.clone(),
            &method.proto.parameters,
            &mut va_list,
        )?;
        let class = value_class_descriptor(&receiver)
            .ok_or_else(|| WinedroidError::new("JNI receiver is not an object"))?
            .to_owned();
        let Some(resolved) = frame
            .program
            .resolve_exact_method_in_class_hierarchy(&class, &method)?
        else {
            return Err(WinedroidError::new(format!(
                "next missing primitive: JNI CallVoidMethodV target {class}.{}",
                method.display_signature()
            )));
        };
        match frame
            .program
            .execute_method(None, &resolved, Some(&arguments), frame.depth + 1)?
        {
            DexExecutionResult::Throw(exception) => {
                *frame.exception.borrow_mut() = Some(exception);
            }
            DexExecutionResult::Parked(execution) => {
                frame.program.discard_parked_execution(&execution);
                return Err(WinedroidError::new(
                    "next missing primitive: JNI upcall that parks",
                ));
            }
            DexExecutionResult::Void
            | DexExecutionResult::Int(_)
            | DexExecutionResult::Value(_) => {}
        }
        Ok(())
    });
}

fn call_static_method_v<const RETURN: u8>(context: &mut CallContext) {
    // cite: art jni_internal.cc#CallStaticLongMethodV, CallStaticVoidMethodV
    // cite: art reflection.cc#InvokeWithVarArgs, BuildArgArrayFromVarArgs
    with_env(context, |frame, context| {
        let method = frame
            .program
            .jni
            .methods_by_id
            .borrow()
            .get(&context.arg_ptr(2))
            .cloned()
            .ok_or_else(|| WinedroidError::new("unknown JNI method ID"))?;
        if method.proto.return_type.as_bytes() != [RETURN] {
            return Err(WinedroidError::new(format!(
                "CallStatic{}MethodV on {}",
                char::from(RETURN),
                method.display_signature()
            )));
        }
        let mut va_list = context.va_list_arg(3)?;
        let mut arguments = decode_va_arguments(
            frame,
            DexValue::Null,
            &method.proto.parameters,
            &mut va_list,
        )?;
        arguments.remove(0);
        let Some(resolved) = frame.program.resolve_exact_method(&method)? else {
            return Err(WinedroidError::new(format!(
                "next missing primitive: JNI static upcall target {}.{}",
                method.class_descriptor,
                method.display_signature()
            )));
        };
        match frame
            .program
            .execute_method(None, &resolved, Some(&arguments), frame.depth + 1)?
        {
            DexExecutionResult::Value(DexValue::Long(value)) if RETURN == b'J' => {
                context.ret_i64(value)
            }
            DexExecutionResult::Void if RETURN == b'V' => {}
            DexExecutionResult::Throw(exception) => {
                *frame.exception.borrow_mut() = Some(exception);
                if RETURN == b'J' {
                    context.ret_i64(0);
                }
            }
            DexExecutionResult::Parked(execution) => {
                frame.program.discard_parked_execution(&execution);
                return Err(WinedroidError::new(
                    "next missing primitive: JNI upcall that parks",
                ));
            }
            other => {
                return Err(WinedroidError::new(format!(
                    "JNI static upcall {} returned {other}",
                    method.display_signature()
                )));
            }
        }
        Ok(())
    });
}

fn decode_va_arguments(
    frame: &Frame<'_>,
    receiver: DexValue,
    parameters: &[String],
    va_list: &mut GuestVaList<'_>,
) -> Result<Vec<DexValue>> {
    // cite: art reflection.cc#BuildArgArrayFromVarArgs
    let mut arguments = vec![receiver];
    for parameter in parameters {
        let value = match parameter.as_str() {
            "Z" | "B" | "C" | "S" | "I" => DexValue::Int(va_list.next_i32()),
            "J" => DexValue::Long(va_list.next_i64()),
            "F" => DexValue::Float(va_list.next_f64() as f32),
            "D" => DexValue::Long(va_list.next_f64().to_bits() as i64),
            descriptor if descriptor.starts_with(['L', '[']) => frame.value(va_list.next_ptr())?,
            _ => {
                return Err(WinedroidError::new(format!(
                    "invalid JNI parameter descriptor {parameter}"
                )))
            }
        };
        arguments.push(value.clone());
        if matches!(parameter.as_str(), "J" | "D") {
            arguments.push(value);
        }
    }

    Ok(arguments)
}

fn exception_occurred(context: &mut CallContext) {
    // cite: art jni_internal.cc#ExceptionOccurred
    with_env(context, |frame, context| {
        context.ret_ptr(frame.exception().map_or(0, |value| frame.reference(value)));
        Ok(())
    });
}

fn exception_clear(context: &mut CallContext) {
    // cite: art jni_internal.cc#ExceptionClear
    with_env(context, |frame, _| {
        frame.exception.borrow_mut().take();
        Ok(())
    });
}

fn exception_check(context: &mut CallContext) {
    // cite: art jni_internal.cc#ExceptionCheck
    with_env(context, |frame, context| {
        context.ret_u32(frame.exception.borrow().is_some() as u32);
        Ok(())
    });
}

fn delete_local_ref(context: &mut CallContext) {
    // cite: art jni_internal.cc#DeleteLocalRef
    with_env(context, |frame, context| {
        frame.refs.borrow_mut().remove(&context.arg_ptr(1));
        Ok(())
    });
}

fn new_global_ref(context: &mut CallContext) {
    // cite: art jni_internal.cc#NewGlobalRef
    with_env(context, |frame, context| {
        let value = frame.value(context.arg_ptr(1))?;
        if value == DexValue::Null {
            context.ret_ptr(0);
            return Ok(());
        }
        let handle = next_reference();
        frame.program.jni.globals.borrow_mut().insert(handle, value);
        context.ret_ptr(handle);
        Ok(())
    });
}

fn delete_global_ref(context: &mut CallContext) {
    // cite: art jni_internal.cc#DeleteGlobalRef
    with_env(context, |frame, context| {
        frame
            .program
            .jni
            .globals
            .borrow_mut()
            .remove(&context.arg_ptr(1));
        Ok(())
    });
}

fn get_java_vm(context: &mut CallContext) {
    // cite: art jni_internal.cc#GetJavaVM
    with_env(context, |_, context| {
        let output = context.arg_ptr(1);
        if output == 0 {
            return Err(WinedroidError::new("null JNI JavaVM output"));
        }
        write_value(output, vm_pointer());
        context.ret_i32(JNI_OK);
        Ok(())
    });
}

fn new_string_utf(context: &mut CallContext) {
    // cite: art jni_internal.cc#NewStringUTF
    with_env(context, |frame, context| {
        let address = context.arg_ptr(1);
        if address == 0 {
            context.ret_ptr(0);
            return Ok(());
        }
        let bytes = cstr_bytes_bounded(address, MAX_NEW_STRING_UTF_BYTES);
        if bytes.len() == MAX_NEW_STRING_UTF_BYTES {
            return Err(WinedroidError::new("unterminated or oversized JNI string"));
        }
        let value = decode_new_string_utf(&bytes)?;
        context.ret_ptr(frame.reference(DexValue::String(Rc::from(value))));
        Ok(())
    });
}

fn decode_new_string_utf(bytes: &[u8]) -> Result<String> {
    // cite: art jni_internal.cc#VisitUtf8Chars, DecodeModifiedUtf8Character, kBadUtf8ReplacementChar, NewStringUTFVisitor
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let lead = u32::from(bytes[index]);
        let length = if lead & 0x80 == 0 {
            1
        } else if lead & 0x20 == 0 {
            2
        } else if lead & 0x10 == 0 {
            3
        } else {
            4
        };
        if bytes.len() - index < length {
            units.push(u16::from(b'?'));
            break;
        }
        let tail = |offset: usize| u32::from(bytes[index + offset]) & 0x3f;
        match length {
            1 => units.push(lead as u16),
            2 => units.push((((lead & 0x1f) << 6) | tail(1)) as u16),
            3 => units.push((((lead & 0x0f) << 12) | (tail(1) << 6) | tail(2)) as u16),
            _ => {
                let code_point = ((lead & 0x0f) << 18) | (tail(1) << 12) | (tail(2) << 6) | tail(3);
                let bits = code_point.wrapping_sub(0x10000);
                units.push((0xd800 | ((bits >> 10) & 0x3ff)) as u16);
                units.push((0xdc00 | (bits & 0x3ff)) as u16);
            }
        }
        index += length;
    }

    String::from_utf16(&units).map_err(|error| {
        WinedroidError::with_source(
            "next missing primitive: NewStringUTF with an unpaired UTF-16 surrogate",
            error,
        )
    })
}

fn get_string_utf_chars(context: &mut CallContext) {
    // cite: art jni_internal.cc#GetStringUTFChars
    with_env(context, |frame, context| {
        let handle = context.arg_ptr(1);
        let DexValue::String(value) = frame.value(handle)? else {
            return Err(WinedroidError::new("JNI reference is not a string"));
        };
        let mut bytes = Vec::with_capacity(value.len() + 1);
        for byte in value.bytes() {
            if byte == 0 {
                bytes.extend([0xc0, 0x80]);
            } else {
                bytes.push(byte);
            }
        }
        bytes.push(0);
        let address = bytes.as_ptr() as usize;
        frame.strings.borrow_mut().insert(address, (handle, bytes));
        let is_copy = context.arg_ptr(2);
        if is_copy != 0 {
            write_value(is_copy, 1u8);
        }
        context.ret_ptr(address);
        Ok(())
    });
}

fn release_string_utf_chars(context: &mut CallContext) {
    // cite: art jni_internal.cc#ReleaseStringUTFChars
    with_env(context, |frame, context| {
        let address = context.arg_ptr(2);
        let mut strings = frame.strings.borrow_mut();
        let Some((handle, _)) = strings.get(&address) else {
            return Err(WinedroidError::new("unknown JNI string buffer"));
        };
        if *handle != context.arg_ptr(1) {
            return Err(WinedroidError::new(
                "JNI string buffer belongs to another reference",
            ));
        }
        strings.remove(&address);
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::va_list::GuestVaListRaw;

    #[test]
    fn art_call_void_method_v_decodes_stack_varargs_by_shorty_with_float_promoted_to_double() {
        // cite: art reflection.cc#BuildArgArrayFromVarArgs
        let program = DexProgram::new(Vec::new());
        let frame = Frame::new(&program, 0);
        let text = frame.reference(DexValue::String(Rc::from("text")));
        let slots: [u64; 6] = [
            0xffff_ffff_0000_0007,
            0x1_0000_0002,
            f64::from(1.5f32).to_bits(),
            (-2.25f64).to_bits(),
            text as u64,
            0x2a,
        ];
        let raw = GuestVaListRaw {
            stack: slots.as_ptr() as u64,
            gr_top: 0,
            vr_top: 0,
            gr_offs: 0,
            vr_offs: 0,
        };
        let mut va_list = GuestVaList::from_guest(std::ptr::from_ref(&raw) as u64).unwrap();
        let parameters = ["I", "J", "F", "D", "Ljava/lang/String;", "Z"]
            .map(str::to_owned)
            .to_vec();

        let arguments =
            decode_va_arguments(&frame, DexValue::Int(9), &parameters, &mut va_list).unwrap();

        assert_eq!(
            arguments,
            vec![
                DexValue::Int(9),
                DexValue::Int(7),
                DexValue::Long(0x1_0000_0002),
                DexValue::Long(0x1_0000_0002),
                DexValue::Float(1.5),
                DexValue::Long((-2.25f64).to_bits() as i64),
                DexValue::Long((-2.25f64).to_bits() as i64),
                DexValue::String(Rc::from("text")),
                DexValue::Int(0x2a),
            ]
        );
    }

    #[test]
    fn art_call_void_method_v_refuses_an_unknown_reference_argument() {
        // cite: art reflection.cc#BuildArgArrayFromVarArgs
        let program = DexProgram::new(Vec::new());
        let frame = Frame::new(&program, 0);
        let slots: [u64; 1] = [0xdead_beef];
        let raw = GuestVaListRaw {
            stack: slots.as_ptr() as u64,
            gr_top: 0,
            vr_top: 0,
            gr_offs: 0,
            vr_offs: 0,
        };
        let mut va_list = GuestVaList::from_guest(std::ptr::from_ref(&raw) as u64).unwrap();
        let parameters = vec!["Ljava/lang/String;".to_owned()];

        assert!(decode_va_arguments(&frame, DexValue::Int(9), &parameters, &mut va_list).is_err());
    }

    #[test]
    fn art_new_string_utf_decodes_modified_utf8_nul_and_four_byte_sequences() {
        // cite: art jni_internal.cc#NewStringUTF, DecodeModifiedUtf8Character
        assert_eq!(decode_new_string_utf(b"abc").unwrap(), "abc");
        assert_eq!(decode_new_string_utf(&[0xc0, 0x80]).unwrap(), "\0");
        assert_eq!(
            decode_new_string_utf("\u{e9}\u{20ac}".as_bytes()).unwrap(),
            "\u{e9}\u{20ac}"
        );
        assert_eq!(
            decode_new_string_utf("\u{1f600}".as_bytes()).unwrap(),
            "\u{1f600}"
        );
    }

    #[test]
    fn art_new_string_utf_replaces_a_truncated_sequence_with_a_question_mark() {
        // cite: art jni_internal.cc#VisitUtf8Chars, kBadUtf8ReplacementChar
        assert_eq!(decode_new_string_utf(&[b'a', 0xe2, 0x82]).unwrap(), "a?");
    }

    #[test]
    fn art_new_string_utf_refuses_an_unpaired_surrogate_the_string_model_cannot_hold() {
        // cite: art jni_internal.cc#NewStringUTF
        assert!(decode_new_string_utf(&[0xed, 0xa0, 0x80]).is_err());
    }
    #[test]
    fn art_get_array_length_reads_guest_array_and_rejects_invalid_references() {
        // cite: art jni_internal.cc#GetArrayLength
        let program = DexProgram::new(Vec::new());
        let frame = Frame::new(&program, 0);
        let array = program
            .allocate_filled_array("[I".to_owned(), vec![DexValue::Int(3), DexValue::Int(7)]);
        let array_ref = frame.reference(array);
        let non_array_ref = frame.reference(DexValue::String(Rc::from("not an array")));
        let function = env_table()[171];
        for (handle, expected) in [
            (array_ref, 2),
            (
                frame.reference(program.allocate_filled_array("[I".to_owned(), vec![])),
                0,
            ),
        ] {
            let result = frame
                .call(CallBuilder::new(function).ptr(env_pointer()).ptr(handle))
                .unwrap();
            assert_eq!(result.i32(), expected);
        }
        for handle in [0, non_array_ref, usize::MAX] {
            assert!(frame
                .call(CallBuilder::new(function).ptr(env_pointer()).ptr(handle))
                .is_err());
        }
    }

    #[test]
    fn art_get_object_array_element_returns_local_reference_and_throws_on_bad_index() {
        // cite: art jni_internal.cc#GetObjectArrayElement
        let program = DexProgram::new(Vec::new());
        let frame = Frame::new(&program, 0);
        let array = program.allocate_filled_array(
            "[Ljava/lang/String;".to_owned(),
            vec![DexValue::String(Rc::from("hello")), DexValue::Null],
        );
        let handle = frame.reference(array);
        let call = |index| {
            frame
                .call(
                    CallBuilder::new(env_table()[173])
                        .ptr(env_pointer())
                        .ptr(handle)
                        .i32(index),
                )
                .unwrap()
                .u64() as usize
        };
        assert_eq!(
            frame.value(call(0)).unwrap(),
            DexValue::String(Rc::from("hello"))
        );
        assert_eq!(call(1), 0);
        assert_eq!(call(2), 0);
        assert!(program.value_matches_type(
            &frame.exception().unwrap(),
            "Ljava/lang/ArrayIndexOutOfBoundsException;"
        ));
        let primitive =
            frame.reference(program.allocate_filled_array("[I".to_owned(), vec![DexValue::Int(1)]));
        assert!(frame
            .call(
                CallBuilder::new(env_table()[173])
                    .ptr(env_pointer())
                    .ptr(primitive)
                    .i32(0)
            )
            .is_err());
    }

    const STATICS: &str = "Lwinedroid/tests/JniStatics;";

    fn statics() -> DexProgram {
        let dex = dex::parse_dex_file(
            "jni_static_fields.dex",
            include_bytes!("../tests/fixtures/jni_static_fields.dex").to_vec(),
        )
        .unwrap();
        DexProgram::new(vec![dex])
    }

    fn static_field_id(
        frame: &Frame<'_>,
        class: &str,
        name: &str,
        signature: &str,
    ) -> Result<usize> {
        let name = std::ffi::CString::new(name).unwrap();
        let signature = std::ffi::CString::new(signature).unwrap();
        let call = CallBuilder::new(env_table()[144])
            .ptr(env_pointer())
            .ptr(frame.class_reference(class))
            .ptr(name.as_ptr() as usize)
            .ptr(signature.as_ptr() as usize);
        Ok(frame.call(call)?.u64() as usize)
    }

    fn static_access(slot: usize, id: usize) -> CallBuilder {
        CallBuilder::new(env_table()[slot])
            .ptr(env_pointer())
            .ptr(0)
            .ptr(id)
    }

    #[test]
    fn art_get_static_field_reads_every_jni_type_from_the_declaring_class() {
        // cite: art jni_internal.cc#GetStaticFieldID, GetStaticObjectField, GET_STATIC_PRIMITIVE_FIELD
        // cite: art class.cc#FindStaticField
        let program = statics();
        let frame = Frame::new(&program, 0);
        let get = |slot, name, signature| {
            let id = static_field_id(&frame, STATICS, name, signature).unwrap();
            assert_ne!(id, 0, "{name}");
            frame.call(static_access(slot, id)).unwrap()
        };
        let object = |name, signature| {
            let handle = get(145, name, signature).u64() as usize;
            frame.value(handle).unwrap()
        };

        assert_eq!(get(146, "flag", "Z").u64() as u8, 1);
        assert_eq!(get(147, "small", "B").u64() as i8, -2);
        assert_eq!(get(148, "letter", "C").u64() as u16, u16::from(b'w'));
        assert_eq!(get(149, "half", "S").u64() as i16, -300);
        assert_eq!(get(150, "count", "I").i32(), 7);
        assert_eq!(get(151, "big", "J").u64(), 0x1_0000_0002);
        assert_eq!(get(152, "ratio", "F").f32(), 1.5);
        assert_eq!(get(153, "precise", "D").f64(), 0.0);
        let ratio = static_field_id(&frame, STATICS, "ratio", "F").unwrap();
        program.write_stored_static_field(
            &program.jni.field_ids.borrow()[ratio - 1].clone(),
            DexValue::Int(0x4020_0000),
        );
        assert_eq!(get(152, "ratio", "F").f32(), 2.5);
        assert_eq!(
            object("text", "Ljava/lang/String;"),
            DexValue::String(Rc::from("dex"))
        );
        assert_eq!(
            object("inherited", "Ljava/lang/String;"),
            DexValue::String(Rc::from("base"))
        );
        assert_eq!(object("numbers", "[I"), DexValue::Null);
        assert_ne!(
            static_field_id(&frame, STATICS, "LIMIT", "I").unwrap(),
            0,
            "interface field"
        );
        assert_eq!(
            static_field_id(&frame, STATICS, "count", "I").unwrap(),
            static_field_id(&frame, STATICS, "count", "I").unwrap()
        );
    }

    #[test]
    fn art_set_static_field_stores_every_jni_type_in_the_declaring_class() {
        // cite: art jni_internal.cc#SetStaticObjectField, SET_STATIC_PRIMITIVE_FIELD
        let program = statics();
        let frame = Frame::new(&program, 0);
        let set = |slot, name: &str, signature: &str, value: fn(CallBuilder) -> CallBuilder| {
            let id = static_field_id(&frame, STATICS, name, signature).unwrap();
            frame.call(value(static_access(slot, id))).unwrap();
            program.read_stored_static_field(&DexFieldId {
                class_descriptor: STATICS.to_owned(),
                type_descriptor: signature.to_owned(),
                name: name.to_owned(),
            })
        };

        assert_eq!(
            set(155, "flag", "Z", |call| call.int(0x100)),
            Some(DexValue::Int(0))
        );
        assert_eq!(
            set(156, "small", "B", |call| call.int(0xff)),
            Some(DexValue::Int(-1))
        );
        assert_eq!(
            set(157, "letter", "C", |call| call.int(0x1_ffff)),
            Some(DexValue::Int(0xffff))
        );
        assert_eq!(
            set(158, "half", "S", |call| call.int(0x8000)),
            Some(DexValue::Int(-0x8000))
        );
        assert_eq!(
            set(159, "count", "I", |call| call.i32(-5)),
            Some(DexValue::Int(-5))
        );
        assert_eq!(
            set(160, "big", "J", |call| call.int(u64::MAX - 1)),
            Some(DexValue::Long(-2))
        );
        assert_eq!(
            set(161, "ratio", "F", |call| call.f32(2.5)),
            Some(DexValue::Float(2.5))
        );
        assert_eq!(
            set(162, "precise", "D", |call| call.f64(-0.5)),
            Some(DexValue::Long((-0.5f64).to_bits() as i64))
        );
        assert_eq!(
            set(154, "text", "Ljava/lang/String;", |call| call.ptr(0)),
            Some(DexValue::Null)
        );
        let text = static_field_id(&frame, STATICS, "text", "Ljava/lang/String;").unwrap();
        let value = frame.reference(DexValue::String(Rc::from("native")));
        frame.call(static_access(154, text).ptr(value)).unwrap();
        let read = frame.call(static_access(145, text)).unwrap().u64() as usize;
        assert_eq!(
            frame.value(read).unwrap(),
            DexValue::String(Rc::from("native"))
        );
    }

    #[test]
    fn art_static_field_access_refuses_absent_unmodelled_and_mistyped_fields() {
        // cite: art jni_internal.cc#FindFieldJNI, GetStaticFieldID
        let program = statics();
        let frame = Frame::new(&program, 0);
        for (name, signature) in [("count", "J"), ("instance", "I"), ("absent", "I")] {
            assert_eq!(
                static_field_id(&frame, STATICS, name, signature).unwrap(),
                0,
                "{name}"
            );
            assert!(program.value_matches_type(
                &frame.exception.borrow_mut().take().unwrap(),
                "Ljava/lang/NoSuchFieldError;"
            ));
        }
        let sdk = static_field_id(&frame, "Landroid/os/Build$VERSION;", "SDK_INT", "I").unwrap();
        assert_eq!(
            Some(DexValue::Int(
                frame.call(static_access(150, sdk)).unwrap().i32()
            )),
            bootstrap_static_field(&program.jni.field_ids.borrow()[sdk - 1])
        );
        assert!(static_field_id(&frame, "Ljava/lang/Thread;", "absent", "I").is_err());
        let count = static_field_id(&frame, STATICS, "count", "I").unwrap();
        assert!(frame.call(static_access(151, count)).is_err());
        for id in [0, usize::MAX] {
            assert!(frame.call(static_access(150, id)).is_err());
        }
    }
}
