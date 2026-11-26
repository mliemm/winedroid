use super::super::*;
use super::helpers::*;

fn lru_method(name: &str, return_type: &str, parameters: &[&str]) -> DexMethodId {
    builtin_method("Landroid/util/LruCache;", name, return_type, parameters)
}

fn lru_call(
    program: &DexProgram,
    method: &DexMethodId,
    arguments: Vec<DexValue>,
) -> DexExecutionResult {
    program
        .invoke_lru_cache_method(method, &test_invocation(arguments), 0)
        .expect("LruCache call should not fail")
        .expect("LruCache method should be handled")
}

fn text(value: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(value))
}

fn lru_int(program: &DexProgram, cache: &DexValue, name: &str) -> DexExecutionResult {
    lru_call(program, &lru_method(name, "I", &[]), vec![cache.clone()])
}

fn new_cache(program: &DexProgram, class_descriptor: &str, max_size: i32) -> DexValue {
    let cache = program.allocate_object(class_descriptor.to_owned());
    assert_eq!(
        lru_call(
            program,
            &lru_method("<init>", "V", &["I"]),
            vec![cache.clone(), DexValue::Int(max_size)]
        ),
        DexExecutionResult::Void
    );
    cache
}

fn put(program: &DexProgram, cache: &DexValue, key: &str, value: &str) -> DexExecutionResult {
    lru_call(
        program,
        &lru_method(
            "put",
            "Ljava/lang/Object;",
            &["Ljava/lang/Object;", "Ljava/lang/Object;"],
        ),
        vec![cache.clone(), text(key), text(value)],
    )
}

fn get(program: &DexProgram, cache: &DexValue, key: &str) -> DexExecutionResult {
    lru_call(
        program,
        &lru_method("get", "Ljava/lang/Object;", &["Ljava/lang/Object;"]),
        vec![cache.clone(), text(key)],
    )
}

fn thrown_class(program: &DexProgram, result: DexExecutionResult, class_descriptor: &str) {
    let DexExecutionResult::Throw(exception) = result else {
        panic!("expected a thrown {class_descriptor}");
    };
    assert!(program.value_matches_type(&exception, class_descriptor));
}

#[test]
fn frameworks_lru_cache_constructor_rejects_non_positive_max_size() {
    // cite: frameworks-base LruCache.java#LruCache
    let program = DexProgram::new(Vec::new());
    let cache = program.allocate_object("Landroid/util/LruCache;".to_owned());

    for max_size in [0, -1] {
        thrown_class(
            &program,
            lru_call(
                &program,
                &lru_method("<init>", "V", &["I"]),
                vec![cache.clone(), DexValue::Int(max_size)],
            ),
            "Ljava/lang/IllegalArgumentException;",
        );
    }
}

#[test]
fn frameworks_lru_cache_evicts_the_least_recently_accessed_entry() {
    // cite: frameworks-base LruCache.java#get, put, trimToSize
    let program = DexProgram::new(Vec::new());
    let cache = new_cache(&program, "Landroid/util/LruCache;", 2);

    assert_eq!(
        put(&program, &cache, "a", "1"),
        DexExecutionResult::Value(DexValue::Null)
    );
    put(&program, &cache, "b", "2");
    assert_eq!(
        get(&program, &cache, "a"),
        DexExecutionResult::Value(text("1"))
    );
    put(&program, &cache, "c", "3");

    assert_eq!(
        get(&program, &cache, "b"),
        DexExecutionResult::Value(DexValue::Null)
    );
    assert_eq!(
        get(&program, &cache, "a"),
        DexExecutionResult::Value(text("1"))
    );
    assert_eq!(
        get(&program, &cache, "c"),
        DexExecutionResult::Value(text("3"))
    );
    assert_eq!(
        lru_int(&program, &cache, "size"),
        DexExecutionResult::Int(2)
    );
    assert_eq!(
        lru_int(&program, &cache, "evictionCount"),
        DexExecutionResult::Int(1)
    );
    assert_eq!(
        lru_int(&program, &cache, "putCount"),
        DexExecutionResult::Int(3)
    );
    assert_eq!(
        lru_int(&program, &cache, "hitCount"),
        DexExecutionResult::Int(3)
    );
    assert_eq!(
        lru_int(&program, &cache, "missCount"),
        DexExecutionResult::Int(1)
    );
}

#[test]
fn frameworks_lru_cache_put_returns_previous_and_remove_and_evict_all_release_size() {
    // cite: frameworks-base LruCache.java#put, remove, evictAll
    let program = DexProgram::new(Vec::new());
    let cache = new_cache(&program, "Landroid/util/LruCache;", 4);

    put(&program, &cache, "a", "1");
    assert_eq!(
        put(&program, &cache, "a", "2"),
        DexExecutionResult::Value(text("1"))
    );
    assert_eq!(
        lru_int(&program, &cache, "size"),
        DexExecutionResult::Int(1)
    );
    put(&program, &cache, "b", "3");
    let remove = lru_method("remove", "Ljava/lang/Object;", &["Ljava/lang/Object;"]);
    assert_eq!(
        lru_call(&program, &remove, vec![cache.clone(), text("a")]),
        DexExecutionResult::Value(text("2"))
    );
    assert_eq!(
        lru_call(&program, &remove, vec![cache.clone(), text("a")]),
        DexExecutionResult::Value(DexValue::Null)
    );
    assert_eq!(
        lru_int(&program, &cache, "size"),
        DexExecutionResult::Int(1)
    );
    assert_eq!(
        lru_call(
            &program,
            &lru_method("evictAll", "V", &[]),
            vec![cache.clone()]
        ),
        DexExecutionResult::Void
    );
    assert_eq!(
        lru_int(&program, &cache, "size"),
        DexExecutionResult::Int(0)
    );
    assert_eq!(
        get(&program, &cache, "b"),
        DexExecutionResult::Value(DexValue::Null)
    );
}

#[test]
fn frameworks_lru_cache_rejects_null_keys_and_values_with_the_libcore_messages() {
    // cite: frameworks-base LruCache.java#get, put, remove
    let program = DexProgram::new(Vec::new());
    let cache = new_cache(&program, "Landroid/util/LruCache;", 2);
    let object = "Ljava/lang/Object;";

    for result in [
        lru_call(
            &program,
            &lru_method("get", object, &[object]),
            vec![cache.clone(), DexValue::Null],
        ),
        lru_call(
            &program,
            &lru_method("remove", object, &[object]),
            vec![cache.clone(), DexValue::Null],
        ),
        lru_call(
            &program,
            &lru_method("put", object, &[object, object]),
            vec![cache.clone(), text("a"), DexValue::Null],
        ),
    ] {
        thrown_class(&program, result, "Ljava/lang/NullPointerException;");
    }
}

#[test]
fn frameworks_lru_cache_resize_trims_to_the_new_maximum() {
    // cite: frameworks-base LruCache.java#resize, trimToSize
    let program = DexProgram::new(Vec::new());
    let cache = new_cache(&program, "Landroid/util/LruCache;", 3);
    for key in ["a", "b", "c"] {
        put(&program, &cache, key, key);
    }
    let resize = lru_method("resize", "V", &["I"]);

    thrown_class(
        &program,
        lru_call(&program, &resize, vec![cache.clone(), DexValue::Int(0)]),
        "Ljava/lang/IllegalArgumentException;",
    );
    assert_eq!(
        lru_call(&program, &resize, vec![cache.clone(), DexValue::Int(1)]),
        DexExecutionResult::Void
    );
    assert_eq!(
        lru_int(&program, &cache, "size"),
        DexExecutionResult::Int(1)
    );
    assert_eq!(
        lru_int(&program, &cache, "maxSize"),
        DexExecutionResult::Int(1)
    );
    assert_eq!(
        get(&program, &cache, "c"),
        DexExecutionResult::Value(text("c"))
    );
    assert_eq!(
        get(&program, &cache, "a"),
        DexExecutionResult::Value(DexValue::Null)
    );
}

#[test]
fn frameworks_lru_cache_dispatches_size_of_to_the_guest_override() {
    // cite: frameworks-base LruCache.java#safeSizeOf, sizeOf
    let dex_file = crate::dex::parse_dex_file("classes.dex", lru_cache_size_of_override_dex(3))
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let cache = new_cache(&program, "Lcom/example/SizedCache;", 5);

    put(&program, &cache, "a", "1");
    assert_eq!(
        lru_int(&program, &cache, "size"),
        DexExecutionResult::Int(3)
    );
    put(&program, &cache, "b", "2");

    assert_eq!(
        lru_int(&program, &cache, "size"),
        DexExecutionResult::Int(3)
    );
    assert_eq!(
        lru_int(&program, &cache, "evictionCount"),
        DexExecutionResult::Int(1)
    );
    assert_eq!(
        get(&program, &cache, "a"),
        DexExecutionResult::Value(DexValue::Null)
    );
    assert_eq!(
        get(&program, &cache, "b"),
        DexExecutionResult::Value(text("2"))
    );
}

fn lru_cache_size_of_override_dex(size: u16) -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const PROTO_ID_SIZE: usize = 12;
    const METHOD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let strings = [
        "Lcom/example/SizedCache;",
        "Landroid/util/LruCache;",
        "I",
        "Ljava/lang/Object;",
        "ILL",
        "sizeOf",
    ];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let proto_ids_off = type_ids_off + TYPE_ID_SIZE * 4;
    let method_ids_off = proto_ids_off + PROTO_ID_SIZE;
    let class_defs_off = method_ids_off + METHOD_ID_SIZE;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let parameters_off = data.len() as u32;
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
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
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&(0x0012u16 | (size << 12)).to_le_bytes());
    data.extend_from_slice(&0x000fu16.to_le_bytes());

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE as u32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 4);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 72, 1);
    write_test_u32(&mut data, 76, proto_ids_off as u32);
    write_test_u32(&mut data, 88, 1);
    write_test_u32(&mut data, 92, method_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }
    for (index, string_index) in [0u32, 1, 2, 3].iter().enumerate() {
        write_test_u32(
            &mut data,
            type_ids_off + index * TYPE_ID_SIZE,
            *string_index,
        );
    }
    write_test_u32(&mut data, proto_ids_off, 4);
    write_test_u32(&mut data, proto_ids_off + 4, 2);
    write_test_u32(&mut data, proto_ids_off + 8, parameters_off);
    write_test_u16(&mut data, method_ids_off, 0);
    write_test_u16(&mut data, method_ids_off + 2, 0);
    write_test_u32(&mut data, method_ids_off + 4, 5);
    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, 1);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}
