use super::super::*;
use crate::external_storage::ExternalStorageAccess;
use std::io::{Read, Seek, SeekFrom};

const BUFFER: &str = "Ljava/nio/Buffer;";
const BYTE_BUFFER: &str = "Ljava/nio/ByteBuffer;";
const BYTE_ORDER: &str = "Ljava/nio/ByteOrder;";
const FILE_CHANNEL: &str = "Ljava/nio/channels/FileChannel;";
const FILE_CHANNEL_IMPL: &str = "Lsun/nio/ch/FileChannelImpl;";
const HEAP_BYTE_BUFFER: &str = "Ljava/nio/HeapByteBuffer;";

struct ByteBufferState {
    hb: DexValue,
    offset: i32,
    position: i32,
    limit: i32,
    capacity: i32,
    big_endian: bool,
    read_only: bool,
}

struct FileChannelState {
    path: String,
    position: i64,
    open: bool,
}

impl DexProgram {
    pub(super) fn invoke_java_nio_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        let class = method.class_descriptor.as_str();
        if !matches!(class, BUFFER | BYTE_BUFFER | FILE_CHANNEL) {
            return None;
        }
        let invocation = invocation?;
        let name = method.name.as_str();
        let proto = method.proto.descriptor();

        if class == FILE_CHANNEL {
            return match (name, proto.as_str()) {
                ("size", "()J") => self.file_channel_size(invocation),
                ("position", "()J") => self.file_channel_position(invocation),
                ("position", "(J)Ljava/nio/channels/FileChannel;") => {
                    self.file_channel_set_position(invocation)
                }
                ("read", "(Ljava/nio/ByteBuffer;)I") => self.file_channel_read(invocation),
                _ => None,
            };
        }

        match (class, name, proto.as_str()) {
            (BYTE_BUFFER, "allocate", "(I)Ljava/nio/ByteBuffer;") => {
                self.byte_buffer_allocate(invocation)
            }
            (BYTE_BUFFER, "order", "()Ljava/nio/ByteOrder;") => self.byte_buffer_order(invocation),
            (BYTE_BUFFER, "order", "(Ljava/nio/ByteOrder;)Ljava/nio/ByteBuffer;") => {
                self.byte_buffer_set_order(invocation)
            }
            (BYTE_BUFFER, "getShort", "()S") => self.byte_buffer_get(invocation, 2, None),
            (BYTE_BUFFER, "getShort", "(I)S") => self.byte_buffer_get(invocation, 2, Some(1)),
            (BYTE_BUFFER, "getInt", "()I") => self.byte_buffer_get(invocation, 4, None),
            (BYTE_BUFFER, "getInt", "(I)I") => self.byte_buffer_get(invocation, 4, Some(1)),
            (BYTE_BUFFER, "getLong", "()J") => self.byte_buffer_get(invocation, 8, None),
            (BYTE_BUFFER, "getLong", "(I)J") => self.byte_buffer_get(invocation, 8, Some(1)),
            (BYTE_BUFFER, "slice", "()Ljava/nio/ByteBuffer;") => self.byte_buffer_slice(invocation),
            (BYTE_BUFFER, "array", "()[B") => self.byte_buffer_array(invocation),
            (BYTE_BUFFER, "arrayOffset", "()I") => self.byte_buffer_array_offset(invocation),
            (_, "capacity", "()I") => self.buffer_int(invocation, "capacity"),
            (_, "position", "()I") => self.buffer_int(invocation, "position"),
            (_, "limit", "()I") => self.buffer_int(invocation, "limit"),
            (_, "remaining", "()I") => {
                let state = self.byte_buffer_state(invocation.arguments.first()?)?;
                Some(DexExecutionResult::Int(state.limit - state.position))
            }
            (_, "hasRemaining", "()Z") => {
                let state = self.byte_buffer_state(invocation.arguments.first()?)?;
                Some(DexExecutionResult::Int(i32::from(
                    state.position < state.limit,
                )))
            }
            (_, "position", "(I)Ljava/nio/Buffer;" | "(I)Ljava/nio/ByteBuffer;") => {
                self.buffer_set_position(invocation)
            }
            (_, "limit", "(I)Ljava/nio/Buffer;" | "(I)Ljava/nio/ByteBuffer;") => {
                self.buffer_set_limit(invocation)
            }
            _ => None,
        }
    }

    pub(in crate::dex_run) fn new_file_channel(&self, path: String) -> DexValue {
        let channel = self.allocate_object(FILE_CHANNEL_IMPL.to_owned());
        self.write_stored_instance_field(
            &channel,
            &nio_field(FILE_CHANNEL_IMPL, "Ljava/lang/String;", "path"),
            DexValue::String(std::rc::Rc::from(path)),
        );
        self.write_stored_instance_field(
            &channel,
            &nio_field(FILE_CHANNEL_IMPL, "J", "position"),
            DexValue::Long(0),
        );
        self.write_stored_instance_field(
            &channel,
            &nio_field(FILE_CHANNEL_IMPL, "Z", "open"),
            DexValue::Int(1),
        );
        channel
    }

    pub(in crate::dex_run) fn close_file_channel(&self, channel: &DexValue) {
        self.write_stored_instance_field(
            channel,
            &nio_field(FILE_CHANNEL_IMPL, "Z", "open"),
            DexValue::Int(0),
        );
    }

    pub(in crate::dex_run) fn file_channel_is_open(&self, channel: &DexValue) -> Option<bool> {
        Some(self.file_channel_state(channel)?.open)
    }

    fn nio_throw(
        &self,
        class_descriptor: &str,
        detail_message: Option<String>,
    ) -> DexExecutionResult {
        DexExecutionResult::Throw(self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: class_descriptor.to_owned(),
            detail_message,
        }))
    }

    fn new_heap_byte_buffer(
        &self,
        hb: DexValue,
        offset: i32,
        limit: i32,
        capacity: i32,
        read_only: bool,
    ) -> DexValue {
        let buffer = self.allocate_object(HEAP_BYTE_BUFFER.to_owned());
        let fields = [
            (nio_field(BUFFER, "I", "position"), DexValue::Int(0)),
            (nio_field(BUFFER, "I", "limit"), DexValue::Int(limit)),
            (nio_field(BUFFER, "I", "capacity"), DexValue::Int(capacity)),
            (nio_field(BYTE_BUFFER, "[B", "hb"), hb),
            (nio_field(BYTE_BUFFER, "I", "offset"), DexValue::Int(offset)),
            (nio_field(BYTE_BUFFER, "Z", "bigEndian"), DexValue::Int(1)),
            (
                nio_field(BYTE_BUFFER, "Z", "isReadOnly"),
                DexValue::Int(i32::from(read_only)),
            ),
        ];
        for (field, value) in fields {
            self.write_stored_instance_field(&buffer, &field, value);
        }
        buffer
    }

    fn byte_buffer_state(&self, buffer: &DexValue) -> Option<ByteBufferState> {
        if !matches!(buffer, DexValue::Object(object) if object.class_descriptor.as_ref() == HEAP_BYTE_BUFFER)
        {
            return None;
        }
        let int = |class: &str, name: &str| match self
            .read_stored_instance_field(buffer, &nio_field(class, "I", name))?
        {
            DexValue::Int(value) => Some(value),
            _ => None,
        };
        let flag = |name: &str| match self
            .read_stored_instance_field(buffer, &nio_field(BYTE_BUFFER, "Z", name))?
        {
            DexValue::Int(value) => Some(value != 0),
            _ => None,
        };

        Some(ByteBufferState {
            hb: self.read_stored_instance_field(buffer, &nio_field(BYTE_BUFFER, "[B", "hb"))?,
            offset: int(BYTE_BUFFER, "offset")?,
            position: int(BUFFER, "position")?,
            limit: int(BUFFER, "limit")?,
            capacity: int(BUFFER, "capacity")?,
            big_endian: flag("bigEndian")?,
            read_only: flag("isReadOnly")?,
        })
    }

    fn set_buffer_int(&self, buffer: &DexValue, name: &str, value: i32) {
        self.write_stored_instance_field(
            buffer,
            &nio_field(BUFFER, "I", name),
            DexValue::Int(value),
        );
    }

    // cite: libcore ByteBuffer.java#allocate
    // cite: libcore HeapByteBuffer.java#HeapByteBuffer
    // cite: libcore Buffer.java#createCapacityException
    fn byte_buffer_allocate(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let DexValue::Int(capacity) = *invocation.arguments.first()? else {
            return None;
        };
        if capacity < 0 {
            return Some(self.nio_throw(
                "Ljava/lang/IllegalArgumentException;",
                Some(format!("capacity < 0: ({capacity} < 0)")),
            ));
        }
        let hb = self.allocate_array("[B".to_owned(), capacity)?;

        Some(DexExecutionResult::Value(
            self.new_heap_byte_buffer(hb, 0, capacity, capacity, false),
        ))
    }

    // cite: libcore ByteBuffer.java#order
    fn byte_buffer_order(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let state = self.byte_buffer_state(invocation.arguments.first()?)?;
        let name = if state.big_endian {
            "BIG_ENDIAN"
        } else {
            "LITTLE_ENDIAN"
        };

        Some(DexExecutionResult::Value(byte_order_value(name)))
    }

    // cite: libcore ByteBuffer.java#order
    fn byte_buffer_set_order(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let buffer = invocation.arguments.first()?;
        self.byte_buffer_state(buffer)?;
        let big_endian = match invocation.arguments.get(1)? {
            DexValue::Reference(order) => match order.strip_prefix("Ljava/nio/ByteOrder;:")? {
                "BIG_ENDIAN" => true,
                "LITTLE_ENDIAN" => false,
                _ => return None,
            },
            DexValue::Null | DexValue::Int(0) => false,
            _ => return None,
        };
        self.write_stored_instance_field(
            buffer,
            &nio_field(BYTE_BUFFER, "Z", "bigEndian"),
            DexValue::Int(i32::from(big_endian)),
        );

        Some(DexExecutionResult::Value(buffer.clone()))
    }

    fn buffer_int(&self, invocation: &DexInvocation, name: &str) -> Option<DexExecutionResult> {
        let state = self.byte_buffer_state(invocation.arguments.first()?)?;
        let value = match name {
            "capacity" => state.capacity,
            "position" => state.position,
            "limit" => state.limit,
            _ => return None,
        };

        Some(DexExecutionResult::Int(value))
    }

    // cite: libcore Buffer.java#position
    fn buffer_set_position(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let buffer = invocation.arguments.first()?;
        let state = self.byte_buffer_state(buffer)?;
        let DexValue::Int(new_position) = *invocation.arguments.get(1)? else {
            return None;
        };
        if new_position > state.limit || new_position < 0 {
            return Some(self.nio_throw(
                "Ljava/lang/IllegalArgumentException;",
                Some(format!("Bad position {new_position}/{}", state.limit)),
            ));
        }
        self.set_buffer_int(buffer, "position", new_position);

        Some(DexExecutionResult::Value(buffer.clone()))
    }

    // cite: libcore Buffer.java#limit, createLimitException
    fn buffer_set_limit(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let buffer = invocation.arguments.first()?;
        let state = self.byte_buffer_state(buffer)?;
        let DexValue::Int(new_limit) = *invocation.arguments.get(1)? else {
            return None;
        };
        if new_limit > state.capacity {
            return Some(self.nio_throw(
                "Ljava/lang/IllegalArgumentException;",
                Some(format!(
                    "newLimit > capacity: ({new_limit} > {})",
                    state.capacity
                )),
            ));
        }
        if new_limit < 0 {
            return Some(self.nio_throw(
                "Ljava/lang/IllegalArgumentException;",
                Some(format!("newLimit < 0: ({new_limit} < 0)")),
            ));
        }
        self.set_buffer_int(buffer, "limit", new_limit);
        if state.position > new_limit {
            self.set_buffer_int(buffer, "position", new_limit);
        }

        Some(DexExecutionResult::Value(buffer.clone()))
    }

    // cite: libcore HeapByteBuffer.java#getShort, getInt, getLong
    // cite: libcore Bits.java#getShort, getInt, getLong, getShortB, getShortL, getIntB, getIntL, getLongB, getLongL
    // cite: libcore Buffer.java#nextGetIndex, checkIndex
    fn byte_buffer_get(
        &self,
        invocation: &DexInvocation,
        width: i32,
        absolute_index_argument: Option<usize>,
    ) -> Option<DexExecutionResult> {
        let buffer = invocation.arguments.first()?;
        let state = self.byte_buffer_state(buffer)?;
        let start = match absolute_index_argument {
            Some(argument) => {
                let DexValue::Int(index) = *invocation.arguments.get(argument)? else {
                    return None;
                };
                if index < 0 || width > state.limit - index {
                    return Some(self.nio_throw(
                        "Ljava/lang/IndexOutOfBoundsException;",
                        Some(format!(
                            "index={index} out of bounds (limit={}, nb={width})",
                            state.limit
                        )),
                    ));
                }
                index
            }
            None => {
                if state.limit - state.position < width {
                    return Some(self.nio_throw("Ljava/nio/BufferUnderflowException;", None));
                }
                self.set_buffer_int(buffer, "position", state.position + width);
                state.position
            }
        };

        let mut bytes = Vec::with_capacity(width as usize);
        for index in 0..width {
            let DexValue::Int(byte) = self.read_array_value(
                &state.hb,
                start.checked_add(index)?.checked_add(state.offset)?,
            )?
            else {
                return None;
            };
            bytes.push(byte as u8);
        }
        if !state.big_endian {
            bytes.reverse();
        }
        let bits = bytes
            .iter()
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte));

        Some(match width {
            2 => DexExecutionResult::Int(i32::from(bits as u16 as i16)),
            4 => DexExecutionResult::Int(bits as u32 as i32),
            _ => DexExecutionResult::Value(DexValue::Long(bits as i64)),
        })
    }

    // cite: libcore HeapByteBuffer.java#slice
    fn byte_buffer_slice(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let state = self.byte_buffer_state(invocation.arguments.first()?)?;
        let remaining = state.limit - state.position;

        Some(DexExecutionResult::Value(self.new_heap_byte_buffer(
            state.hb,
            state.position + state.offset,
            remaining,
            remaining,
            state.read_only,
        )))
    }

    // cite: libcore ByteBuffer.java#array
    fn byte_buffer_array(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let state = self.byte_buffer_state(invocation.arguments.first()?)?;
        if state.read_only {
            return Some(self.nio_throw("Ljava/nio/ReadOnlyBufferException;", None));
        }

        Some(DexExecutionResult::Value(state.hb))
    }

    // cite: libcore ByteBuffer.java#arrayOffset
    fn byte_buffer_array_offset(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let state = self.byte_buffer_state(invocation.arguments.first()?)?;
        if state.read_only {
            return Some(self.nio_throw("Ljava/nio/ReadOnlyBufferException;", None));
        }

        Some(DexExecutionResult::Int(state.offset))
    }

    fn file_channel_state(&self, channel: &DexValue) -> Option<FileChannelState> {
        if !matches!(channel, DexValue::Object(object) if object.class_descriptor.as_ref() == FILE_CHANNEL_IMPL)
        {
            return None;
        }
        let DexValue::String(path) = self.read_stored_instance_field(
            channel,
            &nio_field(FILE_CHANNEL_IMPL, "Ljava/lang/String;", "path"),
        )?
        else {
            return None;
        };
        let DexValue::Long(position) = self
            .read_stored_instance_field(channel, &nio_field(FILE_CHANNEL_IMPL, "J", "position"))?
        else {
            return None;
        };
        let DexValue::Int(open) =
            self.read_stored_instance_field(channel, &nio_field(FILE_CHANNEL_IMPL, "Z", "open"))?
        else {
            return None;
        };

        Some(FileChannelState {
            path: path.to_string(),
            position,
            open: open != 0,
        })
    }

    // cite: libcore FileChannelImpl.java#ensureOpen
    fn open_file_channel_state(
        &self,
        channel: &DexValue,
    ) -> Option<std::result::Result<FileChannelState, DexExecutionResult>> {
        let state = self.file_channel_state(channel)?;
        if !state.open {
            return Some(Err(
                self.nio_throw("Ljava/nio/channels/ClosedChannelException;", None)
            ));
        }

        Some(Ok(state))
    }

    // cite: libcore FileChannelImpl.java#size
    fn file_channel_size(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let state = match self.open_file_channel_state(invocation.arguments.first()?)? {
            Ok(state) => state,
            Err(exception) => return Some(exception),
        };
        let path = self.resolved_guest_path_for_channel(&state.path).ok()?;
        let length = path.metadata().ok()?.len;

        Some(DexExecutionResult::Value(DexValue::Long(
            i64::try_from(length).ok()?,
        )))
    }

    // cite: libcore FileChannelImpl.java#position
    fn file_channel_position(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let state = match self.open_file_channel_state(invocation.arguments.first()?)? {
            Ok(state) => state,
            Err(exception) => return Some(exception),
        };

        Some(DexExecutionResult::Value(DexValue::Long(state.position)))
    }

    // cite: libcore FileChannelImpl.java#position
    fn file_channel_set_position(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let channel = invocation.arguments.first()?;
        if let Err(exception) = self.open_file_channel_state(channel)? {
            return Some(exception);
        }
        let DexValue::Long(new_position) = *invocation.arguments.get(1)? else {
            return None;
        };
        if new_position < 0 {
            return Some(self.nio_throw("Ljava/lang/IllegalArgumentException;", None));
        }
        self.write_stored_instance_field(
            channel,
            &nio_field(FILE_CHANNEL_IMPL, "J", "position"),
            DexValue::Long(new_position),
        );

        Some(DexExecutionResult::Value(channel.clone()))
    }

    // cite: libcore FileChannelImpl.java#read
    // cite: libcore IOUtil.java#read, readIntoNativeBuffer
    fn file_channel_read(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        let channel = invocation.arguments.first()?;
        let destination = invocation.arguments.get(1)?;
        let state = match self.open_file_channel_state(channel)? {
            Ok(state) => state,
            Err(exception) => return Some(exception),
        };
        let buffer = self.byte_buffer_state(destination)?;
        if buffer.read_only {
            return Some(self.nio_throw(
                "Ljava/lang/IllegalArgumentException;",
                Some("Read-only buffer".to_owned()),
            ));
        }
        let remaining = usize::try_from(buffer.limit - buffer.position).ok()?;
        if remaining == 0 {
            return Some(DexExecutionResult::Int(0));
        }

        let path = self.resolved_guest_path_for_channel(&state.path).ok()?;
        let mut file = path.open_read().ok()?;
        file.seek(SeekFrom::Start(u64::try_from(state.position).ok()?))
            .ok()?;
        let mut bytes = vec![0u8; remaining];
        let mut read = 0;
        while read < remaining {
            match file.read(&mut bytes[read..]) {
                Ok(0) => break,
                Ok(count) => read += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
        if read == 0 {
            return Some(DexExecutionResult::Int(-1));
        }

        let first = buffer.offset.checked_add(buffer.position)?;
        for (index, byte) in bytes[..read].iter().enumerate() {
            self.write_array_value(
                &buffer.hb,
                first.checked_add(i32::try_from(index).ok()?)?,
                DexValue::Int(i32::from(*byte as i8)),
            )?;
        }
        let read = i32::try_from(read).ok()?;
        self.set_buffer_int(destination, "position", buffer.position + read);
        self.write_stored_instance_field(
            channel,
            &nio_field(FILE_CHANNEL_IMPL, "J", "position"),
            DexValue::Long(state.position + i64::from(read)),
        );

        Some(DexExecutionResult::Int(read))
    }

    fn resolved_guest_path_for_channel(
        &self,
        guest_path: &str,
    ) -> std::result::Result<crate::guest_filesystem::ResolvedGuestPath, String> {
        self.guest_filesystem
            .as_ref()
            .ok_or_else(|| "guest filesystem is unavailable".to_owned())?
            .resolve(
                guest_path,
                ExternalStorageAccess::Read,
                &self.external_storage_policy,
            )
            .map_err(|error| error.to_string())
    }
}

fn nio_field(class_descriptor: &str, type_descriptor: &str, name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: class_descriptor.to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

fn byte_order_value(name: &str) -> DexValue {
    DexValue::Reference(std::rc::Rc::from(format!("{BYTE_ORDER}:{name}")))
}

pub(in crate::dex_run) fn byte_order_static_field(field: &DexFieldId) -> Option<DexValue> {
    if field.class_descriptor != BYTE_ORDER || field.type_descriptor != BYTE_ORDER {
        return None;
    }

    // cite: libcore ByteOrder.java#BIG_ENDIAN, LITTLE_ENDIAN
    matches!(field.name.as_str(), "BIG_ENDIAN" | "LITTLE_ENDIAN")
        .then(|| byte_order_value(&field.name))
}
