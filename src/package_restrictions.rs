use std::collections::BTreeSet;

use crate::error::{Result, WinedroidError};

// cite: frameworks-base PackageManager.java#COMPONENT_ENABLED_STATE_DEFAULT, COMPONENT_ENABLED_STATE_ENABLED, COMPONENT_ENABLED_STATE_DISABLED, COMPONENT_ENABLED_STATE_DISABLED_USER, COMPONENT_ENABLED_STATE_DISABLED_UNTIL_USED
pub const COMPONENT_ENABLED_STATE_DEFAULT: i32 = 0;
pub const COMPONENT_ENABLED_STATE_ENABLED: i32 = 1;
pub const COMPONENT_ENABLED_STATE_DISABLED: i32 = 2;
pub const COMPONENT_ENABLED_STATE_DISABLED_USER: i32 = 3;
pub const COMPONENT_ENABLED_STATE_DISABLED_UNTIL_USED: i32 = 4;

// cite: frameworks-base Settings.java#TAG_ENABLED_COMPONENTS, TAG_DISABLED_COMPONENTS
const ENABLED_TAG: &str = "enabled-components";
const DISABLED_TAG: &str = "disabled-components";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentStates {
    enabled: BTreeSet<String>,
    disabled: BTreeSet<String>,
}

impl ComponentStates {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| WinedroidError::new("package restrictions file is not UTF-8"))?;
        let mut states = Self::default();
        for line in text.lines().filter(|line| !line.is_empty()) {
            match line.split_once(' ') {
                Some((ENABLED_TAG, class)) => {
                    states.enabled.insert(class.to_owned());
                }
                Some((DISABLED_TAG, class)) => {
                    states.disabled.insert(class.to_owned());
                }
                _ => {
                    return Err(WinedroidError::new(format!(
                        "package restrictions file has an unknown line `{line}`"
                    )))
                }
            }
        }

        Ok(states)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut text = String::new();
        for (tag, classes) in [(ENABLED_TAG, &self.enabled), (DISABLED_TAG, &self.disabled)] {
            for class in classes {
                text.push_str(tag);
                text.push(' ');
                text.push_str(class);
                text.push('\n');
            }
        }
        text.into_bytes()
    }

    // cite: frameworks-base PackageSetting.java#getCurrentEnabledStateLPr
    pub fn state(&self, class: &str) -> i32 {
        if self.enabled.contains(class) {
            COMPONENT_ENABLED_STATE_ENABLED
        } else if self.disabled.contains(class) {
            COMPONENT_ENABLED_STATE_DISABLED
        } else {
            COMPONENT_ENABLED_STATE_DEFAULT
        }
    }

    // cite: frameworks-base PackageManagerService.java#setEnabledSettingInternalLocked
    // cite: frameworks-base PackageSetting.java#enableComponentLPw, disableComponentLPw, restoreComponentLPw
    pub fn set(&mut self, class: &str, state: i32) -> bool {
        match state {
            COMPONENT_ENABLED_STATE_ENABLED => {
                let removed = self.disabled.remove(class);
                self.enabled.insert(class.to_owned()) | removed
            }
            COMPONENT_ENABLED_STATE_DISABLED => {
                let removed = self.enabled.remove(class);
                self.disabled.insert(class.to_owned()) | removed
            }
            COMPONENT_ENABLED_STATE_DEFAULT => {
                let removed = self.disabled.remove(class);
                self.enabled.remove(class) | removed
            }
            _ => false,
        }
    }

    // cite: frameworks-base PackageUserStateUtils.java#isEnabled
    pub fn is_enabled(&self, class: &str, manifest_enabled: bool) -> bool {
        match self.state(class) {
            COMPONENT_ENABLED_STATE_ENABLED => true,
            COMPONENT_ENABLED_STATE_DISABLED => false,
            _ => manifest_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_current_enabled_state_reports_default_until_a_set_lands() {
        // cite: frameworks-base PackageSetting.java#getCurrentEnabledStateLPr
        let mut states = ComponentStates::default();
        assert_eq!(states.state("a.B"), COMPONENT_ENABLED_STATE_DEFAULT);
        assert!(states.set("a.B", COMPONENT_ENABLED_STATE_ENABLED));
        assert_eq!(states.state("a.B"), COMPONENT_ENABLED_STATE_ENABLED);
        assert!(states.set("a.B", COMPONENT_ENABLED_STATE_DISABLED));
        assert_eq!(states.state("a.B"), COMPONENT_ENABLED_STATE_DISABLED);
        assert!(states.set("a.B", COMPONENT_ENABLED_STATE_DEFAULT));
        assert_eq!(states.state("a.B"), COMPONENT_ENABLED_STATE_DEFAULT);
    }

    #[test]
    fn enable_disable_restore_report_change_only_when_the_sets_move() {
        // cite: frameworks-base PackageSetting.java#enableComponentLPw, disableComponentLPw, restoreComponentLPw
        let mut states = ComponentStates::default();
        assert!(!states.set("a.B", COMPONENT_ENABLED_STATE_DEFAULT));
        assert!(states.set("a.B", COMPONENT_ENABLED_STATE_DISABLED));
        assert!(!states.set("a.B", COMPONENT_ENABLED_STATE_DISABLED));
        assert!(states.set("a.B", COMPONENT_ENABLED_STATE_ENABLED));
        assert!(!states.set("a.B", COMPONENT_ENABLED_STATE_ENABLED));
        let mut only_enabled = ComponentStates::default();
        only_enabled.set("a.B", COMPONENT_ENABLED_STATE_ENABLED);
        assert_eq!(
            states, only_enabled,
            "enableComponentLPw removes the class from the disabled set"
        );
    }

    #[test]
    fn set_enabled_setting_internal_rejects_user_and_until_used_states_for_components() {
        // cite: frameworks-base PackageManagerService.java#setEnabledSettingInternalLocked
        let mut states = ComponentStates::default();
        assert!(!states.set("a.B", COMPONENT_ENABLED_STATE_DISABLED_USER));
        assert!(!states.set("a.B", COMPONENT_ENABLED_STATE_DISABLED_UNTIL_USED));
        assert_eq!(states, ComponentStates::default());
    }

    #[test]
    fn is_enabled_lets_explicit_state_override_the_manifest_default() {
        // cite: frameworks-base PackageUserStateUtils.java#isEnabled
        let mut states = ComponentStates::default();
        assert!(!states.is_enabled("a.B", false));
        assert!(states.is_enabled("a.B", true));
        states.set("a.B", COMPONENT_ENABLED_STATE_ENABLED);
        assert!(states.is_enabled("a.B", false));
        states.set("a.B", COMPONENT_ENABLED_STATE_DISABLED);
        assert!(!states.is_enabled("a.B", true));
    }

    #[test]
    fn serialized_states_round_trip_and_unknown_lines_fail() {
        // cite: frameworks-base Settings.java#TAG_ENABLED_COMPONENTS, TAG_DISABLED_COMPONENTS
        let mut states = ComponentStates::default();
        states.set("a.On", COMPONENT_ENABLED_STATE_ENABLED);
        states.set("a.Off", COMPONENT_ENABLED_STATE_DISABLED);
        assert_eq!(ComponentStates::parse(&states.serialize()).unwrap(), states);
        assert!(ComponentStates::parse(b"hidden a.B\n").is_err());
    }
}
