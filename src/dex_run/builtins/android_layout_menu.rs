use super::super::*;

impl DexProgram {
    pub(super) fn invoke_android_layout_menu_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Landroid/view/LayoutInflater;"
            && method.name == "from"
            && method.proto.descriptor()
                == "(Landroid/content/Context;)Landroid/view/LayoutInflater;"
        {
            let context = invocation?.arguments.first()?.clone();
            return Some(DexExecutionResult::Value(
                self.layout_inflater_for_context(&context),
            ));
        }

        // cite: frameworks-base LayoutInflater.java#getFactory, getFactory2, mFactory, mFactory2
        if method.class_descriptor == "Landroid/view/LayoutInflater;"
            && matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                ("getFactory", "()Landroid/view/LayoutInflater$Factory;")
                    | ("getFactory2", "()Landroid/view/LayoutInflater$Factory2;")
            )
            && !self.guest_overrides(method, invocation)
        {
            let field = layout_inflater_factory_field(method.name == "getFactory2");
            let factory = invocation
                .and_then(|invocation| invocation.arguments.first())
                .and_then(|inflater| self.read_stored_instance_field(inflater, &field))
                .unwrap_or(DexValue::Null);
            return Some(DexExecutionResult::Value(factory));
        }

        if method.matches_builtin_signature(
            "Landroid/view/LayoutInflater;",
            "getContext",
            "()Landroid/content/Context;",
        ) && !self.guest_overrides(method, invocation)
        {
            // cite: frameworks-base LayoutInflater.java#getContext, mContext
            let inflater = invocation?.arguments.first()?;
            let context = self
                .read_stored_instance_field(inflater, &layout_inflater_context_field())
                .or_else(|| self.runtime.borrow().activity_receiver.clone())
                .unwrap_or_else(|| self.application_value());
            return Some(DexExecutionResult::Value(context));
        }

        if method.class_descriptor == "Landroid/view/LayoutInflater;"
            && method.name == "cloneInContext"
            && method.proto.descriptor()
                == "(Landroid/content/Context;)Landroid/view/LayoutInflater;"
        {
            // cite: frameworks-base LayoutInflater.java#LayoutInflater, cloneInContext
            let [original, context] = invocation?.arguments.as_slice() else {
                return None;
            };
            let clone = self.layout_inflater_for_context(context);
            let clone_has_factory = self
                .read_stored_instance_field(&clone, &layout_inflater_factory_set_field())
                .is_some();
            if !clone_has_factory {
                for factory2 in [false, true] {
                    let field = layout_inflater_factory_field(factory2);
                    if let Some(factory) = self.read_stored_instance_field(original, &field) {
                        self.write_stored_instance_field(&clone, &field, factory);
                    }
                }
            }
            return Some(DexExecutionResult::Value(clone));
        }

        // cite: frameworks-base LayoutInflater.java#setFactory, setFactory2, LayoutInflater
        if method.class_descriptor == "Landroid/view/LayoutInflater;"
            && matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                ("setFactory", "(Landroid/view/LayoutInflater$Factory;)V")
                    | ("setFactory2", "(Landroid/view/LayoutInflater$Factory2;)V")
            )
            && !self.guest_overrides(method, invocation)
        {
            let [inflater, factory] = invocation?.arguments.as_slice() else {
                return None;
            };
            if !dex_value_is_object_like(inflater) {
                return None;
            }
            let owner = inflater.clone();
            let factory_set = layout_inflater_factory_set_field();
            if self.read_stored_instance_field(&owner, &factory_set) == Some(DexValue::Int(1)) {
                return Some(DexExecutionResult::Throw(
                    self.allocate_generated_exception(DexGeneratedException {
                        class_descriptor: "Ljava/lang/IllegalStateException;".to_owned(),
                        detail_message: Some(
                            "A factory has already been set on this LayoutInflater".to_owned(),
                        ),
                    }),
                ));
            }
            if !dex_value_is_object_like(factory) {
                return Some(self.null_pointer_exception_result());
            }
            self.write_stored_instance_field(&owner, &factory_set, DexValue::Int(1));
            let factory2 = method.name == "setFactory2";
            let current = |field| self.read_stored_instance_field(&owner, &field);
            let installed = match current(layout_inflater_factory_field(false)) {
                None | Some(DexValue::Null) => factory.clone(),
                Some(previous) => self.layout_inflater_factory_merger([
                    factory.clone(),
                    if factory2 {
                        factory.clone()
                    } else {
                        DexValue::Null
                    },
                    previous,
                    current(layout_inflater_factory_field(true)).unwrap_or(DexValue::Null),
                ]),
            };
            self.write_stored_instance_field(
                &owner,
                &layout_inflater_factory_field(false),
                installed.clone(),
            );
            if factory2 {
                self.write_stored_instance_field(
                    &owner,
                    &layout_inflater_factory_field(true),
                    installed,
                );
            }
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/LayoutInflater;"
            && method.name == "inflate"
            && method
                .proto
                .descriptor_is("(ILandroid/view/ViewGroup;Z)Landroid/view/View;")
        {
            let invocation = invocation?;
            let value = self.layout_inflater_inflate(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/LayoutInflater;"
            && method.name == "inflate"
            && method
                .proto
                .descriptor_is("(ILandroid/view/ViewGroup;)Landroid/view/View;")
        {
            let invocation = invocation?;
            let value = self.layout_inflater_inflate_without_attach(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/MenuInflater;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Landroid/content/Context;)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/MenuItem;"
            && method.name == "getItemId"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            if let DexValue::Reference(reference) = &invocation.arguments[0] {
                if let Some(item_id) = reference
                    .strip_prefix("Landroid/view/MenuItem;:option/")
                    .and_then(|id| id.parse::<i32>().ok())
                {
                    return Some(DexExecutionResult::Int(item_id));
                }
            }
        }

        if method.class_descriptor == "Ln/n0;"
            && method.name == "setWindowCallback"
            && method
                .proto
                .descriptor_is("(Landroid/view/Window$Callback;)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ln/n0;"
            && method.name == "setWindowTitle"
            && method.proto.descriptor_is("(Ljava/lang/CharSequence;)V")
        {
            return Some(DexExecutionResult::Void);
        }

        None
    }
}

fn layout_inflater_factory_set_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "mFactorySet".to_owned(),
    }
}

impl DexProgram {
    // cite: frameworks-base ContextImpl.java#getSystemService
    // cite: frameworks-base ContextThemeWrapper.java#getSystemService, mInflater
    pub(in crate::dex_run) fn layout_inflater_for_context(&self, context: &DexValue) -> DexValue {
        let context = if dex_value_is_object_like(context) {
            context.clone()
        } else {
            self.application_value()
        };
        let inflater = DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/view/LayoutInflater;:context:{}",
            map_identity_key(&context)
        )));
        self.write_stored_instance_field(&inflater, &layout_inflater_context_field(), context);
        inflater
    }
}

impl DexProgram {
    // cite: frameworks-base LayoutInflater.java#FactoryMerger, mF1, mF2, mF12, mF22
    fn layout_inflater_factory_merger(&self, factories: [DexValue; 4]) -> DexValue {
        let merger = self.allocate_object("Landroid/view/LayoutInflater$FactoryMerger;".to_owned());
        for ((name, type_descriptor), factory) in [
            ("mF1", "Landroid/view/LayoutInflater$Factory;"),
            ("mF12", "Landroid/view/LayoutInflater$Factory2;"),
            ("mF2", "Landroid/view/LayoutInflater$Factory;"),
            ("mF22", "Landroid/view/LayoutInflater$Factory2;"),
        ]
        .into_iter()
        .zip(factories)
        {
            self.write_stored_instance_field(
                &merger,
                &DexFieldId {
                    class_descriptor: "Landroid/view/LayoutInflater$FactoryMerger;".to_owned(),
                    type_descriptor: type_descriptor.to_owned(),
                    name: name.to_owned(),
                },
                factory,
            );
        }
        merger
    }
}

fn layout_inflater_context_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        type_descriptor: "Landroid/content/Context;".to_owned(),
        name: "mContext".to_owned(),
    }
}

fn layout_inflater_factory_field(factory2: bool) -> DexFieldId {
    let (name, type_descriptor) = if factory2 {
        ("mFactory2", "Landroid/view/LayoutInflater$Factory2;")
    } else {
        ("mFactory", "Landroid/view/LayoutInflater$Factory;")
    };
    DexFieldId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}
