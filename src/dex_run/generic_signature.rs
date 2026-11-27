use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum GenericType {
    Class {
        descriptor: String,
    },
    Parameterized {
        owner: Option<Box<GenericType>>,
        raw_descriptor: String,
        arguments: Vec<GenericType>,
    },
    TypeVariable {
        name: String,
        declaration: Option<String>,
        bounds: Vec<GenericType>,
    },
    Array {
        component: Box<GenericType>,
    },
    Wildcard {
        upper: Vec<GenericType>,
        lower: Vec<GenericType>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ClassSignature {
    pub(super) formal_parameters: Vec<GenericType>,
    pub(super) superclass: GenericType,
    pub(super) interfaces: Vec<GenericType>,
}

// cite: libcore GenericSignatureParser.java#parseClassSignature, parseClassTypeSignature, parseTypeArgument
pub(super) fn parse_class_signature(
    class_descriptor: &str,
    signature: &str,
) -> Option<ClassSignature> {
    let mut parser = ClassSignatureParser {
        input: signature.as_bytes(),
        position: 0,
        class_descriptor,
        formal_parameters: Vec::new(),
    };
    parser.parse_formal_type_parameters()?;
    let superclass = parser.parse_class_type_signature()?;
    let mut interfaces = Vec::new();
    while parser.peek().is_some() {
        interfaces.push(parser.parse_class_type_signature()?);
    }

    Some(ClassSignature {
        formal_parameters: parser.formal_parameters,
        superclass,
        interfaces,
    })
}

struct ClassSignatureParser<'a> {
    input: &'a [u8],
    position: usize,
    class_descriptor: &'a str,
    formal_parameters: Vec<GenericType>,
}

impl ClassSignatureParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.position += 1;
            return true;
        }

        false
    }

    fn expect(&mut self, byte: u8) -> Option<()> {
        if self.eat(byte) {
            Some(())
        } else {
            None
        }
    }

    fn parse_identifier(&mut self) -> Option<String> {
        let start = self.position;
        while let Some(byte) = self.peek() {
            if matches!(byte, b':' | b';' | b'<' | b'>') {
                break;
            }
            self.position += 1;
        }
        if self.position == start {
            return None;
        }

        String::from_utf8(self.input[start..self.position].to_vec()).ok()
    }

    fn parse_formal_type_parameters(&mut self) -> Option<()> {
        if !self.eat(b'<') {
            return Some(());
        }

        loop {
            let name = self.parse_identifier()?;
            self.expect(b':')?;

            let mut bounds = Vec::new();
            if matches!(self.peek(), Some(b'L' | b'[' | b'T')) {
                bounds.push(self.parse_field_type_signature()?);
            }
            while self.eat(b':') {
                bounds.push(self.parse_field_type_signature()?);
            }

            self.formal_parameters.push(GenericType::TypeVariable {
                name,
                declaration: Some(self.class_descriptor.to_owned()),
                bounds,
            });

            if self.eat(b'>') {
                break;
            }
        }

        Some(())
    }

    fn parse_type_signature(&mut self) -> Option<GenericType> {
        match self.peek()? {
            b'L' | b'[' | b'T' => self.parse_field_type_signature(),
            byte @ (b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z') => {
                self.position += 1;
                Some(GenericType::Class {
                    descriptor: (byte as char).to_string(),
                })
            }
            _ => None,
        }
    }

    fn parse_field_type_signature(&mut self) -> Option<GenericType> {
        match self.peek()? {
            b'L' => self.parse_class_type_signature(),
            b'[' => {
                self.position += 1;
                Some(GenericType::Array {
                    component: Box::new(self.parse_type_signature()?),
                })
            }
            b'T' => {
                self.position += 1;
                let name = self.parse_identifier()?;
                self.expect(b';')?;
                Some(
                    self.formal_parameters
                        .iter()
                        .find(|parameter| {
                            matches!(
                                parameter,
                                GenericType::TypeVariable { name: parameter_name, .. }
                                    if parameter_name == &name
                            )
                        })
                        .cloned()
                        .unwrap_or(GenericType::TypeVariable {
                            name,
                            declaration: None,
                            bounds: Vec::new(),
                        }),
                )
            }
            _ => None,
        }
    }

    fn parse_class_type_signature(&mut self) -> Option<GenericType> {
        self.expect(b'L')?;

        let mut raw_name = String::new();
        let mut owner = None;

        loop {
            let identifier = self.parse_identifier()?;
            if !raw_name.is_empty() {
                raw_name.push('$');
            }
            raw_name.push_str(&identifier);
            let arguments = self.parse_type_arguments()?;

            if !self.eat(b'.') {
                self.expect(b';')?;

                return Some(build_parameterized(
                    owner,
                    format!("L{raw_name};"),
                    arguments,
                ));
            }

            owner = Some(Box::new(build_parameterized(
                owner.take(),
                format!("L{raw_name};"),
                arguments,
            )));
        }
    }

    fn parse_type_arguments(&mut self) -> Option<Vec<GenericType>> {
        if !self.eat(b'<') {
            return Some(Vec::new());
        }

        let mut arguments = Vec::new();
        loop {
            arguments.push(self.parse_type_argument()?);
            if self.eat(b'>') {
                break;
            }
        }

        Some(arguments)
    }

    fn parse_type_argument(&mut self) -> Option<GenericType> {
        match self.peek()? {
            b'*' => {
                self.position += 1;
                Some(GenericType::Wildcard {
                    upper: vec![object_class_type()],
                    lower: Vec::new(),
                })
            }
            b'+' => {
                self.position += 1;
                Some(GenericType::Wildcard {
                    upper: vec![self.parse_field_type_signature()?],
                    lower: Vec::new(),
                })
            }
            b'-' => {
                self.position += 1;
                Some(GenericType::Wildcard {
                    upper: vec![object_class_type()],
                    lower: vec![self.parse_field_type_signature()?],
                })
            }
            _ => self.parse_field_type_signature(),
        }
    }
}

fn build_parameterized(
    owner: Option<Box<GenericType>>,
    raw_descriptor: String,
    arguments: Vec<GenericType>,
) -> GenericType {
    if owner.is_none() && arguments.is_empty() {
        return GenericType::Class {
            descriptor: raw_descriptor,
        };
    }

    GenericType::Parameterized {
        owner,
        raw_descriptor,
        arguments,
    }
}

fn object_class_type() -> GenericType {
    GenericType::Class {
        descriptor: "Ljava/lang/Object;".to_owned(),
    }
}

// cite: art dex_file_annotations.cc#GetSignatureValue
// cite: libcore Class.java#getSignatureAttribute
impl DexProgram {
    // cite: libcore Class.java#getEnclosingClass, getDeclaringClass
    // cite: art dex_file_annotations.cc#GetEnclosingClass
    pub(super) fn class_get_enclosing_class(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let descriptor = class_reference_descriptor(invocation)?;
        let enclosing = self.class_enclosing_class_descriptor(descriptor);
        let value = match enclosing {
            Some(descriptor) => {
                DexValue::Reference(std::rc::Rc::from(format!("Ljava/lang/Class;:{descriptor}")))
            }
            None => DexValue::Null,
        };

        Some(DexExecutionResult::Value(value))
    }

    pub(super) fn class_get_declaring_class(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let descriptor = class_reference_descriptor(invocation)?;
        let declaring = self
            .class_enclosing_class_descriptor(descriptor)
            .filter(|_| self.class_inner_class_is_named(descriptor));
        let value = match declaring {
            Some(descriptor) => {
                DexValue::Reference(std::rc::Rc::from(format!("Ljava/lang/Class;:{descriptor}")))
            }
            None => DexValue::Null,
        };

        Some(DexExecutionResult::Value(value))
    }

    fn class_enclosing_class_descriptor(&self, descriptor: &str) -> Option<String> {
        self.class_annotation_element(descriptor, "Ldalvik/annotation/EnclosingClass;", "value")
            .and_then(|value| match value {
                DexAnnotationValue::Type(descriptor) => Some(descriptor),
                _ => None,
            })
    }

    fn class_inner_class_is_named(&self, descriptor: &str) -> bool {
        matches!(
            self.class_annotation_element(descriptor, "Ldalvik/annotation/InnerClass;", "name"),
            Some(DexAnnotationValue::String(name)) if !name.is_empty()
        )
    }

    fn class_annotation_element(
        &self,
        descriptor: &str,
        annotation_descriptor: &str,
        element_name: &str,
    ) -> Option<DexAnnotationValue> {
        let loaded_class = self.load_class(descriptor)?;
        let annotations = self.dex_files[loaded_class.dex_file_index]
            .class_annotations(&loaded_class.class_def)
            .ok()?;
        let annotation = annotations.into_iter().find(|annotation| {
            annotation.visibility == DEX_VISIBILITY_SYSTEM
                && annotation.type_descriptor == annotation_descriptor
        })?;

        annotation
            .elements
            .into_iter()
            .find(|element| element.name == element_name)
            .map(|element| element.value)
    }

    pub(super) fn class_signature(&self, descriptor: &str) -> Option<String> {
        let loaded_class = self.load_class(descriptor)?;
        let annotations = self.dex_files[loaded_class.dex_file_index]
            .class_annotations(&loaded_class.class_def)
            .ok()?;
        let annotation = annotations.into_iter().find(|annotation| {
            annotation.visibility == DEX_VISIBILITY_SYSTEM
                && annotation.type_descriptor == "Ldalvik/annotation/Signature;"
        })?;
        let value = annotation
            .elements
            .into_iter()
            .find(|element| element.name == "value")?
            .value;
        let DexAnnotationValue::Array(values) = value else {
            return None;
        };

        let mut signature = String::new();
        for value in values {
            let DexAnnotationValue::String(segment) = value else {
                return None;
            };
            signature.push_str(&segment);
        }

        Some(signature)
    }

    pub(super) fn class_generic_superclass(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let descriptor = class_reference_descriptor(invocation)?;
        if self.superclass_descriptor(descriptor).is_none() {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if let Some(signature) = self.class_signature(descriptor) {
            let Some(parsed) = parse_class_signature(descriptor, &signature) else {
                return Some(DexExecutionResult::Throw(self.proxy_exception(
                    "Ljava/lang/reflect/GenericSignatureFormatError;",
                    None,
                )));
            };

            return Some(DexExecutionResult::Value(
                self.generic_type_value(&parsed.superclass),
            ));
        }

        let Some(value) = self.class_get_superclass(invocation) else {
            return None;
        };

        Some(DexExecutionResult::Value(value))
    }

    pub(super) fn class_generic_interfaces(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let descriptor = class_reference_descriptor(invocation)?;
        if let Some(signature) = self.class_signature(descriptor) {
            let Some(parsed) = parse_class_signature(descriptor, &signature) else {
                return Some(DexExecutionResult::Throw(self.proxy_exception(
                    "Ljava/lang/reflect/GenericSignatureFormatError;",
                    None,
                )));
            };
            let values = parsed
                .interfaces
                .iter()
                .map(|interface| self.generic_type_value(interface))
                .collect();

            return Some(DexExecutionResult::Value(self.allocate_filled_array(
                "[Ljava/lang/reflect/Type;".to_owned(),
                values,
            )));
        }

        let Some(value) = self.class_get_interfaces(invocation) else {
            return None;
        };

        Some(DexExecutionResult::Value(value))
    }

    pub(super) fn class_type_parameters(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let descriptor = class_reference_descriptor(invocation)?;
        let parameters = self
            .class_signature(descriptor)
            .and_then(|signature| parse_class_signature(descriptor, &signature))
            .map(|parsed| {
                parsed
                    .formal_parameters
                    .iter()
                    .map(|parameter| self.generic_type_value(parameter))
                    .collect()
            })
            .unwrap_or_default();

        Some(DexExecutionResult::Value(self.allocate_filled_array(
            "[Ljava/lang/reflect/TypeVariable;".to_owned(),
            parameters,
        )))
    }

    pub(super) fn parameterized_type_arguments(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::Parameterized { arguments, .. } = ty else {
            return None;
        };
        let values = arguments
            .iter()
            .map(|argument| self.generic_type_value(argument))
            .collect();

        Some(DexExecutionResult::Value(self.allocate_filled_array(
            "[Ljava/lang/reflect/Type;".to_owned(),
            values,
        )))
    }

    pub(super) fn parameterized_type_raw_type(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::Parameterized { raw_descriptor, .. } = ty else {
            return None;
        };

        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/lang/Class;:{raw_descriptor}")),
        )))
    }

    pub(super) fn parameterized_type_owner_type(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::Parameterized { owner, .. } = ty else {
            return None;
        };
        let value = match owner {
            Some(owner) => self.generic_type_value(&owner),
            None => DexValue::Null,
        };

        Some(DexExecutionResult::Value(value))
    }

    pub(super) fn generic_array_component_type(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::Array { component } = ty else {
            return None;
        };

        Some(DexExecutionResult::Value(
            self.generic_type_value(&component),
        ))
    }

    pub(super) fn wildcard_upper_bounds(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::Wildcard { upper, .. } = ty else {
            return None;
        };
        let values = upper
            .iter()
            .map(|bound| self.generic_type_value(bound))
            .collect();

        Some(DexExecutionResult::Value(self.allocate_filled_array(
            "[Ljava/lang/reflect/Type;".to_owned(),
            values,
        )))
    }

    pub(super) fn wildcard_lower_bounds(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::Wildcard { lower, .. } = ty else {
            return None;
        };
        let values = lower
            .iter()
            .map(|bound| self.generic_type_value(bound))
            .collect();

        Some(DexExecutionResult::Value(self.allocate_filled_array(
            "[Ljava/lang/reflect/Type;".to_owned(),
            values,
        )))
    }

    pub(super) fn type_variable_name(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::TypeVariable { name, .. } = ty else {
            return None;
        };

        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(name),
        )))
    }

    pub(super) fn type_variable_bounds(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::TypeVariable { bounds, .. } = ty else {
            return None;
        };
        let values = bounds
            .iter()
            .map(|bound| self.generic_type_value(bound))
            .collect();

        Some(DexExecutionResult::Value(self.allocate_filled_array(
            "[Ljava/lang/reflect/Type;".to_owned(),
            values,
        )))
    }

    pub(super) fn type_variable_declaration(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let ty = self.generic_type_argument(invocation)?;
        let GenericType::TypeVariable { declaration, .. } = ty else {
            return None;
        };
        let value = match declaration {
            Some(descriptor) => {
                DexValue::Reference(std::rc::Rc::from(format!("Ljava/lang/Class;:{descriptor}")))
            }
            None => DexValue::Null,
        };

        Some(DexExecutionResult::Value(value))
    }

    fn generic_type_argument(&self, invocation: &DexInvocation) -> Option<GenericType> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.generic_type_from_value(&invocation.arguments[0])
    }

    fn generic_type_value(&self, ty: &GenericType) -> DexValue {
        match ty {
            GenericType::Class { descriptor } => {
                DexValue::Reference(std::rc::Rc::from(format!("Ljava/lang/Class;:{descriptor}")))
            }
            GenericType::Parameterized { .. } => {
                self.allocate_generic_type("Ljava/lang/reflect/ParameterizedType;", ty.clone())
            }
            GenericType::TypeVariable { .. } => {
                self.allocate_generic_type("Ljava/lang/reflect/TypeVariable;", ty.clone())
            }
            GenericType::Array { .. } => {
                self.allocate_generic_type("Ljava/lang/reflect/GenericArrayType;", ty.clone())
            }
            GenericType::Wildcard { .. } => {
                self.allocate_generic_type("Ljava/lang/reflect/WildcardType;", ty.clone())
            }
        }
    }

    fn allocate_generic_type(&self, interface_descriptor: &str, ty: GenericType) -> DexValue {
        let id = {
            let mut runtime = self.runtime.borrow_mut();
            let id = runtime.next_generic_type_id;
            runtime.next_generic_type_id += 1;
            runtime.generic_types.insert(id, ty);
            id
        };

        DexValue::Reference(std::rc::Rc::from(format!("{interface_descriptor}:{id}")))
    }

    pub(super) fn generic_type_from_value(&self, value: &DexValue) -> Option<GenericType> {
        let DexValue::Reference(reference) = value else {
            return None;
        };
        for interface_descriptor in [
            "Ljava/lang/reflect/ParameterizedType;:",
            "Ljava/lang/reflect/TypeVariable;:",
            "Ljava/lang/reflect/GenericArrayType;:",
            "Ljava/lang/reflect/WildcardType;:",
        ] {
            let Some(id) = reference.strip_prefix(interface_descriptor) else {
                continue;
            };
            let id = id.parse::<u64>().ok()?;

            return self.runtime.borrow().generic_types.get(&id).cloned();
        }

        None
    }

    pub(super) fn generic_type_hash_code(&self, ty: &GenericType) -> i32 {
        match ty {
            GenericType::Class { descriptor } => {
                stable_hash_i32(&format!("reference:Ljava/lang/Class;:{descriptor}"))
            }
            GenericType::Parameterized {
                owner,
                raw_descriptor,
                arguments,
            } => {
                let raw_type = GenericType::Class {
                    descriptor: raw_descriptor.clone(),
                };
                31 * (31 * self.generic_type_hash_code(&raw_type)
                    + owner
                        .as_deref()
                        .map(|owner| self.generic_type_hash_code(owner))
                        .unwrap_or(0))
                    + self.generic_type_array_hash_code(arguments)
            }
            GenericType::TypeVariable {
                name, declaration, ..
            } => {
                let declaration_hash = declaration
                    .as_deref()
                    .map(|descriptor| {
                        self.generic_type_hash_code(&GenericType::Class {
                            descriptor: descriptor.to_owned(),
                        })
                    })
                    .unwrap_or(0);
                31 * stable_hash_i32(name) + declaration_hash
            }
            GenericType::Array { component } => self.generic_type_hash_code(component),
            GenericType::Wildcard { upper, lower } => {
                31 * self.generic_type_array_hash_code(lower)
                    + self.generic_type_array_hash_code(upper)
            }
        }
    }

    fn generic_type_array_hash_code(&self, types: &[GenericType]) -> i32 {
        let mut hash = 1;
        for ty in types {
            hash = 31 * hash + self.generic_type_hash_code(ty);
        }

        hash
    }

    pub(super) fn generic_type_values_equal(
        &self,
        left: &DexValue,
        right: &DexValue,
    ) -> Option<bool> {
        let left = self.generic_type_from_value(left);
        let right = self.generic_type_from_value(right);

        match (left, right) {
            (Some(left), Some(right)) => Some(left == right),
            (None, None) => None,
            _ => Some(false),
        }
    }

    pub(super) fn object_hash_code_value(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if let Some(ty) = self.generic_type_from_value(&invocation.arguments[0]) {
            return Some(self.generic_type_hash_code(&ty));
        }

        object_hash_code(invocation)
    }

    pub(super) fn object_equals_value(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.generic_type_from_value(&invocation.arguments[0]);
        let right = self.generic_type_from_value(&invocation.arguments[1]);

        match (left, right) {
            (Some(left), Some(right)) => Some(i32::from(left == right)),
            (None, None) => object_equals(invocation),
            _ => Some(0),
        }
    }

    pub(super) fn generic_type_object_to_string(&self, value: &DexValue) -> Option<String> {
        self.generic_type_from_value(value)
            .map(|ty| self.generic_type_to_string(&ty))
    }

    pub(super) fn generic_type_to_string(&self, ty: &GenericType) -> String {
        match ty {
            GenericType::Class { descriptor } => {
                descriptor_to_java_name(descriptor).unwrap_or_else(|| descriptor.clone())
            }
            GenericType::Parameterized {
                owner,
                raw_descriptor,
                arguments,
            } => {
                let mut value = String::new();
                if let Some(owner) = owner {
                    value.push_str(&self.generic_type_to_string(owner));
                    value.push('$');
                    value.push_str(&self.class_simple_name(raw_descriptor));
                } else {
                    value.push_str(
                        &descriptor_to_java_name(raw_descriptor)
                            .unwrap_or_else(|| raw_descriptor.clone()),
                    );
                }
                if !arguments.is_empty() {
                    value.push('<');
                    value.push_str(
                        &arguments
                            .iter()
                            .map(|argument| self.generic_type_to_string(argument))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    value.push('>');
                }

                value
            }
            GenericType::TypeVariable { name, .. } => name.clone(),
            GenericType::Array { component } => {
                format!("{}[]", self.generic_type_to_string(component))
            }
            GenericType::Wildcard { upper, lower } => {
                let mut value = "?".to_owned();
                if !lower.is_empty() {
                    value.push_str(" super ");
                    value.push_str(
                        &lower
                            .iter()
                            .map(|bound| self.generic_type_to_string(bound))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                } else if !(upper.len() == 1
                    && upper.first().is_some_and(|bound| {
                        bound
                            == &GenericType::Class {
                                descriptor: "Ljava/lang/Object;".to_owned(),
                            }
                    }))
                    && !upper.is_empty()
                {
                    value.push_str(" extends ");
                    value.push_str(
                        &upper
                            .iter()
                            .map(|bound| self.generic_type_to_string(bound))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                }

                value
            }
        }
    }

    fn class_simple_name(&self, descriptor: &str) -> String {
        descriptor
            .trim_start_matches('L')
            .trim_end_matches(';')
            .rsplit(['/', '$'])
            .next()
            .unwrap_or(descriptor)
            .to_owned()
    }
}
