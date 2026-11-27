use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::binary_xml::XmlEvent;
use crate::dex::{DexFieldId, DexMethodId};
use crate::dex_interpreter::{DexInvocation, DexParkedExecution, DexValue};
use crate::resource_table::ResourcePrimitive;

use super::helpers::map_identity_key;
use super::{DexAdapterRows, DexHostAction, DexRuntimeViewRoot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeSubList {
    pub(super) parent_key: String,
    pub(super) from_index: usize,
    pub(super) to_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct AdapterItemClick {
    pub(super) list_view: DexValue,
    pub(super) position: i32,
    pub(super) adapter_view_id: i32,
}

pub(super) const ADAPTER_ITEM_CLICK_ID_BASE: i32 = 0x6F00_0000;
const ADAPTER_ITEM_CLICK_ID_MAX: i32 = 0x6FFF_FFFF;

pub(super) struct DexRuntimeState {
    pub(super) next_object_id: u64,
    pub(super) instance_fields: InstanceFieldStore,
    pub(super) reference_fields: ReferenceFieldStore,
    // cite: frameworks-base View.java#measure, requestLayout, mMeasureCache
    pub(super) view_measure_caches: crate::fxhash::FxHashMap<String, Vec<(u64, u64)>>,
    pub(super) static_fields: StaticFieldStore,
    pub(super) initialized_classes: HashSet<String>,
    pub(super) initializing_classes: HashMap<String, String>,
    pub(super) class_initialization_waiters: HashMap<String, Vec<ClassInitializationWaiter>>,
    pub(super) lru_caches: HashMap<String, super::android::lru_cache::LruCacheState>,
    pub(super) uri_matchers: HashMap<String, super::android::uri_matcher::UriMatcherNode>,
    pub(super) content_providers: HashMap<String, DexValue>,
    pub(super) map_entries: HashMap<(String, String), DexValue>,
    pub(super) map_entry_keys: HashMap<(String, String), DexValue>,
    pub(super) map_entry_objects: HashMap<String, (DexValue, DexValue)>,
    pub(super) proxy_classes: HashMap<String, super::DynamicProxyClass>,
    pub(super) proxy_class_keys: HashMap<String, String>,
    pub(super) proxy_instances: HashMap<u64, super::DynamicProxyInstance>,
    pub(super) next_proxy_index: u64,
    pub(super) generic_types: HashMap<u64, super::GenericType>,
    pub(super) next_generic_type_id: u64,
    pub(super) list_entries: HashMap<String, Vec<DexValue>>,
    pub(super) list_sub_lists: HashMap<String, RuntimeSubList>,
    pub(super) tree_set_comparators: HashMap<String, DexValue>,
    pub(super) iterator_entries: HashMap<String, Vec<DexValue>>,
    pub(super) iterator_positions: HashMap<String, usize>,
    pub(super) iterator_sources: HashMap<String, String>,
    pub(super) iterator_last_returned_indices: HashMap<String, usize>,
    pub(super) array_entries: HashMap<String, Vec<DexValue>>,
    pub(super) json_entries: HashMap<(String, String), DexValue>,
    pub(super) json_key_order: HashMap<String, Vec<String>>,
    pub(super) content_values_entries: HashMap<(String, String), DexValue>,
    pub(super) bundle_entries: HashMap<(String, String), DexValue>,
    pub(super) sparse_array_entries: HashMap<String, Vec<(i32, DexValue)>>,
    pub(super) sqlite_database_paths: HashMap<String, PathBuf>,
    pub(super) sqlite_cursors: HashMap<String, SqliteCursorState>,
    // cite: frameworks-base SQLiteProgram.java#bindString, bindLong
    pub(super) sqlite_bind_values: HashMap<(String, usize), DexValue>,
    // cite: frameworks-base SQLiteStatement.java#SQLiteStatement
    // cite: frameworks-base SQLiteProgram.java#mSql
    pub(super) sqlite_statement_sql: HashMap<String, String>,
    pub(super) sqlite_statement_database: HashMap<String, DexValue>,
    // cite: frameworks-base SQLiteOpenHelper.java#getDatabaseLocked, mDatabase
    pub(super) sqlite_open_helpers_opened: HashSet<String>,
    pub(super) shared_preferences_entries: HashMap<(String, String), DexValue>,
    pub(super) shared_preferences_storage_kinds:
        HashMap<String, crate::guest_filesystem::PrivateStorageKind>,
    pub(super) context_storage_kinds: HashMap<String, crate::guest_filesystem::PrivateStorageKind>,
    pub(super) intent_extras: HashMap<(String, String), DexValue>,
    pub(super) intent_extra_types: HashMap<(String, String), String>,
    pub(super) adapter_item_click_listeners: HashMap<(String, i32), DexValue>,
    pub(super) shared_preferences_editors: HashMap<String, SharedPreferencesEditorState>,
    pub(super) string_values: HashMap<String, String>,
    pub(super) string_builder_values: HashMap<String, String>,
    pub(super) string_writer_values: HashMap<String, String>,
    pub(super) number_format_values: HashMap<String, NumberFormatState>,
    pub(super) input_stream_values: HashMap<String, InputStreamState>,
    pub(super) zip_files: HashMap<String, super::builtins::java_zip::ZipFileState>,
    pub(super) sax_attributes: HashMap<String, Vec<(String, String)>>,
    pub(super) spannable_spans: HashMap<String, Vec<SpannableSpanState>>,
    pub(super) enum_values: HashMap<String, EnumRuntimeValue>,
    pub(super) atomic_boolean_values: HashMap<String, i32>,
    pub(super) atomic_integer_values: HashMap<String, i32>,
    pub(super) atomic_long_values: HashMap<String, i64>,
    pub(super) semaphore_permits: HashMap<String, i32>,
    pub(super) condition_waiters: HashMap<String, Vec<DexValue>>,
    pub(super) atomic_reference_values: HashMap<String, DexValue>,
    pub(super) weak_reference_values: HashMap<String, DexValue>,
    pub(super) handler_loopers: HashMap<String, DexValue>,
    pub(super) shutdown_executors: HashSet<String>,
    pub(super) current_guest_thread: DexValue,
    pub(super) guest_thread_ids: HashMap<String, i32>,
    pub(super) thread_priorities: HashMap<i32, i32>,
    pub(super) finished_activities: HashSet<String>,
    pub(super) pending_destroyed_activities: Vec<String>,
    pub(super) destroyed_activities: HashSet<String>,
    pub(super) idle_handlers: Vec<(String, DexValue)>,
    pub(super) queue_capacities: HashMap<String, usize>,
    pub(super) scrollers: HashMap<String, super::android::scroller::ScrollerState>,
    pub(super) executor_handlers: HashMap<String, DexValue>,
    pub(super) executor_workers: HashMap<String, DexValue>,
    pub(super) next_thread_pool_number: u32,
    pub(super) next_thread_number: u32,
    pub(super) idle_handlers_ran: bool,
    pub(super) default_uncaught_exception_handler: Option<DexValue>,
    pub(super) boot_wall_clock_millis: i64,
    pub(super) main_thread_uptime_millis: i64,
    pub(super) process_start_uptime_millis: i64,
    pub(super) next_frame_time_nanos: i64,
    pub(super) frame_time_taken_since_drive: bool,
    pub(super) traversal_dispatched_since_drive: bool,
    pub(super) frame_interval_nanos: i64,
    pub(super) next_main_thread_sequence: u64,
    pub(super) main_thread_tasks: Vec<MainThreadTask>,
    pub(super) handler_posts: Vec<HandlerPost>,
    pub(super) choreographer_frame_callbacks: Vec<DexValue>,
    pub(super) choreographer_delayed_frame_callbacks: Vec<ChoreographerFrameCallback>,
    pub(super) choreographer_runnable_callbacks: Vec<ChoreographerRunnableCallback>,
    pub(super) parked_threads: HashMap<String, DexParkedExecution>,
    pub(super) next_parked_runtime_action_id: u64,
    pub(super) parked_runtime_actions: HashMap<u64, ParkedRuntimeAction>,
    pub(super) thread_park_permits: HashSet<String>,
    pub(super) parked_thread_values: HashMap<String, DexValue>,
    pub(super) parked_thread_deadlines: HashMap<String, i64>,
    pub(super) thread_resumes: Vec<ThreadResume>,
    pub(super) random_seeds: HashMap<String, i64>,
    pub(super) string_reader_values: HashMap<String, StringReaderState>,
    pub(super) regex_patterns: HashMap<String, String>,
    pub(super) regex_matchers: HashMap<String, RegexMatcherState>,
    pub(super) bootstrap_field_owners: HashMap<String, BootstrapFieldOwner>,
    pub(super) unsafe_field_offsets: HashMap<i64, DexFieldId>,
    pub(super) accessibility_state_change_listeners: HashMap<String, Vec<DexValue>>,
    pub(super) accessibility_touch_exploration_state_change_listeners:
        HashMap<String, Vec<DexValue>>,
    pub(super) matrix_values: HashMap<String, AndroidMatrixState>,
    pub(super) big_decimal_values: HashMap<String, String>,
    pub(super) big_integer_values: HashMap<String, String>,
    pub(super) math_context_values: HashMap<String, MathContextRuntimeValue>,
    pub(super) intent_filter_actions: HashMap<String, Vec<String>>,
    pub(super) component_callbacks: HashMap<String, Vec<DexValue>>,
    pub(super) activity_lifecycle_callbacks: HashMap<String, Vec<DexValue>>,
    pub(super) view_tags: HashMap<(String, i32), DexValue>,
    pub(super) annotation_elements: HashMap<(String, String), DexValue>,
    pub(super) back_invoked_callbacks: HashMap<String, Vec<(i32, DexValue)>>,
    pub(super) view_click_listeners: HashMap<String, DexValue>,
    pub(super) view_text_watchers: HashMap<String, Vec<DexValue>>,
    pub(super) view_attach_state_listeners: HashMap<String, Vec<DexValue>>,
    pub(super) view_tree_global_layout_listeners: HashMap<String, Vec<DexValue>>,
    pub(super) view_tree_draw_listeners: HashMap<String, Vec<DexValue>>,
    pub(super) view_tree_pre_draw_listeners: HashMap<String, Vec<DexValue>>,
    pub(super) view_tree_scroll_changed_listeners: HashMap<String, Vec<DexValue>>,
    pub(super) view_tree_window_focus_change_listeners: HashMap<String, Vec<DexValue>>,
    pub(super) view_tree_touch_mode_change_listeners: HashMap<String, Vec<DexValue>>,
    pub(super) bitmap_encoded_images: HashMap<String, Vec<u8>>,
    pub(super) canvas_draw_ops: HashMap<String, Vec<CanvasDrawOp>>,
    pub(super) active_canvas_key: Option<String>,
    pub(super) render_node_draw_ops: HashMap<String, Vec<CanvasDrawOp>>,
    pub(super) render_node_generations: HashMap<String, u64>,
    pub(super) view_render_nodes: HashMap<String, DexValue>,
    pub(super) view_render_node_dirty: HashSet<String>,
    pub(super) view_render_node_generations: HashMap<String, u64>,
    pub(super) root_render_nodes: HashMap<String, DexValue>,
    pub(super) root_render_node_generations: HashMap<String, u64>,
    pub(super) runtime_canvas_frames: Vec<RuntimeCanvasFrame>,
    pub(super) window_visible_frame: Option<(i32, i32)>,
    pub(super) path_ops: HashMap<String, Vec<PathOp>>,
    pub(super) view_declared_on_click_handlers: HashMap<String, String>,
    pub(super) view_group_children: HashMap<String, Vec<DexValue>>,
    pub(super) view_parents: HashMap<String, DexValue>,
    pub(super) adapter_bound_view: HashMap<String, DexValue>,
    pub(super) adapter_rows: HashMap<i32, DexAdapterRows>,
    pub(super) adapter_item_clicks: HashMap<i32, AdapterItemClick>,
    pub(super) next_adapter_item_click_id: i32,
    // cite: frameworks-base View.java#setOnClickListener
    pub(super) dynamic_view_click_targets: HashMap<i32, DexValue>,
    pub(super) focused_view: Option<DexValue>,
    pub(super) options_menu: Vec<RuntimeMenuItem>,
    pub(super) inflated_preferences: Vec<InflatedPreference>,
    pub(super) pending_fragment: Option<DexValue>,
    pub(super) action_bar_title: Option<String>,
    pub(super) action_bar_nav_icon: Option<i32>,
    pub(super) action_bar_nav_click: Option<DexValue>,
    pub(super) action_bar_overflow_open: bool,
    pub(super) action_bar_present: bool,
    pub(super) action_bar_home_as_up: bool,
    pub(super) runtime_view_roots: Vec<DexRuntimeViewRoot>,
    pub(super) traversal_request: TraversalRequest,
    pub(super) inflated_view_children: HashMap<(String, i32), DexValue>,
    pub(super) layout_root_descriptors: HashMap<i32, Option<String>>,
    pub(super) decor_action_bar: Option<bool>,
    pub(super) xml_parsers: HashMap<String, XmlParserState>,
    pub(super) menus: HashMap<String, RuntimeMenu>,
    pub(super) active_menu_overlays: Vec<RuntimeMenuOverlay>,
    pub(super) active_dialog_overlays: Vec<RuntimeDialogOverlay>,
    pub(super) themes: HashMap<String, RuntimeTheme>,
    pub(super) typed_arrays: HashMap<String, RuntimeTypedArray>,
    pub(super) host_actions: Vec<DexHostAction>,
    pub(super) activity_stack: Vec<RuntimeActivity>,
    pub(super) activity_receiver: Option<DexValue>,
    pub(super) application_instance: Option<DexValue>,
    pub(super) content_layouts: Vec<i32>,
    pub(super) inflated_layouts: Vec<i32>,
    pub(super) dialog_content_layouts: Vec<i32>,
    pub(super) view_lookup_ids: Vec<i32>,
}

impl DexRuntimeState {
    // cite: libcore JSONObject.java#put, nameValuePairs
    pub(super) fn insert_json_entry(
        &mut self,
        object_key: String,
        entry_key: String,
        value: DexValue,
    ) {
        let order = self.json_key_order.entry(object_key.clone()).or_default();
        if !self
            .json_entries
            .contains_key(&(object_key.clone(), entry_key.clone()))
        {
            order.push(entry_key.clone());
        }
        self.json_entries.insert((object_key, entry_key), value);
    }

    pub(super) fn json_entries_in_order(&self, object_key: &str) -> Vec<(String, DexValue)> {
        let mut ordered: Vec<(String, DexValue)> = self
            .json_key_order
            .get(object_key)
            .into_iter()
            .flatten()
            .filter_map(|key| {
                self.json_entries
                    .get(&(object_key.to_owned(), key.clone()))
                    .map(|value| (key.clone(), value.clone()))
            })
            .collect();
        let mut unordered: Vec<(String, DexValue)> = self
            .json_entries
            .iter()
            .filter(|((owner, key), _)| {
                owner == object_key && !ordered.iter().any(|(known, _)| known == key)
            })
            .map(|((_, key), value)| (key.clone(), value.clone()))
            .collect();
        unordered.sort_by(|left, right| left.0.cmp(&right.0));
        ordered.extend(unordered);
        ordered
    }

    pub(super) fn allocate_adapter_item_click_id(&mut self) -> i32 {
        let id = self.next_adapter_item_click_id;
        self.next_adapter_item_click_id = if id >= ADAPTER_ITEM_CLICK_ID_MAX {
            ADAPTER_ITEM_CLICK_ID_BASE
        } else {
            id + 1
        };
        id
    }

    pub(super) fn field(&self, owner_key: &str, field_signature: &str) -> Option<&DexValue> {
        match parse_owner(owner_key)? {
            Ok(id) => self.instance_fields.object_field(id, field_signature),
            Err(reference) => self
                .reference_fields
                .reference_field(reference, field_signature),
        }
    }

    pub(super) fn value_field(
        &self,
        object: &DexValue,
        field_signature: &str,
    ) -> Option<&DexValue> {
        match object {
            DexValue::Object(object) => self
                .instance_fields
                .object_field(object.id, field_signature),
            DexValue::Reference(reference) => self
                .reference_fields
                .reference_field(reference, field_signature),
            _ => None,
        }
    }

    pub(super) fn value_field_cached(
        &self,
        object: &DexValue,
        field_signature: &str,
        ic: &mut Option<(u32, u16)>,
    ) -> Option<&DexValue> {
        match object {
            DexValue::Object(object) => {
                self.instance_fields
                    .object_field_cached(object.id, field_signature, ic)
            }
            DexValue::Reference(reference) => self
                .reference_fields
                .reference_field(reference, field_signature),
            _ => None,
        }
    }

    pub(super) fn set_field(&mut self, owner_key: &str, field_signature: &str, value: DexValue) {
        match parse_owner(owner_key) {
            Some(Ok(id)) => {
                let mut ic = None;
                self.instance_fields
                    .set_object_field_cached(id, field_signature, value, &mut ic);
            }
            Some(Err(reference)) => {
                let reference = reference.to_owned();
                self.reference_fields
                    .set_reference_field(&reference, field_signature, value);
            }
            None => {}
        }
    }

    pub(super) fn remove_field(
        &mut self,
        owner_key: &str,
        field_signature: &str,
    ) -> Option<DexValue> {
        match parse_owner(owner_key)? {
            Ok(id) => self
                .instance_fields
                .remove_object_field(id, field_signature),
            Err(reference) => self
                .reference_fields
                .remove_reference_field(reference, field_signature),
        }
    }

    pub(super) fn insert_field(
        &mut self,
        key: (String, String),
        value: DexValue,
    ) -> Option<DexValue> {
        let previous = self.field(&key.0, &key.1).cloned();
        self.set_field(&key.0, &key.1, value);
        previous
    }

    pub(super) fn fields_of(&self, owner_key: &str) -> Option<FieldsView<'_>> {
        match parse_owner(owner_key)? {
            Ok(id) => self.instance_fields.fields_of_object(id),
            Err(reference) => self.reference_fields.fields_of_reference(reference),
        }
    }

    pub(super) fn iter_fields(&self) -> impl Iterator<Item = ((OwnerRef<'_>, &str), &DexValue)> {
        self.iter_field_owners().flat_map(|(owner, fields)| {
            fields
                .iter()
                .map(move |(signature, value)| ((owner, signature), value))
        })
    }

    pub(super) fn iter_field_owners(&self) -> impl Iterator<Item = (OwnerRef<'_>, FieldsView<'_>)> {
        self.instance_fields.iter_objects().chain(
            self.reference_fields
                .iter_references()
                .map(|(reference, fields)| (OwnerRef::Reference(reference), fields)),
        )
    }

    pub(super) fn iter_reference_fields(&self) -> impl Iterator<Item = (&str, FieldsView<'_>)> {
        self.reference_fields.iter_references()
    }

    pub(super) fn new() -> Self {
        Self {
            next_object_id: 1,
            instance_fields: InstanceFieldStore::default(),
            reference_fields: ReferenceFieldStore::default(),
            view_measure_caches: crate::fxhash::FxHashMap::default(),
            static_fields: StaticFieldStore::default(),
            initialized_classes: HashSet::new(),
            initializing_classes: HashMap::new(),
            class_initialization_waiters: HashMap::new(),
            lru_caches: HashMap::new(),
            uri_matchers: HashMap::new(),
            content_providers: HashMap::new(),
            map_entries: HashMap::new(),
            map_entry_keys: HashMap::new(),
            map_entry_objects: HashMap::new(),
            proxy_classes: HashMap::new(),
            proxy_class_keys: HashMap::new(),
            proxy_instances: HashMap::new(),
            next_proxy_index: 0,
            generic_types: HashMap::new(),
            next_generic_type_id: 1,
            list_entries: HashMap::from([
                (
                    "reference:Ljava/util/List;:collections:emptyList".to_owned(),
                    Vec::new(),
                ),
                (
                    "reference:Ljava/util/Set;:collections:emptySet".to_owned(),
                    Vec::new(),
                ),
            ]),
            list_sub_lists: HashMap::new(),
            tree_set_comparators: HashMap::new(),
            iterator_entries: HashMap::new(),
            iterator_positions: HashMap::new(),
            iterator_sources: HashMap::new(),
            iterator_last_returned_indices: HashMap::new(),
            array_entries: HashMap::new(),
            json_entries: HashMap::new(),
            json_key_order: HashMap::new(),
            content_values_entries: HashMap::new(),
            bundle_entries: HashMap::new(),
            sparse_array_entries: HashMap::new(),
            sqlite_database_paths: HashMap::new(),
            sqlite_cursors: HashMap::new(),
            sqlite_bind_values: HashMap::new(),
            sqlite_statement_sql: HashMap::new(),
            sqlite_statement_database: HashMap::new(),
            sqlite_open_helpers_opened: HashSet::new(),
            shared_preferences_entries: HashMap::new(),
            shared_preferences_storage_kinds: HashMap::new(),
            context_storage_kinds: HashMap::new(),
            intent_extras: HashMap::new(),
            intent_extra_types: HashMap::new(),
            adapter_item_click_listeners: HashMap::new(),
            shared_preferences_editors: HashMap::new(),
            string_values: HashMap::new(),
            string_builder_values: HashMap::new(),
            string_writer_values: HashMap::new(),
            number_format_values: HashMap::new(),
            input_stream_values: HashMap::new(),
            zip_files: HashMap::new(),
            sax_attributes: HashMap::new(),
            spannable_spans: HashMap::new(),
            enum_values: HashMap::new(),
            atomic_boolean_values: HashMap::new(),
            atomic_integer_values: HashMap::new(),
            atomic_long_values: HashMap::new(),
            semaphore_permits: HashMap::new(),
            condition_waiters: HashMap::new(),
            atomic_reference_values: HashMap::new(),
            weak_reference_values: HashMap::new(),
            handler_loopers: HashMap::new(),
            shutdown_executors: HashSet::new(),
            current_guest_thread: DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main")),
            guest_thread_ids: HashMap::new(),
            thread_priorities: HashMap::new(),
            finished_activities: HashSet::new(),
            pending_destroyed_activities: Vec::new(),
            destroyed_activities: HashSet::new(),
            idle_handlers: Vec::new(),
            queue_capacities: HashMap::new(),
            scrollers: HashMap::new(),
            executor_handlers: HashMap::new(),
            executor_workers: HashMap::new(),
            next_thread_pool_number: 1,
            next_thread_number: 0,
            idle_handlers_ran: false,
            default_uncaught_exception_handler: None,
            boot_wall_clock_millis: current_wall_clock_millis(),
            main_thread_uptime_millis: 0,
            process_start_uptime_millis: 0,
            next_frame_time_nanos: super::FRAME_INTERVAL_NANOS,
            frame_time_taken_since_drive: true,
            traversal_dispatched_since_drive: false,
            frame_interval_nanos: super::FRAME_INTERVAL_NANOS,
            next_main_thread_sequence: 1,
            main_thread_tasks: Vec::new(),
            handler_posts: Vec::new(),
            choreographer_frame_callbacks: Vec::new(),
            choreographer_delayed_frame_callbacks: Vec::new(),
            choreographer_runnable_callbacks: Vec::new(),
            parked_threads: HashMap::new(),
            next_parked_runtime_action_id: 1,
            parked_runtime_actions: HashMap::new(),
            thread_park_permits: HashSet::new(),
            parked_thread_values: HashMap::new(),
            parked_thread_deadlines: HashMap::new(),
            thread_resumes: Vec::new(),
            random_seeds: HashMap::new(),
            string_reader_values: HashMap::new(),
            regex_patterns: HashMap::new(),
            regex_matchers: HashMap::new(),
            bootstrap_field_owners: HashMap::new(),
            unsafe_field_offsets: HashMap::new(),
            accessibility_state_change_listeners: HashMap::new(),
            accessibility_touch_exploration_state_change_listeners: HashMap::new(),
            matrix_values: HashMap::new(),
            big_decimal_values: HashMap::new(),
            big_integer_values: HashMap::new(),
            math_context_values: HashMap::new(),
            intent_filter_actions: HashMap::new(),
            component_callbacks: HashMap::new(),
            activity_lifecycle_callbacks: HashMap::new(),
            view_tags: HashMap::new(),
            annotation_elements: HashMap::new(),
            back_invoked_callbacks: HashMap::new(),
            view_click_listeners: HashMap::new(),
            view_text_watchers: HashMap::new(),
            view_attach_state_listeners: HashMap::new(),
            view_tree_global_layout_listeners: HashMap::new(),
            view_tree_draw_listeners: HashMap::new(),
            view_tree_pre_draw_listeners: HashMap::new(),
            view_tree_scroll_changed_listeners: HashMap::new(),
            view_tree_window_focus_change_listeners: HashMap::new(),
            view_tree_touch_mode_change_listeners: HashMap::new(),
            bitmap_encoded_images: HashMap::new(),
            canvas_draw_ops: HashMap::new(),
            active_canvas_key: None,
            render_node_draw_ops: HashMap::new(),
            render_node_generations: HashMap::new(),
            view_render_nodes: HashMap::new(),
            view_render_node_dirty: HashSet::new(),
            view_render_node_generations: HashMap::new(),
            root_render_nodes: HashMap::new(),
            root_render_node_generations: HashMap::new(),
            runtime_canvas_frames: Vec::new(),
            window_visible_frame: None,
            path_ops: HashMap::new(),
            view_declared_on_click_handlers: HashMap::new(),
            view_group_children: HashMap::new(),
            view_parents: HashMap::new(),
            adapter_bound_view: HashMap::new(),
            adapter_rows: HashMap::new(),
            adapter_item_clicks: HashMap::new(),
            next_adapter_item_click_id: ADAPTER_ITEM_CLICK_ID_BASE,
            dynamic_view_click_targets: HashMap::new(),
            focused_view: None,
            options_menu: Vec::new(),
            inflated_preferences: Vec::new(),
            pending_fragment: None,
            action_bar_title: None,
            action_bar_nav_icon: None,
            action_bar_nav_click: None,
            action_bar_overflow_open: false,
            action_bar_present: false,
            action_bar_home_as_up: false,
            runtime_view_roots: Vec::new(),
            traversal_request: TraversalRequest::default(),
            inflated_view_children: HashMap::new(),
            layout_root_descriptors: HashMap::new(),
            decor_action_bar: None,
            xml_parsers: HashMap::new(),
            menus: HashMap::new(),
            active_menu_overlays: Vec::new(),
            active_dialog_overlays: Vec::new(),
            themes: HashMap::new(),
            typed_arrays: HashMap::new(),
            host_actions: Vec::new(),
            activity_stack: Vec::new(),
            activity_receiver: None,
            application_instance: None,
            content_layouts: Vec::new(),
            inflated_layouts: Vec::new(),
            dialog_content_layouts: Vec::new(),
            view_lookup_ids: Vec::new(),
        }
    }

    pub(super) fn take_main_thread_work_with_limit(
        &mut self,
        include_frame_callbacks: bool,
        max_work_items: Option<usize>,
    ) -> MainThreadWork {
        self.resume_expired_parked_threads();
        let mut remaining = max_work_items.unwrap_or(usize::MAX);
        let mut immediate_handler_posts = Vec::new();
        let mut delayed_handler_posts = Vec::new();

        for post in self.handler_posts.drain(..) {
            if post.due_time_millis <= self.main_thread_uptime_millis {
                immediate_handler_posts.push(post);
            } else {
                delayed_handler_posts.push(post);
            }
        }

        self.handler_posts = delayed_handler_posts;
        immediate_handler_posts.sort_by(handler_post_dispatch_order);

        let tasks = take_limited(&mut self.main_thread_tasks, remaining);
        remaining = remaining.saturating_sub(tasks.len());
        let handler_posts = take_limited(&mut immediate_handler_posts, remaining);
        remaining = remaining.saturating_sub(handler_posts.len());
        immediate_handler_posts.extend(std::mem::take(&mut self.handler_posts));
        self.handler_posts = immediate_handler_posts;
        let thread_resumes = take_limited(&mut self.thread_resumes, remaining);
        remaining = remaining.saturating_sub(thread_resumes.len());
        let choreographer_callbacks = if include_frame_callbacks
            && tasks.is_empty()
            && handler_posts.is_empty()
            && thread_resumes.is_empty()
        {
            self.take_due_choreographer_runnable_callbacks(remaining)
        } else {
            Vec::new()
        };
        remaining = remaining.saturating_sub(choreographer_callbacks.len());
        let delayed_frame_callbacks = if include_frame_callbacks
            && tasks.is_empty()
            && handler_posts.is_empty()
            && thread_resumes.is_empty()
        {
            self.take_due_choreographer_frame_callbacks(remaining)
        } else {
            Vec::new()
        };
        remaining = remaining.saturating_sub(delayed_frame_callbacks.len());
        let frame_callbacks = if include_frame_callbacks
            && tasks.is_empty()
            && handler_posts.is_empty()
            && thread_resumes.is_empty()
        {
            let mut callbacks = delayed_frame_callbacks;
            callbacks.extend(take_limited(
                &mut self.choreographer_frame_callbacks,
                remaining,
            ));
            callbacks
        } else {
            Vec::new()
        };

        MainThreadWork {
            tasks,
            handler_posts,
            choreographer_callbacks,
            frame_callbacks,
            thread_resumes,
        }
    }

    fn take_due_choreographer_runnable_callbacks(
        &mut self,
        max_items: usize,
    ) -> Vec<ChoreographerRunnableCallback> {
        if max_items == 0 {
            return Vec::new();
        }

        let mut due = Vec::new();
        let mut pending = Vec::new();
        for callback in self.choreographer_runnable_callbacks.drain(..) {
            if callback.due_time_millis <= self.main_thread_uptime_millis {
                due.push(callback);
            } else {
                pending.push(callback);
            }
        }
        due.sort_by(choreographer_runnable_callback_dispatch_order);
        let mut selected = take_limited(&mut due, max_items);
        due.extend(pending);
        self.choreographer_runnable_callbacks = due;
        selected.shrink_to_fit();
        selected
    }

    fn take_due_choreographer_frame_callbacks(&mut self, max_items: usize) -> Vec<DexValue> {
        if max_items == 0 {
            return Vec::new();
        }

        let mut due = Vec::new();
        let mut pending = Vec::new();
        for callback in self.choreographer_delayed_frame_callbacks.drain(..) {
            if callback.due_time_millis <= self.main_thread_uptime_millis {
                due.push(callback);
            } else {
                pending.push(callback);
            }
        }
        due.sort_by(choreographer_frame_callback_dispatch_order);
        let selected = take_limited(&mut due, max_items)
            .into_iter()
            .map(|callback| callback.callback)
            .collect();
        due.extend(pending);
        self.choreographer_delayed_frame_callbacks = due;
        selected
    }

    pub(super) fn take_frame_time_nanos(&mut self) -> i64 {
        let frame_time_nanos = self.next_frame_time_nanos;
        self.next_frame_time_nanos = self
            .next_frame_time_nanos
            .saturating_add(self.frame_interval_nanos);
        self.frame_time_taken_since_drive = true;
        frame_time_nanos
    }

    pub(super) fn begin_driven_frame(&mut self, skipped_intervals: i64) {
        self.advance_frame_clock_by_skipped_intervals(skipped_intervals);
        self.frame_time_taken_since_drive = false;
        self.traversal_dispatched_since_drive = false;
    }

    // cite: frameworks-base SystemClock.java#uptimeMillis
    pub(super) fn finish_driven_frame(&mut self) {
        if self.frame_time_taken_since_drive {
            return;
        }
        let frame_time_nanos = self.take_frame_time_nanos();
        self.advance_main_thread_uptime_to_frame(frame_time_nanos);
    }

    pub(super) fn has_delayed_handler_posts(&self) -> bool {
        !self.handler_posts.is_empty()
            || !self.parked_thread_deadlines.is_empty()
            || !self.choreographer_delayed_frame_callbacks.is_empty()
            || !self.choreographer_runnable_callbacks.is_empty()
    }

    #[cfg(test)]
    pub(super) fn frame_interval_nanos(&self) -> i64 {
        self.frame_interval_nanos
    }

    pub(super) fn frame_interval_millis_ceil(&self) -> i64 {
        (self.frame_interval_nanos + 999_999) / 1_000_000
    }

    pub(super) fn set_frame_interval_nanos(&mut self, frame_interval_nanos: i64) {
        assert!(
            frame_interval_nanos > 0,
            "frame interval must be positive, got {frame_interval_nanos}"
        );
        if self.next_frame_time_nanos == self.frame_interval_nanos {
            self.next_frame_time_nanos = frame_interval_nanos;
        }
        self.frame_interval_nanos = frame_interval_nanos;
    }

    pub(super) fn advance_frame_clock_by_skipped_intervals(&mut self, skipped_intervals: i64) {
        if skipped_intervals <= 0 {
            return;
        }
        let advance = skipped_intervals.saturating_mul(self.frame_interval_nanos);
        self.next_frame_time_nanos = self.next_frame_time_nanos.saturating_add(advance);
    }

    pub(super) fn current_wall_clock_millis(&self) -> i64 {
        self.boot_wall_clock_millis
            .saturating_add(self.main_thread_uptime_millis)
    }

    pub(super) fn current_monotonic_time_nanos(&self) -> i64 {
        let uptime_nanos = self.main_thread_uptime_millis.saturating_mul(1_000_000);
        let last_frame_time_nanos = self
            .next_frame_time_nanos
            .saturating_sub(self.frame_interval_nanos);

        uptime_nanos.max(last_frame_time_nanos)
    }

    pub(super) fn advance_main_thread_uptime_to_frame(&mut self, frame_time_nanos: i64) {
        let frame_time_millis = frame_time_nanos.div_euclid(1_000_000);
        if frame_time_millis > self.main_thread_uptime_millis {
            self.main_thread_uptime_millis = frame_time_millis;
        }
    }

    pub(super) fn advance_main_thread_uptime_to_next_timed_event(
        &mut self,
        max_wait_millis: i64,
    ) -> bool {
        let next_post_due_millis = self
            .handler_posts
            .iter()
            .map(|post| post.due_time_millis)
            .min();
        let next_park_deadline_millis = self.parked_thread_deadlines.values().copied().min();
        let next_choreographer_frame_millis = self
            .choreographer_delayed_frame_callbacks
            .iter()
            .map(|callback| callback.due_time_millis)
            .min();
        let next_choreographer_runnable_millis = self
            .choreographer_runnable_callbacks
            .iter()
            .map(|callback| callback.due_time_millis)
            .min();
        let next_due_time_millis = [
            next_post_due_millis,
            next_park_deadline_millis,
            next_choreographer_frame_millis,
            next_choreographer_runnable_millis,
        ]
        .into_iter()
        .flatten()
        .min();
        let Some(next_due_time_millis) = next_due_time_millis else {
            return false;
        };

        if next_due_time_millis <= self.main_thread_uptime_millis {
            return false;
        }

        if next_due_time_millis - self.main_thread_uptime_millis > max_wait_millis {
            return false;
        }

        self.main_thread_uptime_millis = next_due_time_millis;
        true
    }

    pub(super) fn main_thread_is_idle(&self) -> bool {
        !self.has_pending_main_work()
    }

    pub(super) fn register_parked_runtime_action(&mut self, action: ParkedRuntimeAction) -> u64 {
        let action_id = self.next_parked_runtime_action_id;
        self.next_parked_runtime_action_id = self.next_parked_runtime_action_id.wrapping_add(1);
        self.parked_runtime_actions.insert(action_id, action);
        action_id
    }

    pub(super) fn take_parked_runtime_action(
        &mut self,
        action_id: u64,
    ) -> Option<ParkedRuntimeAction> {
        self.parked_runtime_actions.remove(&action_id)
    }

    pub(super) fn complete_class_initialization(&mut self, class_descriptor: &str) {
        self.initializing_classes.remove(class_descriptor);
        self.initialized_classes.insert(class_descriptor.to_owned());
        self.resume_class_initialization_waiters(class_descriptor);
    }

    pub(super) fn abort_class_initialization(&mut self, class_descriptor: &str) {
        self.initializing_classes.remove(class_descriptor);
        self.resume_class_initialization_waiters(class_descriptor);
    }

    fn resume_class_initialization_waiters(&mut self, class_descriptor: &str) {
        let Some(waiters) = self.class_initialization_waiters.remove(class_descriptor) else {
            return;
        };
        self.thread_resumes
            .extend(waiters.into_iter().map(|waiter| ThreadResume {
                thread: waiter.thread,
                execution: waiter.execution,
            }));
    }

    pub(super) fn discard_parked_runtime_actions(&mut self, execution: &DexParkedExecution) {
        for action_id in execution.runtime_action_ids() {
            let Some(action) = self.parked_runtime_actions.remove(&action_id) else {
                continue;
            };
            match action {
                ParkedRuntimeAction::ContinueClassInitialization(continuation) => {
                    self.abort_class_initialization(&continuation.class_descriptor);
                }
                ParkedRuntimeAction::FinishClassInitialization(class_descriptor) => {
                    self.abort_class_initialization(&class_descriptor);
                }
                ParkedRuntimeAction::InvokeExactVoid { .. }
                | ParkedRuntimeAction::InvokeBuiltinVoid { .. } => {}
            }
        }
    }

    // cite: libcore LockSupport.java#park, unpark
    pub(super) fn store_parked_thread(
        &mut self,
        thread: &DexValue,
        execution: DexParkedExecution,
    ) -> Option<()> {
        if let Some(class_descriptor) = execution.waiting_class_descriptor().map(str::to_owned) {
            self.class_initialization_waiters
                .entry(class_descriptor)
                .or_default()
                .push(ClassInitializationWaiter {
                    thread: thread.clone(),
                    execution,
                });
            return Some(());
        }
        let key = thread_storage_key(thread)?;
        if self.thread_park_permits.remove(&key) {
            self.thread_resumes.push(ThreadResume {
                thread: thread.clone(),
                execution,
            });
            return Some(());
        }
        // cite: libcore LockSupport.java#parkNanos
        if let Some(nanos) = execution.park_timeout_nanos() {
            let deadline = self
                .main_thread_uptime_millis
                .saturating_add(nanos.max(0) / 1_000_000);
            self.parked_thread_deadlines.insert(key.clone(), deadline);
        }
        self.parked_thread_values
            .insert(key.clone(), thread.clone());
        self.parked_threads.insert(key, execution);
        Some(())
    }

    pub(super) fn resume_expired_parked_threads(&mut self) {
        let expired: Vec<String> = self
            .parked_thread_deadlines
            .iter()
            .filter(|(_, deadline)| **deadline <= self.main_thread_uptime_millis)
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            self.parked_thread_deadlines.remove(&key);
            let Some(execution) = self.parked_threads.remove(&key) else {
                continue;
            };
            let Some(thread) = self.parked_thread_values.remove(&key) else {
                continue;
            };
            self.thread_resumes.push(ThreadResume { thread, execution });
        }
    }

    // cite: libcore LockSupport.java#unpark
    pub(super) fn unpark_thread(&mut self, thread: &DexValue) -> Option<bool> {
        let key = thread_storage_key(thread)?;
        let Some(execution) = self.parked_threads.remove(&key) else {
            self.thread_park_permits.insert(key);
            return Some(false);
        };
        self.parked_thread_deadlines.remove(&key);
        self.parked_thread_values.remove(&key);

        self.thread_resumes.push(ThreadResume {
            thread: thread.clone(),
            execution,
        });
        Some(true)
    }

    pub(super) fn post_main_thread_task(&mut self, task: MainThreadTask) {
        self.main_thread_tasks.push(task);
    }

    pub(super) fn take_start_thread_tasks(&mut self) -> Vec<DexValue> {
        let (starts, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.main_thread_tasks)
            .into_iter()
            .partition(|task| matches!(task, MainThreadTask::StartThread { .. }));
        self.main_thread_tasks = rest;

        starts
            .into_iter()
            .filter_map(|task| match task {
                MainThreadTask::StartThread { thread } => Some(thread),
                _ => None,
            })
            .collect()
    }

    pub(super) fn post_handler_runnable(
        &mut self,
        handler_key: String,
        runnable: DexValue,
        delay_millis: i64,
        at_front: bool,
    ) {
        self.enqueue_handler_runnable(handler_key, None, runnable, delay_millis, at_front);
    }

    pub(super) fn post_handler_callback(
        &mut self,
        handler: DexValue,
        runnable: DexValue,
        delay_millis: i64,
        at_front: bool,
    ) {
        self.enqueue_handler_runnable(
            map_identity_key(&handler),
            Some(handler),
            runnable,
            delay_millis,
            at_front,
        );
    }

    fn enqueue_handler_runnable(
        &mut self,
        handler_key: String,
        handler: Option<DexValue>,
        runnable: DexValue,
        delay_millis: i64,
        at_front: bool,
    ) {
        let sequence = self.next_main_thread_sequence;
        self.next_main_thread_sequence += 1;
        let due_time_millis = self.main_thread_uptime_millis + delay_millis.max(0);
        let post = HandlerPost {
            handler_key,
            handler,
            runnable,
            delay_millis,
            due_time_millis,
            sequence,
            at_front,
        };
        if at_front {
            self.handler_posts.insert(0, post);
        } else {
            self.handler_posts.push(post);
        }
    }

    pub(super) fn post_choreographer_frame_callback(
        &mut self,
        callback: DexValue,
        delay_millis: i64,
    ) {
        if delay_millis <= 0 {
            self.choreographer_frame_callbacks.push(callback);
            return;
        }

        let sequence = self.next_main_thread_sequence;
        self.next_main_thread_sequence += 1;
        self.choreographer_delayed_frame_callbacks
            .push(ChoreographerFrameCallback {
                callback,
                due_time_millis: self.main_thread_uptime_millis + delay_millis,
                sequence,
            });
    }

    pub(super) fn remove_choreographer_frame_callback(&mut self, callback: &DexValue) {
        let callback_key = map_identity_key(callback);
        self.choreographer_frame_callbacks
            .retain(|registered| map_identity_key(registered) != callback_key);
        self.choreographer_delayed_frame_callbacks
            .retain(|registered| map_identity_key(&registered.callback) != callback_key);
    }

    pub(super) fn post_choreographer_runnable_callback(
        &mut self,
        callback_type: i32,
        action: DexValue,
        token: DexValue,
        delay_millis: i64,
    ) {
        let sequence = self.next_main_thread_sequence;
        self.next_main_thread_sequence += 1;
        self.choreographer_runnable_callbacks
            .push(ChoreographerRunnableCallback {
                callback_type,
                action,
                token,
                due_time_millis: self.main_thread_uptime_millis + delay_millis.max(0),
                sequence,
            });
    }

    pub(super) fn remove_choreographer_runnable_callbacks(
        &mut self,
        callback_type: i32,
        action: &DexValue,
        token: &DexValue,
    ) {
        let action_key = runtime_value_is_object_like(action).then(|| map_identity_key(action));
        let token_key = runtime_value_is_object_like(token).then(|| map_identity_key(token));
        self.choreographer_runnable_callbacks.retain(|callback| {
            if callback.callback_type != callback_type {
                return true;
            }
            let action_matches = match action_key.as_ref() {
                Some(key) => map_identity_key(&callback.action) == *key,
                None => true,
            };
            let token_matches = match token_key.as_ref() {
                Some(key) => map_identity_key(&callback.token) == *key,
                None => true,
            };
            !(action_matches && token_matches)
        });
    }

    pub(super) fn request_layout_traversal(&mut self) {
        self.traversal_request.layout_requested = true;
        self.traversal_request.draw_requested = true;
    }

    pub(super) fn request_draw_traversal(&mut self) {
        self.traversal_request.draw_requested = true;
    }

    pub(super) fn has_traversal_request(&self) -> bool {
        self.traversal_request.layout_requested || self.traversal_request.draw_requested
    }

    pub(super) fn has_dispatchable_traversal_request(&self) -> bool {
        self.has_traversal_request() && !self.traversal_dispatched_since_drive
    }

    // cite: frameworks-base ViewRootImpl.java#scheduleTraversals
    // cite: androidx AndroidUiDispatcher.android.kt#postFrameCallback
    pub(super) fn next_wake_delay_millis(&self) -> Option<i64> {
        if self.has_traversal_request()
            || !self.main_thread_tasks.is_empty()
            || !self.thread_resumes.is_empty()
            || !self.choreographer_frame_callbacks.is_empty()
            || self.has_due_choreographer_frame_work()
        {
            return Some(0);
        }

        let frame_interval = self.frame_interval_millis_ceil();
        let now = self.main_thread_uptime_millis;
        let mut earliest: Option<i64> = None;
        for post in &self.handler_posts {
            let delay = post.due_time_millis.saturating_sub(now).max(0);
            earliest = Some(earliest.map_or(delay, |current| current.min(delay)));
        }
        for deadline in self.parked_thread_deadlines.values() {
            let delay = deadline.saturating_sub(now).max(0);
            earliest = Some(earliest.map_or(delay, |current| current.min(delay)));
        }
        for callback in &self.choreographer_delayed_frame_callbacks {
            let delay = callback.due_time_millis.saturating_sub(now).max(0);
            earliest = Some(earliest.map_or(delay, |current| current.min(delay)));
        }
        for callback in &self.choreographer_runnable_callbacks {
            let delay = callback.due_time_millis.saturating_sub(now).max(0);
            earliest = Some(earliest.map_or(delay, |current| current.min(delay)));
        }

        match earliest {
            Some(delay) if delay <= frame_interval => Some(0),
            Some(delay) => Some(delay),
            None => None,
        }
    }

    fn has_due_choreographer_frame_work(&self) -> bool {
        self.choreographer_delayed_frame_callbacks
            .iter()
            .any(|callback| callback.due_time_millis <= self.main_thread_uptime_millis)
            || self
                .choreographer_runnable_callbacks
                .iter()
                .any(|callback| callback.due_time_millis <= self.main_thread_uptime_millis)
    }

    pub(super) fn has_pending_non_frame_main_work(&self) -> bool {
        let horizon = self
            .main_thread_uptime_millis
            .saturating_add(self.frame_interval_millis_ceil());
        self.has_traversal_request()
            || !self.main_thread_tasks.is_empty()
            || !self.thread_resumes.is_empty()
            || self
                .handler_posts
                .iter()
                .any(|post| post.due_time_millis <= horizon)
            || self
                .parked_thread_deadlines
                .values()
                .any(|deadline| *deadline <= horizon)
    }

    pub(super) fn has_pending_main_work(&self) -> bool {
        self.has_pending_non_frame_main_work()
            || !self.choreographer_frame_callbacks.is_empty()
            || self.has_due_choreographer_frame_work()
    }

    pub(super) fn take_traversal_request(&mut self) -> Option<TraversalRequest> {
        if self.traversal_dispatched_since_drive {
            return None;
        }

        let request = std::mem::take(&mut self.traversal_request);
        if !request.layout_requested && !request.draw_requested {
            return None;
        }

        self.traversal_dispatched_since_drive = true;
        Some(request)
    }
}

fn take_limited<T>(items: &mut Vec<T>, max_items: usize) -> Vec<T> {
    if max_items == 0 {
        return Vec::new();
    }
    if items.len() <= max_items {
        return std::mem::take(items);
    }

    let remaining = items.split_off(max_items);
    std::mem::replace(items, remaining)
}

fn current_wall_clock_millis() -> i64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };

    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct RuntimeTheme {
    pub(super) styles: Vec<RuntimeThemeStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeThemeStyle {
    pub(super) resource_id: u32,
    pub(super) force: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct RuntimeTypedArray {
    pub(super) values: HashMap<i32, ResourcePrimitive>,
}

#[derive(Debug, Default)]
pub(super) struct InstanceFieldStore {
    objects: Vec<Option<ObjectData>>,
    class_indices: crate::fxhash::FxHashMap<String, u32>,
    defined_class_layouts: crate::fxhash::FxHashSet<String>,
    layouts: Vec<FieldLayout>,
}

#[derive(Debug, Default)]
pub(super) struct ReferenceFieldStore {
    fields: crate::fxhash::FxHashMap<String, crate::fxhash::FxHashMap<String, DexValue>>,
}

#[derive(Debug, Default)]
pub(super) struct FieldLayout {
    slots: crate::fxhash::FxHashMap<String, u16>,
    signatures: Vec<String>,
}

#[derive(Debug)]
struct ObjectData {
    class_descriptor: Option<String>,
    storage: FieldStorage,
}

#[derive(Debug)]
enum FieldStorage {
    Slots {
        layout: u32,
        values: Vec<Option<DexValue>>,
    },
    Map(crate::fxhash::FxHashMap<String, DexValue>),
}

// cite: art object-inl.h#GetField32, SetField32, GetFieldObject, SetFieldObject
#[derive(Clone, Copy)]
pub(super) enum OwnerRef<'a> {
    Object(u64),
    Reference(&'a str),
}

impl std::fmt::Display for OwnerRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnerRef::Object(id) => write!(f, "object:{id}"),
            OwnerRef::Reference(value) => write!(f, "reference:{value}"),
        }
    }
}

impl OwnerRef<'_> {
    pub(super) fn matches_key(&self, key: &str) -> bool {
        match self {
            OwnerRef::Object(id) => key
                .strip_prefix("object:")
                .is_some_and(|rest| rest.parse() == Ok(*id)),
            OwnerRef::Reference(value) => key
                .strip_prefix("reference:")
                .is_some_and(|rest| rest == *value),
        }
    }

    pub(super) fn is_in_active_object_scope(&self, min_object_id: u64) -> bool {
        match self {
            OwnerRef::Object(id) => *id >= min_object_id,
            OwnerRef::Reference(_) => true,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct FieldsView<'a> {
    kind: FieldsViewKind<'a>,
}

#[derive(Clone, Copy)]
enum FieldsViewKind<'a> {
    Object {
        data: &'a ObjectData,
        layouts: &'a [FieldLayout],
    },
    Map(&'a crate::fxhash::FxHashMap<String, DexValue>),
}

pub(super) enum FieldsIter<'a> {
    Slots {
        layout: &'a FieldLayout,
        values: std::iter::Enumerate<std::slice::Iter<'a, Option<DexValue>>>,
    },
    Map(std::collections::hash_map::Iter<'a, String, DexValue>),
}

impl<'a> Iterator for FieldsIter<'a> {
    type Item = (&'a str, &'a DexValue);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FieldsIter::Slots { layout, values } => loop {
                let (slot, value) = values.next()?;
                let Some(value) = value.as_ref() else {
                    continue;
                };
                let signature = layout.signatures.get(slot)?;
                return Some((signature.as_str(), value));
            },
            FieldsIter::Map(iter) => iter.next().map(|(key, value)| (key.as_str(), value)),
        }
    }
}

impl<'a> FieldsView<'a> {
    fn object(data: &'a ObjectData, layouts: &'a [FieldLayout]) -> Self {
        Self {
            kind: FieldsViewKind::Object { data, layouts },
        }
    }

    fn map(fields: &'a crate::fxhash::FxHashMap<String, DexValue>) -> Self {
        Self {
            kind: FieldsViewKind::Map(fields),
        }
    }

    pub(super) fn layout_index(&self) -> Option<u32> {
        let FieldsViewKind::Object { data, .. } = self.kind else {
            return None;
        };
        match &data.storage {
            FieldStorage::Slots { layout, .. } => Some(*layout),
            FieldStorage::Map(_) => None,
        }
    }

    pub(super) fn slot_of(&self, field_signature: &str) -> Option<u16> {
        let FieldsViewKind::Object { data, layouts } = self.kind else {
            return None;
        };
        match &data.storage {
            FieldStorage::Slots { layout, .. } => layouts[*layout as usize]
                .slots
                .get(field_signature)
                .copied(),
            FieldStorage::Map(_) => None,
        }
    }

    pub(super) fn get_slot(&self, slot: u16) -> Option<&'a DexValue> {
        let FieldsViewKind::Object { data, .. } = self.kind else {
            return None;
        };
        match &data.storage {
            FieldStorage::Slots { values, .. } => values.get(slot as usize)?.as_ref(),
            FieldStorage::Map(_) => None,
        }
    }

    pub(super) fn get(&self, field_signature: &str) -> Option<&'a DexValue> {
        match self.kind {
            FieldsViewKind::Object { data, layouts } => match &data.storage {
                FieldStorage::Slots { layout, values } => {
                    let slot = *layouts[*layout as usize].slots.get(field_signature)?;
                    values.get(slot as usize)?.as_ref()
                }
                FieldStorage::Map(map) => map.get(field_signature),
            },
            FieldsViewKind::Map(fields) => fields.get(field_signature),
        }
    }

    pub(super) fn iter(&self) -> FieldsIter<'a> {
        match self.kind {
            FieldsViewKind::Object { data, layouts } => match &data.storage {
                FieldStorage::Slots { layout, values } => FieldsIter::Slots {
                    layout: &layouts[*layout as usize],
                    values: values.iter().enumerate(),
                },
                FieldStorage::Map(map) => FieldsIter::Map(map.iter()),
            },
            FieldsViewKind::Map(fields) => FieldsIter::Map(fields.iter()),
        }
    }

    pub(super) fn values(&self) -> impl Iterator<Item = &'a DexValue> + 'a {
        self.iter().map(|(_, value)| value)
    }
}

fn parse_owner(key: &str) -> Option<Result<u64, &str>> {
    if let Some(id) = key.strip_prefix("object:") {
        return id.parse().ok().map(Ok);
    }
    key.strip_prefix("reference:").map(Err)
}

impl ReferenceFieldStore {
    pub(super) fn reference_field(
        &self,
        reference: &str,
        field_signature: &str,
    ) -> Option<&DexValue> {
        self.fields.get(reference)?.get(field_signature)
    }

    pub(super) fn set_reference_field(
        &mut self,
        reference: &str,
        field_signature: &str,
        value: DexValue,
    ) {
        self.fields
            .entry(reference.to_owned())
            .or_default()
            .insert(field_signature.to_owned(), value);
    }

    pub(super) fn remove_reference_field(
        &mut self,
        reference: &str,
        field_signature: &str,
    ) -> Option<DexValue> {
        self.fields.get_mut(reference)?.remove(field_signature)
    }

    pub(super) fn fields_of_reference(&self, reference: &str) -> Option<FieldsView<'_>> {
        self.fields.get(reference).map(FieldsView::map)
    }

    pub(super) fn iter_references(&self) -> impl Iterator<Item = (&str, FieldsView<'_>)> {
        self.fields
            .iter()
            .map(|(reference, fields)| (reference.as_str(), FieldsView::map(fields)))
    }
}

impl InstanceFieldStore {
    fn layout_index(&mut self, class_descriptor: &str) -> u32 {
        if let Some(index) = self.class_indices.get(class_descriptor) {
            return *index;
        }
        let index = self.layouts.len() as u32;
        self.layouts.push(FieldLayout::default());
        self.class_indices
            .insert(class_descriptor.to_owned(), index);
        index
    }

    fn slot_for(&mut self, layout: u32, field_signature: &str) -> u16 {
        let layout = &mut self.layouts[layout as usize];
        if let Some(slot) = layout.slots.get(field_signature) {
            return *slot;
        }
        let slot = layout.signatures.len() as u16;
        layout.signatures.push(field_signature.to_owned());
        layout.slots.insert(field_signature.to_owned(), slot);
        slot
    }

    fn object_data(&self, id: u64) -> Option<&ObjectData> {
        self.objects.get(id as usize)?.as_ref()
    }

    pub(super) fn object_class_descriptor(&self, id: u64) -> Option<&str> {
        self.object_data(id)?.class_descriptor.as_deref()
    }

    pub(super) fn class_layout_defined(&self, class_descriptor: &str) -> bool {
        self.defined_class_layouts.contains(class_descriptor)
    }

    pub(super) fn define_class_layout(
        &mut self,
        class_descriptor: &str,
        field_signatures: &[Rc<str>],
    ) {
        let layout = self.layout_index(class_descriptor);
        for signature in field_signatures {
            self.slot_for(layout, signature);
        }
        self.defined_class_layouts
            .insert(class_descriptor.to_owned());
    }

    pub(super) fn class_field_slot(
        &self,
        class_descriptor: &str,
        field_signature: &str,
    ) -> Option<(u32, u16)> {
        let layout = *self.class_indices.get(class_descriptor)?;
        let slot = *self.layouts[layout as usize].slots.get(field_signature)?;
        Some((layout, slot))
    }

    pub(super) fn register_object_id(&mut self, id: u64, class_descriptor: String) {
        let layout = self.layout_index(&class_descriptor);
        let index = id as usize;
        if self.objects.len() <= index {
            self.objects.resize_with(index + 1, || None);
        }
        match &mut self.objects[index] {
            Some(data) => data.class_descriptor = Some(class_descriptor),
            slot @ None => {
                *slot = Some(ObjectData {
                    class_descriptor: Some(class_descriptor),
                    storage: FieldStorage::Slots {
                        layout,
                        values: Vec::new(),
                    },
                });
            }
        }
    }

    pub(super) fn object_field(&self, id: u64, field_signature: &str) -> Option<&DexValue> {
        let data = self.object_data(id)?;
        match &data.storage {
            FieldStorage::Slots { layout, values } => {
                let slot = *self.layouts[*layout as usize].slots.get(field_signature)?;
                values.get(slot as usize)?.as_ref()
            }
            FieldStorage::Map(map) => map.get(field_signature),
        }
    }

    pub(super) fn object_field_slot(&self, id: u64, slot: u16) -> Option<&DexValue> {
        let data = self.object_data(id)?;
        match &data.storage {
            FieldStorage::Slots { values, .. } => values.get(slot as usize)?.as_ref(),
            FieldStorage::Map(_) => None,
        }
    }

    pub(super) fn object_field_cached(
        &self,
        id: u64,
        field_signature: &str,
        ic: &mut Option<(u32, u16)>,
    ) -> Option<&DexValue> {
        let data = self.object_data(id)?;
        match &data.storage {
            FieldStorage::Slots { layout, values } => {
                if let Some((ic_layout, ic_slot)) = ic {
                    if *ic_layout == *layout {
                        return values.get(*ic_slot as usize)?.as_ref();
                    }
                }
                let slot = *self.layouts[*layout as usize].slots.get(field_signature)?;
                *ic = Some((*layout, slot));
                values.get(slot as usize)?.as_ref()
            }
            FieldStorage::Map(map) => map.get(field_signature),
        }
    }

    pub(super) fn set_object_field_cached(
        &mut self,
        id: u64,
        field_signature: &str,
        value: DexValue,
        ic: &mut Option<(u32, u16)>,
    ) {
        let index = id as usize;
        if self.objects.len() <= index {
            self.objects.resize_with(index + 1, || None);
        }
        let assigned = match &self.objects[index] {
            Some(ObjectData {
                storage: FieldStorage::Slots { layout, .. },
                ..
            }) => {
                let layout = *layout;
                let slot = match ic {
                    Some((ic_layout, ic_slot)) if *ic_layout == layout => *ic_slot,
                    _ => {
                        let slot = self.slot_for(layout, field_signature);
                        *ic = Some((layout, slot));
                        slot
                    }
                };
                Some(slot)
            }
            _ => None,
        };
        match (&mut self.objects[index], assigned) {
            (
                Some(ObjectData {
                    storage: FieldStorage::Slots { values, .. },
                    ..
                }),
                Some(slot),
            ) => {
                let slot = slot as usize;
                if values.len() <= slot {
                    values.resize_with(slot + 1, || None);
                }
                values[slot] = Some(value);
            }
            (
                Some(ObjectData {
                    storage: FieldStorage::Map(map),
                    ..
                }),
                _,
            ) => {
                map.insert(field_signature.to_owned(), value);
            }
            (slot @ None, _) => {
                let mut map = crate::fxhash::FxHashMap::default();
                map.insert(field_signature.to_owned(), value);
                *slot = Some(ObjectData {
                    class_descriptor: None,
                    storage: FieldStorage::Map(map),
                });
            }
            (Some(_), _) => {}
        }
    }

    pub(super) fn set_object_field_slot(&mut self, id: u64, slot: u16, value: DexValue) -> bool {
        let Some(data) = self.objects.get_mut(id as usize).and_then(Option::as_mut) else {
            return false;
        };
        let FieldStorage::Slots { values, .. } = &mut data.storage else {
            return false;
        };
        let slot = slot as usize;
        if values.len() <= slot {
            values.resize_with(slot + 1, || None);
        }
        values[slot] = Some(value);
        true
    }

    pub(super) fn get(&self, key: &(String, String)) -> Option<&DexValue> {
        self.get_field(&key.0, &key.1)
    }

    pub(super) fn get_field(&self, owner_key: &str, field_signature: &str) -> Option<&DexValue> {
        let Ok(id) = parse_owner(owner_key)? else {
            return None;
        };
        self.object_field(id, field_signature)
    }

    pub(super) fn remove_object_field(
        &mut self,
        id: u64,
        field_signature: &str,
    ) -> Option<DexValue> {
        let data = self.objects.get_mut(id as usize)?.as_mut()?;
        match &mut data.storage {
            FieldStorage::Slots { layout, values } => {
                let slot = *self.layouts[*layout as usize].slots.get(field_signature)?;
                values.get_mut(slot as usize)?.take()
            }
            FieldStorage::Map(map) => map.remove(field_signature),
        }
    }

    pub(super) fn iter_objects(&self) -> impl Iterator<Item = (OwnerRef<'_>, FieldsView<'_>)> {
        let layouts = self.layouts.as_slice();
        self.objects
            .iter()
            .enumerate()
            .filter_map(move |(id, entry)| {
                let data = entry.as_ref()?;
                Some((
                    OwnerRef::Object(id as u64),
                    FieldsView::object(data, layouts),
                ))
            })
    }

    pub(super) fn fields_of_object(&self, id: u64) -> Option<FieldsView<'_>> {
        let data = self.object_data(id)?;
        Some(FieldsView::object(data, &self.layouts))
    }
}

#[derive(Debug, Default)]
pub(super) struct StaticFieldStore {
    classes: HashMap<String, RuntimeClassStaticFields>,
}

impl StaticFieldStore {
    pub(super) fn class_layout_defined(&self, class_descriptor: &str) -> bool {
        self.classes
            .get(class_descriptor)
            .is_some_and(|class| class.layout_defined)
    }

    pub(super) fn define_class_layout(
        &mut self,
        class_descriptor: &str,
        field_signatures: &[Rc<str>],
    ) {
        let class = self.classes.entry(class_descriptor.to_owned()).or_default();
        if class.layout_defined {
            return;
        }
        for signature in field_signatures {
            class.slot_for(signature);
        }
        class.layout_defined = true;
    }

    pub(super) fn class_field_slot(
        &self,
        class_descriptor: &str,
        field_signature: &str,
    ) -> Option<u16> {
        self.classes.get(class_descriptor)?.slot_of(field_signature)
    }

    pub(super) fn get_by_signature(
        &self,
        class_descriptor: &str,
        field_signature: &str,
    ) -> Option<&DexValue> {
        self.classes.get(class_descriptor)?.get(field_signature)
    }

    pub(super) fn get(&self, field: &DexFieldId) -> Option<&DexValue> {
        self.get_by_signature(&field.class_descriptor, &field.display_signature())
    }

    pub(super) fn get_slot(&self, class_descriptor: &str, slot: u16) -> Option<&DexValue> {
        self.classes.get(class_descriptor)?.get_slot(slot)
    }

    pub(super) fn set_slot(&mut self, class_descriptor: &str, slot: u16, value: DexValue) -> bool {
        self.classes
            .entry(class_descriptor.to_owned())
            .or_default()
            .set_slot(slot, value)
    }

    pub(super) fn insert(&mut self, field: &DexFieldId, value: DexValue) -> Option<DexValue> {
        self.classes
            .entry(field.class_descriptor.clone())
            .or_default()
            .insert(field.display_signature(), value)
    }
}

#[derive(Debug, Default)]
struct RuntimeClassStaticFields {
    layout_defined: bool,
    signatures: Vec<String>,
    slots: HashMap<String, u16>,
    values: Vec<Option<DexValue>>,
}

impl RuntimeClassStaticFields {
    fn slot_for(&mut self, field_signature: &str) -> u16 {
        if let Some(slot) = self.slots.get(field_signature) {
            return *slot;
        }
        let slot = self.signatures.len() as u16;
        self.signatures.push(field_signature.to_owned());
        self.slots.insert(field_signature.to_owned(), slot);
        slot
    }

    fn slot_of(&self, field_signature: &str) -> Option<u16> {
        self.slots.get(field_signature).copied()
    }

    fn get(&self, field_signature: &str) -> Option<&DexValue> {
        let slot = self.slot_of(field_signature)?;
        self.get_slot(slot)
    }

    fn get_slot(&self, slot: u16) -> Option<&DexValue> {
        self.values.get(slot as usize)?.as_ref()
    }

    fn set_slot(&mut self, slot: u16, value: DexValue) -> bool {
        let slot = slot as usize;
        if self.values.len() <= slot {
            self.values.resize_with(slot + 1, || None);
        }
        self.values[slot] = Some(value);
        true
    }

    fn insert(&mut self, field_signature: String, value: DexValue) -> Option<DexValue> {
        let slot = self.slot_for(&field_signature);
        let old = self.get_slot(slot).cloned();
        self.set_slot(slot, value);
        old
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct AndroidMatrixState {
    pub(super) values: [f32; 9],
}

impl AndroidMatrixState {
    pub(super) fn identity() -> Self {
        Self {
            values: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SharedPreferencesEditorState {
    pub(super) preferences_key: String,
    pub(super) clear_first: bool,
    pub(super) changes: Vec<SharedPreferencesEditorChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InputStreamState {
    pub(super) bytes: Vec<u8>,
    pub(super) position: usize,
    pub(super) pending_low_surrogate: Option<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct NumberFormatState {
    pub(super) minimum_fraction_digits: u32,
    pub(super) maximum_fraction_digits: u32,
    pub(super) grouping_used: bool,
}

impl Default for NumberFormatState {
    fn default() -> Self {
        Self {
            minimum_fraction_digits: 0,
            maximum_fraction_digits: 3,
            grouping_used: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum SharedPreferencesEditorChange {
    Put { key: String, value: DexValue },
    Remove { key: String },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct StringReaderState {
    pub(super) units: Vec<u16>,
    pub(super) position: usize,
    pub(super) closed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SpannableSpanState {
    pub(super) value: DexValue,
    pub(super) start: i32,
    pub(super) end: i32,
    pub(super) flags: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct XmlParserState {
    pub(super) resource_id: i32,
    pub(super) events: Vec<XmlEvent>,
    pub(super) position: XmlParserPosition,
    pub(super) closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeMenu {
    pub(super) resource_id: i32,
    pub(super) items: Vec<RuntimeMenuItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeMenuItem {
    pub(super) item_id: i32,
    pub(super) title: String,
    pub(super) enabled: bool,
    pub(super) visible: bool,
    pub(super) on_click: Option<String>,
    pub(super) icon_resource: Option<i32>,
    pub(super) show_as_action: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InflatedPreference {
    pub(super) key: Option<String>,
    pub(super) title: String,
    pub(super) summary: Option<String>,
    pub(super) icon_resource_id: Option<u32>,
    pub(super) class_descriptor: String,
    pub(super) kind: crate::dex_run::DexPreferenceKind,
    pub(super) depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RuntimeMenuOverlay {
    pub(super) popup_key: String,
    pub(super) anchor_view_id: Option<i32>,
    pub(super) list_view: DexValue,
    pub(super) items: Vec<RuntimeMenuOverlayItem>,
    pub(super) menu_item_click_listener: Option<DexValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeMenuOverlayItem {
    pub(super) item_id: i32,
    pub(super) title: String,
    pub(super) enabled: bool,
    pub(super) position: i32,
    pub(super) on_click: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeDialogOverlay {
    pub(super) dialog_key: String,
    pub(super) title: Option<String>,
    pub(super) content_layouts: Vec<i32>,
    pub(super) items: Vec<RuntimeDialogItem>,
    pub(super) buttons: Vec<RuntimeDialogButton>,
    pub(super) input_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeDialogItem {
    pub(super) text: String,
    pub(super) enabled: bool,
    pub(super) selected: bool,
    pub(super) action: RuntimeDialogItemAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RuntimeDialogItemAction {
    ListPreferenceValue {
        preference_key: String,
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeDialogButton {
    pub(super) text: String,
    pub(super) enabled: bool,
    pub(super) action: RuntimeDialogButtonAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RuntimeDialogButtonAction {
    Dismiss,
    CommitEditTextPreference { preference_key: String },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RuntimeActivity {
    pub(super) class_descriptor: String,
    pub(super) receiver: DexValue,
    pub(super) method_signature: String,
    pub(super) result: String,
    pub(super) object_start: u64,
    pub(super) content_layout_start: usize,
    pub(super) inflated_layout_start: usize,
    pub(super) initial_inflated_layout_end: usize,
    pub(super) dialog_content_layout_start: usize,
    pub(super) runtime_view_root_start: usize,
    pub(super) view_lookup_start: usize,
    pub(super) saved_chrome: Option<SavedActivityChrome>,
    pub(super) result_target: Option<(DexValue, i32)>,
    pub(super) result_code: i32,
    pub(super) result_intent: Option<DexValue>,
    // cite: frameworks-base ActivityThread.java#performLaunchActivity
    pub(super) adapter_rows_snapshot: HashMap<i32, DexAdapterRows>,
    pub(super) adapter_item_clicks_snapshot: HashMap<i32, AdapterItemClick>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SavedActivityChrome {
    pub(super) action_bar_title: Option<String>,
    pub(super) action_bar_nav_icon: Option<i32>,
    pub(super) action_bar_nav_click: Option<DexValue>,
    pub(super) action_bar_present: bool,
    pub(super) action_bar_home_as_up: bool,
    pub(super) options_menu: Vec<RuntimeMenuItem>,
    pub(super) inflated_preferences: Vec<InflatedPreference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum XmlParserPosition {
    StartDocument,
    Event(usize),
    EndDocument,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum MainThreadTask {
    LaunchActivity { class_descriptor: String },
    InvalidateOptionsMenu { activity: DexValue },
    StartThread { thread: DexValue },
}

fn handler_post_dispatch_order(left: &HandlerPost, right: &HandlerPost) -> std::cmp::Ordering {
    match (left.at_front, right.at_front) {
        (true, true) => right.sequence.cmp(&left.sequence),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => left.sequence.cmp(&right.sequence),
    }
}

fn choreographer_frame_callback_dispatch_order(
    left: &ChoreographerFrameCallback,
    right: &ChoreographerFrameCallback,
) -> std::cmp::Ordering {
    left.due_time_millis
        .cmp(&right.due_time_millis)
        .then_with(|| left.sequence.cmp(&right.sequence))
}

fn choreographer_runnable_callback_dispatch_order(
    left: &ChoreographerRunnableCallback,
    right: &ChoreographerRunnableCallback,
) -> std::cmp::Ordering {
    left.callback_type
        .cmp(&right.callback_type)
        .then_with(|| left.due_time_millis.cmp(&right.due_time_millis))
        .then_with(|| left.sequence.cmp(&right.sequence))
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct HandlerPost {
    pub(super) handler_key: String,
    pub(super) handler: Option<DexValue>,
    pub(super) runnable: DexValue,
    pub(super) delay_millis: i64,
    pub(super) due_time_millis: i64,
    pub(super) sequence: u64,
    pub(super) at_front: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ChoreographerFrameCallback {
    pub(super) callback: DexValue,
    pub(super) due_time_millis: i64,
    pub(super) sequence: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ChoreographerRunnableCallback {
    pub(super) callback_type: i32,
    pub(super) action: DexValue,
    pub(super) token: DexValue,
    pub(super) due_time_millis: i64,
    pub(super) sequence: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MainThreadWork {
    pub(super) tasks: Vec<MainThreadTask>,
    pub(super) handler_posts: Vec<HandlerPost>,
    pub(super) choreographer_callbacks: Vec<ChoreographerRunnableCallback>,
    pub(super) frame_callbacks: Vec<DexValue>,
    pub(super) thread_resumes: Vec<ThreadResume>,
}

fn runtime_value_is_object_like(value: &DexValue) -> bool {
    matches!(value, DexValue::Object(_) | DexValue::Reference(_))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClassInitializationContinuation {
    pub(super) class_descriptor: String,
    pub(super) default_interfaces: Vec<String>,
    pub(super) next_interface_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ParkedRuntimeAction {
    ContinueClassInitialization(ClassInitializationContinuation),
    FinishClassInitialization(String),
    InvokeExactVoid {
        method: DexMethodId,
        arguments: Vec<DexValue>,
    },
    InvokeBuiltinVoid {
        method: DexMethodId,
        invocation: DexInvocation,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ThreadResume {
    pub(super) thread: DexValue,
    pub(super) execution: DexParkedExecution,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ClassInitializationWaiter {
    pub(super) thread: DexValue,
    pub(super) execution: DexParkedExecution,
}

fn thread_storage_key(thread: &DexValue) -> Option<String> {
    match thread {
        DexValue::Object(object) => {
            Some(format!("object:{}:{}", object.id, object.class_descriptor))
        }
        DexValue::Reference(reference) => Some(format!("reference:{reference}")),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct TraversalRequest {
    pub(super) layout_requested: bool,
    pub(super) draw_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeCanvasFrame {
    pub(super) root_view_key: String,
    pub(super) class_descriptor: String,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) canvas_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum CanvasDrawOp {
    Save,
    Restore,
    Translate {
        dx: f32,
        dy: f32,
    },
    Scale {
        sx: f32,
        sy: f32,
    },
    Rotate {
        degrees: f32,
    },
    ConcatMatrix {
        matrix: DexValue,
    },
    ClipRect {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        op: DexValue,
    },
    ClipPath {
        ops: Vec<PathOp>,
        op: DexValue,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        paint: DexValue,
    },
    Rect {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        paint: DexValue,
    },
    Circle {
        cx: f32,
        cy: f32,
        radius: f32,
        paint: DexValue,
    },
    RoundRect {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        rx: f32,
        ry: f32,
        paint: DexValue,
    },
    TextLayout {
        text: String,
        paint: DexValue,
        layout: DexValue,
    },
    Path {
        ops: Vec<PathOp>,
        paint: DexValue,
    },
    Bitmap {
        bitmap: DexValue,
        source: DexValue,
        destination: DexValue,
        paint: DexValue,
    },
    RenderNode {
        render_node: DexValue,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum PathOp {
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    RLineTo {
        dx: f32,
        dy: f32,
    },
    QuadTo {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    CubicTo {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
    },
    RCubicTo {
        dx1: f32,
        dy1: f32,
        dx2: f32,
        dy2: f32,
        dx3: f32,
        dy3: f32,
    },
    BooleanOp {
        left: Vec<PathOp>,
        right: Vec<PathOp>,
        op: PathBooleanOp,
    },
    // cite: frameworks-base Path.java#addRect, addRoundRect
    Rect {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        clockwise: bool,
    },
    RoundRect {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        radii: Vec<f32>,
        clockwise: bool,
    },
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PathBooleanOp {
    Difference,
    Intersect,
    Union,
    Xor,
    ReverseDifference,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SqliteCursorState {
    pub(super) columns: Vec<String>,
    pub(super) rows: Vec<Vec<DexValue>>,
    pub(super) position: isize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RegexMatcherState {
    pub(super) pattern: String,
    pub(super) input: String,
    pub(super) search_start: usize,
    pub(super) last_groups: Vec<Option<RegexMatchRange>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RegexMatchRange {
    pub(super) start: usize,
    pub(super) end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct BootstrapFieldOwner {
    pub(super) value: DexValue,
    pub(super) class_descriptor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EnumRuntimeValue {
    pub(super) class_descriptor: String,
    pub(super) name: String,
    pub(super) ordinal: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MathContextRuntimeValue {
    pub(super) precision: i32,
    pub(super) rounding_mode: String,
}
