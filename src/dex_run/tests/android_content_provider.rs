use super::super::*;
use super::helpers::*;

const RESOLVER: &str = "Landroid/content/ContentResolver;";
const PROVIDER: &str = "Landroid/content/ContentProvider;";
const QUERY_PARAMETERS: [&str; 5] = [
    "Landroid/net/Uri;",
    "[Ljava/lang/String;",
    "Ljava/lang/String;",
    "[Ljava/lang/String;",
    "Ljava/lang/String;",
];

fn text(value: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(value))
}

fn manifest_provider(authority: &str, enabled: Option<bool>) -> crate::manifest::Provider {
    crate::manifest::Provider {
        name: "com.example.SharedProvider".to_owned(),
        authority: authority.to_owned(),
        exported: Some(false),
        enabled,
    }
}

fn program_with_provider(provider: crate::manifest::Provider) -> DexProgram {
    program_with_provider_overrides(provider, None)
}

fn program_with_provider_overrides(
    provider: crate::manifest::Provider,
    extra_query: Option<&[u16]>,
) -> DexProgram {
    let dex = crate::dex::parse_dex_file("provider.dex", shared_provider_dex(extra_query))
        .expect("provider DEX should parse");
    let mut program = DexProgram::with_package_name(vec![dex], Some("com.example.app".to_owned()));
    program.manifest_providers = vec![provider];
    program
}

fn query(program: &DexProgram, uri: DexValue) -> Result<Option<DexExecutionResult>> {
    let method = builtin_method(
        RESOLVER,
        "query",
        "Landroid/database/Cursor;",
        &QUERY_PARAMETERS,
    );
    let resolver = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/ContentResolver;.bootstrap",
    ));
    program.invoke_content_provider_method(
        &method,
        &test_invocation(vec![
            resolver,
            uri,
            DexValue::Null,
            DexValue::Null,
            DexValue::Null,
            DexValue::Null,
        ]),
        0,
    )
}

fn parsed_uri(program: &DexProgram, text_value: &str) -> DexValue {
    program
        .parse_uri(&test_invocation(vec![text(text_value)]))
        .expect("Uri.parse should accept the text")
}

#[test]
fn frameworks_content_resolver_query_installs_the_manifest_provider_and_reaches_its_query() {
    // cite: frameworks-base ContentResolver.java#query, acquireUnstableProvider
    // cite: frameworks-base ActivityThread.java#installProvider
    // cite: frameworks-base ContentProvider.java#attachInfo, getCallingPackage
    let program = program_with_provider(manifest_provider("com.example.one;com.example.two", None));

    let first = parsed_uri(&program, "content://com.example.two/items");
    assert_eq!(
        query(&program, first).unwrap(),
        Some(DexExecutionResult::Value(text("com.example.app")))
    );

    let providers = program.runtime.borrow().content_providers.clone();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers["com.example.one"], providers["com.example.two"]);
    let get_context = builtin_method(PROVIDER, "getContext", "Landroid/content/Context;", &[]);
    assert!(matches!(
        program.invoke_content_provider_method(
            &get_context,
            &test_invocation(vec![providers["com.example.one"].clone()]),
            0
        ),
        Ok(Some(DexExecutionResult::Value(DexValue::Reference(_))))
    ));

    let calling_package = builtin_method(PROVIDER, "getCallingPackage", "Ljava/lang/String;", &[]);
    assert_eq!(
        program
            .invoke_content_provider_method(
                &calling_package,
                &test_invocation(vec![providers["com.example.one"].clone()]),
                0
            )
            .unwrap(),
        Some(DexExecutionResult::Value(DexValue::Null))
    );

    let second = parsed_uri(&program, "content://com.example.one/other");
    query(&program, second).unwrap();
    assert_eq!(program.runtime.borrow().content_providers.len(), 2);
    assert_eq!(
        program.runtime.borrow().content_providers["com.example.one"],
        providers["com.example.one"]
    );
}

#[test]
fn frameworks_content_provider_calling_package_is_null_outside_a_query() {
    // cite: frameworks-base ContentProvider.java#getCallingPackage
    let program = program_with_provider(manifest_provider("com.example.one", None));
    let provider = program.allocate_object("Lcom/example/SharedProvider;".to_owned());
    let method = builtin_method(PROVIDER, "getCallingPackage", "Ljava/lang/String;", &[]);

    assert_eq!(
        program
            .invoke_content_provider_method(&method, &test_invocation(vec![provider]), 0)
            .unwrap(),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn frameworks_content_resolver_query_returns_null_for_a_non_content_scheme() {
    // cite: frameworks-base ContentResolver.java#acquireUnstableProvider
    let program = program_with_provider(manifest_provider("com.example.one", None));
    let uri = parsed_uri(&program, "file://com.example.one/items");

    assert_eq!(
        query(&program, uri).unwrap(),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert!(program.runtime.borrow().content_providers.is_empty());
}

#[test]
fn frameworks_content_resolver_query_returns_null_without_an_authority() {
    // cite: frameworks-base ContentResolver.java#acquireUnstableProvider
    let program = program_with_provider(manifest_provider("com.example.one", None));
    let uri = parsed_uri(&program, "content:items");

    assert_eq!(
        query(&program, uri).unwrap(),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn frameworks_content_resolver_query_returns_null_for_a_disabled_provider() {
    // cite: frameworks-base ActivityThread.java#acquireProvider
    // cite: frameworks-base PackageParser.java#enabledRes
    let program = program_with_provider(manifest_provider("com.example.one", Some(false)));
    let uri = parsed_uri(&program, "content://com.example.one/items");

    assert_eq!(
        query(&program, uri).unwrap(),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert!(program.runtime.borrow().content_providers.is_empty());
}

#[test]
fn frameworks_content_resolver_query_rejects_a_null_uri_like_requirenonnull() {
    // cite: frameworks-base ContentResolver.java#query
    let program = program_with_provider(manifest_provider("com.example.one", None));
    let exception = thrown_exception(query(&program, DexValue::Null).unwrap());

    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
    assert_eq!(
        thrown_detail_message(&program, &exception).as_deref(),
        Some("uri")
    );
}

#[test]
fn frameworks_content_resolver_query_for_an_unknown_authority_is_an_explicit_missing_primitive() {
    // cite: frameworks-base ActivityThread.java#installProviderAuthoritiesLocked, ProviderKey
    let program = program_with_provider(manifest_provider("com.example.one", None));
    let uri = parsed_uri(&program, "content://settings/system");

    let error = query(&program, uri).expect_err("a system authority has no provider table");
    assert!(error.to_string().contains("next missing primitive"));
}

#[test]
fn frameworks_content_resolver_query_with_a_wider_provider_override_is_an_explicit_missing_primitive(
) {
    // cite: frameworks-base ContentProvider.java#query, Transport
    const URI: u16 = 5;
    const STRING_ARRAY: u16 = 6;
    const STRING: u16 = 4;
    const BUNDLE: u16 = 8;
    const CANCELLATION_SIGNAL: u16 = 9;
    for parameters in [
        &[URI, STRING_ARRAY, BUNDLE, CANCELLATION_SIGNAL][..],
        &[
            URI,
            STRING_ARRAY,
            STRING,
            STRING_ARRAY,
            STRING,
            CANCELLATION_SIGNAL,
        ][..],
    ] {
        let program = program_with_provider_overrides(
            manifest_provider("com.example.one", None),
            Some(parameters),
        );
        let uri = parsed_uri(&program, "content://com.example.one/items");

        let error = query(&program, uri).expect_err("the wider override is not dispatched");
        assert!(error
            .to_string()
            .contains("next missing primitive: ContentProvider.query override"));
    }
}

#[test]
fn frameworks_provider_info_constructor_starts_enabled() {
    // cite: frameworks-base ProviderInfo.java#ProviderInfo
    // cite: frameworks-base ComponentInfo.java#enabled
    let program = DexProgram::new(Vec::new());
    let info = program.allocate_object("Landroid/content/pm/ProviderInfo;".to_owned());
    let method = builtin_method("Landroid/content/pm/ProviderInfo;", "<init>", "V", &[]);

    assert_eq!(
        program
            .invoke_content_provider_method(&method, &test_invocation(vec![info.clone()]), 0)
            .unwrap(),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &info,
            &DexFieldId {
                class_descriptor: "Landroid/content/pm/ProviderInfo;".to_owned(),
                type_descriptor: "Z".to_owned(),
                name: "enabled".to_owned(),
            }
        ),
        Some(DexValue::Int(1))
    );
}

fn shared_provider_dex(extra_query: Option<&[u16]>) -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const PROTO_ID_SIZE: usize = 12;
    const METHOD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let extra_parameters = extra_query.unwrap_or(&[]);
    let extra = usize::from(extra_query.is_some());
    let extra_shorty = "L".repeat(extra_parameters.len() + 1);
    let mut strings = vec![
        "Lcom/example/SharedProvider;",
        "Landroid/content/ContentProvider;",
        "V",
        "Z",
        "Ljava/lang/String;",
        "Landroid/net/Uri;",
        "[Ljava/lang/String;",
        "Landroid/database/Cursor;",
        "L",
        "LLLLLL",
        "<init>",
        "onCreate",
        "query",
        "getCallingPackage",
    ];
    if extra == 1 {
        strings.extend(["Landroid/os/Bundle;", "Landroid/os/CancellationSignal;"]);
        strings.push(extra_shorty.as_str());
    }
    let type_count = 8 + 2 * extra;
    let proto_count = 4 + extra;
    let method_count = 5 + extra;
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let proto_ids_off = type_ids_off + TYPE_ID_SIZE * type_count;
    let method_ids_off = proto_ids_off + PROTO_ID_SIZE * proto_count;
    let class_defs_off = method_ids_off + METHOD_ID_SIZE * method_count;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in &strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let parameters_off = data.len() as u32;
    data.extend_from_slice(&5u32.to_le_bytes());
    for type_index in [5u16, 6, 4, 6, 4] {
        data.extend_from_slice(&type_index.to_le_bytes());
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let extra_parameters_off = data.len() as u32;
    if extra == 1 {
        data.extend_from_slice(&(extra_parameters.len() as u32).to_le_bytes());
        for type_index in extra_parameters {
            data.extend_from_slice(&type_index.to_le_bytes());
        }
        while data.len() % 4 != 0 {
            data.push(0);
        }
    }

    let mut code_items: Vec<(u16, u16, u16, Vec<u16>)> = vec![
        (1, 1, 1, vec![0x1070, 3, 0x0000, 0x000e]),
        (1, 1, 0, vec![0x1012, 0x000f]),
        (7, 6, 1, vec![0x106e, 4, 0x0001, 0x000c, 0x0011]),
    ];
    if extra == 1 {
        let ins = extra_parameters.len() as u16 + 1;
        code_items.push((ins + 1, ins, 0, vec![0x0012, 0x0011]));
    }
    let mut code_offsets = Vec::new();
    for (registers, ins, outs, insns) in code_items {
        while data.len() % 4 != 0 {
            data.push(0);
        }
        code_offsets.push(data.len() as u32);
        data.extend_from_slice(&registers.to_le_bytes());
        data.extend_from_slice(&ins.to_le_bytes());
        data.extend_from_slice(&outs.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&(insns.len() as u32).to_le_bytes());
        for unit in insns {
            data.extend_from_slice(&unit.to_le_bytes());
        }
    }

    while data.len() % 4 != 0 {
        data.push(0);
    }
    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 2 + extra as u32);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC | 0x10000);
    write_test_uleb128(&mut data, code_offsets[0]);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    write_test_uleb128(&mut data, code_offsets[1]);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    write_test_uleb128(&mut data, code_offsets[2]);
    if extra == 1 {
        write_test_uleb128(&mut data, 3);
        write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
        write_test_uleb128(&mut data, code_offsets[3]);
    }

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE as u32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, type_count as u32);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 72, proto_count as u32);
    write_test_u32(&mut data, 76, proto_ids_off as u32);
    write_test_u32(&mut data, 88, method_count as u32);
    write_test_u32(&mut data, 92, method_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }
    let type_strings: Vec<u32> = (0..8).chain((14..16).take(2 * extra)).collect();
    for (index, string_index) in type_strings.iter().enumerate() {
        write_test_u32(
            &mut data,
            type_ids_off + index * TYPE_ID_SIZE,
            *string_index,
        );
    }
    let mut protos = vec![
        (2u32, 2u32, 0u32),
        (3, 3, 0),
        (8, 4, 0),
        (9, 7, parameters_off),
    ];
    if extra == 1 {
        protos.push((16, 7, extra_parameters_off));
    }
    for (index, (shorty, return_type, parameters)) in protos.iter().enumerate() {
        let base = proto_ids_off + index * PROTO_ID_SIZE;
        write_test_u32(&mut data, base, *shorty);
        write_test_u32(&mut data, base + 4, *return_type);
        write_test_u32(&mut data, base + 8, *parameters);
    }
    let mut methods = vec![
        (0u16, 0u16, 10u32),
        (0, 1, 11),
        (0, 3, 12),
        (1, 0, 10),
        (1, 2, 13),
    ];
    if extra == 1 {
        methods.push((0, 4, 12));
    }
    for (index, (class_index, proto_index, name_index)) in methods.iter().enumerate() {
        let base = method_ids_off + index * METHOD_ID_SIZE;
        write_test_u16(&mut data, base, *class_index);
        write_test_u16(&mut data, base + 2, *proto_index);
        write_test_u32(&mut data, base + 4, *name_index);
    }
    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, 1);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}
