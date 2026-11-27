use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DynamicProxyClass {
    pub(super) descriptor: String,
    pub(super) interfaces: Vec<String>,
    pub(super) public: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DynamicProxyInstance {
    pub(super) class_descriptor: String,
    pub(super) handler: DexValue,
}

// cite: libcore Proxy.java#newProxyInstance, getProxyClass0, ProxyClassFactory
impl DexProgram {
    pub(super) fn new_dynamic_proxy_instance(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [loader, interfaces, handler] = invocation.arguments.as_slice() else {
            return None;
        };

        if matches!(handler, DexValue::Null | DexValue::Int(0)) {
            return Some(DexExecutionResult::Throw(
                self.proxy_exception("Ljava/lang/NullPointerException;", None),
            ));
        }

        let interface_values = match interfaces {
            DexValue::Null | DexValue::Int(0) => {
                return Some(DexExecutionResult::Throw(
                    self.proxy_exception("Ljava/lang/NullPointerException;", None),
                ));
            }
            DexValue::Object(_) | DexValue::Reference(_) => {
                self.reflect_method_argument_values(interfaces)?
            }
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        let mut descriptors = Vec::with_capacity(interface_values.len());
        for value in &interface_values {
            let Some(descriptor) = class_reference_descriptor_from_value(value) else {
                if matches!(value, DexValue::Null | DexValue::Int(0)) {
                    return Some(DexExecutionResult::Throw(
                        self.proxy_exception("Ljava/lang/NullPointerException;", None),
                    ));
                }

                return None;
            };

            descriptors.push(descriptor.to_owned());
        }

        if let Some(exception) = self.validate_dynamic_proxy_interfaces(&descriptors) {
            return Some(DexExecutionResult::Throw(exception));
        }

        let class = self.ensure_dynamic_proxy_class(loader, &descriptors);
        let proxy = self.allocate_object(class.descriptor.clone());
        let DexValue::Object(object) = &proxy else {
            return None;
        };
        self.runtime.borrow_mut().proxy_instances.insert(
            object.id,
            DynamicProxyInstance {
                class_descriptor: class.descriptor,
                handler: handler.clone(),
            },
        );

        Some(DexExecutionResult::Value(proxy))
    }

    fn validate_dynamic_proxy_interfaces(&self, descriptors: &[String]) -> Option<DexValue> {
        let mut seen = Vec::with_capacity(descriptors.len());

        for descriptor in descriptors {
            let is_interface = self
                .load_class(descriptor)
                .map(|loaded_class| loaded_class.class_def.access_flags & JAVA_ACC_INTERFACE != 0)
                .unwrap_or_else(|| is_bootstrap_interface_descriptor(descriptor));
            if !is_interface {
                let name = descriptor_to_java_name(descriptor)?;
                return Some(self.proxy_exception(
                    "Ljava/lang/IllegalArgumentException;",
                    Some(format!("{name} is not an interface")),
                ));
            }

            if seen.contains(descriptor) {
                let name = descriptor_to_java_name(descriptor)?;
                return Some(self.proxy_exception(
                    "Ljava/lang/IllegalArgumentException;",
                    Some(format!("repeated interface: {name}")),
                ));
            }
            seen.push(descriptor.clone());
        }

        let mut proxy_package: Option<&str> = None;
        for descriptor in descriptors {
            let modifiers = self.dynamic_proxy_interface_modifiers(descriptor);
            if modifiers & JAVA_ACC_PUBLIC != 0 {
                continue;
            }

            let package = descriptor_package_prefix(descriptor);
            match proxy_package {
                None => proxy_package = Some(package),
                Some(existing) if existing != package => {
                    return Some(self.proxy_exception(
                        "Ljava/lang/IllegalArgumentException;",
                        Some("non-public interfaces from different packages".to_owned()),
                    ));
                }
                Some(_) => {}
            }
        }

        None
    }

    fn ensure_dynamic_proxy_class(
        &self,
        loader: &DexValue,
        interfaces: &[String],
    ) -> DynamicProxyClass {
        let class_key = format!("{}|{}", map_identity_key(loader), interfaces.join("\u{0}"));
        let cached = self
            .runtime
            .borrow()
            .proxy_class_keys
            .get(&class_key)
            .and_then(|descriptor| self.runtime.borrow().proxy_classes.get(descriptor).cloned());
        if let Some(class) = cached {
            return class;
        }

        let proxy_package = interfaces
            .iter()
            .find(|descriptor| {
                self.dynamic_proxy_interface_modifiers(descriptor) & JAVA_ACC_PUBLIC == 0
            })
            .map(|descriptor| descriptor_package_prefix(descriptor))
            .unwrap_or("");
        let index = {
            let mut runtime = self.runtime.borrow_mut();
            let index = runtime.next_proxy_index;
            runtime.next_proxy_index += 1;
            index
        };
        let name = format!("$Proxy{index}");
        let descriptor = if proxy_package.is_empty() {
            format!("L{name};")
        } else {
            format!("L{proxy_package}/{name};")
        };
        let class = DynamicProxyClass {
            descriptor,
            interfaces: interfaces.to_vec(),
            public: proxy_package.is_empty(),
        };

        let mut runtime = self.runtime.borrow_mut();
        runtime
            .proxy_class_keys
            .insert(class_key, class.descriptor.clone());
        runtime
            .proxy_classes
            .insert(class.descriptor.clone(), class.clone());

        class
    }

    fn dynamic_proxy_interface_modifiers(&self, descriptor: &str) -> u32 {
        self.load_class(descriptor)
            .map(|loaded_class| loaded_class.class_def.access_flags)
            .unwrap_or_else(|| bootstrap_class_modifiers(descriptor))
    }

    pub(super) fn dynamic_proxy_class_modifiers(&self, descriptor: &str) -> Option<u32> {
        let class = self.dynamic_proxy_class(descriptor)?;
        let mut modifiers = JAVA_ACC_FINAL;
        if class.public {
            modifiers |= JAVA_ACC_PUBLIC;
        }

        Some(modifiers)
    }

    pub(super) fn dynamic_proxy_class(&self, descriptor: &str) -> Option<DynamicProxyClass> {
        self.runtime.borrow().proxy_classes.get(descriptor).cloned()
    }

    // cite: art entrypoint_utils.cc#InvokeProxyInvocationHandler
    // cite: libcore Proxy.java#invoke
    pub(super) fn invoke_dynamic_proxy_method(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        let DexValue::Object(object) = receiver else {
            return Ok(None);
        };
        let instance = self
            .runtime
            .borrow()
            .proxy_instances
            .get(&object.id)
            .cloned();
        let Some(instance) = instance else {
            return Ok(None);
        };

        if method.class_descriptor == "Ljava/lang/Object;" {
            if !matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                ("equals", "(Ljava/lang/Object;)Z")
                    | ("hashCode", "()I")
                    | ("toString", "()Ljava/lang/String;")
            ) {
                return Ok(None);
            }
        } else {
            let class = self
                .dynamic_proxy_class(&instance.class_descriptor)
                .ok_or_else(|| {
                    WinedroidError::new(format!(
                        "proxy instance has no registered class\nproxy class: {}\nnext missing primitive: java.lang.reflect.Proxy dispatch",
                        instance.class_descriptor
                    ))
                })?;
            let declared_by_interface = class.interfaces.iter().any(|interface_descriptor| {
                interface_descriptor == &method.class_descriptor
                    || self.descriptor_matches_type(interface_descriptor, &method.class_descriptor)
            });
            if !declared_by_interface {
                return Ok(None);
            }
        }

        let result = self.invoke_dynamic_proxy_handler(&instance, method, invocation, depth + 1)?;

        Ok(Some(result))
    }

    fn invoke_dynamic_proxy_handler(
        &self,
        instance: &DynamicProxyInstance,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<DexExecutionResult> {
        let parameter_classes = self
            .allocate_class_array(&method.proto.parameters)
            .ok_or_else(|| {
                WinedroidError::new(format!(
                    "proxy method parameters could not be represented\nmethod: {}{}",
                    method.name,
                    method.proto.descriptor()
                ))
            })?;
        let parameter_key = map_identity_key(&parameter_classes);
        let method_value = DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/lang/reflect/Method;:{}:{}:{parameter_key}",
            method.class_descriptor, method.name
        )));

        let argument_values = compact_proxy_method_arguments(method, &invocation.arguments[1..])?;
        let argument_array = if argument_values.is_empty() {
            if self.target_sdk_version > 21 {
                DexValue::Null
            } else {
                self.allocate_filled_array("[Ljava/lang/Object;".to_owned(), Vec::new())
            }
        } else {
            let mut values = Vec::with_capacity(argument_values.len());
            for (descriptor, value) in method.proto.parameters.iter().zip(argument_values.iter()) {
                let boxed = box_reflected_field_value(
                    &DexFieldId {
                        class_descriptor: "Ljava/lang/Object;".to_owned(),
                        type_descriptor: descriptor.clone(),
                        name: String::new(),
                    },
                    value.clone(),
                )
                .ok_or_else(|| {
                    WinedroidError::new(format!(
                        "proxy method argument could not be boxed\nmethod: {}{}\nargument type: {descriptor}\nargument: {value}",
                        method.name,
                        method.proto.descriptor()
                    ))
                })?;
                values.push(boxed);
            }

            self.allocate_filled_array("[Ljava/lang/Object;".to_owned(), values)
        };

        let handler_descriptor = value_class_descriptor(&instance.handler).ok_or_else(|| {
            WinedroidError::new(format!(
                "proxy invocation handler is not object-backed\nhandler: {}\nnext missing primitive: java.lang.reflect.InvocationHandler.invoke",
                instance.handler
            ))
        })?;
        let invoke_method = DexMethodId {
            class_descriptor: "Ljava/lang/reflect/InvocationHandler;".to_owned(),
            name: "invoke".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "LLL".to_owned(),
                return_type: "Ljava/lang/Object;".to_owned(),
                parameters: vec![
                    "Ljava/lang/Object;".to_owned(),
                    "Ljava/lang/reflect/Method;".to_owned(),
                    "[Ljava/lang/Object;".to_owned(),
                ],
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(handler_descriptor, &invoke_method)?
        else {
            return Err(WinedroidError::new(format!(
                "InvocationHandler.invoke was not found\nhandler class: {handler_descriptor}\nnext missing primitive: method Ljava/lang/reflect/InvocationHandler;.invoke(Ljava/lang/Object;Ljava/lang/reflect/Method;[Ljava/lang/Object;)Ljava/lang/Object;"
            )));
        };

        let arguments = vec![
            instance.handler.clone(),
            invocation.arguments[0].clone(),
            method_value,
            argument_array,
        ];
        let result = self.execute_method(None, &resolved, Some(&arguments), depth + 1)?;

        self.proxy_handler_result(&method.proto.return_type, result)
    }

    fn proxy_handler_result(
        &self,
        return_descriptor: &str,
        result: DexExecutionResult,
    ) -> Result<DexExecutionResult> {
        match result {
            DexExecutionResult::Value(value) => {
                if return_descriptor == "V" {
                    return Ok(DexExecutionResult::Void);
                }

                self.proxy_return_value(return_descriptor, value)
            }
            DexExecutionResult::Void => {
                if return_descriptor == "V" {
                    Ok(DexExecutionResult::Void)
                } else {
                    Err(WinedroidError::new(
                        "InvocationHandler.invoke returned void for a non-void proxy method",
                    ))
                }
            }
            DexExecutionResult::Int(value) => {
                self.proxy_return_value(return_descriptor, DexValue::Int(value))
            }
            DexExecutionResult::Throw(exception) => Ok(DexExecutionResult::Throw(exception)),
            DexExecutionResult::Parked(_) => Err(WinedroidError::new(
                "InvocationHandler.invoke parked, which a proxy call boundary cannot resume",
            )),
        }
    }

    fn proxy_return_value(
        &self,
        return_descriptor: &str,
        value: DexValue,
    ) -> Result<DexExecutionResult> {
        if return_descriptor == "V" {
            return Ok(DexExecutionResult::Void);
        }

        if !return_descriptor.starts_with('L') && !return_descriptor.starts_with('[') {
            return self.unbox_proxy_result(return_descriptor, value);
        }

        if value == DexValue::Null {
            return Ok(DexExecutionResult::Value(DexValue::Null));
        }

        if !matches!(
            value,
            DexValue::Object(_) | DexValue::Reference(_) | DexValue::String(_)
        ) {
            return Err(WinedroidError::new(format!(
                "InvocationHandler.invoke returned a primitive for proxy return type {return_descriptor}\nresult: {value}"
            )));
        }

        if !self.value_matches_type(&value, return_descriptor) {
            return Ok(DexExecutionResult::Throw(self.proxy_exception(
                "Ljava/lang/ClassCastException;",
                Some(format!("cannot cast {value} to {return_descriptor}")),
            )));
        }

        Ok(DexExecutionResult::Value(value))
    }

    fn unbox_proxy_result(
        &self,
        return_descriptor: &str,
        value: DexValue,
    ) -> Result<DexExecutionResult> {
        let result = match return_descriptor {
            "Z" => match &value {
                DexValue::Int(value) => Some(DexExecutionResult::Int(i32::from(*value != 0))),
                DexValue::Reference(reference)
                    if reference.as_ref() == "Ljava/lang/Boolean;:true" =>
                {
                    Some(DexExecutionResult::Int(1))
                }
                DexValue::Reference(reference)
                    if reference.as_ref() == "Ljava/lang/Boolean;:false" =>
                {
                    Some(DexExecutionResult::Int(0))
                }
                _ => None,
            },
            "B" | "S" | "C" | "I" => match &value {
                DexValue::Int(value) => Some(DexExecutionResult::Int(*value)),
                DexValue::Reference(reference) => {
                    let prefix = match return_descriptor {
                        "B" => "Ljava/lang/Byte;:",
                        "S" => "Ljava/lang/Short;:",
                        "C" => "Ljava/lang/Character;:",
                        _ => "Ljava/lang/Integer;:",
                    };
                    reference
                        .strip_prefix(prefix)
                        .and_then(|value| value.parse::<i32>().ok())
                        .map(DexExecutionResult::Int)
                }
                _ => None,
            },
            "J" => match &value {
                DexValue::Long(value) => Some(DexExecutionResult::Value(DexValue::Long(*value))),
                DexValue::Int(value) => {
                    Some(DexExecutionResult::Value(DexValue::Long(i64::from(*value))))
                }
                DexValue::Reference(reference) => boxed_long_reference_value(reference)
                    .map(|value| DexExecutionResult::Value(DexValue::Long(value))),
                _ => None,
            },
            "F" => match &value {
                DexValue::Float(value) => Some(DexExecutionResult::Value(DexValue::Float(*value))),
                DexValue::Reference(reference) => boxed_float_reference_value(reference)
                    .map(|value| DexExecutionResult::Value(DexValue::Float(value))),
                _ => None,
            },
            "D" => match &value {
                DexValue::Long(bits) => Some(DexExecutionResult::Value(DexValue::Long(*bits))),
                DexValue::Reference(reference) => boxed_double_reference_value(reference)
                    .map(|bits| DexExecutionResult::Value(DexValue::Long(bits))),
                _ => None,
            },
            _ => None,
        };

        match result {
            Some(result) => Ok(result),
            None if value == DexValue::Null => Ok(DexExecutionResult::Throw(
                self.proxy_exception("Ljava/lang/NullPointerException;", None),
            )),
            None => Ok(DexExecutionResult::Throw(self.proxy_exception(
                "Ljava/lang/ClassCastException;",
                Some(format!(
                    "cannot unbox {value} to proxy return type {return_descriptor}"
                )),
            ))),
        }
    }

    pub(super) fn proxy_exception(
        &self,
        class_descriptor: &str,
        detail_message: Option<String>,
    ) -> DexValue {
        self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: class_descriptor.to_owned(),
            detail_message,
        })
    }

    pub(super) fn proxy_get_invocation_handler(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let proxy = &invocation.arguments[0];
        if matches!(proxy, DexValue::Null | DexValue::Int(0)) {
            return Some(DexExecutionResult::Throw(
                self.proxy_exception("Ljava/lang/NullPointerException;", None),
            ));
        }
        let DexValue::Object(object) = proxy else {
            return Some(DexExecutionResult::Throw(self.proxy_exception(
                "Ljava/lang/IllegalArgumentException;",
                Some("not a proxy instance".to_owned()),
            )));
        };
        let handler = self
            .runtime
            .borrow()
            .proxy_instances
            .get(&object.id)
            .map(|instance| instance.handler.clone());

        match handler {
            Some(handler) => Some(DexExecutionResult::Value(handler)),
            None => Some(DexExecutionResult::Throw(self.proxy_exception(
                "Ljava/lang/IllegalArgumentException;",
                Some("not a proxy instance".to_owned()),
            ))),
        }
    }

    pub(super) fn proxy_is_proxy_class(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if matches!(invocation.arguments[0], DexValue::Null | DexValue::Int(0)) {
            return Some(DexExecutionResult::Throw(
                self.proxy_exception("Ljava/lang/NullPointerException;", None),
            ));
        }
        let descriptor = class_reference_descriptor_from_value(&invocation.arguments[0])?;
        let is_proxy = self.runtime.borrow().proxy_classes.contains_key(descriptor);

        Some(DexExecutionResult::Int(i32::from(is_proxy)))
    }
}

fn descriptor_package_prefix(descriptor: &str) -> &str {
    let descriptor = descriptor.strip_prefix('L').unwrap_or(descriptor);
    match descriptor.rfind('/') {
        Some(index) => &descriptor[..index],
        None => "",
    }
}

fn compact_proxy_method_arguments(
    method: &DexMethodId,
    arguments: &[DexValue],
) -> Result<Vec<DexValue>> {
    let mut values = Vec::with_capacity(method.proto.parameters.len());
    let mut index = 0;

    for parameter in &method.proto.parameters {
        if matches!(parameter.as_str(), "J" | "D") {
            let Some(low) = arguments.get(index) else {
                return Err(proxy_argument_count_error(method, arguments.len()));
            };
            let Some(high) = arguments.get(index + 1) else {
                return Err(proxy_argument_count_error(method, arguments.len()));
            };
            let DexValue::Long(low_value) = low else {
                return Err(proxy_argument_type_error(method, parameter, low));
            };
            let DexValue::Long(high_value) = high else {
                return Err(proxy_argument_type_error(method, parameter, high));
            };
            if low_value != high_value {
                return Err(WinedroidError::new(format!(
                    "proxy method wide argument halves differ\nmethod: {}{}\nargument: {low_value} != {high_value}",
                    method.name,
                    method.proto.descriptor()
                )));
            }
            values.push(low.clone());
            index += 2;
        } else {
            let Some(value) = arguments.get(index) else {
                return Err(proxy_argument_count_error(method, arguments.len()));
            };
            values.push(value.clone());
            index += 1;
        }
    }

    if index != arguments.len() {
        return Err(proxy_argument_count_error(method, arguments.len()));
    }

    Ok(values)
}

fn proxy_argument_count_error(method: &DexMethodId, count: usize) -> WinedroidError {
    WinedroidError::new(format!(
        "proxy method argument register count does not match its prototype\nmethod: {}{}\narguments: {count}",
        method.name,
        method.proto.descriptor()
    ))
}

fn proxy_argument_type_error(
    method: &DexMethodId,
    parameter: &str,
    value: &DexValue,
) -> WinedroidError {
    WinedroidError::new(format!(
        "proxy method wide argument is not a long value\nmethod: {}{}\nparameter: {parameter}\nargument: {value}",
        method.name,
        method.proto.descriptor()
    ))
}
