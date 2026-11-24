use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_json_collection_extra_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Ljava/lang/ThreadLocal;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/lang/ThreadLocal;"
            && method.name == "get"
            && method.proto.descriptor_is("()Ljava/lang/Object;")
        {
            let invocation = invocation?;

            return self.read_thread_local_value(invocation);
        }

        if method.class_descriptor == "Ljava/lang/ThreadLocal;"
            && method.name == "set"
            && method.proto.descriptor_is("(Ljava/lang/Object;)V")
        {
            let invocation = invocation?;
            self.write_thread_local_value(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/lang/ThreadLocal;"
            && method.name == "remove"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.remove_thread_local_value(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            self.init_json_object_from_string(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && method.name == "put"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;I)Lorg/json/JSONObject;")
        {
            let invocation = invocation?;
            let object = self.write_json_object_value(invocation)?;

            return Some(DexExecutionResult::Value(object));
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && method.name == "put"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;J)Lorg/json/JSONObject;")
        {
            let invocation = invocation?;
            let object = self.write_json_object_long(invocation)?;

            return Some(DexExecutionResult::Value(object));
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && method.name == "put"
            && method.proto.descriptor()
                == "(Ljava/lang/String;Ljava/lang/Object;)Lorg/json/JSONObject;"
        {
            let invocation = invocation?;
            let object = self.write_json_object_value(invocation)?;

            return Some(DexExecutionResult::Value(object));
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && method.name == "length"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.json_object_length(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && method.name == "isNull"
            && method.proto.descriptor_is("(Ljava/lang/String;)Z")
        {
            let invocation = invocation?;
            let value = self.json_object_is_null(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && method.name == "toString"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.json_object_to_string(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/util/Collection;)V")
        {
            let invocation = invocation?;
            self.init_json_array_from_collection(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            self.init_json_array_from_string(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && matches!(method.name.as_str(), "getJSONArray" | "optJSONArray")
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Lorg/json/JSONArray;")
        {
            let invocation = invocation?;
            let DexValue::String(entry_key) = invocation.arguments.get(1)? else {
                return None;
            };
            let entry_key = entry_key.clone();
            match self.json_object_get_value(invocation) {
                Some(DexValue::Object(object))
                    if object.class_descriptor == std::rc::Rc::from("Lorg/json/JSONArray;") =>
                {
                    return Some(DexExecutionResult::Value(DexValue::Object(object)));
                }
                _ if method.name == "optJSONArray" => {
                    return Some(DexExecutionResult::Value(DexValue::Null));
                }
                _ => {
                    return Some(DexExecutionResult::Throw(
                        self.json_exception(&format!("No value for {entry_key}")),
                    ));
                }
            }
        }

        if method.class_descriptor == "Lorg/json/JSONObject;"
            && matches!(method.name.as_str(), "getString" | "optString")
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/lang/String;")
        {
            let invocation = invocation?;
            let DexValue::String(entry_key) = invocation.arguments.get(1)? else {
                return None;
            };
            let entry_key = entry_key.clone();
            let value = self
                .json_object_get_value(invocation)
                .as_ref()
                .and_then(json_coerce_to_string);
            match value {
                Some(value) => {
                    return Some(DexExecutionResult::Value(DexValue::String(
                        std::rc::Rc::from(value),
                    )));
                }
                None if method.name == "optString" => {
                    return Some(DexExecutionResult::Value(DexValue::String(
                        std::rc::Rc::from(String::new()),
                    )));
                }
                None => {
                    return Some(DexExecutionResult::Throw(
                        self.json_exception(&format!("No value for {entry_key}")),
                    ));
                }
            }
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && matches!(method.name.as_str(), "getJSONObject" | "optJSONObject")
            && method.proto.descriptor_is("(I)Lorg/json/JSONObject;")
        {
            let invocation = invocation?;
            let DexValue::Int(index) = invocation.arguments.get(1)? else {
                return None;
            };
            let index = *index;
            match self.json_array_get_value(invocation) {
                Some(DexValue::Object(object))
                    if object.class_descriptor == std::rc::Rc::from("Lorg/json/JSONObject;") =>
                {
                    return Some(DexExecutionResult::Value(DexValue::Object(object)));
                }
                _ if method.name == "optJSONObject" => {
                    return Some(DexExecutionResult::Value(DexValue::Null));
                }
                _ => {
                    return Some(DexExecutionResult::Throw(
                        self.json_exception(&format!("No value at index {index}")),
                    ));
                }
            }
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && matches!(method.name.as_str(), "getString" | "optString")
            && method.proto.descriptor_is("(I)Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.json_array_get_string(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && method.name == "isNull"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = self.json_array_is_null(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && method.name == "put"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Lorg/json/JSONArray;")
        {
            let invocation = invocation?;
            let array = self.json_array_put_value(invocation)?;

            return Some(DexExecutionResult::Value(array));
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && method.name == "length"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.json_array_length(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Lorg/json/JSONArray;"
            && method.name == "toString"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.json_array_to_string(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/util/AbstractList;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.initialize_empty_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.initialize_empty_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/util/Collection;)V")
        {
            let invocation = invocation?;
            self.initialize_array_list_from_collection(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/CopyOnWriteArrayList;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.initialize_empty_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/ConcurrentLinkedQueue;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            // cite: libcore ConcurrentLinkedQueue.java#ConcurrentLinkedQueue
            let invocation = invocation?;
            self.initialize_empty_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/LinkedBlockingQueue;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            // cite: libcore LinkedBlockingQueue.java#LinkedBlockingQueue
            let invocation = invocation?;
            self.initialize_empty_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/SynchronousQueue;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            // cite: libcore SynchronousQueue.java#SynchronousQueue
            let invocation = invocation?;
            self.initialize_empty_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "add"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            self.add_array_list_value(invocation)?;

            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "add"
            && method.proto.descriptor_is("(ILjava/lang/Object;)V")
        {
            let invocation = invocation?;
            self.add_array_list_value_at(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/CopyOnWriteArrayList;"
            && method.name == "add"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            self.add_array_list_value(invocation)?;

            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Ljava/util/concurrent/CopyOnWriteArrayList;"
            && method.name == "remove"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let removed = self.remove_collection_value(invocation)?;

            return Some(DexExecutionResult::Int(removed));
        }

        if method.class_descriptor == "Ljava/util/concurrent/CopyOnWriteArrayList;"
            && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            let value = self.concrete_list_size(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/CopyOnWriteArrayList;"
            && method.name == "isEmpty"
            && method.proto.descriptor_is("()Z")
        {
            let value = self.concrete_list_is_empty(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/CopyOnWriteArrayList;"
            && method.name == "iterator"
            && method.proto.descriptor_is("()Ljava/util/Iterator;")
        {
            let invocation = invocation?;
            let value = self.list_iterator(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "get"
            && method.proto.descriptor_is("(I)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.read_array_list_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "remove"
            && method.proto.descriptor_is("(I)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.remove_array_list_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "remove"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let removed = self.remove_collection_value(invocation)?;

            return Some(DexExecutionResult::Int(removed));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "clear"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.clear_list(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            let value = self.concrete_list_size(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "isEmpty"
            && method.proto.descriptor_is("()Z")
        {
            let value = self.concrete_list_is_empty(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "contains"
            && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
        {
            let invocation = invocation?;
            let value = self.list_contains(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/ArrayList;" | "Ljava/util/concurrent/CopyOnWriteArrayList;"
        ) && method.name == "indexOf"
            && method.proto.descriptor_is("(Ljava/lang/Object;)I")
        {
            let invocation = invocation?;
            let value = self.list_index_of(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "iterator"
            && method.proto.descriptor_is("()Ljava/util/Iterator;")
        {
            let invocation = invocation?;
            let value = self.list_iterator(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "listIterator"
            && method.proto.descriptor_is("(I)Ljava/util/ListIterator;")
        {
            let invocation = invocation?;
            let value = self.list_iterator_at(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/ArrayList;"
                | "Ljava/util/Collection;"
                | "Ljava/util/AbstractCollection;"
                | "Ljava/util/AbstractSet;"
                | "Ljava/util/Set;"
                | "Ljava/util/HashSet;"
                | "Ljava/util/LinkedHashSet;"
                | "Ljava/util/TreeSet;"
                | "Ljava/util/concurrent/CopyOnWriteArraySet;"
        ) && method.name == "toArray"
            && method
                .proto
                .descriptor_is("([Ljava/lang/Object;)[Ljava/lang/Object;")
        {
            let invocation = invocation?;
            let value = self.array_list_to_array(invocation)?;

            return Some(DexExecutionResult::Value(value));
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

        if is_supported_map_class(&method.class_descriptor)
            && method.name == "putAll"
            && method.proto.descriptor_is("(Ljava/util/Map;)V")
        {
            let invocation = invocation?;
            self.initialize_hash_map_from_map(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        None
    }

    fn json_exception(&self, message: &str) -> DexValue {
        let exception = self.allocate_object("Lorg/json/JSONException;".to_owned());
        self.write_stored_instance_field(
            &exception,
            &json_throwable_detail_message_field(),
            DexValue::String(std::rc::Rc::from(message.to_owned())),
        );

        exception
    }
}

fn json_throwable_detail_message_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    }
}
