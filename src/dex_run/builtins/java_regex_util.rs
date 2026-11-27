use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_regex_util_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Ljava/util/regex/Pattern;"
            && method.name == "compile"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/util/regex/Pattern;")
        {
            let invocation = invocation?;
            let value = self.regex_pattern_compile(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: libcore Pattern.java#quote
        if method.matches_builtin_signature(
            "Ljava/util/regex/Pattern;",
            "quote",
            "(Ljava/lang/String;)Ljava/lang/String;",
        ) {
            let invocation = invocation?;
            let DexValue::String(value) = invocation.arguments.first()? else {
                return None;
            };

            let quoted = if value.contains("\\E") {
                let mut quoted = String::with_capacity(value.len() * 2);
                quoted.push_str("\\Q");
                let mut rest: &str = value.as_ref();
                while let Some(index) = rest.find("\\E") {
                    quoted.push_str(&rest[..index]);
                    quoted.push_str("\\E\\\\E\\Q");
                    rest = &rest[index + 2..];
                }
                quoted.push_str(rest);
                quoted.push_str("\\E");
                quoted
            } else {
                format!("\\Q{value}\\E")
            };

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(quoted),
            )));
        }

        if method.matches_builtin_signature(
            "Ljava/util/regex/Pattern;",
            "compile",
            "(Ljava/lang/String;I)Ljava/util/regex/Pattern;",
        ) {
            let invocation = invocation?;
            let value = self.regex_pattern_compile_with_flags(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: libcore Pattern.java#quote
        if method.matches_builtin_signature(
            "Ljava/util/regex/Pattern;",
            "quote",
            "(Ljava/lang/String;)Ljava/lang/String;",
        ) {
            let invocation = invocation?;
            if invocation.arguments.len() != 1 {
                return None;
            }
            let DexValue::String(text) = &invocation.arguments[0] else {
                return None;
            };
            let quoted = format!("\\Q{}\\E", text.replace("\\E", "\\E\\\\E\\Q"));

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(quoted),
            )));
        }

        if method.matches_builtin_signature(
            "Ljava/util/regex/Pattern;",
            "matcher",
            "(Ljava/lang/CharSequence;)Ljava/util/regex/Matcher;",
        ) {
            let invocation = invocation?;
            let value = self.regex_pattern_matcher(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature("Ljava/util/regex/Matcher;", "find", "()Z") {
            let invocation = invocation?;
            let value = self.regex_matcher_find(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature("Ljava/util/regex/Matcher;", "find", "(I)Z") {
            let invocation = invocation?;
            let value = self.regex_matcher_find_from(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/regex/Matcher;",
            "pattern",
            "()Ljava/util/regex/Pattern;",
        ) {
            let invocation = invocation?;
            let value = self.regex_matcher_pattern(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature("Ljava/util/regex/Matcher;", "matches", "()Z") {
            let invocation = invocation?;
            let value = self.regex_matcher_matches(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_any_builtin_signature(
            "Ljava/util/regex/Matcher;",
            "group",
            &["()Ljava/lang/String;", "(I)Ljava/lang/String;"],
        ) {
            let invocation = invocation?;
            let value = self.regex_matcher_group(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_any_builtin_signature(
            "Ljava/util/regex/Matcher;",
            "start",
            &["()I", "(I)I"],
        ) {
            let invocation = invocation?;
            let value = self.regex_matcher_start(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_any_builtin_signature(
            "Ljava/util/regex/Matcher;",
            "end",
            &["()I", "(I)I"],
        ) {
            let invocation = invocation?;
            let value = self.regex_matcher_end(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/regex/Matcher;",
            "replaceAll",
            "(Ljava/lang/String;)Ljava/lang/String;",
        ) {
            let invocation = invocation?;
            let value = self.regex_matcher_replace_all(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.matches_builtin_signature(
            "Ljava/util/Calendar;",
            "getInstance",
            "()Ljava/util/Calendar;",
        ) {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/Calendar;.bootstrap"),
            )));
        }

        if method.matches_builtin_signature("Ljava/util/Calendar;", "get", "(I)I") {
            let invocation = invocation?;
            let value = calendar_get_field(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature("Lo8/a;", "a", "()Ljava/util/Random;") {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/Random;.bootstrap"),
            )));
        }

        if method.matches_builtin_signature("LB1/a;", "a", "()Ljava/util/Random;") {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/Random;.bootstrap"),
            )));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ThreadLocalRandom;",
            "current",
            "()Ljava/util/concurrent/ThreadLocalRandom;",
        ) {
            // cite: libcore ThreadLocalRandom.java#current
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/Random;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Ljava/util/Random;"
            && method.name == "<init>"
            && matches!(method.proto.descriptor().as_str(), "()V" | "(J)V")
        {
            // cite: libcore Random.java#Random
            if let Some(invocation) = invocation {
                self.initialize_random(invocation).ok()??;
            }
            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature("Ljava/security/SecureRandom;", "<init>", "()V") {
            // cite: libcore SecureRandom.java#SecureRandom
            let invocation = invocation?;
            let mut seed = [0u8; 8];
            std::io::Read::read_exact(&mut std::fs::File::open("/dev/urandom").ok()?, &mut seed)
                .ok()?;
            let seeded = DexInvocation {
                arguments: smallvec::smallvec![
                    invocation.arguments.first()?.clone(),
                    DexValue::Long(i64::from_ne_bytes(seed)),
                ],
                ..invocation.clone()
            };
            self.initialize_random(&seeded).ok()??;
            return Some(DexExecutionResult::Void);
        }

        if (method.matches_builtin_signature("Ljava/util/Random;", "nextInt", "()I")
            || method.matches_builtin_signature("Ljava/security/SecureRandom;", "nextInt", "()I"))
        {
            let invocation = invocation?;
            // cite: libcore Random.java#nextInt, next
            let value = self.random_next_unbounded_int(invocation).ok()??;

            return Some(DexExecutionResult::Int(value));
        }

        if (method.matches_builtin_signature("Ljava/util/Random;", "nextFloat", "()F")
            || method.matches_builtin_signature("Ljava/security/SecureRandom;", "nextFloat", "()F"))
        {
            let invocation = invocation?;
            // cite: libcore Random.java#nextFloat, next
            let value = self.random_next_float(invocation).ok()??;

            return Some(DexExecutionResult::Value(DexValue::Float(value)));
        }

        if method.matches_builtin_signature("Ljava/util/Random;", "nextDouble", "()D")
            || method.matches_builtin_signature("Ljava/security/SecureRandom;", "nextDouble", "()D")
        {
            let invocation = invocation?;
            // cite: libcore Random.java#nextDouble, next
            let value = self.random_next_double(invocation).ok()??;

            return Some(DexExecutionResult::Value(DexValue::Long(
                value.to_bits() as i64
            )));
        }

        if (method.matches_builtin_signature("Ljava/util/Random;", "nextInt", "(I)I")
            || method.matches_builtin_signature("Ljava/security/SecureRandom;", "nextInt", "(I)I"))
        {
            let invocation = invocation?;
            // cite: libcore Random.java#nextInt, next
            let value = self.random_next_int(invocation).ok()??;

            return Some(DexExecutionResult::Int(value));
        }

        None
    }
}
