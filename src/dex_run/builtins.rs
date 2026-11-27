use super::*;

mod activity_window;
mod android_content_resources;
mod android_content_storage;
mod android_layout_menu;
mod android_net;
mod android_platform_misc;
mod android_preferences;
mod android_text_image;
mod android_view_core;
mod android_widget_adapters;
mod androidx_runtime_widget;
mod concurrency_json_androidx;
mod content_resource_storage;
mod java_box_locale;
mod java_collection;
mod java_concurrency_atomic;
mod java_core_runtime;
mod java_core_text;
mod java_date;
mod java_io;
mod java_json_collection_extra;
mod java_locale;
#[cfg(test)]
pub(in crate::dex_run) use java_locale::java_locale_parts;
pub(in crate::dex_run) use java_locale::locale_language_tag;
mod java_math;
mod java_math_regex_reflection_io;
mod java_native_library;
mod java_net;
pub(in crate::dex_run) mod java_nio;
mod java_number_format;
mod java_properties;
mod java_reflection;
mod java_regex_util;
mod java_string_text;
pub(in crate::dex_run) mod java_zip;
mod org_xml_sax;
mod third_party_runtime;
mod view_widget;

impl DexMethodId {
    fn matches_builtin_signature(
        &self,
        class_descriptor: &str,
        name: &str,
        proto_descriptor: &str,
    ) -> bool {
        self.name == name
            && self.class_descriptor == class_descriptor
            && self.proto.descriptor_is(proto_descriptor)
    }

    fn matches_any_builtin_signature(
        &self,
        class_descriptor: &str,
        name: &str,
        proto_descriptors: &[&str],
    ) -> bool {
        if self.class_descriptor != class_descriptor || self.name != name {
            return false;
        }

        proto_descriptors
            .iter()
            .any(|descriptor| self.proto.descriptor_is(descriptor))
    }

    fn matches_builtin_member(&self, name: &str, proto_descriptor: &str) -> bool {
        self.name == name && self.proto.descriptor_is(proto_descriptor)
    }
}

impl DexProgram {
    // cite: art interpreter_common.h#DoInvoke, FindMethodToCall
    pub(super) fn invoke_builtin_method_from_class_hierarchy(
        &self,
        class_descriptor: &str,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Result<Option<DexExecutionResult>> {
        let mut current = Some(class_descriptor.to_owned());
        let mut visited = HashSet::new();

        while let Some(class_descriptor) = current {
            if !visited.insert(class_descriptor.clone()) {
                return Err(WinedroidError::new(format!(
                    "builtin method receiver hierarchy contains a cycle\nclass: {class_descriptor}\nmethod: {}",
                    method.display_signature()
                )));
            }

            if is_bootstrap_class_descriptor(&class_descriptor) {
                let candidate = DexMethodId {
                    class_descriptor: class_descriptor.clone(),
                    name: method.name.clone(),
                    proto: method.proto.clone(),
                };
                if let Some(result) = self.invoke_builtin_method(&candidate, Some(invocation)) {
                    return Ok(Some(result));
                }
            }
            current = self.superclass_descriptor(&class_descriptor);
        }

        Ok(None)
    }

    pub(super) fn invoke_builtin_method_in_hierarchy(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let mut current = self.superclass_descriptor(&method.class_descriptor);
        while let Some(class_descriptor) = current {
            let candidate = DexMethodId {
                class_descriptor: class_descriptor.clone(),
                name: method.name.clone(),
                proto: method.proto.clone(),
            };
            if let Some(result) = self.invoke_builtin_method(&candidate, Some(invocation)) {
                return Some(result);
            }
            current = self.superclass_descriptor(&class_descriptor);
        }
        None
    }

    const BUILTIN_FAMILIES: [fn(
        &DexProgram,
        &DexMethodId,
        Option<&DexInvocation>,
    ) -> Option<DexExecutionResult>; 13] = [
        DexProgram::invoke_android_activity_window_builtin,
        DexProgram::invoke_android_preferences_builtin,
        DexProgram::invoke_android_view_widget_builtin,
        DexProgram::invoke_android_content_resource_storage_builtin,
        DexProgram::invoke_java_collection_builtin,
        DexProgram::invoke_java_math_regex_reflection_io_builtin,
        DexProgram::invoke_java_number_format_builtin,
        DexProgram::invoke_org_xml_sax_builtin,
        DexProgram::invoke_java_date_builtin,
        DexProgram::invoke_android_net_builtin,
        DexProgram::invoke_java_net_builtin,
        DexProgram::invoke_concurrency_json_androidx_builtin,
        DexProgram::invoke_java_core_text_builtin,
    ];

    pub(super) fn invoke_builtin_method(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        self.invoke_builtin_method_hinted(None, method, invocation)
    }

    pub(super) fn invoke_builtin_method_hinted(
        &self,
        hint_key: Option<usize>,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == STRING_BUFFER {
            return self.invoke_builtin_method_hinted(
                hint_key,
                &string_buffer_as_string_builder(method),
                invocation,
            );
        }
        let method_metric_start = crate::method_metrics::start_timing();
        if let Some(key) = hint_key {
            let hint = self.caches.borrow().builtin_family_hints.get(&key).copied();
            if let Some(family) = hint {
                if let Some(result) =
                    Self::BUILTIN_FAMILIES[family as usize](self, method, invocation)
                {
                    crate::method_metrics::record_builtin_call_elapsed(method_metric_start);
                    return Some(result);
                }
            }
        }
        for (index, family) in Self::BUILTIN_FAMILIES.iter().enumerate() {
            if let Some(result) = family(self, method, invocation) {
                if let Some(key) = hint_key {
                    self.caches
                        .borrow_mut()
                        .builtin_family_hints
                        .insert(key, index as u8);
                }
                crate::method_metrics::record_builtin_call_elapsed(method_metric_start);
                return Some(result);
            }
        }
        None
    }
}

const STRING_BUFFER: &str = "Ljava/lang/StringBuffer;";

// cite: libcore StringBuffer.java#StringBuffer, append, toString
// cite: libcore AbstractStringBuilder.java#append
pub(super) fn string_buffer_as_string_builder(method: &DexMethodId) -> DexMethodId {
    let builder = |descriptor: &String| {
        if descriptor == STRING_BUFFER {
            "Ljava/lang/StringBuilder;".to_owned()
        } else {
            descriptor.clone()
        }
    };
    DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: method.name.clone(),
        proto: crate::dex::DexProto {
            shorty: method.proto.shorty.clone(),
            return_type: builder(&method.proto.return_type),
            parameters: method.proto.parameters.iter().map(builder).collect(),
        },
    }
}

impl DexProgram {
    // cite: art interpreter_common.h#DoInvoke, FindMethodToCall
    pub(super) fn guest_overrides(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> bool {
        let Some(invocation) = invocation else {
            return false;
        };
        let Some(receiver @ DexValue::Object(_)) = invocation.arguments.first() else {
            return false;
        };
        if !matches!(
            invocation.kind,
            crate::dex_interpreter::DexInvokeKind::Virtual
                | crate::dex_interpreter::DexInvokeKind::Interface
        ) {
            return false;
        }
        let Some(class_descriptor) = value_class_descriptor(receiver) else {
            return false;
        };
        let receiver_method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            ..method.clone()
        };
        self.resolve_exact_method_in_class_hierarchy(class_descriptor, &receiver_method)
            .is_ok_and(|resolved| resolved.is_some())
    }
}
