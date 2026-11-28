use super::super::*;
use super::helpers::*;
use crate::package_restrictions::{
    COMPONENT_ENABLED_STATE_DEFAULT, COMPONENT_ENABLED_STATE_DISABLED,
    COMPONENT_ENABLED_STATE_DISABLED_USER, COMPONENT_ENABLED_STATE_ENABLED,
};
use std::sync::atomic::{AtomicU64, Ordering};

const PACKAGE_MANAGER: &str = "Landroid/content/pm/PackageManager;";
const PACKAGE: &str = "com.example";
const RECEIVER: &str = "com.example.TimeSetReceiver";
const MAIN: &str = "com.example.Main";
const SECOND: &str = "com.example.Second";
const DONT_KILL_APP: i32 = 1;

struct Prefix {
    root: PathBuf,
}

impl Prefix {
    fn new(name: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "winedroid-component-state-{name}-{}-{id}",
            std::process::id()
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(root.join("apps/com.example/data")).expect("data dir");
        fs::create_dir_all(root.join("storage")).expect("shared dir");
        Self { root }
    }

    fn program(&self, target_sdk: &str) -> DexProgram {
        let mut program = DexProgram::with_guest_filesystem(
            Vec::new(),
            PACKAGE,
            self.root.join("apps/com.example/data"),
            self.root.join("storage"),
        )
        .expect("program should load guest filesystem");
        program.apply_apk_manifest_context(
            ApkManifestContext::from_manifest(manifest(target_sdk)).expect("manifest context"),
        );
        program
    }
}

impl Drop for Prefix {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn activity(name: &str, enabled: Option<bool>, launcher: bool) -> crate::manifest::Activity {
    crate::manifest::Activity {
        name: name.to_owned(),
        exported: None,
        enabled,
        is_alias: false,
        target_activity: None,
        has_main_action: launcher,
        has_launcher_category: launcher,
        label: None,
        theme: None,
    }
}

fn manifest(target_sdk: &str) -> crate::manifest::AndroidManifest {
    crate::manifest::AndroidManifest {
        package_name: Some(PACKAGE.to_owned()),
        target_sdk_version: Some(target_sdk.to_owned()),
        uses_sdk_declared: true,
        activities: vec![
            activity(MAIN, None, true),
            activity(SECOND, Some(false), false),
            activity("com.example.Hidden", Some(false), true),
        ],
        receivers: vec![RECEIVER.to_owned()],
        ..crate::manifest::AndroidManifest::default()
    }
}

fn manager() -> DexValue {
    DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/pm/PackageManager;.bootstrap",
    ))
}

fn get(program: &DexProgram, component: DexValue) -> Option<DexExecutionResult> {
    let method = builtin_method(
        PACKAGE_MANAGER,
        "getComponentEnabledSetting",
        "I",
        &["Landroid/content/ComponentName;"],
    );
    program
        .package_manager_component_state_intrinsic(
            &method,
            &test_invocation(vec![manager(), component]),
        )
        .expect("get should not fail")
}

fn set(
    program: &DexProgram,
    component: DexValue,
    state: i32,
    flags: i32,
) -> Result<Option<DexExecutionResult>> {
    let method = builtin_method(
        PACKAGE_MANAGER,
        "setComponentEnabledSetting",
        "V",
        &["Landroid/content/ComponentName;", "I", "I"],
    );
    program.package_manager_component_state_intrinsic(
        &method,
        &test_invocation(vec![
            manager(),
            component,
            DexValue::Int(state),
            DexValue::Int(flags),
        ]),
    )
}

fn component(program: &DexProgram, package: &str, class: &str) -> DexValue {
    program.new_component_name(package, class)
}

fn thrown(program: &DexProgram, result: Option<DexExecutionResult>, class: &str) -> String {
    let exception = thrown_exception(result);
    assert!(program.value_matches_type(&exception, class), "{class}");
    thrown_detail_message(program, &exception).unwrap_or_default()
}

#[test]
fn frameworks_base_settings_get_component_enabled_setting_reports_default_for_unset_own_classes() {
    // cite: frameworks-base ComputerEngine.java#getComponentEnabledSettingInternal
    // cite: frameworks-base PackageSetting.java#getCurrentEnabledStateLPr
    let prefix = Prefix::new("get_default");
    let program = prefix.program("34");

    for class in [RECEIVER, "com.example.NotDeclared"] {
        assert_eq!(
            get(&program, component(&program, PACKAGE, class)),
            Some(DexExecutionResult::Int(COMPONENT_ENABLED_STATE_DEFAULT)),
            "{class}"
        );
    }
    for null in [DexValue::Null, DexValue::Int(0)] {
        assert_eq!(
            get(&program, null),
            Some(DexExecutionResult::Int(COMPONENT_ENABLED_STATE_DEFAULT)),
            "getComponentEnabledSettingInternal returns DEFAULT for a null component"
        );
    }
}

#[test]
fn frameworks_base_computer_engine_get_component_enabled_setting_rejects_unknown_packages() {
    // cite: frameworks-base ComputerEngine.java#getComponentEnabledSettingInternal
    // cite: frameworks-base ComponentName.java#toString
    let prefix = Prefix::new("get_unknown");
    let program = prefix.program("34");

    let message = thrown(
        &program,
        get(&program, component(&program, "com.other", "com.other.R")),
        "Ljava/lang/IllegalArgumentException;",
    );
    assert_eq!(
        message,
        "Unknown component: ComponentInfo{com.other/com.other.R}"
    );
}

#[test]
fn frameworks_base_set_enabled_settings_stores_enable_disable_and_restore() {
    // cite: frameworks-base PackageManagerService.java#setEnabledSettings, setEnabledSettingInternalLocked
    // cite: frameworks-base PackageSetting.java#enableComponentLPw, disableComponentLPw, restoreComponentLPw
    let prefix = Prefix::new("set_cycle");
    let program = prefix.program("34");
    let receiver = component(&program, PACKAGE, RECEIVER);

    for state in [
        COMPONENT_ENABLED_STATE_ENABLED,
        COMPONENT_ENABLED_STATE_DISABLED,
        COMPONENT_ENABLED_STATE_DEFAULT,
    ] {
        assert_eq!(
            set(&program, receiver.clone(), state, DONT_KILL_APP).unwrap(),
            Some(DexExecutionResult::Void)
        );
        assert_eq!(
            get(&program, receiver.clone()),
            Some(DexExecutionResult::Int(state)),
            "state {state}"
        );
    }
}

#[test]
fn frameworks_base_set_enabled_settings_ignores_user_states_for_a_component() {
    // cite: frameworks-base PackageManagerService.java#setEnabledSettingInternalLocked
    let prefix = Prefix::new("set_user_state");
    let program = prefix.program("34");
    let receiver = component(&program, PACKAGE, RECEIVER);

    assert_eq!(
        set(
            &program,
            receiver.clone(),
            COMPONENT_ENABLED_STATE_DISABLED_USER,
            DONT_KILL_APP
        )
        .unwrap(),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        get(&program, receiver),
        Some(DexExecutionResult::Int(COMPONENT_ENABLED_STATE_DEFAULT))
    );
}

#[test]
fn frameworks_base_set_enabled_settings_validates_state_package_and_class() {
    // cite: frameworks-base PackageManagerService.java#setEnabledSettings
    // cite: frameworks-base PackageManager.java#ComponentEnabledSetting
    let prefix = Prefix::new("set_checks");
    let program = prefix.program("34");
    let receiver = component(&program, PACKAGE, RECEIVER);

    for state in [-1, 5] {
        let message = thrown(
            &program,
            set(&program, receiver.clone(), state, DONT_KILL_APP).unwrap(),
            "Ljava/lang/IllegalArgumentException;",
        );
        assert_eq!(message, format!("Invalid new component state: {state}"));
    }
    let message = thrown(
        &program,
        set(
            &program,
            component(&program, "com.other", "com.other.R"),
            COMPONENT_ENABLED_STATE_ENABLED,
            DONT_KILL_APP,
        )
        .unwrap(),
        "Ljava/lang/SecurityException;",
    );
    assert_eq!(
        message,
        format!("Attempt to change component state; pid={BOOTSTRAP_PROCESS_ID}, uid={BOOTSTRAP_PROCESS_UID}, component=ComponentInfo{{com.other/com.other.R}}")
    );
    let message = thrown(
        &program,
        set(
            &program,
            component(&program, PACKAGE, "android.app.AppDetailsActivity"),
            COMPONENT_ENABLED_STATE_DISABLED,
            DONT_KILL_APP,
        )
        .unwrap(),
        "Ljava/lang/SecurityException;",
    );
    assert_eq!(message, "Cannot disable a system-generated component");
    let message = thrown(
        &program,
        set(
            &program,
            component(&program, PACKAGE, "com.example.NotDeclared"),
            COMPONENT_ENABLED_STATE_DISABLED,
            DONT_KILL_APP,
        )
        .unwrap(),
        "Ljava/lang/IllegalArgumentException;",
    );
    assert_eq!(
        message,
        "Component class com.example.NotDeclared does not exist in com.example"
    );
    for null in [DexValue::Null, DexValue::Int(0)] {
        thrown(
            &program,
            set(
                &program,
                null,
                COMPONENT_ENABLED_STATE_ENABLED,
                DONT_KILL_APP,
            )
            .unwrap(),
            "Ljava/lang/NullPointerException;",
        );
    }
}

#[test]
fn frameworks_base_set_enabled_settings_skips_an_unknown_class_before_jelly_bean() {
    // cite: frameworks-base PackageManagerService.java#setEnabledSettings
    // cite: frameworks-base Build.java#JELLY_BEAN
    let prefix = Prefix::new("set_pre_jb");
    let program = prefix.program("15");
    let unknown = component(&program, PACKAGE, "com.example.NotDeclared");

    assert_eq!(
        set(
            &program,
            unknown.clone(),
            COMPONENT_ENABLED_STATE_DISABLED,
            DONT_KILL_APP
        )
        .unwrap(),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        get(&program, unknown),
        Some(DexExecutionResult::Int(COMPONENT_ENABLED_STATE_DEFAULT))
    );
}

#[test]
fn frameworks_base_package_changed_without_dont_kill_app_is_a_missing_process_kill() {
    // cite: frameworks-base PackageManagerService.java#setEnabledSettings
    // cite: frameworks-base ActivityManagerService.java#cleanupDisabledPackageComponentsLocked
    let prefix = Prefix::new("set_kill");
    let program = prefix.program("34");
    let receiver = component(&program, PACKAGE, RECEIVER);

    let error = set(
        &program,
        receiver.clone(),
        COMPONENT_ENABLED_STATE_ENABLED,
        0,
    )
    .unwrap_err();
    assert_eq!(
        error.missing_primitive(),
        Some("package state change process kill")
    );
    assert_eq!(
        get(&program, receiver.clone()),
        Some(DexExecutionResult::Int(COMPONENT_ENABLED_STATE_DEFAULT)),
        "a refused kill leaves the state unchanged"
    );
    assert_eq!(
        set(&program, receiver, COMPONENT_ENABLED_STATE_DEFAULT, 0).unwrap(),
        Some(DexExecutionResult::Void),
        "setEnabledSettings returns before any broadcast when nothing changed"
    );
}

#[test]
fn frameworks_base_activity_manager_disabling_a_running_activity_is_a_missing_cleanup() {
    // cite: frameworks-base ActivityManagerService.java#cleanupDisabledPackageComponentsLocked
    let prefix = Prefix::new("live_activity");
    let program = prefix.program("34");
    program
        .runtime
        .borrow_mut()
        .activity_stack
        .push(RuntimeActivity {
            class_descriptor: "Lcom/example/Main;".to_owned(),
            receiver: DexValue::Null,
            method_signature: "onCreate(Landroid/os/Bundle;)V".to_owned(),
            result: "void".to_owned(),
            object_start: 0,
            content_layout_start: 0,
            inflated_layout_start: 0,
            initial_inflated_layout_end: 0,
            dialog_content_layout_start: 0,
            runtime_view_root_start: 0,
            view_lookup_start: 0,
            saved_chrome: None,
            result_target: None,
            result_code: 0,
            result_intent: None,
            adapter_rows_snapshot: std::collections::HashMap::new(),
            adapter_item_clicks_snapshot: std::collections::HashMap::new(),
        });

    let error = set(
        &program,
        component(&program, PACKAGE, MAIN),
        COMPONENT_ENABLED_STATE_DISABLED,
        DONT_KILL_APP,
    )
    .unwrap_err();
    assert_eq!(
        error.missing_primitive(),
        Some("ActivityManagerService cleanupDisabledPackageComponentsLocked")
    );
}

#[test]
fn frameworks_base_has_component_class_name_accepts_services_instrumentation_and_backup_agent() {
    // cite: frameworks-base AndroidPackageUtils.java#hasComponentClassName
    let prefix = Prefix::new("service");
    let mut program = prefix.program("34");
    program.apply_apk_manifest_context(
        ApkManifestContext::from_manifest(crate::manifest::AndroidManifest {
            services: vec!["com.example.Sync".to_owned()],
            instrumentations: vec!["com.example.Runner".to_owned()],
            backup_agent: Some("com.example.Backup".to_owned()),
            ..manifest("34")
        })
        .expect("manifest context"),
    );

    for class in [
        "com.example.Sync",
        "com.example.Runner",
        "com.example.Backup",
    ] {
        assert_eq!(
            set(
                &program,
                component(&program, PACKAGE, class),
                COMPONENT_ENABLED_STATE_DISABLED,
                DONT_KILL_APP,
            )
            .unwrap(),
            Some(DexExecutionResult::Void),
            "{class}"
        );
    }
}

#[test]
fn frameworks_base_package_restrictions_persist_across_program_instances() {
    // cite: frameworks-base Settings.java#writePackageRestrictionsLPr, readPackageRestrictionsLPr
    let prefix = Prefix::new("persist");
    let first = prefix.program("34");
    set(
        &first,
        component(&first, PACKAGE, RECEIVER),
        COMPONENT_ENABLED_STATE_DISABLED,
        DONT_KILL_APP,
    )
    .unwrap();
    assert!(prefix
        .root
        .join("apps/com.example/package-restrictions")
        .is_file());
    drop(first);

    let second = prefix.program("34");
    assert_eq!(
        get(&second, component(&second, PACKAGE, RECEIVER)),
        Some(DexExecutionResult::Int(COMPONENT_ENABLED_STATE_DISABLED))
    );
}

#[test]
fn frameworks_base_get_activity_info_hides_disabled_activities_unless_match_disabled() {
    // cite: frameworks-base ApplicationPackageManager.java#getActivityInfo
    // cite: frameworks-base ComputerEngine.java#getActivityInfoInternalBody
    // cite: frameworks-base PackageUserStateUtils.java#isEnabled
    let prefix = Prefix::new("activity_info");
    let program = prefix.program("34");
    let method = builtin_method(
        "Landroid/content/pm/PackageManager;",
        "getActivityInfo",
        "Landroid/content/pm/ActivityInfo;",
        &["Landroid/content/ComponentName;", "I"],
    );
    let ask = |class: &str, flags: i32| {
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                manager(),
                component(&program, PACKAGE, class),
                DexValue::Int(flags),
            ])),
        )
    };
    let found = Some(DexExecutionResult::Value(DexValue::Reference(
        std::rc::Rc::from("Landroid/content/pm/ActivityInfo;.bootstrap"),
    )));

    assert_eq!(ask(MAIN, 0), found);
    let message = thrown(
        &program,
        ask(SECOND, 0),
        "Landroid/content/pm/PackageManager$NameNotFoundException;",
    );
    assert_eq!(message, "ComponentInfo{com.example/com.example.Second}");
    assert_eq!(ask(SECOND, 0x200), found, "MATCH_DISABLED_COMPONENTS");
    for class in [RECEIVER, "com.example.NotDeclared"] {
        for flags in [0, 0x200] {
            thrown(
                &program,
                ask(class, flags),
                "Landroid/content/pm/PackageManager$NameNotFoundException;",
            );
        }
    }

    set(
        &program,
        component(&program, PACKAGE, SECOND),
        COMPONENT_ENABLED_STATE_ENABLED,
        DONT_KILL_APP,
    )
    .unwrap();
    assert_eq!(
        ask(SECOND, 0),
        found,
        "ENABLED overrides android:enabled=false"
    );
    set(
        &program,
        component(&program, PACKAGE, MAIN),
        COMPONENT_ENABLED_STATE_DISABLED,
        DONT_KILL_APP,
    )
    .unwrap();
    thrown(
        &program,
        ask(MAIN, 0),
        "Landroid/content/pm/PackageManager$NameNotFoundException;",
    );
}

#[test]
fn frameworks_base_instrumentation_explicit_start_of_a_disabled_activity_throws_activity_not_found()
{
    // cite: frameworks-base Instrumentation.java#checkStartActivityResult
    // cite: frameworks-base ComponentName.java#toShortString
    let prefix = Prefix::new("start_activity");
    let program = prefix.program("34");

    let exception = program
        .disabled_explicit_activity_exception("Lcom/example/Second;")
        .expect("a manifest disabled activity cannot be started");
    assert!(program.value_matches_type(&exception, "Ljava/lang/RuntimeException;"));
    assert_eq!(
        thrown_detail_message(&program, &exception).as_deref(),
        Some("Unable to find explicit activity class {com.example/com.example.Second}; have you declared this activity in your AndroidManifest.xml, or does your intent not match its declared <intent-filter>?")
    );
    assert_eq!(
        program.disabled_explicit_activity_exception("Lcom/example/Main;"),
        None
    );

    set(
        &program,
        component(&program, PACKAGE, SECOND),
        COMPONENT_ENABLED_STATE_ENABLED,
        DONT_KILL_APP,
    )
    .unwrap();
    assert_eq!(
        program.disabled_explicit_activity_exception("Lcom/example/Second;"),
        None,
        "ENABLED overrides android:enabled=false"
    );
}

#[test]
fn frameworks_base_launcher_query_resolves_the_single_launcher_left_by_stored_states() {
    // cite: frameworks-base ComputerEngine.java#queryIntentActivitiesInternal
    // cite: frameworks-base PackageUserStateUtils.java#isEnabled
    // cite: frameworks-base PackageParser.java#parseActivityAlias
    let prefix = Prefix::new("launcher_switch");
    let mut program = prefix.program("34");
    let alias = |name: &str, target: &str, enabled: Option<bool>| crate::manifest::Activity {
        is_alias: true,
        target_activity: Some(target.to_owned()),
        ..activity(name, enabled, true)
    };
    program.apply_apk_manifest_context(
        ApkManifestContext::from_manifest(crate::manifest::AndroidManifest {
            activities: vec![
                activity(MAIN, None, false),
                activity(SECOND, None, false),
                alias("com.example.IconDefault", MAIN, None),
                alias("com.example.IconNew", SECOND, Some(false)),
            ],
            ..manifest("34")
        })
        .expect("manifest context"),
    );
    assert_eq!(program.launcher_under_component_states(MAIN).unwrap(), MAIN);

    for (class, state) in [
        ("com.example.IconNew", COMPONENT_ENABLED_STATE_ENABLED),
        ("com.example.IconDefault", COMPONENT_ENABLED_STATE_DISABLED),
    ] {
        set(
            &program,
            component(&program, PACKAGE, class),
            state,
            DONT_KILL_APP,
        )
        .unwrap();
    }
    assert_eq!(
        program.launcher_under_component_states(MAIN).unwrap(),
        SECOND
    );
}

#[test]
fn frameworks_base_launcher_query_without_a_single_launcher_is_refused_visibly() {
    // cite: frameworks-base ComputerEngine.java#queryIntentActivitiesInternal
    // cite: frameworks-base PackageUserStateUtils.java#isEnabled
    let prefix = Prefix::new("launcher");
    for (class, state) in [
        (MAIN, COMPONENT_ENABLED_STATE_DISABLED),
        ("com.example.Hidden", COMPONENT_ENABLED_STATE_ENABLED),
    ] {
        let program = prefix.program("34");
        program.component_states.borrow_mut().set(class, state);
        let error = program.launcher_under_component_states(MAIN).unwrap_err();
        assert_eq!(
            error.missing_primitive(),
            Some("launcher activity resolution through package restrictions"),
            "{class}"
        );
    }
}

#[test]
fn frameworks_base_package_info_get_providers_honours_the_stored_component_state() {
    // cite: frameworks-base PackageInfoUtils.java#generateWithComponents, generateProviderInfo
    // cite: frameworks-base PackageUserStateUtils.java#isEnabled
    let prefix = Prefix::new("get_providers");
    let mut program = prefix.program("34");
    let provider = |name: &str, enabled: Option<bool>| crate::manifest::Provider {
        name: name.to_owned(),
        authority: format!("{name}.authority"),
        exported: None,
        enabled,
    };
    program.apply_apk_manifest_context(
        ApkManifestContext::from_manifest(crate::manifest::AndroidManifest {
            providers: vec![
                provider("com.example.On", None),
                provider("com.example.Off", Some(false)),
            ],
            ..manifest("34")
        })
        .expect("manifest context"),
    );
    set(
        &program,
        component(&program, PACKAGE, "com.example.On"),
        COMPONENT_ENABLED_STATE_DISABLED,
        DONT_KILL_APP,
    )
    .unwrap();
    set(
        &program,
        component(&program, PACKAGE, "com.example.Off"),
        COMPONENT_ENABLED_STATE_ENABLED,
        DONT_KILL_APP,
    )
    .unwrap();
    let method = builtin_method(
        "Landroid/content/pm/PackageManager;",
        "getPackageInfo",
        "Landroid/content/pm/PackageInfo;",
        &["Ljava/lang/String;", "I"],
    );
    let Some(DexExecutionResult::Value(info)) = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![
            manager(),
            DexValue::String(std::rc::Rc::from(PACKAGE)),
            DexValue::Int(0x8),
        ])),
    ) else {
        panic!("GET_PROVIDERS should return a PackageInfo");
    };
    let field = |owner: &str, name: &str, type_descriptor: &str| DexFieldId {
        class_descriptor: owner.to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    };
    let providers = program
        .read_stored_instance_field(
            &info,
            &field(
                "Landroid/content/pm/PackageInfo;",
                "providers",
                "[Landroid/content/pm/ProviderInfo;",
            ),
        )
        .expect("providers");
    assert_eq!(program.array_length(&providers), Some(1));
    let only = program.read_array_value(&providers, 0).expect("provider");
    let provider_info = |name: &str, type_descriptor: &str| {
        program.read_stored_instance_field(
            &only,
            &field("Landroid/content/pm/ProviderInfo;", name, type_descriptor),
        )
    };
    assert_eq!(
        provider_info("name", "Ljava/lang/String;"),
        Some(DexValue::String(std::rc::Rc::from("com.example.Off")))
    );
    assert_eq!(
        provider_info("enabled", "Z"),
        Some(DexValue::Int(0)),
        "ComponentInfo.enabled carries the manifest value"
    );
}

fn invoke(program: &DexProgram, method: DexMethodId, arguments: Vec<DexValue>) -> String {
    let descriptor = std::rc::Rc::from(method.proto.descriptor());
    program.caches.borrow_mut().method_ids = vec![vec![Some(std::rc::Rc::new(method))]];
    program.caches.borrow_mut().method_proto_descriptors = vec![vec![Some(descriptor)]];
    let result = DexRunInvocationHandler {
        program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    }
    .invoke(test_invocation(arguments))
    .expect("invoke should be handled");
    match result {
        DexInvocationResult::Throw(exception) => value_class_descriptor(&exception)
            .expect("exception class")
            .to_owned(),
        other => format!("{other:?}"),
    }
}

#[test]
fn frameworks_base_guest_calls_reach_the_component_state_and_start_activity_checks() {
    // cite: frameworks-base ApplicationPackageManager.java#setComponentEnabledSetting
    // cite: frameworks-base Instrumentation.java#execStartActivity, checkStartActivityResult
    let prefix = Prefix::new("guest_calls");
    let program = prefix.program("34");
    let receiver = component(&program, PACKAGE, RECEIVER);

    invoke(
        &program,
        builtin_method(
            PACKAGE_MANAGER,
            "setComponentEnabledSetting",
            "V",
            &["Landroid/content/ComponentName;", "I", "I"],
        ),
        vec![
            manager(),
            receiver.clone(),
            DexValue::Int(COMPONENT_ENABLED_STATE_ENABLED),
            DexValue::Int(DONT_KILL_APP),
        ],
    );
    assert_eq!(
        get(&program, receiver),
        Some(DexExecutionResult::Int(COMPONENT_ENABLED_STATE_ENABLED))
    );

    let intent = program.allocate_object("Landroid/content/Intent;".to_owned());
    program.write_stored_instance_field(
        &intent,
        &intent_target_class_field(),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Second;")),
    );
    let activity = DexValue::Reference(std::rc::Rc::from("Lcom/example/Main;.bootstrap"));
    assert_eq!(
        invoke(
            &program,
            builtin_method(
                "Lcom/example/Main;",
                "startActivity",
                "V",
                &["Landroid/content/Intent;"],
            ),
            vec![activity, intent],
        ),
        "Landroid/content/ActivityNotFoundException;"
    );
}

#[test]
fn frameworks_base_context_start_activity_routes_explicit_intents_through_the_enabled_check() {
    // cite: frameworks-base Instrumentation.java#execStartActivity, checkStartActivityResult
    let prefix = Prefix::new("start_route");
    let program = prefix.program("34");
    let handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let intent = program.allocate_object("Landroid/content/Intent;".to_owned());
    program.write_stored_instance_field(
        &intent,
        &intent_target_class_field(),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Second;")),
    );
    let activity = DexValue::Reference(std::rc::Rc::from("Lcom/example/Main;.bootstrap"));
    let start = |name: &str, parameters: &[&str], arguments: Vec<DexValue>| {
        handler.disabled_start_activity_target(
            &builtin_method("Lcom/example/Main;", name, "V", parameters),
            &test_invocation(arguments),
        )
    };

    for (name, parameters, arguments) in [
        (
            "startActivity",
            &["Landroid/content/Intent;"][..],
            vec![activity.clone(), intent.clone()],
        ),
        (
            "startActivityForResult",
            &["Landroid/content/Intent;", "I"][..],
            vec![activity.clone(), intent.clone(), DexValue::Int(7)],
        ),
    ] {
        let exception = start(name, parameters, arguments).expect(name);
        assert!(
            program.value_matches_type(&exception, "Landroid/content/ActivityNotFoundException;")
        );
    }
    assert_eq!(
        start(
            "startService",
            &["Landroid/content/Intent;"],
            vec![activity, intent]
        ),
        None
    );
}
