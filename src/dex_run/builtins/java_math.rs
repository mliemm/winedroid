use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_math_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "toString",
            "()Ljava/lang/String;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_to_string(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.matches_builtin_signature("Ljava/math/BigDecimal;", "floatValue", "()F") {
            let invocation = invocation?;
            let value = self.big_decimal_float_value(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Float(value)));
        }

        if method.matches_builtin_signature("Ljava/math/BigDecimal;", "doubleValue", "()D") {
            let invocation = invocation?;
            let value = self.big_decimal_double_value(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature("Ljava/math/BigDecimal;", "intValue", "()I") {
            let invocation = invocation?;
            let value = self.big_decimal_int_value(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "valueOf",
            "(D)Ljava/math/BigDecimal;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_value_of_double(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigInteger;",
            "toString",
            "()Ljava/lang/String;",
        ) {
            let invocation = invocation?;
            let value = self.big_integer_to_string(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigInteger;",
            "valueOf",
            "(J)Ljava/math/BigInteger;",
        ) {
            let invocation = invocation?;
            let value = self.big_integer_value_of_long(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigInteger;",
            "multiply",
            "(Ljava/math/BigInteger;)Ljava/math/BigInteger;",
        ) {
            let invocation = invocation?;
            let value = self.big_integer_multiply(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "<init>",
            "(Ljava/lang/String;)V",
        ) {
            let invocation = invocation?;
            self.initialize_big_decimal_from_string(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "<init>",
            "(Ljava/math/BigInteger;)V",
        ) {
            let invocation = invocation?;
            self.initialize_big_decimal_from_big_integer(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature("Ljava/math/BigDecimal;", "<init>", "(I)V") {
            let invocation = invocation?;
            self.initialize_big_decimal_from_int(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature("Ljava/math/BigDecimal;", "<init>", "(D)V") {
            let invocation = invocation?;
            self.initialize_big_decimal_from_double(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigInteger;",
            "<init>",
            "(Ljava/lang/String;)V",
        ) {
            let invocation = invocation?;
            self.initialize_big_integer_from_string(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/math/MathContext;",
            "<init>",
            "(ILjava/math/RoundingMode;)V",
        ) {
            let invocation = invocation?;
            self.initialize_math_context(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "setScale",
            "(II)Ljava/math/BigDecimal;",
        ) {
            // cite: libcore BigDecimal.java#setScale, ROUND_UP, ROUND_DOWN, ROUND_CEILING, ROUND_FLOOR, ROUND_HALF_UP, ROUND_HALF_DOWN, ROUND_HALF_EVEN, ROUND_UNNECESSARY
            let invocation = invocation?;
            let [_, _, DexValue::Int(mode)] = invocation.arguments.as_slice() else {
                return None;
            };
            const MODES: [&str; 8] = [
                "UP",
                "DOWN",
                "CEILING",
                "FLOOR",
                "HALF_UP",
                "HALF_DOWN",
                "HALF_EVEN",
                "UNNECESSARY",
            ];
            let Some(mode) = usize::try_from(*mode)
                .ok()
                .and_then(|index| MODES.get(index))
            else {
                return Some(DexExecutionResult::Throw(
                    self.allocate_generated_exception(DexGeneratedException {
                        class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
                        detail_message: Some("Invalid rounding mode".to_owned()),
                    }),
                ));
            };
            let value = self.big_decimal_value(invocation.arguments.first()?)?;
            let DexValue::Int(scale) = invocation.arguments[1] else {
                return None;
            };
            let scaled = set_big_decimal_scale(&value, scale, mode)?;
            let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
            self.runtime
                .borrow_mut()
                .big_decimal_values
                .insert(map_identity_key(&result), scaled);

            return Some(DexExecutionResult::Value(result));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "setScale",
            "(ILjava/math/RoundingMode;)Ljava/math/BigDecimal;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_set_scale(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "add",
            "(Ljava/math/BigDecimal;)Ljava/math/BigDecimal;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_add(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "subtract",
            "(Ljava/math/BigDecimal;)Ljava/math/BigDecimal;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_subtract(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "multiply",
            "(Ljava/math/BigDecimal;)Ljava/math/BigDecimal;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_multiply(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "divide",
            "(Ljava/math/BigDecimal;)Ljava/math/BigDecimal;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_divide(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "sqrt",
            "(Ljava/math/MathContext;)Ljava/math/BigDecimal;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_sqrt(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "pow",
            "(ILjava/math/MathContext;)Ljava/math/BigDecimal;",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_pow(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/math/BigDecimal;",
            "compareTo",
            "(Ljava/math/BigDecimal;)I",
        ) {
            let invocation = invocation?;
            let value = self.big_decimal_compare_to(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        None
    }
}
