use super::super::*;
use crate::capability_fs::MetadataKind;
use crate::external_storage::ExternalStorageAccess;

impl DexProgram {
    pub(super) fn invoke_java_io_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "<init>"
            && method
                .proto
                .descriptor_is("(Ljava/io/File;Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            if file_trailing_path_is_null(invocation) {
                return Some(self.null_pointer_exception_result());
            }
            self.initialize_file_from_parent_and_child(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "<init>"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            if file_trailing_path_is_null(invocation) {
                return Some(self.null_pointer_exception_result());
            }
            self.initialize_file_from_parent_string_and_child(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/os/StatFs;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            return self.initialize_stat_fs(invocation);
        }

        if method.class_descriptor == "Landroid/os/StatFs;"
            && method.name == "getBlockSizeLong"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.stat_fs_block_size(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Landroid/os/StatFs;"
            && method.name == "getBlockCountLong"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.stat_fs_block_count(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Landroid/os/StatFs;"
            && method.name == "getFreeBlocksLong"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.stat_fs_free_blocks(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Landroid/os/StatFs;"
            && method.name == "getFreeBytes"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.stat_fs_free_bytes(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Landroid/os/StatFs;"
            && method.name == "getAvailableBlocksLong"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.stat_fs_available_blocks(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Landroid/os/StatFs;"
            && method.name == "getAvailableBytes"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.stat_fs_available_bytes(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Landroid/os/StatFs;"
            && method.name == "getTotalBytes"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.stat_fs_total_bytes(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        // cite: libcore File.java#File
        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            if file_trailing_path_is_null(invocation) {
                return Some(self.null_pointer_exception_result());
            }
            self.initialize_file_from_path(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "getParentFile"
            && method.proto.descriptor_is("()Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.file_get_parent_file(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "getParent"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = match self.file_parent(invocation)? {
                Some(parent) => DexValue::String(std::rc::Rc::from(parent)),
                None => DexValue::Null,
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "getName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.file_get_name(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "getAbsoluteFile"
            && method.proto.descriptor_is("()Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.file_get_absolute_file(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "getAbsolutePath"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.file_get_absolute_path(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "getCanonicalFile"
            && method.proto.descriptor_is("()Ljava/io/File;")
        {
            let invocation = invocation?;
            return self.file_get_canonical_file(invocation);
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "mkdirs"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.file_mkdirs(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "isDirectory"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.file_is_directory(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "isHidden"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.file_is_hidden(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "exists"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.file_exists(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        // cite: libcore File.java#equals, compareTo
        // cite: libcore UnixFileSystem.java#compare
        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "equals"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.file_equals(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/io/FileOutputStream;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/io/File;)V")
        {
            let invocation = invocation?;
            return self.initialize_file_output_stream_from_file(invocation);
        }

        if method.class_descriptor == "Ljava/io/RandomAccessFile;"
            && method.name == "<init>"
            && matches!(
                method.proto.descriptor().as_str(),
                "(Ljava/lang/String;Ljava/lang/String;)V" | "(Ljava/io/File;Ljava/lang/String;)V"
            )
        {
            let invocation = invocation?;
            let from_file = method
                .proto
                .descriptor_is("(Ljava/io/File;Ljava/lang/String;)V");
            return self.initialize_random_access_file(invocation, from_file);
        }

        if method.class_descriptor == "Ljava/io/RandomAccessFile;"
            && method.name == "getChannel"
            && method
                .proto
                .descriptor_is("()Ljava/nio/channels/FileChannel;")
        {
            let invocation = invocation?;
            let value = self.random_access_file_channel(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/RandomAccessFile;"
            && method.name == "close"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.close_random_access_file(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/FileInputStream;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/io/File;)V")
        {
            let invocation = invocation?;
            let result = self.initialize_file_input_stream_from_file(invocation)?;

            return Some(result);
        }

        if method.class_descriptor == "Ljava/io/FileInputStream;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            let result = self.initialize_file_input_stream_from_path(invocation)?;

            return Some(result);
        }

        // cite: libcore FileReader.java#FileReader
        if method.class_descriptor == "Ljava/io/FileReader;" && method.name == "<init>" {
            let from_file = match method.proto.descriptor().as_str() {
                "(Ljava/io/File;)V" => true,
                "(Ljava/lang/String;)V" => false,
                _ => return None,
            };
            return self.initialize_file_reader(invocation?, from_file);
        }

        if method.class_descriptor == "Ljava/io/InputStreamReader;"
            && method.name == "<init>"
            && matches!(
                method.proto.descriptor().as_str(),
                "(Ljava/io/InputStream;)V"
                    | "(Ljava/io/InputStream;Ljava/lang/String;)V"
                    | "(Ljava/io/InputStream;Ljava/nio/charset/Charset;)V"
            )
        {
            let invocation = invocation?;
            self.write_reader_delegate(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/BufferedReader;"
            && method.name == "<init>"
            && matches!(
                method.proto.descriptor().as_str(),
                "(Ljava/io/Reader;)V" | "(Ljava/io/Reader;I)V"
            )
        {
            let invocation = invocation?;
            self.write_reader_delegate(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/io/BufferedReader;" | "Ljava/io/InputStreamReader;" | "Ljava/io/FileReader;"
        ) && method.name == "read"
            && method.proto.descriptor_is("()I")
        {
            // cite: libcore BufferedReader.java#read, fill
            // cite: libcore InputStreamReader.java#read
            // cite: libcore StreamDecoder.java#read0
            let invocation = invocation?;
            let value = self.reader_read_char(invocation.arguments.first()?)?;

            return Some(DexExecutionResult::Int(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/io/BufferedReader;"
                | "Ljava/io/InputStreamReader;"
                | "Ljava/io/FileReader;"
                | "Ljava/io/Reader;"
        ) && method.name == "readLine"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.reader_read_line(&invocation.arguments[0])?;

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/io/BufferedReader;" | "Ljava/io/InputStreamReader;" | "Ljava/io/FileReader;"
        ) && method.name == "close"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/InputStream;"
            && method.name == "close"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            if let Some(key) = self.resolve_input_stream_key(invocation.arguments.first()?) {
                self.runtime.borrow_mut().input_stream_values.remove(&key);
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "listFiles"
            && method.proto.descriptor_is("()[Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.file_list_files(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "lastModified"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.file_last_modified(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "getFreeSpace"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let value = self.file_free_space(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "length"
            && method.proto.descriptor_is("()J")
        {
            let invocation = invocation?;
            let path = self
                .resolved_guest_file(invocation.arguments.first()?, ExternalStorageAccess::Read);
            // cite: libcore File.java#length
            let value = path
                .and_then(|path| path.metadata().ok())
                .filter(|metadata| metadata.kind != MetadataKind::Symlink)
                .map(|metadata| metadata.len)
                .unwrap_or(0);

            return Some(DexExecutionResult::Value(DexValue::Long(value as i64)));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "delete"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let Some(path) = self
                .resolved_guest_file(invocation.arguments.first()?, ExternalStorageAccess::Write)
            else {
                return Some(DexExecutionResult::Int(0));
            };
            // cite: libcore File.java#delete
            if path.is_mutation_protected() {
                return Some(DexExecutionResult::Int(0));
            }
            let deleted = path.remove().is_ok();

            return Some(DexExecutionResult::Int(i32::from(deleted)));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "isFile"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let path = self
                .resolved_guest_file(invocation.arguments.first()?, ExternalStorageAccess::Read);
            // cite: libcore File.java#isFile
            return Some(DexExecutionResult::Int(i32::from(
                path.and_then(|path| path.metadata().ok())
                    .is_some_and(|metadata| metadata.kind == MetadataKind::File),
            )));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "getPath"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            // cite: libcore File.java#getPath
            let path = file_path_from_value(self, invocation.arguments.first()?)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(path),
            )));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "mkdir"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let Some(path) = self
                .resolved_guest_file(invocation.arguments.first()?, ExternalStorageAccess::Write)
            else {
                return Some(DexExecutionResult::Int(0));
            };
            // cite: libcore File.java#mkdir
            return Some(DexExecutionResult::Int(i32::from(path.mkdir().is_ok())));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "createNewFile"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let Some(path) = self
                .resolved_guest_file(invocation.arguments.first()?, ExternalStorageAccess::Write)
            else {
                return Some(DexExecutionResult::Int(0));
            };
            // cite: libcore File.java#createNewFile
            let created = path.create_new().is_ok();

            return Some(DexExecutionResult::Int(i32::from(created)));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "renameTo"
            && method.proto.descriptor_is("(Ljava/io/File;)Z")
        {
            let invocation = invocation?;
            let Some(from) = self
                .resolved_guest_file(invocation.arguments.first()?, ExternalStorageAccess::Write)
            else {
                return Some(DexExecutionResult::Int(0));
            };
            let Some(to) = self
                .resolved_guest_file(invocation.arguments.get(1)?, ExternalStorageAccess::Write)
            else {
                return Some(DexExecutionResult::Int(0));
            };
            // cite: libcore File.java#renameTo
            if from.mount() != to.mount()
                || from.is_mutation_protected()
                || to.is_mutation_protected()
            {
                return Some(DexExecutionResult::Int(0));
            }
            return Some(DexExecutionResult::Int(i32::from(
                from.rename_to(&to).is_ok(),
            )));
        }

        if method.class_descriptor == "Ljava/io/File;"
            && matches!(method.name.as_str(), "canRead" | "canWrite")
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let access = if method.name == "canWrite" {
                ExternalStorageAccess::Write
            } else {
                ExternalStorageAccess::Read
            };
            let path = self.resolved_guest_file(invocation.arguments.first()?, access);
            // cite: libcore File.java#canRead, canWrite
            // cite: libcore UnixFileSystem.java#checkAccess
            let allowed = path
                .and_then(|path| path.can_access(method.name == "canWrite").ok())
                .unwrap_or(false);

            return Some(DexExecutionResult::Int(i32::from(allowed)));
        }

        if method.class_descriptor == "Ljava/io/FileWriter;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/io/File;)V")
        {
            let invocation = invocation?;
            // cite: libcore FileWriter.java#FileWriter
            return self.initialize_file_writer_from_file(invocation);
        }

        if method.class_descriptor == "Ljava/io/BufferedWriter;"
            && method.name == "<init>"
            && matches!(
                method.proto.descriptor().as_str(),
                "(Ljava/io/Writer;)V" | "(Ljava/io/Writer;I)V"
            )
        {
            let invocation = invocation?;
            self.write_reader_delegate(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/io/BufferedWriter;" | "Ljava/io/FileWriter;" | "Ljava/io/Writer;"
        ) && method.name == "write"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            // cite: libcore Writer.java#write
            self.file_writer_write_string(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/io/BufferedWriter;" | "Ljava/io/FileWriter;"
        ) && matches!(method.name.as_str(), "close" | "flush")
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/FileOutputStream;"
            && method.name == "getChannel"
            && method
                .proto
                .descriptor_is("()Ljava/nio/channels/FileChannel;")
        {
            let invocation = invocation?;
            let value = self.file_output_stream_channel(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/nio/channels/FileChannel;"
            && method.name == "lock"
            && method.proto.descriptor_is("()Ljava/nio/channels/FileLock;")
        {
            let invocation = invocation?;
            let value = file_channel_lock(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/nio/channels/spi/AbstractInterruptibleChannel;"
            && method.name == "close"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/ByteArrayOutputStream;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/StringReader;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            self.initialize_string_reader(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/StringReader;"
            && method.name == "read"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.read_string_reader_char(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/io/StringReader;"
            && method.name == "read"
            && method.proto.descriptor_is("([CII)I")
        {
            let invocation = invocation?;
            let value = self.read_string_reader_buffer(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/io/StringReader;"
            && method.name == "close"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.close_string_reader(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/Reader;"
            && method.name == "read"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            self.require_string_reader_receiver(invocation)?;
            let value = self.read_string_reader_char(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/io/Reader;"
            && method.name == "read"
            && method.proto.descriptor_is("([CII)I")
        {
            let invocation = invocation?;
            self.require_string_reader_receiver(invocation)?;
            let value = self.read_string_reader_buffer(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/io/Reader;"
            && method.name == "close"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.require_string_reader_receiver(invocation)?;
            self.close_string_reader(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && method.name == "<init>"
            && matches!(method.proto.descriptor().as_str(), "()V" | "(I)V")
        {
            let invocation = invocation?;
            self.initialize_string_writer(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && method.name == "write"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            self.write_string_writer_string(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && method.name == "write"
            && method.proto.descriptor_is("(Ljava/lang/String;II)V")
        {
            let invocation = invocation?;
            self.write_string_writer_string_range(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && method.name == "write"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.write_string_writer_char(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && method.name == "append"
            && method
                .proto
                .descriptor_is("(Ljava/lang/CharSequence;)Ljava/io/StringWriter;")
        {
            let invocation = invocation?;
            let value = self.append_string_writer(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && method.name == "append"
            && method
                .proto
                .descriptor_is("(Ljava/lang/CharSequence;II)Ljava/io/StringWriter;")
        {
            let invocation = invocation?;
            let value = self.append_string_writer_range(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && method.name == "append"
            && method.proto.descriptor_is("(C)Ljava/io/StringWriter;")
        {
            let invocation = invocation?;
            let value = self.append_string_writer_char(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && method.name == "toString"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.string_writer_to_string(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/io/StringWriter;"
            && matches!(method.name.as_str(), "flush" | "close")
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/Writer;"
            && method.name == "write"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            self.require_string_writer_receiver(invocation)?;
            self.write_string_writer_string(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/Writer;"
            && method.name == "write"
            && method.proto.descriptor_is("(Ljava/lang/String;II)V")
        {
            let invocation = invocation?;
            self.require_string_writer_receiver(invocation)?;
            self.write_string_writer_string_range(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/Writer;"
            && method.name == "write"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.require_string_writer_receiver(invocation)?;
            self.write_string_writer_char(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/Writer;"
            && method.name == "append"
            && method
                .proto
                .descriptor_is("(Ljava/lang/CharSequence;)Ljava/io/Writer;")
        {
            let invocation = invocation?;
            self.require_string_writer_receiver(invocation)?;
            let value = self.append_string_writer(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/Writer;"
            && method.name == "append"
            && method
                .proto
                .descriptor_is("(Ljava/lang/CharSequence;II)Ljava/io/Writer;")
        {
            let invocation = invocation?;
            self.require_string_writer_receiver(invocation)?;
            let value = self.append_string_writer_range(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/Writer;"
            && method.name == "append"
            && method.proto.descriptor_is("(C)Ljava/io/Writer;")
        {
            let invocation = invocation?;
            self.require_string_writer_receiver(invocation)?;
            let value = self.append_string_writer_char(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/Writer;"
            && matches!(method.name.as_str(), "flush" | "close")
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.require_string_writer_receiver(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/DataOutputStream;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/io/OutputStream;)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/DataOutputStream;"
            && method.name == "writeShort"
            && method.proto.descriptor_is("(I)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/DataOutputStream;"
            && method.name == "writeInt"
            && method.proto.descriptor_is("(I)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/DataOutputStream;"
            && method.name == "writeByte"
            && method.proto.descriptor_is("(I)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/DataOutputStream;"
            && method.name == "writeBoolean"
            && method.proto.descriptor_is("(Z)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/DataOutputStream;"
            && method.name == "writeUTF"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/DataOutputStream;"
            && method.name == "flush"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/io/DataOutputStream;"
            && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Ljava/io/ByteArrayOutputStream;"
            && method.name == "toByteArray"
            && method.proto.descriptor_is("()[B")
        {
            let value = self.allocate_filled_array("[B".to_owned(), Vec::new());

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/io/Closeable;"
            && method.name == "close"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        None
    }
}

impl DexProgram {
    fn initialize_file_from_parent_and_child(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 3 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let parent = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => None,
            value => Some(file_path_from_value(self, value)?),
        };
        let DexValue::String(child) = &invocation.arguments[2] else {
            return None;
        };

        // cite: libcore File.java#File
        // cite: libcore UnixFileSystem.java#normalize, resolve, getDefaultParent
        let path = resolve_file_parent_and_child(parent.as_deref(), child);

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &file_path_field(),
            DexValue::String(std::rc::Rc::from(path)),
        )
        .then_some(())
    }

    fn initialize_file_from_parent_string_and_child(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let parent = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => None,
            DexValue::String(parent) => Some(parent),
            _ => return None,
        };
        let DexValue::String(child) = &invocation.arguments[2] else {
            return None;
        };

        // cite: libcore File.java#File
        // cite: libcore UnixFileSystem.java#normalize, resolve
        let path = match parent {
            Some(parent) if !parent.is_empty() => {
                let parent = crate::guest_filesystem::normalize_guest_file_path(parent);
                resolve_file_parent_and_child(Some(&parent), child)
            }
            _ => resolve_file_parent_and_child(None, child),
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &file_path_field(),
            DexValue::String(std::rc::Rc::from(path)),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn null_pointer_exception_result(&self) -> DexExecutionResult {
        DexExecutionResult::Throw(self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: "Ljava/lang/NullPointerException;".to_owned(),
            detail_message: None,
        }))
    }

    fn initialize_file_from_path(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::String(pathname) = &invocation.arguments[1] else {
            return None;
        };

        // cite: libcore File.java#File
        // cite: libcore UnixFileSystem.java#normalize
        let path = resolve_file_parent_and_child(None, pathname);

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &file_path_field(),
            DexValue::String(std::rc::Rc::from(path)),
        )
        .then_some(())
    }

    fn file_parent(&self, invocation: &DexInvocation) -> Option<Option<String>> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let path = file_path_from_value(self, &invocation.arguments[0])?;
        // cite: libcore File.java#getParent
        Some(file_parent_path(&path).map(ToOwned::to_owned))
    }

    fn file_get_parent_file(&self, invocation: &DexInvocation) -> Option<DexValue> {
        // cite: libcore File.java#getParentFile
        let Some(parent) = self.file_parent(invocation)? else {
            return Some(DexValue::Null);
        };

        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/io/File;:{parent}"
        ))))
    }

    fn file_get_name(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let path = file_path_from_value(self, &invocation.arguments[0])?;
        // cite: libcore File.java#getName
        Some(file_name(&path).to_owned())
    }

    fn file_get_absolute_file(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let absolute_path = self.file_absolute_path(invocation)?;

        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/io/File;:{absolute_path}"
        ))))
    }

    fn file_get_absolute_path(&self, invocation: &DexInvocation) -> Option<String> {
        self.file_absolute_path(invocation)
    }

    fn file_absolute_path(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let path = file_path_from_value(self, &invocation.arguments[0])?;
        // cite: libcore File.java#getAbsolutePath
        // cite: libcore UnixFileSystem.java#resolve
        // cite: libcore System.java#getProperties
        Some(crate::guest_filesystem::absolute_guest_path(&path))
    }

    fn file_get_canonical_file(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let path = file_path_from_value(self, &invocation.arguments[0])?;
        let absolute_path = crate::guest_filesystem::absolute_guest_path(&path);
        let filesystem = self.guest_filesystem.as_ref()?;
        if filesystem
            .resolve(
                &absolute_path,
                ExternalStorageAccess::Read,
                &self.external_storage_policy,
            )
            .is_err()
        {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/io/IOException;".to_owned()),
            ));
        }
        // cite: libcore File.java#getCanonicalPath
        // cite: libcore canonicalize_md.c#canonicalize, collapse
        let canonical_path = match crate::guest_filesystem::canonical_guest_path(&absolute_path) {
            Ok(path) => path,
            Err(_) => {
                return Some(DexExecutionResult::Throw(
                    self.allocate_object("Ljava/io/IOException;".to_owned()),
                ));
            }
        };

        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/io/File;:{canonical_path}")),
        )))
    }

    fn file_mkdirs(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let Some(path) =
            self.resolved_guest_file(&invocation.arguments[0], ExternalStorageAccess::Write)
        else {
            return Some(0);
        };
        if path
            .metadata()
            .is_ok_and(|metadata| metadata.kind == MetadataKind::Directory)
        {
            return Some(0);
        }

        Some(i32::from(path.mkdir_all().is_ok()))
    }

    fn file_is_directory(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let path = self.resolved_guest_file(&invocation.arguments[0], ExternalStorageAccess::Read);
        Some(i32::from(
            path.and_then(|path| path.metadata().ok())
                .is_some_and(|metadata| metadata.kind == MetadataKind::Directory),
        ))
    }

    // cite: libcore File.java#isHidden
    // cite: libcore UnixFileSystem.java#getBooleanAttributes
    fn file_is_hidden(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let path = file_path_from_value(self, &invocation.arguments[0])?;
        let name = file_name(&path);
        Some(i32::from(name.starts_with('.')))
    }

    fn file_equals(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let path = file_path_from_value(self, &invocation.arguments[0])?;
        let other = &invocation.arguments[1];
        if !self.value_matches_type(other, "Ljava/io/File;") {
            return Some(0);
        }
        let other_path = file_path_from_value(self, other)?;

        Some(i32::from(path == other_path))
    }

    // cite: libcore File.java#getFreeSpace, isInvalid
    // cite: libcore UnixFileSystem.java#getSpace
    // cite: libcore UnixFileSystem_md.c#getSpace0
    fn file_free_space(&self, invocation: &DexInvocation) -> Option<i64> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let path = file_path_from_value(self, &invocation.arguments[0])?;
        if path.contains('\0') {
            return Some(0);
        }
        let free_bytes = self
            .resolved_guest_path(&path, ExternalStorageAccess::Read)
            .and_then(|resolved| resolved.volume_stats().ok())
            .map(|stats| (stats.free_blocks as i64).wrapping_mul(stats.block_size as i64))
            .unwrap_or(0);

        Some(free_bytes)
    }

    fn file_exists(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let path = self.resolved_guest_file(&invocation.arguments[0], ExternalStorageAccess::Read);
        Some(i32::from(
            path.and_then(|path| path.metadata().ok())
                .is_some_and(|metadata| metadata.kind != MetadataKind::Symlink),
        ))
    }

    fn initialize_file_output_stream_from_file(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let guest_path = file_path_from_value(self, &invocation.arguments[1])?;
        let path = match self.resolved_guest_path_result(&guest_path, ExternalStorageAccess::Write)
        {
            Ok(path) => path,
            Err(detail) => {
                let exception = self.file_not_found_exception_for_path(&guest_path, &detail);
                return Some(DexExecutionResult::Throw(exception));
            }
        };
        // cite: libcore FileOutputStream.java#FileOutputStream
        // cite: libcore IoBridge.java#open
        if let Err(error) = path.open_truncate() {
            let exception = self.file_not_found_exception_for_path(&guest_path, &error.to_string());
            return Some(DexExecutionResult::Throw(exception));
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &file_output_stream_path_field(),
            DexValue::String(std::rc::Rc::from(guest_path)),
        )
        .then_some(DexExecutionResult::Void)
    }

    fn initialize_file_input_stream_from_file(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let guest_path = file_path_from_value(self, &invocation.arguments[1])?;

        self.open_file_input_stream(&invocation.arguments[0], guest_path)
    }

    fn initialize_file_input_stream_from_path(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        // cite: libcore FileInputStream.java#FileInputStream
        // cite: libcore File.java#File
        // cite: libcore UnixFileSystem.java#normalize
        let guest_path = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => return Some(self.null_pointer_exception_result()),
            DexValue::String(name) => resolve_file_parent_and_child(None, name),
            _ => return None,
        };

        self.open_file_input_stream(&invocation.arguments[0], guest_path)
    }

    pub(in crate::dex_run) fn open_guest_file_for_read(
        &self,
        guest_path: &str,
    ) -> std::result::Result<crate::guest_filesystem::ResolvedGuestPath, DexValue> {
        // cite: libcore File.java#isInvalid
        if guest_path.contains('\0') {
            let exception = self.allocate_object("Ljava/io/FileNotFoundException;".to_owned());
            self.write_stored_instance_field(
                &exception,
                &throwable_detail_message_field(),
                DexValue::String(std::rc::Rc::from("Invalid file path")),
            );
            return Err(exception);
        }

        let path = self
            .resolved_guest_path_result(guest_path, ExternalStorageAccess::Read)
            .map_err(|detail| self.file_not_found_exception_for_path(guest_path, &detail))?;

        // cite: libcore IoBridge.java#open
        let file = path.open_read().map_err(|error| {
            self.file_not_found_exception_for_path(guest_path, &error.to_string())
        })?;
        let metadata = file.metadata().map_err(|error| {
            self.file_not_found_exception_for_path(guest_path, &error.to_string())
        })?;
        if metadata.is_dir() {
            return Err(self.file_not_found_exception_for_path(guest_path, "Is a directory"));
        }

        Ok(path)
    }

    fn open_file_input_stream(
        &self,
        stream: &DexValue,
        guest_path: String,
    ) -> Option<DexExecutionResult> {
        // cite: libcore FileInputStream.java#FileInputStream
        if let Err(exception) = self.open_guest_file_for_read(&guest_path) {
            return Some(DexExecutionResult::Throw(exception));
        }

        self.write_stored_instance_field(
            stream,
            &file_input_stream_path_field(),
            DexValue::String(std::rc::Rc::from(guest_path)),
        )
        .then_some(DexExecutionResult::Void)
    }

    // cite: libcore RandomAccessFile.java#RandomAccessFile
    fn initialize_random_access_file(
        &self,
        invocation: &DexInvocation,
        from_file: bool,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 3 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let name = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => None,
            value if from_file => Some(file_path_from_value(self, value)?),
            DexValue::String(name) => Some(resolve_file_parent_and_child(None, name)),
            _ => return None,
        };
        let mode = match &invocation.arguments[2] {
            DexValue::String(mode) => mode.clone(),
            DexValue::Null | DexValue::Int(0) => return Some(self.null_pointer_exception_result()),
            _ => return None,
        };
        match mode.as_ref() {
            "r" => {}
            "rw" | "rws" | "rwd" => return None,
            _ => {
                return Some(DexExecutionResult::Throw(self.allocate_generated_exception(
                    DexGeneratedException {
                        class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
                        detail_message: Some(format!(
                            "Illegal mode \"{mode}\" must be one of \"r\", \"rw\", \"rws\", or \"rwd\""
                        )),
                    },
                )));
            }
        }
        let Some(name) = name else {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/NullPointerException;".to_owned(),
                    detail_message: Some("file == null".to_owned()),
                }),
            ));
        };
        if let Err(exception) = self.open_guest_file_for_read(&name) {
            return Some(DexExecutionResult::Throw(exception));
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &random_access_file_field("Ljava/lang/String;", "path"),
            DexValue::String(std::rc::Rc::from(name)),
        )
        .then_some(DexExecutionResult::Void)
    }

    // cite: libcore RandomAccessFile.java#getChannel
    fn random_access_file_channel(&self, invocation: &DexInvocation) -> Option<DexValue> {
        let file = invocation.arguments.first()?;
        let channel_field = random_access_file_field("Ljava/nio/channels/FileChannel;", "channel");
        if let Some(channel @ DexValue::Object(_)) =
            self.read_stored_instance_field(file, &channel_field)
        {
            return Some(channel);
        }

        let DexValue::String(path) = self.read_stored_instance_field(
            file,
            &random_access_file_field("Ljava/lang/String;", "path"),
        )?
        else {
            return None;
        };
        if self.random_access_file_is_closed(file) {
            return None;
        }
        let channel = self.new_file_channel(path.to_string());
        self.write_stored_instance_field(file, &channel_field, channel.clone());

        Some(channel)
    }

    // cite: libcore RandomAccessFile.java#close
    fn close_random_access_file(&self, invocation: &DexInvocation) -> Option<()> {
        let file = invocation.arguments.first()?;
        let DexValue::String(_) = self.read_stored_instance_field(
            file,
            &random_access_file_field("Ljava/lang/String;", "path"),
        )?
        else {
            return None;
        };
        if let Some(channel @ DexValue::Object(_)) = self.read_stored_instance_field(
            file,
            &random_access_file_field("Ljava/nio/channels/FileChannel;", "channel"),
        ) {
            if self.file_channel_is_open(&channel)? {
                self.close_file_channel(&channel);
            }
        }

        self.write_stored_instance_field(
            file,
            &random_access_file_field("Z", "closed"),
            DexValue::Int(1),
        )
        .then_some(())
    }

    fn random_access_file_is_closed(&self, file: &DexValue) -> bool {
        matches!(
            self.read_stored_instance_field(file, &random_access_file_field("Z", "closed")),
            Some(DexValue::Int(closed)) if closed != 0
        )
    }

    fn file_output_stream_channel(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::String(path) = self.read_stored_instance_field(
            &invocation.arguments[0],
            &file_output_stream_path_field(),
        )?
        else {
            return None;
        };

        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/nio/channels/FileChannel;:{path}"
        ))))
    }

    // cite: libcore File.java#listFiles
    pub(in crate::dex_run) fn file_list_files_with_filter(
        &self,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<DexValue> {
        if invocation.arguments.len() != 2 {
            return Err(WinedroidError::new(format!(
                "File.listFiles(FileFilter) expected 2 arguments, got {}",
                invocation.arguments.len()
            )));
        }

        let unfiltered_invocation = DexInvocation {
            arguments: smallvec::smallvec![invocation.arguments[0].clone()],
            ..invocation.clone()
        };
        let Some(unfiltered) = self.file_list_files(&unfiltered_invocation) else {
            return Err(WinedroidError::new(
                "File.listFiles(FileFilter) could not resolve the receiver's directory",
            ));
        };
        if unfiltered == DexValue::Null {
            return Ok(DexValue::Null);
        }

        let filter = invocation.arguments[1].clone();
        if filter == DexValue::Null {
            return Ok(unfiltered);
        }

        let candidates = self
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&unfiltered))
            .cloned()
            .unwrap_or_default();

        let Some(filter_class) = value_class_descriptor(&filter).map(str::to_owned) else {
            return Err(WinedroidError::new(
                "File.listFiles(FileFilter) requires an object-backed filter",
            ));
        };
        let accept_method = DexMethodId {
            class_descriptor: filter_class.clone(),
            name: "accept".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "ZL".to_owned(),
                return_type: "Z".to_owned(),
                parameters: vec!["Ljava/io/File;".to_owned()],
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(&filter_class, &accept_method)?
        else {
            return Err(WinedroidError::new(format!(
                "File.listFiles(FileFilter) accept() was not found\nfilter class: {filter_class}"
            )));
        };

        let mut kept = Vec::new();
        for candidate in candidates {
            let result = self.execute_method(
                None,
                &resolved,
                Some(&[filter.clone(), candidate.clone()]),
                depth + 1,
            )?;
            let accepted = match result {
                DexExecutionResult::Int(value) => value != 0,
                DexExecutionResult::Throw(exception) => {
                    return Err(WinedroidError::new(format!(
                        "File.listFiles(FileFilter) accept() threw uncaught {}",
                        self.describe_uncaught_throw(&exception)
                    )));
                }
                DexExecutionResult::Value(_) | DexExecutionResult::Void => {
                    return Err(WinedroidError::new(
                        "File.listFiles(FileFilter) accept() did not return a boolean",
                    ));
                }
                DexExecutionResult::Parked(execution) => {
                    self.discard_parked_execution(&execution);
                    return Err(WinedroidError::new(
                        "File.listFiles(FileFilter) accept() did not complete normally",
                    ));
                }
            };
            if accepted {
                kept.push(candidate);
            }
        }

        Ok(self.allocate_filled_array("[Ljava/io/File;".to_owned(), kept))
    }

    fn file_list_files(&self, invocation: &DexInvocation) -> Option<DexValue> {
        let guest_path = file_path_from_value(self, invocation.arguments.first()?)?;
        let Some(path) = self.resolved_guest_path(&guest_path, ExternalStorageAccess::Read) else {
            return Some(DexValue::Null);
        };

        // cite: libcore File.java#listFiles
        let Ok(entries) = path.read_dir_names() else {
            return Some(DexValue::Null);
        };

        let mut files = Vec::new();
        for name in entries {
            let child_path = resolve_file_parent_and_child(Some(&guest_path), &name);
            files.push(DexValue::Reference(std::rc::Rc::from(format!(
                "Ljava/io/File;:{child_path}"
            ))));
        }

        Some(self.allocate_filled_array("[Ljava/io/File;".to_owned(), files))
    }

    fn file_last_modified(&self, invocation: &DexInvocation) -> Option<i64> {
        let path =
            self.resolved_guest_file(invocation.arguments.first()?, ExternalStorageAccess::Read);

        // cite: libcore File.java#lastModified
        let millis = path
            .and_then(|path| path.metadata().ok())
            .filter(|metadata| metadata.kind != MetadataKind::Symlink)
            .map(|metadata| metadata.modified_millis)
            .unwrap_or(0);

        Some(millis)
    }

    fn initialize_file_writer_from_file(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let guest_path = file_path_from_value(self, &invocation.arguments[1])?;
        let path = match self.resolved_guest_path_result(&guest_path, ExternalStorageAccess::Write)
        {
            Ok(path) => path,
            Err(detail) => {
                let exception = self.file_not_found_exception_for_path(&guest_path, &detail);
                return Some(DexExecutionResult::Throw(exception));
            }
        };
        // cite: libcore FileWriter.java#FileWriter
        if let Err(error) = path.open_truncate() {
            let exception = self.file_not_found_exception_for_path(&guest_path, &error.to_string());
            return Some(DexExecutionResult::Throw(exception));
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &file_writer_path_field(),
            DexValue::String(std::rc::Rc::from(guest_path)),
        )
        .then_some(DexExecutionResult::Void)
    }

    fn resolve_file_writer_path(&self, value: &DexValue) -> Option<String> {
        if let Some(DexValue::String(path)) =
            self.read_stored_instance_field(value, &file_writer_path_field())
        {
            return Some(path.to_string());
        }

        let delegate = self.read_stored_instance_field(value, &reader_delegate_field())?;
        self.resolve_file_writer_path(&delegate)
    }

    fn file_writer_write_string(&self, invocation: &DexInvocation) -> Option<()> {
        let writer = invocation.arguments.first()?;
        let DexValue::String(text) = invocation.arguments.get(1)? else {
            return None;
        };
        let guest_path = self.resolve_file_writer_path(writer)?;
        let path = self.resolved_guest_path(&guest_path, ExternalStorageAccess::Write)?;

        use std::io::Write;
        let mut file = path.open_append().ok()?;
        file.write_all(text.as_bytes()).ok()?;

        Some(())
    }

    // cite: libcore FileReader.java#FileReader
    // cite: libcore FileInputStream.java#FileInputStream
    fn initialize_file_reader(
        &self,
        invocation: &DexInvocation,
        from_file: bool,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let guest_path = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => return Some(self.null_pointer_exception_result()),
            value if from_file => file_path_from_value(self, value)?,
            DexValue::String(path) => resolve_file_parent_and_child(None, path),
            _ => return None,
        };
        // cite: frameworks-base android_util_Process.cpp#android_os_Process_getPidsForCommands
        let bytes = if guest_path == "/proc/self/cmdline"
            || guest_path == format!("/proc/{BOOTSTRAP_PROCESS_ID}/cmdline")
        {
            crate::native::bionic::proc_process::cmdline_for_name(&self.bootstrap_process_name())
        } else {
            let path =
                match self.resolved_guest_path_result(&guest_path, ExternalStorageAccess::Read) {
                    Ok(path) => path,
                    Err(detail) => {
                        return Some(DexExecutionResult::Throw(
                            self.file_not_found_exception_for_path(&guest_path, &detail),
                        ));
                    }
                };
            match path.read() {
                Ok(bytes) => bytes,
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.file_not_found_exception_for_path(&guest_path, &error.to_string()),
                    ));
                }
            }
        };

        self.runtime.borrow_mut().input_stream_values.insert(
            map_identity_key(&invocation.arguments[0]),
            InputStreamState {
                bytes,
                position: 0,
                pending_low_surrogate: None,
            },
        );

        Some(DexExecutionResult::Void)
    }

    fn write_reader_delegate(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() < 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &reader_delegate_field(),
            invocation.arguments[1].clone(),
        )
        .then_some(())
    }

    pub(super) fn drain_input_stream_bytes(&self, stream: &DexValue) -> Option<Vec<u8>> {
        let key = self.resolve_input_stream_key(stream)?;
        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.input_stream_values.get_mut(&key)?;
        let start = state.position.min(state.bytes.len());
        state.position = state.bytes.len();
        Some(state.bytes[start..].to_vec())
    }

    fn resolve_input_stream_key(&self, value: &DexValue) -> Option<String> {
        if dex_value_is_object_like(value) {
            let key = map_identity_key(value);
            if self.runtime.borrow().input_stream_values.contains_key(&key) {
                return Some(key);
            }
        }

        let delegate = self.read_stored_instance_field(value, &reader_delegate_field())?;
        self.resolve_input_stream_key(&delegate)
    }

    fn reader_read_char(&self, reader: &DexValue) -> Option<i32> {
        let key = self.resolve_input_stream_key(reader)?;
        let mut runtime = self.runtime.borrow_mut();
        let stream = runtime.input_stream_values.get_mut(&key)?;
        if let Some(low_surrogate) = stream.pending_low_surrogate.take() {
            return Some(i32::from(low_surrogate));
        }
        if stream.position >= stream.bytes.len() {
            return Some(-1);
        }

        let (scalar, consumed) = decode_utf8_scalar_replacing(&stream.bytes[stream.position..]);
        stream.position += consumed;
        let mut units = [0u16; 2];
        let encoded = scalar.encode_utf16(&mut units);
        if let [high, low] = *encoded {
            stream.pending_low_surrogate = Some(low);
            return Some(i32::from(high));
        }

        Some(i32::from(encoded[0]))
    }

    fn reader_read_line(&self, reader: &DexValue) -> Option<DexValue> {
        let key = self.resolve_input_stream_key(reader)?;
        let mut runtime = self.runtime.borrow_mut();
        let stream = runtime.input_stream_values.get_mut(&key)?;
        if stream.pending_low_surrogate.is_some() {
            return None;
        }
        if stream.position >= stream.bytes.len() {
            return Some(DexValue::Null);
        }

        let start = stream.position;
        let mut end = start;
        while end < stream.bytes.len() && stream.bytes[end] != b'\n' {
            end += 1;
        }

        let mut line_bytes = &stream.bytes[start..end];
        if line_bytes.last() == Some(&b'\r') {
            line_bytes = &line_bytes[..line_bytes.len() - 1];
        }
        let line = String::from_utf8_lossy(line_bytes).into_owned();

        stream.position = if end < stream.bytes.len() {
            end + 1
        } else {
            end
        };

        Some(DexValue::String(std::rc::Rc::from(line)))
    }

    fn file_not_found_exception_for_path(&self, path: &str, detail: &str) -> DexValue {
        let exception = self.allocate_object("Ljava/io/FileNotFoundException;".to_owned());
        self.write_stored_instance_field(
            &exception,
            &throwable_detail_message_field(),
            DexValue::String(std::rc::Rc::from(format!("{path}: {detail}"))),
        );

        exception
    }

    // cite: frameworks-base StatFs.java#StatFs
    pub(in crate::dex_run) fn initialize_stat_fs(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::String(path) = &invocation.arguments[1] else {
            return None;
        };

        let Some(resolved) = self.resolved_guest_path(path, ExternalStorageAccess::Read) else {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
            ));
        };
        let Ok(stats) = resolved.volume_stats() else {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
            ));
        };

        let receiver = &invocation.arguments[0];
        self.write_stored_instance_field(
            receiver,
            &stat_fs_field("blockSize"),
            DexValue::Long(stats.block_size as i64),
        );
        self.write_stored_instance_field(
            receiver,
            &stat_fs_field("blockCount"),
            DexValue::Long(stats.total_blocks as i64),
        );
        self.write_stored_instance_field(
            receiver,
            &stat_fs_field("freeBlocks"),
            DexValue::Long(stats.free_blocks as i64),
        );
        self.write_stored_instance_field(
            receiver,
            &stat_fs_field("availableBlocks"),
            DexValue::Long(stats.available_blocks as i64),
        );

        Some(DexExecutionResult::Void)
    }

    fn stat_fs_field_value(&self, invocation: &DexInvocation, name: &str) -> Option<i64> {
        if invocation.arguments.len() != 1 {
            return None;
        }
        let DexValue::Long(value) =
            self.read_stored_instance_field(&invocation.arguments[0], &stat_fs_field(name))?
        else {
            return None;
        };

        Some(value)
    }

    // cite: frameworks-base StatFs.java#getBlockSizeLong
    pub(in crate::dex_run) fn stat_fs_block_size(&self, invocation: &DexInvocation) -> Option<i64> {
        self.stat_fs_field_value(invocation, "blockSize")
    }

    // cite: frameworks-base StatFs.java#getBlockCountLong
    pub(in crate::dex_run) fn stat_fs_block_count(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        self.stat_fs_field_value(invocation, "blockCount")
    }

    // cite: frameworks-base StatFs.java#getFreeBlocksLong
    pub(in crate::dex_run) fn stat_fs_free_blocks(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        self.stat_fs_field_value(invocation, "freeBlocks")
    }

    // cite: frameworks-base StatFs.java#getFreeBytes
    pub(in crate::dex_run) fn stat_fs_free_bytes(&self, invocation: &DexInvocation) -> Option<i64> {
        let free_blocks = self.stat_fs_field_value(invocation, "freeBlocks")?;
        let block_size = self.stat_fs_field_value(invocation, "blockSize")?;
        Some(free_blocks.wrapping_mul(block_size))
    }

    // cite: frameworks-base StatFs.java#getAvailableBlocksLong
    pub(in crate::dex_run) fn stat_fs_available_blocks(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        self.stat_fs_field_value(invocation, "availableBlocks")
    }

    // cite: frameworks-base StatFs.java#getAvailableBytes
    pub(in crate::dex_run) fn stat_fs_available_bytes(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        let available_blocks = self.stat_fs_field_value(invocation, "availableBlocks")?;
        let block_size = self.stat_fs_field_value(invocation, "blockSize")?;
        Some(available_blocks.wrapping_mul(block_size))
    }

    // cite: frameworks-base StatFs.java#getTotalBytes
    pub(in crate::dex_run) fn stat_fs_total_bytes(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        let total_blocks = self.stat_fs_field_value(invocation, "blockCount")?;
        let block_size = self.stat_fs_field_value(invocation, "blockSize")?;
        Some(total_blocks.wrapping_mul(block_size))
    }

    fn resolved_guest_file(
        &self,
        value: &DexValue,
        access: ExternalStorageAccess,
    ) -> Option<crate::guest_filesystem::ResolvedGuestPath> {
        let guest_path = file_path_from_value(self, value)?;
        self.guest_filesystem
            .as_ref()?
            .resolve(&guest_path, access, &self.external_storage_policy)
            .ok()
    }

    fn resolved_guest_path(
        &self,
        guest_path: &str,
        access: ExternalStorageAccess,
    ) -> Option<crate::guest_filesystem::ResolvedGuestPath> {
        self.resolved_guest_path_result(guest_path, access).ok()
    }

    fn resolved_guest_path_result(
        &self,
        guest_path: &str,
        access: ExternalStorageAccess,
    ) -> std::result::Result<crate::guest_filesystem::ResolvedGuestPath, String> {
        let filesystem = self
            .guest_filesystem
            .as_ref()
            .ok_or_else(|| "guest filesystem is unavailable".to_owned())?;
        filesystem
            .resolve(guest_path, access, &self.external_storage_policy)
            .map_err(|error| error.to_string())
    }
}

fn file_trailing_path_is_null(invocation: &DexInvocation) -> bool {
    matches!(
        invocation.arguments.last(),
        Some(DexValue::Null | DexValue::Int(0))
    )
}

// cite: libcore StreamDecoder.java#read0
fn decode_utf8_scalar_replacing(bytes: &[u8]) -> (char, usize) {
    let window = &bytes[..bytes.len().min(4)];
    let valid_length = match std::str::from_utf8(window) {
        Ok(_) => window.len(),
        Err(error) => error.valid_up_to(),
    };
    if valid_length > 0 {
        if let Some(scalar) = std::str::from_utf8(&window[..valid_length])
            .ok()
            .and_then(|valid| valid.chars().next())
        {
            return (scalar, scalar.len_utf8());
        }
    }

    let invalid_length = std::str::from_utf8(window)
        .err()
        .and_then(|error| error.error_len())
        .unwrap_or(window.len());

    (char::REPLACEMENT_CHARACTER, invalid_length)
}

fn resolve_file_parent_and_child(parent: Option<&str>, child: &str) -> String {
    let child = crate::guest_filesystem::normalize_guest_file_path(child);
    let Some(parent) = parent else {
        return child;
    };
    let parent = if parent.is_empty() { "/" } else { parent };

    if child.is_empty() || child == "/" {
        return parent.to_owned();
    }
    if child.starts_with('/') {
        if parent == "/" {
            return child;
        }
        return format!("{parent}{child}");
    }
    if parent == "/" {
        return format!("/{child}");
    }

    format!("{parent}/{child}")
}

fn file_parent_path(path: &str) -> Option<&str> {
    let prefix_length = usize::from(path.starts_with('/'));
    let index = path.rfind('/')?;
    if index < prefix_length {
        return (prefix_length > 0 && path.len() > prefix_length).then_some(&path[..prefix_length]);
    }

    Some(&path[..index])
}

fn file_name(path: &str) -> &str {
    let prefix_length = usize::from(path.starts_with('/'));
    match path.rfind('/') {
        Some(index) if index >= prefix_length => &path[index + 1..],
        _ => &path[prefix_length..],
    }
}

pub(super) fn file_path_from_value(program: &DexProgram, value: &DexValue) -> Option<String> {
    match value {
        DexValue::Reference(reference) => file_reference_path(reference).map(ToOwned::to_owned),
        DexValue::Object(_) => {
            let DexValue::String(path) =
                program.read_stored_instance_field(value, &file_path_field())?
            else {
                return None;
            };
            Some(path.to_string())
        }
        DexValue::Uninitialized
        | DexValue::Null
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_) => None,
    }
}

fn file_reference_path(reference: &str) -> Option<&str> {
    reference.strip_prefix("Ljava/io/File;:")
}

fn file_channel_path(value: &DexValue) -> Option<&str> {
    let DexValue::Reference(reference) = value else {
        return None;
    };

    reference.strip_prefix("Ljava/nio/channels/FileChannel;:")
}

fn file_channel_lock(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let path = file_channel_path(&invocation.arguments[0])?;
    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/nio/channels/FileLock;:{path}"
    ))))
}

fn stat_fs_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/os/StatFs;".to_owned(),
        type_descriptor: "J".to_owned(),
        name: name.to_owned(),
    }
}

fn file_path_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    }
}

fn random_access_file_field(type_descriptor: &str, name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/io/RandomAccessFile;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

fn file_output_stream_path_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/io/FileOutputStream;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    }
}

fn reader_delegate_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/io/Reader;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "winedroidDelegate".to_owned(),
    }
}

fn file_writer_path_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/io/FileWriter;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "winedroidPath".to_owned(),
    }
}

fn file_input_stream_path_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/io/FileInputStream;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    }
}

fn throwable_detail_message_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    }
}
