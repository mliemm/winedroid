use super::*;

impl DexProgram {
    pub(in crate::dex_run) fn initialize_atomic_boolean(
        &self,
        invocation: &DexInvocation,
        initial_value: i32,
    ) -> Option<()> {
        let object = invocation.arguments.first()?;
        let object_key = map_identity_key(object);

        self.runtime
            .borrow_mut()
            .atomic_boolean_values
            .insert(object_key, normalize_atomic_boolean(initial_value));

        Some(())
    }

    pub(in crate::dex_run) fn atomic_boolean_get(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);

        Some(
            self.runtime
                .borrow()
                .atomic_boolean_values
                .get(&object_key)
                .copied()
                .unwrap_or(0),
        )
    }

    pub(in crate::dex_run) fn atomic_boolean_set(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.runtime
            .borrow_mut()
            .atomic_boolean_values
            .insert(object_key, normalize_atomic_boolean(value));

        Some(())
    }

    pub(in crate::dex_run) fn atomic_boolean_get_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(update) = invocation.arguments[1] else {
            return None;
        };
        let mut runtime = self.runtime.borrow_mut();
        let previous = runtime
            .atomic_boolean_values
            .insert(object_key, normalize_atomic_boolean(update))
            .unwrap_or(0);

        Some(previous)
    }

    pub(in crate::dex_run) fn atomic_boolean_compare_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(expected) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(update) = invocation.arguments[2] else {
            return None;
        };
        let expected = normalize_atomic_boolean(expected);
        let update = normalize_atomic_boolean(update);
        let mut runtime = self.runtime.borrow_mut();
        let current = runtime
            .atomic_boolean_values
            .get(&object_key)
            .copied()
            .unwrap_or(0);

        if current != expected {
            return Some(0);
        }

        runtime.atomic_boolean_values.insert(object_key, update);
        Some(1)
    }

    pub(in crate::dex_run) fn initialize_atomic_integer(
        &self,
        invocation: &DexInvocation,
        initial_value: i32,
    ) -> Option<()> {
        let object = invocation.arguments.first()?;
        let object_key = map_identity_key(object);

        self.runtime
            .borrow_mut()
            .atomic_integer_values
            .insert(object_key, initial_value);

        Some(())
    }

    pub(in crate::dex_run) fn atomic_integer_get(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);

        Some(
            self.runtime
                .borrow()
                .atomic_integer_values
                .get(&object_key)
                .copied()
                .unwrap_or(0),
        )
    }

    pub(in crate::dex_run) fn atomic_integer_set(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.runtime
            .borrow_mut()
            .atomic_integer_values
            .insert(object_key, value);

        Some(())
    }

    pub(in crate::dex_run) fn atomic_integer_add(
        &self,
        invocation: &DexInvocation,
    ) -> Option<(i32, i32)> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(delta) = invocation.arguments[1] else {
            return None;
        };
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime.atomic_integer_values.entry(object_key).or_insert(0);
        let previous = *value;

        *value = (*value).checked_add(delta)?;

        Some((previous, *value))
    }

    pub(in crate::dex_run) fn atomic_integer_compare_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(expected) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(update) = invocation.arguments[2] else {
            return None;
        };
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime.atomic_integer_values.entry(object_key).or_insert(0);

        if *value != expected {
            return Some(0);
        }

        *value = update;
        Some(1)
    }

    pub(in crate::dex_run) fn increment_atomic_integer(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        let object = invocation.arguments.first()?;
        let object_key = map_identity_key(object);
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime.atomic_integer_values.entry(object_key).or_insert(0);

        *value = (*value).checked_add(1)?;

        Some(*value)
    }

    pub(in crate::dex_run) fn get_and_increment_atomic_integer(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        let object = invocation.arguments.first()?;
        let object_key = map_identity_key(object);
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime.atomic_integer_values.entry(object_key).or_insert(0);
        let previous = *value;

        *value = (*value).checked_add(1)?;

        Some(previous)
    }

    pub(in crate::dex_run) fn decrement_atomic_integer(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        let object = invocation.arguments.first()?;
        let object_key = map_identity_key(object);
        let mut runtime = self.runtime.borrow_mut();
        let value = runtime.atomic_integer_values.entry(object_key).or_insert(0);

        *value = (*value).checked_sub(1)?;

        Some(*value)
    }

    pub(in crate::dex_run) fn count_down_latch(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.atomic_integer_values.get_mut(&object_key)?;

        if *state > 0 {
            *state -= 1;
        }

        Some(())
    }

    pub(in crate::dex_run) fn initialize_atomic_reference(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let object_key = atomic_reference_key(&invocation.arguments[0])?;
        let value = atomic_reference_value(&invocation.arguments[1])?;

        self.runtime
            .borrow_mut()
            .atomic_reference_values
            .insert(object_key, value);

        Some(())
    }

    pub(in crate::dex_run) fn initialize_empty_atomic_reference(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let object_key = atomic_reference_key(&invocation.arguments[0])?;

        self.runtime
            .borrow_mut()
            .atomic_reference_values
            .insert(object_key, DexValue::Null);

        Some(())
    }

    pub(in crate::dex_run) fn atomic_reference_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let object_key = atomic_reference_key(&invocation.arguments[0])?;

        Some(
            self.runtime
                .borrow()
                .atomic_reference_values
                .get(&object_key)
                .cloned()
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn atomic_reference_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let object_key = atomic_reference_key(&invocation.arguments[0])?;

        let value = atomic_reference_value(&invocation.arguments[1])?;

        self.runtime
            .borrow_mut()
            .atomic_reference_values
            .insert(object_key, value);

        Some(())
    }

    pub(in crate::dex_run) fn atomic_reference_get_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let object_key = atomic_reference_key(&invocation.arguments[0])?;
        let update = atomic_reference_value(&invocation.arguments[1])?;
        let mut runtime = self.runtime.borrow_mut();
        let previous = runtime
            .atomic_reference_values
            .insert(object_key, update)
            .unwrap_or(DexValue::Null);

        Some(previous)
    }

    pub(in crate::dex_run) fn atomic_reference_compare_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let object_key = atomic_reference_key(&invocation.arguments[0])?;
        let expected = atomic_reference_value(&invocation.arguments[1])?;
        let update = atomic_reference_value(&invocation.arguments[2])?;
        let mut runtime = self.runtime.borrow_mut();
        let current = runtime
            .atomic_reference_values
            .get(&object_key)
            .cloned()
            .unwrap_or(DexValue::Null);

        if current != expected {
            return Some(0);
        }

        runtime.atomic_reference_values.insert(object_key, update);
        Some(1)
    }

    pub(in crate::dex_run) fn initialize_atomic_reference_array(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let size = usize::try_from(primitive_int_argument(&invocation.arguments[1])?).ok()?;
        let array_key = atomic_reference_key(&invocation.arguments[0])?;

        self.runtime
            .borrow_mut()
            .array_entries
            .insert(array_key, vec![DexValue::Null; size]);

        Some(())
    }

    pub(in crate::dex_run) fn atomic_reference_array_length(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.array_length(&invocation.arguments[0])
    }

    pub(in crate::dex_run) fn atomic_reference_array_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let index = primitive_int_argument(&invocation.arguments[1])?;

        self.read_array_value(&invocation.arguments[0], index)
    }

    pub(in crate::dex_run) fn atomic_reference_array_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let index = primitive_int_argument(&invocation.arguments[1])?;
        let value = atomic_reference_value(&invocation.arguments[2])?;

        self.write_array_value(&invocation.arguments[0], index, value)
    }

    pub(in crate::dex_run) fn atomic_reference_array_get_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let array_key = atomic_reference_key(&invocation.arguments[0])?;
        let index = usize::try_from(primitive_int_argument(&invocation.arguments[1])?).ok()?;
        let value = atomic_reference_value(&invocation.arguments[2])?;
        let mut runtime = self.runtime.borrow_mut();
        let slot = runtime.array_entries.get_mut(&array_key)?.get_mut(index)?;
        let previous = slot.clone();
        *slot = value;

        Some(previous)
    }

    pub(in crate::dex_run) fn atomic_reference_array_compare_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let array_key = atomic_reference_key(&invocation.arguments[0])?;
        let index = usize::try_from(primitive_int_argument(&invocation.arguments[1])?).ok()?;
        let expected = atomic_reference_value(&invocation.arguments[2])?;
        let update = atomic_reference_value(&invocation.arguments[3])?;
        let mut runtime = self.runtime.borrow_mut();
        let slot = runtime.array_entries.get_mut(&array_key)?.get_mut(index)?;

        if *slot != expected {
            return Some(0);
        }

        *slot = update;
        Some(1)
    }

    pub(in crate::dex_run) fn atomic_reference_field_updater_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let (target_descriptor, value_descriptor, field_name) =
            atomic_reference_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(DexValue::Null);
        }

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: value_descriptor,
            name: field_name,
        };

        Some(
            self.read_stored_instance_field(target, &field)
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn atomic_reference_field_updater_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let (target_descriptor, value_descriptor, field_name) =
            atomic_reference_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(());
        }
        let value = atomic_reference_value(&invocation.arguments[2])?;

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: value_descriptor,
            name: field_name,
        };

        self.write_stored_instance_field(target, &field, value)
            .then_some(())
    }

    pub(in crate::dex_run) fn atomic_reference_field_updater_get_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        // cite: libcore AtomicReferenceFieldUpdater.java#getAndSet
        if invocation.arguments.len() != 3 {
            return None;
        }

        let (target_descriptor, value_descriptor, field_name) =
            atomic_reference_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(DexValue::Null);
        }
        let value = atomic_reference_value(&invocation.arguments[2])?;
        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: value_descriptor,
            name: field_name,
        };
        let previous = self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))?;
        self.write_stored_instance_field(target, &field, value)
            .then_some(previous)
    }

    pub(in crate::dex_run) fn atomic_reference_field_updater_compare_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let (target_descriptor, value_descriptor, field_name) =
            atomic_reference_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(0);
        }
        let expected = atomic_reference_value(&invocation.arguments[2])?;
        let update = atomic_reference_value(&invocation.arguments[3])?;

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: value_descriptor,
            name: field_name,
        };
        let current = self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))?;

        if current != expected {
            return Some(0);
        }

        self.write_stored_instance_field(target, &field, update)
            .then_some(1)
    }

    pub(in crate::dex_run) fn atomic_integer_field_updater_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_integer_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(0);
        }

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "I".to_owned(),
            name: field_name,
        };

        match self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))
        {
            Some(DexValue::Int(value)) => Some(value),
            _ => None,
        }
    }

    pub(in crate::dex_run) fn atomic_integer_field_updater_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_integer_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(());
        }
        let DexValue::Int(value) = invocation.arguments[2] else {
            return None;
        };

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "I".to_owned(),
            name: field_name,
        };

        self.write_stored_instance_field(target, &field, DexValue::Int(value))
            .then_some(())
    }

    pub(in crate::dex_run) fn atomic_integer_field_updater_compare_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_integer_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(0);
        }
        let DexValue::Int(expected) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Int(update) = invocation.arguments[3] else {
            return None;
        };

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "I".to_owned(),
            name: field_name,
        };
        let current = match self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))
        {
            Some(DexValue::Int(value)) => value,
            _ => return None,
        };

        if current != expected {
            return Some(0);
        }

        self.write_stored_instance_field(target, &field, DexValue::Int(update))
            .then_some(1)
    }

    pub(in crate::dex_run) fn atomic_integer_field_updater_increment_and_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let (_current, next) = self.atomic_integer_field_updater_add(invocation, 1)?;

        Some(next)
    }

    pub(in crate::dex_run) fn atomic_integer_field_updater_get_and_increment(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let (current, _next) = self.atomic_integer_field_updater_add(invocation, 1)?;

        Some(current)
    }

    pub(in crate::dex_run) fn atomic_integer_field_updater_get_and_decrement(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let (current, _next) = self.atomic_integer_field_updater_add(invocation, -1)?;

        Some(current)
    }

    pub(in crate::dex_run) fn atomic_integer_field_updater_add_and_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 3 {
            return None;
        }
        let DexValue::Int(delta) = invocation.arguments[2] else {
            return None;
        };

        let (_current, next) = self.atomic_integer_field_updater_add(invocation, delta)?;

        Some(next)
    }

    pub(in crate::dex_run) fn atomic_integer_field_updater_decrement_and_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let (_current, next) = self.atomic_integer_field_updater_add(invocation, -1)?;

        Some(next)
    }

    fn atomic_integer_field_updater_add(
        &self,
        invocation: &DexInvocation,
        delta: i32,
    ) -> Option<(i32, i32)> {
        if invocation.arguments.len() < 2 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_integer_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some((0, 0));
        }

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "I".to_owned(),
            name: field_name,
        };
        let current = match self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))
        {
            Some(DexValue::Int(value)) => value,
            _ => return None,
        };
        let next = current.checked_add(delta)?;

        self.write_stored_instance_field(target, &field, DexValue::Int(next))
            .then_some((current, next))
    }

    pub(in crate::dex_run) fn atomic_long_field_updater_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_long_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(0);
        }

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "J".to_owned(),
            name: field_name,
        };

        match self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))
        {
            Some(DexValue::Long(value)) => Some(value),
            Some(DexValue::Int(value)) => Some(i64::from(value)),
            _ => None,
        }
    }

    pub(in crate::dex_run) fn atomic_long_field_updater_get_and_increment(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        let (current, _next) = self.atomic_long_field_updater_add(invocation, 1)?;

        Some(current)
    }

    pub(in crate::dex_run) fn atomic_long_field_updater_increment_and_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        let (_current, next) = self.atomic_long_field_updater_add(invocation, 1)?;

        Some(next)
    }

    pub(in crate::dex_run) fn atomic_long_field_updater_get_and_decrement(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        let (current, _next) = self.atomic_long_field_updater_add(invocation, -1)?;

        Some(current)
    }

    pub(in crate::dex_run) fn atomic_long_field_updater_decrement_and_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        let (_current, next) = self.atomic_long_field_updater_add(invocation, -1)?;

        Some(next)
    }

    fn atomic_long_field_updater_add(
        &self,
        invocation: &DexInvocation,
        delta: i64,
    ) -> Option<(i64, i64)> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_long_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some((0, 0));
        }

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "J".to_owned(),
            name: field_name,
        };
        let current = match self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))
        {
            Some(DexValue::Long(value)) => value,
            Some(DexValue::Int(value)) => i64::from(value),
            _ => return None,
        };
        let next = current.checked_add(delta)?;

        self.write_stored_instance_field(target, &field, DexValue::Long(next))
            .then_some((current, next))
    }

    pub(in crate::dex_run) fn atomic_long_field_updater_add_and_get(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_long_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(0);
        }
        let DexValue::Long(delta) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Long(delta_high_slot) = invocation.arguments[3] else {
            return None;
        };
        if delta != delta_high_slot {
            return None;
        }

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "J".to_owned(),
            name: field_name,
        };
        let current = match self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))
        {
            Some(DexValue::Long(value)) => value,
            Some(DexValue::Int(value)) => i64::from(value),
            _ => return None,
        };
        let next = current.checked_add(delta)?;

        self.write_stored_instance_field(target, &field, DexValue::Long(next))
            .then_some(next)
    }

    pub(in crate::dex_run) fn atomic_long_field_updater_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_long_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(());
        }
        let DexValue::Long(value) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Long(high_slot_value) = invocation.arguments[3] else {
            return None;
        };
        if value != high_slot_value {
            return None;
        }

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "J".to_owned(),
            name: field_name,
        };

        self.write_stored_instance_field(target, &field, DexValue::Long(value))
            .then_some(())
    }

    pub(in crate::dex_run) fn atomic_long_field_updater_compare_and_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 6 {
            return None;
        }

        let (target_descriptor, field_name) =
            atomic_long_field_updater_parts(&invocation.arguments[0])?;
        let target = &invocation.arguments[1];
        let class_descriptor = value_class_descriptor(target)?;
        if !self.descriptor_matches_type(class_descriptor, &target_descriptor) {
            return Some(0);
        }
        let DexValue::Long(expected) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Long(expected_high_slot) = invocation.arguments[3] else {
            return None;
        };
        let DexValue::Long(update) = invocation.arguments[4] else {
            return None;
        };
        let DexValue::Long(update_high_slot) = invocation.arguments[5] else {
            return None;
        };
        if expected != expected_high_slot || update != update_high_slot {
            return None;
        }

        let field = DexFieldId {
            class_descriptor: target_descriptor,
            type_descriptor: "J".to_owned(),
            name: field_name,
        };
        let current = match self
            .read_stored_instance_field(target, &field)
            .or_else(|| default_field_value(&field))
        {
            Some(DexValue::Long(value)) => value,
            Some(DexValue::Int(value)) => i64::from(value),
            _ => return None,
        };

        if current != expected {
            return Some(0);
        }

        self.write_stored_instance_field(target, &field, DexValue::Long(update))
            .then_some(1)
    }

    pub(in crate::dex_run) fn initialize_weak_reference(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        let value = invocation.arguments[1].clone();

        self.runtime
            .borrow_mut()
            .weak_reference_values
            .insert(object_key, value);

        Some(())
    }

    pub(in crate::dex_run) fn initialize_weak_reference_with_queue(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !matches!(
            invocation.arguments[2],
            DexValue::Object(_) | DexValue::Reference(_) | DexValue::Null
        ) {
            return None;
        }

        let weak_reference_invocation = DexInvocation {
            arguments: smallvec::smallvec![
                invocation.arguments[0].clone(),
                invocation.arguments[1].clone(),
            ],
            ..invocation.clone()
        };

        self.initialize_weak_reference(&weak_reference_invocation)
    }

    // cite: libcore Reference.java#clear
    pub(in crate::dex_run) fn weak_reference_clear(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);
        self.runtime
            .borrow_mut()
            .weak_reference_values
            .insert(object_key, DexValue::Null);

        Some(())
    }

    pub(in crate::dex_run) fn weak_reference_get(
        &self,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexValue> {
        let Some(invocation) = invocation else {
            return Some(DexValue::Reference(std::rc::Rc::from(
                "Landroidx/lifecycle/LifecycleOwner;",
            )));
        };

        if invocation.arguments.len() != 1 {
            return None;
        }

        let object_key = map_identity_key(&invocation.arguments[0]);

        Some(
            self.runtime
                .borrow()
                .weak_reference_values
                .get(&object_key)
                .cloned()
                .unwrap_or(DexValue::Reference(std::rc::Rc::from(
                    "Landroidx/lifecycle/LifecycleOwner;",
                ))),
        )
    }

    pub(in crate::dex_run) fn initialize_handler(&self, invocation: &DexInvocation) -> Option<()> {
        let handler_key = map_identity_key(&invocation.arguments[0]);
        let looper = match invocation.arguments.as_slice() {
            [_handler] => DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main")),
            [_handler, DexValue::Object(_) | DexValue::Reference(_)] => {
                invocation.arguments[1].clone()
            }
            _ => return None,
        };

        self.runtime
            .borrow_mut()
            .handler_loopers
            .insert(handler_key, looper);

        Some(())
    }

    pub(in crate::dex_run) fn initialize_handler_with_callback(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        let [handler, _, callback] = invocation.arguments.as_slice() else {
            return None;
        };
        let looper_only = DexInvocation {
            arguments: invocation.arguments.iter().take(2).cloned().collect(),
            ..invocation.clone()
        };
        self.initialize_handler(&looper_only)?;

        // cite: frameworks-base Handler.java#Handler, mCallback
        self.write_stored_instance_field(
            handler,
            &DexFieldId {
                class_descriptor: "Landroid/os/Handler;".to_owned(),
                type_descriptor: "Landroid/os/Handler$Callback;".to_owned(),
                name: "mCallback".to_owned(),
            },
            callback.clone(),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn create_async_handler(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let looper = match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[0].clone(),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        Some(self.create_handler_for_looper(looper))
    }

    pub(in crate::dex_run) fn create_handler_for_looper(&self, looper: DexValue) -> DexValue {
        let handler = self.allocate_object("Landroid/os/Handler;".to_owned());
        let handler_key = map_identity_key(&handler);

        self.runtime
            .borrow_mut()
            .handler_loopers
            .insert(handler_key, looper);

        handler
    }

    pub(in crate::dex_run) fn handler_looper(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let handler_key = map_identity_key(&invocation.arguments[0]);

        Some(
            self.runtime
                .borrow()
                .handler_loopers
                .get(&handler_key)
                .cloned()
                .unwrap_or(DexValue::Reference(std::rc::Rc::from(
                    "Landroid/os/Looper;:main",
                ))),
        )
    }

    pub(in crate::dex_run) fn post_handler_runnable(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let handler_key = map_identity_key(&invocation.arguments[0]);
        let runnable = match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        self.runtime
            .borrow_mut()
            .post_handler_runnable(handler_key, runnable, 0, false);

        Some(())
    }

    pub(in crate::dex_run) fn executor_execute(&self, invocation: &DexInvocation) -> Option<()> {
        let executor = invocation.arguments.first()?;

        // cite: frameworks-base HandlerExecutor.java#execute
        // cite: libcore ThreadPoolExecutor.java#execute
        let known_executor = match executor {
            DexValue::Reference(executor) => {
                executor.as_ref() == "Ljava/util/concurrent/Executor;.main"
                    || is_executors_factory_reference(executor)
            }
            DexValue::Object(_) => self.is_thread_pool_executor(executor),
            _ => false,
        };
        if !known_executor {
            return None;
        }

        self.post_handler_runnable(invocation)
    }

    // cite: libcore ThreadPoolExecutor.java#isShutdown, runStateAtLeast
    // cite: libcore ExecutorService.java#isShutdown
    pub(in crate::dex_run) fn executor_service_is_shutdown(
        &self,
        invocation: &DexInvocation,
    ) -> Option<bool> {
        let executor = invocation.arguments.first()?;
        let known_service = match executor {
            DexValue::Reference(executor) => is_executors_factory_reference(executor),
            DexValue::Object(_) => self.is_thread_pool_executor(executor),
            _ => false,
        };

        known_service.then_some(false)
    }

    pub(in crate::dex_run) fn is_thread_pool_executor(&self, value: &DexValue) -> bool {
        matches!(value, DexValue::Object(_))
            && self.value_matches_type(value, "Ljava/util/concurrent/ThreadPoolExecutor;")
    }

    pub(in crate::dex_run) fn thread_pool_executor_init(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: libcore ThreadPoolExecutor.java#ThreadPoolExecutor, defaultHandler, keepAliveTime
        let (pool_arguments, handler) = match invocation.arguments.as_slice() {
            [pool_arguments @ .., handler] if pool_arguments.len() == 8 => {
                (pool_arguments, Some(handler))
            }
            pool_arguments if pool_arguments.len() == 8 => (pool_arguments, None),
            _ => return None,
        };
        let [receiver, DexValue::Int(core), DexValue::Int(maximum), DexValue::Long(keep_alive), DexValue::Long(keep_alive_high), unit, work_queue, thread_factory] =
            pool_arguments
        else {
            return None;
        };
        if keep_alive != keep_alive_high || !dex_value_is_object_like(receiver) {
            return None;
        }
        if *core < 0 || *maximum <= 0 || maximum < core || *keep_alive < 0 {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
            ));
        }
        if *work_queue == DexValue::Null
            || *thread_factory == DexValue::Null
            || handler == Some(&DexValue::Null)
            || *unit == DexValue::Null
        {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/NullPointerException;".to_owned()),
            ));
        }
        let keep_alive_nanos = scale_to_nanos(time_unit_reference_name(unit)?, *keep_alive)?;

        self.store_thread_pool_executor_state(
            receiver,
            ThreadPoolExecutorState {
                core: *core,
                maximum: *maximum,
                keep_alive_nanos,
                work_queue,
                thread_factory,
                handler,
            },
        );

        Some(DexExecutionResult::Void)
    }

    pub(in crate::dex_run) fn scheduled_thread_pool_executor_init(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: libcore ScheduledThreadPoolExecutor.java#ScheduledThreadPoolExecutor, DEFAULT_KEEPALIVE_MILLIS, DelayedWorkQueue
        let [receiver, DexValue::Int(core), thread_factory] = invocation.arguments.as_slice()
        else {
            return None;
        };
        if !dex_value_is_object_like(receiver) {
            return None;
        }
        if *core < 0 {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
            ));
        }
        if *thread_factory == DexValue::Null {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/NullPointerException;".to_owned()),
            ));
        }
        let work_queue = self.allocate_object(
            "Ljava/util/concurrent/ScheduledThreadPoolExecutor$DelayedWorkQueue;".to_owned(),
        );

        self.store_thread_pool_executor_state(
            receiver,
            ThreadPoolExecutorState {
                core: *core,
                maximum: i32::MAX,
                keep_alive_nanos: scale_to_nanos("MILLISECONDS", 10)?,
                work_queue: &work_queue,
                thread_factory,
                handler: None,
            },
        );

        Some(DexExecutionResult::Void)
    }

    fn store_thread_pool_executor_state(
        &self,
        receiver: &DexValue,
        state: ThreadPoolExecutorState,
    ) {
        self.write_stored_instance_field(
            receiver,
            &thread_pool_executor_core_field(),
            DexValue::Int(state.core),
        );
        self.write_stored_instance_field(
            receiver,
            &thread_pool_executor_maximum_field(),
            DexValue::Int(state.maximum),
        );
        self.write_stored_instance_field(
            receiver,
            &thread_pool_executor_keep_alive_field(),
            DexValue::Long(state.keep_alive_nanos),
        );
        self.write_stored_instance_field(
            receiver,
            &thread_pool_executor_work_queue_field(),
            state.work_queue.clone(),
        );
        self.write_stored_instance_field(
            receiver,
            &thread_pool_executor_thread_factory_field(),
            state.thread_factory.clone(),
        );
        if let Some(handler) = state.handler {
            self.write_stored_instance_field(
                receiver,
                &thread_pool_executor_handler_field(),
                handler.clone(),
            );
        }
    }

    pub(in crate::dex_run) fn thread_pool_executor_allow_core_thread_time_out(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: libcore ThreadPoolExecutor.java#allowCoreThreadTimeOut, interruptIdleWorkers, keepAliveTime
        let [receiver, DexValue::Int(flag)] = invocation.arguments.as_slice() else {
            return None;
        };
        if !dex_value_is_object_like(receiver) {
            return None;
        }
        let value = i32::from(*flag != 0);
        let keep_alive_nanos = self.thread_pool_executor_keep_alive_nanos(receiver)?;
        if value != 0 && keep_alive_nanos <= 0 {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
                    detail_message: Some(
                        "Core threads must have nonzero keep alive times".to_owned(),
                    ),
                }),
            ));
        }
        let field = thread_pool_executor_allow_core_thread_time_out_field();
        let current = match self.read_stored_instance_field(receiver, &field) {
            Some(DexValue::Int(current)) => current,
            _ => 0,
        };
        if value != current {
            self.write_stored_instance_field(receiver, &field, DexValue::Int(value));
        }

        Some(DexExecutionResult::Void)
    }

    pub(in crate::dex_run) fn scheduled_thread_pool_executor_schedule_runnable(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: libcore ScheduledThreadPoolExecutor.java#schedule, triggerTime, delayedExecute
        let [executor, command, DexValue::Long(delay), DexValue::Long(delay_high), unit] =
            invocation.arguments.as_slice()
        else {
            return None;
        };
        if delay != delay_high || !self.is_thread_pool_executor(executor) {
            return None;
        }
        if is_android_null_value(command) || is_android_null_value(unit) {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/NullPointerException;".to_owned()),
            ));
        }
        if !dex_value_is_object_like(command) {
            return None;
        }
        let trigger_nanos = scale_to_nanos(time_unit_reference_name(unit)?, (*delay).max(0))?;
        let delay_millis = trigger_nanos.saturating_add(999_999) / 1_000_000;

        // cite: libcore ScheduledThreadPoolExecutor.java#ScheduledFutureTask
        let task = self.allocate_object(
            "Ljava/util/concurrent/ScheduledThreadPoolExecutor$ScheduledFutureTask;".to_owned(),
        );
        self.write_stored_instance_field(&task, &future_task_runnable_field(), command.clone());
        self.write_stored_instance_field(
            &task,
            &future_task_runnable_result_field(),
            DexValue::Null,
        );
        self.runtime.borrow_mut().post_handler_runnable(
            map_identity_key(executor),
            task.clone(),
            delay_millis,
            false,
        );

        Some(DexExecutionResult::Value(task))
    }

    pub(in crate::dex_run) fn thread_pool_executor_set_keep_alive_time(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: libcore ThreadPoolExecutor.java#setKeepAliveTime, allowsCoreThreadTimeOut, interruptIdleWorkers
        let [receiver, DexValue::Long(time), DexValue::Long(time_high), unit] =
            invocation.arguments.as_slice()
        else {
            return None;
        };
        if time != time_high || !self.is_thread_pool_executor(receiver) {
            return None;
        }
        if *time < 0 {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
            ));
        }
        let allows_core_time_out = matches!(
            self.read_stored_instance_field(
                receiver,
                &thread_pool_executor_allow_core_thread_time_out_field()
            ),
            Some(DexValue::Int(flag)) if flag != 0
        );
        if *time == 0 && allows_core_time_out {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
                    detail_message: Some(
                        "Core threads must have nonzero keep alive times".to_owned(),
                    ),
                }),
            ));
        }
        if *unit == DexValue::Null {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/NullPointerException;".to_owned()),
            ));
        }
        let keep_alive_nanos = scale_to_nanos(time_unit_reference_name(unit)?, *time)?;
        self.write_stored_instance_field(
            receiver,
            &thread_pool_executor_keep_alive_field(),
            DexValue::Long(keep_alive_nanos),
        );

        Some(DexExecutionResult::Void)
    }

    fn thread_pool_executor_keep_alive_nanos(&self, receiver: &DexValue) -> Option<i64> {
        if let Some(DexValue::Long(nanos)) =
            self.read_stored_instance_field(receiver, &thread_pool_executor_keep_alive_field())
        {
            return Some(nanos);
        }
        let DexValue::Reference(reference) = receiver else {
            return None;
        };

        // cite: libcore Executors.java#newFixedThreadPool, newCachedThreadPool
        if reference.starts_with("Ljava/util/concurrent/ExecutorService;.fixed/") {
            return Some(0);
        }
        if reference.as_ref() == "Ljava/util/concurrent/ExecutorService;.cached" {
            return scale_to_nanos("SECONDS", 60);
        }

        None
    }

    // cite: libcore FutureTask.java#FutureTask
    pub(in crate::dex_run) fn future_task_init(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        if !dex_value_is_object_like(&invocation.arguments[1]) {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &future_task_callable_field(),
            invocation.arguments[1].clone(),
        );

        Some(())
    }

    // cite: libcore FutureTask.java#FutureTask
    pub(in crate::dex_run) fn future_task_init_runnable(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        if !dex_value_is_object_like(&invocation.arguments[1]) {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &future_task_runnable_field(),
            invocation.arguments[1].clone(),
        );
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &future_task_runnable_result_field(),
            invocation.arguments[2].clone(),
        );

        Some(())
    }

    // cite: libcore FutureTask.java#cancel
    pub(in crate::dex_run) fn future_task_cancel(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }
        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(_may_interrupt) = invocation.arguments[1] else {
            return None;
        };

        let receiver = &invocation.arguments[0];
        let already_done = self.read_stored_instance_field(receiver, &future_task_done_field())
            == Some(DexValue::Int(1));
        if already_done {
            return Some(0);
        }

        self.write_stored_instance_field(receiver, &future_task_done_field(), DexValue::Int(1));
        self.write_stored_instance_field(
            receiver,
            &future_task_cancelled_field(),
            DexValue::Int(1),
        );

        Some(1)
    }

    // cite: libcore FutureTask.java#isCancelled
    pub(in crate::dex_run) fn future_task_is_cancelled(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }
        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            match self.read_stored_instance_field(
                &invocation.arguments[0],
                &future_task_cancelled_field(),
            ) {
                Some(DexValue::Int(1)) => 1,
                _ => 0,
            },
        )
    }

    // cite: libcore FutureTask.java#isDone
    pub(in crate::dex_run) fn future_task_is_done(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }
        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            match self
                .read_stored_instance_field(&invocation.arguments[0], &future_task_done_field())
            {
                Some(DexValue::Int(1)) => 1,
                _ => 0,
            },
        )
    }

    pub(in crate::dex_run) fn post_handler_runnable_at_front(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let handler_key = map_identity_key(&invocation.arguments[0]);
        let runnable = match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        self.runtime
            .borrow_mut()
            .post_handler_runnable(handler_key, runnable, 0, true);

        Some(())
    }

    pub(in crate::dex_run) fn post_delayed_handler_runnable(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let handler_key = map_identity_key(&invocation.arguments[0]);
        let runnable = match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };
        let DexValue::Long(delay_millis) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Long(high_slot_value) = invocation.arguments[3] else {
            return None;
        };

        if high_slot_value != delay_millis {
            return None;
        }

        self.runtime
            .borrow_mut()
            .post_handler_runnable(handler_key, runnable, delay_millis, false);

        Some(())
    }

    // cite: frameworks-base Handler.java#removeCallbacks
    // cite: frameworks-base MessageQueue.java#removeMessages
    pub(in crate::dex_run) fn remove_handler_callbacks(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let handler_key = map_identity_key(&invocation.arguments[0]);
        let runnable = match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        let runnable_key = map_identity_key(&runnable);
        self.runtime.borrow_mut().handler_posts.retain(|post| {
            post.handler_key != handler_key || map_identity_key(&post.runnable) != runnable_key
        });

        Some(())
    }

    pub(in crate::dex_run) fn post_choreographer_frame_callback(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2
            || !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        self.store_choreographer_frame_callback(invocation.arguments[1].clone(), 0)
    }

    pub(in crate::dex_run) fn post_delayed_choreographer_frame_callback(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let DexValue::Long(delay_millis) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Long(high_slot_value) = invocation.arguments[3] else {
            return None;
        };
        if delay_millis != high_slot_value {
            return None;
        }
        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        self.store_choreographer_frame_callback(invocation.arguments[1].clone(), delay_millis)
    }

    pub(in crate::dex_run) fn remove_choreographer_frame_callback(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2
            || !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        self.runtime
            .borrow_mut()
            .remove_choreographer_frame_callback(&invocation.arguments[1]);

        Some(())
    }

    pub(in crate::dex_run) fn post_choreographer_callback(
        &self,
        invocation: &DexInvocation,
        delayed: bool,
    ) -> Option<()> {
        let expected_len = if delayed { 6 } else { 4 };
        if invocation.arguments.len() != expected_len
            || !dex_value_is_object_like(&invocation.arguments[0])
        {
            return None;
        }
        let DexValue::Int(callback_type) = invocation.arguments[1] else {
            return None;
        };
        if !(0..=4).contains(&callback_type) || !dex_value_is_object_like(&invocation.arguments[2])
        {
            return None;
        }
        let delay_millis = if delayed {
            let DexValue::Long(delay_millis) = invocation.arguments[4] else {
                return None;
            };
            let DexValue::Long(high_slot_value) = invocation.arguments[5] else {
                return None;
            };
            if delay_millis != high_slot_value {
                return None;
            }
            delay_millis
        } else {
            0
        };

        // cite: frameworks-base Choreographer.java#postCallback, postCallbackDelayedInternal
        self.runtime
            .borrow_mut()
            .post_choreographer_runnable_callback(
                callback_type,
                invocation.arguments[2].clone(),
                invocation.arguments[3].clone(),
                delay_millis,
            );

        Some(())
    }

    pub(in crate::dex_run) fn remove_choreographer_callbacks(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 4 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(callback_type) = invocation.arguments[1] else {
            return None;
        };
        if !(0..=4).contains(&callback_type) {
            return None;
        }

        // cite: frameworks-base Choreographer.java#removeCallbacks, removeCallbacksInternal
        self.runtime
            .borrow_mut()
            .remove_choreographer_runnable_callbacks(
                callback_type,
                &invocation.arguments[2],
                &invocation.arguments[3],
            );

        Some(())
    }

    fn store_choreographer_frame_callback(
        &self,
        callback: DexValue,
        delay_millis: i64,
    ) -> Option<()> {
        // cite: frameworks-base Choreographer.java#postFrameCallbackDelayed, postCallbackDelayedInternal
        self.runtime
            .borrow_mut()
            .post_choreographer_frame_callback(callback, delay_millis);

        Some(())
    }

    // cite: frameworks-base Handler.java#obtainMessage
    // cite: frameworks-base Message.java#obtain
    pub(in crate::dex_run) fn handler_obtain_message(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let (handler, arguments) = invocation.arguments.split_first()?;
        if !dex_value_is_object_like(handler) {
            return None;
        }
        let (what, arg1, arg2, object) = match arguments {
            [] => (0, 0, 0, DexValue::Null),
            [DexValue::Int(what)] => (*what, 0, 0, DexValue::Null),
            [DexValue::Int(what), object] => (*what, 0, 0, message_object_argument(object)?),
            [DexValue::Int(what), DexValue::Int(arg1), DexValue::Int(arg2)] => {
                (*what, *arg1, *arg2, DexValue::Null)
            }
            [DexValue::Int(what), DexValue::Int(arg1), DexValue::Int(arg2), object] => {
                (*what, *arg1, *arg2, message_object_argument(object)?)
            }
            _ => return None,
        };

        let message = self.allocate_object("Landroid/os/Message;".to_owned());
        self.write_stored_instance_field(&message, &message_int_field("what"), DexValue::Int(what));
        self.write_stored_instance_field(&message, &message_int_field("arg1"), DexValue::Int(arg1));
        self.write_stored_instance_field(&message, &message_int_field("arg2"), DexValue::Int(arg2));
        self.write_stored_instance_field(&message, &message_object_field("obj"), object);
        self.write_stored_instance_field(
            &message,
            &message_handler_field("target"),
            handler.clone(),
        );

        Some(message)
    }
}

fn message_object_argument(value: &DexValue) -> Option<DexValue> {
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

pub(in crate::dex_run) fn message_int_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/os/Message;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    }
}

pub(in crate::dex_run) fn message_object_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/os/Message;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: name.to_owned(),
    }
}

pub(in crate::dex_run) fn message_handler_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/os/Message;".to_owned(),
        type_descriptor: "Landroid/os/Handler;".to_owned(),
        name: name.to_owned(),
    }
}

pub(in crate::dex_run) fn atomic_reference_key(value: &DexValue) -> Option<String> {
    match value {
        DexValue::Object(_) | DexValue::Reference(_) => Some(map_identity_key(value)),
        DexValue::Uninitialized
        | DexValue::Null
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_) => None,
    }
}

pub(in crate::dex_run) fn atomic_reference_value(value: &DexValue) -> Option<DexValue> {
    match value {
        DexValue::Null | DexValue::Int(0) => Some(DexValue::Null),
        DexValue::String(_) | DexValue::Object(_) | DexValue::Reference(_) => Some(value.clone()),
        DexValue::Uninitialized | DexValue::Int(_) | DexValue::Long(_) | DexValue::Float(_) => None,
    }
}

fn normalize_atomic_boolean(value: i32) -> i32 {
    if value == 0 {
        0
    } else {
        1
    }
}

pub(in crate::dex_run) fn atomic_reference_field_updater_new(
    invocation: &DexInvocation,
) -> Option<DexValue> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    let target_descriptor = class_reference_descriptor_from_value(&invocation.arguments[0])?;
    let value_descriptor = class_reference_descriptor_from_value(&invocation.arguments[1])?;
    let DexValue::String(field_name) = &invocation.arguments[2] else {
        return None;
    };

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;:{target_descriptor}:{value_descriptor}:{field_name}"
    ))))
}

pub(in crate::dex_run) fn atomic_integer_field_updater_new(
    invocation: &DexInvocation,
) -> Option<DexValue> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let target_descriptor = class_reference_descriptor_from_value(&invocation.arguments[0])?;
    let DexValue::String(field_name) = &invocation.arguments[1] else {
        return None;
    };

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;:{target_descriptor}:{field_name}"
    ))))
}

pub(in crate::dex_run) fn atomic_long_field_updater_new(
    invocation: &DexInvocation,
) -> Option<DexValue> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let target_descriptor = class_reference_descriptor_from_value(&invocation.arguments[0])?;
    let DexValue::String(field_name) = &invocation.arguments[1] else {
        return None;
    };

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;:{target_descriptor}:{field_name}"
    ))))
}

pub(in crate::dex_run) fn atomic_reference_field_updater_parts(
    value: &DexValue,
) -> Option<(String, String, String)> {
    let DexValue::Reference(reference) = value else {
        return None;
    };
    let payload =
        reference.strip_prefix("Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;:")?;

    let mut parts = payload.splitn(3, ':');
    let target_descriptor = parts.next()?;
    let value_or_field = parts.next();
    let field_name = parts.next();

    if let (Some(value_descriptor), Some(field_name)) = (value_or_field, field_name) {
        return Some((
            target_descriptor.to_owned(),
            value_descriptor.to_owned(),
            field_name.to_owned(),
        ));
    }

    let (target_descriptor, field_name) = payload.rsplit_once('.')?;

    Some((
        target_descriptor.to_owned(),
        "Ljava/lang/Object;".to_owned(),
        field_name.to_owned(),
    ))
}

pub(in crate::dex_run) fn atomic_integer_field_updater_parts(
    value: &DexValue,
) -> Option<(String, String)> {
    let DexValue::Reference(reference) = value else {
        return None;
    };
    let payload =
        reference.strip_prefix("Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;:")?;
    let (target_descriptor, field_name) = payload.rsplit_once(':')?;

    Some((target_descriptor.to_owned(), field_name.to_owned()))
}

pub(in crate::dex_run) fn atomic_long_field_updater_parts(
    value: &DexValue,
) -> Option<(String, String)> {
    let DexValue::Reference(reference) = value else {
        return None;
    };
    let payload =
        reference.strip_prefix("Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;:")?;
    let (target_descriptor, field_name) = payload.rsplit_once(':')?;

    Some((target_descriptor.to_owned(), field_name.to_owned()))
}

fn thread_pool_executor_core_field() -> DexFieldId {
    // cite: libcore ThreadPoolExecutor.java#corePoolSize
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/ThreadPoolExecutor;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "corePoolSize".to_owned(),
    }
}

fn thread_pool_executor_maximum_field() -> DexFieldId {
    // cite: libcore ThreadPoolExecutor.java#maximumPoolSize
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/ThreadPoolExecutor;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "maximumPoolSize".to_owned(),
    }
}

fn thread_pool_executor_keep_alive_field() -> DexFieldId {
    // cite: libcore ThreadPoolExecutor.java#keepAliveTime
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/ThreadPoolExecutor;".to_owned(),
        type_descriptor: "J".to_owned(),
        name: "keepAliveTime".to_owned(),
    }
}

fn thread_pool_executor_work_queue_field() -> DexFieldId {
    // cite: libcore ThreadPoolExecutor.java#workQueue
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/ThreadPoolExecutor;".to_owned(),
        type_descriptor: "Ljava/util/concurrent/BlockingQueue;".to_owned(),
        name: "workQueue".to_owned(),
    }
}

fn thread_pool_executor_thread_factory_field() -> DexFieldId {
    // cite: libcore ThreadPoolExecutor.java#threadFactory
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/ThreadPoolExecutor;".to_owned(),
        type_descriptor: "Ljava/util/concurrent/ThreadFactory;".to_owned(),
        name: "threadFactory".to_owned(),
    }
}

fn thread_pool_executor_handler_field() -> DexFieldId {
    // cite: libcore ThreadPoolExecutor.java#handler
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/ThreadPoolExecutor;".to_owned(),
        type_descriptor: "Ljava/util/concurrent/RejectedExecutionHandler;".to_owned(),
        name: "handler".to_owned(),
    }
}

fn thread_pool_executor_allow_core_thread_time_out_field() -> DexFieldId {
    // cite: libcore ThreadPoolExecutor.java#allowCoreThreadTimeOut
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/ThreadPoolExecutor;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "allowCoreThreadTimeOut".to_owned(),
    }
}

pub(in crate::dex_run) fn future_task_callable_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/util/concurrent/Callable;".to_owned(),
        name: "callable".to_owned(),
    }
}

pub(in crate::dex_run) fn future_task_runnable_field() -> DexFieldId {
    // cite: libcore FutureTask.java#FutureTask
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/lang/Runnable;".to_owned(),
        name: "runnable".to_owned(),
    }
}

pub(in crate::dex_run) fn future_task_runnable_result_field() -> DexFieldId {
    // cite: libcore Executors.java#callable, RunnableAdapter
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "runnableResult".to_owned(),
    }
}

pub(in crate::dex_run) fn future_task_done_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "done".to_owned(),
    }
}

pub(in crate::dex_run) fn future_task_cancelled_field() -> DexFieldId {
    // cite: libcore FutureTask.java#cancel, isCancelled
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "cancelled".to_owned(),
    }
}

pub(in crate::dex_run) fn future_task_outcome_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "outcome".to_owned(),
    }
}

pub(in crate::dex_run) fn future_task_exception_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/lang/Throwable;".to_owned(),
        name: "outcomeException".to_owned(),
    }
}

pub(in crate::dex_run) fn fixed_thread_pool_reference(
    invocation: &DexInvocation,
) -> Option<DexValue> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Int(thread_count) = invocation.arguments[0] else {
        return None;
    };

    if thread_count <= 0 {
        return None;
    }

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/util/concurrent/ExecutorService;.fixed/{thread_count}"
    ))))
}

struct ThreadPoolExecutorState<'a> {
    core: i32,
    maximum: i32,
    keep_alive_nanos: i64,
    work_queue: &'a DexValue,
    thread_factory: &'a DexValue,
    handler: Option<&'a DexValue>,
}

fn is_executors_factory_reference(reference: &str) -> bool {
    matches!(
        reference,
        "Ljava/util/concurrent/ExecutorService;.single"
            | "Ljava/util/concurrent/ExecutorService;.cached"
            | "Ljava/util/concurrent/ScheduledExecutorService;.single"
    ) || reference.starts_with("Ljava/util/concurrent/ExecutorService;.fixed/")
}
