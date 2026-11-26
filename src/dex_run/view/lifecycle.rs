use super::helpers::*;
use super::*;
use crate::trace;

pub(in crate::dex_run::view) struct RuntimeRootGeometry {
    pub(in crate::dex_run::view) x: i32,
    pub(in crate::dex_run::view) y: i32,
    pub(in crate::dex_run::view) width_spec: i32,
    pub(in crate::dex_run::view) height_spec: i32,
}

impl DexProgram {
    pub(in crate::dex_run) fn read_view_id(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.view_id_from_value(&invocation.arguments[0])
            .or(Some(-1))
    }

    pub(in crate::dex_run) fn view_unique_drawing_id(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let owner_key = instance_field_owner_key(&invocation.arguments[0])?;
        if let Some(object_id) = identity_object_id(&owner_key) {
            return i64::try_from(object_id).ok();
        }

        Some(stable_view_drawing_id(&owner_key))
    }

    pub(in crate::dex_run) fn view_id_from_value(&self, value: &DexValue) -> Option<i32> {
        if let Some(DexValue::Int(id)) = self.read_stored_instance_field(value, &view_id_field()) {
            return Some(id);
        }

        if let DexValue::Reference(reference) = value {
            if let Some((_, id)) = reference.rsplit_once(":id/") {
                return id.parse().ok();
            }
            if reference.as_ref() == "Landroid/view/View;:action-bar-navigation" {
                return Some(0x0102_002c);
            }
        }

        None
    }

    pub(in crate::dex_run) fn write_view_id(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(id) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_id_field(),
            DexValue::Int(id),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_view_visibility(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
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

        let value = self
            .read_stored_instance_field(&invocation.arguments[0], &view_visibility_field())
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(visibility) = value else {
            return None;
        };

        Some(visibility)
    }

    // cite: frameworks-base View.java#isShown
    pub(in crate::dex_run) fn view_is_shown(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let decor_key = map_identity_key(&decor_view_reference());
        let mut current = invocation.arguments[0].clone();
        for _ in 0..256 {
            let visibility =
                match self.read_stored_instance_field(&current, &view_visibility_field()) {
                    Some(DexValue::Int(visibility)) => visibility,
                    Some(_) => return None,
                    None => 0,
                };
            if visibility != 0 {
                return Some(0);
            }

            let current_key = map_identity_key(&current);
            if current_key == decor_key {
                return Some(1);
            }

            let Some(parent) = self
                .runtime
                .borrow()
                .view_parents
                .get(&current_key)
                .cloned()
            else {
                return Some(0);
            };
            current = parent;
        }

        Some(0)
    }

    pub(in crate::dex_run) fn write_view_visibility(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(visibility) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_visibility_field(),
            DexValue::Int(visibility),
        )
        .then_some(())
    }

    // cite: frameworks-base View.java#getSystemUiVisibility
    pub(in crate::dex_run) fn read_view_system_ui_visibility(
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
                &view_system_ui_visibility_field(),
            )
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(visibility) = value else {
            return None;
        };

        Some(visibility)
    }

    // cite: frameworks-base View.java#setSystemUiVisibility
    pub(in crate::dex_run) fn write_view_system_ui_visibility(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(visibility) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_system_ui_visibility_field(),
            DexValue::Int(visibility),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_view_bool_field(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
        default_value: i32,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
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

        let value = self
            .read_stored_instance_field(&invocation.arguments[0], field)
            .unwrap_or(DexValue::Int(default_value));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(if value == 0 { 0 } else { 1 })
    }

    pub(in crate::dex_run) fn write_view_bool_field(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
    ) -> Option<()> {
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
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            field,
            DexValue::Int(if value == 0 { 0 } else { 1 }),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_view_system_gesture_exclusion_rects(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2
            || !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_system_gesture_exclusion_rects_field(),
            invocation.arguments[1].clone(),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_view_int_field(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
        default_value: i32,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
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

        let value = self
            .read_stored_instance_field(&invocation.arguments[0], field)
            .unwrap_or(DexValue::Int(default_value));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_view_int_field(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
    ) -> Option<()> {
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
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(&invocation.arguments[0], field, DexValue::Int(value))
            .then_some(())
    }

    pub(in crate::dex_run) fn write_view_layout_direction(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        const LAYOUT_DIRECTION_INHERIT: i32 = 2;

        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(layout_direction) = invocation.arguments[1] else {
            return None;
        };

        let current = self
            .read_stored_instance_field(&invocation.arguments[0], &view_layout_direction_field())
            .unwrap_or(DexValue::Int(LAYOUT_DIRECTION_INHERIT));
        let DexValue::Int(current_layout_direction) = current else {
            return None;
        };

        if current_layout_direction == layout_direction {
            return Some(());
        }

        let wrote = self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_layout_direction_field(),
            DexValue::Int(layout_direction),
        );
        if !wrote {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_layout_requested_field(),
            DexValue::Int(1),
        );
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_invalidated_field(),
            DexValue::Int(1),
        );
        self.mark_view_render_node_dirty(&invocation.arguments[0]);
        self.runtime.borrow_mut().request_layout_traversal();

        Some(())
    }

    pub(in crate::dex_run) fn write_view_layout_requested(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 1 {
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

        let wrote = self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_layout_requested_field(),
            DexValue::Int(1),
        );
        if wrote {
            self.mark_view_render_node_dirty(&invocation.arguments[0]);
            let mut runtime = self.runtime.borrow_mut();
            // cite: frameworks-base View.java#requestLayout
            runtime
                .view_measure_caches
                .remove(&map_identity_key(&invocation.arguments[0]));
            runtime.request_layout_traversal();
        }

        wrote.then_some(())
    }

    pub(in crate::dex_run) fn write_view_requested_frame_rate(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        self.write_view_float_field(invocation, &view_float_field("requestedFrameRate"))
    }

    pub(in crate::dex_run) fn write_view_float_field(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let frame_rate = canvas_float_argument(&invocation.arguments[1])?;

        self.write_stored_instance_field(
            &invocation.arguments[0],
            field,
            DexValue::Float(frame_rate),
        )
        .then_some(())
    }

    // cite: frameworks-base View.java#setAlpha, setElevation, invalidateViewProperty, damageInParent
    // cite: frameworks-base RenderNode.java#setAlpha, setElevation
    pub(in crate::dex_run) fn write_view_render_node_float_property(
        &self,
        invocation: &DexInvocation,
        view_field: &DexFieldId,
        render_node_field: &DexFieldId,
        default: f32,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = canvas_float_argument(&invocation.arguments[1])?;
        let previous = match self.read_stored_instance_field(&invocation.arguments[0], view_field) {
            Some(DexValue::Float(value)) => value,
            Some(DexValue::Int(bits)) => f32::from_bits(bits as u32),
            _ => default,
        };
        self.write_stored_instance_field(
            &invocation.arguments[0],
            view_field,
            DexValue::Float(value),
        )
        .then_some(())?;

        if previous != value {
            let view_key = map_identity_key(&invocation.arguments[0]);
            let render_node = self
                .runtime
                .borrow()
                .view_render_nodes
                .get(&view_key)
                .cloned();
            if let Some(render_node) = render_node {
                self.write_stored_instance_field(
                    &render_node,
                    render_node_field,
                    DexValue::Float(value),
                );
                self.runtime.borrow_mut().request_draw_traversal();
            }
        }

        Some(())
    }

    pub(in crate::dex_run) fn read_view_float_field(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
        default: f32,
    ) -> Option<f32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = self
            .read_stored_instance_field(&invocation.arguments[0], field)
            .unwrap_or(DexValue::Float(default));
        let DexValue::Float(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn read_view_drawing_time(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i64> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = self
            .read_stored_instance_field(&invocation.arguments[0], &view_long_field("drawingTime"))
            .unwrap_or(DexValue::Long(0));
        let DexValue::Long(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_view_invalidated(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let wrote = self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_invalidated_field(),
            DexValue::Int(1),
        );
        if wrote {
            self.mark_view_render_node_dirty(&invocation.arguments[0]);
            self.runtime.borrow_mut().request_draw_traversal();
        }

        wrote.then_some(())
    }

    // cite: frameworks-base ViewParent.java#onDescendantInvalidated
    // cite: frameworks-base ViewRootImpl.java#onDescendantInvalidated, invalidate
    // cite: androidx RenderNodeLayer.android.kt#triggerRepaint
    pub(in crate::dex_run) fn view_parent_on_descendant_invalidated(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
            || !dex_value_is_object_like(&invocation.arguments[2])
        {
            return None;
        }

        self.runtime.borrow_mut().request_draw_traversal();
        Some(())
    }

    pub(in crate::dex_run) fn write_view_focusable_bool(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };
        let mode = if value == 0 { 0 } else { 1 };

        self.write_view_focusable_state(invocation, mode)
    }

    pub(in crate::dex_run) fn write_view_focusable_mode(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(mode) = invocation.arguments[1] else {
            return None;
        };

        self.write_view_focusable_state(invocation, mode)
    }

    pub(in crate::dex_run::view) fn write_view_focusable_state(
        &self,
        invocation: &DexInvocation,
        mode: i32,
    ) -> Option<()> {
        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        if !self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_focusable_mode_field(),
            DexValue::Int(mode),
        ) {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_focus_field("setFocusable")?,
            DexValue::Int(i32::from(mode != 0)),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn clear_view_focus(&self, invocation: &DexInvocation) -> Option<()> {
        let [view] = invocation.arguments.as_slice() else {
            return None;
        };
        if !dex_value_is_object_like(view) {
            return None;
        }

        let is_focused = self
            .focused_view()
            .is_some_and(|focused| map_identity_key(&focused) == map_identity_key(view));
        if is_focused {
            self.set_focused_view(None);
        } else {
            self.write_stored_instance_field(view, &view_focused_field(), DexValue::Int(0));
        }
        Some(())
    }

    pub(in crate::dex_run) fn request_view_focus(&self, invocation: &DexInvocation) -> Option<i32> {
        match invocation.arguments.as_slice() {
            [view] if dex_value_is_object_like(view) => {}
            [view, DexValue::Int(_)] if dex_value_is_object_like(view) => {}
            [view, DexValue::Int(_), rect]
                if dex_value_is_object_like(view) && dex_value_is_nullable_object_like(rect) => {}
            _ => return None,
        }

        self.set_focused_view(Some(invocation.arguments[0].clone()));
        Some(1)
    }

    pub(in crate::dex_run) fn read_view_layout_params(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
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

        Some(
            self.read_stored_instance_field(&invocation.arguments[0], &view_layout_params_field())
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_view_layout_params(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let layout_params = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::Null,
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_layout_params_field(),
            layout_params,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn view_group_remove_all_views(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if invocation.arguments[0] == DexValue::Null {
            return Some(());
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let group_key = map_identity_key(&invocation.arguments[0]);
        let mut runtime = self.runtime.borrow_mut();
        let removed_children = runtime
            .view_group_children
            .remove(&group_key)
            .unwrap_or_default();
        for child in &removed_children {
            runtime.view_parents.remove(&map_identity_key(child));
        }
        drop(runtime);
        for child in &removed_children {
            self.discard_view_render_node_subtree(child);
        }
        self.mark_view_render_node_dirty(&invocation.arguments[0]);

        for child in &removed_children {
            self.notify_view_hierarchy_change(
                &invocation.arguments[0],
                child,
                "onChildViewRemoved",
            )?;
        }

        Some(())
    }

    pub(in crate::dex_run) fn view_group_remove_view_at(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(index) = invocation.arguments[1] else {
            return None;
        };
        let index = usize::try_from(index).ok()?;
        let group_key = map_identity_key(&invocation.arguments[0]);
        let mut runtime = self.runtime.borrow_mut();
        let children = runtime.view_group_children.get_mut(&group_key)?;

        if index >= children.len() {
            return Some(());
        }

        let child = children.remove(index);
        runtime.view_parents.remove(&map_identity_key(&child));
        drop(runtime);
        self.discard_view_render_node_subtree(&child);
        self.mark_view_render_node_dirty(&invocation.arguments[0]);

        self.notify_view_hierarchy_change(&invocation.arguments[0], &child, "onChildViewRemoved")?;

        Some(())
    }

    // cite: frameworks-base ViewGroup.java#removeView, removeViewInternal
    pub(in crate::dex_run) fn view_group_remove_view(
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

        let group_key = map_identity_key(&invocation.arguments[0]);
        let child_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        let children = runtime.view_group_children.get_mut(&group_key)?;
        let Some(index) = children
            .iter()
            .position(|child| map_identity_key(child) == child_key)
        else {
            return Some(());
        };

        let child = children.remove(index);
        runtime.view_parents.remove(&child_key);
        drop(runtime);
        self.discard_view_render_node_subtree(&child);
        self.mark_view_render_node_dirty(&invocation.arguments[0]);
        self.runtime.borrow_mut().request_layout_traversal();

        self.notify_view_hierarchy_change(&invocation.arguments[0], &child, "onChildViewRemoved")?;

        Some(())
    }

    pub(in crate::dex_run) fn view_group_add_view(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        let group_key = map_identity_key(&invocation.arguments[0]);
        let child_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        if let Some(previous_parent) = runtime.view_parents.get(&child_key).cloned() {
            let previous_parent_key = map_identity_key(&previous_parent);
            if let Some(previous_siblings) =
                runtime.view_group_children.get_mut(&previous_parent_key)
            {
                previous_siblings.retain(|child| map_identity_key(child) != child_key);
            }
        }
        runtime
            .view_parents
            .insert(child_key, invocation.arguments[0].clone());
        runtime
            .view_group_children
            .entry(group_key)
            .or_default()
            .push(invocation.arguments[1].clone());
        drop(runtime);

        self.notify_view_hierarchy_change(
            &invocation.arguments[0],
            &invocation.arguments[1],
            "onChildViewAdded",
        )?;

        Some(())
    }

    pub(in crate::dex_run) fn read_view_parent(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        if is_decor_view(&invocation.arguments[0]) {
            return Some(DexValue::Null);
        }

        if self.view_id_from_value(&invocation.arguments[0]) == Some(ANDROID_CONTENT_VIEW_ID) {
            return Some(decor_view_reference());
        }

        let view_key = map_identity_key(&invocation.arguments[0]);
        Some(
            self.runtime
                .borrow()
                .view_parents
                .get(&view_key)
                .cloned()
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn read_view_root(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        self.read_view_root_value(&invocation.arguments[0])
    }

    pub(in crate::dex_run::view) fn read_view_root_value(
        &self,
        view: &DexValue,
    ) -> Option<DexValue> {
        if !dex_value_is_object_like(view) {
            return None;
        }

        let mut current = view.clone();
        for _ in 0..64 {
            let current_key = map_identity_key(&current);
            let parent = self
                .runtime
                .borrow()
                .view_parents
                .get(&current_key)
                .cloned();
            let Some(parent) = parent else {
                break;
            };
            if parent == DexValue::Null || is_decor_view(&parent) {
                break;
            }
            if map_identity_key(&parent) == current_key {
                break;
            }

            current = parent;
        }

        Some(current)
    }

    pub(in crate::dex_run) fn view_root_window_insets(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        if !self.view_value_is_attached_to_window(&invocation.arguments[0]) {
            return Some(DexValue::Null);
        }

        let root = self.read_view_root(invocation)?;
        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/view/WindowInsets;:root:{}",
            map_identity_key(&root)
        ))))
    }

    pub(in crate::dex_run) fn view_handler(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.view_handler_for_value(&invocation.arguments[0])
    }

    pub(in crate::dex_run::view) fn view_handler_for_value(
        &self,
        view: &DexValue,
    ) -> Option<DexValue> {
        if !dex_value_is_object_like(view) {
            return None;
        }

        if !self.view_value_is_attached_to_window(view) {
            return Some(DexValue::Null);
        }

        let view_key = map_identity_key(view);
        let handler = DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/os/Handler;:view:{view_key}"
        )));
        let handler_key = map_identity_key(&handler);
        self.runtime.borrow_mut().handler_loopers.insert(
            handler_key,
            DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main")),
        );

        Some(handler)
    }

    pub(in crate::dex_run) fn post_view_runnable(
        &self,
        invocation: &DexInvocation,
        delay_millis: Option<i64>,
    ) -> Option<i32> {
        let expected_len = if delay_millis.is_some() { 2 } else { 4 };
        if invocation.arguments.len() != expected_len {
            return None;
        }

        let runnable = match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };
        let delay_millis = match delay_millis {
            Some(delay_millis) => delay_millis,
            None => {
                let DexValue::Long(delay_millis) = invocation.arguments[2] else {
                    return None;
                };
                let DexValue::Long(high_slot_value) = invocation.arguments[3] else {
                    return None;
                };
                if high_slot_value != delay_millis {
                    return None;
                }
                delay_millis
            }
        };

        let handler = self.view_handler_for_value(&invocation.arguments[0])?;
        if handler == DexValue::Null {
            return Some(0);
        }

        self.runtime
            .borrow_mut()
            .post_handler_callback(handler, runnable, delay_millis, false);

        Some(1)
    }

    pub(in crate::dex_run) fn remove_view_callbacks(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let runnable = match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };
        let handler = self.view_handler_for_value(&invocation.arguments[0])?;
        if handler == DexValue::Null {
            return Some(0);
        }

        let handler_key = map_identity_key(&handler);
        self.runtime
            .borrow_mut()
            .handler_posts
            .retain(|post| post.handler_key != handler_key || post.runnable != runnable);

        Some(1)
    }

    pub(in crate::dex_run) fn view_unschedule_drawable(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_nullable_object_like(&invocation.arguments[1])
        {
            return None;
        }

        Some(())
    }

    pub(in crate::dex_run) fn view_autofill_id(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let view_key = map_identity_key(&invocation.arguments[0]);
        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/view/autofill/AutofillId;:view:{view_key}"
        ))))
    }

    pub(in crate::dex_run) fn view_is_attached_to_window(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let attached = self.view_value_is_attached_to_window(&invocation.arguments[0]);
        Some(i32::from(attached))
    }

    pub(in crate::dex_run) fn view_is_hardware_accelerated(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        // cite: frameworks-base View.java#isHardwareAccelerated, mAttachInfo, mHardwareAccelerated
        Some(i32::from(
            self.view_value_is_attached_to_window(&invocation.arguments[0]),
        ))
    }

    pub(in crate::dex_run) fn view_has_window_focus(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        if let Some(DexValue::Int(value)) =
            self.read_stored_instance_field(&invocation.arguments[0], &view_window_focus_field())
        {
            return Some(if value == 0 { 0 } else { 1 });
        }

        Some(i32::from(
            self.view_value_is_attached_to_window(&invocation.arguments[0]),
        ))
    }

    pub(in crate::dex_run) fn view_value_is_attached_to_window(&self, view: &DexValue) -> bool {
        self.view_value_is_attached_to_window_inner(view, &mut Vec::new())
    }

    pub(in crate::dex_run::view) fn view_value_is_attached_to_window_inner(
        &self,
        view: &DexValue,
        visited: &mut Vec<String>,
    ) -> bool {
        let view_key = map_identity_key(view);
        if visited.contains(&view_key) {
            return false;
        }
        visited.push(view_key.clone());

        if is_decor_view(view) || self.view_id_from_value(view) == Some(ANDROID_CONTENT_VIEW_ID) {
            return true;
        }

        // cite: frameworks-base ViewRootImpl.java#setView
        if self
            .runtime
            .borrow()
            .runtime_view_roots
            .iter()
            .any(|root| root.view_key == view_key)
        {
            return true;
        }

        let parent = self.runtime.borrow().view_parents.get(&view_key).cloned();
        parent
            .as_ref()
            .map(|parent| self.view_value_is_attached_to_window_inner(parent, visited))
            .unwrap_or(false)
    }

    pub(in crate::dex_run) fn dispatch_view_attached_to_window(
        &self,
        view: &DexValue,
        depth: usize,
    ) -> Result<()> {
        self.dispatch_view_attached_to_window_inner(view, &mut Vec::new(), depth)
    }

    pub(in crate::dex_run::view) fn dispatch_view_attached_to_window_inner(
        &self,
        view: &DexValue,
        visited: &mut Vec<String>,
        depth: usize,
    ) -> Result<()> {
        let view_key = map_identity_key(view);
        if visited.contains(&view_key) {
            return Ok(());
        }
        visited.push(view_key.clone());

        let (listeners, children) = {
            let runtime = self.runtime.borrow();
            (
                runtime
                    .view_attach_state_listeners
                    .get(&view_key)
                    .cloned()
                    .unwrap_or_default(),
                runtime
                    .view_group_children
                    .get(&view_key)
                    .cloned()
                    .unwrap_or_default(),
            )
        };

        self.dispatch_single_view_on_attached_to_window(view.clone(), depth)?;
        for listener in listeners {
            self.dispatch_single_view_attached_to_window(listener, view.clone(), depth)?;
        }
        for child in children {
            self.dispatch_view_attached_to_window_inner(&child, visited, depth)?;
        }

        Ok(())
    }

    pub(in crate::dex_run::view) fn dispatch_single_view_on_attached_to_window(
        &self,
        view: DexValue,
        depth: usize,
    ) -> Result<()> {
        let Some(view_descriptor) = value_class_descriptor(&view) else {
            return Err(WinedroidError::new(format!(
                "View.onAttachedToWindow dispatch requires an object-backed view\nview: {view}"
            )));
        };

        let on_attached_method = DexMethodId {
            class_descriptor: view_descriptor.to_owned(),
            name: "onAttachedToWindow".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(view_descriptor, &on_attached_method)?
        else {
            return Ok(());
        };

        let result = self.execute_method(None, &resolved, Some(&[view]), depth + 1)?;

        match result {
            DexExecutionResult::Void => Ok(()),
            DexExecutionResult::Int(_)
            | DexExecutionResult::Value(_)
            | DexExecutionResult::Throw(_) => Err(WinedroidError::new(
                "View.onAttachedToWindow did not complete normally",
            )),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(
                    "View.onAttachedToWindow did not complete normally",
                ))
            }
        }
    }

    pub(in crate::dex_run::view) fn dispatch_single_view_attached_to_window(
        &self,
        listener: DexValue,
        view: DexValue,
        depth: usize,
    ) -> Result<()> {
        let Some(listener_descriptor) = value_class_descriptor(&listener) else {
            return Err(WinedroidError::new(format!(
                "View attach listener dispatch requires an object-backed listener\nlistener: {listener}"
            )));
        };

        let on_attached_method = DexMethodId {
            class_descriptor: listener_descriptor.to_owned(),
            name: "onViewAttachedToWindow".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "VL".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec!["Landroid/view/View;".to_owned()],
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(listener_descriptor, &on_attached_method)?
        else {
            return Err(WinedroidError::new(format!(
                "View attach listener method was not found\nlistener class: {listener_descriptor}\nnext missing primitive: View.OnAttachStateChangeListener dispatch"
            )));
        };

        let result = self.execute_method(None, &resolved, Some(&[listener, view]), depth + 1)?;

        match result {
            DexExecutionResult::Void => Ok(()),
            DexExecutionResult::Int(_)
            | DexExecutionResult::Value(_)
            | DexExecutionResult::Throw(_) => Err(WinedroidError::new(
                "View attach listener onViewAttachedToWindow did not complete normally",
            )),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(
                    "View attach listener onViewAttachedToWindow did not complete normally",
                ))
            }
        }
    }

    pub(in crate::dex_run) fn dispatch_view_tree_draw_listeners(
        &self,
        depth: usize,
    ) -> Result<usize> {
        let listeners = self.view_tree_draw_listener_snapshot();
        let listener_count = listeners.len();

        for listener in listeners {
            self.dispatch_single_view_tree_draw_listener(listener, depth)?;
        }

        Ok(listener_count)
    }

    pub(in crate::dex_run) fn dispatch_view_tree_global_layout_listeners(
        &self,
        depth: usize,
    ) -> Result<usize> {
        let listeners = self.view_tree_global_layout_listener_snapshot();
        let listener_count = listeners.len();

        for listener in listeners {
            self.dispatch_single_view_tree_global_layout_listener(listener, depth)?;
        }

        Ok(listener_count)
    }

    pub(in crate::dex_run) fn dispatch_runtime_view_frames(
        &self,
        width: u32,
        height: u32,
        depth: usize,
    ) -> Result<usize> {
        let right = i32::try_from(width).map_err(|_| {
            WinedroidError::new(format!("runtime View frame width exceeds i32: {width}"))
        })?;
        let bottom = i32::try_from(height).map_err(|_| {
            WinedroidError::new(format!("runtime View frame height exceeds i32: {height}"))
        })?;
        let (roots, request) = {
            let mut runtime = self.runtime.borrow_mut();
            runtime.window_visible_frame = Some((right, bottom));
            let Some(request) = runtime.take_traversal_request() else {
                trace::event(
                    "runtime",
                    "runtime View traversal skipped with no pending request".to_owned(),
                );
                return Ok(0);
            };

            runtime.runtime_canvas_frames.clear();
            (runtime.runtime_view_roots.clone(), request)
        };
        let mut frame_count = 0;
        trace::count(
            "perf",
            "runtime view root frame attempts",
            roots.len() as u64,
        );

        for root in roots {
            let Some(view) = runtime_view_root_value(&root) else {
                continue;
            };

            let canvas = self.allocate_object("Landroid/graphics/Canvas;".to_owned());
            trace::event(
                "runtime",
                format!(
                    "runtime View root frame start class={} view={} layout_requested={} draw_requested={}",
                    root.class_descriptor,
                    view,
                    request.layout_requested,
                    request.draw_requested
                ),
            );
            let geometry = self.runtime_view_root_window_geometry(&view, right, bottom);
            if request.layout_requested {
                let render_metric_start = crate::render_metrics::start_timing();
                let _span = trace::span("runtime", "runtime View measure/layout root");
                let _stat = trace::stat_span("perf", "runtime View measure/layout root");
                self.dispatch_runtime_view_frame(
                    &view,
                    geometry.width_spec,
                    geometry.height_spec,
                    depth,
                )?;
                crate::render_metrics::record_measure_layout_elapsed(render_metric_start);
            }
            if request.draw_requested {
                let render_metric_start = crate::render_metrics::start_timing();
                let _span = trace::span("runtime", "runtime View draw root");
                let _stat = trace::stat_span("perf", "runtime View draw root");
                let view_render_node = self.update_retained_view_render_node(&view, depth)?;
                let frame_width = self.view_measured_dimension_or(&view, "getMeasuredWidth", right);
                let frame_height =
                    self.view_measured_dimension_or(&view, "getMeasuredHeight", bottom);
                let root_render_node = self.update_retained_root_render_node(
                    &view,
                    &view_render_node,
                    frame_width,
                    frame_height,
                );
                self.record_canvas_draw_op(
                    &canvas,
                    CanvasDrawOp::RenderNode {
                        render_node: root_render_node,
                    },
                );
                crate::render_metrics::record_draw_elapsed(render_metric_start);

                self.runtime
                    .borrow_mut()
                    .runtime_canvas_frames
                    .push(RuntimeCanvasFrame {
                        root_view_key: map_identity_key(&view),
                        class_descriptor: root.class_descriptor,
                        x: geometry.x,
                        y: geometry.y,
                        width: frame_width.max(0) as u32,
                        height: frame_height.max(0) as u32,
                        canvas_key: map_identity_key(&canvas),
                    });
                frame_count += 1;
            }
        }

        Ok(frame_count)
    }

    pub(in crate::dex_run::view) fn dispatch_runtime_view_frame(
        &self,
        view: &DexValue,
        width_spec: i32,
        height_spec: i32,
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_INVOCATION_DEPTH {
            return Err(WinedroidError::new(format!(
                "runtime View frame dispatch depth exceeded {MAX_INVOCATION_DEPTH}"
            )));
        }

        trace::count("perf", "runtime view frame visits", 1);

        self.measure_view(view, width_spec, height_spec, depth)?;

        let measured_width = self.view_measured_dimension_or(
            view,
            "getMeasuredWidth",
            measure_spec_size(width_spec),
        );
        let measured_height = self.view_measured_dimension_or(
            view,
            "getMeasuredHeight",
            measure_spec_size(height_spec),
        );

        // cite: frameworks-base View.java#layout, setFrame
        let bounds_changed = self.view_int_field(view, &view_bounds_int_field("left")) != Some(0)
            || self.view_int_field(view, &view_bounds_int_field("top")) != Some(0)
            || self.view_int_field(view, &view_bounds_int_field("right")) != Some(measured_width)
            || self.view_int_field(view, &view_bounds_int_field("bottom")) != Some(measured_height);
        let layout_required = matches!(
            self.read_stored_instance_field(view, &view_layout_required_field()),
            Some(DexValue::Int(value)) if value != 0
        );

        if bounds_changed || layout_required {
            self.write_view_bounds(view, 0, 0, measured_width, measured_height);
            self.mark_view_render_node_dirty(view);
            {
                let _span = trace::span("runtime", "View.onLayout");
                let _stat = trace::stat_span("perf", "runtime View.onLayout");
                self.execute_optional_view_void_method(
                    view,
                    "onLayout",
                    vec![
                        "Z".to_owned(),
                        "I".to_owned(),
                        "I".to_owned(),
                        "I".to_owned(),
                        "I".to_owned(),
                    ],
                    vec![
                        view.clone(),
                        DexValue::Int(i32::from(bounds_changed)),
                        DexValue::Int(0),
                        DexValue::Int(0),
                        DexValue::Int(measured_width),
                        DexValue::Int(measured_height),
                    ],
                    depth,
                )?;
            }
            self.write_stored_instance_field(view, &view_layout_required_field(), DexValue::Int(0));
        }
        self.write_stored_instance_field(view, &view_layout_requested_field(), DexValue::Int(0));

        for child in self.view_group_child_snapshot(view) {
            let _span = trace::span("runtime", "runtime View child measure/layout");
            self.dispatch_runtime_view_frame(
                &child,
                exact_measure_spec(measured_width),
                exact_measure_spec(measured_height),
                depth + 1,
            )?;
        }

        Ok(())
    }

    // cite: frameworks-base View.java#measure, PFLAG_FORCE_LAYOUT, mMeasureCache
    pub(in crate::dex_run) fn measure_view(
        &self,
        view: &DexValue,
        width_spec: i32,
        height_spec: i32,
        depth: usize,
    ) -> Result<()> {
        let force_layout =
            match self.read_stored_instance_field(view, &view_layout_requested_field()) {
                Some(DexValue::Int(value)) => value != 0,
                _ => true,
            };
        let old_width_spec =
            self.view_int_field(view, &view_old_measure_spec_field("oldWidthMeasureSpec"));
        let old_height_spec =
            self.view_int_field(view, &view_old_measure_spec_field("oldHeightMeasureSpec"));
        let specs_changed =
            old_width_spec != Some(width_spec) || old_height_spec != Some(height_spec);
        // cite: frameworks-base View.java#measure
        let is_spec_exactly =
            measure_spec_is_exactly(width_spec) && measure_spec_is_exactly(height_spec);
        let matches_spec_size = self
            .view_int_field(view, &view_measured_width_field())
            .is_some_and(|width| width == measure_spec_size(width_spec))
            && self
                .view_int_field(view, &view_measured_height_field())
                .is_some_and(|height| height == measure_spec_size(height_spec));
        let needs_layout = specs_changed && (!is_spec_exactly || !matches_spec_size);

        if !force_layout && !needs_layout {
            return Ok(());
        }

        let cache_key = ((width_spec as u32 as u64) << 32) | (height_spec as u32 as u64);
        let view_key = map_identity_key(view);
        // cite: frameworks-base View.java#measure
        let cached = if force_layout {
            None
        } else {
            self.runtime
                .borrow()
                .view_measure_caches
                .get(&view_key)
                .and_then(|entries| {
                    entries
                        .iter()
                        .find(|(key, _)| *key == cache_key)
                        .map(|(_, dims)| *dims)
                })
        };

        if let Some(dims) = cached {
            self.write_view_measured_dimensions(view, (dims >> 32) as i32, dims as u32 as i32);
        } else {
            self.write_view_measured_dimensions(
                view,
                measure_spec_size(width_spec),
                measure_spec_size(height_spec),
            );
            let _span = trace::span("runtime", "View.onMeasure");
            let _stat = trace::stat_span("perf", "runtime View.onMeasure");
            self.execute_optional_view_void_method(
                view,
                "onMeasure",
                vec!["I".to_owned(), "I".to_owned()],
                vec![
                    view.clone(),
                    DexValue::Int(width_spec),
                    DexValue::Int(height_spec),
                ],
                depth,
            )?;
        }
        self.write_stored_instance_field(
            view,
            &view_old_measure_spec_field("oldWidthMeasureSpec"),
            DexValue::Int(width_spec),
        );
        self.write_stored_instance_field(
            view,
            &view_old_measure_spec_field("oldHeightMeasureSpec"),
            DexValue::Int(height_spec),
        );
        self.write_stored_instance_field(view, &view_layout_required_field(), DexValue::Int(1));

        // cite: frameworks-base View.java#measure, mMeasureCache
        let measured_width = self
            .view_int_field(view, &view_measured_width_field())
            .unwrap_or_else(|| measure_spec_size(width_spec));
        let measured_height = self
            .view_int_field(view, &view_measured_height_field())
            .unwrap_or_else(|| measure_spec_size(height_spec));
        let dims = ((measured_width as u32 as u64) << 32) | (measured_height as u32 as u64);
        let mut runtime = self.runtime.borrow_mut();
        let entries = runtime.view_measure_caches.entry(view_key).or_default();
        match entries.iter_mut().find(|(key, _)| *key == cache_key) {
            Some(entry) => entry.1 = dims,
            None => entries.push((cache_key, dims)),
        }

        Ok(())
    }

    pub(in crate::dex_run) fn measure_view_with_dispatch(
        &self,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<()>> {
        if invocation.arguments.len() != 3 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return Ok(None);
        }
        let DexValue::Int(width_spec) = invocation.arguments[1] else {
            return Ok(None);
        };
        let DexValue::Int(height_spec) = invocation.arguments[2] else {
            return Ok(None);
        };

        self.measure_view(&invocation.arguments[0], width_spec, height_spec, depth)?;

        Ok(Some(()))
    }

    fn view_int_field(&self, view: &DexValue, field: &DexFieldId) -> Option<i32> {
        match self.read_stored_instance_field(view, field)? {
            DexValue::Int(value) => Some(value),
            _ => None,
        }
    }

    pub(in crate::dex_run::view) fn view_measured_dimension_or(
        &self,
        view: &DexValue,
        getter: &str,
        fallback: i32,
    ) -> i32 {
        let field = match getter {
            "getMeasuredWidth" => view_measured_width_field(),
            "getMeasuredHeight" => view_measured_height_field(),
            _ => return fallback,
        };
        match self.read_stored_instance_field(view, &field) {
            Some(DexValue::Int(value)) if value > 0 => value,
            _ => fallback,
        }
    }

    pub(in crate::dex_run::view) fn runtime_view_root_window_geometry(
        &self,
        view: &DexValue,
        window_width: i32,
        window_height: i32,
    ) -> RuntimeRootGeometry {
        let params = self
            .read_stored_instance_field(view, &view_layout_params_field())
            .filter(|value| dex_value_is_object_like(value));
        let (width, height, x, y) = match &params {
            Some(params) => (
                self.view_int_field(params, &view_group_layout_params_int_field("width"))
                    .unwrap_or(-1),
                self.view_int_field(params, &view_group_layout_params_int_field("height"))
                    .unwrap_or(-1),
                self.view_int_field(params, &window_manager_layout_params_int_field("x"))
                    .unwrap_or(0),
                self.view_int_field(params, &window_manager_layout_params_int_field("y"))
                    .unwrap_or(0),
            ),
            None => (-1, -1, 0, 0),
        };

        RuntimeRootGeometry {
            x,
            y,
            width_spec: root_measure_spec(window_width, width),
            height_spec: root_measure_spec(window_height, height),
        }
    }

    // cite: frameworks-base View.java#invalidate, invalidateInternal, damageInParent
    // cite: frameworks-base ViewRootImpl.java#onDescendantInvalidated, invalidate
    pub(in crate::dex_run::view) fn mark_view_render_node_dirty(&self, view: &DexValue) {
        let mut runtime = self.runtime.borrow_mut();
        let mut current = Some(view.clone());
        let mut depth = 0usize;
        while let Some(current_view) = current {
            if depth > MAX_INVOCATION_DEPTH {
                break;
            }
            let current_key = map_identity_key(&current_view);
            runtime.view_render_node_dirty.insert(current_key.clone());
            current = runtime.view_parents.get(&current_key).cloned();
            depth += 1;
        }
    }

    // cite: frameworks-base RenderNode.java#discardDisplayList
    pub(in crate::dex_run::view) fn discard_view_render_node_subtree(&self, view: &DexValue) {
        let mut pending = vec![view.clone()];
        while let Some(current) = pending.pop() {
            let current_key = map_identity_key(&current);
            let children = self
                .runtime
                .borrow()
                .view_group_children
                .get(&current_key)
                .cloned()
                .unwrap_or_default();
            pending.extend(children);

            let mut runtime = self.runtime.borrow_mut();
            if let Some(render_node) = runtime.view_render_nodes.remove(&current_key) {
                runtime
                    .render_node_draw_ops
                    .remove(&map_identity_key(&render_node));
            }
            if let Some(render_node) = runtime.root_render_nodes.remove(&current_key) {
                runtime
                    .render_node_draw_ops
                    .remove(&map_identity_key(&render_node));
            }
            runtime.view_render_node_dirty.remove(&current_key);
            runtime.view_render_node_generations.remove(&current_key);
            runtime.root_render_node_generations.remove(&current_key);
        }
    }

    fn retained_view_render_node(&self, view: &DexValue) -> DexValue {
        let view_key = map_identity_key(view);
        let mut runtime = self.runtime.borrow_mut();
        if let Some(render_node) = runtime.view_render_nodes.get(&view_key) {
            return render_node.clone();
        }

        let render_node = DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/graphics/RenderNode;:view:{view_key}"
        )));
        runtime
            .view_render_nodes
            .insert(view_key.clone(), render_node.clone());
        runtime.view_render_node_dirty.insert(view_key);
        render_node
    }

    fn retained_root_render_node(&self, view: &DexValue) -> DexValue {
        let view_key = map_identity_key(view);
        let mut runtime = self.runtime.borrow_mut();
        if let Some(render_node) = runtime.root_render_nodes.get(&view_key) {
            return render_node.clone();
        }

        let render_node = DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/graphics/RenderNode;:root:{view_key}"
        )));
        runtime
            .root_render_nodes
            .insert(view_key, render_node.clone());
        render_node
    }

    // cite: frameworks-base RenderNode.java#setPosition
    // cite: frameworks-base View.java#updateDisplayListIfDirty
    fn update_retained_view_render_node_bounds(
        &self,
        view: &DexValue,
        render_node: &DexValue,
    ) -> bool {
        let left = self
            .view_int_field(view, &view_bounds_int_field("left"))
            .unwrap_or(0);
        let top = self
            .view_int_field(view, &view_bounds_int_field("top"))
            .unwrap_or(0);
        let measured_width = self.view_measured_dimension_or(view, "getMeasuredWidth", 0);
        let measured_height = self.view_measured_dimension_or(view, "getMeasuredHeight", 0);
        let right = self
            .view_int_field(view, &view_bounds_int_field("right"))
            .unwrap_or_else(|| left.saturating_add(measured_width));
        let bottom = self
            .view_int_field(view, &view_bounds_int_field("bottom"))
            .unwrap_or_else(|| top.saturating_add(measured_height));
        let mut changed = false;

        for (name, value) in [
            ("positionLeft", left),
            ("positionTop", top),
            ("positionRight", right),
            ("positionBottom", bottom),
        ] {
            let field = render_node_position_field(name);
            if self.read_stored_instance_field(render_node, &field) != Some(DexValue::Int(value)) {
                changed = true;
            }
            self.write_stored_instance_field(render_node, &field, DexValue::Int(value));
        }

        changed
    }

    // cite: frameworks-base View.java#setAlpha, setElevation
    // cite: frameworks-base RenderNode.java#setAlpha, setElevation
    fn update_retained_view_render_node_properties(&self, view: &DexValue, render_node: &DexValue) {
        for (view_field, render_node_field, default) in [
            (
                view_float_field("alpha"),
                render_node_alpha_field(),
                1.0_f32,
            ),
            (
                view_float_field("elevation"),
                render_node_elevation_field(),
                0.0_f32,
            ),
        ] {
            let value = match self.read_stored_instance_field(view, &view_field) {
                Some(DexValue::Float(value)) => value,
                Some(DexValue::Int(bits)) => f32::from_bits(bits as u32),
                _ => default,
            };
            self.write_stored_instance_field(
                render_node,
                &render_node_field,
                DexValue::Float(value),
            );
        }
    }

    // cite: frameworks-base ThreadedRenderer.java#updateRootDisplayList
    fn update_retained_root_render_node_bounds(
        &self,
        render_node: &DexValue,
        width: i32,
        height: i32,
    ) -> bool {
        let mut changed = false;
        for (name, value) in [
            ("positionLeft", 0),
            ("positionTop", 0),
            ("positionRight", width.max(0)),
            ("positionBottom", height.max(0)),
        ] {
            let field = render_node_position_field(name);
            if self.read_stored_instance_field(render_node, &field) != Some(DexValue::Int(value)) {
                changed = true;
            }
            self.write_stored_instance_field(render_node, &field, DexValue::Int(value));
        }

        changed
    }

    fn retained_view_render_node_needs_recording(
        &self,
        view: &DexValue,
        render_node: &DexValue,
        bounds_changed: bool,
    ) -> bool {
        let view_key = map_identity_key(view);
        let render_node_key = map_identity_key(render_node);
        let view_invalidated = matches!(
            self.read_stored_instance_field(view, &view_invalidated_field()),
            Some(DexValue::Int(value)) if value != 0
        );
        let runtime = self.runtime.borrow();
        bounds_changed
            || view_invalidated
            || runtime.view_render_node_dirty.contains(&view_key)
            || !runtime.render_node_draw_ops.contains_key(&render_node_key)
    }

    fn retained_root_render_node_needs_recording(
        &self,
        render_node: &DexValue,
        bounds_changed: bool,
    ) -> bool {
        let render_node_key = map_identity_key(render_node);
        let runtime = self.runtime.borrow();
        bounds_changed || !runtime.render_node_draw_ops.contains_key(&render_node_key)
    }

    // cite: frameworks-base View.java#updateDisplayListIfDirty, draw
    // cite: frameworks-base RenderNode.java#beginRecording, endRecording, hasDisplayList
    fn record_retained_view_render_node(
        &self,
        view: &DexValue,
        render_node: &DexValue,
        depth: usize,
    ) -> Result<()> {
        let view_key = map_identity_key(view);
        let render_node_key = map_identity_key(render_node);
        let recording_canvas = DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/graphics/RecordingCanvas;:view:{view_key}"
        )));
        let recording_canvas_key = map_identity_key(&recording_canvas);

        self.write_stored_instance_field(
            &recording_canvas,
            &recording_canvas_render_node_field(),
            render_node.clone(),
        );
        self.write_stored_instance_field(
            render_node,
            &render_node_recording_canvas_field(),
            recording_canvas.clone(),
        );
        self.write_stored_instance_field(view, &view_invalidated_field(), DexValue::Int(0));
        {
            let mut runtime = self.runtime.borrow_mut();
            runtime
                .canvas_draw_ops
                .insert(recording_canvas_key.clone(), Vec::new());
            runtime.view_render_node_dirty.remove(&view_key);
        }

        self.dispatch_runtime_view_draw_contents(view, &recording_canvas, depth)?;

        let draw_ops = self
            .runtime
            .borrow()
            .canvas_draw_ops
            .get(&recording_canvas_key)
            .cloned()
            .unwrap_or_default();
        {
            let mut runtime = self.runtime.borrow_mut();
            runtime
                .render_node_draw_ops
                .insert(render_node_key, draw_ops);
            *runtime
                .view_render_node_generations
                .entry(view_key)
                .or_insert(0) += 1;
        }
        self.write_stored_instance_field(
            render_node,
            &render_node_recording_canvas_field(),
            DexValue::Null,
        );

        Ok(())
    }

    // cite: frameworks-base ThreadedRenderer.java#updateRootDisplayList
    // cite: frameworks-base HardwareRenderer.java#setContentRoot
    fn record_retained_root_render_node(
        &self,
        view: &DexValue,
        root_render_node: &DexValue,
        view_render_node: &DexValue,
    ) {
        let view_key = map_identity_key(view);
        let root_render_node_key = map_identity_key(root_render_node);
        let draw_ops = vec![CanvasDrawOp::RenderNode {
            render_node: view_render_node.clone(),
        }];
        let mut runtime = self.runtime.borrow_mut();
        runtime
            .render_node_draw_ops
            .insert(root_render_node_key, draw_ops);
        *runtime
            .root_render_node_generations
            .entry(view_key)
            .or_insert(0) += 1;
    }

    fn update_retained_view_render_node(&self, view: &DexValue, depth: usize) -> Result<DexValue> {
        let render_node = self.retained_view_render_node(view);
        let bounds_changed = self.update_retained_view_render_node_bounds(view, &render_node);
        self.update_retained_view_render_node_properties(view, &render_node);
        if self.retained_view_render_node_needs_recording(view, &render_node, bounds_changed) {
            self.record_retained_view_render_node(view, &render_node, depth)?;
        }

        Ok(render_node)
    }

    fn update_retained_root_render_node(
        &self,
        view: &DexValue,
        view_render_node: &DexValue,
        width: i32,
        height: i32,
    ) -> DexValue {
        let render_node = self.retained_root_render_node(view);
        let bounds_changed =
            self.update_retained_root_render_node_bounds(&render_node, width, height);
        if self.retained_root_render_node_needs_recording(&render_node, bounds_changed) {
            self.record_retained_root_render_node(view, &render_node, view_render_node);
        }

        render_node
    }

    pub(in crate::dex_run) fn dispatch_runtime_view_draw(
        &self,
        view: &DexValue,
        canvas: &DexValue,
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_INVOCATION_DEPTH {
            return Err(WinedroidError::new(format!(
                "runtime View draw dispatch depth exceeded {MAX_INVOCATION_DEPTH}"
            )));
        }

        let render_node = self.update_retained_view_render_node(view, depth)?;
        self.record_canvas_draw_op(
            canvas,
            CanvasDrawOp::RenderNode {
                render_node: render_node.clone(),
            },
        );

        Ok(())
    }

    fn dispatch_runtime_view_draw_contents(
        &self,
        view: &DexValue,
        canvas: &DexValue,
        depth: usize,
    ) -> Result<()> {
        {
            let _span = trace::span("runtime", "View.draw");
            let _stat = trace::stat_span("perf", "runtime View.draw");
            if self.execute_optional_view_void_method(
                view,
                "draw",
                vec!["Landroid/graphics/Canvas;".to_owned()],
                vec![view.clone(), canvas.clone()],
                depth,
            )? {
                return Ok(());
            }
        }

        {
            let _span = trace::span("runtime", "View.onDraw");
            let _stat = trace::stat_span("perf", "runtime View.onDraw");
            self.execute_optional_view_void_method(
                view,
                "onDraw",
                vec!["Landroid/graphics/Canvas;".to_owned()],
                vec![view.clone(), canvas.clone()],
                depth,
            )?;
        }
        {
            let _span = trace::span("runtime", "View.dispatchDraw");
            let _stat = trace::stat_span("perf", "runtime View.dispatchDraw");
            self.execute_optional_view_void_method(
                view,
                "dispatchDraw",
                vec!["Landroid/graphics/Canvas;".to_owned()],
                vec![view.clone(), canvas.clone()],
                depth,
            )?;
        }

        for child in self.view_group_child_snapshot(view) {
            let _span = trace::span("runtime", "runtime View child draw");
            self.dispatch_runtime_view_draw(&child, canvas, depth + 1)?;
        }

        Ok(())
    }

    pub(in crate::dex_run::view) fn execute_optional_view_void_method(
        &self,
        view: &DexValue,
        method_name: &str,
        parameters: Vec<String>,
        arguments: Vec<DexValue>,
        depth: usize,
    ) -> Result<bool> {
        let Some(class_descriptor) = value_class_descriptor(view) else {
            return Ok(false);
        };
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: method_name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters,
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(class_descriptor, &method)?
        else {
            return Ok(false);
        };
        let method_metric_start = crate::method_metrics::start_timing();
        let started_at = std::time::Instant::now();
        let result = self.execute_method(None, &resolved, Some(&arguments), depth + 1)?;
        let elapsed = started_at.elapsed();
        crate::method_metrics::record_render_callback_elapsed(method_metric_start);
        if trace::enabled("perf") && elapsed >= std::time::Duration::from_millis(25) {
            trace::event(
                "perf",
                format!(
                    "slow View lifecycle method view_class={class_descriptor} resolved_class={} method={} depth={} took {:.2}ms",
                    resolved.class_descriptor,
                    resolved.method.display_signature(),
                    depth,
                    elapsed.as_secs_f64() * 1000.0
                ),
            );
        }

        match result {
            DexExecutionResult::Void => return Ok(true),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                return Err(WinedroidError::new(format!(
                    "runtime View method parked without a guest-thread scheduler\nclass: {class_descriptor}\nmethod: {}",
                    method.display_signature()
                )));
            }
            DexExecutionResult::Int(_)
            | DexExecutionResult::Value(_)
            | DexExecutionResult::Throw(_) => {}
        }

        Err(WinedroidError::new(format!(
            "runtime View method returned a value\nclass: {class_descriptor}\nmethod: {}\nresult: {result}",
            method.display_signature()
        )))
    }

    // cite: frameworks-base ViewTreeObserver.java#dispatchOnPreDraw
    pub(in crate::dex_run) fn dispatch_view_tree_pre_draw_listeners(
        &self,
        depth: usize,
    ) -> Result<bool> {
        let listeners = self.view_tree_pre_draw_listener_snapshot();
        let mut cancel_draw = false;

        for listener in listeners {
            cancel_draw |= !self.dispatch_single_view_tree_pre_draw_listener(listener, depth)?;
        }

        Ok(cancel_draw)
    }

    pub(in crate::dex_run::view) fn view_tree_pre_draw_listener_snapshot(&self) -> Vec<DexValue> {
        self.runtime
            .borrow()
            .view_tree_pre_draw_listeners
            .values()
            .flat_map(|listeners| listeners.iter().cloned())
            .collect()
    }

    pub(in crate::dex_run::view) fn dispatch_single_view_tree_pre_draw_listener(
        &self,
        listener: DexValue,
        depth: usize,
    ) -> Result<bool> {
        let Some(listener_descriptor) = value_class_descriptor(&listener) else {
            return Err(WinedroidError::new(format!(
                "ViewTreeObserver pre-draw listener dispatch requires an object-backed listener\nlistener: {listener}"
            )));
        };

        let on_pre_draw_method = DexMethodId {
            class_descriptor: listener_descriptor.to_owned(),
            name: "onPreDraw".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "Z".to_owned(),
                return_type: "Z".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(listener_descriptor, &on_pre_draw_method)?
        else {
            return Err(WinedroidError::new(format!(
                "ViewTreeObserver pre-draw listener method was not found\nlistener class: {listener_descriptor}\nnext missing primitive: ViewTreeObserver.OnPreDrawListener dispatch"
            )));
        };

        let method_metric_start = crate::method_metrics::start_timing();
        let result = self.execute_method(None, &resolved, Some(&[listener]), depth + 1)?;
        crate::method_metrics::record_render_callback_elapsed(method_metric_start);

        match result {
            DexExecutionResult::Int(value) => Ok(value != 0),
            DexExecutionResult::Void
            | DexExecutionResult::Value(_)
            | DexExecutionResult::Throw(_) => Err(WinedroidError::new(
                "ViewTreeObserver pre-draw listener onPreDraw did not complete normally",
            )),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(
                    "ViewTreeObserver pre-draw listener onPreDraw did not complete normally",
                ))
            }
        }
    }

    pub(in crate::dex_run::view) fn view_tree_draw_listener_snapshot(&self) -> Vec<DexValue> {
        self.runtime
            .borrow()
            .view_tree_draw_listeners
            .values()
            .flat_map(|listeners| listeners.iter().cloned())
            .collect()
    }

    pub(in crate::dex_run::view) fn view_tree_global_layout_listener_snapshot(
        &self,
    ) -> Vec<DexValue> {
        self.runtime
            .borrow()
            .view_tree_global_layout_listeners
            .values()
            .flat_map(|listeners| listeners.iter().cloned())
            .collect()
    }

    pub(in crate::dex_run::view) fn dispatch_single_view_tree_draw_listener(
        &self,
        listener: DexValue,
        depth: usize,
    ) -> Result<()> {
        let Some(listener_descriptor) = value_class_descriptor(&listener) else {
            return Err(WinedroidError::new(format!(
                "ViewTreeObserver draw listener dispatch requires an object-backed listener\nlistener: {listener}"
            )));
        };

        let on_draw_method = DexMethodId {
            class_descriptor: listener_descriptor.to_owned(),
            name: "onDraw".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(listener_descriptor, &on_draw_method)?
        else {
            return Err(WinedroidError::new(format!(
                "ViewTreeObserver draw listener method was not found\nlistener class: {listener_descriptor}\nnext missing primitive: ViewTreeObserver.OnDrawListener dispatch"
            )));
        };

        let method_metric_start = crate::method_metrics::start_timing();
        let result = self.execute_method(None, &resolved, Some(&[listener]), depth + 1)?;
        crate::method_metrics::record_render_callback_elapsed(method_metric_start);

        match result {
            DexExecutionResult::Void => Ok(()),
            DexExecutionResult::Int(_)
            | DexExecutionResult::Value(_)
            | DexExecutionResult::Throw(_) => Err(WinedroidError::new(
                "ViewTreeObserver draw listener onDraw did not complete normally",
            )),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(
                    "ViewTreeObserver draw listener onDraw did not complete normally",
                ))
            }
        }
    }

    pub(in crate::dex_run::view) fn dispatch_single_view_tree_global_layout_listener(
        &self,
        listener: DexValue,
        depth: usize,
    ) -> Result<()> {
        let Some(listener_descriptor) = value_class_descriptor(&listener) else {
            return Err(WinedroidError::new(format!(
                "ViewTreeObserver global layout listener dispatch requires an object-backed listener\nlistener: {listener}"
            )));
        };

        let on_global_layout_method = DexMethodId {
            class_descriptor: listener_descriptor.to_owned(),
            name: "onGlobalLayout".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) = self.resolve_exact_method_in_class_hierarchy(
            listener_descriptor,
            &on_global_layout_method,
        )?
        else {
            return Err(WinedroidError::new(format!(
                "ViewTreeObserver global layout listener method was not found\nlistener class: {listener_descriptor}\nnext missing primitive: ViewTreeObserver.OnGlobalLayoutListener dispatch"
            )));
        };

        let method_metric_start = crate::method_metrics::start_timing();
        let result = self.execute_method(None, &resolved, Some(&[listener]), depth + 1)?;
        crate::method_metrics::record_render_callback_elapsed(method_metric_start);

        match result {
            DexExecutionResult::Void => Ok(()),
            DexExecutionResult::Int(_)
            | DexExecutionResult::Value(_)
            | DexExecutionResult::Throw(_) => Err(WinedroidError::new(
                "ViewTreeObserver global layout listener onGlobalLayout did not complete normally",
            )),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(
                    "ViewTreeObserver global layout listener onGlobalLayout did not complete normally",
                ))
            }
        }
    }
}
