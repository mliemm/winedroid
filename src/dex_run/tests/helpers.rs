use super::super::*;
use crate::dex_interpreter::DexInvokeKind;
pub(super) fn test_invocation(arguments: Vec<DexValue>) -> DexInvocation {
    DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: Default::default(),
        arguments: arguments.into_iter().collect(),
    }
}

pub(super) fn minimal_field_class_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const FIELD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * 5;
    let field_ids_off = type_ids_off + TYPE_ID_SIZE * 3;
    let class_defs_off = field_ids_off + FIELD_ID_SIZE * 2;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let strings = [
        "Lcom/example/Test;",
        "I",
        "Ljava/lang/String;",
        "staticValue",
        "value",
    ];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC | DEX_ACC_STATIC);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0x0002);

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, 5);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 3);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 80, 2);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }

    write_test_u32(&mut data, type_ids_off, 0);
    write_test_u32(&mut data, type_ids_off + TYPE_ID_SIZE, 1);
    write_test_u32(&mut data, type_ids_off + TYPE_ID_SIZE * 2, 2);

    write_test_u16(&mut data, field_ids_off, 0);
    write_test_u16(&mut data, field_ids_off + 2, 1);
    write_test_u32(&mut data, field_ids_off + 4, 3);

    write_test_u16(&mut data, field_ids_off + FIELD_ID_SIZE, 0);
    write_test_u16(&mut data, field_ids_off + FIELD_ID_SIZE + 2, 2);
    write_test_u32(&mut data, field_ids_off + FIELD_ID_SIZE + 4, 4);

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(
        &mut data,
        class_defs_off + 4,
        JAVA_ACC_PUBLIC | JAVA_ACC_FINAL,
    );
    write_test_u32(&mut data, class_defs_off + 8, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}

pub(super) fn minimal_slot_gap_field_class_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const FIELD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let strings = [
        "Lcom/example/Slot;",
        "I",
        "[Ljava/lang/Object;",
        "[I",
        "slotIndex",
        "slotLimit",
        "objectArray",
        "objectGapStart",
        "objectGapSize",
        "objectAnchor",
        "groupArray",
        "groupGapStart",
        "groupGapSize",
        "insertCount",
        "insertAnchor",
    ];
    let field_type_indices = [1u16, 1, 2, 1, 1, 1, 3, 1, 1, 1, 1];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let field_ids_off = type_ids_off + TYPE_ID_SIZE * 4;
    let class_defs_off = field_ids_off + FIELD_ID_SIZE * field_type_indices.len();
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, field_type_indices.len() as u32);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    for index in 0..field_type_indices.len() {
        write_test_uleb128(&mut data, if index == 0 { 0 } else { 1 });
        write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    }

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 4);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 80, field_type_indices.len() as u32);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }

    for index in 0..4 {
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }

    for (index, type_index) in field_type_indices.iter().enumerate() {
        let offset = field_ids_off + index * FIELD_ID_SIZE;
        write_test_u16(&mut data, offset, 0);
        write_test_u16(&mut data, offset + 2, *type_index);
        write_test_u32(&mut data, offset + 4, (4 + index) as u32);
    }

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(
        &mut data,
        class_defs_off + 4,
        JAVA_ACC_PUBLIC | JAVA_ACC_FINAL,
    );
    write_test_u32(&mut data, class_defs_off + 8, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}

pub(super) fn minimal_list_builder_field_class_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const FIELD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let strings = [
        "Lcom/example/ListBuilder;",
        "Ljava/util/AbstractList;",
        "I",
        "Z",
        "[Ljava/lang/Object;",
        "array",
        "size",
        "readOnly",
        "modCount",
    ];
    let field_class_indices = [0u16, 0, 0, 1];
    let field_type_indices = [4u16, 2, 3, 2];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let field_ids_off = type_ids_off + TYPE_ID_SIZE * 5;
    let class_defs_off = field_ids_off + FIELD_ID_SIZE * field_type_indices.len();
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 3);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    for index in 0..3 {
        write_test_uleb128(&mut data, if index == 0 { 0 } else { 1 });
        write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    }

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 5);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 80, field_type_indices.len() as u32);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }

    for index in 0..5 {
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }

    for (index, type_index) in field_type_indices.iter().enumerate() {
        let offset = field_ids_off + index * FIELD_ID_SIZE;
        write_test_u16(&mut data, offset, field_class_indices[index]);
        write_test_u16(&mut data, offset + 2, *type_index);
        write_test_u32(&mut data, offset + 4, (5 + index) as u32);
    }

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(
        &mut data,
        class_defs_off + 4,
        JAVA_ACC_PUBLIC | JAVA_ACC_FINAL,
    );
    write_test_u32(&mut data, class_defs_off + 8, 1);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}

pub(super) fn minimal_inherited_shadow_field_class_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const FIELD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let strings = [
        "Lcom/example/Parent;",
        "Lcom/example/Child;",
        "I",
        "value",
        "childOnly",
    ];
    let field_class_indices = [0u16, 1, 1];
    let field_name_indices = [3u32, 3, 4];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let field_ids_off = type_ids_off + TYPE_ID_SIZE * 3;
    let class_defs_off = field_ids_off + FIELD_ID_SIZE * field_class_indices.len();
    let data_off = class_defs_off + CLASS_DEF_SIZE * 2;
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let parent_class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);

    let child_class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 2);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 3);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 80, field_class_indices.len() as u32);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 96, 2);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }

    for index in 0..3 {
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }

    for index in 0..field_class_indices.len() {
        let offset = field_ids_off + index * FIELD_ID_SIZE;
        write_test_u16(&mut data, offset, field_class_indices[index]);
        write_test_u16(&mut data, offset + 2, 2);
        write_test_u32(&mut data, offset + 4, field_name_indices[index]);
    }

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, parent_class_data_off);

    let child_class_def_off = class_defs_off + CLASS_DEF_SIZE;
    write_test_u32(&mut data, child_class_def_off, 1);
    write_test_u32(&mut data, child_class_def_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, child_class_def_off + 8, 0);
    write_test_u32(&mut data, child_class_def_off + 16, u32::MAX);
    write_test_u32(&mut data, child_class_def_off + 24, child_class_data_off);

    data
}

pub(super) fn minimal_static_fields_class_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const FIELD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let strings = [
        "Lcom/example/Statics;",
        "Ljava/lang/Object;",
        "J",
        "I",
        "Z",
        "count",
        "shared",
        "total",
        "enabled",
        "seed",
    ];
    let field_type_indices = [3u16, 1, 2, 4];
    let field_name_indices = [5u32, 6, 7, 8];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let field_ids_off = type_ids_off + TYPE_ID_SIZE * 5;
    let class_defs_off = field_ids_off + FIELD_ID_SIZE * field_type_indices.len();
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, field_type_indices.len() as u32);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    for index in 0..field_type_indices.len() {
        write_test_uleb128(&mut data, if index == 0 { 0 } else { 1 });
        write_test_uleb128(&mut data, JAVA_ACC_PUBLIC | DEX_ACC_STATIC);
    }

    let static_values_off = data.len() as u32;
    write_test_uleb128(&mut data, field_type_indices.len() as u32);
    data.push(0x04);
    data.push(7);
    data.push(0x17);
    data.push(9);
    data.push(0x06);
    data.push(11);
    data.push(0x3f);

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 5);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 80, field_type_indices.len() as u32);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }

    for index in 0..5 {
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }

    for index in 0..field_type_indices.len() {
        let offset = field_ids_off + index * FIELD_ID_SIZE;
        write_test_u16(&mut data, offset, 0);
        write_test_u16(&mut data, offset + 2, field_type_indices[index]);
        write_test_u32(&mut data, offset + 4, field_name_indices[index]);
    }

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);
    write_test_u32(&mut data, class_defs_off + 28, static_values_off);

    data
}

pub(super) fn minimal_class_stub_dex(class_descriptor: &str) -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const CLASS_DEF_SIZE: usize = 32;

    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE;
    let class_defs_off = type_ids_off + TYPE_ID_SIZE;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];

    let string_offset = data.len() as u32;
    write_test_uleb128(&mut data, class_descriptor.len() as u32);
    data.extend_from_slice(class_descriptor.as_bytes());
    data.push(0);

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, 1);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 1);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    write_test_u32(&mut data, string_ids_off, string_offset);
    write_test_u32(&mut data, type_ids_off, 0);
    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);

    data
}

pub(super) fn minimal_interface_dex() -> Vec<u8> {
    minimal_interface_implementation_dex("Lcom/example/Test;", "Lcom/example/Marker;", true)
}

pub(super) fn minimal_external_interface_implementation_dex(
    class_descriptor: &str,
    interface_descriptor: &str,
) -> Vec<u8> {
    minimal_interface_implementation_dex(class_descriptor, interface_descriptor, false)
}

fn minimal_interface_implementation_dex(
    class_descriptor: &str,
    interface_descriptor: &str,
    define_interface: bool,
) -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const CLASS_DEF_SIZE: usize = 32;

    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * 2;
    let class_defs_off = type_ids_off + TYPE_ID_SIZE * 2;
    let class_def_count = if define_interface { 2 } else { 1 };
    let data_off = class_defs_off + CLASS_DEF_SIZE * class_def_count;
    let mut data = vec![0; data_off];
    let strings = [class_descriptor, interface_descriptor];

    let interfaces_off = data.len() as u32;
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());

    let mut string_offsets = Vec::new();
    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, 2);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 2);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 96, class_def_count as u32);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(
        &mut data,
        class_defs_off + 4,
        JAVA_ACC_PUBLIC | JAVA_ACC_FINAL,
    );
    write_test_u32(&mut data, class_defs_off + 8, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 12, interfaces_off);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);

    if define_interface {
        let marker_class_def_off = class_defs_off + CLASS_DEF_SIZE;
        write_test_u32(&mut data, marker_class_def_off, 1);
        write_test_u32(
            &mut data,
            marker_class_def_off + 4,
            JAVA_ACC_PUBLIC | JAVA_ACC_INTERFACE | JAVA_ACC_ABSTRACT,
        );
        write_test_u32(&mut data, marker_class_def_off + 8, u32::MAX);
        write_test_u32(&mut data, marker_class_def_off + 12, 0);
        write_test_u32(&mut data, marker_class_def_off + 16, u32::MAX);
    }

    data
}

pub(super) fn minimal_measured_cubic_fields_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const FIELD_ID_SIZE: usize = 8;

    let strings = [
        "Lcom/example/Measured;",
        "Lcom/example/Cubic;",
        "F",
        "Lcom/example/Owner;",
        "Lcom/example/Center;",
        "[F",
        "Lcom/example/Pair;",
        "Ljava/lang/Object;",
        "cubic",
        "size",
        "start",
        "end",
        "owner",
        "center",
        "x",
        "y",
        "points",
        "first",
        "second",
    ];
    let field_class_indices = [0u16, 0, 0, 0, 0, 3, 4, 4, 1, 6, 6];
    let field_type_indices = [1u16, 2, 2, 2, 3, 4, 2, 2, 5, 7, 7];
    let field_name_indices = [8u32, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let field_ids_off = type_ids_off + TYPE_ID_SIZE * 8;
    let data_off = field_ids_off + FIELD_ID_SIZE * field_type_indices.len();
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 8);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 80, field_type_indices.len() as u32);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }
    for index in 0..8 {
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }
    for index in 0..field_type_indices.len() {
        let offset = field_ids_off + index * FIELD_ID_SIZE;
        write_test_u16(&mut data, offset, field_class_indices[index]);
        write_test_u16(&mut data, offset + 2, field_type_indices[index]);
        write_test_u32(&mut data, offset + 4, field_name_indices[index]);
    }

    data
}

pub(super) fn minimal_feature_distance_fields_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const FIELD_ID_SIZE: usize = 8;

    let strings = [
        "Lcom/example/Feature;",
        "Lcom/example/Corner;",
        "Lcom/example/Cubic;",
        "Ljava/util/List;",
        "[F",
        "Z",
        "cubics",
        "points",
        "convex",
    ];
    let field_class_indices = [0u16, 2, 1];
    let field_type_indices = [3u16, 4, 5];
    let field_name_indices = [6u32, 7, 8];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let field_ids_off = type_ids_off + TYPE_ID_SIZE * 6;
    let data_off = field_ids_off + FIELD_ID_SIZE * field_type_indices.len();
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 6);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 80, field_type_indices.len() as u32);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }
    for index in 0..6 {
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }
    for index in 0..field_type_indices.len() {
        let offset = field_ids_off + index * FIELD_ID_SIZE;
        write_test_u16(&mut data, offset, field_class_indices[index]);
        write_test_u16(&mut data, offset + 2, field_type_indices[index]);
        write_test_u32(&mut data, offset + 4, field_name_indices[index]);
    }

    data
}

pub(super) fn minimal_inflated_child_constructor_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const PROTO_ID_SIZE: usize = 12;
    const FIELD_ID_SIZE: usize = 8;
    const METHOD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;
    const DEX_ACC_CONSTRUCTOR: u32 = 0x0001_0000;

    let strings = [
        "Lexample/InflatedChild;",
        "Ljava/lang/Object;",
        "Landroid/content/Context;",
        "Landroid/util/AttributeSet;",
        "I",
        "V",
        "initialized",
        "<init>",
        "VLL",
    ];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let proto_ids_off = type_ids_off + TYPE_ID_SIZE * 6;
    let field_ids_off = proto_ids_off + PROTO_ID_SIZE;
    let method_ids_off = field_ids_off + FIELD_ID_SIZE;
    let class_defs_off = method_ids_off + METHOD_ID_SIZE;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];

    let parameters_off = data.len() as u32;
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());

    let mut string_offsets = Vec::new();
    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC | DEX_ACC_CONSTRUCTOR);
    let code_off_index = data.len();
    data.extend_from_slice(&[0, 0]);

    while data.len() % 4 != 0 {
        data.push(0);
    }
    let code_off = data.len() as u32;
    assert!((0x80..0x4000).contains(&code_off));
    data[code_off_index] = ((code_off & 0x7f) as u8) | 0x80;
    data[code_off_index + 1] = (code_off >> 7) as u8;

    data.extend_from_slice(&4u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&4u32.to_le_bytes());
    data.extend_from_slice(&0x1012u16.to_le_bytes());
    data.extend_from_slice(&0x1059u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0x000eu16.to_le_bytes());

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 6);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 72, 1);
    write_test_u32(&mut data, 76, proto_ids_off as u32);
    write_test_u32(&mut data, 80, 1);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 88, 1);
    write_test_u32(&mut data, 92, method_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }
    for index in 0..6 {
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }

    write_test_u32(&mut data, proto_ids_off, 8);
    write_test_u32(&mut data, proto_ids_off + 4, 5);
    write_test_u32(&mut data, proto_ids_off + 8, parameters_off);

    write_test_u16(&mut data, field_ids_off, 0);
    write_test_u16(&mut data, field_ids_off + 2, 4);
    write_test_u32(&mut data, field_ids_off + 4, 6);

    write_test_u16(&mut data, method_ids_off, 0);
    write_test_u16(&mut data, method_ids_off + 2, 0);
    write_test_u32(&mut data, method_ids_off + 4, 7);

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, 1);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}

pub(super) fn minimal_inflated_attribute_constructor_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const PROTO_ID_SIZE: usize = 12;
    const FIELD_ID_SIZE: usize = 8;
    const METHOD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;
    const DEX_ACC_CONSTRUCTOR: u32 = 0x0001_0000;

    let strings = [
        "Lexample/InflatedAttributeChild;",
        "Ljava/lang/Object;",
        "Landroid/content/Context;",
        "Landroid/util/AttributeSet;",
        "Landroid/content/res/TypedArray;",
        "[I",
        "I",
        "V",
        "gravityValue",
        "<init>",
        "obtainStyledAttributes",
        "getInt",
        "VLL",
        "LLL",
        "III",
    ];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let proto_ids_off = type_ids_off + TYPE_ID_SIZE * 8;
    let field_ids_off = proto_ids_off + PROTO_ID_SIZE * 3;
    let method_ids_off = field_ids_off + FIELD_ID_SIZE;
    let class_defs_off = method_ids_off + METHOD_ID_SIZE * 3;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];

    let constructor_parameters_off = data.len() as u32;
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());

    let obtain_parameters_off = data.len() as u32;
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    data.extend_from_slice(&5u16.to_le_bytes());

    let get_int_parameters_off = data.len() as u32;
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&6u16.to_le_bytes());
    data.extend_from_slice(&6u16.to_le_bytes());

    let mut string_offsets = Vec::new();
    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC | DEX_ACC_CONSTRUCTOR);
    let code_off_index = data.len();
    data.extend_from_slice(&[0, 0]);

    while data.len() % 4 != 0 {
        data.push(0);
    }
    let code_off = data.len() as u32;
    assert!((0x80..0x4000).contains(&code_off));
    data[code_off_index] = ((code_off & 0x7f) as u8) | 0x80;
    data[code_off_index + 1] = (code_off >> 7) as u8;

    data.extend_from_slice(&8u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&21u32.to_le_bytes());
    for instruction in [
        0x1012u16, 0x0023, 0x0005, 0x0114, 0x00af, 0x0101, 0x0212, 0x014b, 0x0200, 0x306e, 0x0001,
        0x0076, 0x030c, 0xf412, 0x306e, 0x0002, 0x0423, 0x010a, 0x5159, 0x0000, 0x000e,
    ] {
        data.extend_from_slice(&instruction.to_le_bytes());
    }

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 8);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 72, 3);
    write_test_u32(&mut data, 76, proto_ids_off as u32);
    write_test_u32(&mut data, 80, 1);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 88, 3);
    write_test_u32(&mut data, 92, method_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }
    for index in 0..8 {
        write_test_u32(&mut data, type_ids_off + index * TYPE_ID_SIZE, index as u32);
    }

    write_test_u32(&mut data, proto_ids_off, 12);
    write_test_u32(&mut data, proto_ids_off + 4, 7);
    write_test_u32(&mut data, proto_ids_off + 8, constructor_parameters_off);
    write_test_u32(&mut data, proto_ids_off + PROTO_ID_SIZE, 13);
    write_test_u32(&mut data, proto_ids_off + PROTO_ID_SIZE + 4, 4);
    write_test_u32(
        &mut data,
        proto_ids_off + PROTO_ID_SIZE + 8,
        obtain_parameters_off,
    );
    write_test_u32(&mut data, proto_ids_off + PROTO_ID_SIZE * 2, 14);
    write_test_u32(&mut data, proto_ids_off + PROTO_ID_SIZE * 2 + 4, 6);
    write_test_u32(
        &mut data,
        proto_ids_off + PROTO_ID_SIZE * 2 + 8,
        get_int_parameters_off,
    );

    write_test_u16(&mut data, field_ids_off, 0);
    write_test_u16(&mut data, field_ids_off + 2, 6);
    write_test_u32(&mut data, field_ids_off + 4, 8);

    write_test_u16(&mut data, method_ids_off, 0);
    write_test_u16(&mut data, method_ids_off + 2, 0);
    write_test_u32(&mut data, method_ids_off + 4, 9);
    write_test_u16(&mut data, method_ids_off + METHOD_ID_SIZE, 2);
    write_test_u16(&mut data, method_ids_off + METHOD_ID_SIZE + 2, 1);
    write_test_u32(&mut data, method_ids_off + METHOD_ID_SIZE + 4, 10);
    write_test_u16(&mut data, method_ids_off + METHOD_ID_SIZE * 2, 4);
    write_test_u16(&mut data, method_ids_off + METHOD_ID_SIZE * 2 + 2, 2);
    write_test_u32(&mut data, method_ids_off + METHOD_ID_SIZE * 2 + 4, 11);

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, 1);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}

pub(super) fn write_test_uleb128(data: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;

        if value != 0 {
            byte |= 0x80;
        }

        data.push(byte);

        if value == 0 {
            return;
        }
    }
}

pub(super) fn write_test_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub(super) fn write_test_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// cite: art dex_file_structs.h#AnnotationsDirectoryItem, AnnotationSetItem, AnnotationItem
// cite: art dex_file.h#kDexVisibilityRuntime, kDexAnnotationString
pub(super) fn minimal_annotated_class_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const CLASS_DEF_SIZE: usize = 32;
    const DEX_VISIBILITY_RUNTIME: u8 = 0x01;
    const DEX_ANNOTATION_STRING: u8 = 0x17;

    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * 4;
    let class_defs_off = type_ids_off + TYPE_ID_SIZE * 2;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let strings = ["Lcom/example/Test;", "Lcom/example/Name;", "value", "hello"];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let annotation_item_off = data.len() as u32;
    data.push(DEX_VISIBILITY_RUNTIME);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 2);
    data.push(DEX_ANNOTATION_STRING);
    data.push(3);

    while data.len() % 4 != 0 {
        data.push(0);
    }
    let annotation_set_off = data.len() as u32;
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&annotation_item_off.to_le_bytes());

    let annotations_directory_off = data.len() as u32;
    data.extend_from_slice(&annotation_set_off.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, 4);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 2);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }

    write_test_u32(&mut data, type_ids_off, 0);
    write_test_u32(&mut data, type_ids_off + TYPE_ID_SIZE, 1);

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 20, annotations_directory_off);

    data
}

pub(super) fn thrown_exception(result: Option<DexExecutionResult>) -> DexValue {
    let Some(DexExecutionResult::Throw(exception)) = result else {
        panic!("expected a thrown exception, got {result:?}");
    };

    exception
}

pub(super) fn thrown_detail_message(program: &DexProgram, exception: &DexValue) -> Option<String> {
    let field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };
    match program.read_stored_instance_field(exception, &field) {
        Some(DexValue::String(message)) => Some(message.to_string()),
        _ => None,
    }
}

pub(super) fn builtin_method(
    class_descriptor: &str,
    name: &str,
    return_type: &str,
    parameters: &[&str],
) -> DexMethodId {
    DexMethodId {
        class_descriptor: class_descriptor.to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: String::new(),
            return_type: return_type.to_owned(),
            parameters: parameters.iter().map(|value| (*value).to_owned()).collect(),
        },
    }
}
