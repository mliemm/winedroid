use super::helpers::*;
use super::*;

const WINDOW_MANAGER_LAYOUT_PARAMS_TYPE_APPLICATION: i32 = 2;
const WINDOW_MANAGER_LAYOUT_PARAMS_FORMAT_OPAQUE: i32 = -1;

impl DexProgram {
    pub(in crate::dex_run) fn init_window_manager_layout_params(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let receiver = &invocation.arguments[0];
        let wrote_width = self.write_stored_instance_field(
            receiver,
            &view_group_layout_params_int_field("width"),
            DexValue::Int(-1),
        );
        let wrote_height = self.write_stored_instance_field(
            receiver,
            &view_group_layout_params_int_field("height"),
            DexValue::Int(-1),
        );
        let wrote_type = self.write_stored_instance_field(
            receiver,
            &window_manager_layout_params_int_field("type"),
            DexValue::Int(WINDOW_MANAGER_LAYOUT_PARAMS_TYPE_APPLICATION),
        );
        let wrote_format = self.write_stored_instance_field(
            receiver,
            &window_manager_layout_params_int_field("format"),
            DexValue::Int(WINDOW_MANAGER_LAYOUT_PARAMS_FORMAT_OPAQUE),
        );

        (wrote_width && wrote_height && wrote_type && wrote_format).then_some(())
    }

    pub(in crate::dex_run) fn window_manager_layout_params_set_title(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let title = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::String(std::rc::Rc::from(String::new())),
            DexValue::String(_) | DexValue::Object(_) | DexValue::Reference(_) => {
                invocation.arguments[1].clone()
            }
            DexValue::Uninitialized | DexValue::Int(_) | DexValue::Long(_) | DexValue::Float(_) => {
                return None
            }
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &window_manager_layout_params_title_field(),
            title,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn window_manager_layout_params_title(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &window_manager_layout_params_title_field(),
            )
            .unwrap_or(DexValue::String(std::rc::Rc::from(String::new()))),
        )
    }

    pub(in crate::dex_run) fn read_window_color(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = self
            .read_stored_instance_field(&invocation.arguments[0], field)
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(color) = value else {
            return None;
        };

        Some(color)
    }

    pub(in crate::dex_run) fn write_window_color(
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

        let DexValue::Int(color) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(&invocation.arguments[0], field, DexValue::Int(color))
            .then_some(())
    }

    pub(in crate::dex_run) fn read_window_callback(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            self.read_stored_instance_field(&invocation.arguments[0], &window_callback_field())
                .unwrap_or(DexValue::Reference(std::rc::Rc::from(
                    "Landroid/view/Window$Callback;.bootstrap",
                ))),
        )
    }

    pub(in crate::dex_run) fn write_window_callback(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::Null,
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        self.write_stored_instance_field(&invocation.arguments[0], &window_callback_field(), value)
            .then_some(())
    }

    pub(in crate::dex_run) fn window_layout_params(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        if let Some(value) =
            self.read_stored_instance_field(&invocation.arguments[0], &window_layout_params_field())
        {
            if dex_value_is_object_like(&value) {
                return Some(value);
            }

            return None;
        }

        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/view/WindowManager$LayoutParams;:window:{}",
            map_identity_key(&invocation.arguments[0])
        ))))
    }

    pub(in crate::dex_run) fn write_window_layout_params(
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
            &window_layout_params_field(),
            invocation.arguments[1].clone(),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_window_boolean(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = self
            .read_stored_instance_field(&invocation.arguments[0], field)
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(boolean_value) = value else {
            return None;
        };

        Some(i32::from(boolean_value != 0))
    }

    pub(in crate::dex_run) fn write_window_boolean(
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

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            field,
            DexValue::Int(i32::from(value != 0)),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn window_insets_controller(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/view/WindowInsetsController;:window:{}",
            map_identity_key(&invocation.arguments[0])
        ))))
    }

    pub(in crate::dex_run) fn read_window_insets_controller_system_bars_appearance(
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
                &window_insets_controller_system_bars_appearance_field(),
            )
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(appearance) = value else {
            return None;
        };

        Some(appearance)
    }

    pub(in crate::dex_run) fn write_window_insets_controller_system_bars_appearance(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(appearance) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(mask) = invocation.arguments[2] else {
            return None;
        };

        let current = self
            .read_stored_instance_field(
                &invocation.arguments[0],
                &window_insets_controller_system_bars_appearance_field(),
            )
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(current) = current else {
            return None;
        };
        let next = (current & !mask) | (appearance & mask);

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &window_insets_controller_system_bars_appearance_field(),
            DexValue::Int(next),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_popup_window_background(
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
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &popup_window_background_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_popup_window_background(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = match &invocation.arguments[1] {
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
            &popup_window_background_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_popup_window_overlap_anchor(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        // cite: frameworks-base PopupWindow.java#setOverlapAnchor
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &popup_window_overlap_anchor_field(),
            DexValue::Int(i32::from(value != 0)),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn popup_window_max_available_height(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if !(2..=4).contains(&invocation.arguments.len())
            || !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }
        let y_offset = match invocation.arguments.get(2) {
            Some(DexValue::Int(value)) => *value,
            Some(_) => return None,
            None => 0,
        };
        let ignore_bottom_decorations = match invocation.arguments.get(3) {
            Some(DexValue::Int(value)) => *value != 0,
            Some(_) => return None,
            None => false,
        };
        let background = self
            .read_stored_instance_field(&invocation.arguments[0], &popup_window_background_field())
            .unwrap_or(DexValue::Null);
        if background != DexValue::Null && background != DexValue::Int(0) {
            return None;
        }

        // cite: frameworks-base PopupWindow.java#getMaxAvailableHeight
        let display_size = self.display_size();
        let visible_height = self
            .runtime
            .borrow()
            .window_visible_frame
            .map(|(_, height)| height.max(0))
            .unwrap_or(display_size.height_i32());
        let bottom_edge = if ignore_bottom_decorations {
            display_size.height_i32()
        } else {
            visible_height
        };
        let (_, anchor_y) = self.view_location_on_screen(&invocation.arguments[1])?;
        let anchor_height = self
            .read_view_bound(&invocation.arguments[1], "bottom")
            .wrapping_sub(self.read_view_bound(&invocation.arguments[1], "top"));
        let overlap_anchor = matches!(
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &popup_window_overlap_anchor_field(),
            ),
            Some(DexValue::Int(value)) if value != 0
        );
        let distance_to_bottom = if overlap_anchor {
            bottom_edge.wrapping_sub(anchor_y).wrapping_sub(y_offset)
        } else {
            bottom_edge
                .wrapping_sub(anchor_y.wrapping_add(anchor_height))
                .wrapping_sub(y_offset)
        };
        let distance_to_top = anchor_y.wrapping_add(y_offset);

        Some(distance_to_bottom.max(distance_to_top))
    }

    pub(in crate::dex_run) fn read_popup_window_input_method_mode(
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
            .read_stored_instance_field(
                &invocation.arguments[0],
                &popup_window_input_method_mode_field(),
            )
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_popup_window_input_method_mode(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &popup_window_input_method_mode_field(),
            DexValue::Int(value),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_popup_window_window_layout_type(
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
            .read_stored_instance_field(
                &invocation.arguments[0],
                &popup_window_window_layout_type_field(),
            )
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_popup_window_window_layout_type(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &popup_window_window_layout_type_field(),
            DexValue::Int(value),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_popup_window_is_clipped_to_screen(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &popup_window_is_clipped_to_screen_field(),
            DexValue::Int(if value == 0 { 0 } else { 1 }),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_popup_window_epicenter_bounds(
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
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &popup_window_epicenter_bounds_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_popup_window_epicenter_bounds(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let bounds = match &invocation.arguments[1] {
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
            &popup_window_epicenter_bounds_field(),
            bounds,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_popup_window_content_view(
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
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &popup_window_content_view_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_popup_window_content_view(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let content_view = match &invocation.arguments[1] {
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
            &popup_window_content_view_field(),
            content_view,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn inflate_popup_menu(&self, invocation: &DexInvocation) -> Option<()> {
        let popup = invocation.arguments.first()?;
        let DexValue::Int(resource_id) = invocation.arguments.get(1)? else {
            return None;
        };
        let runtime_menu = self.parse_menu_resource(*resource_id).ok()?;
        let menu_ref = popup_menu_menu_reference(&map_identity_key(popup));
        self.runtime
            .borrow_mut()
            .menus
            .insert(map_identity_key(&menu_ref), runtime_menu);
        Some(())
    }

    pub(in crate::dex_run) fn show_popup_menu(&self, invocation: &DexInvocation) -> Option<()> {
        let popup = invocation.arguments.first()?;
        let popup_key = map_identity_key(popup);
        let menu_ref = popup_menu_menu_reference(&popup_key);
        let menu = self
            .runtime
            .borrow()
            .menus
            .get(&map_identity_key(&menu_ref))?
            .clone();
        let items = menu
            .items
            .iter()
            .filter(|item| item.visible)
            .enumerate()
            .map(|(position, item)| RuntimeMenuOverlayItem {
                item_id: item.item_id,
                title: item.title.clone(),
                enabled: item.enabled,
                position: position as i32,
                on_click: item.on_click.clone(),
            })
            .collect();
        let anchor_view_id = self
            .read_stored_instance_field(popup, &popup_menu_anchor_field())
            .and_then(|anchor| self.view_id_from_value(&anchor));
        let menu_item_click_listener =
            self.read_stored_instance_field(popup, &popup_menu_click_listener_field());

        let overlay = RuntimeMenuOverlay {
            popup_key: popup_key.clone(),
            anchor_view_id,
            list_view: popup.clone(),
            items,
            menu_item_click_listener,
        };
        let mut runtime = self.runtime.borrow_mut();
        runtime
            .active_menu_overlays
            .retain(|active| active.popup_key != popup_key);
        runtime.active_menu_overlays.push(overlay);
        Some(())
    }

    pub(in crate::dex_run) fn dismiss_popup_menu(&self, invocation: &DexInvocation) -> Option<()> {
        let popup_key = map_identity_key(invocation.arguments.first()?);
        self.runtime
            .borrow_mut()
            .active_menu_overlays
            .retain(|active| active.popup_key != popup_key);
        Some(())
    }

    // cite: frameworks-base PopupWindow.java#dismiss
    pub(in crate::dex_run) fn dismiss_top_menu_overlay(&self) -> bool {
        let overlay = self.runtime.borrow_mut().active_menu_overlays.pop();
        let Some(overlay) = overlay else {
            return false;
        };
        let signature = self.field_signature(&popup_window_showing_field());
        self.runtime
            .borrow_mut()
            .set_field(&overlay.popup_key, &signature, DexValue::Int(0));
        true
    }

    // cite: frameworks-base PopupWindow.java#dismiss
    pub(in crate::dex_run) fn dismiss_menu_overlay_containing_item(&self, item_id: i32) -> bool {
        let signature = self.field_signature(&popup_window_showing_field());
        let mut runtime = self.runtime.borrow_mut();
        let Some(index) = runtime
            .active_menu_overlays
            .iter()
            .rposition(|overlay| overlay.items.iter().any(|item| item.item_id == item_id))
        else {
            return false;
        };
        let overlay = runtime.active_menu_overlays.remove(index);
        runtime.set_field(&overlay.popup_key, &signature, DexValue::Int(0));
        true
    }

    pub(in crate::dex_run) fn show_popup_window(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() < 2 {
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

        let popup_key = map_identity_key(&invocation.arguments[0]);
        let Some(overlay) = self.popup_window_overlay(invocation, &popup_key) else {
            return Some(());
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &popup_window_showing_field(),
            DexValue::Int(1),
        );
        let mut runtime = self.runtime.borrow_mut();
        runtime
            .active_menu_overlays
            .retain(|active| active.popup_key != popup_key);
        runtime.active_menu_overlays.push(overlay);

        Some(())
    }

    pub(in crate::dex_run) fn dismiss_popup_window(
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

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &popup_window_showing_field(),
            DexValue::Int(0),
        );
        let popup_key = map_identity_key(&invocation.arguments[0]);
        self.runtime
            .borrow_mut()
            .active_menu_overlays
            .retain(|active| active.popup_key != popup_key);

        Some(())
    }

    pub(in crate::dex_run) fn popup_window_is_showing(
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

        match self
            .read_stored_instance_field(&invocation.arguments[0], &popup_window_showing_field())
        {
            Some(DexValue::Int(value)) => Some(i32::from(value != 0)),
            Some(_) | None => Some(0),
        }
    }

    pub(in crate::dex_run) fn popup_window_overlay(
        &self,
        invocation: &DexInvocation,
        popup_key: &str,
    ) -> Option<RuntimeMenuOverlay> {
        let content_view = self.read_stored_instance_field(
            &invocation.arguments[0],
            &popup_window_content_view_field(),
        )?;
        let adapter =
            self.read_stored_instance_field(&content_view, &adapter_view_adapter_field())?;
        let menu_key = self.menu_key_for_adapter(&adapter)?;
        let menu = self.runtime.borrow().menus.get(&menu_key)?.clone();
        let items = menu
            .items
            .iter()
            .filter(|item| item.visible)
            .enumerate()
            .map(|(position, item)| RuntimeMenuOverlayItem {
                item_id: item.item_id,
                title: item.title.clone(),
                enabled: item.enabled,
                position: position as i32,
                on_click: item.on_click.clone(),
            })
            .collect();

        Some(RuntimeMenuOverlay {
            popup_key: popup_key.to_owned(),
            anchor_view_id: self.view_id_from_value(&invocation.arguments[1]),
            list_view: content_view,
            items,
            menu_item_click_listener: None,
        })
    }

    pub(in crate::dex_run) fn menu_key_for_adapter(&self, adapter: &DexValue) -> Option<String> {
        let adapter_key = instance_field_owner_key(adapter)?;
        let runtime = self.runtime.borrow();

        for ((owner_key, _field), value) in runtime.iter_fields() {
            if !owner_key.matches_key(&adapter_key) {
                continue;
            }

            let value_key = map_identity_key(value);
            if runtime.menus.contains_key(&value_key) {
                return Some(value_key);
            }
        }

        None
    }

    pub(in crate::dex_run) fn read_abs_list_view_cache_color_hint(
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
            .read_stored_instance_field(
                &invocation.arguments[0],
                &abs_list_view_cache_color_hint_field(),
            )
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_abs_list_view_cache_color_hint(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &abs_list_view_cache_color_hint_field(),
            DexValue::Int(value),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_abs_list_view_scroll_listener(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let listener = match &invocation.arguments[1] {
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
            &abs_list_view_scroll_listener_field(),
            listener,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn horizontal_scroll_view_full_scroll(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
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

        let DexValue::Int(direction) = invocation.arguments[1] else {
            return None;
        };
        let scroll_x = match direction {
            17 => 0,
            66 => i32::MAX,
            _ => return None,
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &horizontal_scroll_view_scroll_x_field(),
            DexValue::Int(scroll_x),
        )
        .then_some(1)
    }

    pub(in crate::dex_run) fn read_list_view_divider_height(
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
            .read_stored_instance_field(&invocation.arguments[0], &list_view_divider_height_field())
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_list_view_divider_height(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &list_view_divider_height_field(),
            DexValue::Int(value),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_list_view_divider(
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
            self.read_stored_instance_field(&invocation.arguments[0], &list_view_divider_field())
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_list_view_divider(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let divider = match &invocation.arguments[1] {
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
            &list_view_divider_field(),
            divider,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_adapter_data_set_observer(
        &self,
        invocation: &DexInvocation,
        method_name: &str,
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
        let observer = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::Null,
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        if method_name == "registerDataSetObserver" {
            return self
                .write_stored_instance_field(
                    &invocation.arguments[0],
                    &adapter_data_set_observer_field(),
                    observer,
                )
                .then_some(());
        }

        let registered = self
            .read_stored_instance_field(
                &invocation.arguments[0],
                &adapter_data_set_observer_field(),
            )
            .unwrap_or(DexValue::Null);
        if registered == observer {
            return self
                .write_stored_instance_field(
                    &invocation.arguments[0],
                    &adapter_data_set_observer_field(),
                    DexValue::Null,
                )
                .then_some(());
        }

        Some(())
    }

    pub(in crate::dex_run) fn read_adapter_view_adapter(
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
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &adapter_view_adapter_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_adapter_view_adapter(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let adapter = match &invocation.arguments[1] {
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
            &adapter_view_adapter_field(),
            adapter,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_adapter_view_item_click_listener(
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
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &adapter_view_item_click_listener_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_adapter_view_item_click_listener(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let listener = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::Null,
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        if let (Some(view_id), Some(activity)) = (
            self.view_id_from_value(&invocation.arguments[0]),
            self.active_activity_receiver(),
        ) {
            self.runtime
                .borrow_mut()
                .adapter_item_click_listeners
                .insert((map_identity_key(&activity), view_id), listener.clone());
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &adapter_view_item_click_listener_field(),
            listener,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_adapter_view_item_selected_listener(
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
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &adapter_view_item_selected_listener_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_adapter_view_item_selected_listener(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let listener = match &invocation.arguments[1] {
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
            &adapter_view_item_selected_listener_field(),
            listener,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_adapter_view_selected_position(
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
            .read_stored_instance_field(
                &invocation.arguments[0],
                &adapter_view_selected_position_field(),
            )
            .unwrap_or(DexValue::Int(-1));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_adapter_view_selected_position(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &adapter_view_selected_position_field(),
            DexValue::Int(value),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_image_view_drawable(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = match &invocation.arguments[1] {
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
            &image_view_drawable_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_image_view_resource(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(resource_id) = invocation.arguments[1] else {
            return None;
        };
        let value = if resource_id == 0 {
            DexValue::Null
        } else {
            self.drawable_resource_value(resource_id as u32)
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &image_view_drawable_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_toolbar_navigation_icon(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }
        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::Null,
            DexValue::Int(resource_id) => DexValue::Reference(std::rc::Rc::from(format!(
                "Landroid/graphics/drawable/Drawable;:resource:{resource_id}"
            ))),
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => {
                return None;
            }
        };

        if let DexValue::Int(resource_id) = &invocation.arguments[1] {
            self.runtime.borrow_mut().action_bar_nav_icon = match *resource_id {
                0 => None,
                resource_id => Some(resource_id),
            };
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &toolbar_navigation_icon_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_image_view_drawable(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            self.read_stored_instance_field(&invocation.arguments[0], &image_view_drawable_field())
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn read_image_view_tint_list(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &image_view_tint_list_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_image_view_tint_list(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = match &invocation.arguments[1] {
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
            &image_view_tint_list_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_image_view_tint_mode(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &image_view_tint_mode_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_image_view_tint_mode(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = match &invocation.arguments[1] {
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
            &image_view_tint_mode_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_view_content_description(
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
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &view_content_description_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_view_content_description(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = text_view_text_value(&invocation.arguments[1])?;

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_content_description_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_view_tooltip_text(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            self.read_stored_instance_field(&invocation.arguments[0], &view_tooltip_text_field())
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_view_tooltip_text(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = text_view_text_value(&invocation.arguments[1])?;

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_tooltip_text_field(),
            value,
        )
        .then_some(())
    }

    // cite: libcore ThreadLocal.java#get, setInitialValue
    pub(in crate::dex_run) fn read_thread_local_value(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let receiver = &invocation.arguments[0];

        match receiver {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        if let Some(stored) = self.read_stored_instance_field(receiver, &thread_local_value_field())
        {
            return Some(DexExecutionResult::Value(stored));
        }

        let initial = match self.thread_local_initial_value(receiver)? {
            DexExecutionResult::Value(value) => value,
            DexExecutionResult::Int(value) => DexValue::Int(value),
            DexExecutionResult::Void => DexValue::Null,
            throw @ DexExecutionResult::Throw(_) => return Some(throw),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                return None;
            }
        };

        self.write_stored_instance_field(receiver, &thread_local_value_field(), initial.clone())
            .then_some(DexExecutionResult::Value(initial))
    }

    // cite: libcore ThreadLocal.java#initialValue
    fn thread_local_initial_value(&self, receiver: &DexValue) -> Option<DexExecutionResult> {
        let class_descriptor = value_class_descriptor(receiver)?;
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "initialValue".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "L".to_owned(),
                return_type: "Ljava/lang/Object;".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) = self
            .resolve_exact_method_in_class_hierarchy(class_descriptor, &method)
            .ok()?
        else {
            return Some(DexExecutionResult::Value(DexValue::Null));
        };

        self.execute_method(None, &resolved, Some(&[receiver.clone()]), 0)
            .ok()
    }

    pub(in crate::dex_run) fn write_thread_local_value(
        &self,
        invocation: &DexInvocation,
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

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &thread_local_value_field(),
            invocation.arguments[1].clone(),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn remove_thread_local_value(
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

        self.remove_stored_instance_field(&invocation.arguments[0], &thread_local_value_field())
            .then_some(())
    }
}
