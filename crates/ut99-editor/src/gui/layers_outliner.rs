// ==============================================================================
// Layers & Scene Outliner (01 / Photoshop-Style Scene Hierarchy)
// ==============================================================================
// Provides a unified, Photoshop-style hierarchy of all scene elements:
// - Red Builder Brush (with dedicated toggle and visibility)
// - World CSG Brushes (Add, Subtract, SemiSolid, NonSolid)
// - Movers (Doors & Lifts with Keyframe tracking)
// - Static Meshes
// - Actors (Lights, PlayerStarts, PathNodes, Inventory, Zones, Triggers)
//
// Features:
// - Eye icon (👁 / 🚫) to show/hide items from 3D/2D viewports & raycast picking.
// - Lock icon (🔒 / 🔓) to prevent accidental transforms or edits.
// - Top Action Bar: Show/Hide All, Unlock All, Duplicate (Ctrl+D), Delete (Del), Focus (F), Deselect.
// - Fast Search Filter and Category Chips.
// - Single-click select, Ctrl+click multi-toggle, Shift+click contiguous range select.
// - Rich right-click context menu.
// ==============================================================================

use egui::{Color32, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use crate::editor_state::{EditorState, InspectorPanelMode, OutlinerCategory};
use crate::gui::icons::{context_menu_item, context_menu_item_danger, paint_icon, Icon};
use crate::gui::theme::SemanticPalette;
use crate::gui::widgets;
use crate::history::Action;

fn eye_toggle(ui: &mut egui::Ui, is_vis: bool, tooltip: &str) -> egui::Response {
    let palette = SemanticPalette::current(ui.ctx());
    let size = Vec2::splat(24.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            ui.is_enabled(),
            is_vis,
            if is_vis { "Visible (click to hide)" } else { "Hidden (click to show)" },
        )
    });
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(rect, 3.0, palette.bg_widget_hover);
        }
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect,
                3.0,
                Stroke::new(2.0_f32, palette.border_focus),
                StrokeKind::Inside,
            );
        }
        let (ico, col) = if is_vis {
            (
                Icon::EyeVisible,
                if resp.hovered() { palette.text_primary } else { palette.accent_azure },
            )
        } else {
            (
                Icon::EyeHidden,
                if resp.hovered() { palette.text_secondary } else { palette.text_muted },
            )
        };
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(15.0));
        paint_icon(ui.painter(), icon_rect, ico, col);
    }
    resp.on_hover_text(tooltip)
}

fn lock_toggle(ui: &mut egui::Ui, is_locked: bool, tooltip: &str) -> egui::Response {
    let palette = SemanticPalette::current(ui.ctx());
    let size = Vec2::splat(24.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            ui.is_enabled(),
            is_locked,
            if is_locked { "Locked (click to unlock)" } else { "Unlocked (click to lock)" },
        )
    });
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(rect, 3.0, palette.bg_widget_hover);
        }
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect,
                3.0,
                Stroke::new(2.0_f32, palette.border_focus),
                StrokeKind::Inside,
            );
        }
        let (ico, col) = if is_locked {
            (
                Icon::LockClosed,
                if resp.hovered() { palette.text_primary } else { palette.accent_gold },
            )
        } else {
            (
                Icon::LockOpen,
                if resp.hovered() { palette.text_secondary } else { palette.text_muted },
            )
        };
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(15.0));
        paint_icon(ui.painter(), icon_rect, ico, col);
    }
    resp.on_hover_text(tooltip)
}

/// Identifies an element in the unified Layers list for range selections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerItemRef {
    Builder,
    Brush(usize),
    Mesh(usize),
    Mover(usize),
    Actor(usize),
}

#[derive(Clone)]
struct OutlinerRangeAnchor {
    row: usize,
    item: LayerItemRef,
    name: String,
    counts: [usize; 4],
}

impl LayerItemRef {
    fn name(self, state: &EditorState) -> Option<&str> {
        match self {
            Self::Builder => Some("ActiveBuilder"),
            Self::Brush(i) => state.world_brushes.get(i).map(|b| b.name.as_str()),
            Self::Mesh(i) => state.static_meshes.get(i).map(|m| m.name.as_str()),
            Self::Mover(i) => state.movers.get(i).map(|m| m.name.as_str()),
            Self::Actor(i) => state.loaded_unr.as_ref()?.actors.get(i).map(|a| a.name.as_str()),
        }
    }
}

fn csg_oper_label(op: ut99_bsp::CsgOperation) -> &'static str {
    match op {
        ut99_bsp::CsgOperation::Active => "Active",
        ut99_bsp::CsgOperation::Add => "Add",
        ut99_bsp::CsgOperation::Subtract => "Subtract",
        ut99_bsp::CsgOperation::Intersect => "Intersect",
        ut99_bsp::CsgOperation::Deintersect => "Deintersect",
    }
}

fn outliner_row_height(ui: &egui::Ui) -> f32 {
    24.0_f32.max(ui.spacing().interact_size.y)
        .max(ui.text_style_height(&egui::TextStyle::Button) + 2.0 * ui.spacing().button_padding.y)
}

fn row_label(ui: &mut egui::Ui, selected: bool, label: String, color: Color32, strong: bool, actions: usize) -> egui::Response {
    let action_width = ui.spacing().interact_size.x + ui.spacing().item_spacing.x;
    let width = (ui.available_width() - action_width * actions as f32).max(1.0);
    let height = outliner_row_height(ui);
    ui.allocate_ui_with_layout(Vec2::new(width, height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let text = RichText::new(&label).color(color);
        ui.selectable_label(selected, if strong { text.strong() } else { text }).on_hover_text(label)
    }).inner
}

pub fn render_layers_outliner(ui: &mut egui::Ui, state: &mut EditorState) {
    let palette = SemanticPalette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.heading("Layers & Scene Elements");
    });

    let _has_unr = state.loaded_unr.is_some();
    let actor_count = state.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
    let total_elements = 1 // Builder
        + state.world_brushes.len()
        + state.static_meshes.len()
        + state.movers.len()
        + actor_count;
    let selected_count = state.total_selected_count()
        + if state.is_builder_selected { 1 } else { 0 }
        + if state.selected_mover_index.is_some() { 1 } else { 0 }
        + if state.selected_static_mesh_index.is_some() { 1 } else { 0 };

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Total: {} items", total_elements)).size(11.0).color(palette.text_secondary));
        if selected_count > 0 {
            ui.separator();
            ui.label(RichText::new(format!("Selected: {}", selected_count)).size(11.0).color(palette.accent_azure).strong());
        }
    });

    ui.separator();

    // 1. Top Quick Actions Toolbar (Photoshop-style batch operations)
    let mut batch_duplicate_req = false;
    let mut batch_delete_req = false;
    let mut focus_requested = false;

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        if ui.button("Show All").on_hover_text("Make all elements visible in viewports").clicked() {
            state.show_all_elements();
        }
        if ui.button("Hide All").on_hover_text("Hide all elements from viewports").clicked() {
            state.hide_all_elements();
        }
        if ui.button("Unlock All").on_hover_text("Unlock all locked elements").clicked() {
            state.unlock_all_elements();
        }

        ui.separator();

        if ui.button("Duplicate").on_hover_text("Duplicate selected elements (Ctrl+D)").clicked() {
            batch_duplicate_req = true;
        }

        let del_btn = ui.button(RichText::new("Delete").color(palette.error));
        if del_btn.on_hover_text("Delete selected elements (Del)").clicked() {
            batch_delete_req = true;
        }

        if ui.button("Focus").on_hover_text("Frame selection with camera (F)").clicked() {
            focus_requested = true;
        }

        if ui.button("Deselect").on_hover_text("Deselect all (Esc)").clicked() {
            state.deselect_all();
        }
    });

    ui.add_space(4.0);

    // 2. Filter & Search Controls (Vector Search Icon + 24x24px Clear Button)
    let previous_category = state.outliner_category;
    let filter_changed = widgets::search_filter_bar(ui, &mut state.outliner_filter, "Filter scene hierarchy...");

    // 3. Category Chips
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let categories = [
            (OutlinerCategory::All, "All"),
            (OutlinerCategory::Brushes, "Brushes"),
            (OutlinerCategory::Movers, "Movers"),
            (OutlinerCategory::Lights, "Lights"),
            (OutlinerCategory::Navigation, "Nav"),
            (OutlinerCategory::Pickups, "Items"),
        ];
        for (cat, label) in categories {
            let is_active = state.outliner_category == cat;
            if ui.selectable_label(is_active, label).clicked() {
                state.outliner_category = cat;
            }
        }
    });

    ui.separator();

    let actor_rev = state.loaded_unr.as_ref().map_or(0, |l| l.actors.rev());
    let cur_cat = state.outliner_category;
    let cache_key = (
        state.map_instance_id,
        state.world_brushes.rev(),
        state.static_meshes.rev(),
        state.movers.rev(),
        actor_rev,
        cur_cat,
    );
    let filter_differs = state.outliner_cache.filter != state.outliner_filter;
    if filter_changed || previous_category != cur_cat || filter_differs {
        state.outliner_last_clicked_row = None;
    }

    let mut snap_floor_req = false;
    let mut delete_brush_req: Option<usize> = None;
    let mut delete_mesh_req: Option<usize> = None;
    let mut delete_mover_req: Option<usize> = None;
    let mut delete_actor_req: Option<usize> = None;
    let mut switch_mover_key_req: Option<(usize, usize)> = None;

    // Rebuild the ordered flat list only when scene revisions, category, or filter text change.
    if state.outliner_cache.key != Some(cache_key) || filter_differs {
        let filter_trimmed = state.outliner_filter.trim();
        let filter_lower = if filter_trimmed.is_empty() {
            String::new()
        } else {
            filter_trimmed.to_lowercase()
        };
        let filter_ascii = filter_lower.is_ascii();
        let matches_query = |name: &str, extra: &str| -> bool {
            if filter_lower.is_empty() {
                true
            } else if filter_ascii {
                crate::renderer::overlay_cache::contains_ascii_icase(name, &filter_lower)
                    || crate::renderer::overlay_cache::contains_ascii_icase(extra, &filter_lower)
            } else {
                name.to_lowercase().contains(&filter_lower)
                    || extra.to_lowercase().contains(&filter_lower)
            }
        };

        state.outliner_cache.items.clear();

        // Add Builder Brush if category allows
        if (cur_cat == OutlinerCategory::All || cur_cat == OutlinerCategory::Brushes)
            && matches_query("Red Builder Brush", "ActiveBuilder")
        {
            state.outliner_cache.items.push(LayerItemRef::Builder);
        }

        // Add Brushes
        if cur_cat == OutlinerCategory::All || cur_cat == OutlinerCategory::Brushes {
            for (i, b) in state.world_brushes.iter().enumerate() {
                if matches_query(&b.name, csg_oper_label(b.csg_oper)) {
                    state.outliner_cache.items.push(LayerItemRef::Brush(i));
                }
            }
        }

        // Add Static Meshes
        if cur_cat == OutlinerCategory::All || cur_cat == OutlinerCategory::Brushes {
            for (i, m) in state.static_meshes.iter().enumerate() {
                if matches_query(&m.name, "StaticMesh") {
                    state.outliner_cache.items.push(LayerItemRef::Mesh(i));
                }
            }
        }

        // Add Movers
        if cur_cat == OutlinerCategory::All || cur_cat == OutlinerCategory::Movers {
            for (i, m) in state.movers.iter().enumerate() {
                if matches_query(&m.name, "Mover") {
                    state.outliner_cache.items.push(LayerItemRef::Mover(i));
                }
            }
        }

        // Add Actors
        if let Some(ref level) = state.loaded_unr {
            for (i, actor) in level.actors.iter().enumerate() {
                if actor.class_name.contains("Brush") {
                    continue;
                }
                let actor_cat = if actor.class_name.contains("Light") {
                    Some(OutlinerCategory::Lights)
                } else if actor.class_name.contains("PathNode")
                    || actor.class_name.contains("PlayerStart")
                    || actor.class_name.contains("Teleporter")
                    || actor.class_name.contains("Lift")
                {
                    Some(OutlinerCategory::Navigation)
                } else if actor.class_name.contains("Weapon")
                    || actor.class_name.contains("Ammo")
                    || actor.class_name.contains("Pickup")
                    || actor.class_name.contains("Health")
                    || actor.class_name.contains("Armor")
                    || actor.class_name.contains("Shield")
                    || actor.class_name.contains("UDamage")
                    || actor.class_name.contains("JumpBoots")
                    || actor.class_name.contains("Inventory")
                {
                    Some(OutlinerCategory::Pickups)
                } else {
                    None
                };

                let cat_ok = match cur_cat {
                    OutlinerCategory::All => true,
                    _ => actor_cat == Some(cur_cat),
                };

                if cat_ok && matches_query(&actor.name, &actor.class_name) {
                    state.outliner_cache.items.push(LayerItemRef::Actor(i));
                }
            }
        }

        state.outliner_cache.key = Some(cache_key);
        state.outliner_cache.filter.clear();
        state.outliner_cache.filter.push_str(&state.outliner_filter);
    }

    let flat_items = std::mem::take(&mut state.outliner_cache.items);

    // Edits from the viewport, shortcuts, and Undo can invalidate positional anchors.
    let anchor_id = ui.make_persistent_id("layers_outliner_range_anchor");
    let counts = [state.world_brushes.len(), state.static_meshes.len(), state.movers.len(), actor_count];
    if let Some(row) = state.outliner_last_clicked_row {
        let anchor = ui.ctx().data(|data| data.get_temp::<OutlinerRangeAnchor>(anchor_id));
        let valid = anchor.as_ref().is_some_and(|anchor| {
            anchor.row == row && anchor.counts == counts
                && flat_items.get(row) == Some(&anchor.item)
                && anchor.item.name(state) == Some(anchor.name.as_str())
        });
        if !valid {
            state.outliner_last_clicked_row = None;
        }
    }

    // 4. Scrollable Outliner List (adaptive full height or capped split height)
    let outliner_scroll = if state.inspector_panel_mode == InspectorPanelMode::OutlinerOnly {
        egui::ScrollArea::vertical().id_salt("layers_outliner_scroll").auto_shrink([false, false])
    } else {
        egui::ScrollArea::vertical().id_salt("layers_outliner_scroll").max_height(280.0).auto_shrink([false, false])
    };

    let ctrl_down = ui.input(|inp| inp.modifiers.ctrl);
    let shift_down = ui.input(|inp| inp.modifiers.shift);

    let mut clicked_item_idx: Option<usize> = None;

    let row_height = outliner_row_height(ui);
    outliner_scroll.show_rows(ui, row_height, flat_items.len().max(1), |ui, visible_rows| {
        if flat_items.is_empty() {
            ui.label(RichText::new("No matching elements found.").italics().color(Color32::GRAY));
            return;
        }

        for row_idx in visible_rows {
            let item_ref = flat_items[row_idx];
            let rect = Rect::from_min_size(ui.next_widget_position(), Vec2::new(ui.available_width(), row_height));
            ui.scope_builder(egui::UiBuilder::new().id_salt(("layer_item", item_ref)).max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
                ui.set_min_height(row_height);
                ui.spacing_mut().item_spacing.x = 4.0;

                match item_ref {
                    LayerItemRef::Builder => {
                        // Eye (Visibility)
                        let is_vis = state.show_builder_brush;
                        if eye_toggle(ui, is_vis, if is_vis { "Hide Builder Brush" } else { "Show Builder Brush" }).clicked() {
                            state.toggle_builder_visibility();
                        }

                        // Lock
                        let is_locked = state.is_builder_locked;
                        if lock_toggle(ui, is_locked, if is_locked { "Unlock Builder Brush" } else { "Lock Builder Brush" }).clicked() {
                            state.toggle_builder_lock();
                        }

                        // Selectable Label
                        let is_sel = state.is_builder_selected;
                        let label = format!("🟥 [Active] Red Builder Brush ({:.0}, {:.0}, {:.0})", state.builder_brush.location.x, state.builder_brush.location.y, state.builder_brush.location.z);
                        let text_color = if is_vis { Color32::from_rgb(255, 120, 120) } else { Color32::from_rgb(120, 70, 70) };
                        let res = row_label(ui, is_sel, label, text_color, true, 1);

                        if res.clicked() {
                            clicked_item_idx = Some(row_idx);
                        }

                        // Quick Focus
                        if ui.small_button("🎯").on_hover_text("Focus camera on Builder Brush").clicked() {
                            state.is_builder_selected = true;
                            state.camera.position = state.builder_brush.location - state.camera.forward() * 350.0;
                        }
                    }

                    LayerItemRef::Brush(i) => {
                        let is_vis = state.is_brush_visible(i);
                        if eye_toggle(ui, is_vis, if is_vis { "Hide Brush" } else { "Show Brush" }).clicked() {
                            state.toggle_brush_visibility(i);
                        }

                        let is_locked = state.is_brush_locked(i);
                        if lock_toggle(ui, is_locked, if is_locked { "Unlock Brush" } else { "Lock Brush" }).clicked() {
                            state.toggle_brush_lock(i);
                        }

                        let is_sel = state.is_brush_selected(i);
                        let b = &state.world_brushes[i];
                        let op_icon = match b.csg_oper {
                            ut99_bsp::CsgOperation::Add => "🟦 Add",
                            ut99_bsp::CsgOperation::Subtract => "🟧 Sub",
                            _ => "Brush",
                        };
                        let solidity = state.get_brush_solidity(i);
                        let solid_badge = match solidity {
                            crate::editor_state::BrushSolidity::SemiSolid => " [SemiSolid]",
                            crate::editor_state::BrushSolidity::NonSolid => " [NonSolid]",
                            crate::editor_state::BrushSolidity::Solid => "",
                        };
                        let backdrop_badge = if state.is_brush_fake_backdrop(i) { " [Backdrop]" } else { "" };
                        let label = format!("[{}] {} ({}){}{}", i, b.name, op_icon, solid_badge, backdrop_badge);
                        let text_color = if is_vis { Color32::WHITE } else { Color32::from_rgb(110, 115, 125) };

                        let res = row_label(ui, is_sel, label, text_color, false, 2);

                        if res.clicked() {
                            clicked_item_idx = Some(row_idx);
                        }
                        if res.double_clicked() {
                            state.selected_brush_index = Some(i);
                            state.show_surface_properties = true;
                        }

                        // Action buttons
                        if ui.small_button("🎯").on_hover_text("Frame brush with camera (F)").clicked() {
                            state.selected_brush_index = Some(i);
                            focus_requested = true;
                        }
                        if ui.small_button("📋").on_hover_text("Duplicate brush (Ctrl+D)").clicked() {
                            state.selected_brush_index = Some(i);
                            state.duplicate_selected();
                        }

                        res.context_menu(|ui| {
                            if context_menu_item(ui, None, "Focus Camera", Some("F"), None).clicked() {
                                state.selected_brush_index = Some(i);
                                focus_requested = true;
                                ui.close_menu();
                            }
                            if context_menu_item(ui, Some(Icon::Copy), "Duplicate", Some("Ctrl+D"), None).clicked() {
                                state.selected_brush_index = Some(i);
                                state.duplicate_selected();
                                ui.close_menu();
                            }
                            if context_menu_item(ui, Some(Icon::AlignFloor), "Snap to Floor", Some("End"), None).clicked() {
                                state.selected_brush_index = Some(i);
                                snap_floor_req = true;
                                ui.close_menu();
                            }
                            if context_menu_item(ui, Some(Icon::AlignGrid), "Snap to Grid", Some("Ctrl+Alt+G"), None).clicked() {
                                state.selected_brush_index = Some(i);
                                state.snap_selected_to_grid();
                                ui.close_menu();
                            }
                            ui.separator();
                            if context_menu_item(ui, Some(Icon::SurfaceProperties), "Surface Properties...", Some("F5"), None).clicked() {
                                state.selected_brush_index = Some(i);
                                state.show_surface_properties = true;
                                ui.close_menu();
                            }
                            if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Brush", Some("Del")).clicked() {
                                delete_brush_req = Some(i);
                                ui.close_menu();
                            }
                        });
                    }

                    LayerItemRef::Mesh(i) => {
                        let is_vis = state.is_static_mesh_visible(i);
                        if eye_toggle(ui, is_vis, if is_vis { "Hide Static Mesh" } else { "Show Static Mesh" }).clicked() {
                            state.toggle_static_mesh_visibility(i);
                        }

                        let is_locked = state.is_static_mesh_locked(i);
                        if lock_toggle(ui, is_locked, if is_locked { "Unlock Static Mesh" } else { "Lock Static Mesh" }).clicked() {
                            state.toggle_static_mesh_lock(i);
                        }

                        let is_sel = state.selected_static_mesh_index == Some(i) && !state.has_multi_selection();
                        let m = &state.static_meshes[i];
                        let label = format!("🗿 [{}] {} ({} polys)", i, m.name, m.polys.len());
                        let text_color = if is_vis { Color32::from_rgb(140, 240, 160) } else { Color32::from_rgb(90, 130, 100) };

                        let res = row_label(ui, is_sel, label, text_color, false, 2);

                        if res.clicked() {
                            clicked_item_idx = Some(row_idx);
                        }

                        if ui.small_button("🎯").on_hover_text("Frame mesh with camera (F)").clicked() {
                            state.selected_static_mesh_index = Some(i);
                            focus_requested = true;
                        }
                        if ui.small_button("📋").on_hover_text("Duplicate mesh (Ctrl+D)").clicked() {
                            state.selected_static_mesh_index = Some(i);
                            state.duplicate_selected();
                        }

                        res.context_menu(|ui| {
                            if context_menu_item(ui, None, "Focus Camera", Some("F"), None).clicked() {
                                state.selected_static_mesh_index = Some(i);
                                focus_requested = true;
                                ui.close_menu();
                            }
                            if context_menu_item(ui, Some(Icon::Copy), "Duplicate", Some("Ctrl+D"), None).clicked() {
                                state.selected_static_mesh_index = Some(i);
                                state.duplicate_selected();
                                ui.close_menu();
                            }
                            ui.separator();
                            if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Mesh", Some("Del")).clicked() {
                                delete_mesh_req = Some(i);
                                ui.close_menu();
                            }
                        });
                    }

                    LayerItemRef::Mover(i) => {
                        let is_vis = state.is_mover_visible(i);
                        if eye_toggle(ui, is_vis, if is_vis { "Hide Mover" } else { "Show Mover" }).clicked() {
                            state.toggle_mover_visibility(i);
                        }

                        let is_locked = state.is_mover_locked(i);
                        if lock_toggle(ui, is_locked, if is_locked { "Unlock Mover" } else { "Lock Mover" }).clicked() {
                            state.toggle_mover_lock(i);
                        }

                        let is_sel = state.selected_mover_index == Some(i) && !state.has_multi_selection();
                        let m = &state.movers[i];
                        let label = format!("🚪 [{}] {} (Key {})", i, m.name, m.current_key);
                        let text_color = if is_vis { Color32::from_rgb(255, 140, 255) } else { Color32::from_rgb(130, 80, 130) };

                        let res = row_label(ui, is_sel, label, text_color, false, 2);

                        if res.clicked() {
                            clicked_item_idx = Some(row_idx);
                        }

                        if ui.small_button("🎯").on_hover_text("Frame mover with camera (F)").clicked() {
                            state.selected_mover_index = Some(i);
                            focus_requested = true;
                        }
                        if ui.small_button("📋").on_hover_text("Duplicate mover (Ctrl+D)").clicked() {
                            state.selected_mover_index = Some(i);
                            state.duplicate_selected();
                        }

                        res.context_menu(|ui| {
                            if context_menu_item(ui, None, "Focus Camera", Some("F"), None).clicked() {
                                state.selected_mover_index = Some(i);
                                focus_requested = true;
                                ui.close_menu();
                            }
                            if context_menu_item(ui, Some(Icon::Copy), "Duplicate", Some("Ctrl+D"), None).clicked() {
                                state.selected_mover_index = Some(i);
                                state.duplicate_selected();
                                ui.close_menu();
                            }
                            ui.separator();
                            ui.menu_button("Switch Keyframe (0..7)", |ui| {
                                if let Some(m) = state.movers.get(i) {
                                    for k in 0..8 {
                                        let is_cur = m.current_key == k;
                                        let text = if is_cur {
                                            RichText::new(format!("*Key {k} (Active)*")).color(Color32::from_rgb(255, 220, 50))
                                        } else {
                                            RichText::new(format!("Key {k}"))
                                        };
                                        if ui.button(text).clicked() {
                                            switch_mover_key_req = Some((i, k));
                                            ui.close_menu();
                                        }
                                    }
                                }
                            });
                            if context_menu_item(ui, None, "Next Keyframe", None, None).clicked() {
                                state.selected_mover_index = Some(i);
                                state.cycle_selected_mover_key(true);
                                ui.close_menu();
                            }
                            if context_menu_item(ui, None, "Previous Keyframe", None, None).clicked() {
                                state.selected_mover_index = Some(i);
                                state.cycle_selected_mover_key(false);
                                ui.close_menu();
                            }
                            ui.separator();
                            if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Mover", Some("Del")).clicked() {
                                delete_mover_req = Some(i);
                                ui.close_menu();
                            }
                        });
                    }

                    LayerItemRef::Actor(i) => {
                        let is_vis = state.is_actor_visible(i);
                        if eye_toggle(ui, is_vis, if is_vis { "Hide Actor" } else { "Show Actor" }).clicked() {
                            state.toggle_actor_visibility(i);
                        }

                        let is_locked = state.is_actor_locked(i);
                        if lock_toggle(ui, is_locked, if is_locked { "Unlock Actor" } else { "Lock Actor" }).clicked() {
                            state.toggle_actor_lock(i);
                        }

                        let is_sel = state.is_actor_selected(i);
                        let Some(unr) = state.loaded_unr.as_ref() else { return; };
                        let Some(a) = unr.actors.get(i) else { return; };
                        let icon = if a.class_name.contains("Light") {
                            "💡"
                        } else if a.class_name.contains("PlayerStart") {
                            "🚩"
                        } else if a.class_name.contains("PathNode") {
                            "🔷"
                        } else if a.class_name.contains("Weapon") || a.class_name.contains("Ammo") {
                            "🔫"
                        } else if a.class_name.contains("SkyZoneInfo") {
                            "🌌"
                        } else if a.class_name.contains("WaterZone") {
                            "🌊"
                        } else if a.class_name.contains("ZoneInfo") {
                            "🌐"
                        } else {
                            "⚙"
                        };
                        let label = format!("{} {} [{}]", icon, a.name, a.class_name);
                        let text_color = if is_vis { Color32::from_rgb(220, 230, 245) } else { Color32::from_rgb(110, 115, 125) };

                        let res = row_label(ui, is_sel, label, text_color, false, 2);

                        if res.clicked() {
                            clicked_item_idx = Some(row_idx);
                        }
                        if res.double_clicked() {
                            state.selected_actor_index = Some(i);
                            state.show_actor_properties = true;
                        }

                        if ui.small_button("🎯").on_hover_text("Frame actor with camera (F)").clicked() {
                            state.selected_actor_index = Some(i);
                            focus_requested = true;
                        }
                        if ui.small_button("📋").on_hover_text("Duplicate actor (Ctrl+D)").clicked() {
                            state.selected_actor_index = Some(i);
                            state.duplicate_selected();
                        }

                        res.context_menu(|ui| {
                            if context_menu_item(ui, Some(Icon::ActorProperties), "Actor Properties...", Some("Enter"), None).clicked() {
                                state.selected_actor_index = Some(i);
                                state.show_actor_properties = true;
                                ui.close_menu();
                            }
                            if context_menu_item(ui, None, "Focus Camera", Some("F"), None).clicked() {
                                state.selected_actor_index = Some(i);
                                focus_requested = true;
                                ui.close_menu();
                            }
                            if context_menu_item(ui, Some(Icon::Copy), "Duplicate", Some("Ctrl+D"), None).clicked() {
                                state.selected_actor_index = Some(i);
                                state.duplicate_selected();
                                ui.close_menu();
                            }
                            if context_menu_item(ui, Some(Icon::AlignFloor), "Snap to Floor", Some("End"), None).clicked() {
                                state.selected_actor_index = Some(i);
                                snap_floor_req = true;
                                ui.close_menu();
                            }
                            if context_menu_item(ui, Some(Icon::AlignGrid), "Snap to Grid", Some("Ctrl+Alt+G"), None).clicked() {
                                state.selected_actor_index = Some(i);
                                state.snap_selected_to_grid();
                                ui.close_menu();
                            }
                            ui.separator();
                            if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Actor", Some("Del")).clicked() {
                                delete_actor_req = Some(i);
                                ui.close_menu();
                            }
                        });
                    }
                }
            });
        }
    });

    // 5. Handle Click / Range Selection logic
    if let Some(target_idx) = clicked_item_idx {
        if let (true, Some(last_row)) = (shift_down, state.outliner_last_clicked_row) {
            let start = last_row.min(target_idx);
            let end = last_row.max(target_idx);

            for idx in start..=end {
                if let Some(item) = flat_items.get(idx) {
                    match *item {
                        LayerItemRef::Builder => { state.is_builder_selected = true; }
                        LayerItemRef::Brush(b) => { state.multi_selected_brushes.insert(b); }
                        LayerItemRef::Actor(a) => { state.multi_selected_actors.insert(a); }
                        LayerItemRef::Mesh(s) => { state.selected_static_mesh_index = Some(s); }
                        LayerItemRef::Mover(m) => { state.selected_mover_index = Some(m); }
                    }
                }
            }
        } else if ctrl_down {
            // Toggle item in multi-selection
            if let Some(item) = flat_items.get(target_idx) {
                match *item {
                    LayerItemRef::Builder => { state.is_builder_selected = !state.is_builder_selected; }
                    LayerItemRef::Brush(b) => {
                        if state.multi_selected_brushes.contains(&b) {
                            state.multi_selected_brushes.remove(&b);
                        } else {
                            state.multi_selected_brushes.insert(b);
                        }
                        state.selected_brush_index = state.multi_selected_brushes.iter().next().copied();
                    }
                    LayerItemRef::Actor(a) => {
                        if state.multi_selected_actors.contains(&a) {
                            state.multi_selected_actors.remove(&a);
                        } else {
                            state.multi_selected_actors.insert(a);
                        }
                        state.selected_actor_index = state.multi_selected_actors.iter().next().copied();
                    }
                    LayerItemRef::Mesh(s) => {
                        state.selected_static_mesh_index = if state.selected_static_mesh_index == Some(s) { None } else { Some(s) };
                    }
                    LayerItemRef::Mover(m) => {
                        state.selected_mover_index = if state.selected_mover_index == Some(m) { None } else { Some(m) };
                    }
                }
            }
        } else {
            // Plain single select: clear others
            state.deselect_all();
            if let Some(item) = flat_items.get(target_idx) {
                match *item {
                    LayerItemRef::Builder => { state.is_builder_selected = true; }
                    LayerItemRef::Brush(b) => {
                        state.selected_brush_index = Some(b);
                        state.multi_selected_brushes.insert(b);
                    }
                    LayerItemRef::Actor(a) => {
                        state.selected_actor_index = Some(a);
                        state.multi_selected_actors.insert(a);
                    }
                    LayerItemRef::Mesh(s) => { state.selected_static_mesh_index = Some(s); }
                    LayerItemRef::Mover(m) => { state.selected_mover_index = Some(m); }
                }
            }
        }
        state.outliner_last_clicked_row = Some(target_idx);
        if let Some(&item) = flat_items.get(target_idx) {
            let anchor = OutlinerRangeAnchor {
                row: target_idx, item, name: item.name(state).unwrap_or_default().to_string(), counts,
            };
            ui.ctx().data_mut(|data| data.insert_temp(anchor_id, anchor));
        }
    }

    state.outliner_cache.items = flat_items;

    // 6. Handle Batch Actions & Execution Requests
    if focus_requested {
        state.focus_camera_on_selection();
    }
    if batch_duplicate_req {
        state.duplicate_selected();
    }
    if batch_delete_req {
        state.delete_selected_batch();
    }
    if snap_floor_req {
        state.snap_selected_to_floor();
    }
    if let Some((m_idx, k)) = switch_mover_key_req {
        state.set_mover_key(m_idx, k);
    }
    if let Some(i) = delete_brush_req {
        if i < state.world_brushes.len() {
            state.outliner_last_clicked_row = None;
            let b = state.world_brushes.remove(i);
            state.history.push(Action::DeleteBrush { index: i, brush: b });
            state.selected_brush_index = None;
            state.map_is_dirty = true;
        }
    }
    if let Some(i) = delete_mesh_req {
        if i < state.static_meshes.len() {
            state.outliner_last_clicked_row = None;
            let m = state.static_meshes.remove(i);
            state.history.push(Action::DeleteStaticMesh { index: i, mesh: m });
            state.selected_static_mesh_index = None;
            state.map_is_dirty = true;
        }
    }
    if let Some(i) = delete_mover_req {
        if i < state.movers.len() {
            state.outliner_last_clicked_row = None;
            let mv = state.movers.remove(i);
            state.history.push(Action::DeleteMover { index: i, mover: mv });
            state.selected_mover_index = None;
            state.map_is_dirty = true;
        }
    }
    if let Some(i) = delete_actor_req {
        if let Some(ref mut level) = state.loaded_unr {
            if i < level.actors.len() {
                state.outliner_last_clicked_row = None;
                let a = level.actors.remove(i);
                state.history.push(Action::DeleteActor { index: i, actor: a });
                state.selected_actor_index = None;
                state.map_is_dirty = true;
            }
        }
    }
}


#[cfg(test)]
mod virtualization_tests {
    use super::*;
    use egui::{Context, Event, FullOutput, Id, Modifiers, PointerButton, Pos2};

    fn scene(count: usize) -> EditorState {
        let mut state = EditorState::default();
        state.loaded_unr = None;
        state.world_brushes = (0..count).map(|i| {
            let mut brush = state.builder_brush.clone();
            brush.name = format!("TestBrush{i:04}");
            brush
        }).collect();
        state
    }

    fn frame(ctx: &Context, state: &mut EditorState, events: Vec<Event>, modifiers: Modifiers, width: f32) -> (FullOutput, Id) {
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 720.0)));
        input.time = Some(ctx.input(|i| i.time) + 0.2);
        input.events = events;
        input.modifiers = modifiers;
        let mut scroll_id = Id::NULL;
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                scroll_id = ui.make_persistent_id(Id::new("layers_outliner_scroll"));
                render_layers_outliner(ui, state);
            });
        });
        (output, scroll_id)
    }

    fn text_rect(output: &FullOutput, needle: &str) -> Rect {
        output.shapes.iter().find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape {
                if text.galley.text().contains(needle) {
                    let rect = Rect::from_min_size(text.pos, text.galley.size()).intersect(shape.clip_rect);
                    if rect.is_positive() { return Some(rect); }
                }
            }
            None
        }).unwrap_or_else(|| panic!("Visible text not found: {needle}; painted labels: {:?}", output.shapes.iter().filter_map(|s| {
            if let egui::epaint::Shape::Text(t) = &s.shape { Some(t.galley.text()) } else { None }
        }).collect::<Vec<_>>()))
    }

    fn click(ctx: &Context, state: &mut EditorState, pos: Pos2, modifiers: Modifiers) {
        for pressed in [true, false] {
            frame(ctx, state, vec![Event::PointerMoved(pos), Event::PointerButton {
                pos, button: PointerButton::Primary, pressed, modifiers,
            }], modifiers, 500.0);
        }
    }

    fn scroll(ctx: &Context, id: Id, offset: f32) {
        let mut saved = egui::scroll_area::State::load(ctx, id).unwrap();
        saved.offset.y = offset;
        saved.store(ctx, id);
    }

    #[test]
    fn only_visible_rows_render_and_last_row_is_reachable() {
        let ctx = Context::default();
        let mut state = scene(4000);
        let (_, id) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let rendered = output.shapes.iter().filter(|shape| matches!(&shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.text().contains("TestBrush"))).count();
        assert!(rendered > 0 && rendered < 30, "Rendered {rendered} brush labels");
        scroll(&ctx, id, 1_000_000.0);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let pos = text_rect(&output, "TestBrush3999").center();
        click(&ctx, &mut state, pos, Modifiers::NONE);
        assert_eq!(state.selected_brush_index, Some(3999));
    }

    #[test]
    fn shift_and_ctrl_selection_keep_global_indices_across_scroll() {
        let ctx = Context::default();
        let mut state = scene(300);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, id) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        click(&ctx, &mut state, text_rect(&output, "TestBrush0001").center(), Modifiers::NONE);
        assert_eq!(state.selected_brush_index, Some(1));
        let stride = 24.0 + ctx.style().spacing.item_spacing.y;
        scroll(&ctx, id, 100.0 * stride);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let target = text_rect(&output, "TestBrush0102").center();
        click(&ctx, &mut state, target, Modifiers { shift: true, ..Modifiers::NONE });
        assert_eq!(state.multi_selected_brushes.len(), 102);
        assert!((1..=102).all(|i| state.multi_selected_brushes.contains(&i)));
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let target = text_rect(&output, "TestBrush0102").center();
        click(&ctx, &mut state, target, Modifiers { ctrl: true, ..Modifiers::NONE });
        assert!(!state.multi_selected_brushes.contains(&102));
        assert_eq!(state.multi_selected_brushes.len(), 101);
    }

    #[test]
    fn filter_and_category_changes_reset_range_anchor() {
        let ctx = Context::default();
        let mut state = scene(300);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, id) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        click(&ctx, &mut state, text_rect(&output, "TestBrush0001").center(), Modifiers::NONE);
        assert!(state.outliner_last_clicked_row.is_some());
        click(&ctx, &mut state, text_rect(&output, "Filter scene hierarchy").center(), Modifiers::NONE);
        frame(&ctx, &mut state, vec![Event::Text("TestBrush02".into())], Modifiers::NONE, 500.0);
        assert_eq!(state.outliner_filter, "TestBrush02");
        assert_eq!(state.outliner_last_clicked_row, None);
        scroll(&ctx, id, 0.0);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        click(&ctx, &mut state, text_rect(&output, "TestBrush0200").center(), Modifiers::NONE);
        assert_eq!(state.selected_brush_index, Some(200));
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        click(&ctx, &mut state, text_rect(&output, "Lights").center(), Modifiers::NONE);
        assert_eq!(state.outliner_category, OutlinerCategory::Lights);
        assert_eq!(state.outliner_last_clicked_row, None);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        text_rect(&output, "No matching elements found");
    }

    #[test]
    fn a_pressed_row_does_not_transfer_its_click_when_scrolled_away() {
        let ctx = Context::default();
        let mut state = scene(300);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, id) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let pos = text_rect(&output, "TestBrush0001").center();
        frame(&ctx, &mut state, vec![Event::PointerMoved(pos), Event::PointerButton {
            pos, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE,
        }], Modifiers::NONE, 500.0);
        scroll(&ctx, id, 100.0 * (24.0 + ctx.style().spacing.item_spacing.y));
        frame(&ctx, &mut state, vec![Event::PointerButton {
            pos, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE,
        }], Modifiers::NONE, 500.0);
        assert_eq!(state.selected_brush_index, None);
    }

    #[test]
    fn row_visibility_lock_duplicate_and_context_delete_target_correct_brush() {
        let ctx = Context::default();
        let mut state = scene(30);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let y = text_rect(&output, "TestBrush0001").center().y;
        click(&ctx, &mut state, Pos2::new(20.0, y), Modifiers::NONE);
        assert!(!state.is_brush_visible(1));
        assert!(state.is_brush_visible(0));
        click(&ctx, &mut state, Pos2::new(48.0, y), Modifiers::NONE);
        assert!(state.is_brush_locked(1));
        click(&ctx, &mut state, Pos2::new(48.0, y), Modifiers::NONE);
        assert!(!state.is_brush_locked(1));
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let duplicate = output.shapes.iter().find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape {
                let rect = Rect::from_min_size(text.pos, text.galley.size());
                if text.galley.text() == "📋" && (rect.center().y - y).abs() < 5.0 {
                    return Some(rect.center());
                }
            }
            None
        }).expect("Duplicate control stays visible alongside the label");
        click(&ctx, &mut state, duplicate, Modifiers::NONE);
        assert_eq!(state.world_brushes.len(), 31);
        assert!(state.world_brushes.last().unwrap().name.contains("TestBrush0001"));
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let pos = text_rect(&output, "TestBrush0002").center();
        for pressed in [true, false] {
            frame(&ctx, &mut state, vec![Event::PointerMoved(pos), Event::PointerButton {
                pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE,
            }], Modifiers::NONE, 500.0);
        }
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let delete = text_rect(&output, "Delete Brush").center();
        click(&ctx, &mut state, delete, Modifiers::NONE);
        assert_eq!(state.world_brushes.len(), 30);
        assert!(!state.world_brushes.iter().any(|b| b.name == "TestBrush0002"));
        assert_eq!(state.outliner_last_clicked_row, None);
    }

    #[test]
    fn mixed_element_types_remain_selectable_at_the_end_of_the_list() {
        let ctx = Context::default();
        let mut state = scene(300);
        let mut mesh = state.builder_brush.clone();
        mesh.name = "MixedStatic".into();
        state.static_meshes.push(mesh);
        state.add_mover_from_builder();
        state.movers[0].name = "MixedMover".into();
        let mut level = ut99_format::UnrLevel::default();
        let mut actor = ut99_format::ActorRecord::default();
        actor.name = "MixedLight".into();
        actor.class_name = "Light".into();
        level.actors.push(actor);
        state.loaded_unr = Some(level);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (_, id) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        scroll(&ctx, id, 1_000_000.0);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (initial, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        text_rect(&initial, "MixedLight");
        for name in ["MixedStatic", "MixedMover", "MixedLight"] {
            scroll(&ctx, id, 1_000_000.0);
            frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
            let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
            click(&ctx, &mut state, text_rect(&output, name).center(), Modifiers::NONE);
            match name {
                "MixedStatic" => assert_eq!(state.selected_static_mesh_index, Some(0)),
                "MixedMover" => assert_eq!(state.selected_mover_index, Some(0)),
                _ => assert_eq!(state.selected_actor_index, Some(0)),
            }
        }
    }

    #[test]
    fn focus_and_double_click_still_use_the_clicked_row() {
        let ctx = Context::default();
        ctx.options_mut(|options| options.input_options.max_double_click_delay = 2.0);
        let mut state = scene(30);
        state.world_brushes[1].location.x = 2000.0;
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let y = text_rect(&output, "TestBrush0001").center().y;
        let focus = output.shapes.iter().find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape {
                let rect = Rect::from_min_size(text.pos, text.galley.size());
                if text.galley.text() == "🎯" && (rect.center().y - y).abs() < 5.0 {
                    return Some(rect.center());
                }
            }
            None
        }).unwrap();
        let previous_camera = state.camera.position;
        click(&ctx, &mut state, focus, Modifiers::NONE);
        assert_eq!(state.selected_brush_index, Some(1));
        assert_ne!(state.camera.position, previous_camera);
        state.show_surface_properties = false;
        for _ in 0..2 {
            let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
            click(&ctx, &mut state, text_rect(&output, "TestBrush0002").center(), Modifiers::NONE);
        }
        assert_eq!(state.selected_brush_index, Some(2));
        assert!(state.show_surface_properties);
    }

    #[test]
    fn deleting_before_the_anchor_outside_the_outliner_invalidates_range_selection() {
        let ctx = Context::default();
        let mut state = scene(30);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        click(&ctx, &mut state, text_rect(&output, "TestBrush0005").center(), Modifiers::NONE);
        assert!(state.outliner_last_clicked_row.is_some());
        // The viewport and Delete shortcut can change the scene between UI frames.
        state.deselect_all();
        state.selected_brush_index = Some(0);
        state.delete_selected_batch();
        assert_eq!(state.world_brushes.len(), 29);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        assert_eq!(state.outliner_last_clicked_row, None);
    }

    #[test]
    fn range_anchor_survives_transforms_but_not_reordering() {
        let ctx = Context::default();
        let mut state = scene(30);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        click(&ctx, &mut state, text_rect(&output, "TestBrush0005").center(), Modifiers::NONE);
        let anchor = state.outliner_last_clicked_row;
        state.world_brushes[5].location.x += 100.0;
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        assert_eq!(state.outliner_last_clicked_row, anchor);
        state.world_brushes.swap(5, 6);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        assert_eq!(state.outliner_last_clicked_row, None);
    }

    #[test]
    fn long_names_and_larger_text_keep_uniform_rows_in_both_layouts() {
        for mode in [InspectorPanelMode::Split, InspectorPanelMode::OutlinerOnly] {
            let ctx = Context::default();
            ctx.style_mut(|style| { style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(28.0)); });
            let mut state = scene(30);
            state.inspector_panel_mode = mode;
            for brush in &mut state.world_brushes { brush.name.push_str(&" long name".repeat(30)); }
            frame(&ctx, &mut state, vec![], Modifiers::NONE, 320.0);
            let (output, id) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 320.0);
            let first = text_rect(&output, "TestBrush0000");
            let second = text_rect(&output, "TestBrush0001");
            assert!(first.max.y <= second.min.y);
            assert!(first.max.x <= 320.0 && second.max.x <= 320.0);
            scroll(&ctx, id, 1_000_000.0);
            frame(&ctx, &mut state, vec![], Modifiers::NONE, 320.0);
            let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 320.0);
            text_rect(&output, "TestBrush0029");
        }
    }

    #[test]
    fn outliner_cache_reuses_buffer_and_invalidates_on_filter_category_and_edits() {
        let ctx = Context::default();
        let mut state = scene(200);
        state.world_brushes[0].csg_oper = ut99_bsp::CsgOperation::Add;
        state.world_brushes[1].csg_oper = ut99_bsp::CsgOperation::Subtract;

        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        let ptr_before = state.outliner_cache.items.as_ptr();
        let len_before = state.outliner_cache.items.len();
        let key_before = state.outliner_cache.key;

        // Unchanged frames keep the exact cached buffer pointer and key
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        assert_eq!(state.outliner_cache.items.as_ptr(), ptr_before);
        assert_eq!(state.outliner_cache.items.len(), len_before);
        assert_eq!(state.outliner_cache.key, key_before);

        // Case-insensitive CsgOperation filter matches "subtract" without format!
        state.outliner_filter = "subtract".to_string();
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        text_rect(&output, "TestBrush0001");
        assert_eq!(state.outliner_cache.filter, "subtract");
        assert!(state.outliner_cache.items.contains(&LayerItemRef::Brush(1)));
        assert!(!state.outliner_cache.items.contains(&LayerItemRef::Brush(0)));

        // "add" filter switches items
        state.outliner_filter = "add".to_string();
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        assert!(state.outliner_cache.items.contains(&LayerItemRef::Brush(0)));
        assert!(!state.outliner_cache.items.contains(&LayerItemRef::Brush(1)));

        // Category change invalidates cache
        state.outliner_filter.clear();
        state.outliner_category = OutlinerCategory::Movers;
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        assert!(state.outliner_cache.items.is_empty());

        // Adding/removing mover, mesh, actor, and renaming brush invalidates immediately
        state.add_mover_from_builder();
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        assert_eq!(state.outliner_cache.items, vec![LayerItemRef::Mover(0)]);

        state.outliner_category = OutlinerCategory::All;
        let mut mesh = state.builder_brush.clone();
        mesh.name = "UniqueMeshTag".into();
        state.static_meshes.push(mesh);
        let mut level = ut99_format::UnrLevel::default();
        let mut actor = ut99_format::ActorRecord::default();
        actor.name = "UniqueActorTag".into();
        actor.class_name = "Light".into();
        level.actors.push(actor);
        state.loaded_unr = Some(level);
        frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        assert!(state.outliner_cache.items.contains(&LayerItemRef::Mesh(0)));
        assert!(state.outliner_cache.items.contains(&LayerItemRef::Actor(0)));

        // Rename brush & filter by new name
        state.world_brushes[3].name = "RenamedMagicBrush".into();
        state.outliner_filter = "renamedmagic".to_string();
        let (output, _) = frame(&ctx, &mut state, vec![], Modifiers::NONE, 500.0);
        text_rect(&output, "RenamedMagicBrush");
        assert_eq!(state.outliner_cache.items, vec![LayerItemRef::Brush(3)]);

        // new_map clears outliner cache
        state.new_map();
        assert_eq!(state.outliner_cache.key, None);
        assert!(state.outliner_cache.items.is_empty());
    }
}
