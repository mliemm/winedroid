use std::collections::HashSet;
use std::fmt;

use crate::binary_xml::{XmlEvent, XmlStartElement, XmlValue};
use crate::error::{Result, WinedroidError};

const ACTION_MAIN: &str = "android.intent.action.MAIN";
const CATEGORY_LAUNCHER: &str = "android.intent.category.LAUNCHER";
const ANDROID_NAMESPACE: &str = "http://schemas.android.com/apk/res/android";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AndroidManifest {
    pub package_name: Option<String>,
    pub application_name: Option<String>,
    pub application_theme: Option<u32>,
    pub version_code: Option<String>,
    pub version_name: Option<String>,
    pub compile_sdk_version: Option<String>,
    pub min_sdk_version: Option<String>,
    pub target_sdk_version: Option<String>,
    pub uses_sdk_declared: bool,
    pub permissions: Vec<UsesPermission>,
    pub activities: Vec<Activity>,
    pub providers: Vec<Provider>,
    pub application_label: Option<u32>,
    pub request_legacy_external_storage: Option<bool>,
    pub default_to_device_protected_storage: Option<bool>,
    pub direct_boot_aware: Option<bool>,
}

impl AndroidManifest {
    pub fn effective_min_sdk_version(&self) -> Result<i32> {
        // cite: frameworks-base PackageParser.java#DEFAULT_MIN_SDK_VERSION, parseBaseApkCommon
        let min_sdk_version = match self.min_sdk_version.as_deref() {
            Some(value) => parse_release_sdk_version("minSdkVersion", value)?,
            None if self.uses_sdk_declared => 1,
            None => 0,
        };
        Ok(min_sdk_version)
    }

    pub fn effective_target_sdk_version(&self) -> Result<i32> {
        // cite: frameworks-base PackageParser.java#DEFAULT_MIN_SDK_VERSION, DEFAULT_TARGET_SDK_VERSION, parseBaseApkCommon, computeTargetSdkVersion
        let min_sdk_version = self.effective_min_sdk_version()?;

        match self.target_sdk_version.as_deref() {
            Some(value) => parse_release_sdk_version("targetSdkVersion", value),
            None if self.uses_sdk_declared => Ok(min_sdk_version),
            None => Ok(0),
        }
    }

    pub fn requests_legacy_external_storage(&self) -> Result<bool> {
        // cite: frameworks-base PackageParser.java#parseBaseApplication, AndroidManifestApplication_requestLegacyExternalStorage
        match self.request_legacy_external_storage {
            Some(value) => Ok(value),
            None => Ok(self.effective_target_sdk_version()? < 29),
        }
    }

    pub fn defaults_to_device_protected_storage(&self, is_system_app: bool) -> bool {
        // cite: frameworks-base ScanPackageUtils.java#applyPolicy
        is_system_app && self.default_to_device_protected_storage.unwrap_or(false)
    }

    pub fn is_direct_boot_aware(&self, is_system_app: bool) -> bool {
        // cite: frameworks-base ScanPackageUtils.java#applyPolicy
        is_system_app && self.direct_boot_aware.unwrap_or(false)
    }

    pub fn active_permission_names(
        &self,
        platform_sdk_version: i32,
        system_features: &HashSet<String>,
    ) -> Vec<String> {
        // cite: frameworks-base PackageParser.java#parseUsesPermission
        let mut names = self
            .permissions
            .iter()
            .filter(|permission| {
                permission
                    .max_sdk_version
                    .is_none_or(|maximum| maximum == 0 || maximum >= platform_sdk_version)
            })
            .filter(|permission| {
                permission
                    .required_feature
                    .as_ref()
                    .is_none_or(|feature| system_features.contains(feature))
            })
            .filter(|permission| {
                permission
                    .required_not_feature
                    .as_ref()
                    .is_none_or(|feature| !system_features.contains(feature))
            })
            .map(|permission| permission.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        names
    }

    pub fn launchable_activities(&self) -> Vec<String> {
        // cite: frameworks-base ComponentInfo.java#isEnabled
        self.activities
            .iter()
            .filter(|activity| activity.has_main_action && activity.has_launcher_category)
            .filter(|activity| activity.is_enabled())
            .map(|activity| activity.name.clone())
            .collect()
    }

    pub fn activity(&self, name: &str) -> Option<&Activity> {
        self.activities
            .iter()
            .find(|activity| activity.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UsesPermission {
    pub name: String,
    pub max_sdk_version: Option<i32>,
    pub required_feature: Option<String>,
    pub required_not_feature: Option<String>,
}

impl fmt::Display for UsesPermission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut constraints = Vec::new();
        if let Some(maximum) = self.max_sdk_version {
            constraints.push(format!("maxSdkVersion={maximum}"));
        }
        if let Some(feature) = self.required_feature.as_deref() {
            constraints.push(format!("requiredFeature={feature}"));
        }
        if let Some(feature) = self.required_not_feature.as_deref() {
            constraints.push(format!("requiredNotFeature={feature}"));
        }
        if constraints.is_empty() {
            formatter.write_str(&self.name)
        } else {
            write!(formatter, "{} ({})", self.name, constraints.join(", "))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    pub name: String,
    pub exported: Option<bool>,
    pub enabled: Option<bool>,
    pub is_alias: bool,
    pub target_activity: Option<String>,
    pub has_main_action: bool,
    pub has_launcher_category: bool,
    pub label: Option<u32>,
    pub theme: Option<u32>,
}

impl Activity {
    pub fn is_enabled(&self) -> bool {
        // cite: frameworks-base PackageParser.java#enabledRes
        self.enabled.unwrap_or(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub name: String,
    pub authority: String,
    pub exported: Option<bool>,
    pub enabled: Option<bool>,
}

impl Provider {
    pub fn is_enabled(&self) -> bool {
        // cite: frameworks-base PackageParser.java#enabledRes
        self.enabled.unwrap_or(true)
    }
}

#[derive(Debug, Clone)]
struct ActivityBuilder {
    name: String,
    exported: Option<bool>,
    enabled: Option<bool>,
    is_alias: bool,
    target_activity: Option<String>,
    has_main_action: bool,
    has_launcher_category: bool,
    label: Option<u32>,
    theme: Option<u32>,
}

impl ActivityBuilder {
    fn finish(self) -> Activity {
        Activity {
            name: self.name,
            exported: self.exported,
            enabled: self.enabled,
            is_alias: self.is_alias,
            target_activity: self.target_activity,
            has_main_action: self.has_main_action,
            has_launcher_category: self.has_launcher_category,
            label: self.label,
            theme: self.theme,
        }
    }
}

pub fn parse_manifest(data: &[u8]) -> Result<AndroidManifest> {
    let events = crate::binary_xml::parse_binary_xml(data)?;
    parse_events(&events)
}

fn parse_events(events: &[XmlEvent]) -> Result<AndroidManifest> {
    let mut manifest = AndroidManifest::default();
    let mut element_stack = Vec::new();
    let mut current_activity = None;
    let mut in_intent_filter = false;
    let mut application_seen = false;
    let mut application_is_primary = false;

    for event in events {
        match event {
            XmlEvent::Start(element) => {
                handle_start_element(
                    &mut manifest,
                    &mut current_activity,
                    &mut in_intent_filter,
                    &mut application_seen,
                    &mut application_is_primary,
                    &element_stack,
                    element,
                )?;

                element_stack.push((element.name.clone(), element.namespace.clone()));
            }
            XmlEvent::End(element) => {
                if element.name == "intent-filter" && path_ends_in_intent_filter(&element_stack) {
                    in_intent_filter = false;
                }

                if is_activity_element(&element.name)
                    && (path_is(&element_stack, &["manifest", "application", "activity"])
                        || path_is(
                            &element_stack,
                            &["manifest", "application", "activity-alias"],
                        ))
                {
                    if let Some(activity) = current_activity.take() {
                        manifest.activities.push(activity.finish());
                    }
                }

                if element.name == "application"
                    && path_is(&element_stack, &["manifest", "application"])
                {
                    application_is_primary = false;
                }

                pop_element(
                    &mut element_stack,
                    &element.name,
                    element.namespace.as_deref(),
                );
            }
        }
    }

    manifest.permissions.sort();
    manifest.permissions.dedup();
    manifest
        .activities
        .sort_by(|left, right| left.name.cmp(&right.name));
    manifest.effective_target_sdk_version()?;

    Ok(manifest)
}

fn handle_start_element(
    manifest: &mut AndroidManifest,
    current_activity: &mut Option<ActivityBuilder>,
    in_intent_filter: &mut bool,
    application_seen: &mut bool,
    application_is_primary: &mut bool,
    element_stack: &[(String, Option<String>)],
    element: &XmlStartElement,
) -> Result<()> {
    // cite: frameworks-base PackageParser.java#parseBaseApkCommon, parseBaseApplication, parseUsesPermission
    match element.name.as_str() {
        "manifest" if element_stack.is_empty() => read_manifest_attributes(manifest, element),
        "application" if path_is(element_stack, &["manifest"]) => {
            if *application_seen {
                *application_is_primary = false;
                return Ok(());
            }
            *application_seen = true;
            *application_is_primary = true;
            read_application(manifest, element)?
        }
        "uses-sdk" if path_is(element_stack, &["manifest"]) => {
            read_sdk_attributes(manifest, element)?
        }
        "uses-permission" | "uses-permission-sdk-23" | "uses-permission-sdk-m"
            if path_is(element_stack, &["manifest"]) =>
        {
            read_permission(manifest, element)?
        }
        "activity" | "activity-alias"
            if *application_is_primary && path_is(element_stack, &["manifest", "application"]) =>
        {
            *current_activity = read_activity(manifest, element);
        }
        "provider"
            if *application_is_primary && path_is(element_stack, &["manifest", "application"]) =>
        {
            read_provider(manifest, element)?;
        }
        "intent-filter"
            if current_activity.is_some()
                && (path_is(element_stack, &["manifest", "application", "activity"])
                    || path_is(
                        element_stack,
                        &["manifest", "application", "activity-alias"],
                    )) =>
        {
            *in_intent_filter = true;
        }
        "action" if *in_intent_filter && path_ends_in_intent_filter(element_stack) => {
            read_action(current_activity, element);
        }
        "category" if *in_intent_filter && path_ends_in_intent_filter(element_stack) => {
            read_category(current_activity, element);
        }
        _ => {}
    }

    Ok(())
}

fn read_manifest_attributes(manifest: &mut AndroidManifest, element: &XmlStartElement) {
    manifest.package_name = unnamespaced_text_attribute(element, "package");
    manifest.version_code = text_attribute(element, "versionCode");
    manifest.version_name = text_attribute(element, "versionName");
    manifest.compile_sdk_version = text_attribute(element, "compileSdkVersion");
}

fn read_application(manifest: &mut AndroidManifest, element: &XmlStartElement) -> Result<()> {
    manifest.application_label = reference_attribute(element, "label");
    manifest.application_theme = reference_attribute(element, "theme");
    // cite: frameworks-base PackageParser.java#parseBaseApplication, AndroidManifestApplication_requestLegacyExternalStorage
    manifest.request_legacy_external_storage =
        strict_bool_attribute(element, "requestLegacyExternalStorage")?;
    // cite: frameworks-base PackageParser.java#parseBaseApplication, AndroidManifestApplication_defaultToDeviceProtectedStorage, AndroidManifestApplication_directBootAware
    manifest.default_to_device_protected_storage =
        strict_bool_attribute(element, "defaultToDeviceProtectedStorage")?;
    manifest.direct_boot_aware = strict_bool_attribute(element, "directBootAware")?;

    let Some(name) = text_attribute(element, "name") else {
        return Ok(());
    };

    manifest.application_name = Some(resolve_component_name(
        manifest.package_name.as_deref(),
        &name,
    ));

    Ok(())
}

fn read_sdk_attributes(manifest: &mut AndroidManifest, element: &XmlStartElement) -> Result<()> {
    manifest.uses_sdk_declared = true;
    manifest.min_sdk_version = sdk_version_attribute(element, "minSdkVersion")?;
    manifest.target_sdk_version = sdk_version_attribute(element, "targetSdkVersion")?;

    Ok(())
}

fn read_permission(manifest: &mut AndroidManifest, element: &XmlStartElement) -> Result<()> {
    // cite: frameworks-base PackageParser.java#parseUsesPermission
    if let Some(name) = non_resource_string_attribute(element, "name") {
        manifest.permissions.push(UsesPermission {
            name,
            max_sdk_version: strict_integer_attribute(element, "maxSdkVersion")?,
            required_feature: strict_string_attribute(element, "requiredFeature")?,
            required_not_feature: strict_string_attribute(element, "requiredNotFeature")?,
        });
    }

    Ok(())
}

fn parse_release_sdk_version(attribute: &str, value: &str) -> Result<i32> {
    match value.parse::<i32>() {
        Ok(version) if version >= 0 => Ok(version),
        Ok(_) => Err(WinedroidError::new(format!(
            "manifest {attribute} `{value}` is negative"
        ))),
        Err(_) if value.chars().all(|character| character.is_ascii_digit()) => {
            Err(WinedroidError::new(format!(
                "manifest {attribute} `{value}` is outside the supported integer range"
            )))
        }
        Err(_) => Err(WinedroidError::new(format!(
            "manifest {attribute} `{value}` requires an unsupported development platform codename on this release build"
        ))),
    }
}

fn sdk_version_attribute(element: &XmlStartElement, name: &str) -> Result<Option<String>> {
    // cite: frameworks-base PackageParser.java#parseBaseApkCommon
    let Some(attribute) = android_attribute(element, name) else {
        return Ok(None);
    };
    match &attribute.value {
        XmlValue::Integer(value) => Ok(Some(value.to_string())),
        XmlValue::Hex(value) | XmlValue::Color(value) => Ok(Some((*value as i32).to_string())),
        XmlValue::Boolean(value) => Ok(Some(i32::from(*value).to_string())),
        XmlValue::String(value) => Err(WinedroidError::new(format!(
            "manifest {name} `{value}` requires an unsupported development platform codename on this release build"
        ))),
        value => Err(unsupported_attribute_value(name, value)),
    }
}

fn read_activity(manifest: &AndroidManifest, element: &XmlStartElement) -> Option<ActivityBuilder> {
    let name = text_attribute(element, "name")?;
    let resolved_name = resolve_component_name(manifest.package_name.as_deref(), &name);
    let is_alias = element.name == "activity-alias";

    // cite: frameworks-base PackageParser.java#parseActivityAlias
    let target_activity = if is_alias {
        text_attribute(element, "targetActivity")
            .map(|target| resolve_component_name(manifest.package_name.as_deref(), &target))
    } else {
        None
    };

    Some(ActivityBuilder {
        name: resolved_name,
        exported: bool_attribute(element, "exported"),
        enabled: bool_attribute(element, "enabled"),
        is_alias,
        target_activity,
        has_main_action: false,
        has_launcher_category: false,
        label: reference_attribute(element, "label"),
        theme: reference_attribute(element, "theme"),
    })
}

fn read_provider(manifest: &mut AndroidManifest, element: &XmlStartElement) -> Result<()> {
    // cite: frameworks-base PackageParser.java#parseProvider
    let Some(name) = text_attribute(element, "name") else {
        return Err(WinedroidError::new(
            "manifest <provider> does not include a name attribute",
        ));
    };
    let Some(authority) = non_resource_string_attribute(element, "authorities") else {
        return Err(WinedroidError::new(
            "manifest <provider> does not include authorities attribute",
        ));
    };
    if authority.is_empty() {
        return Err(WinedroidError::new(
            "manifest <provider> has empty authorities attribute",
        ));
    }
    manifest.providers.push(Provider {
        name: resolve_component_name(manifest.package_name.as_deref(), &name),
        authority,
        exported: strict_bool_attribute(element, "exported")?,
        enabled: strict_bool_attribute(element, "enabled")?,
    });

    Ok(())
}

fn reference_attribute(element: &XmlStartElement, name: &str) -> Option<u32> {
    match android_attribute(element, name)?.value {
        XmlValue::Reference(value) | XmlValue::Hex(value) => Some(value),
        _ => None,
    }
}

fn read_action(current_activity: &mut Option<ActivityBuilder>, element: &XmlStartElement) {
    let Some(action_name) = text_attribute(element, "name") else {
        return;
    };

    if action_name == ACTION_MAIN {
        if let Some(activity) = current_activity {
            activity.has_main_action = true;
        }
    }
}

fn read_category(current_activity: &mut Option<ActivityBuilder>, element: &XmlStartElement) {
    let Some(category_name) = text_attribute(element, "name") else {
        return;
    };

    if category_name == CATEGORY_LAUNCHER {
        if let Some(activity) = current_activity {
            activity.has_launcher_category = true;
        }
    }
}

fn resolve_component_name(package_name: Option<&str>, component_name: &str) -> String {
    let Some(package_name) = package_name else {
        return component_name.to_owned();
    };

    if component_name.starts_with('.') {
        return format!("{package_name}{component_name}");
    }

    if component_name.contains('.') {
        return component_name.to_owned();
    }

    format!("{package_name}.{component_name}")
}

fn text_attribute(element: &XmlStartElement, name: &str) -> Option<String> {
    android_attribute(element, name).map(|attribute| attribute.value.as_text())
}

fn unnamespaced_text_attribute(element: &XmlStartElement, name: &str) -> Option<String> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name == name && attribute.namespace.is_none())
        .map(|attribute| attribute.value.as_text())
}

fn bool_attribute(element: &XmlStartElement, name: &str) -> Option<bool> {
    android_attribute(element, name).and_then(|attribute| attribute.value.as_bool())
}

fn non_resource_string_attribute(element: &XmlStartElement, name: &str) -> Option<String> {
    let attribute = android_attribute(element, name)?;
    match &attribute.value {
        XmlValue::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn strict_string_attribute(element: &XmlStartElement, name: &str) -> Result<Option<String>> {
    let Some(attribute) = android_attribute(element, name) else {
        return Ok(None);
    };
    match &attribute.value {
        XmlValue::String(value) => Ok(Some(value.clone())),
        value => Err(unsupported_attribute_value(name, value)),
    }
}

fn strict_bool_attribute(element: &XmlStartElement, name: &str) -> Result<Option<bool>> {
    let Some(attribute) = android_attribute(element, name) else {
        return Ok(None);
    };
    match &attribute.value {
        XmlValue::Boolean(value) => Ok(Some(*value)),
        XmlValue::String(value) if value == "true" => Ok(Some(true)),
        XmlValue::String(value) if value == "false" => Ok(Some(false)),
        value => Err(unsupported_attribute_value(name, value)),
    }
}

fn strict_integer_attribute(element: &XmlStartElement, name: &str) -> Result<Option<i32>> {
    // cite: frameworks-base PackageParser.java#parseUsesPermission
    let Some(attribute) = android_attribute(element, name) else {
        return Ok(None);
    };
    match attribute.value {
        XmlValue::Integer(value) => Ok(Some(value)),
        XmlValue::Hex(value) | XmlValue::Color(value) => Ok(Some(value as i32)),
        XmlValue::Boolean(value) => Ok(Some(i32::from(value))),
        XmlValue::Null
        | XmlValue::String(_)
        | XmlValue::Reference(_)
        | XmlValue::AttributeReference(_)
        | XmlValue::Float(_)
        | XmlValue::Dimension(_)
        | XmlValue::Fraction(_)
        | XmlValue::Unknown { .. } => Err(unsupported_attribute_value(name, &attribute.value)),
    }
}

fn unsupported_attribute_value(name: &str, value: &XmlValue) -> WinedroidError {
    WinedroidError::new(format!(
        "manifest android:{name} has unsupported typed value `{}`",
        value.as_text()
    ))
}

fn android_attribute<'a>(
    element: &'a XmlStartElement,
    name: &str,
) -> Option<&'a crate::binary_xml::XmlAttribute> {
    element.attributes.iter().find(|attribute| {
        attribute.name == name && attribute.namespace.as_deref() == Some(ANDROID_NAMESPACE)
    })
}

fn is_activity_element(name: &str) -> bool {
    name == "activity" || name == "activity-alias"
}

fn path_is(element_stack: &[(String, Option<String>)], expected: &[&str]) -> bool {
    element_stack.len() == expected.len()
        && element_stack
            .iter()
            .zip(expected)
            .all(|((name, _), expected)| name == expected)
}

fn path_ends_in_intent_filter(element_stack: &[(String, Option<String>)]) -> bool {
    path_is(
        element_stack,
        &["manifest", "application", "activity", "intent-filter"],
    ) || path_is(
        element_stack,
        &["manifest", "application", "activity-alias", "intent-filter"],
    )
}

fn pop_element(
    element_stack: &mut Vec<(String, Option<String>)>,
    name: &str,
    _namespace: Option<&str>,
) {
    if element_stack.last().is_some_and(|last| last.0 == name) {
        element_stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_xml::{XmlAttribute, XmlEndElement, XmlValue};

    #[test]
    fn finds_launchable_activity() {
        let events = vec![
            start(
                "manifest",
                vec![
                    manifest_attr("package", "com.example.app"),
                    text_attr("versionCode", "7"),
                ],
            ),
            start("uses-sdk", vec![integer_attr("minSdkVersion", 23)]),
            end("uses-sdk"),
            start(
                "uses-permission",
                vec![text_attr("name", "android.permission.INTERNET")],
            ),
            end("uses-permission"),
            start(
                "application",
                vec![text_attr("name", ".ExampleApplication")],
            ),
            start(
                "activity",
                vec![
                    text_attr("name", ".MainActivity"),
                    bool_attr("exported", true),
                ],
            ),
            start("intent-filter", Vec::new()),
            start("action", vec![text_attr("name", ACTION_MAIN)]),
            end("action"),
            start("category", vec![text_attr("name", CATEGORY_LAUNCHER)]),
            end("category"),
            end("intent-filter"),
            end("activity"),
            end("application"),
            end("manifest"),
        ];

        let manifest = parse_events(&events).expect("manifest should parse");

        assert_eq!(manifest.package_name.as_deref(), Some("com.example.app"));
        assert_eq!(
            manifest.application_name.as_deref(),
            Some("com.example.app.ExampleApplication")
        );
        assert_eq!(manifest.version_code.as_deref(), Some("7"));
        assert_eq!(manifest.min_sdk_version.as_deref(), Some("23"));
        assert_eq!(
            manifest.permissions,
            vec![UsesPermission {
                name: "android.permission.INTERNET".to_owned(),
                max_sdk_version: None,
                required_feature: None,
                required_not_feature: None,
            }]
        );
        assert_eq!(
            manifest.launchable_activities(),
            vec!["com.example.app.MainActivity".to_owned()]
        );
        assert_eq!(manifest.activities[0].exported, Some(true));
    }

    #[test]
    fn launcher_query_excludes_manifest_disabled_components() {
        // cite: frameworks-base ComponentInfo.java#isEnabled
        // cite: frameworks-base PackageParser.java#enabledRes
        let mut events = vec![
            start(
                "manifest",
                vec![manifest_attr("package", "com.example.app")],
            ),
            start("application", Vec::new()),
            start("activity", vec![text_attr("name", ".MainActivity")]),
            end("activity"),
        ];
        events.extend(launcher_alias(".AliasRed", ".MainActivity", Some(false)));
        events.extend(launcher_alias(".AliasDefault", ".MainActivity", Some(true)));
        events.extend(launcher_alias(".AliasNoAttr", ".MainActivity", None));
        events.push(end("application"));
        events.push(end("manifest"));

        let manifest = parse_events(&events).expect("manifest should parse");

        assert_eq!(
            manifest.launchable_activities(),
            vec![
                "com.example.app.AliasDefault".to_owned(),
                "com.example.app.AliasNoAttr".to_owned(),
            ]
        );
    }

    #[test]
    fn alias_records_resolved_target_activity() {
        // cite: frameworks-base PackageParser.java#parseActivityAlias
        let mut events = vec![
            start(
                "manifest",
                vec![manifest_attr("package", "com.example.app")],
            ),
            start("application", Vec::new()),
            start("activity", vec![text_attr("name", ".MainActivity")]),
            end("activity"),
        ];
        events.extend(launcher_alias(".Alias", ".MainActivity", Some(true)));
        events.push(end("application"));
        events.push(end("manifest"));

        let manifest = parse_events(&events).expect("manifest should parse");
        let alias = manifest.activity("com.example.app.Alias").unwrap();

        assert!(alias.is_alias);
        assert_eq!(
            alias.target_activity.as_deref(),
            Some("com.example.app.MainActivity")
        );

        let target = manifest.activity("com.example.app.MainActivity").unwrap();
        assert!(!target.is_alias);
        assert_eq!(target.target_activity, None);
    }

    #[test]
    fn package_parser_target_sdk_defaults_to_zero_or_declared_min_sdk() {
        // cite: frameworks-base PackageParser.java#DEFAULT_MIN_SDK_VERSION, DEFAULT_TARGET_SDK_VERSION, parseBaseApkCommon
        let default_manifest = parse_events(&[start("manifest", Vec::new()), end("manifest")])
            .expect("default SDK manifest should parse");
        let empty_uses_sdk_manifest = parse_events(&[
            start("manifest", Vec::new()),
            start("uses-sdk", Vec::new()),
            end("uses-sdk"),
            end("manifest"),
        ])
        .expect("empty uses-sdk manifest should parse");
        let min_only_manifest = parse_events(&[
            start("manifest", Vec::new()),
            start("uses-sdk", vec![integer_attr("minSdkVersion", 23)]),
            end("uses-sdk"),
            end("manifest"),
        ])
        .expect("minimum SDK manifest should parse");
        let explicit_target_manifest = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "uses-sdk",
                vec![
                    integer_attr("minSdkVersion", 23),
                    integer_attr("targetSdkVersion", 34),
                ],
            ),
            end("uses-sdk"),
            end("manifest"),
        ])
        .expect("target SDK manifest should parse");

        assert_eq!(default_manifest.effective_target_sdk_version().unwrap(), 0);
        assert_eq!(default_manifest.effective_min_sdk_version().unwrap(), 0);
        assert_eq!(
            empty_uses_sdk_manifest
                .effective_target_sdk_version()
                .unwrap(),
            1
        );
        assert_eq!(
            empty_uses_sdk_manifest.effective_min_sdk_version().unwrap(),
            1
        );
        assert_eq!(
            min_only_manifest.effective_target_sdk_version().unwrap(),
            23
        );
        assert_eq!(min_only_manifest.effective_min_sdk_version().unwrap(), 23);
        assert_eq!(
            explicit_target_manifest
                .effective_target_sdk_version()
                .unwrap(),
            34
        );
    }

    #[test]
    fn package_parser_rejects_string_typed_sdk_values_on_release_platform() {
        // cite: frameworks-base PackageParser.java#computeTargetSdkVersion
        let codename_error = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "uses-sdk",
                vec![text_attr("targetSdkVersion", "VanillaIceCream")],
            ),
            end("uses-sdk"),
            end("manifest"),
        ])
        .expect_err("release platform should reject a development codename");
        let numeric_string_error = parse_events(&[
            start("manifest", Vec::new()),
            start("uses-sdk", vec![text_attr("targetSdkVersion", "22")]),
            end("uses-sdk"),
            end("manifest"),
        ])
        .expect_err("release platform should not reinterpret a string as an SDK integer");

        assert!(codename_error
            .to_string()
            .contains("unsupported development platform"));
        assert!(codename_error.to_string().contains("VanillaIceCream"));
        assert!(numeric_string_error.to_string().contains("22"));
    }

    #[test]
    fn package_parser_filters_requested_permissions_by_max_sdk_version() {
        // cite: frameworks-base PackageParser.java#parseUsesPermission
        let manifest = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "uses-permission",
                vec![
                    text_attr("name", "android.permission.READ_EXTERNAL_STORAGE"),
                    integer_attr("maxSdkVersion", 32),
                ],
            ),
            end("uses-permission"),
            start(
                "uses-permission",
                vec![text_attr(
                    "name",
                    "android.permission.READ_EXTERNAL_STORAGE",
                )],
            ),
            end("uses-permission"),
            start(
                "uses-permission",
                vec![
                    text_attr("name", "permission.max.zero"),
                    integer_attr("maxSdkVersion", 0),
                ],
            ),
            end("uses-permission"),
            start(
                "uses-permission",
                vec![text_attr("name", "android.permission.INTERNET")],
            ),
            end("uses-permission"),
            start(
                "uses-permission-sdk-23",
                vec![text_attr("name", "android.permission.INTERNET")],
            ),
            end("uses-permission-sdk-23"),
            end("manifest"),
        ])
        .expect("permission manifest should parse");

        assert_eq!(
            manifest.active_permission_names(32, &HashSet::new()),
            vec![
                "android.permission.INTERNET".to_owned(),
                "android.permission.READ_EXTERNAL_STORAGE".to_owned(),
                "permission.max.zero".to_owned(),
            ]
        );
        assert_eq!(
            manifest.active_permission_names(33, &HashSet::new()),
            vec![
                "android.permission.INTERNET".to_owned(),
                "android.permission.READ_EXTERNAL_STORAGE".to_owned(),
                "permission.max.zero".to_owned(),
            ]
        );
        assert_eq!(
            manifest
                .permissions
                .iter()
                .find(|permission| {
                    permission.name == "android.permission.READ_EXTERNAL_STORAGE"
                        && permission.max_sdk_version == Some(32)
                })
                .expect("capped duplicate should remain inspectable")
                .to_string(),
            "android.permission.READ_EXTERNAL_STORAGE (maxSdkVersion=32)"
        );
    }

    #[test]
    fn package_parser_requires_manifest_depth_and_android_attribute_namespace() {
        // cite: frameworks-base PackageParser.java#parseBaseApkCommon, parseBaseApplication, parseUsesPermission
        let manifest = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "application",
                vec![custom_text_attr("requestLegacyExternalStorage", "true")],
            ),
            start("uses-sdk", vec![integer_attr("targetSdkVersion", 22)]),
            end("uses-sdk"),
            start(
                "uses-permission",
                vec![text_attr(
                    "name",
                    "android.permission.READ_EXTERNAL_STORAGE",
                )],
            ),
            end("uses-permission"),
            end("application"),
            start(
                "uses-sdk",
                vec![custom_integer_attr("targetSdkVersion", 22)],
            ),
            end("uses-sdk"),
            start(
                "uses-permission",
                vec![custom_text_attr(
                    "name",
                    "android.permission.WRITE_EXTERNAL_STORAGE",
                )],
            ),
            end("uses-permission"),
            end("manifest"),
        ])
        .expect("malplaced and custom-namespace attributes should be ignored");

        assert_eq!(manifest.effective_target_sdk_version().unwrap(), 1);
        assert_eq!(manifest.request_legacy_external_storage, None);
        assert!(manifest.permissions.is_empty());
    }

    #[test]
    fn package_parser_keeps_the_first_direct_application_element() {
        // cite: frameworks-base PackageParser.java#parseBaseApkCommon
        let manifest = parse_events(&[
            start("manifest", vec![manifest_attr("package", "com.example")]),
            start(
                "application",
                vec![bool_attr("requestLegacyExternalStorage", false)],
            ),
            end("application"),
            start(
                "application",
                vec![bool_attr("requestLegacyExternalStorage", true)],
            ),
            start("activity", vec![text_attr("name", ".Ignored")]),
            end("activity"),
            end("application"),
            end("manifest"),
        ])
        .expect("later applications should be ignored");

        assert_eq!(manifest.request_legacy_external_storage, Some(false));
        assert!(manifest.activities.is_empty());
    }

    #[test]
    fn package_parser_rejects_unresolved_security_attribute_values() {
        // cite: frameworks-base PackageParser.java#parseBaseApplication, parseUsesPermission
        let max_sdk_error = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "uses-permission",
                vec![
                    text_attr("name", "android.permission.READ_EXTERNAL_STORAGE"),
                    reference_attr("maxSdkVersion", 0x7f01_0000),
                ],
            ),
            end("uses-permission"),
            end("manifest"),
        ])
        .expect_err("unresolved maxSdkVersion must not become unbounded");
        let feature_error = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "uses-permission",
                vec![
                    text_attr("name", "android.permission.READ_EXTERNAL_STORAGE"),
                    reference_attr("requiredNotFeature", 0x7f01_0001),
                ],
            ),
            end("uses-permission"),
            end("manifest"),
        ])
        .expect_err("unresolved requiredNotFeature must not activate a permission");
        let legacy_error = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "application",
                vec![reference_attr("requestLegacyExternalStorage", 0x7f01_0002)],
            ),
            end("application"),
            end("manifest"),
        ])
        .expect_err("unresolved requestLegacyExternalStorage must fail explicitly");
        let device_default_error = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "application",
                vec![reference_attr(
                    "defaultToDeviceProtectedStorage",
                    0x7f01_0003,
                )],
            ),
            end("application"),
            end("manifest"),
        ])
        .expect_err("unresolved defaultToDeviceProtectedStorage must fail explicitly");
        let direct_boot_error = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "application",
                vec![reference_attr("directBootAware", 0x7f01_0004)],
            ),
            end("application"),
            end("manifest"),
        ])
        .expect_err("unresolved directBootAware must fail explicitly");

        assert!(max_sdk_error.to_string().contains("maxSdkVersion"));
        assert!(feature_error.to_string().contains("requiredNotFeature"));
        assert!(legacy_error
            .to_string()
            .contains("requestLegacyExternalStorage"));
        assert!(device_default_error
            .to_string()
            .contains("defaultToDeviceProtectedStorage"));
        assert!(direct_boot_error.to_string().contains("directBootAware"));
    }

    #[test]
    fn package_parser_applies_permission_feature_constraints_and_sdk_m_alias() {
        // cite: frameworks-base PackageParser.java#parseBaseApkCommon, parseUsesPermission
        let manifest = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "uses-permission",
                vec![
                    text_attr("name", "permission.camera"),
                    text_attr("requiredFeature", "feature.camera"),
                ],
            ),
            end("uses-permission"),
            start(
                "uses-permission",
                vec![
                    text_attr("name", "permission.no.telephony"),
                    text_attr("requiredNotFeature", "feature.telephony"),
                ],
            ),
            end("uses-permission"),
            start(
                "uses-permission-sdk-m",
                vec![text_attr("name", "permission.sdk.m")],
            ),
            end("uses-permission-sdk-m"),
            end("manifest"),
        ])
        .expect("feature-constrained permissions should parse");
        let no_features = HashSet::new();
        let camera = HashSet::from(["feature.camera".to_owned()]);
        let telephony = HashSet::from(["feature.telephony".to_owned()]);

        assert_eq!(
            manifest.active_permission_names(35, &no_features),
            vec![
                "permission.no.telephony".to_owned(),
                "permission.sdk.m".to_owned(),
            ]
        );
        assert_eq!(
            manifest.active_permission_names(35, &camera),
            vec![
                "permission.camera".to_owned(),
                "permission.no.telephony".to_owned(),
                "permission.sdk.m".to_owned(),
            ]
        );
        assert_eq!(
            manifest.active_permission_names(35, &telephony),
            vec!["permission.sdk.m".to_owned()]
        );
    }

    #[test]
    fn scan_package_policy_keeps_ordinary_app_contexts_credential_protected() {
        // cite: frameworks-base ParsingPackageUtils.java#parseBaseAppBasicFlags
        // cite: frameworks-base ScanPackageUtils.java#applyPolicy
        let declared = parse_events(&[
            start("manifest", Vec::new()),
            start(
                "application",
                vec![
                    bool_attr("defaultToDeviceProtectedStorage", true),
                    bool_attr("directBootAware", true),
                ],
            ),
            end("application"),
            end("manifest"),
        ])
        .expect("direct-boot application attributes should parse");
        let absent = parse_events(&[
            start("manifest", Vec::new()),
            start("application", Vec::new()),
            end("application"),
            end("manifest"),
        ])
        .expect("absent direct-boot application attributes should default false");

        assert_eq!(declared.default_to_device_protected_storage, Some(true));
        assert_eq!(declared.direct_boot_aware, Some(true));
        assert!(!declared.defaults_to_device_protected_storage(false));
        assert!(!declared.is_direct_boot_aware(false));
        assert!(declared.defaults_to_device_protected_storage(true));
        assert!(declared.is_direct_boot_aware(true));
        assert!(!absent.defaults_to_device_protected_storage(true));
        assert!(!absent.is_direct_boot_aware(true));
    }

    #[test]
    fn package_parser_reads_request_legacy_external_storage_with_target_default() {
        // cite: frameworks-base PackageParser.java#parseBaseApplication, AndroidManifestApplication_requestLegacyExternalStorage
        let target_28 = parse_events(&[
            start("manifest", Vec::new()),
            start("uses-sdk", vec![integer_attr("targetSdkVersion", 28)]),
            end("uses-sdk"),
            start("application", Vec::new()),
            end("application"),
            end("manifest"),
        ])
        .expect("target 28 manifest should parse");
        let target_29 = parse_events(&[
            start("manifest", Vec::new()),
            start("uses-sdk", vec![integer_attr("targetSdkVersion", 29)]),
            end("uses-sdk"),
            start("application", Vec::new()),
            end("application"),
            end("manifest"),
        ])
        .expect("target 29 manifest should parse");
        let explicit = parse_events(&[
            start("manifest", Vec::new()),
            start("uses-sdk", vec![integer_attr("targetSdkVersion", 34)]),
            end("uses-sdk"),
            start(
                "application",
                vec![bool_attr("requestLegacyExternalStorage", true)],
            ),
            end("application"),
            end("manifest"),
        ])
        .expect("explicit legacy request manifest should parse");

        assert_eq!(target_28.request_legacy_external_storage, None);
        assert!(target_28.requests_legacy_external_storage().unwrap());
        assert!(!target_29.requests_legacy_external_storage().unwrap());
        assert_eq!(explicit.request_legacy_external_storage, Some(true));
        assert!(explicit.requests_legacy_external_storage().unwrap());
    }

    #[test]
    fn package_parser_reads_application_providers_per_parse_provider() {
        // cite: frameworks-base PackageParser.java#parseProvider
        let events = vec![
            start(
                "manifest",
                vec![manifest_attr("package", "com.example.app")],
            ),
            start("application", Vec::new()),
            start(
                "provider",
                vec![
                    text_attr("name", ".Share"),
                    text_attr("authorities", "com.example.app.share"),
                    bool_attr("exported", false),
                ],
            ),
            end("provider"),
            start(
                "provider",
                vec![
                    text_attr("name", "other.Files"),
                    text_attr("authorities", "a.b;c.d"),
                    bool_attr("enabled", false),
                ],
            ),
            end("provider"),
            end("application"),
            end("manifest"),
        ];

        let manifest = parse_events(&events).expect("manifest should parse");

        assert_eq!(
            manifest.providers,
            vec![
                Provider {
                    name: "com.example.app.Share".to_owned(),
                    authority: "com.example.app.share".to_owned(),
                    exported: Some(false),
                    enabled: None,
                },
                Provider {
                    name: "other.Files".to_owned(),
                    authority: "a.b;c.d".to_owned(),
                    exported: None,
                    enabled: Some(false),
                },
            ]
        );
        assert!(manifest.providers[0].is_enabled());
        assert!(!manifest.providers[1].is_enabled());
    }

    #[test]
    fn package_parser_rejects_provider_without_authorities() {
        // cite: frameworks-base PackageParser.java#parseProvider
        let events = vec![
            start(
                "manifest",
                vec![manifest_attr("package", "com.example.app")],
            ),
            start("application", Vec::new()),
            start("provider", vec![text_attr("name", ".Share")]),
            end("provider"),
            end("application"),
            end("manifest"),
        ];

        assert!(parse_events(&events).is_err());
    }

    fn launcher_alias(name: &str, target: &str, enabled: Option<bool>) -> Vec<XmlEvent> {
        let mut attributes = vec![text_attr("name", name), text_attr("targetActivity", target)];
        if let Some(enabled) = enabled {
            attributes.push(bool_attr("enabled", enabled));
        }

        vec![
            start("activity-alias", attributes),
            start("intent-filter", Vec::new()),
            start("action", vec![text_attr("name", ACTION_MAIN)]),
            end("action"),
            start("category", vec![text_attr("name", CATEGORY_LAUNCHER)]),
            end("category"),
            end("intent-filter"),
            end("activity-alias"),
        ]
    }

    fn start(name: &str, attributes: Vec<XmlAttribute>) -> XmlEvent {
        XmlEvent::Start(XmlStartElement {
            name: name.to_owned(),
            namespace: None,
            attributes,
        })
    }

    fn end(name: &str) -> XmlEvent {
        XmlEvent::End(XmlEndElement {
            name: name.to_owned(),
            namespace: None,
        })
    }

    fn text_attr(name: &str, value: &str) -> XmlAttribute {
        XmlAttribute {
            name: name.to_owned(),
            namespace: Some(ANDROID_NAMESPACE.to_owned()),
            value: XmlValue::String(value.to_owned()),
        }
    }

    fn manifest_attr(name: &str, value: &str) -> XmlAttribute {
        XmlAttribute {
            name: name.to_owned(),
            namespace: None,
            value: XmlValue::String(value.to_owned()),
        }
    }

    fn custom_text_attr(name: &str, value: &str) -> XmlAttribute {
        XmlAttribute {
            name: name.to_owned(),
            namespace: Some("urn:custom".to_owned()),
            value: XmlValue::String(value.to_owned()),
        }
    }

    fn custom_integer_attr(name: &str, value: i32) -> XmlAttribute {
        XmlAttribute {
            name: name.to_owned(),
            namespace: Some("urn:custom".to_owned()),
            value: XmlValue::Integer(value),
        }
    }

    fn bool_attr(name: &str, value: bool) -> XmlAttribute {
        XmlAttribute {
            name: name.to_owned(),
            namespace: Some(ANDROID_NAMESPACE.to_owned()),
            value: XmlValue::Boolean(value),
        }
    }

    fn integer_attr(name: &str, value: i32) -> XmlAttribute {
        XmlAttribute {
            name: name.to_owned(),
            namespace: Some(ANDROID_NAMESPACE.to_owned()),
            value: XmlValue::Integer(value),
        }
    }

    fn reference_attr(name: &str, value: u32) -> XmlAttribute {
        XmlAttribute {
            name: name.to_owned(),
            namespace: Some(ANDROID_NAMESPACE.to_owned()),
            value: XmlValue::Reference(value),
        }
    }
}
