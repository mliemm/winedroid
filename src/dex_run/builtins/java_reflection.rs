use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_reflection_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "toString"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.class_to_string(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = class_get_name(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getTypeParameters"
            && method
                .proto
                .descriptor_is("()[Ljava/lang/reflect/TypeVariable;")
        {
            let invocation = invocation?;

            return self.class_type_parameters(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/ParameterizedType;"
            && method.name == "getActualTypeArguments"
            && method.proto.descriptor_is("()[Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.parameterized_type_arguments(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/ParameterizedType;"
            && method.name == "getRawType"
            && method.proto.descriptor_is("()Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.parameterized_type_raw_type(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/ParameterizedType;"
            && method.name == "getOwnerType"
            && method.proto.descriptor_is("()Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.parameterized_type_owner_type(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/GenericArrayType;"
            && method.name == "getGenericComponentType"
            && method.proto.descriptor_is("()Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.generic_array_component_type(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/WildcardType;"
            && method.name == "getUpperBounds"
            && method.proto.descriptor_is("()[Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.wildcard_upper_bounds(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/WildcardType;"
            && method.name == "getLowerBounds"
            && method.proto.descriptor_is("()[Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.wildcard_lower_bounds(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/TypeVariable;"
            && method.name == "getName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;

            return self.type_variable_name(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/TypeVariable;"
            && method.name == "getBounds"
            && method.proto.descriptor_is("()[Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.type_variable_bounds(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/TypeVariable;"
            && method.name == "getGenericDeclaration"
            && method
                .proto
                .descriptor_is("()Ljava/lang/reflect/GenericDeclaration;")
        {
            let invocation = invocation?;

            return self.type_variable_declaration(invocation);
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getCanonicalName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = class_get_name(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getSimpleName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = class_get_simple_name(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "asSubclass"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Class;)Ljava/lang/Class;")
        {
            // cite: libcore Class.java#asSubclass
            let invocation = invocation?;
            let class = class_reference_descriptor_from_value(&invocation.arguments[0])?;
            let superclass = class_reference_descriptor_from_value(&invocation.arguments[1])?;
            let class_value = DexValue::Reference(std::rc::Rc::from(class));
            if !self.value_matches_type(&class_value, superclass) {
                return Some(DexExecutionResult::Throw(
                    self.allocate_object("Ljava/lang/ClassCastException;".to_owned()),
                ));
            }

            return Some(DexExecutionResult::Value(invocation.arguments[0].clone()));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getClassLoader"
            && method.proto.descriptor_is("()Ljava/lang/ClassLoader;")
        {
            let invocation = invocation?;
            let value = self.class_get_class_loader(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/ClassLoader;"
            && method.name == "loadClass"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/lang/Class;")
        {
            let invocation = invocation?;
            return self.class_loader_load_class(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/Array;"
            && method.name == "newInstance"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Class;I)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.reflect_array_new_instance(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getPackage"
            && method.proto.descriptor_is("()Ljava/lang/Package;")
        {
            let invocation = invocation?;
            let value = class_get_package(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getConstructors"
            && method
                .proto
                .descriptor_is("()[Ljava/lang/reflect/Constructor;")
        {
            let invocation = invocation?;
            let value = self.class_get_constructors(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Proxy;"
            && method.name == "newProxyInstance"
            && method.proto.descriptor()
                == "(Ljava/lang/ClassLoader;[Ljava/lang/Class;Ljava/lang/reflect/InvocationHandler;)Ljava/lang/Object;"
        {
            let invocation = invocation?;

            return self.new_dynamic_proxy_instance(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/Proxy;"
            && method.name == "getInvocationHandler"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/lang/reflect/InvocationHandler;")
        {
            let invocation = invocation?;

            return self.proxy_get_invocation_handler(invocation);
        }

        if method.class_descriptor == "Ljava/lang/reflect/Proxy;"
            && method.name == "isProxyClass"
            && method.proto.descriptor_is("(Ljava/lang/Class;)Z")
        {
            let invocation = invocation?;

            return self.proxy_is_proxy_class(invocation);
        }

        if method.matches_builtin_signature(
            "Ljava/lang/reflect/Constructor;",
            "getModifiers",
            "()I",
        ) {
            let invocation = invocation?;
            let value = self.constructor_get_modifiers(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Constructor;"
            && method.name == "getParameterTypes"
            && method.proto.descriptor_is("()[Ljava/lang/Class;")
        {
            let invocation = invocation?;
            let value = self.constructor_get_parameter_types(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getDeclaredConstructor"
            && method
                .proto
                .descriptor_is("([Ljava/lang/Class;)Ljava/lang/reflect/Constructor;")
        {
            let invocation = invocation?;
            let value = class_get_declared_constructor(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getConstructor"
            && method
                .proto
                .descriptor_is("([Ljava/lang/Class;)Ljava/lang/reflect/Constructor;")
        {
            let invocation = invocation?;
            let value = class_get_declared_constructor(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getMethod"
            && method.proto.descriptor()
                == "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;"
        {
            let invocation = invocation?;
            let value = class_get_method(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getDeclaredMethod"
            && method.proto.descriptor()
                == "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;"
        {
            let invocation = invocation?;
            let value = class_get_method(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getField"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/lang/reflect/Field;")
        {
            let invocation = invocation?;
            let value = class_get_field(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getDeclaredField"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/lang/reflect/Field;")
        {
            let invocation = invocation?;

            return self.class_get_declared_field(invocation);
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getDeclaredFields"
            && method.proto.descriptor_is("()[Ljava/lang/reflect/Field;")
        {
            let invocation = invocation?;
            let value = self.class_get_declared_fields(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getDeclaredMethods"
            && method.proto.descriptor_is("()[Ljava/lang/reflect/Method;")
        {
            let value =
                self.allocate_filled_array("[Ljava/lang/reflect/Method;".to_owned(), Vec::new());

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getInterfaces"
            && method.proto.descriptor_is("()[Ljava/lang/Class;")
        {
            let invocation = invocation?;
            let value = self.class_get_interfaces(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getGenericInterfaces"
            && method.proto.descriptor_is("()[Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.class_generic_interfaces(invocation);
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getSuperclass"
            && method.proto.descriptor_is("()Ljava/lang/Class;")
        {
            let invocation = invocation?;
            let value = self.class_get_superclass(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getGenericSuperclass"
            && method.proto.descriptor_is("()Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;

            return self.class_generic_superclass(invocation);
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getEnclosingClass"
            && method.proto.descriptor_is("()Ljava/lang/Class;")
        {
            let invocation = invocation?;

            return self.class_get_enclosing_class(invocation);
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getDeclaringClass"
            && method.proto.descriptor_is("()Ljava/lang/Class;")
        {
            let invocation = invocation?;

            return self.class_get_declaring_class(invocation);
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getComponentType"
            && method.proto.descriptor_is("()Ljava/lang/Class;")
        {
            let invocation = invocation?;
            let value = class_get_component_type(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "isAssignableFrom"
            && method.proto.descriptor_is("(Ljava/lang/Class;)Z")
        {
            let invocation = invocation?;
            let value = self.class_is_assignable_from(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "cast"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            return self.class_cast(invocation);
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getModifiers"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.class_get_modifiers(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        // cite: libcore Class.java#getAnnotation
        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "getAnnotation"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Class;)Ljava/lang/annotation/Annotation;")
        {
            let invocation = invocation?;
            let value = self.class_get_annotation(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "isAnonymousClass"
            && method.proto.descriptor_is("()Z")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "isLocalClass"
            && method.proto.descriptor_is("()Z")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "isMemberClass"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = class_is_member_class(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "isArray"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = class_is_array(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "isPrimitive"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = class_is_primitive(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "isInterface"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.class_is_interface(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "isInstance"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.class_is_instance(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/Package;"
            && method.name == "getName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = package_get_name(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/lang/reflect/AccessibleObject;"
            && method.name == "isAccessible"
            && method.proto.descriptor_is("()Z")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Ljava/lang/reflect/AccessibleObject;"
            && method.name == "setAccessible"
            && method.proto.descriptor_is("(Z)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/lang/reflect/AccessibleObject;"
            && method.name == "getAnnotations"
            && method
                .proto
                .descriptor_is("()[Ljava/lang/annotation/Annotation;")
        {
            let value = self
                .allocate_filled_array("[Ljava/lang/annotation/Annotation;".to_owned(), Vec::new());

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Method;"
            && method.name == "invoke"
            && method.proto.descriptor()
                == "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;"
        {
            let invocation = invocation?;
            let value = self.reflect_method_invoke(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Method;"
            && method.name == "getName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = reflect_method_get_name(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Method;"
            && method.name == "getDeclaringClass"
            && method.proto.descriptor_is("()Ljava/lang/Class;")
        {
            let invocation = invocation?;
            let value = reflect_method_get_declaring_class(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/lang/reflect/Method;",
            "getDefaultValue",
            "()Ljava/lang/Object;",
        ) {
            // cite: libcore Method.java#getDefaultValue
            // cite: art java_lang_reflect_Method.cc#Method_getDefaultValue
            let invocation = invocation?;
            let (class_descriptor, _) = reflect_method_reference(invocation.arguments.first()?)?;
            let loaded_class = self.load_class(class_descriptor)?;
            if loaded_class.class_def.access_flags & JAVA_ACC_ANNOTATION != 0 {
                return None;
            }

            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Method;"
            && method.name == "getParameterTypes"
            && method.proto.descriptor_is("()[Ljava/lang/Class;")
        {
            let invocation = invocation?;
            let value = self.method_get_parameter_types(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Method;"
            && method.name == "getReturnType"
            && method.proto.descriptor_is("()Ljava/lang/Class;")
        {
            let invocation = invocation?;
            let value = self.reflect_method_get_return_type(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "getInt"
            && method.proto.descriptor_is("(Ljava/lang/Object;)I")
        {
            let invocation = invocation?;
            let value = self.reflect_field_get_int(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "get"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.reflect_field_get(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "getName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = reflect_field_get_name(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "getType"
            && method.proto.descriptor_is("()Ljava/lang/Class;")
        {
            let invocation = invocation?;
            let value = reflect_field_get_type(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "getGenericType"
            && method.proto.descriptor_is("()Ljava/lang/reflect/Type;")
        {
            let invocation = invocation?;
            let value = reflect_field_get_type(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "getModifiers"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = reflect_field_get_modifiers(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "isSynthetic"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = reflect_field_is_synthetic(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "getAnnotation"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Class;)Ljava/lang/annotation/Annotation;")
        {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "setAccessible"
            && method.proto.descriptor_is("(Z)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "isAccessible"
            && method.proto.descriptor_is("()Z")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "equals"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = reflect_field_equals(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Field;"
            && method.name == "hashCode"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = reflect_field_hash_code(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Modifier;"
            && method.name == "isAbstract"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = modifier_has_flag(invocation, JAVA_ACC_ABSTRACT)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Modifier;"
            && method.name == "isStatic"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = modifier_has_flag(invocation, DEX_ACC_STATIC)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Modifier;"
            && method.name == "isTransient"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = modifier_has_flag(invocation, JAVA_ACC_TRANSIENT)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Modifier;"
            && method.name == "isFinal"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = modifier_has_flag(invocation, JAVA_ACC_FINAL)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Modifier;"
            && method.name == "isPrivate"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = modifier_has_flag(invocation, JAVA_ACC_PRIVATE)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Modifier;"
            && method.name == "isProtected"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = modifier_has_flag(invocation, JAVA_ACC_PROTECTED)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Modifier;"
            && method.name == "isPublic"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = modifier_has_flag(invocation, JAVA_ACC_PUBLIC)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/TimeUnit;"
            && matches!(method.name.as_str(), "toMillis" | "toNanos")
            && method.proto.descriptor_is("(J)J")
        {
            let invocation = invocation?;
            let value = if method.name == "toMillis" {
                time_unit_to_millis(invocation)?
            } else {
                time_unit_to_nanos(invocation)?
            };

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/TimeUnit;",
            "convert",
            "(JLjava/util/concurrent/TimeUnit;)J",
        ) {
            let invocation = invocation?;
            let value = time_unit_convert(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        None
    }
}
