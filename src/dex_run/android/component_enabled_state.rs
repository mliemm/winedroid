use super::*;
use crate::package_restrictions::{
    ComponentStates, COMPONENT_ENABLED_STATE_DEFAULT, COMPONENT_ENABLED_STATE_DISABLED,
    COMPONENT_ENABLED_STATE_DISABLED_UNTIL_USED, COMPONENT_ENABLED_STATE_DISABLED_USER,
    COMPONENT_ENABLED_STATE_ENABLED,
};

const PACKAGE_MANAGER: &str = "Landroid/content/pm/PackageManager;";
// cite: frameworks-base PackageManager.java#DONT_KILL_APP, MATCH_DISABLED_COMPONENTS, APP_DETAILS_ACTIVITY_CLASS_NAME
const DONT_KILL_APP: i32 = 0x0000_0001;
const MATCH_DISABLED_COMPONENTS: i32 = 0x0000_0200;
const APP_DETAILS_ACTIVITY_CLASS_NAME: &str = "android.app.AppDetailsActivity";
// cite: frameworks-base Build.java#JELLY_BEAN
const ANDROID_SDK_JELLY_BEAN: i32 = 16;

impl DexProgram {
    pub(in crate::dex_run) fn load_component_states(&self) -> Result<()> {
        let Some(filesystem) = self.guest_filesystem.as_ref() else {
            return Ok(());
        };
        let bytes = filesystem.read_package_restrictions().map_err(|error| {
            WinedroidError::with_source("failed to read package restrictions", error)
        })?;
        if let Some(bytes) = bytes {
            *self.component_states.borrow_mut() = ComponentStates::parse(&bytes)?;
        }

        Ok(())
    }

    fn persist_component_states(&self, states: &ComponentStates) -> Result<()> {
        let Some(filesystem) = self.guest_filesystem.as_ref() else {
            return Err(WinedroidError::new(
                "PackageManager.setComponentEnabledSetting has no app prefix to persist into\nnext missing primitive: package restrictions storage without an installed app prefix",
            ));
        };
        filesystem
            .replace_package_restrictions(&states.serialize())
            .map_err(|error| {
                WinedroidError::with_source("failed to write package restrictions", error)
            })
    }

    // cite: frameworks-base PackageUserStateUtils.java#isEnabled
    pub(in crate::dex_run) fn component_enabled(
        &self,
        class: &str,
        manifest_enabled: bool,
    ) -> bool {
        self.component_states
            .borrow()
            .is_enabled(class, manifest_enabled)
    }

    // cite: frameworks-base ComputerEngine.java#queryIntentActivitiesInternal
    // cite: frameworks-base PackageUserStateUtils.java#isEnabled
    // cite: frameworks-base PackageParser.java#parseActivityAlias
    pub(in crate::dex_run) fn launcher_under_component_states(
        &self,
        requested: &str,
    ) -> Result<String> {
        let launchers = self
            .manifest_activities
            .iter()
            .filter(|activity| activity.has_main_action && activity.has_launcher_category);
        let manifest = launchers
            .clone()
            .filter(|activity| activity.is_enabled())
            .map(|activity| activity.name.as_str())
            .collect::<Vec<_>>();
        let effective = launchers
            .filter(|activity| self.component_enabled(&activity.name, activity.is_enabled()))
            .collect::<Vec<_>>();
        let effective_names = effective
            .iter()
            .map(|activity| activity.name.as_str())
            .collect::<Vec<_>>();
        if manifest == effective_names {
            return Ok(requested.to_owned());
        }
        if let [only] = effective.as_slice() {
            let target = if only.is_alias {
                only.target_activity.as_ref()
            } else {
                Some(&only.name)
            };
            if let Some(target) = target {
                return Ok(target.clone());
            }
        }

        Err(WinedroidError::new(format!(
            "stored component enabled states leave no single launcher activity\nmanifest: {}\nwith package restrictions: {}\nnext missing primitive: launcher activity resolution through package restrictions",
            manifest.join(", "),
            effective_names.join(", ")
        )))
    }

    // cite: frameworks-base AndroidPackageUtils.java#hasComponentClassName
    fn declares_component(&self, class: &str) -> bool {
        self.manifest_activities
            .iter()
            .any(|activity| activity.name == class)
            || self
                .manifest_providers
                .iter()
                .any(|provider| provider.name == class)
            || self
                .manifest_component_classes
                .iter()
                .any(|name| name == class)
    }

    pub(in crate::dex_run) fn package_manager_component_state_intrinsic(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Result<Option<DexExecutionResult>> {
        if method.class_descriptor != PACKAGE_MANAGER {
            return Ok(None);
        }
        match (method.name.as_str(), method.proto.descriptor().as_str()) {
            ("getComponentEnabledSetting", "(Landroid/content/ComponentName;)I") => {
                Ok(self.get_component_enabled_setting(invocation))
            }
            ("setComponentEnabledSetting", "(Landroid/content/ComponentName;II)V") => {
                self.set_component_enabled_setting(invocation)
            }
            _ => Ok(None),
        }
    }

    // cite: frameworks-base ApplicationPackageManager.java#getComponentEnabledSetting
    // cite: frameworks-base ComputerEngine.java#getComponentEnabledSettingInternal
    // cite: frameworks-base Settings.java#getComponentEnabledSettingLPr
    fn get_component_enabled_setting(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [_receiver, component] = invocation.arguments.as_slice() else {
            return None;
        };
        if matches!(component, DexValue::Null | DexValue::Int(0)) {
            return Some(DexExecutionResult::Int(COMPONENT_ENABLED_STATE_DEFAULT));
        }
        let (package, class) = self.component_name_parts(component)?;
        if package != self.bootstrap_process_name() {
            return Some(self.illegal_argument(format!(
                "Unknown component: ComponentInfo{{{package}/{class}}}"
            )));
        }

        Some(DexExecutionResult::Int(
            self.component_states.borrow().state(&class),
        ))
    }

    // cite: frameworks-base ApplicationPackageManager.java#setComponentEnabledSetting
    // cite: frameworks-base PackageManagerService.java#setComponentEnabledSetting, setEnabledSettings, setEnabledSettingInternalLocked
    // cite: frameworks-base PackageManager.java#ComponentEnabledSetting
    fn set_component_enabled_setting(
        &self,
        invocation: &DexInvocation,
    ) -> Result<Option<DexExecutionResult>> {
        let [_receiver, component, DexValue::Int(new_state), DexValue::Int(flags)] =
            invocation.arguments.as_slice()
        else {
            return Ok(None);
        };
        if matches!(component, DexValue::Null | DexValue::Int(0)) {
            return Ok(Some(self.null_pointer_exception_result()));
        }
        let Some((package, class)) = self.component_name_parts(component) else {
            return Ok(None);
        };
        if !matches!(
            *new_state,
            COMPONENT_ENABLED_STATE_DEFAULT
                | COMPONENT_ENABLED_STATE_ENABLED
                | COMPONENT_ENABLED_STATE_DISABLED
                | COMPONENT_ENABLED_STATE_DISABLED_USER
                | COMPONENT_ENABLED_STATE_DISABLED_UNTIL_USED
        ) {
            return Ok(Some(self.illegal_argument(format!(
                "Invalid new component state: {new_state}"
            ))));
        }
        if package != self.bootstrap_process_name() {
            return Ok(Some(self.security_exception(format!(
                "Attempt to change component state; pid={BOOTSTRAP_PROCESS_ID}, uid={BOOTSTRAP_PROCESS_UID}, component=ComponentInfo{{{package}/{class}}}"
            ))));
        }
        if class == APP_DETAILS_ACTIVITY_CLASS_NAME {
            return Ok(Some(self.security_exception(
                "Cannot disable a system-generated component".to_owned(),
            )));
        }
        if !self.declares_component(&class) {
            if self.target_sdk_version >= ANDROID_SDK_JELLY_BEAN {
                return Ok(Some(self.illegal_argument(format!(
                    "Component class {class} does not exist in {package}"
                ))));
            }
            return Ok(Some(DexExecutionResult::Void));
        }

        let mut states = self.component_states.borrow().clone();
        if !states.set(&class, *new_state) {
            return Ok(Some(DexExecutionResult::Void));
        }
        if flags & DONT_KILL_APP == 0 {
            return Err(WinedroidError::new(format!(
                "PackageManager.setComponentEnabledSetting changed {class} without DONT_KILL_APP, so ActivityManagerService kills the app process\nnext missing primitive: package state change process kill"
            )));
        }
        if states.state(&class) == COMPONENT_ENABLED_STATE_DISABLED
            && self.component_is_live(&class)
        {
            return Err(WinedroidError::new(format!(
                "PackageManager.setComponentEnabledSetting disabled running component {class}\nnext missing primitive: ActivityManagerService cleanupDisabledPackageComponentsLocked"
            )));
        }
        self.persist_component_states(&states)?;
        *self.component_states.borrow_mut() = states;

        Ok(Some(DexExecutionResult::Void))
    }

    // cite: frameworks-base ActivityManagerService.java#cleanupDisabledPackageComponentsLocked
    fn component_is_live(&self, class: &str) -> bool {
        let descriptor = normalize_class_name(class);
        let runtime = self.runtime.borrow();
        runtime
            .activity_stack
            .iter()
            .any(|activity| activity.class_descriptor == descriptor)
            || runtime
                .content_providers
                .values()
                .any(|provider| value_class_descriptor(provider) == Some(descriptor.as_str()))
    }

    // cite: frameworks-base ComputerEngine.java#getActivityInfoInternalBody
    // cite: frameworks-base PackageUserStateUtils.java#isEnabled
    pub(in crate::dex_run) fn activity_resolvable(&self, class: &str, flags: i32) -> Option<bool> {
        let activity = self
            .manifest_activities
            .iter()
            .find(|activity| activity.name == class)?;
        Some(
            flags & MATCH_DISABLED_COMPONENTS != 0
                || self.component_enabled(class, activity.is_enabled()),
        )
    }

    // cite: frameworks-base Instrumentation.java#checkStartActivityResult
    // cite: frameworks-base ComponentName.java#toShortString
    pub(in crate::dex_run) fn disabled_explicit_activity_exception(
        &self,
        target_descriptor: &str,
    ) -> Option<DexValue> {
        let class = descriptor_to_java_name(target_descriptor)?;
        if self.activity_resolvable(&class, 0)? {
            return None;
        }
        let package = self.bootstrap_process_name();
        Some(self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: "Landroid/content/ActivityNotFoundException;".to_owned(),
            detail_message: Some(format!(
                "Unable to find explicit activity class {{{package}/{class}}}; have you declared this activity in your AndroidManifest.xml, or does your intent not match its declared <intent-filter>?"
            )),
        }))
    }

    fn illegal_argument(&self, message: String) -> DexExecutionResult {
        DexExecutionResult::Throw(self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
            detail_message: Some(message),
        }))
    }

    fn security_exception(&self, message: String) -> DexExecutionResult {
        DexExecutionResult::Throw(self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: "Ljava/lang/SecurityException;".to_owned(),
            detail_message: Some(message),
        }))
    }
}
