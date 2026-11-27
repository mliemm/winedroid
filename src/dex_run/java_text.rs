use super::*;

// cite: libcore Pattern.java#compile
thread_local! {
    static REGEX_CACHE: std::cell::RefCell<std::collections::HashMap<String, Option<std::rc::Rc<JavaRegex>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn cached_regex(pattern: &str) -> Option<std::rc::Rc<JavaRegex>> {
    REGEX_CACHE.with(|cache| {
        if let Some(entry) = cache.borrow().get(pattern) {
            return entry.clone();
        }
        let compiled = JavaRegex::new(pattern).map(std::rc::Rc::new);
        cache
            .borrow_mut()
            .insert(pattern.to_owned(), compiled.clone());
        compiled
    })
}

impl DexProgram {
    pub(super) fn break_iterator_get_line_instance(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let iterator = self.allocate_object("Ljava/text/BreakIterator;".to_owned());
        self.write_stored_instance_field(
            &iterator,
            &break_iterator_string_field("kind"),
            DexValue::String(std::rc::Rc::from("line")),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &iterator,
            &break_iterator_object_field("locale", "Ljava/util/Locale;"),
            invocation.arguments[0].clone(),
        )
        .then_some(())?;

        Some(iterator)
    }

    pub(super) fn break_iterator_set_character_iterator_text(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &break_iterator_object_field("text", "Ljava/text/CharacterIterator;"),
            invocation.arguments[1].clone(),
        )
        .then_some(())?;

        let boundaries = self.character_iterator_line_boundaries(&invocation.arguments[1])?;
        let boundary_array = self.allocate_filled_array(
            "[I".to_owned(),
            boundaries.into_iter().map(DexValue::Int).collect(),
        );
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &break_iterator_object_field("boundaries", "[I"),
            boundary_array,
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &break_iterator_int_field("boundaryIndex"),
            DexValue::Int(0),
        )
        .then_some(())
    }

    pub(super) fn break_iterator_next(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let boundary_array = self.read_stored_instance_field(
            &invocation.arguments[0],
            &break_iterator_object_field("boundaries", "[I"),
        )?;
        let boundary_index = match self
            .read_stored_instance_field(
                &invocation.arguments[0],
                &break_iterator_int_field("boundaryIndex"),
            )
            .unwrap_or(DexValue::Int(0))
        {
            DexValue::Int(value) => value,
            _ => return None,
        };
        let next_index = boundary_index.checked_add(1)?;
        let entries = self
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&boundary_array))?
            .clone();
        let Some(DexValue::Int(boundary)) = entries.get(next_index as usize) else {
            return Some(-1);
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &break_iterator_int_field("boundaryIndex"),
            DexValue::Int(next_index),
        )
        .then_some(*boundary)
    }

    fn character_iterator_line_boundaries(&self, iterator: &DexValue) -> Option<Vec<i32>> {
        let begin = self.character_iterator_int_method(iterator, "getBeginIndex")?;
        let end = self.character_iterator_int_method(iterator, "getEndIndex")?;
        if begin > end {
            return None;
        }

        let mut boundaries = vec![begin];
        let mut index = begin;
        while index < end {
            let unit = self.character_iterator_set_index(iterator, index)?;
            if is_line_break_opportunity(unit)
                && unit != '\r' as i32
                && !boundary_already_added(&boundaries, index + 1)
            {
                boundaries.push(index + 1);
            }

            if unit == '\r' as i32 {
                let next_index = index + 1;
                let boundary = if next_index < end
                    && self.character_iterator_set_index(iterator, next_index)? == '\n' as i32
                {
                    index + 2
                } else {
                    index + 1
                };
                if !boundary_already_added(&boundaries, boundary) {
                    boundaries.push(boundary);
                }
            }

            index += 1;
        }

        if !boundary_already_added(&boundaries, end) {
            boundaries.push(end);
        }

        Some(boundaries)
    }

    fn character_iterator_int_method(&self, iterator: &DexValue, method_name: &str) -> Option<i32> {
        let class_descriptor = value_class_descriptor(iterator)?;
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: method_name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        };
        let resolved = self
            .resolve_exact_method_in_class_hierarchy(class_descriptor, &method)
            .ok()??;
        let result = self
            .execute_method(None, &resolved, Some(&[iterator.clone()]), 0)
            .ok()?;

        match result {
            DexExecutionResult::Int(value) => Some(value),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                None
            }
            _ => None,
        }
    }

    fn character_iterator_set_index(&self, iterator: &DexValue, index: i32) -> Option<i32> {
        let class_descriptor = value_class_descriptor(iterator)?;
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "setIndex".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "CI".to_owned(),
                return_type: "C".to_owned(),
                parameters: vec!["I".to_owned()],
            },
        };
        let resolved = self
            .resolve_exact_method_in_class_hierarchy(class_descriptor, &method)
            .ok()??;
        let result = self
            .execute_method(
                None,
                &resolved,
                Some(&[iterator.clone(), DexValue::Int(index)]),
                0,
            )
            .ok()?;

        match result {
            DexExecutionResult::Int(value) => Some(value),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                None
            }
            _ => None,
        }
    }

    pub(super) fn regex_pattern_compile(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let DexValue::String(pattern) = &invocation.arguments[0] else {
            return None;
        };

        let effective_pattern = translate_java_regex_quoting(pattern);
        cached_regex(&effective_pattern)?;

        let pattern_object = self.allocate_object("Ljava/util/regex/Pattern;".to_owned());
        let pattern_key = map_identity_key(&pattern_object);

        self.runtime
            .borrow_mut()
            .regex_patterns
            .insert(pattern_key, effective_pattern);

        Some(pattern_object)
    }

    // cite: libcore Pattern.java#compile, CASE_INSENSITIVE, COMMENTS, MULTILINE, LITERAL, DOTALL, UNICODE_CASE
    pub(super) fn regex_pattern_compile_with_flags(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::String(pattern) = &invocation.arguments[0] else {
            return None;
        };
        let DexValue::Int(flags) = invocation.arguments[1] else {
            return None;
        };

        const CASE_INSENSITIVE: i32 = 0x02;
        const COMMENTS: i32 = 0x04;
        const MULTILINE: i32 = 0x08;
        const LITERAL: i32 = 0x10;
        const DOTALL: i32 = 0x20;
        const UNICODE_CASE: i32 = 0x40;
        if flags & !(CASE_INSENSITIVE | COMMENTS | MULTILINE | LITERAL | DOTALL | UNICODE_CASE) != 0
        {
            return None;
        }

        let mut inline_flags = String::new();
        if flags & CASE_INSENSITIVE != 0 {
            inline_flags.push('i');
        }
        if flags & COMMENTS != 0 {
            inline_flags.push('x');
        }
        if flags & MULTILINE != 0 {
            inline_flags.push('m');
        }
        if flags & DOTALL != 0 {
            inline_flags.push('s');
        }

        let mut effective_pattern = if flags & LITERAL != 0 {
            regex::escape(pattern)
        } else {
            translate_java_regex_quoting(pattern)
        };
        if !inline_flags.is_empty() {
            effective_pattern = format!("(?{inline_flags}){effective_pattern}");
        }

        cached_regex(&effective_pattern)?;

        let pattern_object = self.allocate_object("Ljava/util/regex/Pattern;".to_owned());
        let pattern_key = map_identity_key(&pattern_object);

        self.runtime
            .borrow_mut()
            .regex_patterns
            .insert(pattern_key, effective_pattern);

        Some(pattern_object)
    }

    pub(super) fn regex_pattern_matcher(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let pattern_key = map_identity_key(&invocation.arguments[0]);
        let pattern = self
            .runtime
            .borrow()
            .regex_patterns
            .get(&pattern_key)?
            .clone();
        let input = self.char_sequence_text(&invocation.arguments[1])?;
        let matcher = self.allocate_object("Ljava/util/regex/Matcher;".to_owned());
        let matcher_key = map_identity_key(&matcher);

        self.runtime.borrow_mut().regex_matchers.insert(
            matcher_key,
            RegexMatcherState {
                pattern,
                input,
                search_start: 0,
                last_groups: Vec::new(),
            },
        );

        Some(matcher)
    }

    pub(super) fn regex_matcher_find(&self, invocation: &DexInvocation) -> Option<i32> {
        let matcher_key = regex_matcher_key(invocation)?;
        let (pattern, input, search_start) = self.regex_matcher_search_snapshot(&matcher_key)?;
        self.regex_matcher_find_at_byte_index(&matcher_key, &pattern, &input, search_start)
    }

    pub(super) fn regex_matcher_find_from(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let matcher_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(start) = invocation.arguments[1] else {
            return None;
        };
        let (pattern, input, _search_start) = self.regex_matcher_search_snapshot(&matcher_key)?;
        let search_start = utf16_units_to_byte_index(&input, start)?;

        self.regex_matcher_find_at_byte_index(&matcher_key, &pattern, &input, search_start)
    }

    pub(super) fn regex_matcher_pattern(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let matcher_key = map_identity_key(&invocation.arguments[0]);
        let pattern = self
            .runtime
            .borrow()
            .regex_matchers
            .get(&matcher_key)?
            .pattern
            .clone();
        let pattern_object = self.allocate_object("Ljava/util/regex/Pattern;".to_owned());
        let pattern_key = map_identity_key(&pattern_object);

        self.runtime
            .borrow_mut()
            .regex_patterns
            .insert(pattern_key, pattern);

        Some(pattern_object)
    }

    fn regex_matcher_find_at_byte_index(
        &self,
        matcher_key: &str,
        pattern: &str,
        input: &str,
        search_start: usize,
    ) -> Option<i32> {
        let regex = cached_regex(&pattern)?;
        let captures = regex.captures_at(input, search_start)?;

        let Some(captures) = captures else {
            self.write_regex_matcher_no_match(&matcher_key, input.len())?;
            return Some(0);
        };
        let (_, full_match_end) = captures.first().copied().flatten()?;
        let groups = regex_capture_groups(&captures);
        let next_search_start = next_regex_search_start(&input, full_match_end);

        self.write_regex_matcher_match(&matcher_key, next_search_start, groups)?;

        Some(1)
    }

    // cite: libcore Matcher.java#matches
    pub(super) fn regex_matcher_matches(&self, invocation: &DexInvocation) -> Option<i32> {
        let matcher_key = regex_matcher_key(invocation)?;
        let (pattern, input, _search_start) = self.regex_matcher_search_snapshot(&matcher_key)?;
        let anchored_pattern = format!("^(?:{pattern})$");
        let regex = cached_regex(&anchored_pattern)?;
        let captures = regex.captures(&input)?;

        let Some(captures) = captures else {
            self.write_regex_matcher_no_match(&matcher_key, 0)?;
            return Some(0);
        };

        let groups = regex_capture_groups(&captures);
        self.write_regex_matcher_match(&matcher_key, input.len(), groups)?;

        Some(1)
    }

    pub(super) fn regex_matcher_group(&self, invocation: &DexInvocation) -> Option<DexValue> {
        let (matcher_key, group_index) = regex_matcher_group_key(invocation)?;
        let runtime = self.runtime.borrow();
        let state = runtime.regex_matchers.get(&matcher_key)?;
        let range = state.last_groups.get(group_index)?.as_ref()?;
        let value = state.input.get(range.start..range.end)?;

        Some(DexValue::String(std::rc::Rc::from(value.to_owned())))
    }

    pub(super) fn regex_matcher_start(&self, invocation: &DexInvocation) -> Option<i32> {
        let (matcher_key, group_index) = regex_matcher_group_key(invocation)?;
        let runtime = self.runtime.borrow();
        let state = runtime.regex_matchers.get(&matcher_key)?;
        let range = state.last_groups.get(group_index)?.as_ref()?;

        utf16_units_before_byte(&state.input, range.start)
    }

    pub(super) fn regex_matcher_end(&self, invocation: &DexInvocation) -> Option<i32> {
        let (matcher_key, group_index) = regex_matcher_group_key(invocation)?;
        let runtime = self.runtime.borrow();
        let state = runtime.regex_matchers.get(&matcher_key)?;
        let range = state.last_groups.get(group_index)?.as_ref()?;

        utf16_units_before_byte(&state.input, range.end)
    }

    pub(super) fn regex_matcher_replace_all(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let matcher_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::String(replacement) = &invocation.arguments[1] else {
            return None;
        };
        let runtime = self.runtime.borrow();
        let state = runtime.regex_matchers.get(&matcher_key)?;
        let regex = cached_regex(&state.pattern)?;

        regex.replace_all(&state.input, replacement.as_ref())
    }

    pub(super) fn regex_matcher_search_snapshot(
        &self,
        matcher_key: &str,
    ) -> Option<(String, String, usize)> {
        let runtime = self.runtime.borrow();
        let state = runtime.regex_matchers.get(matcher_key)?;

        Some((
            state.pattern.clone(),
            state.input.clone(),
            state.search_start,
        ))
    }

    pub(super) fn write_regex_matcher_no_match(
        &self,
        matcher_key: &str,
        search_start: usize,
    ) -> Option<()> {
        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.regex_matchers.get_mut(matcher_key)?;

        state.search_start = search_start;
        state.last_groups.clear();

        Some(())
    }

    pub(super) fn write_regex_matcher_match(
        &self,
        matcher_key: &str,
        search_start: usize,
        groups: Vec<Option<RegexMatchRange>>,
    ) -> Option<()> {
        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.regex_matchers.get_mut(matcher_key)?;

        state.search_start = search_start;
        state.last_groups = groups;

        Some(())
    }

    pub(super) fn array_length(&self, array: &DexValue) -> Option<i32> {
        let array_key = map_identity_key(array);
        let length = self.runtime.borrow().array_entries.get(&array_key)?.len();

        i32::try_from(length).ok()
    }

    pub(super) fn read_array_value(&self, array: &DexValue, index: i32) -> Option<DexValue> {
        if index < 0 {
            return None;
        }

        let array_key = map_identity_key(array);
        self.runtime
            .borrow()
            .array_entries
            .get(&array_key)?
            .get(index as usize)
            .cloned()
    }

    pub(super) fn write_array_value(
        &self,
        array: &DexValue,
        index: i32,
        value: DexValue,
    ) -> Option<()> {
        if index < 0 {
            return None;
        }

        let array_key = map_identity_key(array);
        let class_descriptor = value_class_descriptor(array).unwrap_or("");
        let value = normalize_array_value_for_descriptor(class_descriptor, value)?;
        let mut runtime = self.runtime.borrow_mut();
        let values = runtime.array_entries.get_mut(&array_key)?;
        let slot = values.get_mut(index as usize)?;

        *slot = value;

        Some(())
    }

    pub(super) fn system_arraycopy(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 5 {
            return None;
        }

        let source_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(source_position) = invocation.arguments[1] else {
            return None;
        };
        let target_key = map_identity_key(&invocation.arguments[2]);
        let target_descriptor = value_class_descriptor(&invocation.arguments[2]).unwrap_or("");
        let DexValue::Int(target_position) = invocation.arguments[3] else {
            return None;
        };
        let DexValue::Int(length) = invocation.arguments[4] else {
            return None;
        };

        let source_position = usize::try_from(source_position).ok()?;
        let target_position = usize::try_from(target_position).ok()?;
        let length = usize::try_from(length).ok()?;
        let source_end = source_position.checked_add(length)?;
        let target_end = target_position.checked_add(length)?;

        let copied_values = {
            let runtime = self.runtime.borrow();
            let source_values = runtime.array_entries.get(&source_key)?;
            if source_end > source_values.len() {
                return None;
            }

            source_values[source_position..source_end].to_vec()
        };

        let mut runtime = self.runtime.borrow_mut();
        let target_values = runtime.array_entries.get_mut(&target_key)?;
        if target_end > target_values.len() {
            return None;
        }

        for (index, value) in copied_values.into_iter().enumerate() {
            let value = normalize_array_value_for_descriptor(target_descriptor, value)?;
            target_values[target_position + index] = value;
        }

        Some(())
    }

    pub(super) fn initialize_string_from_char_array(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(object)
                if object.class_descriptor == std::rc::Rc::from("Ljava/lang/String;") => {}
            DexValue::Reference(reference)
                if reference_descriptor(reference) == Some("Ljava/lang/String;") => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_)
            | DexValue::Object(_)
            | DexValue::Reference(_) => return None,
        }

        let text = self.char_array_string(
            &invocation.arguments[1],
            invocation.arguments[2].clone(),
            invocation.arguments[3].clone(),
        )?;
        let key = map_identity_key(&invocation.arguments[0]);

        self.runtime.borrow_mut().string_values.insert(key, text);

        Some(())
    }

    pub(super) fn initialize_string_from_byte_array(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        // cite: libcore String.java#String, StringCoding
        // cite: libcore Charset.java#defaultCharset
        let charset = match invocation.arguments.get(2) {
            None if invocation.arguments.len() == 2 => "UTF-8".to_owned(),
            Some(name) if invocation.arguments.len() == 3 => self.string_text(name)?,
            _ => return None,
        };
        let bytes = self.byte_array_contents(&invocation.arguments[1])?;
        let text = decode_charset_bytes(&bytes, &charset)?;
        let key = map_identity_key(&invocation.arguments[0]);

        self.runtime.borrow_mut().string_values.insert(key, text);

        Some(())
    }

    // cite: libcore String.java#getBytes
    pub(super) fn string_get_bytes(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let text = self.string_text(invocation.arguments.first()?)?;
        let charset = match (
            method.proto.descriptor().as_str(),
            invocation.arguments.get(1),
        ) {
            ("()[B", None) => "UTF-8".to_owned(),
            ("(Ljava/lang/String;)[B", Some(name)) => self.string_text(name)?,
            ("(Ljava/nio/charset/Charset;)[B", Some(DexValue::Reference(reference))) => reference
                .strip_prefix("Ljava/nio/charset/Charset;:")?
                .to_owned(),
            (_, Some(DexValue::Null | DexValue::Int(0))) => {
                return Some(self.null_pointer_exception_result());
            }
            _ => return None,
        };
        let Some(bytes) = encode_charset_text(&text, &charset) else {
            if method.proto.descriptor() == "(Ljava/lang/String;)[B" {
                return Some(DexExecutionResult::Throw(self.allocate_object(
                    "Ljava/io/UnsupportedEncodingException;".to_owned(),
                )));
            }
            return None;
        };
        let values = bytes
            .into_iter()
            .map(|byte| DexValue::Int(i32::from(byte as i8)))
            .collect();

        Some(DexExecutionResult::Value(
            self.allocate_filled_array("[B".to_owned(), values),
        ))
    }

    pub(super) fn byte_array_contents(&self, value: &DexValue) -> Option<Vec<u8>> {
        let key = map_identity_key(value);
        let runtime = self.runtime.borrow();
        let entries = runtime.array_entries.get(&key)?;
        entries
            .iter()
            .map(|entry| match entry {
                DexValue::Int(value) => Some(*value as u8),
                _ => None,
            })
            .collect()
    }

    pub(super) fn initialize_string_from_full_char_array(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(object)
                if object.class_descriptor == std::rc::Rc::from("Ljava/lang/String;") => {}
            DexValue::Reference(reference)
                if reference_descriptor(reference) == Some("Ljava/lang/String;") => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_)
            | DexValue::Object(_)
            | DexValue::Reference(_) => return None,
        }

        let array_key = map_identity_key(&invocation.arguments[1]);
        let count = self.runtime.borrow().array_entries.get(&array_key)?.len();
        let count = i32::try_from(count).ok()?;
        let text = self.char_array_string(
            &invocation.arguments[1],
            DexValue::Int(0),
            DexValue::Int(count),
        )?;
        let key = map_identity_key(&invocation.arguments[0]);

        self.runtime.borrow_mut().string_values.insert(key, text);

        Some(())
    }

    pub(super) fn initialize_spannable_string(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(object)
                if object.class_descriptor
                    == std::rc::Rc::from("Landroid/text/SpannableString;") => {}
            DexValue::Reference(reference)
                if reference_descriptor(reference) == Some("Landroid/text/SpannableString;") => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_)
            | DexValue::Object(_)
            | DexValue::Reference(_) => return None,
        }

        let text = self.char_sequence_text(&invocation.arguments[1])?;
        let key = map_identity_key(&invocation.arguments[0]);

        self.runtime.borrow_mut().string_values.insert(key, text);

        Some(())
    }

    pub(super) fn spannable_set_span(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 5 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        let DexValue::Int(start) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Int(end) = invocation.arguments[3] else {
            return None;
        };
        let DexValue::Int(flags) = invocation.arguments[4] else {
            return None;
        };
        let length = i32::try_from(
            self.char_sequence_text(&invocation.arguments[0])?
                .encode_utf16()
                .count(),
        )
        .ok()?;
        if start < 0 || end < start || end > length {
            return None;
        }

        let key = map_identity_key(&invocation.arguments[0]);
        self.runtime
            .borrow_mut()
            .spannable_spans
            .entry(key)
            .or_default()
            .push(SpannableSpanState {
                value: invocation.arguments[1].clone(),
                start,
                end,
                flags,
            });

        Some(())
    }

    pub(super) fn initialize_foreground_color_span(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(color) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &foreground_color_span_color_field(),
            DexValue::Int(color),
        )
        .then_some(())
    }

    pub(super) fn foreground_color_span_color(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        match self.read_stored_instance_field(
            &invocation.arguments[0],
            &foreground_color_span_color_field(),
        )? {
            DexValue::Int(color) => Some(color),
            _ => None,
        }
    }

    pub(super) fn foreground_color_span_update_draw_state(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2
            || !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }
        let color = match self.read_stored_instance_field(
            &invocation.arguments[0],
            &foreground_color_span_color_field(),
        )? {
            DexValue::Int(color) => color,
            _ => return None,
        };

        self.write_stored_instance_field(
            &invocation.arguments[1],
            &paint_color_field(),
            DexValue::Int(color),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[1],
            &paint_alpha_field(),
            DexValue::Int(((color as u32) >> 24) as i32),
        )
        .then_some(())?;
        self.remove_stored_instance_field(&invocation.arguments[1], &paint_color_long_field());

        Some(())
    }

    pub(super) fn initialize_background_color_span(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(color) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &background_color_span_color_field(),
            DexValue::Int(color),
        )
        .then_some(())
    }

    pub(super) fn background_color_span_color(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        match self.read_stored_instance_field(
            &invocation.arguments[0],
            &background_color_span_color_field(),
        )? {
            DexValue::Int(color) => Some(color),
            _ => None,
        }
    }

    pub(super) fn background_color_span_update_draw_state(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2
            || !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }
        let color = match self.read_stored_instance_field(
            &invocation.arguments[0],
            &background_color_span_color_field(),
        )? {
            DexValue::Int(color) => color,
            _ => return None,
        };

        self.write_stored_instance_field(
            &invocation.arguments[1],
            &text_paint_background_color_field(),
            DexValue::Int(color),
        )
        .then_some(())
    }

    pub(super) fn boring_layout_is_boring(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 5 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[1]) {
            return None;
        }

        if !dex_value_is_nullable_object_like(&invocation.arguments[2])
            || !dex_value_is_nullable_object_like(&invocation.arguments[4])
        {
            return None;
        }

        let DexValue::Int(_) = invocation.arguments[3] else {
            return None;
        };
        let text = self.char_sequence_text(&invocation.arguments[0])?;
        if text.contains('\n') || text.contains('\t') {
            return Some(DexValue::Null);
        }

        let metrics = match &invocation.arguments[4] {
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[4].clone(),
            DexValue::Null | DexValue::Int(0) => {
                self.allocate_object("Landroid/text/BoringLayout$Metrics;".to_owned())
            }
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };
        let size = self
            .paint_text_size(&invocation.arguments[1])
            .unwrap_or(16.0);
        let text_size = size.max(0.0);
        let width = (text.encode_utf16().count() as f32 * text_size * 0.5).ceil() as i32;
        let ascent = -(text_size * 0.8).ceil() as i32;
        let descent = (text_size * 0.2).ceil() as i32;
        let top = -(text_size * 0.95).ceil() as i32;
        let bottom = (text_size * 0.25).ceil() as i32;

        for (field, value) in [
            (boring_layout_metrics_width_field(), width),
            (paint_font_metrics_int_field("top"), top),
            (paint_font_metrics_int_field("ascent"), ascent),
            (paint_font_metrics_int_field("descent"), descent),
            (paint_font_metrics_int_field("bottom"), bottom),
            (paint_font_metrics_int_field("leading"), 0),
        ] {
            self.write_stored_instance_field(&metrics, &field, DexValue::Int(value))
                .then_some(())?;
        }

        Some(metrics)
    }

    pub(super) fn initialize_boring_layout(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 12 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[2])
            || !dex_value_is_object_like(&invocation.arguments[4])
            || !dex_value_is_nullable_object_like(&invocation.arguments[7])
            || !dex_value_is_nullable_object_like(&invocation.arguments[9])
        {
            return None;
        }

        let _text = self.char_sequence_text(&invocation.arguments[1])?;
        let DexValue::Int(width) = invocation.arguments[3] else {
            return None;
        };
        let spacing_mult = read_float_argument(&invocation.arguments[5])?;
        let spacing_add = read_float_argument(&invocation.arguments[6])?;
        let DexValue::Int(include_pad) = invocation.arguments[8] else {
            return None;
        };
        let DexValue::Int(ellipsized_width) = invocation.arguments[10] else {
            return None;
        };
        let DexValue::Int(trust_width) = invocation.arguments[11] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_layout_char_sequence_field("text"),
            invocation.arguments[1].clone(),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_layout_paint_field(),
            invocation.arguments[2].clone(),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_layout_int_field("width"),
            DexValue::Int(width),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_layout_alignment_field(),
            invocation.arguments[4].clone(),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_layout_float_field("spacingMult"),
            DexValue::Float(spacing_mult),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_layout_float_field("spacingAdd"),
            DexValue::Float(spacing_add),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &boring_layout_metrics_field(),
            normalize_nullable_object(&invocation.arguments[7])?,
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &boring_layout_int_field("includePad"),
            DexValue::Int(include_pad),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &boring_layout_truncate_at_field(),
            normalize_nullable_object(&invocation.arguments[9])?,
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &boring_layout_int_field("ellipsizedWidth"),
            DexValue::Int(ellipsized_width),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &boring_layout_int_field("trustWidth"),
            DexValue::Int(trust_width),
        )
        .then_some(())?;

        let height = boring_layout_metrics_height(self, &invocation.arguments[7]).unwrap_or(0);
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &boring_layout_int_field("height"),
            DexValue::Int(height),
        )
        .then_some(())
    }

    pub(super) fn static_layout_builder_obtain(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 5 {
            return None;
        }

        if matches!(invocation.arguments[0], DexValue::Null | DexValue::Int(0))
            || !dex_value_is_object_like(&invocation.arguments[3])
        {
            return None;
        }

        let text = self.char_sequence_text(&invocation.arguments[0])?;
        let DexValue::Int(start) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(end) = invocation.arguments[2] else {
            return None;
        };
        let text = utf16_string_slice(&text, start, end)?;
        let DexValue::Int(width) = invocation.arguments[4] else {
            return None;
        };

        let builder = self.allocate_object("Landroid/text/StaticLayout$Builder;".to_owned());
        let alignment = DexValue::Reference(std::rc::Rc::from(
            "Landroid/text/Layout$Alignment;.ALIGN_NORMAL",
        ));

        for (field, value) in [
            (
                static_layout_builder_char_sequence_field("text"),
                DexValue::String(std::rc::Rc::from(text)),
            ),
            (
                static_layout_builder_int_field("start"),
                DexValue::Int(start),
            ),
            (static_layout_builder_int_field("end"), DexValue::Int(end)),
            (
                static_layout_builder_int_field("width"),
                DexValue::Int(width),
            ),
            (
                static_layout_builder_object_field("alignment", "Landroid/text/Layout$Alignment;"),
                alignment,
            ),
            (
                static_layout_builder_int_field("includePad"),
                DexValue::Int(1),
            ),
            (
                static_layout_builder_float_field("spacingMult"),
                DexValue::Float(1.0),
            ),
            (
                static_layout_builder_float_field("spacingAdd"),
                DexValue::Float(0.0),
            ),
            (
                static_layout_builder_int_field("ellipsizedWidth"),
                DexValue::Int(width),
            ),
            (
                static_layout_builder_int_field("maxLines"),
                DexValue::Int(i32::MAX),
            ),
        ] {
            self.write_stored_instance_field(&builder, &field, value)
                .then_some(())?;
        }

        self.write_stored_instance_field(
            &builder,
            &static_layout_builder_object_field("paint", "Landroid/text/TextPaint;"),
            invocation.arguments[3].clone(),
        )
        .then_some(())?;

        Some(builder)
    }

    pub(super) fn static_layout_builder_set_object(
        &self,
        invocation: &DexInvocation,
        field_name: &str,
        type_descriptor: &str,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &static_layout_builder_object_field(field_name, type_descriptor),
            invocation.arguments[1].clone(),
        )
        .then_some(invocation.arguments[0].clone())
    }

    pub(super) fn static_layout_builder_set_nullable_object(
        &self,
        invocation: &DexInvocation,
        field_name: &str,
        type_descriptor: &str,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_nullable_object_like(&invocation.arguments[1])
        {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &static_layout_builder_object_field(field_name, type_descriptor),
            normalize_nullable_object(&invocation.arguments[1])?,
        )
        .then_some(invocation.arguments[0].clone())
    }

    pub(super) fn static_layout_builder_set_int(
        &self,
        invocation: &DexInvocation,
        field_name: &str,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &static_layout_builder_int_field(field_name),
            DexValue::Int(value),
        )
        .then_some(invocation.arguments[0].clone())
    }

    pub(super) fn static_layout_builder_set_bool(
        &self,
        invocation: &DexInvocation,
        field_name: &str,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };
        if value != 0 && value != 1 {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &static_layout_builder_int_field(field_name),
            DexValue::Int(value),
        )
        .then_some(invocation.arguments[0].clone())
    }

    pub(super) fn static_layout_builder_set_line_spacing(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let spacing_add = read_float_argument(&invocation.arguments[1])?;
        let spacing_mult = read_float_argument(&invocation.arguments[2])?;

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &static_layout_builder_float_field("spacingAdd"),
            DexValue::Float(spacing_add),
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &static_layout_builder_float_field("spacingMult"),
            DexValue::Float(spacing_mult),
        )
        .then_some(invocation.arguments[0].clone())
    }

    pub(super) fn static_layout_builder_set_indents(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_nullable_object_like(&invocation.arguments[1])
            || !dex_value_is_nullable_object_like(&invocation.arguments[2])
        {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &static_layout_builder_object_field("leftIndents", "[I"),
            normalize_nullable_object(&invocation.arguments[1])?,
        )
        .then_some(())?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &static_layout_builder_object_field("rightIndents", "[I"),
            normalize_nullable_object(&invocation.arguments[2])?,
        )
        .then_some(invocation.arguments[0].clone())
    }

    pub(super) fn static_layout_builder_build(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let builder = &invocation.arguments[0];
        let text = self
            .read_stored_instance_field(builder, &static_layout_builder_char_sequence_field("text"))
            .unwrap_or_else(|| DexValue::String(std::rc::Rc::from(String::new())));
        let text_string = self.char_sequence_text(&text)?;
        let paint = self
            .read_stored_instance_field(
                builder,
                &static_layout_builder_object_field("paint", "Landroid/text/TextPaint;"),
            )
            .filter(dex_value_is_object_like)?;
        let width = self
            .read_stored_instance_field(builder, &static_layout_builder_int_field("width"))
            .and_then(|value| match value {
                DexValue::Int(value) => Some(value),
                _ => None,
            })?;
        let alignment = self
            .read_stored_instance_field(
                builder,
                &static_layout_builder_object_field("alignment", "Landroid/text/Layout$Alignment;"),
            )
            .unwrap_or_else(|| {
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/text/Layout$Alignment;.ALIGN_NORMAL",
                ))
            });
        let spacing_mult = self
            .read_stored_instance_field(builder, &static_layout_builder_float_field("spacingMult"))
            .and_then(|value| match value {
                DexValue::Float(value) => Some(value),
                _ => None,
            })
            .unwrap_or(1.0);
        let spacing_add = self
            .read_stored_instance_field(builder, &static_layout_builder_float_field("spacingAdd"))
            .and_then(|value| match value {
                DexValue::Float(value) => Some(value),
                _ => None,
            })
            .unwrap_or(0.0);
        let include_pad = self
            .read_stored_instance_field(builder, &static_layout_builder_int_field("includePad"))
            .and_then(|value| match value {
                DexValue::Int(value) => Some(value),
                _ => None,
            })
            .unwrap_or(1);
        let ellipsized_width = self
            .read_stored_instance_field(
                builder,
                &static_layout_builder_int_field("ellipsizedWidth"),
            )
            .and_then(|value| match value {
                DexValue::Int(value) => Some(value),
                _ => None,
            })
            .unwrap_or(width);
        let ellipsize = self
            .read_stored_instance_field(
                builder,
                &static_layout_builder_object_field(
                    "ellipsize",
                    "Landroid/text/TextUtils$TruncateAt;",
                ),
            )
            .unwrap_or(DexValue::Null);

        let layout = self.allocate_object("Landroid/text/StaticLayout;".to_owned());
        for (field, value) in [
            (text_layout_char_sequence_field("text"), text),
            (text_layout_paint_field(), paint.clone()),
            (text_layout_int_field("width"), DexValue::Int(width)),
            (text_layout_alignment_field(), alignment),
            (
                text_layout_float_field("spacingMult"),
                DexValue::Float(spacing_mult),
            ),
            (
                text_layout_float_field("spacingAdd"),
                DexValue::Float(spacing_add),
            ),
            (
                boring_layout_int_field("includePad"),
                DexValue::Int(include_pad),
            ),
            (boring_layout_truncate_at_field(), ellipsize),
            (
                boring_layout_int_field("ellipsizedWidth"),
                DexValue::Int(ellipsized_width),
            ),
            (boring_layout_int_field("trustWidth"), DexValue::Int(0)),
        ] {
            self.write_stored_instance_field(&layout, &field, value)
                .then_some(())?;
        }

        for field_name in [
            "maxLines",
            "breakStrategy",
            "hyphenationFrequency",
            "justificationMode",
            "useLineSpacingFromFallbacks",
            "useBoundsForWidth",
        ] {
            if let Some(value) = self
                .read_stored_instance_field(builder, &static_layout_builder_int_field(field_name))
            {
                self.write_stored_instance_field(
                    &layout,
                    &text_layout_int_field(field_name),
                    value,
                )
                .then_some(())?;
            }
        }

        for (field_name, type_descriptor) in [
            ("leftIndents", "[I"),
            ("rightIndents", "[I"),
            ("lineBreakConfig", "Landroid/graphics/text/LineBreakConfig;"),
        ] {
            if let Some(value) = self.read_stored_instance_field(
                builder,
                &static_layout_builder_object_field(field_name, type_descriptor),
            ) {
                self.write_stored_instance_field(
                    &layout,
                    &text_layout_object_field(field_name, type_descriptor),
                    value,
                )
                .then_some(())?;
            }
        }

        let metrics = self.allocate_object("Landroid/text/BoringLayout$Metrics;".to_owned());
        let text_size = self.paint_text_size(&paint).unwrap_or(16.0).max(0.0);
        let width = self
            .text_layout_measure_desired_width(&text_string, &paint)?
            .ceil() as i32;
        let top = -(text_size * 0.95).ceil() as i32;
        let ascent = -(text_size * 0.8).ceil() as i32;
        let descent = (text_size * 0.2).ceil() as i32;
        let bottom = (text_size * 0.25).ceil() as i32;
        for (field, value) in [
            (boring_layout_metrics_width_field(), width),
            (paint_font_metrics_int_field("top"), top),
            (paint_font_metrics_int_field("ascent"), ascent),
            (paint_font_metrics_int_field("descent"), descent),
            (paint_font_metrics_int_field("bottom"), bottom),
            (paint_font_metrics_int_field("leading"), 0),
        ] {
            self.write_stored_instance_field(&metrics, &field, DexValue::Int(value))
                .then_some(())?;
        }

        let line_count = i32::try_from(
            text_layout_line_starts(&text_string)
                .len()
                .saturating_sub(1),
        )
        .ok()?;
        let metric_height = bottom.saturating_sub(top).max(0);
        let line_height = ((metric_height as f32 * spacing_mult) + spacing_add)
            .ceil()
            .max(0.0) as i32;
        let height = line_count.max(1).saturating_mul(line_height);
        self.write_stored_instance_field(&layout, &boring_layout_metrics_field(), metrics)
            .then_some(())?;
        self.write_stored_instance_field(
            &layout,
            &boring_layout_int_field("height"),
            DexValue::Int(height),
        )
        .then_some(())?;

        Some(layout)
    }

    pub(super) fn initialize_line_break_config_builder(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        for field_name in ["lineBreakStyle", "lineBreakWordStyle", "hyphenation"] {
            self.write_stored_instance_field(
                &invocation.arguments[0],
                &line_break_config_builder_int_field(field_name),
                DexValue::Int(0),
            )
            .then_some(())?;
        }

        Some(())
    }

    pub(super) fn line_break_config_builder_set_int(
        &self,
        invocation: &DexInvocation,
        field_name: &str,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &line_break_config_builder_int_field(field_name),
            DexValue::Int(value),
        )
        .then_some(invocation.arguments[0].clone())
    }

    pub(super) fn line_break_config_builder_build(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let config = self.allocate_object("Landroid/graphics/text/LineBreakConfig;".to_owned());
        for field_name in ["lineBreakStyle", "lineBreakWordStyle", "hyphenation"] {
            let value = self
                .read_stored_instance_field(
                    &invocation.arguments[0],
                    &line_break_config_builder_int_field(field_name),
                )
                .unwrap_or(DexValue::Int(0));
            let DexValue::Int(value) = value else {
                return None;
            };
            self.write_stored_instance_field(
                &config,
                &line_break_config_int_field(field_name),
                DexValue::Int(value),
            )
            .then_some(())?;
        }

        Some(config)
    }

    pub(super) fn layout_alignment_values(&self) -> DexValue {
        self.allocate_filled_array(
            "[Landroid/text/Layout$Alignment;".to_owned(),
            layout_alignment_names()
                .iter()
                .map(|name| {
                    DexValue::Reference(std::rc::Rc::from(format!(
                        "Landroid/text/Layout$Alignment;.{name}"
                    )))
                })
                .collect(),
        )
    }

    pub(super) fn text_layout_get_line_count(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let text = self
            .read_stored_instance_field(
                &invocation.arguments[0],
                &text_layout_char_sequence_field("text"),
            )
            .and_then(|value| self.char_sequence_text(&value))
            .unwrap_or_default();
        let line_count = text
            .encode_utf16()
            .filter(|unit| *unit == b'\n' as u16)
            .count()
            + 1;

        i32::try_from(line_count).ok()
    }

    pub(super) fn text_layout_get_desired_width(&self, invocation: &DexInvocation) -> Option<f32> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        if matches!(invocation.arguments[0], DexValue::Null | DexValue::Int(0))
            || !dex_value_is_object_like(&invocation.arguments[3])
        {
            return None;
        }

        let text = self.char_sequence_text(&invocation.arguments[0])?;
        let DexValue::Int(start) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(end) = invocation.arguments[2] else {
            return None;
        };
        let text = utf16_string_slice(&text, start, end)?;

        self.text_layout_measure_desired_width(&text, &invocation.arguments[3])
    }

    pub(super) fn text_layout_get_line_start(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        if line < 0 {
            return None;
        }

        let text = self.text_layout_text(&invocation.arguments[0])?;
        let starts = text_layout_line_starts(&text);
        let line = usize::try_from(line).ok()?;
        starts.get(line).copied()
    }

    pub(super) fn text_layout_get_line_end(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        if line < 0 {
            return None;
        }

        let text = self.text_layout_text(&invocation.arguments[0])?;
        let starts = text_layout_line_starts(&text);
        let line = usize::try_from(line).ok()?;
        starts.get(line + 1).copied()
    }

    pub(super) fn text_layout_get_line_for_offset(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(offset) = invocation.arguments[1] else {
            return None;
        };
        if offset < 0 {
            return None;
        }

        let text = self.text_layout_text(&invocation.arguments[0])?;
        let text_length = i32::try_from(text.encode_utf16().count()).ok()?;
        if offset > text_length {
            return None;
        }

        i32::try_from(self.text_layout_line_for_offset(&text, offset)?).ok()
    }

    pub(super) fn text_layout_get_line_for_vertical(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(vertical) = invocation.arguments[1] else {
            return None;
        };
        let line_count = self.text_layout_line_count_for_layout(&invocation.arguments[0])?;
        if line_count <= 1 {
            return Some(0);
        }

        let line_height = self.text_layout_line_height(&invocation.arguments[0], line_count)?;
        if line_height <= 0 {
            return Some(0);
        }

        let mut low = -1;
        let mut high = line_count;
        while high - low > 1 {
            let guess = (high + low) / 2;
            let line_top = guess.checked_mul(line_height)?;
            if line_top > vertical {
                high = guess;
            } else {
                low = guess;
            }
        }

        Some(low.max(0).min(line_count - 1))
    }

    pub(super) fn text_layout_get_line_metric(
        &self,
        invocation: &DexInvocation,
        metric_name: &str,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        if line < 0 || line >= self.text_layout_line_count_for_layout(&invocation.arguments[0])? {
            return None;
        }

        if let Some(metrics) = self
            .read_stored_instance_field(&invocation.arguments[0], &boring_layout_metrics_field())
        {
            if let Some(value) = self
                .read_stored_instance_field(&metrics, &paint_font_metrics_int_field(metric_name))
            {
                let DexValue::Int(metric) = value else {
                    return None;
                };
                return Some(metric);
            }
        }

        let text_size = self
            .read_stored_instance_field(&invocation.arguments[0], &text_layout_paint_field())
            .and_then(|paint| self.paint_text_size(&paint))
            .unwrap_or(16.0)
            .max(0.0);
        match metric_name {
            "ascent" => Some(-(text_size * 0.8).ceil() as i32),
            "descent" => Some((text_size * 0.2).ceil() as i32),
            "top" => Some(-(text_size * 0.95).ceil() as i32),
            "bottom" => Some((text_size * 0.25).ceil() as i32),
            _ => None,
        }
    }

    pub(super) fn text_layout_get_line_left(&self, invocation: &DexInvocation) -> Option<f32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        if line < 0 || line >= self.text_layout_line_count_for_layout(&invocation.arguments[0])? {
            return None;
        }

        Some(0.0)
    }

    pub(super) fn text_layout_get_primary_horizontal(
        &self,
        invocation: &DexInvocation,
    ) -> Option<f32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(offset) = invocation.arguments[1] else {
            return None;
        };
        if offset < 0 {
            return None;
        }

        let text = self.text_layout_text(&invocation.arguments[0])?;
        let text_length = i32::try_from(text.encode_utf16().count()).ok()?;
        if offset > text_length {
            return None;
        }

        let line = self.text_layout_line_for_offset(&text, offset)?;
        let starts = text_layout_line_starts(&text);
        let line_start = starts.get(line).copied()?;
        let text = utf16_string_slice(&text, line_start, offset)?;
        let paint =
            self.read_stored_instance_field(&invocation.arguments[0], &text_layout_paint_field())?;

        self.text_layout_measure_desired_width(&text, &paint)
    }

    pub(super) fn text_layout_get_offset_for_horizontal(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        let horizontal = read_float_argument(&invocation.arguments[2])?;
        let line_count = self.text_layout_line_count_for_layout(&invocation.arguments[0])?;
        if line < 0 || line >= line_count {
            return None;
        }

        let text = self.text_layout_text(&invocation.arguments[0])?;
        let starts = text_layout_line_starts(&text);
        let line_index = usize::try_from(line).ok()?;
        let line_start = starts.get(line_index).copied()?;
        let line_end = starts.get(line_index + 1).copied()?;
        let line_end = text_layout_line_visible_end(&text, line_start, line_end)?;
        if horizontal <= 0.0 || line_start >= line_end {
            return Some(line_start);
        }

        let paint =
            self.read_stored_instance_field(&invocation.arguments[0], &text_layout_paint_field())?;
        let mut best_offset = line_start;
        let mut best_distance = f32::INFINITY;
        for offset in line_start..=line_end {
            let text = utf16_string_slice(&text, line_start, offset)?;
            let measured = self.text_layout_measure_desired_width(&text, &paint)?;
            let distance = (measured - horizontal).abs();
            if distance < best_distance {
                best_distance = distance;
                best_offset = offset;
            }
        }

        Some(best_offset)
    }

    pub(super) fn text_layout_get_ellipsis_count(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        if line < 0 || line >= self.text_layout_line_count_for_layout(&invocation.arguments[0])? {
            return None;
        }

        Some(0)
    }

    pub(super) fn text_layout_get_height(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = self.read_stored_instance_field(
            &invocation.arguments[0],
            &boring_layout_int_field("height"),
        )?;
        let DexValue::Int(height) = value else {
            return None;
        };

        Some(height)
    }

    pub(super) fn text_layout_get_line_baseline(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        let line_count = self.text_layout_line_count_for_layout(&invocation.arguments[0])?;
        if line < 0 || line >= line_count {
            return None;
        }

        let include_pad = self
            .read_stored_instance_field(
                &invocation.arguments[0],
                &boring_layout_int_field("includePad"),
            )
            .and_then(|value| match value {
                DexValue::Int(value) => Some(value != 0),
                _ => None,
            })
            .unwrap_or(false);
        let metric_name = if include_pad { "top" } else { "ascent" };
        let baseline_metric =
            self.text_layout_metric_from_layout(&invocation.arguments[0], metric_name)?;
        let line_height = self.text_layout_line_height(&invocation.arguments[0], line_count)?;
        let line_top = line.saturating_mul(line_height);

        Some(line_top.saturating_sub(baseline_metric))
    }

    pub(super) fn text_layout_get_line_top(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        let line_count = self.text_layout_line_count_for_layout(&invocation.arguments[0])?;
        if line < 0 || line > line_count {
            return None;
        }

        let line_height = self.text_layout_line_height(&invocation.arguments[0], line_count)?;
        line.checked_mul(line_height)
    }

    pub(super) fn text_layout_get_line_bottom(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(line) = invocation.arguments[1] else {
            return None;
        };
        let line_count = self.text_layout_line_count_for_layout(&invocation.arguments[0])?;
        if line < 0 || line >= line_count {
            return None;
        }

        let line_height = self.text_layout_line_height(&invocation.arguments[0], line_count)?;
        line.checked_add(1)?.checked_mul(line_height)
    }

    pub(super) fn text_layout_get_text(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        self.read_stored_instance_field(
            &invocation.arguments[0],
            &text_layout_char_sequence_field("text"),
        )
        .or_else(|| Some(DexValue::String(std::rc::Rc::from(String::new()))))
    }

    pub(super) fn text_layout_get_paint(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        self.read_stored_instance_field(&invocation.arguments[0], &text_layout_paint_field())
    }

    pub(super) fn text_layout_is_fallback_line_spacing_enabled(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = self
            .read_stored_instance_field(
                &invocation.arguments[0],
                &text_layout_int_field("useLineSpacingFromFallbacks"),
            )
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(value) = value else {
            return None;
        };
        if value != 0 && value != 1 {
            return None;
        }

        Some(value)
    }

    pub(super) fn text_layout_draw(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        let text = self.text_layout_text(&invocation.arguments[0])?;
        let paint =
            self.read_stored_instance_field(&invocation.arguments[0], &text_layout_paint_field())?;
        let canvas_key = self.text_layout_canvas_key(&invocation.arguments[1]);
        self.runtime
            .borrow_mut()
            .canvas_draw_ops
            .entry(canvas_key)
            .or_default()
            .push(CanvasDrawOp::TextLayout {
                text,
                paint,
                layout: invocation.arguments[0].clone(),
            });

        Some(())
    }

    fn text_layout_canvas_key(&self, canvas: &DexValue) -> String {
        let canvas_key = map_identity_key(canvas);
        let runtime = self.runtime.borrow();

        if runtime.canvas_draw_ops.contains_key(&canvas_key) {
            return canvas_key;
        }

        let is_platform_canvas = matches!(
            value_class_descriptor(canvas),
            Some("Landroid/graphics/Canvas;" | "Landroid/graphics/RecordingCanvas;")
        );
        if !is_platform_canvas {
            if let Some(active_canvas_key) = &runtime.active_canvas_key {
                return active_canvas_key.clone();
            }
        }

        canvas_key
    }

    fn text_layout_text(&self, layout: &DexValue) -> Option<String> {
        self.read_stored_instance_field(layout, &text_layout_char_sequence_field("text"))
            .and_then(|value| self.char_sequence_text(&value))
            .or_else(|| Some(String::new()))
    }

    fn text_layout_line_count_for_layout(&self, layout: &DexValue) -> Option<i32> {
        let text = self.text_layout_text(layout)?;
        i32::try_from(
            text.encode_utf16()
                .filter(|unit| *unit == b'\n' as u16)
                .count()
                + 1,
        )
        .ok()
    }

    fn text_layout_metric_from_layout(&self, layout: &DexValue, metric_name: &str) -> Option<i32> {
        let metrics = self.read_stored_instance_field(layout, &boring_layout_metrics_field())?;
        let value =
            self.read_stored_instance_field(&metrics, &paint_font_metrics_int_field(metric_name))?;
        let DexValue::Int(metric) = value else {
            return None;
        };

        Some(metric)
    }

    fn text_layout_line_height(&self, layout: &DexValue, line_count: i32) -> Option<i32> {
        if line_count <= 0 {
            return None;
        }

        let value = self.read_stored_instance_field(layout, &boring_layout_int_field("height"))?;
        let DexValue::Int(height) = value else {
            return None;
        };

        Some(height / line_count)
    }

    fn text_layout_line_for_offset(&self, text: &str, offset: i32) -> Option<usize> {
        let starts = text_layout_line_starts(text);
        let line_count = starts.len().checked_sub(1)?;
        let mut line = 0_usize;
        for candidate in 0..line_count {
            if starts[candidate] <= offset {
                line = candidate;
            } else {
                break;
            }
        }
        if line >= line_count {
            line = line_count.saturating_sub(1);
        }

        Some(line)
    }

    fn text_layout_measure_desired_width(&self, text: &str, paint: &DexValue) -> Option<f32> {
        let text_size = self.paint_text_size(paint).unwrap_or(16.0).abs();
        let text_scale_x = self
            .read_stored_instance_field(paint, &paint_text_scale_x_field())
            .unwrap_or(DexValue::Float(1.0));
        let DexValue::Float(text_scale_x) = text_scale_x else {
            return None;
        };
        let text_scale_x = text_scale_x.abs();

        if text_size == 0.0 || text_scale_x == 0.0 {
            return Some(0.0);
        }

        let mut max_units = 0_i32;
        let mut line_units = 0_i32;
        for unit in text.encode_utf16() {
            if unit == b'\n' as u16 {
                max_units = max_units.max(line_units);
                line_units = 0;
            } else {
                line_units = line_units.saturating_add(1);
            }
        }
        max_units = max_units.max(line_units);

        Some(max_units as f32 * text_size * text_scale_x * 0.5)
    }

    pub(super) fn char_array_string(
        &self,
        array: &DexValue,
        offset: DexValue,
        count: DexValue,
    ) -> Option<String> {
        let DexValue::Int(offset) = offset else {
            return None;
        };
        let DexValue::Int(count) = count else {
            return None;
        };
        let offset = usize::try_from(offset).ok()?;
        let count = usize::try_from(count).ok()?;
        let end = offset.checked_add(count)?;
        let array_key = map_identity_key(array);
        let values = self.runtime.borrow().array_entries.get(&array_key)?.clone();

        if end > values.len() {
            return None;
        }

        let mut units = Vec::with_capacity(count);
        for value in &values[offset..end] {
            let DexValue::Int(unit) = value else {
                return None;
            };
            units.push(u16::try_from(*unit).ok()?);
        }

        String::from_utf16(&units).ok()
    }

    pub(super) fn string_text(&self, value: &DexValue) -> Option<String> {
        match value {
            DexValue::String(value) => Some(value.clone().to_string()),
            DexValue::Object(_) | DexValue::Reference(_) => {
                let key = map_identity_key(value);
                self.runtime.borrow().string_values.get(&key).cloned()
            }
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_) => None,
        }
    }

    pub(super) fn string_equals(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.string_text(&invocation.arguments[0])?;
        let Some(right) = self.string_text(&invocation.arguments[1]) else {
            return Some(0);
        };

        Some(i32::from(left == right))
    }

    // cite: libcore String.java#contains, indexOf
    // cite: art entrypoint_utils-inl.h#FindMethodToCall
    pub(super) fn string_contains(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if matches!(invocation.arguments[0], DexValue::Null | DexValue::Int(0)) {
            return Some(self.null_pointer_exception_result());
        }

        let value = self.string_text(&invocation.arguments[0])?;

        if matches!(invocation.arguments[1], DexValue::Null) {
            return Some(self.null_pointer_exception_result());
        }

        let needle = self
            .char_sequence_full_text(&invocation.arguments[1])
            .or_else(|| self.object_value_to_string(&invocation.arguments[1]))?;

        Some(DexExecutionResult::Int(i32::from(value.contains(&needle))))
    }

    pub(super) fn string_equals_ignore_case(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.string_text(&invocation.arguments[0])?;
        let right = self.string_text(&invocation.arguments[1])?;

        Some(i32::from(left.to_lowercase() == right.to_lowercase()))
    }

    pub(super) fn string_compare_to_ignore_case(&self, invocation: &DexInvocation) -> Option<i32> {
        // cite: libcore String.java#compareToIgnoreCase, compare
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.string_text(&invocation.arguments[0])?;
        let right = self.string_text(&invocation.arguments[1])?;
        let left_units: Vec<u16> = left.to_lowercase().encode_utf16().collect();
        let right_units: Vec<u16> = right.to_lowercase().encode_utf16().collect();

        for (left_unit, right_unit) in left_units.iter().zip(right_units.iter()) {
            if left_unit != right_unit {
                return Some(i32::from(*left_unit) - i32::from(*right_unit));
            }
        }

        Some(i32::try_from(left_units.len()).ok()? - i32::try_from(right_units.len()).ok()?)
    }

    pub(super) fn string_starts_with(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let prefix = self.string_text(&invocation.arguments[1])?;

        Some(i32::from(value.starts_with(&prefix)))
    }

    pub(super) fn string_ends_with(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let suffix = self.string_text(&invocation.arguments[1])?;

        Some(i32::from(value.ends_with(&suffix)))
    }

    // cite: libcore String.java#getChars
    pub(super) fn string_get_chars(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 5 {
            return None;
        }

        let text = self.string_text(&invocation.arguments[0])?;
        let DexValue::Int(src_begin) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(src_end) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Int(dst_begin) = invocation.arguments[4] else {
            return None;
        };
        if src_begin < 0 || src_end < src_begin {
            return None;
        }

        let units: Vec<u16> = text.encode_utf16().collect();
        let src_begin = usize::try_from(src_begin).ok()?;
        let src_end = usize::try_from(src_end).ok()?;
        let dst_begin = usize::try_from(dst_begin).ok()?;
        if src_end > units.len() {
            return None;
        }

        let array_key = map_identity_key(&invocation.arguments[3]);
        let mut runtime = self.runtime.borrow_mut();
        let dst = runtime.array_entries.get_mut(&array_key)?;
        if dst_begin + (src_end - src_begin) > dst.len() {
            return None;
        }

        for (offset, unit) in units[src_begin..src_end].iter().enumerate() {
            dst[dst_begin + offset] = DexValue::Int(i32::from(*unit));
        }

        Some(())
    }

    // cite: libcore String.java#toCharArray
    pub(super) fn string_to_char_array(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let text = self.string_text(&invocation.arguments[0])?;
        let units = text
            .encode_utf16()
            .map(|unit| DexValue::Int(i32::from(unit)))
            .collect();

        Some(self.allocate_filled_array("[C".to_owned(), units))
    }

    pub(super) fn string_length(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;

        i32::try_from(value.encode_utf16().count()).ok()
    }

    pub(super) fn string_hash_code(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let mut hash = 0i32;

        for unit in value.encode_utf16() {
            hash = hash.wrapping_mul(31).wrapping_add(i32::from(unit));
        }

        Some(hash)
    }

    pub(super) fn string_char_at(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let DexValue::Int(index) = invocation.arguments[1] else {
            return None;
        };
        let index = usize::try_from(index).ok()?;
        let code_unit = value.encode_utf16().nth(index)?;

        Some(i32::from(code_unit))
    }

    pub(super) fn character_code_point_at_char_sequence(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if matches!(invocation.arguments[0], DexValue::Null | DexValue::Int(0)) {
            return None;
        }

        let sequence = &invocation.arguments[0];
        let length = self.char_sequence_length_value(sequence)?;
        let DexValue::Int(index) = invocation.arguments[1] else {
            return None;
        };
        if index < 0 || index >= length {
            return None;
        }

        let first = u16::try_from(self.char_sequence_char_at_value(sequence, index)?).ok()?;

        if is_high_surrogate(first) {
            let next_index = index.checked_add(1)?;
            if next_index < length {
                let second =
                    u16::try_from(self.char_sequence_char_at_value(sequence, next_index)?).ok()?;
                if is_low_surrogate(second) {
                    let high = u32::from(first) - 0xd800;
                    let low = u32::from(second) - 0xdc00;
                    let code_point = 0x10000 + ((high << 10) | low);

                    return i32::try_from(code_point).ok();
                }
            }
        }

        Some(i32::from(first))
    }

    // cite: libcore AbstractStringBuilder.java#append
    pub(in crate::dex_run) fn char_sequence_full_text(
        &self,
        sequence: &DexValue,
    ) -> Option<String> {
        if let Some(text) = self.char_sequence_text(sequence) {
            return Some(text);
        }
        let length = self.char_sequence_length_value(sequence)?;
        let mut units = Vec::with_capacity(length.max(0) as usize);
        for index in 0..length {
            let unit = self.char_sequence_char_at_value(sequence, index)?;
            units.push(unit as u16);
        }

        Some(String::from_utf16_lossy(&units))
    }

    fn char_sequence_length_value(&self, sequence: &DexValue) -> Option<i32> {
        if let Some(value) = self.char_sequence_text(sequence) {
            return i32::try_from(value.encode_utf16().count()).ok();
        }

        self.char_sequence_int_method(sequence, "length", "I", "I", Vec::new(), Vec::new())
    }

    fn char_sequence_char_at_value(&self, sequence: &DexValue, index: i32) -> Option<i32> {
        if let Some(value) = self.char_sequence_text(sequence) {
            let index = usize::try_from(index).ok()?;
            let unit = value.encode_utf16().nth(index)?;

            return Some(i32::from(unit));
        }

        self.char_sequence_int_method(
            sequence,
            "charAt",
            "CI",
            "C",
            vec!["I".to_owned()],
            vec![DexValue::Int(index)],
        )
    }

    fn char_sequence_int_method(
        &self,
        sequence: &DexValue,
        method_name: &str,
        shorty: &str,
        return_type: &str,
        parameters: Vec<String>,
        arguments: Vec<DexValue>,
    ) -> Option<i32> {
        let class_descriptor = value_class_descriptor(sequence)?;
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: method_name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: shorty.to_owned(),
                return_type: return_type.to_owned(),
                parameters,
            },
        };
        let resolved = self
            .resolve_exact_method_in_class_hierarchy(class_descriptor, &method)
            .ok()??;
        let mut initial_arguments = Vec::with_capacity(arguments.len() + 1);
        initial_arguments.push(sequence.clone());
        initial_arguments.extend(arguments);
        let result = self
            .execute_method(None, &resolved, Some(&initial_arguments), 0)
            .ok()?;

        match result {
            DexExecutionResult::Int(value) => Some(value),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                None
            }
            _ => None,
        }
    }

    pub(super) fn string_is_empty(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;

        Some(i32::from(value.is_empty()))
    }

    pub(super) fn string_to_lower_case_locale(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => Some(value.to_lowercase()),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => None,
        }
    }

    pub(super) fn string_to_upper_case_locale(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => Some(value.to_uppercase()),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => None,
        }
    }

    pub(super) fn string_substring(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let DexValue::Int(start) = invocation.arguments[1] else {
            return None;
        };
        let start = usize::try_from(start).ok()?;
        let units: Vec<u16> = value.encode_utf16().collect();

        if start > units.len() {
            return None;
        }

        String::from_utf16(&units[start..])
            .ok()
            .map(|value| DexValue::String(value.into()))
    }

    pub(super) fn string_substring_range(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let DexValue::Int(start) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(end) = invocation.arguments[2] else {
            return None;
        };

        utf16_string_slice(&value, start, end).map(|value| DexValue::String(value.into()))
    }

    pub(super) fn string_index_of(&self, invocation: &DexInvocation) -> Option<i32> {
        if !(2..=3).contains(&invocation.arguments.len()) {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let needle = self.string_text(&invocation.arguments[1])?;
        let from_index = match invocation.arguments.get(2) {
            Some(DexValue::Int(value)) => *value,
            Some(_) => return None,
            None => 0,
        };

        string_index_of_units(&value, &needle, from_index)
    }

    pub(super) fn string_index_of_code_point(&self, invocation: &DexInvocation) -> Option<i32> {
        if !(2..=3).contains(&invocation.arguments.len()) {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let DexValue::Int(code_point) = invocation.arguments[1] else {
            return None;
        };
        let from_index = match invocation.arguments.get(2) {
            Some(DexValue::Int(value)) => *value,
            Some(_) => return None,
            None => 0,
        };

        string_index_of_code_point_in_text(&value, code_point, from_index)
    }

    pub(super) fn string_last_index_of(&self, invocation: &DexInvocation) -> Option<i32> {
        // cite: libcore String.java#lastIndexOf
        if !(2..=3).contains(&invocation.arguments.len()) {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let needle = self.string_text(&invocation.arguments[1])?;
        let from_index = match invocation.arguments.get(2) {
            Some(DexValue::Int(value)) => *value,
            Some(_) => return None,
            None => i32::try_from(value.encode_utf16().count()).ok()?,
        };

        string_last_index_of_units(&value, &needle, from_index)
    }

    pub(super) fn string_last_index_of_code_point(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        // cite: libcore String.java#lastIndexOf
        if !(2..=3).contains(&invocation.arguments.len()) {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let DexValue::Int(code_point) = invocation.arguments[1] else {
            return None;
        };
        let from_index = match invocation.arguments.get(2) {
            Some(DexValue::Int(value)) => *value,
            Some(_) => return None,
            None => i32::try_from(value.encode_utf16().count()).ok()? - 1,
        };

        string_last_index_of_code_point_in_text(&value, code_point, from_index)
    }

    pub(super) fn string_replace_chars(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let DexValue::Int(old_char) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(new_char) = invocation.arguments[2] else {
            return None;
        };

        replace_utf16_units(&value, old_char, new_char)
    }

    // cite: libcore String.java#replace
    pub(super) fn string_replace_sequences(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [receiver, target, replacement] = invocation.arguments.as_slice() else {
            return None;
        };
        if matches!(target, DexValue::Null | DexValue::Int(0))
            || matches!(replacement, DexValue::Null | DexValue::Int(0))
        {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/NullPointerException;".to_owned(),
                    detail_message: None,
                }),
            ));
        }
        let receiver_text = self.string_text(receiver)?;
        let target: Vec<u16> = self.string_text(target)?.encode_utf16().collect();
        let replacement: Vec<u16> = self.string_text(replacement)?.encode_utf16().collect();
        let receiver_units: Vec<u16> = receiver_text.encode_utf16().collect();

        let utf16_index_of = |from: usize| -> Option<usize> {
            let length = receiver_units.len();
            let from = from.min(length);
            if target.is_empty() {
                return Some(from);
            }
            if target.len() > length {
                return None;
            }
            (from..=length - target.len())
                .find(|start| receiver_units[*start..].starts_with(&target))
        };
        let Some(mut match_start) = utf16_index_of(0) else {
            return Some(DexExecutionResult::Value(receiver.clone()));
        };
        let target_step = target.len().max(1);
        let mut result: Vec<u16> = Vec::new();
        let mut copied_to = 0;
        loop {
            result.extend_from_slice(&receiver_units[copied_to..match_start]);
            result.extend_from_slice(&replacement);
            copied_to = match_start + target.len();
            if match_start >= receiver_units.len() {
                break;
            }
            match utf16_index_of(match_start + target_step) {
                Some(next) if next > 0 => match_start = next,
                _ => break,
            }
        }
        result.extend_from_slice(&receiver_units[copied_to..]);

        let text = String::from_utf16(&result).ok()?;

        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(text),
        )))
    }

    pub(super) fn string_concat(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.string_text(&invocation.arguments[0])?;
        let right = self.string_text(&invocation.arguments[1])?;

        Some(DexValue::String(std::rc::Rc::from(format!(
            "{left}{right}"
        ))))
    }

    pub(super) fn char_sequence_sub_sequence(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let value = self.string_text(&invocation.arguments[0])?;
        let DexValue::Int(start) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(end) = invocation.arguments[2] else {
            return None;
        };

        utf16_string_slice(&value, start, end).map(|value| DexValue::String(value.into()))
    }

    pub(super) fn string_split(&self, invocation: &DexInvocation) -> Option<DexValue> {
        // cite: libcore String.java#split
        let (value, pattern, limit) = match invocation.arguments.as_slice() {
            [value, pattern] => (value, pattern, 0),
            [value, pattern, DexValue::Int(limit)] => (value, pattern, *limit),
            _ => return None,
        };

        let value = self.string_text(value)?;
        let pattern = self.string_text(pattern)?;
        let parts = match string_split_literal_delimiter(&pattern) {
            Some(delimiter) => string_fast_split(&value, delimiter, limit),
            None => {
                let regex = cached_regex(&translate_java_regex_quoting(&pattern))?;
                string_pattern_split(&regex, &value, limit, self.target_sdk_version)?
            }
        };
        let values = parts
            .into_iter()
            .map(|value| DexValue::String(value.into()))
            .collect();

        Some(self.allocate_filled_array("[Ljava/lang/String;".to_owned(), values))
    }

    pub(super) fn string_format(&self, invocation: &DexInvocation) -> Option<String> {
        self.format_with_arguments(invocation.arguments.as_slice())
    }

    pub(super) fn print_stream_format(&self, invocation: &DexInvocation) -> Option<DexValue> {
        let (receiver, format_arguments) = invocation.arguments.as_slice().split_first()?;
        if !dex_value_is_object_like(receiver) {
            return None;
        }
        self.format_with_arguments(format_arguments)?;

        Some(receiver.clone())
    }

    fn format_with_arguments(&self, arguments: &[DexValue]) -> Option<String> {
        let (format, argument_array) = match arguments {
            [DexValue::String(format), arguments] => (format, arguments),
            [locale, DexValue::String(format), arguments] => {
                match locale {
                    DexValue::Null => {}
                    value if dex_value_is_object_like(value) => {}
                    _ => return None,
                }
                (format, arguments)
            }
            _ => return None,
        };
        let values = match argument_array {
            DexValue::Null => None,
            _ => Some(
                self.runtime
                    .borrow()
                    .array_entries
                    .get(&map_identity_key(argument_array))?
                    .clone(),
            ),
        };

        self.format_java_string(format, values.as_deref())
    }

    pub(super) fn initialize_string_builder(&self, invocation: &DexInvocation) -> Option<()> {
        let builder = invocation.arguments.first()?;
        let initial_value = match invocation.arguments.get(1) {
            Some(DexValue::Int(_)) => String::new(),
            Some(value) => self.char_sequence_text(value)?,
            None => String::new(),
        };
        let builder_key = map_identity_key(builder);

        self.runtime
            .borrow_mut()
            .string_builder_values
            .insert(builder_key, initial_value);

        Some(())
    }

    pub(super) fn append_string_builder(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let appended = self.string_builder_append_text(&invocation.arguments[1])?;

        self.append_string_builder_text(builder, &appended)
    }

    pub(super) fn append_string_builder_text(
        &self,
        builder: DexValue,
        appended: &str,
    ) -> Option<DexValue> {
        let builder_key = map_identity_key(&builder);
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();

        value.push_str(appended);

        Some(builder)
    }

    pub(super) fn append_string_builder_char_sequence(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let appended = self.char_sequence_text(&invocation.arguments[1])?;
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();

        value.push_str(&appended);

        Some(builder)
    }

    // cite: libcore AbstractStringBuilder.java#append
    pub(super) fn append_string_builder_char_array_range(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 4 {
            return None;
        }
        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let appended = self.char_array_string(
            &invocation.arguments[1],
            invocation.arguments[2].clone(),
            invocation.arguments[3].clone(),
        )?;
        self.runtime
            .borrow_mut()
            .string_builder_values
            .entry(builder_key)
            .or_default()
            .push_str(&appended);

        Some(builder)
    }

    pub(super) fn append_string_builder_char_sequence_range(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let text = self.char_sequence_full_text(&invocation.arguments[1])?;
        let DexValue::Int(start) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Int(end) = invocation.arguments[3] else {
            return None;
        };
        let appended = utf16_string_slice(&text, start, end)?;
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();

        value.push_str(&appended);

        Some(builder)
    }

    pub(super) fn append_string_builder_char(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };
        let character = char::from_u32(value as u32)?;
        let mut runtime = self.runtime.borrow_mut();
        let text = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();

        text.push(character);

        Some(builder)
    }

    pub(super) fn append_string_builder_code_point(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let DexValue::Int(code_point) = invocation.arguments[1] else {
            return None;
        };
        let code_point = u32::try_from(code_point).ok()?;
        if (0xd800..=0xdfff).contains(&code_point) {
            return None;
        }
        let character = char::from_u32(code_point)?;
        let mut runtime = self.runtime.borrow_mut();
        let text = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();

        text.push(character);

        Some(builder)
    }

    pub(super) fn append_string_builder_boolean(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };
        let mut runtime = self.runtime.borrow_mut();
        let text = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();

        if value == 0 {
            text.push_str("false");
        } else {
            text.push_str("true");
        }

        Some(builder)
    }

    // cite: libcore AbstractStringBuilder.java#append
    pub(super) fn append_string_builder_long(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let DexValue::Long(value) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Long(high_slot_value) = invocation.arguments[2] else {
            return None;
        };
        if value != high_slot_value {
            return None;
        }

        let mut runtime = self.runtime.borrow_mut();
        let text = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();
        text.push_str(&value.to_string());

        Some(builder)
    }

    pub(super) fn append_string_builder_float(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let value = java_float_to_string(read_float_argument(&invocation.arguments[1])?)?;
        let mut runtime = self.runtime.borrow_mut();
        let text = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();

        text.push_str(&value);

        Some(builder)
    }

    // cite: libcore AbstractStringBuilder.java#append
    // cite: libcore Double.java#toString
    pub(super) fn append_string_builder_double(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let [builder, DexValue::Long(bits), DexValue::Long(_)] = invocation.arguments.as_slice()
        else {
            return None;
        };
        let value = java_double_to_string(f64::from_bits(*bits as u64))?;
        self.runtime
            .borrow_mut()
            .string_builder_values
            .entry(map_identity_key(builder))
            .or_default()
            .push_str(&value);

        Some(builder.clone())
    }

    pub(super) fn insert_string_builder_char(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let DexValue::Int(index) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(value) = invocation.arguments[2] else {
            return None;
        };
        let index = usize::try_from(index).ok()?;
        let code_unit = u16::try_from(value).ok()?;
        let mut runtime = self.runtime.borrow_mut();
        let text = runtime
            .string_builder_values
            .entry(builder_key)
            .or_default();
        let mut units: Vec<u16> = text.encode_utf16().collect();

        if index > units.len() {
            return None;
        }

        units.insert(index, code_unit);
        *text = String::from_utf16(&units).ok()?;

        Some(builder)
    }

    pub(super) fn reverse_string_builder(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let builder = invocation.arguments[0].clone();
        let builder_key = map_identity_key(&builder);
        let mut runtime = self.runtime.borrow_mut();
        let text = runtime.string_builder_values.get_mut(&builder_key)?;
        let reversed: String = text.chars().rev().collect();

        *text = reversed;

        Some(builder)
    }

    pub(super) fn string_builder_to_string(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let builder_key = map_identity_key(&invocation.arguments[0]);

        Some(
            self.runtime
                .borrow()
                .string_builder_values
                .get(&builder_key)
                .cloned()
                .unwrap_or_default(),
        )
    }

    // cite: libcore AbstractStringBuilder.java#setLength
    pub(super) fn string_builder_set_length(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 {
            return None;
        }
        let DexValue::Int(new_length) = invocation.arguments[1] else {
            return None;
        };
        if new_length < 0 {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/StringIndexOutOfBoundsException;".to_owned(),
                    detail_message: Some(new_length.to_string()),
                }),
            ));
        }
        let builder_key = map_identity_key(&invocation.arguments[0]);
        let current = self
            .runtime
            .borrow()
            .string_builder_values
            .get(&builder_key)
            .cloned()
            .unwrap_or_default();
        let mut units: Vec<u16> = current.encode_utf16().collect();
        let new_length = usize::try_from(new_length).ok()?;
        if new_length < units.len() {
            units.truncate(new_length);
        } else {
            units.resize(new_length, 0);
        }
        self.runtime
            .borrow_mut()
            .string_builder_values
            .insert(builder_key, String::from_utf16_lossy(&units));

        Some(DexExecutionResult::Void)
    }

    pub(super) fn string_builder_length(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }
        let builder_key = map_identity_key(&invocation.arguments[0]);
        let value = self
            .runtime
            .borrow()
            .string_builder_values
            .get(&builder_key)?
            .encode_utf16()
            .count();

        i32::try_from(value).ok()
    }

    pub(super) fn char_sequence_text(&self, value: &DexValue) -> Option<String> {
        match value {
            DexValue::String(value) => Some(value.clone().to_string()),
            DexValue::Null => Some("null".to_owned()),
            DexValue::Object(_) | DexValue::Reference(_) => {
                let key = map_identity_key(value);
                let runtime = self.runtime.borrow();
                runtime
                    .string_values
                    .get(&key)
                    .or_else(|| runtime.string_builder_values.get(&key))
                    .cloned()
            }
            DexValue::Uninitialized | DexValue::Int(_) | DexValue::Long(_) | DexValue::Float(_) => {
                None
            }
        }
    }

    pub(super) fn object_to_string(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.object_value_to_string(&invocation.arguments[0])
    }

    // cite: libcore Integer.java#toString
    // cite: libcore Long.java#toString
    // cite: libcore Boolean.java#toString
    pub(super) fn object_value_to_string(&self, value: &DexValue) -> Option<String> {
        match value {
            DexValue::String(value) => Some(value.clone().to_string()),
            DexValue::Reference(reference) => {
                if let Some(display) = boxed_reference_display_value(reference) {
                    return Some(display);
                }
                if let Some(display) = self.generic_type_object_to_string(value) {
                    return Some(display);
                }

                self.object_to_string_default(value)
            }
            DexValue::Object(_) => self.object_to_string_default(value),
            DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::Uninitialized => None,
        }
    }

    // cite: libcore Object.java#toString
    fn object_to_string_default(&self, value: &DexValue) -> Option<String> {
        let descriptor = value_class_descriptor(value)?;
        let key = map_identity_key(value);

        if matches!(
            descriptor,
            "Ljava/lang/StringBuilder;" | "Ljava/lang/StringBuffer;"
        ) {
            if let Some(value) = self.runtime.borrow().string_builder_values.get(&key) {
                return Some(value.clone());
            }
        }

        let class_name =
            descriptor_to_java_name(descriptor).unwrap_or_else(|| descriptor.to_owned());
        let identity_hash = stable_hash_i32(&key);

        Some(format!("{class_name}@{:x}", identity_hash as u32))
    }

    pub(super) fn objects_to_string(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if matches!(invocation.arguments[0], DexValue::Null) {
            return Some("null".to_owned());
        }

        self.object_to_string(invocation)
    }

    pub(super) fn string_builder_append_text(&self, value: &DexValue) -> Option<String> {
        match value {
            DexValue::String(value) => Some(value.clone().to_string()),
            DexValue::Int(value) => Some(value.to_string()),
            DexValue::Long(value) => Some(value.to_string()),
            DexValue::Null => Some("null".to_owned()),
            DexValue::Object(_) | DexValue::Reference(_) => self.object_value_to_string(value),
            DexValue::Float(value) => Some(value.to_string()),
            DexValue::Uninitialized => None,
        }
    }

    pub(super) fn initialize_string_reader(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::String(text) = &invocation.arguments[1] else {
            return None;
        };

        let reader_key = map_identity_key(&invocation.arguments[0]);
        let state = StringReaderState {
            units: text.encode_utf16().collect(),
            position: 0,
            closed: false,
        };

        self.runtime
            .borrow_mut()
            .string_reader_values
            .insert(reader_key, state);

        Some(())
    }

    pub(super) fn read_string_reader_char(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let reader_key = map_identity_key(&invocation.arguments[0]);
        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.string_reader_values.get_mut(&reader_key)?;

        if state.closed {
            return None;
        }

        let Some(unit) = state.units.get(state.position).copied() else {
            return Some(-1);
        };
        state.position += 1;

        Some(i32::from(unit))
    }

    pub(super) fn read_string_reader_buffer(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let reader_key = map_identity_key(&invocation.arguments[0]);
        let array_key = map_identity_key(&invocation.arguments[1]);
        let DexValue::Int(offset) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Int(length) = invocation.arguments[3] else {
            return None;
        };
        let offset = usize::try_from(offset).ok()?;
        let length = usize::try_from(length).ok()?;

        let mut runtime = self.runtime.borrow_mut();
        let array_len = runtime.array_entries.get(&array_key)?.len();
        if offset > array_len {
            return None;
        }
        if length > array_len - offset {
            return None;
        }

        let state = runtime.string_reader_values.get(&reader_key)?;
        if state.closed {
            return None;
        }
        if length == 0 {
            return Some(0);
        }

        let (units, read_count) = {
            let state = runtime.string_reader_values.get_mut(&reader_key)?;
            if state.closed {
                return None;
            }
            if state.position >= state.units.len() {
                return Some(-1);
            }

            let remaining = state.units.len() - state.position;
            let read_count = length.min(remaining);
            let end = state.position + read_count;
            let units = state.units[state.position..end].to_vec();
            state.position = end;

            (units, read_count)
        };

        let array = runtime.array_entries.get_mut(&array_key)?;
        for (index, unit) in units.into_iter().enumerate() {
            array[offset + index] = DexValue::Int(i32::from(unit));
        }

        i32::try_from(read_count).ok()
    }

    pub(super) fn close_string_reader(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let reader_key = map_identity_key(&invocation.arguments[0]);
        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.string_reader_values.get_mut(&reader_key)?;
        state.closed = true;

        Some(())
    }

    pub(super) fn require_string_reader_receiver(&self, invocation: &DexInvocation) -> Option<()> {
        let receiver = invocation.arguments.first()?;
        let reader_key = map_identity_key(receiver);

        if self
            .runtime
            .borrow()
            .string_reader_values
            .contains_key(&reader_key)
        {
            return Some(());
        }

        None
    }

    pub(super) fn initialize_string_writer(&self, invocation: &DexInvocation) -> Option<()> {
        let writer = invocation.arguments.first()?;
        let writer_key = map_identity_key(writer);

        self.runtime
            .borrow_mut()
            .string_writer_values
            .insert(writer_key, String::new());

        Some(())
    }

    pub(super) fn write_string_writer_string(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let text = self.char_sequence_text(&invocation.arguments[1])?;
        self.push_string_writer_text(&invocation.arguments[0], &text)
    }

    pub(super) fn write_string_writer_string_range(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let text = self.char_sequence_text(&invocation.arguments[1])?;
        let DexValue::Int(start) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Int(len) = invocation.arguments[3] else {
            return None;
        };
        let end = start.checked_add(len)?;
        let slice = utf16_string_slice(&text, start, end)?;

        self.push_string_writer_text(&invocation.arguments[0], &slice)
    }

    pub(super) fn write_string_writer_char(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };
        let character = char::from_u32(value as u32)?;

        self.push_string_writer_text(&invocation.arguments[0], &character.to_string())
    }

    pub(super) fn append_string_writer(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let writer = invocation.arguments[0].clone();
        let text = self.char_sequence_text(&invocation.arguments[1])?;

        self.push_string_writer_text(&writer, &text)?;

        Some(writer)
    }

    pub(super) fn append_string_writer_range(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let writer = invocation.arguments[0].clone();
        let text = self.char_sequence_text(&invocation.arguments[1])?;
        let DexValue::Int(start) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Int(end) = invocation.arguments[3] else {
            return None;
        };
        let slice = utf16_string_slice(&text, start, end)?;

        self.push_string_writer_text(&writer, &slice)?;

        Some(writer)
    }

    pub(super) fn append_string_writer_char(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let writer = invocation.arguments[0].clone();
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };
        let character = char::from_u32(value as u32)?;

        self.push_string_writer_text(&writer, &character.to_string())?;

        Some(writer)
    }

    pub(super) fn string_writer_to_string(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let writer_key = map_identity_key(&invocation.arguments[0]);

        Some(
            self.runtime
                .borrow()
                .string_writer_values
                .get(&writer_key)
                .cloned()
                .unwrap_or_default(),
        )
    }

    pub(super) fn require_string_writer_receiver(&self, invocation: &DexInvocation) -> Option<()> {
        let receiver = invocation.arguments.first()?;
        let descriptor = value_class_descriptor(receiver)?;

        if descriptor == "Ljava/io/StringWriter;" {
            return Some(());
        }

        None
    }

    pub(super) fn push_string_writer_text(&self, writer: &DexValue, text: &str) -> Option<()> {
        let writer_key = map_identity_key(writer);
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime.string_writer_values.entry(writer_key).or_default();

        value.push_str(text);

        Some(())
    }
}

pub(super) fn decode_charset_bytes(bytes: &[u8], charset: &str) -> Option<String> {
    // cite: libcore String.java#String, StringCoding
    let charset = charset.to_ascii_uppercase().replace('_', "-");
    match charset.as_str() {
        "UTF-8" | "UTF8" => Some(String::from_utf8_lossy(bytes).into_owned()),
        "US-ASCII" | "ASCII" | "ANSI-X3.4-1968" => Some(
            bytes
                .iter()
                .map(|byte| {
                    if byte.is_ascii() {
                        char::from(*byte)
                    } else {
                        '\u{fffd}'
                    }
                })
                .collect(),
        ),
        "ISO-8859-1" | "ISO8859-1" | "ISO8859-1:1987" | "LATIN1" => {
            Some(bytes.iter().map(|byte| char::from(*byte)).collect())
        }
        _ => None,
    }
}

// cite: libcore String.java#getBytes
// cite: libcore StandardCharsets.java#UTF_8, US_ASCII, ISO_8859_1, UTF_16, UTF_16BE, UTF_16LE
pub(super) fn encode_charset_text(text: &str, charset: &str) -> Option<Vec<u8>> {
    let charset = charset.to_ascii_uppercase().replace('_', "-");
    let units = || text.encode_utf16();
    Some(match charset.as_str() {
        "UTF-8" | "UTF8" => text.as_bytes().to_vec(),
        "US-ASCII" | "ASCII" | "ANSI-X3.4-1968" => text
            .chars()
            .map(|character| {
                if character.is_ascii() {
                    character as u8
                } else {
                    b'?'
                }
            })
            .collect(),
        "ISO-8859-1" | "ISO8859-1" | "ISO8859-1:1987" | "LATIN1" => text
            .chars()
            .map(|character| u8::try_from(u32::from(character)).unwrap_or(b'?'))
            .collect(),
        "UTF-16" => [0xfe, 0xff]
            .into_iter()
            .chain(units().flat_map(u16::to_be_bytes))
            .collect(),
        "UTF-16BE" => units().flat_map(u16::to_be_bytes).collect(),
        "UTF-16LE" => units().flat_map(u16::to_le_bytes).collect(),
        _ => return None,
    })
}

pub(super) fn boxed_reference_display_value(reference: &str) -> Option<String> {
    if reference == "Ljava/lang/Boolean;:true" {
        return Some("true".to_owned());
    }

    if reference == "Ljava/lang/Boolean;:false" {
        return Some("false".to_owned());
    }

    if let Some(value) = boxed_integer_reference_value(reference) {
        return Some(value.to_string());
    }

    // cite: libcore Float.java#toString
    if let Some(value) = boxed_float_reference_value(reference) {
        return java_float_to_string(value);
    }

    // cite: libcore Double.java#toString
    if let Some(bits) = boxed_double_reference_value(reference) {
        return java_double_to_string(f64::from_bits(bits as u64));
    }

    None
}

pub(super) fn boxed_integer_reference_value(reference: &str) -> Option<i64> {
    for prefix in [
        "Ljava/lang/Integer;:",
        "Ljava/lang/Long;:",
        "Ljava/lang/Byte;:",
        "Ljava/lang/Short;:",
        "Ljava/lang/Character;:",
    ] {
        if let Some(value) = reference.strip_prefix(prefix) {
            return value.parse().ok();
        }
    }

    None
}

pub(super) fn boxed_float_reference_value(reference: &str) -> Option<f32> {
    let value = reference.strip_prefix("Ljava/lang/Float;:")?;
    let bits = u32::from_str_radix(value, 16).ok()?;

    Some(f32::from_bits(bits))
}

pub(super) fn text_utils_is_empty(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    match &invocation.arguments[0] {
        DexValue::Null => Some(1),
        DexValue::Int(0) => Some(1),
        DexValue::String(value) => Some(i32::from(value.is_empty())),
        DexValue::Object(_) | DexValue::Reference(_) => Some(0),
        DexValue::Uninitialized | DexValue::Int(_) | DexValue::Long(_) | DexValue::Float(_) => None,
    }
}

pub(super) fn text_utils_equals(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let left = &invocation.arguments[0];
    let right = &invocation.arguments[1];
    let left_null = is_android_null_value(left);
    let right_null = is_android_null_value(right);

    if left_null || right_null {
        return Some(i32::from(left_null && right_null));
    }

    match (left, right) {
        (DexValue::String(left), DexValue::String(right)) => Some(i32::from(left == right)),
        (DexValue::Object(_), DexValue::Object(_))
        | (DexValue::Object(_), DexValue::Reference(_))
        | (DexValue::Reference(_), DexValue::Object(_))
        | (DexValue::Reference(_), DexValue::Reference(_)) => Some(i32::from(left == right)),
        (DexValue::Uninitialized, _)
        | (DexValue::Int(_), _)
        | (DexValue::Long(_), _)
        | (DexValue::Float(_), _)
        | (DexValue::String(_), _)
        | (_, DexValue::Uninitialized)
        | (_, DexValue::Int(_))
        | (_, DexValue::Long(_))
        | (_, DexValue::Float(_))
        | (_, DexValue::String(_)) => Some(0),
        (DexValue::Null, _) | (_, DexValue::Null) => Some(0),
    }
}

pub(super) fn is_android_null_value(value: &DexValue) -> bool {
    matches!(value, DexValue::Null | DexValue::Int(0))
}

pub(super) fn regex_matcher_key(invocation: &DexInvocation) -> Option<String> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    Some(map_identity_key(&invocation.arguments[0]))
}

pub(super) fn regex_matcher_group_key(invocation: &DexInvocation) -> Option<(String, usize)> {
    match invocation.arguments.as_slice() {
        [matcher] => Some((map_identity_key(matcher), 0)),
        [matcher, DexValue::Int(group_index)] => {
            let group_index = usize::try_from(*group_index).ok()?;
            Some((map_identity_key(matcher), group_index))
        }
        _ => None,
    }
}

pub(super) fn regex_capture_groups(captures: &JavaCaptures) -> Vec<Option<RegexMatchRange>> {
    captures
        .iter()
        .map(|group| group.map(|(start, end)| RegexMatchRange { start, end }))
        .collect()
}

pub(super) fn next_regex_search_start(input: &str, match_end: usize) -> usize {
    if match_end >= input.len() {
        return match_end;
    }

    let Some(character) = input.get(match_end..).and_then(|tail| tail.chars().next()) else {
        return match_end;
    };

    if character.len_utf8() == 0 {
        return match_end;
    }

    match_end + character.len_utf8()
}

pub(super) fn string_index_of_units(value: &str, needle: &str, from_index: i32) -> Option<i32> {
    let value_units: Vec<u16> = value.encode_utf16().collect();
    let needle_units: Vec<u16> = needle.encode_utf16().collect();
    let start = usize::try_from(from_index.max(0)).ok()?;

    if needle_units.is_empty() {
        let index = start.min(value_units.len());
        return i32::try_from(index).ok();
    }

    if start > value_units.len() || needle_units.len() > value_units.len() {
        return Some(-1);
    }

    let last_start = value_units.len() - needle_units.len();
    if start > last_start {
        return Some(-1);
    }

    for index in start..=last_start {
        if value_units[index..index + needle_units.len()] == needle_units {
            return i32::try_from(index).ok();
        }
    }

    Some(-1)
}

pub(super) fn string_last_index_of_units(
    value: &str,
    needle: &str,
    from_index: i32,
) -> Option<i32> {
    // cite: libcore String.java#lastIndexOf
    if from_index < 0 {
        return Some(-1);
    }

    let value_units: Vec<u16> = value.encode_utf16().collect();
    let needle_units: Vec<u16> = needle.encode_utf16().collect();
    let right_index =
        i32::try_from(value_units.len()).ok()? - i32::try_from(needle_units.len()).ok()?;
    let from_index = from_index.min(right_index);

    if needle_units.is_empty() {
        return Some(from_index);
    }
    if from_index < 0 {
        return Some(-1);
    }

    let from_index = usize::try_from(from_index).ok()?;
    for start in (0..=from_index).rev() {
        if value_units[start..start + needle_units.len()] == needle_units {
            return i32::try_from(start).ok();
        }
    }

    Some(-1)
}

pub(super) fn string_index_of_code_point_in_text(
    value: &str,
    code_point: i32,
    from_index: i32,
) -> Option<i32> {
    if code_point < 0 {
        return Some(-1);
    }

    let needle = if code_point <= 0xffff {
        vec![code_point as u16]
    } else {
        let Some(character) = char::from_u32(code_point as u32) else {
            return Some(-1);
        };
        let mut units = [0_u16; 2];
        character.encode_utf16(&mut units).to_vec()
    };
    let units: Vec<u16> = value.encode_utf16().collect();
    let start = usize::try_from(from_index.max(0)).ok()?;

    if needle.len() > units.len() || start > units.len() {
        return Some(-1);
    }

    let last_start = units.len() - needle.len();
    if start > last_start {
        return Some(-1);
    }

    for index in start..=last_start {
        if units[index..index + needle.len()] == needle {
            return i32::try_from(index).ok();
        }
    }

    Some(-1)
}

pub(super) fn string_last_index_of_code_point_in_text(
    value: &str,
    code_point: i32,
    from_index: i32,
) -> Option<i32> {
    if from_index < 0 {
        return Some(-1);
    }

    let code_point = u32::try_from(code_point).ok()?;
    let character = char::from_u32(code_point)?;
    let units: Vec<u16> = value.encode_utf16().collect();
    let mut needle = [0_u16; 2];
    let needle = character.encode_utf16(&mut needle);

    if needle.len() > units.len() {
        return Some(-1);
    }

    let from_index = usize::try_from(from_index).ok()?;
    let mut index = from_index.min(units.len() - needle.len());

    loop {
        if units[index..index + needle.len()] == *needle {
            return i32::try_from(index).ok();
        }

        if index == 0 {
            return Some(-1);
        }

        index -= 1;
    }
}

pub(super) fn replace_utf16_units(value: &str, old_char: i32, new_char: i32) -> Option<String> {
    let old_char = u16::try_from(old_char).ok()?;
    let new_char = u16::try_from(new_char).ok()?;
    let mut units: Vec<u16> = value.encode_utf16().collect();

    for unit in &mut units {
        if *unit == old_char {
            *unit = new_char;
        }
    }

    String::from_utf16(&units).ok()
}

pub(super) fn string_builder_capacity_constructor(invocation: &DexInvocation) -> Option<()> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    match &invocation.arguments[0] {
        DexValue::Object(_) | DexValue::Reference(_) => {}
        DexValue::Uninitialized
        | DexValue::Null
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_) => return None,
    }

    let DexValue::Int(capacity) = invocation.arguments[1] else {
        return None;
    };
    if capacity < 0 {
        return None;
    }

    Some(())
}

pub(super) fn runtime_available_processors() -> i32 {
    std::thread::available_parallelism()
        .map(|value| value.get().min(i32::MAX as usize) as i32)
        .unwrap_or(1)
}

pub(super) fn foreground_color_span_color_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/style/ForegroundColorSpan;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "mColor".to_owned(),
    }
}

pub(super) fn background_color_span_color_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/style/BackgroundColorSpan;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "mColor".to_owned(),
    }
}

pub(super) fn text_paint_background_color_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/TextPaint;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "bgColor".to_owned(),
    }
}

pub(super) fn boring_layout_metrics_width_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/BoringLayout$Metrics;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "width".to_owned(),
    }
}

pub(super) fn paint_font_metrics_int_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/graphics/Paint$FontMetricsInt;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn text_layout_char_sequence_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        type_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn text_layout_paint_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        type_descriptor: "Landroid/text/TextPaint;".to_owned(),
        name: "paint".to_owned(),
    }
}

pub(super) fn text_layout_int_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn text_layout_float_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn text_layout_object_field(name: &str, type_descriptor: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn text_layout_alignment_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        type_descriptor: "Landroid/text/Layout$Alignment;".to_owned(),
        name: "alignment".to_owned(),
    }
}

pub(super) fn boring_layout_metrics_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/BoringLayout;".to_owned(),
        type_descriptor: "Landroid/text/BoringLayout$Metrics;".to_owned(),
        name: "metrics".to_owned(),
    }
}

pub(super) fn boring_layout_int_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/BoringLayout;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn boring_layout_truncate_at_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/BoringLayout;".to_owned(),
        type_descriptor: "Landroid/text/TextUtils$TruncateAt;".to_owned(),
        name: "ellipsize".to_owned(),
    }
}

pub(super) fn static_layout_builder_char_sequence_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        type_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn static_layout_builder_object_field(name: &str, type_descriptor: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn static_layout_builder_int_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn static_layout_builder_float_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn line_break_config_builder_int_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/graphics/text/LineBreakConfig$Builder;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    }
}

pub(super) fn line_break_config_int_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/graphics/text/LineBreakConfig;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    }
}

pub(in crate::dex_run) fn break_iterator_string_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/text/BreakIterator;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: name.to_owned(),
    }
}

pub(in crate::dex_run) fn break_iterator_object_field(
    name: &str,
    type_descriptor: &str,
) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/text/BreakIterator;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

pub(in crate::dex_run) fn break_iterator_int_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/text/BreakIterator;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    }
}

fn layout_alignment_names() -> &'static [&'static str] {
    &["ALIGN_NORMAL", "ALIGN_OPPOSITE", "ALIGN_CENTER"]
}

fn is_line_break_opportunity(unit: i32) -> bool {
    matches!(
        unit,
        0x0009 | 0x000a | 0x000b | 0x000c | 0x0020 | 0x0085 | 0x2028 | 0x2029
    )
}

fn boundary_already_added(boundaries: &[i32], boundary: i32) -> bool {
    boundaries.last().copied() == Some(boundary)
}

fn normalize_nullable_object(value: &DexValue) -> Option<DexValue> {
    match value {
        DexValue::Null | DexValue::Int(0) => Some(DexValue::Null),
        DexValue::Object(_) | DexValue::Reference(_) => Some(value.clone()),
        DexValue::Uninitialized
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_) => None,
    }
}

fn text_layout_line_starts(text: &str) -> Vec<i32> {
    let mut starts = vec![0];
    let mut offset = 0_i32;
    for unit in text.encode_utf16() {
        offset = offset.saturating_add(1);
        if unit == b'\n' as u16 {
            starts.push(offset);
        }
    }
    starts.push(offset);
    starts
}

fn text_layout_line_visible_end(text: &str, start: i32, end: i32) -> Option<i32> {
    if start < 0 || end < start {
        return None;
    }

    let units: Vec<u16> = text.encode_utf16().collect();
    let start = usize::try_from(start).ok()?;
    let mut end = usize::try_from(end).ok()?;
    if end > units.len() {
        return None;
    }

    while end > start {
        let unit = units[end - 1];
        if unit == b'\n' as u16 || unit == b'\r' as u16 {
            end -= 1;
        } else {
            break;
        }
    }

    i32::try_from(end).ok()
}

fn string_split_literal_delimiter(pattern: &str) -> Option<char> {
    // cite: libcore Pattern.java#fastSplit, FASTSPLIT_METACHARACTERS
    const METACHARACTERS: &str = "\\?*+[](){}^$.|";

    let mut chars = pattern.chars();
    let first = chars.next()?;
    let second = chars.next();
    if chars.next().is_some() {
        return None;
    }

    let delimiter = match (first, second) {
        (ch, None) if !METACHARACTERS.contains(ch) => ch,
        ('\\', Some(ch)) if METACHARACTERS.contains(ch) => ch,
        _ => return None,
    };
    if delimiter.len_utf16() != 1 {
        return None;
    }

    Some(delimiter)
}

// cite: libcore Pattern.java#split
fn string_pattern_split(
    regex: &JavaRegex,
    input: &str,
    limit: i32,
    target_sdk_version: i32,
) -> Option<Vec<String>> {
    let match_limited = limit > 0;
    let mut matches: Vec<String> = Vec::new();
    let mut index = 0;
    let mut search = 0;

    while search <= input.len() {
        let Some((start, end)) = regex.find_at(input, search)? else {
            break;
        };
        if !match_limited || (matches.len() as i32) < limit - 1 {
            let leading_empty_match = index == 0 && start == 0 && end == 0;
            if !(leading_empty_match && target_sdk_version > 28) {
                matches.push(input[index..start].to_owned());
                index = end;
            }
        } else if matches.len() as i32 == limit - 1 {
            matches.push(input[index..].to_owned());
            index = end;
        }
        search = match input[end..].chars().next() {
            Some(next) if start == end => end + next.len_utf8(),
            None if start == end => input.len() + 1,
            _ => end,
        };
    }

    if index == 0 {
        return Some(vec![input.to_owned()]);
    }
    if !match_limited || (matches.len() as i32) < limit {
        matches.push(input[index..].to_owned());
    }
    if limit == 0 {
        while matches.last().is_some_and(String::is_empty) {
            matches.pop();
        }
    }

    Some(matches)
}

fn string_fast_split(value: &str, delimiter: char, limit: i32) -> Vec<String> {
    // cite: libcore Pattern.java#fastSplit
    if value.is_empty() {
        return vec![String::new()];
    }

    let mut boundaries = Vec::new();
    for (index, _) in value.match_indices(delimiter) {
        if limit > 0 && boundaries.len() as i32 + 1 == limit {
            break;
        }
        boundaries.push(index);
    }

    let mut last_part_end = value.len();
    if limit == 0 {
        while boundaries
            .last()
            .is_some_and(|index| index + delimiter.len_utf8() == last_part_end)
        {
            last_part_end = boundaries.pop().unwrap_or(last_part_end);
        }
        if boundaries.is_empty() && last_part_end == 0 {
            return Vec::new();
        }
    }

    let mut parts = Vec::with_capacity(boundaries.len() + 1);
    let mut begin = 0;
    for index in boundaries {
        parts.push(value[begin..index].to_owned());
        begin = index + delimiter.len_utf8();
    }
    parts.push(value[begin..last_part_end].to_owned());

    parts
}

fn is_high_surrogate(unit: u16) -> bool {
    (0xd800..=0xdbff).contains(&unit)
}

fn is_low_surrogate(unit: u16) -> bool {
    (0xdc00..=0xdfff).contains(&unit)
}

fn boring_layout_metrics_height(program: &DexProgram, metrics: &DexValue) -> Option<i32> {
    if !dex_value_is_object_like(metrics) {
        return None;
    }

    let top = program.read_stored_instance_field(metrics, &paint_font_metrics_int_field("top"))?;
    let bottom =
        program.read_stored_instance_field(metrics, &paint_font_metrics_int_field("bottom"))?;
    let DexValue::Int(top) = top else {
        return None;
    };
    let DexValue::Int(bottom) = bottom else {
        return None;
    };

    Some(bottom.saturating_sub(top).max(0))
}

pub(super) fn system_get_property(invocation: &DexInvocation) -> Option<DexValue> {
    let key = match invocation.arguments.as_slice() {
        [DexValue::String(key)] => key,
        [DexValue::String(key), DexValue::String(_)] => key,
        [DexValue::String(key), DexValue::Null] => key,
        _ => return None,
    };

    if let Some(value) = bootstrap_system_property(key) {
        return Some(DexValue::String(std::rc::Rc::from(value)));
    }

    match invocation.arguments.as_slice() {
        [DexValue::String(_)] => Some(DexValue::Null),
        [DexValue::String(_), default_value] => Some(default_value.clone()),
        _ => None,
    }
}

pub(super) fn print_stream_write(invocation: &DexInvocation) -> Option<()> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    match &invocation.arguments[0] {
        DexValue::Object(_) | DexValue::Reference(_) => Some(()),
        DexValue::Uninitialized
        | DexValue::Null
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_) => None,
    }
}

// cite: libcore System.java#getProperties
pub(super) fn system_get_properties(program: &DexProgram) -> DexValue {
    const KEYS: [&str; 6] = [
        "java.version",
        "file.separator",
        "java.vm.name",
        "line.separator",
        "os.name",
        "path.separator",
    ];

    let properties = program.allocate_object("Ljava/util/Properties;".to_owned());
    let properties_key = map_identity_key(&properties);
    let mut runtime = program.runtime.borrow_mut();
    for key in KEYS {
        let Some(value) = bootstrap_system_property(key) else {
            continue;
        };
        let key_value = DexValue::String(std::rc::Rc::from(key));
        let entry_key = map_identity_key(&key_value);
        runtime
            .map_entry_keys
            .insert((properties_key.clone(), entry_key.clone()), key_value);
        runtime.map_entries.insert(
            (properties_key.clone(), entry_key),
            DexValue::String(std::rc::Rc::from(value)),
        );
    }
    drop(runtime);

    properties
}

pub(super) fn bootstrap_system_property(key: &str) -> Option<String> {
    match key {
        // cite: art ti_properties.cc#kProperties
        "java.version" => Some("0".to_owned()),
        "file.separator" => Some("/".to_owned()),
        "java.vm.name" => Some("Winedroid".to_owned()),
        "line.separator" => Some("\n".to_owned()),
        "os.name" => Some(std::env::consts::OS.to_owned()),
        "path.separator" => Some(":".to_owned()),
        _ => None,
    }
}

// cite: libcore Pattern.java#RemoveQEQuoting
pub(super) fn translate_java_regex_quoting(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();

    while let Some(current) = chars.next() {
        if current != '\\' {
            result.push(current);
            continue;
        }

        match chars.next() {
            Some('Q') => {
                let mut literal = String::new();
                while let Some(current) = chars.next() {
                    if current == '\\' && chars.peek() == Some(&'E') {
                        chars.next();
                        break;
                    }
                    literal.push(current);
                }
                result.push_str(&regex::escape(&literal));
            }
            Some(escaped) => {
                result.push('\\');
                result.push(escaped);
            }
            None => result.push('\\'),
        }
    }

    result
}
