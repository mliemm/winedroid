use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_collection_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if is_supported_set_class(&method.class_descriptor)
            && method.name == "iterator"
            && method.proto.descriptor_is("()Ljava/util/Iterator;")
        {
            let value = self.set_iterator(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if is_supported_set_class(&method.class_descriptor)
            && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            let value = self.set_size(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if is_supported_set_class(&method.class_descriptor)
            && method.name == "isEmpty"
            && method.proto.descriptor_is("()Z")
        {
            let value = i32::from(self.set_size(invocation)? == 0);

            return Some(DexExecutionResult::Int(value));
        }

        if is_supported_set_class(&method.class_descriptor)
            && method.name == "contains"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.list_contains(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if is_supported_set_class(&method.class_descriptor)
            && method.name == "remove"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.remove_collection_value(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if is_supported_set_class(&method.class_descriptor)
            && method.name == "clear"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.clear_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/TreeSet;"
            && method.name == "first"
            && method.proto.descriptor_is("()Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.tree_set_first(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if is_supported_set_class(&method.class_descriptor)
            && method.name == "add"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            if is_sorted_set_class(&method.class_descriptor) {
                return None;
            }

            let invocation = invocation?;
            let value = self.add_set_value(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if is_supported_set_class(&method.class_descriptor)
            && method.name == "addAll"
            && method.proto.descriptor_is("(Ljava/util/Collection;)Z")
        {
            let invocation = invocation?;
            let value = self.java_add_all_set_values(invocation, 0).ok()?;

            return Some(DexExecutionResult::Int(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;" | "Ljava/util/AbstractCollection;"
        ) && method.name == "add"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            self.add_collection_value(invocation)?;

            return Some(DexExecutionResult::Int(1));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;" | "Ljava/util/AbstractCollection;"
        ) && method.name == "isEmpty"
            && method.proto.descriptor_is("()Z")
        {
            let value = self.collection_is_empty(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;" | "Ljava/util/AbstractCollection;"
        ) && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            let value = self.list_size(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/PriorityQueue;"
            && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            let value = self.list_size(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/PriorityQueue;"
            && method.name == "add"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.priority_queue_add(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/PriorityQueue;"
            && method.name == "iterator"
            && method.proto.descriptor_is("()Ljava/util/Iterator;")
        {
            let Some(invocation) = invocation else {
                return Some(DexExecutionResult::Value(empty_iterator()));
            };
            let value = self.collection_iterator(&invocation.arguments[0])?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: libcore AbstractList.java#iterator, Itr
        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;" | "Ljava/util/AbstractCollection;" | "Ljava/lang/Iterable;"
        ) && method.name == "iterator"
            && method.proto.descriptor_is("()Ljava/util/Iterator;")
        {
            let Some(invocation) = invocation else {
                return Some(DexExecutionResult::Value(empty_iterator()));
            };
            let value = self
                .collection_iterator(&invocation.arguments[0])
                .or_else(|| self.abstract_list_snapshot_iterator(invocation))?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: libcore AbstractSet.java#removeAll
        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;" | "Ljava/util/AbstractSet;"
        ) && method.name == "removeAll"
            && method.proto.descriptor_is("(Ljava/util/Collection;)Z")
        {
            let invocation = invocation?;
            let removed = self.remove_all_collection_values(invocation)?;

            return Some(DexExecutionResult::Int(removed));
        }

        // cite: libcore AbstractCollection.java#retainAll
        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;" | "Ljava/util/Set;" | "Ljava/util/List;"
        ) && method.name == "retainAll"
            && method.proto.descriptor_is("(Ljava/util/Collection;)Z")
        {
            let invocation = invocation?;
            let changed = self.retain_all_collection_values(invocation)?;

            return Some(DexExecutionResult::Int(changed));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;" | "Ljava/util/AbstractCollection;"
        ) && method.name == "remove"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let removed = self.remove_collection_value(invocation)?;

            return Some(DexExecutionResult::Int(removed));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;"
                | "Ljava/util/List;"
                | "Ljava/util/ArrayList;"
                | "Ljava/util/LinkedList;"
                | "Ljava/util/concurrent/CopyOnWriteArrayList;"
        ) && method.name == "removeIf"
            && method
                .proto
                .descriptor_is("(Ljava/util/function/Predicate;)Z")
        {
            let invocation = invocation?;

            return Some(self.list_remove_if(invocation, 1).ok()?);
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "singletonList"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/util/List;")
        {
            let invocation = invocation?;
            let value = self.collections_singleton_list(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "nCopies"
            && method
                .proto
                .descriptor_is("(ILjava/lang/Object;)Ljava/util/List;")
        {
            let invocation = invocation?;
            let value = self.collections_n_copies(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "singletonMap"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Map;")
        {
            // cite: libcore Collections.java#singletonMap
            let invocation = invocation?;
            let value = self.collections_singleton_map(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "emptyList"
            && method.proto.descriptor_is("()Ljava/util/List;")
        {
            let value = self.collections_empty_list();

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "emptySet"
            && method.proto.descriptor_is("()Ljava/util/Set;")
        {
            // cite: libcore Collections.java#emptySet, EMPTY_SET
            let value = self.collections_empty_set();

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;"
                | "Ljava/util/AbstractCollection;"
                | "Ljava/util/List;"
                | "Ljava/util/Set;"
                | "Ljava/util/Queue;"
        ) && method.name == "toArray"
            && method.proto.descriptor_is("()[Ljava/lang/Object;")
        {
            // cite: libcore Collection.java#toArray
            let invocation = invocation?;
            let value = self.collection_to_object_array(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Collection;"
                | "Ljava/util/AbstractCollection;"
                | "Ljava/util/List;"
                | "Ljava/util/Set;"
                | "Ljava/util/Queue;"
        ) && method.name == "toArray"
            && method
                .proto
                .descriptor_is("([Ljava/lang/Object;)[Ljava/lang/Object;")
        {
            // cite: libcore Collection.java#toArray
            let invocation = invocation?;
            let value = self.collection_to_existing_array(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "addAll"
            && method
                .proto
                .descriptor_is("(Ljava/util/Collection;[Ljava/lang/Object;)Z")
        {
            // cite: libcore Collections.java#addAll
            let invocation = invocation?;
            let value = self.collections_add_all_array(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "reverse"
            && method.proto.descriptor_is("(Ljava/util/List;)V")
        {
            let invocation = invocation?;
            self.collections_reverse_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "sort"
            && method.proto.descriptor_is("(Ljava/util/List;)V")
        {
            let invocation = invocation?;
            self.collections_sort_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "unmodifiableList"
            && method
                .proto
                .descriptor_is("(Ljava/util/List;)Ljava/util/List;")
        {
            let invocation = invocation?;
            let value = Self::collections_identity(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "binarySearch"
            && method
                .proto
                .descriptor_is("(Ljava/util/List;Ljava/lang/Object;)I")
        {
            let invocation = invocation?;
            let value = self.collections_binary_search(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "asList"
            && method
                .proto
                .descriptor_is("([Ljava/lang/Object;)Ljava/util/List;")
        {
            let invocation = invocation?;
            let value = self.arrays_as_list(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "fill"
            && method.proto.descriptor_is("([JJ)V")
        {
            let invocation = invocation?;
            self.arrays_fill_long(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "fill"
            && method.proto.descriptor_is("([II)V")
        {
            let invocation = invocation?;
            self.arrays_fill_int(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "fill"
            && method.proto.descriptor_is("([ZZ)V")
        {
            let invocation = invocation?;
            self.arrays_fill_int(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "fill"
            && method.proto.descriptor_is("([JIIJ)V")
        {
            let invocation = invocation?;
            self.arrays_fill_long_range(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "fill"
            && method.proto.descriptor_is("([IIII)V")
        {
            let invocation = invocation?;
            self.arrays_fill_int_range(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "fill"
            && method
                .proto
                .descriptor_is("([Ljava/lang/Object;IILjava/lang/Object;)V")
        {
            let invocation = invocation?;
            self.arrays_fill_object_range(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "equals"
            && method
                .proto
                .descriptor_is("([Ljava/lang/Object;[Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.arrays_equals_object(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "hashCode"
            && method.proto.descriptor_is("([Ljava/lang/Object;)I")
        {
            let invocation = invocation?;
            let value = self.arrays_hash_code_object(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "sort"
            && method.proto.descriptor_is("([III)V")
        {
            let invocation = invocation?;
            self.arrays_sort_int_range(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "copyOf"
            && method
                .proto
                .descriptor_is("([Ljava/lang/Object;I)[Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.arrays_copy_of_object(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "copyOfRange"
            && matches!(
                method.proto.descriptor().as_str(),
                "([Ljava/lang/Object;II)[Ljava/lang/Object;"
                    | "([Ljava/lang/Object;IILjava/lang/Class;)[Ljava/lang/Object;"
            )
        {
            let invocation = invocation?;
            let value = self.arrays_copy_of_range_object(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "copyOf"
            && method.proto.descriptor_is("([II)[I")
        {
            let invocation = invocation?;
            let value = self.arrays_copy_of_int(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Arrays;"
            && method.name == "copyOf"
            && method.proto.descriptor_is("([JI)[J")
        {
            let invocation = invocation?;
            let value = self.arrays_copy_of_long(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "singleton"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/util/Set;")
        {
            let invocation = invocation?;
            let value = self.collections_singleton_set(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "newSetFromMap"
            && method
                .proto
                .descriptor_is("(Ljava/util/Map;)Ljava/util/Set;")
        {
            let invocation = invocation?;
            let value = self.collections_new_set_from_map(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "emptyMap"
            && method.proto.descriptor_is("()Ljava/util/Map;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/Map;:collections:emptyMap"),
            )));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "unmodifiableSet"
            && method
                .proto
                .descriptor_is("(Ljava/util/Set;)Ljava/util/Set;")
        {
            let invocation = invocation?;
            let value = Self::collections_identity(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "unmodifiableCollection"
            && method
                .proto
                .descriptor_is("(Ljava/util/Collection;)Ljava/util/Collection;")
        {
            let invocation = invocation?;
            let value = Self::collections_identity(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "unmodifiableMap"
            && method
                .proto
                .descriptor_is("(Ljava/util/Map;)Ljava/util/Map;")
        {
            let invocation = invocation?;
            let value = Self::collections_identity(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "synchronizedMap"
            && method
                .proto
                .descriptor_is("(Ljava/util/Map;)Ljava/util/Map;")
        {
            let invocation = invocation?;
            let value = Self::collections_synchronized_map(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "synchronizedSet"
            && method
                .proto
                .descriptor_is("(Ljava/util/Set;)Ljava/util/Set;")
        {
            // cite: libcore Collections.java#synchronizedSet, SynchronizedSet, SynchronizedCollection
            let invocation = invocation?;
            let value = Self::collections_identity(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Collections;"
            && method.name == "synchronizedList"
            && method
                .proto
                .descriptor_is("(Ljava/util/List;)Ljava/util/List;")
        {
            // cite: libcore Collections.java#synchronizedList, SynchronizedList, SynchronizedRandomAccessList
            let invocation = invocation?;
            let value = Self::collections_identity(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/Iterable;"
            && method.name == "iterator"
            && method.proto.descriptor_is("()Ljava/util/Iterator;")
        {
            let value = self.iterable_iterator(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: libcore AbstractList.java#iterator, Itr
        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;" | "Ljava/util/AbstractList;"
        ) && method.name == "iterator"
            && method.proto.descriptor_is("()Ljava/util/Iterator;")
        {
            let invocation = invocation?;
            let value = self
                .list_iterator(invocation)
                .or_else(|| self.abstract_list_snapshot_iterator(invocation))?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/List;"
            && method.name == "listIterator"
            && method.proto.descriptor_is("(I)Ljava/util/ListIterator;")
        {
            let invocation = invocation?;
            let value = self.list_iterator_at(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/List;"
            && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            let value = self.list_size(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/List;"
            && method.name == "isEmpty"
            && method.proto.descriptor_is("()Z")
        {
            let value = self.list_is_empty(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/List;"
            && method.name == "get"
            && method.proto.descriptor_is("(I)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.read_array_list_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;" | "Ljava/util/AbstractList;" | "Ljava/util/ArrayList;"
        ) && method.name == "set"
            && method
                .proto
                .descriptor_is("(ILjava/lang/Object;)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.set_array_list_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/List;"
            && method.name == "contains"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.list_contains(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/List;"
            && method.name == "add"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            self.add_array_list_value(invocation)?;

            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Ljava/util/List;"
            && method.name == "add"
            && method.proto.descriptor_is("(ILjava/lang/Object;)V")
        {
            let invocation = invocation?;
            self.add_array_list_value_at(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;" | "Ljava/util/AbstractList;" | "Ljava/util/ArrayList;"
        ) && method.name == "subList"
            && method.proto.descriptor_is("(II)Ljava/util/List;")
        {
            let invocation = invocation?;
            let value = self.array_list_sub_list(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;" | "Ljava/util/AbstractList;"
        ) && method.name == "indexOf"
            && method.proto.descriptor_is("(Ljava/lang/Object;)I")
        {
            let invocation = invocation?;
            let value = self.list_index_of(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        // cite: libcore AbstractList.java#lastIndexOf
        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;" | "Ljava/util/AbstractList;"
        ) && method.name == "lastIndexOf"
            && method.proto.descriptor_is("(Ljava/lang/Object;)I")
        {
            let invocation = invocation?;
            let value = self.list_last_index_of(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/List;"
            && method.name == "remove"
            && method.proto.descriptor_is("(I)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.remove_array_list_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;"
                | "Ljava/util/AbstractList;"
                | "Ljava/util/ArrayList;"
                | "Ljava/util/LinkedList;"
                | "Ljava/util/concurrent/CopyOnWriteArrayList;"
        ) && method.name == "add"
            && method.proto.descriptor_is("(ILjava/lang/Object;)V")
        {
            // cite: libcore List.java#add
            let invocation = invocation?;
            self.insert_list_value(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;"
                | "Ljava/util/Collection;"
                | "Ljava/util/AbstractCollection;"
                | "Ljava/util/AbstractList;"
                | "Ljava/util/AbstractSequentialList;"
                | "Ljava/util/ArrayList;"
                | "Ljava/util/LinkedList;"
        ) && method.name == "clear"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.clear_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/ConcurrentHashMap;" | "Ljava/util/Hashtable;"
        ) && matches!(method.name.as_str(), "keys" | "elements")
            && method.proto.descriptor_is("()Ljava/util/Enumeration;")
        {
            let invocation = invocation?;
            let value = self.map_enumeration(invocation, method.name == "keys")?;

            return Some(DexExecutionResult::Value(value));
        }

        let enumeration = invocation
            .and_then(|invocation| invocation.arguments.first())
            .is_some_and(|value| {
                matches!(value, DexValue::Reference(reference)
                    if reference.starts_with("Ljava/util/Enumeration;:iterator:"))
            });
        if enumeration && method.class_descriptor == "Ljava/util/Enumeration;" {
            // cite: libcore Enumeration.java#hasMoreElements, nextElement
            match (method.name.as_str(), method.proto.descriptor().as_str()) {
                ("hasMoreElements", "()Z") => {
                    return Some(DexExecutionResult::Int(self.iterator_has_next(invocation)?));
                }
                ("nextElement", "()Ljava/lang/Object;") => {
                    return Some(DexExecutionResult::Value(self.iterator_next(invocation?)?));
                }
                _ => {}
            }
        }

        if method.class_descriptor == "Ljava/util/Iterator;"
            && method.name == "hasNext"
            && method.proto.descriptor_is("()Z")
        {
            let value = self.iterator_has_next(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/Iterator;"
            && method.name == "next"
            && method.proto.descriptor_is("()Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.iterator_next(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/Iterator;"
            && method.name == "remove"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.iterator_remove(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/ListIterator;"
            && method.name == "hasNext"
            && method.proto.descriptor_is("()Z")
        {
            let value = self.iterator_has_next(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/ListIterator;"
            && method.name == "next"
            && method.proto.descriptor_is("()Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.iterator_next(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/ListIterator;"
            && method.name == "hasPrevious"
            && method.proto.descriptor_is("()Z")
        {
            let value = self.iterator_has_previous(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/ListIterator;"
            && method.name == "previous"
            && method.proto.descriptor_is("()Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.iterator_previous(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/ListIterator;"
            && method.name == "remove"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.iterator_remove(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/AbstractMap$SimpleEntry;"
            && method.name == "<init>"
            && (method
                .proto
                .descriptor_is("(Ljava/lang/Object;Ljava/lang/Object;)V")
                || method.proto.descriptor_is("(Ljava/util/Map$Entry;)V"))
        {
            let invocation = invocation?;

            return self.simple_entry_init(invocation);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Map$Entry;" | "Ljava/util/AbstractMap$SimpleEntry;"
        ) && method.name == "getKey"
            && method.proto.descriptor_is("()Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.map_entry_key(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/Map$Entry;" | "Ljava/util/AbstractMap$SimpleEntry;"
        ) && method.name == "getValue"
            && method.proto.descriptor_is("()Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.map_entry_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/ConcurrentHashMap;"
            && method.name == "newKeySet"
            && method
                .proto
                .descriptor_is("()Ljava/util/concurrent/ConcurrentHashMap$KeySetView;")
        {
            // cite: libcore ConcurrentHashMap.java#newKeySet, KeySetView
            let view = self
                .allocate_object("Ljava/util/concurrent/ConcurrentHashMap$KeySetView;".to_owned());
            self.runtime
                .borrow_mut()
                .list_entries
                .insert(map_identity_key(&view), Vec::new());

            return Some(DexExecutionResult::Value(view));
        }

        if method.class_descriptor == "Ljava/util/TreeMap;"
            && method.name == "ceilingEntry"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/util/Map$Entry;")
        {
            // cite: libcore TreeMap.java#ceilingEntry
            let invocation = invocation?;
            let value = self.tree_map_ceiling_entry(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "equals"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            // cite: libcore AbstractMap.java#equals
            let invocation = invocation?;
            let value = self
                .runtime_values_equal(&invocation.arguments[0], &invocation.arguments[1], 1, true)
                .ok()?;

            return Some(DexExecutionResult::Int(i32::from(value)));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "get"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.read_hash_map_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "containsKey"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.hash_map_contains_key(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "put"
            && method.proto.descriptor()
                == "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"
        {
            let invocation = invocation?;
            let previous = self.write_hash_map_value(invocation)?;

            return Some(DexExecutionResult::Value(previous));
        }

        if method.class_descriptor == "Ljava/util/concurrent/ConcurrentHashMap;"
            && method.name == "putIfAbsent"
            && method.proto.descriptor()
                == "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"
        {
            // cite: libcore ConcurrentHashMap.java#putIfAbsent
            let invocation = invocation?;
            if invocation.arguments.len() != 3 {
                return None;
            }
            if matches!(invocation.arguments[1], DexValue::Null)
                || matches!(invocation.arguments[2], DexValue::Null)
            {
                return Some(DexExecutionResult::Throw(
                    self.allocate_object("Ljava/lang/NullPointerException;".to_owned()),
                ));
            }
            let lookup = DexInvocation {
                arguments: smallvec::smallvec![
                    invocation.arguments[0].clone(),
                    invocation.arguments[1].clone()
                ],
                ..invocation.clone()
            };
            let existing = self.read_hash_map_value(&lookup)?;
            if existing != DexValue::Null {
                return Some(DexExecutionResult::Value(existing));
            }
            self.write_hash_map_value(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "remove"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let previous = self.remove_hash_map_value(invocation)?;

            return Some(DexExecutionResult::Value(previous));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "clear"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.clear_hash_map(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.hash_map_size(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "isEmpty"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.hash_map_is_empty(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "entrySet"
            && method.proto.descriptor_is("()Ljava/util/Set;")
        {
            let invocation = invocation?;
            let value = self.map_entry_set(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "values"
            && method.proto.descriptor_is("()Ljava/util/Collection;")
        {
            let invocation = invocation?;
            let value = self.map_values(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "keySet"
            && method.proto.descriptor_is("()Ljava/util/Set;")
        {
            let invocation = invocation?;
            let value = self.map_key_set(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/HashMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/AbstractMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/AbstractCollection;"
                | "Ljava/util/AbstractList;"
                | "Ljava/util/AbstractSet;"
        ) && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        // cite: libcore ConcurrentHashMap.java#ConcurrentHashMap, putAll, putVal
        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ConcurrentHashMap;",
            "<init>",
            "(Ljava/util/Map;)V",
        ) {
            let invocation = invocation?;
            let source = invocation.arguments.get(1)?;
            if matches!(source, DexValue::Null | DexValue::Int(0)) {
                return Some(self.null_pointer_exception_result());
            }
            if !is_supported_map_implementation_class(value_class_descriptor(source)?) {
                return None;
            }
            let source_key = map_identity_key(source);
            let has_null = {
                let runtime = self.runtime.borrow();
                runtime
                    .map_entries
                    .iter()
                    .any(|((map_key, entry_key), value)| {
                        map_key == &source_key
                            && (*value == DexValue::Null
                                || runtime
                                    .map_entry_keys
                                    .get(&(map_key.clone(), entry_key.clone()))
                                    == Some(&DexValue::Null))
                    })
            };
            if has_null {
                return Some(self.null_pointer_exception_result());
            }
            self.initialize_hash_map_from_map(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/HashMap;" | "Ljava/util/LinkedHashMap;"
        ) && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/util/Map;)V")
        {
            let invocation = invocation?;
            self.initialize_hash_map_from_map(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/LinkedHashMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/util/ArrayMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            // cite: frameworks-base ArrayMap.java#ArrayMap
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/IdentityHashMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/WeakHashMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/TreeMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/TreeMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/util/Comparator;)V")
        {
            // cite: libcore TreeMap.java#TreeMap, comparator
            let [map, comparator] = invocation?.arguments.as_slice() else {
                return None;
            };
            if !dex_value_is_object_like(map) || !dex_value_is_nullable_object_like(comparator) {
                return None;
            }
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Properties;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            // cite: libcore Properties.java#Properties
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Properties;"
            && method.name == "load"
            && method.proto.descriptor_is("(Ljava/io/InputStream;)V")
        {
            let invocation = invocation?;

            return self.properties_load_from_stream(invocation);
        }

        if method.class_descriptor == "Ljava/util/Hashtable;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            // cite: libcore Hashtable.java#Hashtable
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/Dictionary;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            // cite: libcore Dictionary.java#Dictionary
            return Some(DexExecutionResult::Void);
        }

        if is_supported_map_implementation_class(&method.class_descriptor)
            && method.name == "<init>"
            && method.proto.descriptor_is("(I)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/HashMap;" | "Ljava/util/LinkedHashMap;"
        ) && method.name == "<init>"
            && matches!(method.proto.descriptor().as_str(), "(IF)V" | "(IFZ)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/ConcurrentHashMap;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/HashSet;" | "Ljava/util/LinkedHashSet;"
        ) && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            if let Some(invocation) = invocation {
                self.initialize_empty_set(invocation)?;
            }

            return Some(DexExecutionResult::Void);
        }

        if is_sorted_set_class(&method.class_descriptor)
            && method.name == "<init>"
            && matches!(
                method.proto.descriptor().as_str(),
                "()V" | "(Ljava/util/Comparator;)V"
            )
        {
            if let Some(invocation) = invocation {
                self.initialize_tree_set(invocation)?;
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/PriorityQueue;"
            && method.name == "<init>"
            && matches!(
                method.proto.descriptor().as_str(),
                "()V" | "(I)V" | "(ILjava/util/Comparator;)V" | "(Ljava/util/Collection;)V"
            )
        {
            if let Some(invocation) = invocation {
                self.initialize_priority_queue(invocation)?;
            }

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/HashSet;" | "Ljava/util/LinkedHashSet;"
        ) && method.name == "<init>"
            && method.proto.descriptor_is("(I)V")
        {
            if let Some(invocation) = invocation {
                self.initialize_empty_set_with_capacity(invocation)?;
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/ArrayDeque;"
            && method.name == "<init>"
            && matches!(method.proto.descriptor().as_str(), "()V" | "(I)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/ArrayDeque;"
                | "Ljava/util/LinkedList;"
                | "Ljava/util/concurrent/ConcurrentLinkedDeque;"
                | "Ljava/util/concurrent/LinkedBlockingDeque;"
        ) && method.name != "<init>"
            && !self.guest_overrides(method, invocation)
        {
            if let Some(result) = self.array_deque_method(method, invocation?) {
                return Some(result);
            }
        }

        if method.class_descriptor == "Ljava/util/ArrayDeque;" {
            if method.name == "offer" && method.proto.descriptor_is("(Ljava/lang/Object;)Z") {
                // cite: libcore ArrayDeque.java#offer, offerLast, addLast
                let invocation = invocation?;
                self.array_deque_add_last(invocation).ok()??;
                return Some(DexExecutionResult::Int(1));
            }
            if method.name == "offerLast" && method.proto.descriptor_is("(Ljava/lang/Object;)Z") {
                // cite: libcore ArrayDeque.java#offerLast, addLast
                let invocation = invocation?;
                self.array_deque_add_last(invocation).ok()??;
                return Some(DexExecutionResult::Int(1));
            }
            if method.name == "add" && method.proto.descriptor_is("(Ljava/lang/Object;)Z") {
                // cite: libcore ArrayDeque.java#add, addLast
                let invocation = invocation?;
                self.array_deque_add_last(invocation).ok()??;
                return Some(DexExecutionResult::Int(1));
            }
            if method.name == "addLast" && method.proto.descriptor_is("(Ljava/lang/Object;)V") {
                // cite: libcore ArrayDeque.java#addLast
                let invocation = invocation?;
                self.array_deque_add_last(invocation).ok()??;
                return Some(DexExecutionResult::Void);
            }
        }

        if method.class_descriptor == "Ljava/util/concurrent/CopyOnWriteArraySet;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            if let Some(invocation) = invocation {
                self.initialize_empty_set(invocation)?;
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/UUID;"
            && method.name == "randomUUID"
            && method.proto.descriptor_is("()Ljava/util/UUID;")
        {
            let value = random_uuid_string()?;

            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from(format!("Ljava/util/UUID;:{value}")),
            )));
        }

        if method.class_descriptor == "Ljava/util/UUID;"
            && method.name == "toString"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = uuid_to_string(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        None
    }
}

// cite: libcore UUID.java#randomUUID
fn random_uuid_string() -> Option<String> {
    use std::io::Read;

    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .ok()?
        .read_exact(&mut bytes)
        .ok()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    Some(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

impl DexProgram {
    // cite: libcore ArrayDeque.java#size, isEmpty, peek, peekFirst, peekLast, poll, pollFirst, pollLast, push, addFirst, offerFirst, pop, removeFirst, removeLast, clear
    // cite: libcore ConcurrentLinkedDeque.java#size, isEmpty, peekFirst, peekLast, pollFirst, pollLast, push, addFirst, offerFirst, pop, removeFirst, removeLast, clear
    // cite: libcore LinkedBlockingDeque.java#putFirst, putLast, offerLast, takeFirst, takeLast
    // cite: libcore LinkedList.java#descendingIterator, peekLast, pollLast, push, pop, linkLast
    fn array_deque_method(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let deque = invocation.arguments.first()?;
        if !dex_value_is_object_like(deque) {
            return None;
        }
        let key = map_identity_key(deque);
        let descriptor = method.proto.descriptor();
        let mut runtime = self.runtime.borrow_mut();
        let values = runtime.list_entries.entry(key).or_default();
        let value =
            |found: Option<DexValue>| DexExecutionResult::Value(found.unwrap_or(DexValue::Null));
        Some(match (method.name.as_str(), descriptor.as_str()) {
            ("size", "()I") => DexExecutionResult::Int(i32::try_from(values.len()).ok()?),
            ("isEmpty", "()Z") => DexExecutionResult::Int(i32::from(values.is_empty())),
            ("peek" | "peekFirst", "()Ljava/lang/Object;") => value(values.first().cloned()),
            ("peekLast", "()Ljava/lang/Object;") => value(values.last().cloned()),
            ("poll" | "pollFirst", "()Ljava/lang/Object;") => {
                value((!values.is_empty()).then(|| values.remove(0)))
            }
            ("pollLast", "()Ljava/lang/Object;") => value(values.pop()),
            ("push" | "addFirst" | "putFirst" | "addLast" | "putLast", "(Ljava/lang/Object;)V")
            | ("offerFirst" | "offerLast" | "offer" | "add", "(Ljava/lang/Object;)Z") => {
                let item = invocation.arguments.get(1)?;
                let permits_null = method.class_descriptor == "Ljava/util/LinkedList;";
                if !permits_null && matches!(item, DexValue::Null | DexValue::Int(0)) {
                    drop(runtime);
                    return Some(self.null_pointer_exception_result());
                }
                let at_front = matches!(
                    method.name.as_str(),
                    "push" | "addFirst" | "putFirst" | "offerFirst"
                );
                if at_front {
                    values.insert(0, item.clone());
                } else {
                    values.push(item.clone());
                }
                if method.proto.return_type == "Z" {
                    DexExecutionResult::Int(1)
                } else {
                    DexExecutionResult::Void
                }
            }
            ("takeFirst" | "takeLast", "()Ljava/lang/Object;") => {
                if values.is_empty() {
                    return None;
                }
                let taken = if method.name == "takeLast" {
                    values.pop()
                } else {
                    Some(values.remove(0))
                };
                value(taken)
            }
            ("pop" | "removeFirst" | "remove", "()Ljava/lang/Object;")
            | ("removeLast", "()Ljava/lang/Object;") => {
                let removed = if values.is_empty() {
                    None
                } else if method.name == "removeLast" {
                    values.pop()
                } else {
                    Some(values.remove(0))
                };
                match removed {
                    Some(removed) => DexExecutionResult::Value(removed),
                    None => {
                        drop(runtime);
                        return Some(DexExecutionResult::Throw(
                            self.allocate_object("Ljava/util/NoSuchElementException;".to_owned()),
                        ));
                    }
                }
            }
            ("clear", "()V") => {
                values.clear();
                DexExecutionResult::Void
            }
            ("descendingIterator", "()Ljava/util/Iterator;") => {
                let reversed = values.iter().rev().cloned().collect();
                drop(runtime);
                DexExecutionResult::Value(self.create_value_iterator_with_position(
                    reversed,
                    0,
                    "Ljava/util/Iterator;",
                    None,
                )?)
            }
            _ => return None,
        })
    }
}
