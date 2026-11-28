use super::*;

impl DexRunInvocationHandler<'_> {
    pub(super) fn invoke_supported(
        &mut self,
        invocation: DexInvocation,
    ) -> Result<DexInvocationResult> {
        let method = self
            .program
            .method_id(self.current_dex_file_index, invocation.method_index)?;

        trace_invocation(self.depth, &invocation, &method);
        let records_menu_inflate = is_menu_inflater_inflate_method(&method);
        if records_menu_inflate {
            self.program.record_menu_inflate(&invocation)?;
            if self.program.resolve_exact_method(&method)?.is_none() {
                self.program.sync_inflated_menu_items(&invocation)?;
                return Ok(DexInvocationResult::Void);
            }
        }
        if records_content_view_method(&method) {
            self.program.record_content_view_argument(&invocation, 1);
        }

        let direct_kind = matches!(
            invocation.kind,
            DexInvokeKind::Static | DexInvokeKind::Direct
        );
        let method_key = method_cache_key(self.current_dex_file_index, invocation.method_index);
        if direct_kind {
            let cached = self
                .program
                .caches
                .borrow()
                .direct_resolved_methods
                .get(&method_key)
                .cloned();
            if let Some(resolved) = cached {
                if let Some(Some(arg_index)) = self
                    .program
                    .caches
                    .borrow()
                    .null_assert_args
                    .get(&method_key)
                    .copied()
                {
                    if null_assert_shortcut_applies(&invocation, &resolved, arg_index)? {
                        return Ok(DexInvocationResult::Void);
                    }
                }
                if let Some(result) = self.invoke_resolved_intrinsic(&resolved, &invocation)? {
                    if records_menu_inflate {
                        self.program.sync_inflated_menu_items(&invocation)?;
                    }
                    return Ok(result);
                }
                let result = self.program.execute_method(
                    None,
                    &resolved,
                    Some(&invocation.arguments),
                    self.depth + 1,
                )?;
                if records_menu_inflate {
                    self.program.sync_inflated_menu_items(&invocation)?;
                }
                return Ok(invocation_result_from_execution(result));
            }
        }
        if !direct_kind {
            let cached =
                object_receiver_dispatch_descriptor(&invocation).and_then(|receiver_descriptor| {
                    self.program
                        .caches
                        .borrow()
                        .virtual_resolved_methods
                        .get(&method_key)
                        .and_then(|by_receiver| by_receiver.get(receiver_descriptor))
                        .cloned()
                });
            if let Some(resolved) = cached {
                if let Some(result) = self.invoke_resolved_intrinsic(&resolved, &invocation)? {
                    if records_menu_inflate {
                        self.program.sync_inflated_menu_items(&invocation)?;
                    }
                    return Ok(result);
                }
                let result = self.program.execute_method(
                    None,
                    &resolved,
                    Some(&invocation.arguments),
                    self.depth + 1,
                )?;
                if records_menu_inflate {
                    self.program.sync_inflated_menu_items(&invocation)?;
                }
                return Ok(invocation_result_from_execution(result));
            }
        }

        let proto_descriptor = self
            .program
            .method_proto_descriptor(self.current_dex_file_index, invocation.method_index)?;
        let proto_descriptor: &str = &proto_descriptor;

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "runOnUiThread"
            && proto_descriptor == "(Ljava/lang/Runnable;)V"
        {
            self.run_on_ui_thread(&invocation)?;
            return Ok(DexInvocationResult::Void);
        }

        if method.class_descriptor == "Landroid/os/Handler;"
            && method.name == "post"
            && proto_descriptor == "(Ljava/lang/Runnable;)Z"
        {
            self.post_handler_runnable(&invocation)?;
            return Ok(DexInvocationResult::Value(DexValue::Int(1)));
        }

        // cite: frameworks-base HandlerExecutor.java#execute
        if method.class_descriptor == "Ljava/util/concurrent/Executor;"
            && method.name == "execute"
            && proto_descriptor == "(Ljava/lang/Runnable;)V"
        {
            self.post_handler_runnable(&invocation)?;
            return Ok(DexInvocationResult::Void);
        }

        if method.name == "start"
            && proto_descriptor == "()V"
            && (method.class_descriptor == "Ljava/lang/Thread;"
                || self
                    .program
                    .is_inherited_thread_start(&method, &invocation)?)
        {
            self.start_thread(&invocation)?;
            return Ok(DexInvocationResult::Void);
        }

        if method.name == "wait" && matches!(proto_descriptor, "()V" | "(J)V" | "(JI)V") {
            return Ok(match self.program.object_wait(&invocation, self.depth)? {
                Some(exception) => DexInvocationResult::Throw(exception),
                None => DexInvocationResult::Void,
            });
        }

        if method.class_descriptor == "Landroid/os/HandlerThread;"
            && method.name == "getLooper"
            && proto_descriptor == "()Landroid/os/Looper;"
        {
            let looper = self
                .program
                .handler_thread_looper(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Value(looper));
        }

        if method.class_descriptor == "Landroid/util/Xml;"
            && method.name == "parse"
            && proto_descriptor
                == "(Ljava/io/InputStream;Landroid/util/Xml$Encoding;Lorg/xml/sax/ContentHandler;)V"
        {
            self.program.sax_parse(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if adapter_view_set_adapter_method(&method) {
            if let (Some(adapter_view), Some(adapter)) =
                (invocation.arguments.first(), invocation.arguments.get(1))
            {
                if dex_value_is_object_like(adapter_view) {
                    let adapter = adapter.clone();
                    let adapter_view = adapter_view.clone();
                    self.program.write_adapter_view_adapter(&invocation);
                    if dex_value_is_object_like(&adapter) {
                        self.program
                            .runtime
                            .borrow_mut()
                            .adapter_bound_view
                            .insert(map_identity_key(&adapter), adapter_view.clone());
                        self.materialize_adapter_view(&adapter_view, &adapter)?;
                    } else {
                        self.program.clear_view_group_children(&adapter_view);
                    }
                    return Ok(DexInvocationResult::Void);
                }
            }
        }

        if method.name == "notifyDataSetChanged"
            && proto_descriptor == "()V"
            && invocation
                .arguments
                .first()
                .is_some_and(dex_value_is_object_like)
        {
            if let Some(adapter) = invocation.arguments.first() {
                let adapter = adapter.clone();
                let bound_view = self
                    .program
                    .runtime
                    .borrow()
                    .adapter_bound_view
                    .get(&map_identity_key(&adapter))
                    .cloned();
                if let Some(adapter_view) = bound_view {
                    self.materialize_adapter_view(&adapter_view, &adapter)?;
                    return Ok(DexInvocationResult::Void);
                }
            }
        }

        if activity_set_content_view_runtime_view_method(&method) {
            self.activity_set_content_view_runtime_view(&invocation)?;
            return Ok(DexInvocationResult::Void);
        }

        if dialog_show_runtime_view_method(&method) {
            self.dialog_show_runtime_view(&invocation)?;
            return Ok(DexInvocationResult::Void);
        }

        if dialog_dismiss_runtime_view_method(&method) {
            self.dialog_dismiss_runtime_view(&invocation)?;
            return Ok(DexInvocationResult::Void);
        }

        if view_group_add_view_runtime_child_method(&method) {
            self.view_group_add_view_runtime_child(&method, &invocation)?;
            return Ok(DexInvocationResult::Void);
        }

        if view_group_remove_all_views_method(&method) {
            self.view_group_remove_all_views(&invocation)?;
            return Ok(DexInvocationResult::Void);
        }

        if view_group_draw_child_method(&method) {
            let handled = self.view_group_draw_child(&invocation)?;
            return Ok(DexInvocationResult::Value(DexValue::Int(handled)));
        }

        if arrays_sort_object_with_comparator_method(&method) {
            self.program
                .arrays_sort_object_with_comparator(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if arrays_sort_object_range_method(&method) {
            self.program
                .arrays_sort_object_range_with_comparator(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if collections_sort_list_with_comparator_method(&method) {
            self.program
                .collections_sort_list_with_comparator(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if tree_set_add_value_method(self.program, &method, &invocation) {
            if self.program.sorted_set_add_rejects_null(&invocation) {
                return Ok(DexInvocationResult::Throw(
                    self.program
                        .allocate_object("Ljava/lang/NullPointerException;".to_owned()),
                ));
            }
            let added = self.program.tree_set_add_value(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Value(DexValue::Int(added)));
        }

        if method.class_descriptor == "Landroid/view/View;"
            && method.name == "performClick"
            && proto_descriptor == "()Z"
        {
            let handled = self.program.perform_view_click(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Value(DexValue::Int(handled)));
        }

        if method.class_descriptor == "Landroid/view/MenuItem$OnMenuItemClickListener;"
            && method.name == "onMenuItemClick"
            && proto_descriptor == "(Landroid/view/MenuItem;)Z"
        {
            let handled = self
                .program
                .dispatch_menu_item_declared_on_click_handler(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Value(DexValue::Int(handled)));
        }

        if method.name == "finish"
            && proto_descriptor == "()V"
            && self.program.is_activity_class(&method.class_descriptor)
        {
            self.program.finish_activity(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if let Some(exception) = self.disabled_start_activity_target(&method, &invocation) {
            return Ok(DexInvocationResult::Throw(exception));
        }

        if method.name == "startActivity"
            && matches!(
                proto_descriptor,
                "(Landroid/content/Intent;)V" | "(Landroid/content/Intent;Landroid/os/Bundle;)V"
            )
        {
            self.program
                .start_activity_from_intent(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if method.name == "startActivityForResult"
            && matches!(
                proto_descriptor,
                "(Landroid/content/Intent;I)V" | "(Landroid/content/Intent;ILandroid/os/Bundle;)V"
            )
        {
            let request_code = match invocation.arguments.get(2) {
                Some(DexValue::Int(code)) => *code,
                _ => 0,
            };
            self.program
                .start_activity_for_result(&invocation, request_code, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if method.name == "setResult"
            && matches!(proto_descriptor, "(I)V" | "(ILandroid/content/Intent;)V")
            && self.program.is_activity_class(&method.class_descriptor)
        {
            let result_code = match invocation.arguments.get(1) {
                Some(DexValue::Int(code)) => *code,
                _ => 0,
            };
            let result_intent = invocation.arguments.get(2).cloned();
            self.program.set_activity_result(result_code, result_intent);
            return Ok(DexInvocationResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/app/FragmentTransaction;" | "Landroidx/fragment/app/FragmentTransaction;"
        ) && matches!(
            method.name.as_str(),
            "commit" | "commitAllowingStateLoss" | "commitNow" | "commitNowAllowingStateLoss"
        ) {
            self.program.run_pending_fragment(self.depth)?;
            return Ok(DexInvocationResult::Value(DexValue::Int(0)));
        }

        if method.class_descriptor == "Landroid/widget/TextView;"
            && method.name == "setText"
            && proto_descriptor == "(Ljava/lang/CharSequence;)V"
        {
            self.program
                .set_text_view_text_and_dispatch(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if method.class_descriptor == "Landroid/widget/TextView;"
            && method.name == "setText"
            && proto_descriptor == "(I)V"
        {
            self.program
                .set_text_view_text_resource_and_dispatch(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Void);
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "forName"
            && proto_descriptor == "(Ljava/lang/String;)Ljava/lang/Class;"
        {
            let result = self.program.class_for_name(&invocation, self.depth)?;
            return Ok(invocation_result_from_execution(result));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "forName"
            && proto_descriptor == "(Ljava/lang/String;ZLjava/lang/ClassLoader;)Ljava/lang/Class;"
        {
            let result = self
                .program
                .class_for_name_with_loader(&invocation, self.depth)?;
            return Ok(invocation_result_from_execution(result));
        }

        if method.class_descriptor == "Ljava/lang/Class;"
            && method.name == "newInstance"
            && proto_descriptor == "()Ljava/lang/Object;"
        {
            let result = self.program.class_new_instance(
                &invocation,
                self.current_class_descriptor.as_deref(),
                self.depth,
            )?;
            return Ok(invocation_result_from_execution(result));
        }

        if method.class_descriptor == "Ljava/lang/reflect/Constructor;"
            && method.name == "newInstance"
            && proto_descriptor == "([Ljava/lang/Object;)Ljava/lang/Object;"
        {
            let value = self
                .program
                .reflect_constructor_new_instance(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Value(value));
        }

        if let Some(result) = self
            .program
            .package_manager_component_state_intrinsic(&method, &invocation)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) = self
            .program
            .future_get_intrinsic(&method, &invocation, self.depth)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) =
            self.program
                .future_task_hook_intrinsic(&method, &invocation, self.depth)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) =
            self.program
                .file_output_stream_intrinsic(&method, &invocation, self.depth)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) =
            self.program
                .print_writer_intrinsic(&method, &invocation, self.depth)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) =
            self.program
                .observable_membership_intrinsic(&method, &invocation, self.depth)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) =
            self.program
                .object_stream_intrinsic(&method, &invocation, self.depth)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if method.class_descriptor == "Ljava/lang/System;"
            && proto_descriptor == "(Ljava/lang/String;)V"
            && matches!(method.name.as_str(), "load" | "loadLibrary")
        {
            let result = if method.name == "load" {
                self.program.system_load_result(&invocation)?
            } else {
                self.program.system_load_library_result(&invocation)?
            };
            if let Some(result) = result {
                return Ok(invocation_result_from_execution(result));
            }
        }

        if method.class_descriptor == "Ljava/io/File;"
            && method.name == "listFiles"
            && matches!(
                proto_descriptor,
                "(Ljava/io/FileFilter;)[Ljava/io/File;"
                    | "(Ljava/io/FilenameFilter;)[Ljava/io/File;"
            )
        {
            let result = self.program.file_list_files_with_filter(
                &invocation,
                proto_descriptor.contains("FilenameFilter"),
                self.depth,
            )?;
            return Ok(invocation_result_from_execution(result));
        }

        if method.class_descriptor == "Landroid/view/LayoutInflater;"
            && method.name == "inflate"
            && proto_descriptor == "(ILandroid/view/ViewGroup;Z)Landroid/view/View;"
        {
            let value =
                self.program
                    .layout_inflater_inflate_and_finish(&invocation, 4, self.depth)?;
            return Ok(DexInvocationResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/LayoutInflater;"
            && method.name == "inflate"
            && proto_descriptor == "(ILandroid/view/ViewGroup;)Landroid/view/View;"
        {
            let value =
                self.program
                    .layout_inflater_inflate_and_finish(&invocation, 3, self.depth)?;
            return Ok(DexInvocationResult::Value(value));
        }

        if java_lang_object_equals_method(&method) {
            let value = self
                .program
                .runtime_object_equals(&invocation, self.depth)?;
            return Ok(DexInvocationResult::Value(DexValue::Int(value)));
        }

        if java_lang_object_virtual_method(&method) {
            if let Some(result) =
                self.invoke_receiver_dispatched_method(&method, &invocation, records_menu_inflate)?
            {
                return Ok(result);
            }
        }

        if java_collection_interface_method(&method) {
            if let Some(result) =
                self.invoke_receiver_dispatched_method(&method, &invocation, records_menu_inflate)?
            {
                return Ok(result);
            }
        }

        if view_group_should_delay_child_pressed_state_method(&method)
            && matches!(
                invocation.kind,
                DexInvokeKind::Virtual | DexInvokeKind::Interface
            )
        {
            // cite: art interpreter_common.h#DoInvoke, FindMethodToCall
            if let Some(result) =
                self.invoke_receiver_dispatched_method(&method, &invocation, records_menu_inflate)?
            {
                return Ok(result);
            }

            let receiver_descriptor =
                receiver_dispatch_descriptor(&invocation).ok_or_else(|| {
                    WinedroidError::new(
                        "ViewGroup.shouldDelayChildPressedState requires an object-backed receiver",
                    )
                })?;
            if let Some(result) = self.program.invoke_builtin_method_from_class_hierarchy(
                receiver_descriptor,
                &method,
                &invocation,
            )? {
                return Ok(invocation_result_from_execution(result));
            }

            return Err(WinedroidError::new(format!(
                "ViewGroup.shouldDelayChildPressedState is not implemented for the receiver hierarchy\nreceiver: {receiver_descriptor}\nnext missing primitive: ViewGroup.shouldDelayChildPressedState receiver dispatch"
            )));
        }

        if is_string_builder_append_object(&method) {
            let value = self.append_string_builder_object(&invocation)?;
            return Ok(DexInvocationResult::Value(value));
        }

        if method.class_descriptor == "Ljava/lang/String;"
            && method.name == "valueOf"
            && proto_descriptor == "(Ljava/lang/Object;)Ljava/lang/String;"
        {
            if invocation.arguments.len() != 1 {
                return Err(WinedroidError::new(format!(
                    "String.valueOf(Object) expected 1 argument, got {}",
                    invocation.arguments.len()
                )));
            }
            let value = self.string_value_of_object(&invocation.arguments[0])?;
            return Ok(DexInvocationResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Ljava/util/ArrayList;"
            && method.name == "<init>"
            && proto_descriptor == "(Ljava/util/Collection;)V"
        {
            self.program
                .initialize_array_list_from_collection_iterating(&invocation, self.depth + 1)?;
            return Ok(DexInvocationResult::Void);
        }

        if let Some(result) =
            self.program
                .invoke_java_zip_method(&method, &invocation, self.depth + 1)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) = self.program.invoke_java_collection_equality_method(
            &method,
            &invocation,
            self.depth + 1,
        )? {
            return Ok(invocation_result_from_execution(result));
        }

        if method.class_descriptor == "Landroid/view/View;"
            && method.name == "measure"
            && proto_descriptor == "(II)V"
        {
            if let Some(()) = self
                .program
                .measure_view_with_dispatch(&invocation, self.depth + 1)?
            {
                return Ok(DexInvocationResult::Void);
            }
        }

        if method.class_descriptor == "Landroid/view/inputmethod/InputConnectionWrapper;" {
            if method.name == "<init>"
                && proto_descriptor == "(Landroid/view/inputmethod/InputConnection;Z)V"
            {
                self.program.input_connection_wrapper_init(&invocation);
                return Ok(DexInvocationResult::Void);
            }
            if method.name == "setTarget"
                && proto_descriptor == "(Landroid/view/inputmethod/InputConnection;)V"
            {
                self.program
                    .input_connection_wrapper_set_target(&invocation);
                return Ok(DexInvocationResult::Void);
            }
            if let Some(result) = self.program.input_connection_wrapper_delegate(
                &method,
                &invocation,
                self.depth + 1,
            )? {
                return Ok(invocation_result_from_execution(result));
            }
        }

        let known_direct_code = direct_kind
            && self
                .program
                .caches
                .borrow()
                .direct_code_methods
                .contains(&method_key);
        let known_virtual_code = !direct_kind
            && object_receiver_dispatch_descriptor(&invocation).is_some_and(|receiver_class| {
                self.program
                    .caches
                    .borrow()
                    .virtual_code_methods
                    .get(&method_key)
                    .is_some_and(|classes| classes.contains(receiver_class))
            });
        if !known_direct_code && !known_virtual_code {
            if let Some(result) = self.program.invoke_builtin_method_hinted(
                Some(Rc::as_ptr(&method) as usize),
                &method,
                Some(&invocation),
            ) {
                return Ok(invocation_result_from_execution(result));
            }
        }

        // cite: libcore AnnotationFactory.java#invoke
        if let Some(value) = self.program.annotation_element_value(&method, &invocation) {
            return Ok(DexInvocationResult::Value(value));
        }

        if let Some(result) =
            self.invoke_receiver_dispatched_method(&method, &invocation, records_menu_inflate)?
        {
            if let Some(receiver_class) = object_receiver_dispatch_descriptor(&invocation) {
                self.program
                    .caches
                    .borrow_mut()
                    .virtual_code_methods
                    .entry(method_key)
                    .or_default()
                    .insert(receiver_class.to_owned());
            }
            return Ok(result);
        }

        if let Some(resolved) = self.program.resolve_exact_method(&method)? {
            if direct_kind && resolved.encoded_method.code_offset != 0 {
                let null_assert = self
                    .program
                    .code_item(&resolved)
                    .ok()
                    .and_then(|code| detect_null_assert_arg(self.program, &resolved, &code));
                let mut caches = self.program.caches.borrow_mut();
                caches.direct_code_methods.insert(method_key);
                caches
                    .direct_resolved_methods
                    .insert(method_key, resolved.clone());
                caches.null_assert_args.insert(method_key, null_assert);
            }
            if let Some(result) = self.invoke_resolved_intrinsic(&resolved, &invocation)? {
                if records_menu_inflate {
                    self.program.sync_inflated_menu_items(&invocation)?;
                }
                return Ok(result);
            }
            let result = self.program.execute_method(
                None,
                &resolved,
                Some(&invocation.arguments),
                self.depth + 1,
            )?;
            if records_menu_inflate {
                self.program.sync_inflated_menu_items(&invocation)?;
            }
            return Ok(invocation_result_from_execution(result));
        }

        if invocation.kind == DexInvokeKind::Super {
            // cite: art class.cc#FindClassMethodWithSignature
            if let Some(resolved) = self
                .program
                .resolve_exact_method_in_class_hierarchy(&method.class_descriptor, &method)?
            {
                if let Some(result) = self.invoke_resolved_intrinsic(&resolved, &invocation)? {
                    if records_menu_inflate {
                        self.program.sync_inflated_menu_items(&invocation)?;
                    }
                    return Ok(result);
                }
                let result = self.program.execute_method(
                    None,
                    &resolved,
                    Some(&invocation.arguments),
                    self.depth + 1,
                )?;
                if records_menu_inflate {
                    self.program.sync_inflated_menu_items(&invocation)?;
                }
                return Ok(invocation_result_from_execution(result));
            }
        }

        // cite: frameworks-base Application.java#unregisterActivityLifecycleCallbacks
        if method.name == "unregisterActivityLifecycleCallbacks"
            && proto_descriptor == "(Landroid/app/Application$ActivityLifecycleCallbacks;)V"
            && self
                .program
                .descriptor_matches_type(&method.class_descriptor, "Landroid/app/Application;")
            && self
                .program
                .unregister_application_activity_lifecycle_callback(&invocation, self.depth + 1)?
                .is_some()
        {
            return Ok(DexInvocationResult::Void);
        }

        if let Some(result) = self
            .program
            .invoke_builtin_method_in_hierarchy(&method, &invocation)
        {
            return Ok(invocation_result_from_execution(result));
        }

        // cite: libcore AbstractList.java#listIterator
        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;" | "Ljava/util/AbstractList;"
        ) && method.name == "listIterator"
            && method.proto.descriptor_is("(I)Ljava/util/ListIterator;")
        {
            if let Some(value) = self
                .program
                .dex_backed_list_iterator_at(&invocation, self.depth + 1)?
            {
                return Ok(DexInvocationResult::Value(value));
            }
        }

        // cite: libcore AbstractList.java#iterator
        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/List;"
                | "Ljava/util/AbstractList;"
                | "Ljava/util/Collection;"
                | "Ljava/util/AbstractCollection;"
                | "Ljava/lang/Iterable;"
        ) && method.name == "iterator"
            && method.proto.descriptor_is("()Ljava/util/Iterator;")
        {
            if let Some(value) = self
                .program
                .dex_backed_list_iterator(&invocation, self.depth + 1)?
            {
                return Ok(DexInvocationResult::Value(value));
            }
        }

        if let Some(result) =
            self.program
                .invoke_lru_cache_method(&method, &invocation, self.depth + 1)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) = self
            .program
            .invoke_uri_builder_method(&method, &invocation)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) = self
            .program
            .invoke_uri_matcher_method(&method, &invocation)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        if let Some(result) =
            self.program
                .invoke_content_provider_method(&method, &invocation, self.depth + 1)?
        {
            return Ok(invocation_result_from_execution(result));
        }

        Err(WinedroidError::new(format!(
            "DEX invocation not implemented\nkind: {}\nmethod: {}.{}\nargument registers: {}\nnext missing primitive: method {}.{}",
            invocation.instruction_name(),
            method.class_descriptor,
            method.display_signature(),
            argument_registers_display(&invocation.argument_registers),
            method.class_descriptor,
            method.display_signature()
        )))
    }
    pub(super) fn invoke_supported_from_frame(
        &mut self,
        invocation: DexInvocationFrame<'_>,
    ) -> Result<DexInvocationResult> {
        #[cfg(all(feature = "jit", not(target_family = "wasm")))]
        if jit_dispatch_enabled() {
            return self.invoke(invocation.materialize()?);
        }

        if cached_env_flag(&DEX_TRACE, "WINEDROID_DEX_TRACE") || method_trace_prefixes().is_some() {
            return self.invoke(invocation.materialize()?);
        }

        let method_key = method_cache_key(self.current_dex_file_index, invocation.method_index());
        if matches!(
            invocation.kind(),
            DexInvokeKind::Static | DexInvokeKind::Direct
        ) {
            let cached = self
                .program
                .caches
                .borrow()
                .direct_resolved_methods
                .get(&method_key)
                .cloned();
            if let Some(resolved) = cached {
                if resolved.encoded_method.code_offset == 0 {
                    return self.invoke(invocation.materialize()?);
                }
                if is_menu_inflater_inflate_method(&resolved.method)
                    || records_content_view_method(&resolved.method)
                {
                    return self.invoke(invocation.materialize()?);
                }
                if let Some(Some(arg_index)) = self
                    .program
                    .caches
                    .borrow()
                    .null_assert_args
                    .get(&method_key)
                    .copied()
                {
                    if null_assert_shortcut_applies(&invocation, &resolved, arg_index)? {
                        return Ok(DexInvocationResult::Void);
                    }
                }
                if let Some(intrinsic) = self.program.method_intrinsic(&resolved)? {
                    if let Some(result) = self.invoke_resolved_intrinsic_from_frame(
                        &resolved,
                        intrinsic,
                        &invocation,
                    )? {
                        return Ok(result);
                    }
                    return self.invoke(invocation.materialize()?);
                }
                let result = self.program.execute_method_from_invocation_frame(
                    &resolved,
                    &invocation,
                    self.depth + 1,
                )?;
                return Ok(invocation_result_from_execution(result));
            }
        }

        if matches!(
            invocation.kind(),
            DexInvokeKind::Virtual | DexInvokeKind::Interface
        ) {
            let cached =
                invocation
                    .object_argument_class_descriptor(0)?
                    .and_then(|receiver_descriptor| {
                        self.program
                            .caches
                            .borrow()
                            .virtual_resolved_methods
                            .get(&method_key)
                            .and_then(|by_receiver| by_receiver.get(receiver_descriptor))
                            .cloned()
                    });
            if let Some(resolved) = cached {
                if resolved.encoded_method.code_offset == 0 {
                    return self.invoke(invocation.materialize()?);
                }
                if is_menu_inflater_inflate_method(&resolved.method)
                    || records_content_view_method(&resolved.method)
                {
                    return self.invoke(invocation.materialize()?);
                }
                if let Some(intrinsic) = self.program.method_intrinsic(&resolved)? {
                    if let Some(result) = self.invoke_resolved_intrinsic_from_frame(
                        &resolved,
                        intrinsic,
                        &invocation,
                    )? {
                        return Ok(result);
                    }
                    return self.invoke(invocation.materialize()?);
                }
                let result = self.program.execute_method_from_invocation_frame(
                    &resolved,
                    &invocation,
                    self.depth + 1,
                )?;
                return Ok(invocation_result_from_execution(result));
            }
        }

        if let Some(result) = invoke_uncached_code_from_frame(self, &invocation, method_key)? {
            return Ok(result);
        }

        self.invoke(invocation.materialize()?)
    }
}

// cite: libcore StringBuilder.java#append
// cite: libcore StringBuffer.java#append
pub(super) fn is_string_builder_append_object(method: &DexMethodId) -> bool {
    method.name == "append"
        && match method.class_descriptor.as_str() {
            "Ljava/lang/StringBuilder;" => method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/lang/StringBuilder;"),
            "Ljava/lang/StringBuffer;" => method
                .proto
                .descriptor_is("(Ljava/lang/Object;)Ljava/lang/StringBuffer;"),
            _ => false,
        }
}

impl DexProgram {
    // cite: libcore FutureTask.java#get, awaitDone
    pub(super) fn future_get_intrinsic(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let future_class = method.class_descriptor == "Ljava/util/concurrent/Future;"
            || self.descriptor_matches_type(
                &method.class_descriptor,
                "Ljava/util/concurrent/FutureTask;",
            );
        let descriptor = method.proto.descriptor();
        let timed = descriptor == "(JLjava/util/concurrent/TimeUnit;)Ljava/lang/Object;";
        if !future_class
            || method.name != "get"
            || !(timed || descriptor == "()Ljava/lang/Object;")
            || self.guest_overrides(method, Some(invocation))
        {
            return Ok(None);
        }
        let mut untimed = invocation.clone();
        untimed.arguments.truncate(1);

        self.future_task_get(&untimed, depth).map(Some)
    }

    // cite: libcore FutureTask.java#run, set, setException, done
    pub(super) fn future_task_hook_intrinsic(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        if !self.descriptor_matches_type(
            &method.class_descriptor,
            "Ljava/util/concurrent/FutureTask;",
        ) || self.guest_overrides(method, Some(invocation))
        {
            return Ok(None);
        }
        let Some(receiver) = invocation
            .arguments
            .first()
            .filter(|receiver| dex_value_is_object_like(receiver))
        else {
            return Ok(None);
        };
        let descriptor = method.proto.descriptor();
        match (
            method.name.as_str(),
            descriptor.as_str(),
            invocation.arguments.get(1),
        ) {
            ("run", "()V", None) => self.future_task_run_result(receiver, depth).map(Some),
            ("set", "(Ljava/lang/Object;)V", Some(value)) => self
                .future_task_set_outcome(receiver, value.clone(), false, depth)
                .map(Some),
            ("setException", "(Ljava/lang/Throwable;)V", Some(exception)) => self
                .future_task_set_outcome(receiver, exception.clone(), true, depth)
                .map(Some),
            ("done", "()V", None) => Ok(Some(DexExecutionResult::Void)),
            // cite: libcore FutureTask.java#cancel, finishCompletion
            ("cancel", "(Z)Z", Some(DexValue::Int(_))) => {
                let Some(cancelled) = self.future_task_cancel(invocation) else {
                    return Ok(None);
                };
                if cancelled == 1 {
                    if let Some(result) =
                        self.guest_void_override(receiver, "done", &[], Vec::new(), depth)?
                    {
                        if !matches!(result, DexExecutionResult::Void) {
                            return Ok(Some(result));
                        }
                    }
                }
                Ok(Some(DexExecutionResult::Int(cancelled)))
            }
            _ => Ok(None),
        }
    }

    // cite: libcore FileOutputStream.java#write, flush, close
    pub(super) fn file_output_stream_intrinsic(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let stream_class = "Ljava/io/FileOutputStream;";
        if !(method.class_descriptor == "Ljava/io/OutputStream;"
            || self.descriptor_matches_type(&method.class_descriptor, stream_class))
        {
            return Ok(None);
        }
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        if !self.value_matches_type(receiver, stream_class)
            || !self.file_output_stream_is_open_by_path(receiver)
            || self.guest_overrides(method, Some(invocation))
        {
            return Ok(None);
        }
        let descriptor = method.proto.descriptor();
        let arguments = &invocation.arguments[1..];
        let range = |array: &DexValue, offset: i32, count: i32| {
            let parameters = ["[B", "I", "I"];
            let values = vec![array.clone(), DexValue::Int(offset), DexValue::Int(count)];
            match self.guest_void_override(receiver, "write", &parameters, values, depth)? {
                Some(result) => Ok(Some(result)),
                None => Ok(self.file_output_stream_write_bytes(receiver, array, offset, count)),
            }
        };
        match (method.name.as_str(), descriptor.as_str(), arguments) {
            ("write", "(I)V", [DexValue::Int(byte)]) => {
                let array = self.allocate_filled_array(
                    "[B".to_owned(),
                    vec![DexValue::Int(i32::from(*byte as i8))],
                );
                range(&array, 0, 1)
            }
            ("write", "([B)V", [array]) => match self.array_length(array) {
                Some(length) => range(array, 0, length),
                None => Ok(Some(self.null_pointer_exception_result())),
            },
            ("write", "([BII)V", [array, DexValue::Int(offset), DexValue::Int(count)]) => {
                Ok(self.file_output_stream_write_bytes(receiver, array, *offset, *count))
            }
            ("flush", "()V", []) => Ok(Some(DexExecutionResult::Void)),
            ("close", "()V", []) => Ok(self.file_output_stream_close(receiver)),
            _ => Ok(None),
        }
    }

    // cite: libcore PrintWriter.java#PrintWriter, println, newLine, close, ensureOpen
    // cite: libcore Writer.java#Writer
    pub(super) fn print_writer_intrinsic(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let print_writer = "Ljava/io/PrintWriter;";
        if !self.descriptor_matches_type(&method.class_descriptor, print_writer)
            || self.guest_overrides(method, Some(invocation))
        {
            return Ok(None);
        }
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        if !self.value_matches_type(receiver, print_writer) {
            return Ok(None);
        }
        let out_field = print_writer_field("Ljava/io/Writer;", "out");
        let descriptor = method.proto.descriptor();
        match (
            method.name.as_str(),
            descriptor.as_str(),
            &invocation.arguments[1..],
        ) {
            ("<init>", "(Ljava/io/Writer;)V" | "(Ljava/io/Writer;Z)V", [out, rest @ ..]) => {
                if matches!(out, DexValue::Null | DexValue::Int(0)) {
                    return Ok(Some(self.null_pointer_exception_result()));
                }
                let auto_flush = match rest {
                    [] => 0,
                    [DexValue::Int(flag)] => i32::from(*flag != 0),
                    _ => return Ok(None),
                };
                self.write_stored_instance_field(receiver, &out_field, out.clone());
                self.write_stored_instance_field(
                    receiver,
                    &print_writer_field("Z", "autoFlush"),
                    DexValue::Int(auto_flush),
                );
                Ok(Some(DexExecutionResult::Void))
            }
            ("println", "()V", []) => {
                let Some(out) = self.print_writer_open_out(receiver) else {
                    self.print_writer_set_trouble(receiver);
                    return Ok(Some(DexExecutionResult::Void));
                };
                let separator = DexValue::String(std::rc::Rc::from("\n"));
                let Some(result) = self.writer_write_string(&out, &separator, depth)? else {
                    return Ok(None);
                };
                if let DexExecutionResult::Throw(exception) = &result {
                    if self.value_matches_type(exception, "Ljava/io/IOException;") {
                        self.print_writer_set_trouble(receiver);
                        return Ok(Some(DexExecutionResult::Void));
                    }
                    return Ok(Some(result));
                }
                let auto_flush = self
                    .read_stored_instance_field(receiver, &print_writer_field("Z", "autoFlush"))
                    == Some(DexValue::Int(1));
                if auto_flush {
                    let flushed =
                        self.guest_void_override(&out, "flush", &[], Vec::new(), depth)?;
                    if flushed.is_none() {
                        return Ok(None);
                    }
                }
                Ok(Some(DexExecutionResult::Void))
            }
            ("close", "()V", []) => {
                let Some(out) = self.print_writer_open_out(receiver) else {
                    return Ok(Some(DexExecutionResult::Void));
                };
                let result =
                    match self.guest_void_override(&out, "close", &[], Vec::new(), depth)? {
                        Some(result) => result,
                        None => {
                            let close = DexMethodId {
                                class_descriptor: value_class_descriptor(&out)
                                    .unwrap_or_default()
                                    .to_owned(),
                                name: "close".to_owned(),
                                proto: dex::DexProto {
                                    shorty: "V".to_owned(),
                                    return_type: "V".to_owned(),
                                    parameters: Vec::new(),
                                },
                            };
                            let mut close_invocation = invocation.clone();
                            close_invocation.arguments = smallvec::smallvec![out.clone()];
                            match self.invoke_builtin_method(&close, Some(&close_invocation)) {
                                Some(result) => result,
                                None => return Ok(None),
                            }
                        }
                    };
                if let DexExecutionResult::Throw(exception) = &result {
                    if !self.value_matches_type(exception, "Ljava/io/IOException;") {
                        return Ok(Some(result));
                    }
                    self.print_writer_set_trouble(receiver);
                    return Ok(Some(DexExecutionResult::Void));
                }
                self.write_stored_instance_field(receiver, &out_field, DexValue::Null);
                Ok(Some(DexExecutionResult::Void))
            }
            _ => Ok(None),
        }
    }

    fn print_writer_open_out(&self, writer: &DexValue) -> Option<DexValue> {
        self.read_stored_instance_field(writer, &print_writer_field("Ljava/io/Writer;", "out"))
            .filter(dex_value_is_object_like)
    }

    fn print_writer_set_trouble(&self, writer: &DexValue) {
        self.write_stored_instance_field(
            writer,
            &print_writer_field("Z", "trouble"),
            DexValue::Int(1),
        );
    }

    // cite: libcore Writer.java#write
    fn writer_write_string(
        &self,
        writer: &DexValue,
        text: &DexValue,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let string = "Ljava/lang/String;";
        if let Some(result) =
            self.guest_void_override(writer, "write", &[string], vec![text.clone()], depth)?
        {
            return Ok(Some(result));
        }
        let Some(content) = self.string_text(text) else {
            return Ok(None);
        };
        let units: Vec<u16> = content.encode_utf16().collect();
        let length = DexValue::Int(i32::try_from(units.len()).unwrap_or(i32::MAX));
        if let Some(result) = self.guest_void_override(
            writer,
            "write",
            &[string, "I", "I"],
            vec![text.clone(), DexValue::Int(0), length.clone()],
            depth,
        )? {
            return Ok(Some(result));
        }
        let chars = self.allocate_filled_array(
            "[C".to_owned(),
            units
                .iter()
                .map(|unit| DexValue::Int(i32::from(*unit)))
                .collect(),
        );
        if let Some(result) = self.guest_void_override(
            writer,
            "write",
            &["[C", "I", "I"],
            vec![chars, DexValue::Int(0), length],
            depth,
        )? {
            return Ok(Some(result));
        }
        let write = DexMethodId {
            class_descriptor: value_class_descriptor(writer)
                .unwrap_or_default()
                .to_owned(),
            name: "write".to_owned(),
            proto: dex::DexProto {
                shorty: "VL".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec![string.to_owned()],
            },
        };
        let invocation = DexInvocation {
            kind: DexInvokeKind::Virtual,
            range: false,
            method_index: 0,
            argument_registers: Default::default(),
            arguments: smallvec::smallvec![writer.clone(), text.clone()],
        };
        Ok(self.invoke_builtin_method(&write, Some(&invocation)))
    }
}

impl DexProgram {
    // cite: libcore Observable.java#addObserver, deleteObserver
    // cite: libcore Vector.java#contains, indexOf, removeElement
    pub(super) fn observable_membership_intrinsic(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        if !self.descriptor_matches_type(&method.class_descriptor, "Ljava/util/Observable;")
            || !matches!(method.name.as_str(), "addObserver" | "deleteObserver")
            || !method.proto.descriptor_is("(Ljava/util/Observer;)V")
            || self.guest_overrides(method, Some(invocation))
        {
            return Ok(None);
        }
        let [observable, observer] = invocation.arguments.as_slice() else {
            return Ok(None);
        };
        if !dex_value_is_object_like(observable) {
            return Ok(None);
        }
        let observers_field = DexFieldId {
            class_descriptor: "Ljava/util/Observable;".to_owned(),
            type_descriptor: "[Ljava/util/Observer;".to_owned(),
            name: "obs".to_owned(),
        };
        let mut observers = match self.read_stored_instance_field(observable, &observers_field) {
            Some(array @ DexValue::Object(_)) => self
                .runtime
                .borrow()
                .array_entries
                .get(&map_identity_key(&array))
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let null_observer = matches!(observer, DexValue::Null | DexValue::Int(0));
        if method.name == "addObserver" && null_observer {
            return Ok(Some(self.null_pointer_exception_result()));
        }
        let mut position = None;
        for (index, existing) in observers.iter().enumerate() {
            let equal = if null_observer {
                matches!(existing, DexValue::Null)
            } else {
                self.runtime_values_equal(observer, existing, depth, true)?
            };
            if equal {
                position = Some(index);
                break;
            }
        }
        match (method.name.as_str(), position) {
            ("addObserver", Some(_)) => return Ok(Some(DexExecutionResult::Void)),
            ("addObserver", None) => observers.push(observer.clone()),
            (_, Some(index)) => {
                observers.remove(index);
            }
            (_, None) => return Ok(Some(DexExecutionResult::Void)),
        }
        let array = self.allocate_filled_array("[Ljava/util/Observer;".to_owned(), observers);
        self.write_stored_instance_field(observable, &observers_field, array);

        Ok(Some(DexExecutionResult::Void))
    }
}

fn print_writer_field(type_descriptor: &str, name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/io/PrintWriter;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

impl DexRunInvocationHandler<'_> {
    // cite: frameworks-base Instrumentation.java#execStartActivity, checkStartActivityResult
    pub(super) fn disabled_start_activity_target(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let starts_activity = matches!(
            (method.name.as_str(), method.proto.descriptor().as_str()),
            ("startActivity", "(Landroid/content/Intent;)V")
                | (
                    "startActivity",
                    "(Landroid/content/Intent;Landroid/os/Bundle;)V"
                )
                | ("startActivityForResult", "(Landroid/content/Intent;I)V")
                | (
                    "startActivityForResult",
                    "(Landroid/content/Intent;ILandroid/os/Bundle;)V"
                )
        );
        if !starts_activity {
            return None;
        }
        let target = self.program.start_activity_target_class(invocation)?;
        self.program.disabled_explicit_activity_exception(&target)
    }
}
