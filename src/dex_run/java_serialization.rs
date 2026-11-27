use super::*;

// cite: libcore ObjectStreamConstants.java#STREAM_MAGIC, STREAM_VERSION, TC_NULL, TC_REFERENCE, TC_CLASSDESC, TC_OBJECT, TC_STRING, TC_BLOCKDATA, TC_ENDBLOCKDATA, TC_BLOCKDATALONG, TC_LONGSTRING, baseWireHandle, SC_WRITE_METHOD, SC_SERIALIZABLE
const STREAM_MAGIC: u16 = 0xaced;
const STREAM_VERSION: u16 = 5;
const TC_NULL: u8 = 0x70;
const TC_REFERENCE: u8 = 0x71;
const TC_CLASSDESC: u8 = 0x72;
const TC_OBJECT: u8 = 0x73;
const TC_STRING: u8 = 0x74;
const TC_BLOCKDATA: u8 = 0x77;
const TC_ENDBLOCKDATA: u8 = 0x78;
const TC_BLOCKDATALONG: u8 = 0x7a;
const TC_LONGSTRING: u8 = 0x7c;
const BASE_WIRE_HANDLE: i32 = 0x7e0000;
const SC_WRITE_METHOD: u8 = 0x01;
const SC_SERIALIZABLE: u8 = 0x02;
const READ_CHUNK: usize = 8192;

const OBJECT_OUTPUT_STREAM: &str = "Ljava/io/ObjectOutputStream;";
const OBJECT_INPUT_STREAM: &str = "Ljava/io/ObjectInputStream;";

#[derive(Debug)]
pub(super) struct StreamClassDesc {
    name: String,
    suid: i64,
    flags: u8,
    fields: Vec<StreamField>,
    super_desc: Option<Rc<StreamClassDesc>>,
}

#[derive(Debug)]
struct StreamField {
    type_code: u8,
    name: String,
}

#[derive(Debug)]
pub(super) enum InputHandle {
    Object(DexValue),
    Class(Rc<StreamClassDesc>),
    Pending,
    Failed(String),
}

pub(super) struct ReadState {
    handles: Vec<InputHandle>,
    class_not_found: Option<String>,
}

struct LocalClass {
    descriptor: String,
    name: String,
    suid: i64,
    fields: Vec<LocalField>,
}

struct LocalField {
    field: DexFieldId,
    type_code: u8,
}

struct LocalHierarchy {
    classes: Vec<LocalClass>,
    constructor_class: String,
}

type Thrown<T> = std::result::Result<T, DexValue>;

fn java_class_name(descriptor: &str) -> String {
    descriptor
        .strip_prefix('L')
        .and_then(|name| name.strip_suffix(';'))
        .unwrap_or(descriptor)
        .replace('/', ".")
}

fn type_code(descriptor: &str) -> u8 {
    descriptor.as_bytes().first().copied().unwrap_or(b'L')
}

fn primitive_width(type_code: u8) -> Option<usize> {
    // cite: libcore ObjectStreamClass.java#computeFieldOffsets
    match type_code {
        b'Z' | b'B' => Some(1),
        b'C' | b'S' => Some(2),
        b'I' | b'F' => Some(4),
        b'J' | b'D' => Some(8),
        _ => None,
    }
}

// cite: libcore DataOutputStream.java#writeUTF
pub(super) fn modified_utf8(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len());
    for unit in text.encode_utf16() {
        match unit {
            0x0001..=0x007f => bytes.push(unit as u8),
            0x0000 | 0x0080..=0x07ff => {
                bytes.push(0xc0 | (unit >> 6) as u8);
                bytes.push(0x80 | (unit & 0x3f) as u8);
            }
            _ => {
                bytes.push(0xe0 | (unit >> 12) as u8);
                bytes.push(0x80 | ((unit >> 6) & 0x3f) as u8);
                bytes.push(0x80 | (unit & 0x3f) as u8);
            }
        }
    }
    bytes
}

// cite: libcore DataInputStream.java#readUTF
fn decode_modified_utf8(bytes: &[u8]) -> Option<String> {
    let mut units = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes[index];
        let unit = if first & 0x80 == 0 {
            index += 1;
            u16::from(first)
        } else if first & 0xe0 == 0xc0 {
            let second = *bytes.get(index + 1)?;
            if second & 0xc0 != 0x80 {
                return None;
            }
            index += 2;
            (u16::from(first & 0x1f) << 6) | u16::from(second & 0x3f)
        } else if first & 0xf0 == 0xe0 {
            let second = *bytes.get(index + 1)?;
            let third = *bytes.get(index + 2)?;
            if second & 0xc0 != 0x80 || third & 0xc0 != 0x80 {
                return None;
            }
            index += 3;
            (u16::from(first & 0x0f) << 12)
                | (u16::from(second & 0x3f) << 6)
                | u16::from(third & 0x3f)
        } else {
            return None;
        };
        units.push(unit);
    }
    Some(String::from_utf16_lossy(&units))
}

fn is_framework_descriptor(descriptor: &str) -> bool {
    is_bootstrap_class_descriptor(descriptor)
        || [
            "Ljava/",
            "Ljavax/",
            "Landroid/",
            "Ldalvik/",
            "Lsun/",
            "Llibcore/",
            "Lcom/android/",
            "Lorg/json/",
            "Lorg/xml/",
            "Lorg/w3c/",
            "Lorg/apache/harmony/",
        ]
        .iter()
        .any(|prefix| descriptor.starts_with(prefix))
}

fn missing(detail: String) -> WinedroidError {
    WinedroidError::new(format!("next missing primitive: {detail}"))
}

impl DexProgram {
    // cite: libcore ObjectOutputStream.java#ObjectOutputStream, writeObject, close, flush
    // cite: libcore ObjectInputStream.java#ObjectInputStream, readObject, close
    pub(super) fn object_stream_intrinsic(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        let output = self.value_matches_type(receiver, OBJECT_OUTPUT_STREAM);
        let input = self.value_matches_type(receiver, OBJECT_INPUT_STREAM);
        if !(output || input) || self.guest_overrides(method, Some(invocation)) {
            return Ok(None);
        }
        if let Some(class) = value_class_descriptor(receiver) {
            if self.load_class(class).is_some() {
                return Err(missing(format!(
                    "object stream subclass {} with overridable stream hooks",
                    java_class_name(class)
                )));
            }
        }
        let descriptor = method.proto.descriptor();
        let result = match (method.name.as_str(), descriptor.as_str(), output) {
            ("<init>", "(Ljava/io/OutputStream;)V", true)
                if method.class_descriptor == OBJECT_OUTPUT_STREAM =>
            {
                self.object_output_init(receiver, invocation.arguments.get(1), depth)?
            }
            ("writeObject", "(Ljava/lang/Object;)V", true) => {
                self.object_output_write(receiver, invocation.arguments.get(1), depth)?
            }
            ("flush", "()V", true) => self.object_output_flush(receiver, depth)?,
            ("close", "()V", true) => self.object_stream_close(receiver, depth)?,
            ("<init>", "(Ljava/io/InputStream;)V", false)
                if method.class_descriptor == OBJECT_INPUT_STREAM =>
            {
                self.object_input_init(receiver, invocation.arguments.get(1), depth)?
            }
            ("readObject", "()Ljava/lang/Object;", false) => {
                self.object_input_read(receiver, depth)?
            }
            ("close", "()V", false) => self.object_stream_close(receiver, depth)?,
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn object_stream_field(&self, output: bool) -> DexFieldId {
        if output {
            DexFieldId {
                class_descriptor: OBJECT_OUTPUT_STREAM.to_owned(),
                type_descriptor: "Ljava/io/OutputStream;".to_owned(),
                name: "out".to_owned(),
            }
        } else {
            DexFieldId {
                class_descriptor: OBJECT_INPUT_STREAM.to_owned(),
                type_descriptor: "Ljava/io/InputStream;".to_owned(),
                name: "in".to_owned(),
            }
        }
    }

    fn serial_exception(&self, class_descriptor: &str, detail: Option<String>) -> DexValue {
        self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: class_descriptor.to_owned(),
            detail_message: detail,
        })
    }

    fn object_output_init(
        &self,
        stream: &DexValue,
        out: Option<&DexValue>,
        depth: usize,
    ) -> Result<DexExecutionResult> {
        let Some(out) = out.filter(|out| dex_value_is_object_like(out)) else {
            return Ok(self.null_pointer_exception_result());
        };
        self.write_stored_instance_field(stream, &self.object_stream_field(true), out.clone());
        self.runtime
            .borrow_mut()
            .object_output_handles
            .insert(map_identity_key(stream), Vec::new());
        // cite: libcore ObjectOutputStream.java#writeStreamHeader
        let mut header = STREAM_MAGIC.to_be_bytes().to_vec();
        header.extend(STREAM_VERSION.to_be_bytes());
        Ok(match self.output_stream_write(out, &header, depth)? {
            Ok(()) => DexExecutionResult::Void,
            Err(exception) => DexExecutionResult::Throw(exception),
        })
    }

    fn object_output_write(
        &self,
        stream: &DexValue,
        value: Option<&DexValue>,
        depth: usize,
    ) -> Result<DexExecutionResult> {
        let Some(out) = self
            .read_stored_instance_field(stream, &self.object_stream_field(true))
            .filter(dex_value_is_object_like)
        else {
            return Err(missing(
                "ObjectOutputStream.writeObject on a stream without an output target".to_owned(),
            ));
        };
        let value = value.cloned().unwrap_or(DexValue::Null);
        let stream_key = map_identity_key(stream);
        let mut handles = self
            .runtime
            .borrow_mut()
            .object_output_handles
            .remove(&stream_key)
            .unwrap_or_default();
        let mut bytes = Vec::new();
        let written = self.write_object0(&mut bytes, &mut handles, &value, depth);
        self.runtime
            .borrow_mut()
            .object_output_handles
            .insert(stream_key, handles);
        if let Err(exception) = written? {
            return Err(missing(format!(
                "ObjectOutputStream.writeFatalException for {}",
                value_class_descriptor(&exception).unwrap_or("unknown exception")
            )));
        }
        // cite: libcore ObjectOutputStream.java#BlockDataOutputStream, drain, MAX_BLOCK_SIZE
        for chunk in bytes.chunks(1024) {
            if let Err(exception) = self.output_stream_write(&out, chunk, depth)? {
                return Ok(DexExecutionResult::Throw(exception));
            }
        }
        Ok(DexExecutionResult::Void)
    }

    // cite: libcore ObjectOutputStream.java#flush
    // cite: libcore OutputStream.java#flush
    fn object_output_flush(&self, stream: &DexValue, depth: usize) -> Result<DexExecutionResult> {
        let Some(out) = self
            .read_stored_instance_field(stream, &self.object_stream_field(true))
            .filter(dex_value_is_object_like)
        else {
            return Ok(DexExecutionResult::Void);
        };
        if let Some(result) = self.guest_void_override(&out, "flush", &[], Vec::new(), depth)? {
            return Ok(result);
        }
        match self.first_framework_class(&out).as_deref() {
            Some("Ljava/io/OutputStream;" | "Ljava/io/FileOutputStream;") => {
                Ok(DexExecutionResult::Void)
            }
            other => Err(missing(format!(
                "flush of the stream under an ObjectOutputStream: {}",
                other.unwrap_or("unknown class")
            ))),
        }
    }

    fn first_framework_class(&self, value: &DexValue) -> Option<String> {
        let mut class = value_class_descriptor(value).map(str::to_owned);
        while let Some(descriptor) = class.as_deref() {
            if self.load_class(descriptor).is_none() {
                break;
            }
            class = self.superclass_descriptor(descriptor);
        }
        class
    }

    fn object_stream_close(&self, stream: &DexValue, depth: usize) -> Result<DexExecutionResult> {
        let output = self.value_matches_type(stream, OBJECT_OUTPUT_STREAM);
        let Some(target) = self
            .read_stored_instance_field(stream, &self.object_stream_field(output))
            .filter(dex_value_is_object_like)
        else {
            return Ok(DexExecutionResult::Void);
        };
        if let Some(result) = self.guest_void_override(&target, "close", &[], Vec::new(), depth)? {
            return Ok(result);
        }
        // cite: libcore OutputStream.java#close
        // cite: libcore InputStream.java#close
        let framework_class = self.first_framework_class(&target);
        if matches!(
            framework_class.as_deref(),
            Some("Ljava/io/OutputStream;" | "Ljava/io/InputStream;")
        ) {
            return Ok(DexExecutionResult::Void);
        }
        if output {
            if self.file_output_stream_is_open_by_path(&target) {
                if let Some(result) = self.file_output_stream_close(&target) {
                    return Ok(result);
                }
            }
        } else if let Some(result) = self.builtin_stream_close(&target, depth)? {
            return Ok(result);
        }
        Err(missing(format!(
            "close of the stream under an object stream: {}",
            value_class_descriptor(&target).unwrap_or("unknown class")
        )))
    }

    fn output_stream_write(
        &self,
        out: &DexValue,
        bytes: &[u8],
        depth: usize,
    ) -> Result<Thrown<()>> {
        let array = self.allocate_filled_array(
            "[B".to_owned(),
            bytes
                .iter()
                .map(|byte| DexValue::Int(i32::from(*byte as i8)))
                .collect(),
        );
        let length = DexValue::Int(i32::try_from(bytes.len()).unwrap_or(i32::MAX));
        let result = match self.guest_void_override(
            out,
            "write",
            &["[B", "I", "I"],
            vec![array.clone(), DexValue::Int(0), length.clone()],
            depth,
        )? {
            Some(result) => Some(result),
            None if self.file_output_stream_is_open_by_path(out) => {
                let DexValue::Int(count) = length else {
                    unreachable!("length is an int")
                };
                self.file_output_stream_write_bytes(out, &array, 0, count)
            }
            None => None,
        };
        match result {
            Some(DexExecutionResult::Throw(exception)) => Ok(Err(exception)),
            Some(DexExecutionResult::Parked(execution)) => {
                self.discard_parked_execution(&execution);
                Err(missing(
                    "ObjectOutputStream write to a stream that parks".to_owned(),
                ))
            }
            Some(_) => Ok(Ok(())),
            None => Err(missing(format!(
                "ObjectOutputStream write to {}",
                value_class_descriptor(out).unwrap_or("unknown class")
            ))),
        }
    }

    // cite: libcore ObjectOutputStream.java#writeObject0, writeNull, writeHandle, writeString, writeOrdinaryObject
    fn write_object0(
        &self,
        bytes: &mut Vec<u8>,
        handles: &mut Vec<String>,
        value: &DexValue,
        depth: usize,
    ) -> Result<Thrown<()>> {
        if matches!(value, DexValue::Null | DexValue::Int(0)) {
            bytes.push(TC_NULL);
            return Ok(Ok(()));
        }
        let text = match value {
            DexValue::String(text) => Some(text.to_string()),
            _ if value_class_descriptor(value) == Some("Ljava/lang/String;") => {
                self.string_text(value)
            }
            _ => None,
        };
        let key = match value {
            DexValue::String(text) => format!("string:{text}"),
            _ => format!("object:{}", map_identity_key(value)),
        };
        if let Some(handle) = handles.iter().position(|known| *known == key) {
            bytes.push(TC_REFERENCE);
            bytes.extend((BASE_WIRE_HANDLE + handle as i32).to_be_bytes());
            return Ok(Ok(()));
        }
        if let Some(text) = text {
            handles.push(key);
            write_string(bytes, &text);
            return Ok(Ok(()));
        }
        let Some(class_descriptor) = value_class_descriptor(value).map(str::to_owned) else {
            return Err(missing(format!(
                "ObjectOutputStream.writeObject of {value}"
            )));
        };
        if self.load_class(&class_descriptor).is_none() {
            return Err(missing(format!(
                "serialization of framework class {}",
                java_class_name(&class_descriptor)
            )));
        }
        if !self.descriptor_matches_type(&class_descriptor, "Ljava/io/Serializable;") {
            return Ok(Err(self.serial_exception(
                "Ljava/io/NotSerializableException;",
                Some(java_class_name(&class_descriptor)),
            )));
        }
        let hierarchy = match self.local_serial_hierarchy(&class_descriptor, depth)? {
            Ok(hierarchy) => hierarchy,
            Err(exception) => return Ok(Err(exception)),
        };
        bytes.push(TC_OBJECT);
        self.write_class_desc(bytes, handles, &hierarchy.classes);
        handles.push(key);
        for class in hierarchy.classes.iter().rev() {
            if let Err(exception) = self.write_fields(bytes, handles, value, class, depth)? {
                return Ok(Err(exception));
            }
        }
        Ok(Ok(()))
    }

    // cite: libcore ObjectOutputStream.java#writeClassDesc, writeNonProxyDesc, writeTypeString
    // cite: libcore ObjectStreamClass.java#writeNonProxy
    fn write_class_desc(
        &self,
        bytes: &mut Vec<u8>,
        handles: &mut Vec<String>,
        chain: &[LocalClass],
    ) {
        let Some((class, supers)) = chain.split_first() else {
            bytes.push(TC_NULL);
            return;
        };
        let key = format!("class:{}", class.descriptor);
        if let Some(handle) = handles.iter().position(|known| *known == key) {
            bytes.push(TC_REFERENCE);
            bytes.extend((BASE_WIRE_HANDLE + handle as i32).to_be_bytes());
            return;
        }
        bytes.push(TC_CLASSDESC);
        handles.push(key);
        write_utf(bytes, &class.name);
        bytes.extend(class.suid.to_be_bytes());
        bytes.push(SC_SERIALIZABLE);
        bytes.extend((class.fields.len() as u16).to_be_bytes());
        for field in &class.fields {
            bytes.push(field.type_code);
            write_utf(bytes, &field.field.name);
            if primitive_width(field.type_code).is_none() {
                let signature = field.field.type_descriptor.clone();
                let string_key = format!("string:{signature}");
                match handles.iter().position(|known| *known == string_key) {
                    Some(handle) => {
                        bytes.push(TC_REFERENCE);
                        bytes.extend((BASE_WIRE_HANDLE + handle as i32).to_be_bytes());
                    }
                    None => {
                        handles.push(string_key);
                        write_string(bytes, &signature);
                    }
                }
            }
        }
        bytes.push(TC_ENDBLOCKDATA);
        self.write_class_desc(bytes, handles, supers);
    }

    // cite: libcore ObjectOutputStream.java#defaultWriteFields
    // cite: libcore ObjectStreamClass.java#getPrimFieldValues, getObjFieldValues
    fn write_fields(
        &self,
        bytes: &mut Vec<u8>,
        handles: &mut Vec<String>,
        object: &DexValue,
        class: &LocalClass,
        depth: usize,
    ) -> Result<Thrown<()>> {
        for field in &class.fields {
            let Some(width) = primitive_width(field.type_code) else {
                continue;
            };
            let value = self.read_stored_instance_field(object, &field.field);
            let bits: i64 = match value {
                Some(DexValue::Int(value)) => i64::from(value),
                Some(DexValue::Long(value)) => value,
                Some(DexValue::Float(value)) => i64::from(value.to_bits()),
                _ => 0,
            };
            bytes.extend(&bits.to_be_bytes()[8 - width..]);
        }
        for field in &class.fields {
            if primitive_width(field.type_code).is_some() {
                continue;
            }
            let value = self
                .read_stored_instance_field(object, &field.field)
                .unwrap_or(DexValue::Null);
            if let Err(exception) = self.write_object0(bytes, handles, &value, depth)? {
                return Ok(Err(exception));
            }
        }
        Ok(Ok(()))
    }

    // cite: libcore ObjectStreamClass.java#lookup, getDeclaredSUID, getDefaultSerialFields, getSuperDesc, getSerializableConstructor
    fn local_serial_hierarchy(
        &self,
        class_descriptor: &str,
        depth: usize,
    ) -> Result<Thrown<LocalHierarchy>> {
        let mut classes = Vec::new();
        let mut current = Some(class_descriptor.to_owned());
        while let Some(descriptor) = current {
            if !self.descriptor_matches_type(&descriptor, "Ljava/io/Serializable;") {
                return Ok(Ok(LocalHierarchy {
                    classes,
                    constructor_class: descriptor,
                }));
            }
            let Some(loaded) = self.load_class(&descriptor) else {
                return Err(missing(format!(
                    "serialization of framework class {}",
                    java_class_name(&descriptor)
                )));
            };
            match self.local_serial_class(&descriptor, &loaded, depth)? {
                Ok(class) => classes.push(class),
                Err(exception) => return Ok(Err(exception)),
            }
            current = loaded.class_def.superclass_descriptor.clone();
        }
        Err(missing(format!(
            "serializable class without a non-serializable superclass: {}",
            java_class_name(class_descriptor)
        )))
    }

    fn local_serial_class(
        &self,
        descriptor: &str,
        loaded: &LoadedClass,
        depth: usize,
    ) -> Result<Thrown<LocalClass>> {
        let name = java_class_name(descriptor);
        if self.descriptor_matches_type(descriptor, "Ljava/io/Externalizable;") {
            return Err(missing(format!("Externalizable class {name}")));
        }
        for encoded in loaded
            .class_def
            .direct_methods
            .iter()
            .chain(&loaded.class_def.virtual_methods)
        {
            let method = self.method_id(loaded.dex_file_index, encoded.method_index)?;
            if matches!(
                method.name.as_str(),
                "writeObject" | "readObject" | "readObjectNoData" | "writeReplace" | "readResolve"
            ) {
                return Err(missing(format!(
                    "serialization hook {name}.{}",
                    method.display_signature()
                )));
            }
        }
        let mut suid = None;
        let mut fields = Vec::new();
        for (index, encoded) in loaded.class_def.static_fields.iter().enumerate() {
            let field = self.field_id(loaded.dex_file_index, encoded.field_index)?;
            if field.name == "serialPersistentFields" {
                return Err(missing(format!("serialPersistentFields in {name}")));
            }
            if field.name == "serialVersionUID"
                && field.type_descriptor == "J"
                && encoded.access_flags & JAVA_ACC_FINAL != 0
            {
                match self.ensure_class_initialized(descriptor, depth)? {
                    ClassInitializationResult::Complete => {}
                    ClassInitializationResult::Throw(exception) => return Ok(Err(exception)),
                    ClassInitializationResult::Parked(execution) => {
                        self.discard_parked_execution(&execution);
                        return Err(missing(format!(
                            "class initialization that parks while serializing {name}"
                        )));
                    }
                }
                suid = match self.read_stored_static_field(&field) {
                    Some(DexValue::Long(value)) => Some(value),
                    _ => match loaded.class_def.static_values.get(index) {
                        Some(crate::dex::DexStaticValue::Long(value)) => Some(*value),
                        _ => Some(0),
                    },
                };
            }
        }
        for encoded in &loaded.class_def.instance_fields {
            if encoded.access_flags & (DEX_ACC_STATIC | JAVA_ACC_TRANSIENT) != 0 {
                continue;
            }
            let field = self.field_id(loaded.dex_file_index, encoded.field_index)?;
            fields.push(LocalField {
                type_code: type_code(&field.type_descriptor),
                field: (*field).clone(),
            });
        }
        // cite: libcore ObjectStreamField.java#compareTo
        fields.sort_by(|left, right| {
            let left_primitive = primitive_width(left.type_code).is_some();
            let right_primitive = primitive_width(right.type_code).is_some();
            right_primitive
                .cmp(&left_primitive)
                .then_with(|| left.field.name.cmp(&right.field.name))
        });
        let Some(suid) = suid else {
            return Err(missing(format!("default serialVersionUID of {name}")));
        };
        Ok(Ok(LocalClass {
            descriptor: descriptor.to_owned(),
            name,
            suid,
            fields,
        }))
    }

    fn object_input_init(
        &self,
        stream: &DexValue,
        input: Option<&DexValue>,
        depth: usize,
    ) -> Result<DexExecutionResult> {
        let Some(input) = input.filter(|input| dex_value_is_object_like(input)) else {
            return Ok(self.null_pointer_exception_result());
        };
        self.write_stored_instance_field(stream, &self.object_stream_field(false), input.clone());
        self.runtime.borrow_mut().object_input_handles.insert(
            map_identity_key(stream),
            ReadState {
                handles: Vec::new(),
                class_not_found: None,
            },
        );
        // cite: libcore ObjectInputStream.java#readStreamHeader
        let header = match self.input_stream_read_exact(input, 4, depth)? {
            Ok(header) => header,
            Err(exception) => return Ok(DexExecutionResult::Throw(exception)),
        };
        let magic = u16::from_be_bytes([header[0], header[1]]);
        let version = u16::from_be_bytes([header[2], header[3]]);
        if magic != STREAM_MAGIC || version != STREAM_VERSION {
            return Ok(DexExecutionResult::Throw(self.serial_exception(
                "Ljava/io/StreamCorruptedException;",
                Some(format!("invalid stream header: {magic:04X}{version:04X}")),
            )));
        }
        Ok(DexExecutionResult::Void)
    }

    fn object_input_read(&self, stream: &DexValue, depth: usize) -> Result<DexExecutionResult> {
        let Some(input) = self
            .read_stored_instance_field(stream, &self.object_stream_field(false))
            .filter(dex_value_is_object_like)
        else {
            return Err(missing(
                "ObjectInputStream.readObject on a stream without a source".to_owned(),
            ));
        };
        let stream_key = map_identity_key(stream);
        let mut state = self
            .runtime
            .borrow_mut()
            .object_input_handles
            .remove(&stream_key)
            .unwrap_or(ReadState {
                handles: Vec::new(),
                class_not_found: None,
            });
        state.class_not_found = None;
        let result = self.read_object0(&input, &mut state, depth);
        let class_not_found = state.class_not_found.take();
        self.runtime
            .borrow_mut()
            .object_input_handles
            .insert(stream_key, state);
        // cite: libcore ObjectInputStream.java#readObject, lookupException
        Ok(match result? {
            Ok(_) if class_not_found.is_some() => DexExecutionResult::Throw(
                self.serial_exception("Ljava/lang/ClassNotFoundException;", class_not_found),
            ),
            Ok(value) => DexExecutionResult::Value(value),
            Err(exception) => DexExecutionResult::Throw(exception),
        })
    }

    // cite: libcore ObjectInputStream.java#PeekInputStream, readFully
    fn input_stream_read_exact(
        &self,
        input: &DexValue,
        count: usize,
        depth: usize,
    ) -> Result<Thrown<Vec<u8>>> {
        let mut bytes = Vec::with_capacity(count.min(READ_CHUNK));
        while bytes.len() < count {
            let remaining = (count - bytes.len()).min(READ_CHUNK);
            let array =
                self.allocate_filled_array("[B".to_owned(), vec![DexValue::Int(0); remaining]);
            let read = self.stream_read_into(
                input,
                &array,
                0,
                i32::try_from(remaining).unwrap_or(i32::MAX),
                depth,
            )?;
            match read {
                Some(DexExecutionResult::Int(-1)) => {
                    return Ok(Err(self.serial_exception("Ljava/io/EOFException;", None)));
                }
                Some(DexExecutionResult::Int(read)) if read > 0 => {
                    let chunk = self.byte_array_contents(&array).ok_or_else(|| {
                        WinedroidError::new("ObjectInputStream read buffer is not a byte[]")
                    })?;
                    bytes.extend(&chunk[..read as usize]);
                }
                Some(DexExecutionResult::Throw(exception)) => return Ok(Err(exception)),
                Some(DexExecutionResult::Parked(execution)) => {
                    self.discard_parked_execution(&execution);
                    return Err(missing(
                        "ObjectInputStream read from a stream that parks".to_owned(),
                    ));
                }
                Some(other) => {
                    return Err(WinedroidError::new(format!(
                        "InputStream.read returned {other} under an ObjectInputStream"
                    )));
                }
                None => {
                    return Err(missing(format!(
                        "ObjectInputStream read from {}",
                        value_class_descriptor(input).unwrap_or("unknown class")
                    )));
                }
            }
        }
        Ok(Ok(bytes))
    }

    fn read_u8(&self, input: &DexValue, depth: usize) -> Result<Thrown<u8>> {
        Ok(self
            .input_stream_read_exact(input, 1, depth)?
            .map(|bytes| bytes[0]))
    }

    fn read_be<const N: usize>(&self, input: &DexValue, depth: usize) -> Result<Thrown<[u8; N]>> {
        Ok(self.input_stream_read_exact(input, N, depth)?.map(|bytes| {
            let mut array = [0u8; N];
            array.copy_from_slice(&bytes);
            array
        }))
    }

    // cite: libcore ObjectInputStream.java#readUTF, readLongUTF, readUTFBody
    fn read_utf(&self, input: &DexValue, long: bool, depth: usize) -> Result<Thrown<String>> {
        let length = if long {
            match self.read_be::<8>(input, depth)? {
                Ok(bytes) => i64::from_be_bytes(bytes),
                Err(exception) => return Ok(Err(exception)),
            }
        } else {
            match self.read_be::<2>(input, depth)? {
                Ok(bytes) => i64::from(u16::from_be_bytes(bytes)),
                Err(exception) => return Ok(Err(exception)),
            }
        };
        let Ok(length) = usize::try_from(length) else {
            return Ok(Err(
                self.serial_exception("Ljava/io/UTFDataFormatException;", None)
            ));
        };
        let bytes = match self.input_stream_read_exact(input, length, depth)? {
            Ok(bytes) => bytes,
            Err(exception) => return Ok(Err(exception)),
        };
        Ok(match decode_modified_utf8(&bytes) {
            Some(text) => Ok(text),
            None => Err(self.serial_exception("Ljava/io/UTFDataFormatException;", None)),
        })
    }

    fn read_handle_index(
        &self,
        input: &DexValue,
        handles: &[InputHandle],
        depth: usize,
    ) -> Result<Thrown<usize>> {
        let handle = match self.read_be::<4>(input, depth)? {
            Ok(bytes) => i32::from_be_bytes(bytes),
            Err(exception) => return Ok(Err(exception)),
        };
        let index = handle - BASE_WIRE_HANDLE;
        match usize::try_from(index)
            .ok()
            .filter(|index| *index < handles.len())
        {
            Some(index) => Ok(Ok(index)),
            None => Ok(Err(self.serial_exception(
                "Ljava/io/StreamCorruptedException;",
                Some(format!("invalid handle value: {handle:08X}")),
            ))),
        }
    }

    // cite: libcore ObjectInputStream.java#readObject0, readNull, readHandle, readString, readOrdinaryObject
    fn read_object0(
        &self,
        input: &DexValue,
        state: &mut ReadState,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        let tc = match self.read_u8(input, depth)? {
            Ok(tc) => tc,
            Err(exception) => return Ok(Err(exception)),
        };
        self.read_object_with_type_code(input, state, tc, depth)
    }

    fn read_object_with_type_code(
        &self,
        input: &DexValue,
        state: &mut ReadState,
        tc: u8,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        match tc {
            TC_NULL => Ok(Ok(DexValue::Null)),
            TC_REFERENCE => {
                let index = match self.read_handle_index(input, &state.handles, depth)? {
                    Ok(index) => index,
                    Err(exception) => return Ok(Err(exception)),
                };
                match &state.handles[index] {
                    InputHandle::Object(value) => Ok(Ok(value.clone())),
                    InputHandle::Failed(name) => {
                        state.class_not_found.get_or_insert_with(|| name.clone());
                        Ok(Ok(DexValue::Null))
                    }
                    InputHandle::Class(_) | InputHandle::Pending => Err(missing(
                        "ObjectInputStream reference to a class descriptor or unfinished object"
                            .to_owned(),
                    )),
                }
            }
            TC_STRING | TC_LONGSTRING => {
                let text = match self.read_utf(input, tc == TC_LONGSTRING, depth)? {
                    Ok(text) => text,
                    Err(exception) => return Ok(Err(exception)),
                };
                let value = DexValue::String(std::rc::Rc::from(text));
                state.handles.push(InputHandle::Object(value.clone()));
                Ok(Ok(value))
            }
            TC_OBJECT => self.read_ordinary_object(input, state, depth),
            0x72 | 0x75..=0x77 | 0x79..=0x7b | 0x7d | 0x7e | TC_ENDBLOCKDATA => {
                Err(missing(format!("ObjectInputStream type code {tc:02X}")))
            }
            _ => Ok(Err(self.serial_exception(
                "Ljava/io/StreamCorruptedException;",
                Some(format!("invalid type code: {tc:02X}")),
            ))),
        }
    }

    // cite: libcore ObjectInputStream.java#readClassDesc, readNonProxyDesc, skipCustomData, readTypeString
    // cite: libcore ObjectStreamClass.java#readNonProxy
    fn read_class_desc(
        &self,
        input: &DexValue,
        state: &mut ReadState,
        depth: usize,
    ) -> Result<Thrown<Option<Rc<StreamClassDesc>>>> {
        let tc = match self.read_u8(input, depth)? {
            Ok(tc) => tc,
            Err(exception) => return Ok(Err(exception)),
        };
        match tc {
            TC_NULL => return Ok(Ok(None)),
            TC_REFERENCE => {
                let index = match self.read_handle_index(input, &state.handles, depth)? {
                    Ok(index) => index,
                    Err(exception) => return Ok(Err(exception)),
                };
                return match &state.handles[index] {
                    InputHandle::Class(desc) => Ok(Ok(Some(desc.clone()))),
                    _ => Ok(Err(self.serial_exception(
                        "Ljava/io/StreamCorruptedException;",
                        Some("invalid class descriptor".to_owned()),
                    ))),
                };
            }
            TC_CLASSDESC => {}
            0x7d => {
                return Err(missing(
                    "ObjectInputStream proxy class descriptor".to_owned(),
                ))
            }
            _ => {
                return Ok(Err(self.serial_exception(
                    "Ljava/io/StreamCorruptedException;",
                    Some(format!("invalid type code: {tc:02X}")),
                )))
            }
        }
        let handle_index = state.handles.len();
        state.handles.push(InputHandle::Pending);
        macro_rules! read {
            ($expression:expr) => {
                match $expression? {
                    Ok(value) => value,
                    Err(exception) => return Ok(Err(exception)),
                }
            };
        }
        let name = read!(self.read_utf(input, false, depth));
        let suid = i64::from_be_bytes(read!(self.read_be::<8>(input, depth)));
        let flags = read!(self.read_u8(input, depth));
        if flags & SC_SERIALIZABLE == 0 || flags & !(SC_SERIALIZABLE | SC_WRITE_METHOD) != 0 {
            return Err(missing(format!(
                "ObjectInputStream class descriptor flags {flags:02X} for {name}"
            )));
        }
        let count = u16::from_be_bytes(read!(self.read_be::<2>(input, depth)));
        let mut fields = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let type_code = read!(self.read_u8(input, depth));
            let field_name = read!(self.read_utf(input, false, depth));
            if primitive_width(type_code).is_none() {
                if !matches!(type_code, b'L' | b'[') {
                    return Ok(Err(self.serial_exception(
                        "Ljava/io/InvalidClassException;",
                        Some(format!("{name}; invalid descriptor for field {field_name}")),
                    )));
                }
                let signature = read!(self.read_object0(input, state, depth));
                if !matches!(signature, DexValue::String(_)) {
                    return Ok(Err(self.serial_exception(
                        "Ljava/io/StreamCorruptedException;",
                        Some("invalid type string".to_owned()),
                    )));
                }
            }
            fields.push(StreamField {
                type_code,
                name: field_name,
            });
        }
        let end = read!(self.read_u8(input, depth));
        if end != TC_ENDBLOCKDATA {
            return Err(missing(format!(
                "ObjectInputStream class annotation data for {name}"
            )));
        }
        let super_desc = read!(self.read_class_desc(input, state, depth));
        let desc = Rc::new(StreamClassDesc {
            name,
            suid,
            flags,
            fields,
            super_desc,
        });
        state.handles[handle_index] = InputHandle::Class(desc.clone());
        Ok(Ok(Some(desc)))
    }

    // cite: libcore ObjectInputStream.java#readOrdinaryObject, readSerialData, defaultReadFields
    // cite: libcore ObjectStreamClass.java#initNonProxy, newInstance, getClassDataLayout, matchFields
    fn read_ordinary_object(
        &self,
        input: &DexValue,
        state: &mut ReadState,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        macro_rules! read {
            ($expression:expr) => {
                match $expression? {
                    Ok(value) => value,
                    Err(exception) => return Ok(Err(exception)),
                }
            };
        }
        let Some(desc) = read!(self.read_class_desc(input, state, depth)) else {
            return Ok(Err(self.serial_exception(
                "Ljava/io/InvalidClassException;",
                Some("null class descriptor".to_owned()),
            )));
        };
        let descriptor = format!("L{};", desc.name.replace('.', "/"));
        if self.load_class(&descriptor).is_none() && is_framework_descriptor(&descriptor) {
            return Err(missing(format!(
                "deserialization of framework class {}",
                desc.name
            )));
        }
        if self.load_class(&descriptor).is_some()
            && !self.descriptor_matches_type(&descriptor, "Ljava/io/Serializable;")
        {
            return Ok(Err(self.serial_exception(
                "Ljava/io/InvalidClassException;",
                Some(format!("{}; class invalid for deserialization", desc.name)),
            )));
        }
        let local = if self.load_class(&descriptor).is_some() {
            Some(read!(self.local_serial_hierarchy(&descriptor, depth)))
        } else {
            None
        };
        let mut stream_chain = Vec::new();
        let mut current = Some(desc.clone());
        while let Some(stream_desc) = current {
            current = stream_desc.super_desc.clone();
            stream_chain.push(stream_desc);
        }
        let object = match &local {
            Some(hierarchy) => {
                for stream_desc in &stream_chain {
                    if let Some(class) = hierarchy
                        .classes
                        .iter()
                        .find(|class| class.name == stream_desc.name)
                    {
                        if class.suid != stream_desc.suid {
                            return Ok(Err(self.serial_exception(
                                "Ljava/io/InvalidClassException;",
                                Some(format!(
                                    "{}; local class incompatible: stream classdesc serialVersionUID = {}, local class serialVersionUID = {}",
                                    class.name, stream_desc.suid, class.suid
                                )),
                            )));
                        }
                    }
                }
                Some(read!(self.serial_new_instance(
                    &descriptor,
                    hierarchy,
                    depth
                )))
            }
            None => None,
        };
        state.handles.push(match &object {
            Some(object) => InputHandle::Object(object.clone()),
            None => InputHandle::Failed(desc.name.clone()),
        });
        if object.is_none() {
            state
                .class_not_found
                .get_or_insert_with(|| desc.name.clone());
        }
        for stream_desc in stream_chain.iter().rev() {
            let class = local.as_ref().and_then(|hierarchy| {
                hierarchy
                    .classes
                    .iter()
                    .find(|class| class.name == stream_desc.name)
            });
            read!(self.read_fields(input, state, object.as_ref(), stream_desc, class, depth));
            if stream_desc.flags & SC_WRITE_METHOD != 0 {
                read!(self.skip_custom_data(input, state, depth));
            }
        }
        Ok(Ok(object.unwrap_or(DexValue::Null)))
    }

    // cite: libcore ObjectInputStream.java#skipCustomData
    fn skip_custom_data(
        &self,
        input: &DexValue,
        state: &mut ReadState,
        depth: usize,
    ) -> Result<Thrown<()>> {
        loop {
            let tc = match self.read_u8(input, depth)? {
                Ok(tc) => tc,
                Err(exception) => return Ok(Err(exception)),
            };
            let length = match tc {
                TC_ENDBLOCKDATA => return Ok(Ok(())),
                TC_BLOCKDATA => match self.read_u8(input, depth)? {
                    Ok(length) => usize::from(length),
                    Err(exception) => return Ok(Err(exception)),
                },
                TC_BLOCKDATALONG => match self.read_be::<4>(input, depth)? {
                    Ok(bytes) => match usize::try_from(i32::from_be_bytes(bytes)) {
                        Ok(length) => length,
                        Err(_) => {
                            return Ok(Err(self.serial_exception(
                                "Ljava/io/StreamCorruptedException;",
                                Some(format!(
                                    "illegal block data header length: {}",
                                    i32::from_be_bytes(bytes)
                                )),
                            )))
                        }
                    },
                    Err(exception) => return Ok(Err(exception)),
                },
                tc => {
                    if let Err(exception) =
                        self.read_object_with_type_code(input, state, tc, depth)?
                    {
                        return Ok(Err(exception));
                    }
                    continue;
                }
            };
            if let Err(exception) = self.input_stream_read_exact(input, length, depth)? {
                return Ok(Err(exception));
            }
        }
    }

    // cite: libcore ObjectStreamClass.java#getSerializableConstructor, newInstance
    fn serial_new_instance(
        &self,
        descriptor: &str,
        hierarchy: &LocalHierarchy,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        let object = self.allocate_object(descriptor.to_owned());
        if hierarchy.constructor_class == "Ljava/lang/Object;" {
            return Ok(Ok(object));
        }
        let constructor = DexMethodId {
            class_descriptor: hierarchy.constructor_class.clone(),
            name: "<init>".to_owned(),
            proto: dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters: Vec::new(),
            },
        };
        let result = match self.resolve_exact_method(&constructor)? {
            Some(resolved) => {
                self.execute_method(None, &resolved, Some(&[object.clone()]), depth + 1)?
            }
            None if self.load_class(&hierarchy.constructor_class).is_some() => {
                return Ok(Err(self.serial_exception(
                    "Ljava/io/InvalidClassException;",
                    Some(format!(
                        "{}; no valid constructor",
                        java_class_name(descriptor)
                    )),
                )));
            }
            None => {
                let invocation = DexInvocation {
                    kind: DexInvokeKind::Direct,
                    range: false,
                    method_index: 0,
                    argument_registers: Default::default(),
                    arguments: smallvec::smallvec![object.clone()],
                };
                self.invoke_builtin_method(&constructor, Some(&invocation))
                    .ok_or_else(|| {
                        missing(format!(
                            "serialization constructor {}.<init>()V",
                            java_class_name(&hierarchy.constructor_class)
                        ))
                    })?
            }
        };
        match result {
            DexExecutionResult::Throw(exception) => Ok(Err(exception)),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(missing("serialization constructor that parks".to_owned()))
            }
            _ => Ok(Ok(object)),
        }
    }

    // cite: libcore ObjectInputStream.java#defaultReadFields
    // cite: libcore ObjectStreamClass.java#setPrimFieldValues, setObjFieldValues, matchFields
    fn read_fields(
        &self,
        input: &DexValue,
        state: &mut ReadState,
        object: Option<&DexValue>,
        stream_desc: &StreamClassDesc,
        class: Option<&LocalClass>,
        depth: usize,
    ) -> Result<Thrown<()>> {
        let local_field = |name: &str| {
            class.and_then(|class| class.fields.iter().find(|field| field.field.name == name))
        };
        for field in &stream_desc.fields {
            if let Some(local) = local_field(&field.name) {
                let stream_primitive = primitive_width(field.type_code).is_some();
                let local_primitive = primitive_width(local.type_code).is_some();
                if (stream_primitive || local_primitive) && field.type_code != local.type_code {
                    return Ok(Err(self.serial_exception(
                        "Ljava/io/InvalidClassException;",
                        Some(format!(
                            "{}; incompatible types for field {}",
                            stream_desc.name, field.name
                        )),
                    )));
                }
            }
        }
        for field in &stream_desc.fields {
            let Some(width) = primitive_width(field.type_code) else {
                continue;
            };
            let bytes = match self.input_stream_read_exact(input, width, depth)? {
                Ok(bytes) => bytes,
                Err(exception) => return Ok(Err(exception)),
            };
            let mut buffer = [0u8; 8];
            buffer[8 - width..].copy_from_slice(&bytes);
            let raw = i64::from_be_bytes(buffer);
            let value = match field.type_code {
                b'Z' => DexValue::Int(i32::from(raw != 0)),
                b'B' => DexValue::Int(i32::from(raw as u8 as i8)),
                b'C' => DexValue::Int(i32::from(raw as u16)),
                b'S' => DexValue::Int(i32::from(raw as u16 as i16)),
                b'I' => DexValue::Int(raw as u32 as i32),
                b'F' => DexValue::Float(f32::from_bits(raw as u32)),
                _ => DexValue::Long(raw),
            };
            if let (Some(object), Some(local)) = (object, local_field(&field.name)) {
                self.write_stored_instance_field(object, &local.field, value);
            }
        }
        for field in &stream_desc.fields {
            if primitive_width(field.type_code).is_some() {
                continue;
            }
            let value = match self.read_object0(input, state, depth)? {
                Ok(value) => value,
                Err(exception) => return Ok(Err(exception)),
            };
            let (Some(object), Some(local)) = (object, local_field(&field.name)) else {
                continue;
            };
            let assignable = matches!(value, DexValue::Null)
                || self.value_matches_type(&value, &local.field.type_descriptor);
            if !assignable {
                let value_class = match &value {
                    DexValue::String(_) => "java.lang.String".to_owned(),
                    other => java_class_name(value_class_descriptor(other).unwrap_or("unknown")),
                };
                return Ok(Err(self.serial_exception(
                    "Ljava/lang/ClassCastException;",
                    Some(format!(
                        "cannot assign instance of {value_class} to field {}.{} of type {} in instance of {}",
                        java_class_name(&local.field.class_descriptor),
                        local.field.name,
                        descriptor_to_pretty_name(&local.field.type_descriptor)
                            .unwrap_or_else(|| local.field.type_descriptor.clone()),
                        java_class_name(value_class_descriptor(object).unwrap_or("unknown"))
                    )),
                )));
            }
            self.write_stored_instance_field(object, &local.field, value);
        }
        Ok(Ok(()))
    }
}

// cite: libcore ObjectOutputStream.java#writeString, writeUTF, writeLongUTF
fn write_string(bytes: &mut Vec<u8>, text: &str) {
    let encoded = modified_utf8(text);
    if encoded.len() <= 0xffff {
        bytes.push(TC_STRING);
        bytes.extend((encoded.len() as u16).to_be_bytes());
    } else {
        bytes.push(TC_LONGSTRING);
        bytes.extend((encoded.len() as i64).to_be_bytes());
    }
    bytes.extend(encoded);
}

fn write_utf(bytes: &mut Vec<u8>, text: &str) {
    let encoded = modified_utf8(text);
    bytes.extend((encoded.len() as u16).to_be_bytes());
    bytes.extend(encoded);
}
