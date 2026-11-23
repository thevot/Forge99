use egui::{Color32, RichText, ScrollArea};
use glam::Vec3;
use std::collections::HashMap;
use ut99_format::unr::tagged_property::{TaggedProperty, TaggedValue};

use crate::editor_state::EditorState;
use crate::gui::ut99_light_to_color;

const LIGHT_TYPES: [&str; 10] = [
    "LT_None",
    "LT_Steady",
    "LT_Pulse",
    "LT_Blink",
    "LT_Flicker",
    "LT_Strobe",
    "LT_BackdropLight",
    "LT_SubtlePulse",
    "LT_TexturePaletteOnce",
    "LT_TexturePaletteLoop",
];

const LIGHT_EFFECTS: [&str; 20] = [
    "LE_None",
    "LE_Torch",
    "LE_Fire",
    "LE_WateryShimmer",
    "LE_Searchlight",
    "LE_SlowWave",
    "LE_FastWave",
    "LE_CloudCast",
    "LE_StaticSpot",
    "LE_Shock",
    "LE_Disco",
    "LE_Warp",
    "LE_Spotlight",
    "LE_NonIncidence",
    "LE_Shell",
    "LE_OmniBumpMap",
    "LE_Interference",
    "LE_Cylinder",
    "LE_Rotor",
    "LE_Unused",
];

pub fn render_actor_properties_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_actor_properties {
        return;
    }

    let act_idx = state.selected_actor_index;
    let mut open = state.show_actor_properties;

    let window_title = if let (Some(ref level), Some(idx)) = (&state.loaded_unr, act_idx) {
        if let Some(actor) = level.actors.get(idx) {
            format!("⚙ Actor Properties - {} [{}]", actor.name, actor.class_name)
        } else {
            "⚙ Actor Properties".to_string()
        }
    } else {
        "⚙ Actor Properties".to_string()
    };

    let mut focus_requested = false;
    let mut duplicate_requested = false;
    let mut snap_floor_requested = false;
    let mut snap_aligned_requested = false;
    let mut delete_requested = false;
    let mut match_tag_requested = false;
    let mut match_event_requested = false;
    let mut find_targets_requested = false;
    let mut find_triggers_requested = false;
    let mut reset_defaults_requested = false;
    let mut pending_actor_undo: Option<(
        usize,
        ut99_format::unr::actor::ActorRecord,
        ut99_format::unr::actor::ActorRecord,
    )> = None;

    crate::gui::modal::standard_dialog_window(ctx, window_title, &mut open, [460.0, 560.0], [340.0, 320.0])
        .show(ctx, |ui| {
            let Some(idx) = act_idx else {
                ui.add_space(16.0);
                crate::gui::widgets::empty_state_card(
                    ui,
                    crate::gui::icons::Icon::ActorProperties,
                    "No Actor Selected",
                    "Select an actor in the 3D Viewport or Level Outliner (or press Enter / double-click) to view and edit its properties.",
                );
                return;
            };

            let Some(ref mut level) = state.loaded_unr else {
                ui.label("No level currently loaded.");
                return;
            };

            let Some(actor) = level.actors.get_mut(idx) else {
                ui.label("Selected actor index out of bounds.");
                return;
            };

            let old_actor_snapshot = actor.clone();

            // 1. Top Quick Action Toolbar
            ui.horizontal(|ui| {
                if ui.button("🎯 Focus").on_hover_text("Center camera on actor (F)").clicked() {
                    focus_requested = true;
                }
                if ui.button("📋 Duplicate").on_hover_text("Duplicate Actor (Ctrl+D)").clicked() {
                    duplicate_requested = true;
                }
                if ui.button("⬇ Snap Floor").on_hover_text("Snap down to floor (End)").clicked() {
                    snap_floor_requested = true;
                }
                if ui.button("📐 Align Floor").on_hover_text("Snap and tilt to floor normal (Shift+End)").clicked() {
                    snap_aligned_requested = true;
                }
                if ui
                    .button("↺ Reset Defaults")
                    .on_hover_text("Reset actor properties back to UnrealScript class defaultproperties while preserving Name, Location, Rotation, Tag, and Event (REQ-E08)")
                    .clicked()
                {
                    reset_defaults_requested = true;
                }
                if ui.button(RichText::new("🗑 Delete").color(Color32::from_rgb(255, 110, 110))).on_hover_text("Delete Actor (Delete)").clicked() {
                    delete_requested = true;
                }
            });

            ui.separator();

            // 2. Scrollable Property Categories
            ScrollArea::vertical().show(ui, |ui| {
                // Category A: Identification
                egui::CollapsingHeader::new(RichText::new("🎯 Identification").strong())
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("actor_id_grid")
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("Name:").strong());
                                ui.text_edit_singleline(&mut actor.name);
                                ui.end_row();

                                ui.label(RichText::new("Class:").strong());
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&actor.class_name).color(Color32::from_rgb(160, 200, 255)).strong());
                                    ui.label(format!("#{}", actor.export_index));
                                });
                                ui.end_row();

                                ui.label(RichText::new("Tag:").strong());
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(&mut actor.tag);
                                    if !actor.tag.trim().is_empty() {
                                        if ui.small_button("👥 Match").on_hover_text("Select all actors in level with this exact Tag").clicked() {
                                            match_tag_requested = true;
                                        }
                                        if ui.small_button("🔍 Find Triggers").on_hover_text("Find and select actors whose Event matches this Tag (Event -> Tag)").clicked() {
                                            find_triggers_requested = true;
                                        }
                                    }
                                });
                                ui.end_row();

                                ui.label(RichText::new("Event:").strong());
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(&mut actor.event);
                                    if !actor.event.trim().is_empty() {
                                        if ui.small_button("👥 Match").on_hover_text("Select all actors in level with this exact Event").clicked() {
                                            match_event_requested = true;
                                        }
                                        if ui.small_button("🎯 Find Targets").on_hover_text("Find and select target actors triggered by this Event (Event -> Tag)").clicked() {
                                            find_targets_requested = true;
                                        }
                                    }
                                });
                                ui.end_row();

                                ui.label(RichText::new("Group:").strong());
                                ui.text_edit_singleline(&mut actor.group);
                                ui.end_row();
                            });
                    });

                // Category B: Movement & Transform
                egui::CollapsingHeader::new(RichText::new("📐 Movement & Transform").strong())
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Location (World Units):").strong().size(11.0));
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            ui.add(egui::DragValue::new(&mut actor.location.x).speed(8.0));
                            ui.label("Y:");
                            ui.add(egui::DragValue::new(&mut actor.location.y).speed(8.0));
                            ui.label("Z:");
                            ui.add(egui::DragValue::new(&mut actor.location.z).speed(8.0));
                        });

                        ui.add_space(4.0);
                        ui.label(RichText::new("Rotation (Unreal Rotator Units & Degrees):").strong().size(11.0));
                        ui.horizontal(|ui| {
                            ui.label("Pitch:");
                            ui.add(egui::DragValue::new(&mut actor.rotation.0).speed(256.0).range(-32768..=32767));
                            let p_deg = (actor.rotation.0 as f32 / 65536.0 * 360.0).round();
                            ui.label(format!("({:.0}°)", p_deg));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Yaw:  ");
                            ui.add(egui::DragValue::new(&mut actor.rotation.1).speed(256.0).range(-32768..=32767));
                            let y_deg = (actor.rotation.1 as f32 / 65536.0 * 360.0).round();
                            ui.label(format!("({:.0}°)", y_deg));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Roll: ");
                            ui.add(egui::DragValue::new(&mut actor.rotation.2).speed(256.0).range(-32768..=32767));
                            let r_deg = (actor.rotation.2 as f32 / 65536.0 * 360.0).round();
                            ui.label(format!("({:.0}°)", r_deg));
                        });
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let has_rot = actor.rotation != (0, 0, 0);
                            let rot_btn = ui.add_enabled(has_rot, egui::Button::new("↺ Reset Rotation").small());
                            if rot_btn.on_hover_text("Reset Pitch, Yaw, and Roll back to (0, 0, 0)").clicked() {
                                actor.rotation = (0, 0, 0);
                            }

                            let has_pivot = actor.prepivot != glam::Vec3::ZERO;
                            let piv_btn = ui.add_enabled(has_pivot, egui::Button::new("↺ Reset PrePivot").small());
                            if piv_btn.on_hover_text("Reset PrePivot offset back to (0, 0, 0)").clicked() {
                                actor.prepivot = glam::Vec3::ZERO;
                            }

                            let has_custom_scale = actor.post_scale != glam::Vec3::ONE;
                            let scale_btn = ui.add_enabled(has_custom_scale, egui::Button::new("↺ Reset Scale").small());
                            if scale_btn.on_hover_text("Reset PostScale back to 1.0 on all axes").clicked() {
                                actor.post_scale = glam::Vec3::ONE;
                            }
                        });

                        ui.add_space(4.0);
                        ui.label(RichText::new("PostScale / DrawScale:").strong().size(11.0));
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            ui.add(egui::DragValue::new(&mut actor.post_scale.x).speed(0.05).range(0.01..=100.0));
                            ui.label("Y:");
                            ui.add(egui::DragValue::new(&mut actor.post_scale.y).speed(0.05).range(0.01..=100.0));
                            ui.label("Z:");
                            ui.add(egui::DragValue::new(&mut actor.post_scale.z).speed(0.05).range(0.01..=100.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("PrePivot:");
                            ui.add(egui::DragValue::new(&mut actor.prepivot.x).speed(1.0));
                            ui.label("Y:");
                            ui.add(egui::DragValue::new(&mut actor.prepivot.y).speed(1.0));
                            ui.label("Z:");
                            ui.add(egui::DragValue::new(&mut actor.prepivot.z).speed(1.0));
                        });
                    });

                // Category C: Lighting (Shown for Lights or actors with Light attributes)
                let is_light = actor.class_name.contains("Light");
                egui::CollapsingHeader::new(RichText::new("💡 Lighting & Attenuation").strong())
                    .default_open(is_light)
                    .show(ui, |ui| {
                        egui::Grid::new("actor_lighting_grid")
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("Brightness:").strong());
                                ui.add(egui::Slider::new(&mut actor.light_brightness, 0..=255));
                                ui.end_row();

                                ui.label(RichText::new("Radius:").strong());
                                ui.add(egui::Slider::new(&mut actor.light_radius, 0..=255));
                                ui.end_row();

                                ui.label(RichText::new("Hue:").strong());
                                ui.add(egui::Slider::new(&mut actor.light_hue, 0..=255));
                                ui.end_row();

                                ui.label(RichText::new("Saturation:").strong());
                                ui.add(egui::Slider::new(&mut actor.light_saturation, 0..=255));
                                ui.end_row();

                                ui.label(RichText::new("Light Type:").strong());
                                let mut cur_type = actor.light_type as usize;
                                egui::ComboBox::from_id_salt("actor_light_type_combo")
                                    .selected_text(LIGHT_TYPES.get(cur_type).copied().unwrap_or("Custom"))
                                    .show_ui(ui, |ui| {
                                        for (i, &lt_name) in LIGHT_TYPES.iter().enumerate() {
                                            ui.selectable_value(&mut cur_type, i, lt_name);
                                        }
                                    });
                                actor.light_type = cur_type as u8;
                                ui.end_row();

                                ui.label(RichText::new("Light Effect:").strong());
                                let mut cur_effect = actor.light_effect as usize;
                                egui::ComboBox::from_id_salt("actor_light_effect_combo")
                                    .selected_text(LIGHT_EFFECTS.get(cur_effect).copied().unwrap_or("Custom"))
                                    .show_ui(ui, |ui| {
                                        for (i, &le_name) in LIGHT_EFFECTS.iter().enumerate() {
                                            ui.selectable_value(&mut cur_effect, i, le_name);
                                        }
                                    });
                                actor.light_effect = cur_effect as u8;
                                ui.end_row();

                                ui.label(RichText::new("Color Preview:").strong());
                                let swatch = ut99_light_to_color(actor.light_hue, actor.light_saturation, actor.light_brightness);
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(48.0, 20.0), egui::Sense::hover());
                                    ui.painter().rect_filled(rect, 3.0, swatch);
                                    ui.label(format!("#{:02X}{:02X}{:02X}", swatch.r(), swatch.g(), swatch.b()));
                                });
                                ui.end_row();
                            });
                    });

                // Category D: Tagged Properties Explorer (UnrealEd 2.0 Parity + UnrealScript Visual Array Inspector )
                egui::CollapsingHeader::new(RichText::new(format!("📦 Tagged Properties ({})", actor.properties.len())).strong())
                    .default_open(!actor.properties.is_empty())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("🔍");
                            ui.text_edit_singleline(&mut state.actor_properties_search);
                            if !state.actor_properties_search.is_empty() && ui.small_button("✖").clicked() {
                                state.actor_properties_search.clear();
                            }
                        });

                        ui.add_space(4.0);

                        let query = state.actor_properties_search.to_lowercase();
                        let class_meta = crate::unrealscript::get_class_metadata(&actor.class_name, None, None);
                        let mut declared_arrays: HashMap<String, (usize, String)> = HashMap::new();
                        for v in &class_meta.variables {
                            if let (Some(open), Some(close)) = (v.var_type.find('['), v.var_type.find(']')) {
                                if close > open + 1 {
                                    if let Ok(cap) = v.var_type[open + 1..close].trim().parse::<usize>() {
                                        let elem_t = v.var_type[..open].trim().to_string();
                                        declared_arrays.insert(v.name.to_lowercase(), (cap, elem_t));
                                    }
                                }
                            }
                        }

                        // Group properties by base name into scalars and arrays
                        let mut groups: HashMap<String, (String, Vec<(i32, String)>)> = HashMap::new();
                        for (key, prop) in &actor.properties {
                            let (base, idx) = parse_prop_key_and_index(key, prop);
                            let low = base.to_lowercase();
                            let entry = groups.entry(low).or_insert_with(|| (base, Vec::new()));
                            entry.1.push((idx, key.clone()));
                        }

                        for (_, (_, list)) in groups.iter_mut() {
                            list.sort_by_key(|(idx, _)| *idx);
                        }

                        let mut sorted_group_keys: Vec<String> = groups.keys().cloned().collect();
                        sorted_group_keys.sort_unstable();

                        let mut pending_action: Option<PropAction> = None;

                        // 1. Render Scalar Properties in a Grid
                        let mut scalar_keys = Vec::new();
                        let mut array_keys = Vec::new();

                        for g_key in &sorted_group_keys {
                            let (base_name, elements) = &groups[g_key];
                            let is_declared_array = declared_arrays.contains_key(g_key);
                            let is_array = is_declared_array || elements.len() > 1 || elements.iter().any(|(i, _)| *i > 0);
                            if is_array {
                                array_keys.push(g_key.clone());
                            } else {
                                scalar_keys.push((base_name.clone(), elements[0].1.clone()));
                            }
                        }

                        if !scalar_keys.is_empty() {
                            egui::Grid::new("tagged_props_scalar_grid")
                                .num_columns(3)
                                .spacing([10.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for (base_name, map_key) in &scalar_keys {
                                        if !query.is_empty() && !base_name.to_lowercase().contains(&query) {
                                            continue;
                                        }

                                        if let Some(prop) = actor.properties.get_mut(map_key) {
                                            ui.label(RichText::new(&prop.name).strong().size(11.0));
                                            render_tagged_value_editor(ui, &mut prop.value);
                                            if ui.small_button("🗑").on_hover_text("Remove property").clicked() {
                                                pending_action = Some(PropAction::DeleteProperty(map_key.clone()));
                                            }
                                            ui.end_row();
                                        }
                                    }
                                });
                        }

                        // 2. Render Array Properties with Visual Array Inspector
                        if !array_keys.is_empty() {
                            ui.add_space(6.0);
                            for a_key in &array_keys {
                                let (base_name, elements) = &groups[a_key];
                                if !query.is_empty() && !base_name.to_lowercase().contains(&query) {
                                    continue;
                                }

                                let cap_info = declared_arrays.get(a_key);
                                let cap_text = match cap_info {
                                    Some((cap, _)) => format!("{}/{}", elements.len(), cap),
                                    None => format!("{}", elements.len()),
                                };

                                let header_label = format!("📚 {} [{}]", base_name, cap_text);
                                egui::CollapsingHeader::new(RichText::new(header_label).strong().color(Color32::from_rgb(180, 215, 255)))
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        let count = elements.len();
                                        for (pos, (arr_idx, map_key)) in elements.iter().enumerate() {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(format!("[{}]", arr_idx)).monospace().strong().color(Color32::from_rgb(200, 200, 200)));

                                                if let Some(prop) = actor.properties.get_mut(map_key) {
                                                    render_tagged_value_editor(ui, &mut prop.value);
                                                }

                                                if pos > 0 && ui.small_button("⬆").on_hover_text("Move slot up").clicked() {
                                                    let prev_key = elements[pos - 1].1.clone();
                                                    pending_action = Some(PropAction::SwapElements(map_key.clone(), prev_key));
                                                }
                                                if pos + 1 < count && ui.small_button("⬇").on_hover_text("Move slot down").clicked() {
                                                    let next_key = elements[pos + 1].1.clone();
                                                    pending_action = Some(PropAction::SwapElements(map_key.clone(), next_key));
                                                }
                                                if ui.small_button("🗑").on_hover_text("Delete slot and compact indices").clicked() {
                                                    pending_action = Some(PropAction::RemoveArraySlot {
                                                        base_name: base_name.clone(),
                                                        remove_idx: pos,
                                                    });
                                                }
                                            });
                                        }

                                        let can_add = cap_info.map(|(cap, _)| elements.len() < *cap).unwrap_or(true);
                                        ui.horizontal(|ui| {
                                            if can_add {
                                                if ui.button(RichText::new("➕ Add Element").size(11.0)).clicked() {
                                                    let next_idx = elements.last().map(|(i, _)| *i + 1).unwrap_or(0);
                                                    let sample_val = elements.last().and_then(|(_, k)| actor.properties.get(k).map(|p| &p.value));
                                                    let elem_t = cap_info.map(|(_, t)| t.as_str());
                                                    let def_val = make_default_tagged_value(sample_val, elem_t);
                                                    pending_action = Some(PropAction::AddArraySlot {
                                                        base_name: base_name.clone(),
                                                        next_idx,
                                                        default_val: def_val,
                                                    });
                                                }
                                            } else {
                                                ui.label(RichText::new("Array is at maximum declared capacity").size(10.5).color(Color32::DARK_GRAY));
                                            }
                                        });
                                    });
                            }
                        }

                        ui.separator();
                        let key_id = egui::Id::new(("add_actor_prop_key", idx));
                        let val_id = egui::Id::new(("add_actor_prop_val", idx));
                        let mut new_key: String = ui.ctx().data_mut(|d| d.get_temp_mut_or_default::<String>(key_id).clone());
                        let mut new_val: String = ui.ctx().data_mut(|d| d.get_temp_mut_or_default::<String>(val_id).clone());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("+ Prop:").size(11.0).color(Color32::LIGHT_GRAY));
                            ui.add(egui::TextEdit::singleline(&mut new_key).hint_text("Name or Name[index]").desired_width(130.0));
                            ui.add(egui::TextEdit::singleline(&mut new_val).hint_text("Value").desired_width(90.0));
                            if ui.small_button("➕ Add").clicked() && !new_key.trim().is_empty() {
                                let raw_input = new_key.trim();
                                let v_trim = new_val.trim();
                                let tv = if v_trim.eq_ignore_ascii_case("true") {
                                    TaggedValue::Bool(true)
                                } else if v_trim.eq_ignore_ascii_case("false") {
                                    TaggedValue::Bool(false)
                                } else if let Ok(iv) = v_trim.parse::<i32>() {
                                    TaggedValue::Int(iv)
                                } else if let Ok(fv) = v_trim.parse::<f32>() {
                                    TaggedValue::Float(fv)
                                } else {
                                    TaggedValue::Str(v_trim.to_string())
                                };

                                let (base_k, parsed_idx) = parse_user_prop_input(raw_input);
                                let (final_idx, map_key) = if let Some(idx) = parsed_idx {
                                    (idx, if idx == 0 { base_k.clone() } else { format!("{}({})", base_k, idx) })
                                } else {
                                    // Check if base_k already exists in properties
                                    let low = base_k.to_lowercase();
                                    let existing_count = groups.get(&low).map(|(_, elems)| elems.len()).unwrap_or(0);
                                    let is_decl_arr = declared_arrays.contains_key(&low);
                                    if is_decl_arr || existing_count > 0 {
                                        let next_i = groups.get(&low).and_then(|(_, elems)| elems.last().map(|(i, _)| i + 1)).unwrap_or(0);
                                        (next_i, if next_i == 0 { base_k.clone() } else { format!("{}({})", base_k, next_i) })
                                    } else {
                                        (0, base_k.clone())
                                    }
                                };

                                actor.properties.insert(
                                    map_key,
                                    ut99_format::TaggedProperty {
                                        name: base_k,
                                        array_index: final_idx,
                                        value: tv,
                                    },
                                );
                                new_key.clear();
                                new_val.clear();
                            }
                        });
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(key_id, new_key);
                            d.insert_temp(val_id, new_val);
                        });

                        // 3. Execute any pending mutations
                        if let Some(act) = pending_action {
                            match act {
                                PropAction::SwapElements(k1, k2) => {
                                    let v1 = actor.properties.get(&k1).map(|p| p.value.clone());
                                    let v2 = actor.properties.get(&k2).map(|p| p.value.clone());
                                    if let (Some(val1), Some(val2)) = (v1, v2) {
                                        if let Some(p1) = actor.properties.get_mut(&k1) { p1.value = val2; }
                                        if let Some(p2) = actor.properties.get_mut(&k2) { p2.value = val1; }
                                    }
                                }
                                PropAction::RemoveArraySlot { base_name, remove_idx } => {
                                    let low = base_name.to_lowercase();
                                    if let Some((_, elems)) = groups.get(&low) {
                                        let mut remaining_props = Vec::new();
                                        for (pos, (_, map_key)) in elems.iter().enumerate() {
                                            if let Some(p) = actor.properties.remove(map_key) {
                                                if pos != remove_idx {
                                                    remaining_props.push(p);
                                                }
                                            }
                                        }
                                        for (new_i, mut prop) in remaining_props.into_iter().enumerate() {
                                            prop.array_index = new_i as i32;
                                            let key = if new_i == 0 {
                                                base_name.clone()
                                            } else {
                                                format!("{}({})", base_name, new_i)
                                            };
                                            actor.properties.insert(key, prop);
                                        }
                                    }
                                }
                                PropAction::AddArraySlot { base_name, next_idx, default_val } => {
                                    let key = if next_idx == 0 {
                                        base_name.clone()
                                    } else {
                                        format!("{}({})", base_name, next_idx)
                                    };
                                    actor.properties.insert(
                                        key,
                                        TaggedProperty {
                                            name: base_name,
                                            array_index: next_idx,
                                            value: default_val,
                                        },
                                    );
                                }
                                PropAction::DeleteProperty(key) => {
                                    actor.properties.remove(&key);
                                }
                            }
                        }
                    });

                if *actor != old_actor_snapshot {
                    pending_actor_undo = Some((idx, old_actor_snapshot, actor.clone()));
                }
            });
        });

    state.show_actor_properties = open;

    let pointer_down = ctx.input(|i| i.pointer.any_down());
    let has_focus = ctx.memory(|m| m.focused().is_some());
    if !pointer_down && !has_focus {
        state.history.end_interaction();
    }

    if let Some((idx, old_actor, new_actor)) = pending_actor_undo {
        let desc = format!("Edit Actor Properties ({})", new_actor.name);
        state.history.push_interactive(
            crate::history::Action::UpdateLightEffects {
                actor_index: idx,
                old_actor,
                new_actor,
                description: desc,
            },
            pointer_down || has_focus,
        );
        if let Some(ref mut level) = state.loaded_unr {
            level.sync_actor_views();
        }
        state.map_is_dirty = true;
    }

    // Handle Quick Action Requests
    if focus_requested {
        state.focus_camera_on_selection();
    }
    if duplicate_requested {
        state.duplicate_selected();
    }
    if snap_floor_requested {
        state.snap_selected_to_floor();
    }
    if snap_aligned_requested {
        state.snap_selected_to_floor_aligned();
    }
    if match_tag_requested {
        state.select_actors_matching_tag();
    }
    if match_event_requested {
        state.select_actors_matching_event();
    }
    if find_targets_requested {
        state.select_actors_targeted_by_event();
    }
    if find_triggers_requested {
        state.select_actors_triggering_tag();
    }
    if reset_defaults_requested {
        if let Some(act_idx) = state.selected_actor_index {
            state.reset_actor_properties_to_class_defaults(act_idx);
        }
    }
    if delete_requested {
        if let Some(act_idx) = state.selected_actor_index {
            state.delete_actor_at(act_idx);
        }
    }
}

enum PropAction {
    SwapElements(String, String),
    RemoveArraySlot {
        base_name: String,
        remove_idx: usize,
    },
    AddArraySlot {
        base_name: String,
        next_idx: i32,
        default_val: TaggedValue,
    },
    DeleteProperty(String),
}

fn parse_prop_key_and_index(key: &str, prop: &TaggedProperty) -> (String, i32) {
    if prop.array_index > 0 {
        return (prop.name.clone(), prop.array_index);
    }
    for (open_c, close_c) in [('(', ')'), ('[', ']')] {
        if let (Some(start), Some(end)) = (key.find(open_c), key.rfind(close_c)) {
            if end > start + 1 {
                if let Ok(idx) = key[start + 1..end].trim().parse::<i32>() {
                    return (key[..start].trim().to_string(), idx);
                }
            }
        }
        if let (Some(start), Some(end)) = (prop.name.find(open_c), prop.name.rfind(close_c)) {
            if end > start + 1 {
                if let Ok(idx) = prop.name[start + 1..end].trim().parse::<i32>() {
                    return (prop.name[..start].trim().to_string(), idx);
                }
            }
        }
    }
    (prop.name.clone(), prop.array_index)
}

fn parse_user_prop_input(input: &str) -> (String, Option<i32>) {
    for (open_c, close_c) in [('(', ')'), ('[', ']')] {
        if let (Some(start), Some(end)) = (input.find(open_c), input.rfind(close_c)) {
            if end > start + 1 {
                if let Ok(idx) = input[start + 1..end].trim().parse::<i32>() {
                    return (input[..start].trim().to_string(), Some(idx));
                }
            }
        }
    }
    (input.trim().to_string(), None)
}

fn make_default_tagged_value(sample_val: Option<&TaggedValue>, elem_type: Option<&str>) -> TaggedValue {
    if let Some(s) = sample_val {
        match s {
            TaggedValue::Byte(_) => TaggedValue::Byte(0),
            TaggedValue::Int(_) => TaggedValue::Int(0),
            TaggedValue::Float(_) => TaggedValue::Float(0.0),
            TaggedValue::Bool(_) => TaggedValue::Bool(false),
            TaggedValue::Str(_) => TaggedValue::Str(String::new()),
            TaggedValue::Name(_) => TaggedValue::Name(String::new()),
            TaggedValue::Vector(_) => TaggedValue::Vector(Vec3::ZERO),
            TaggedValue::Rotator { .. } => TaggedValue::Rotator { pitch: 0, yaw: 0, roll: 0 },
            TaggedValue::Color { .. } => TaggedValue::Color { r: 255, g: 255, b: 255, a: 255 },
            TaggedValue::Object(_) => TaggedValue::Object(0),
            TaggedValue::Scale { .. } => TaggedValue::Scale { scale: Vec3::ONE, sheer_rate: 0.0, sheer_axis: 0 },
            TaggedValue::Raw(_) => TaggedValue::Raw(Vec::new()),
        }
    } else if let Some(et) = elem_type {
        let low = et.to_lowercase();
        if low.contains("float") {
            TaggedValue::Float(0.0)
        } else if low.contains("int") || low.contains("byte") {
            TaggedValue::Int(0)
        } else if low.contains("bool") {
            TaggedValue::Bool(false)
        } else if low.contains("vector") {
            TaggedValue::Vector(Vec3::ZERO)
        } else if low.contains("rotator") {
            TaggedValue::Rotator { pitch: 0, yaw: 0, roll: 0 }
        } else if low.contains("name") {
            TaggedValue::Name(String::new())
        } else {
            TaggedValue::Str(String::new())
        }
    } else {
        TaggedValue::Str(String::new())
    }
}

fn render_tagged_value_editor(ui: &mut egui::Ui, value: &mut TaggedValue) {
    match value {
        TaggedValue::Byte(b) => {
            ui.add(egui::DragValue::new(b).range(0..=255));
        }
        TaggedValue::Int(i) => {
            ui.add(egui::DragValue::new(i));
        }
        TaggedValue::Float(f) => {
            ui.add(egui::DragValue::new(f).speed(0.1));
        }
        TaggedValue::Bool(b) => {
            ui.checkbox(b, "");
        }
        TaggedValue::Str(s) | TaggedValue::Name(s) => {
            ui.text_edit_singleline(s);
        }
        TaggedValue::Vector(v) => {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut v.x).speed(1.0));
                ui.add(egui::DragValue::new(&mut v.y).speed(1.0));
                ui.add(egui::DragValue::new(&mut v.z).speed(1.0));
            });
        }
        TaggedValue::Rotator { pitch, yaw, roll } => {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(pitch).speed(256.0));
                ui.add(egui::DragValue::new(yaw).speed(256.0));
                ui.add(egui::DragValue::new(roll).speed(256.0));
            });
        }
        TaggedValue::Color { r, g, b, .. } => {
            let col = Color32::from_rgb(*r, *g, *b);
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, col);
                ui.label(format!("({}, {}, {})", r, g, b));
            });
        }
        TaggedValue::Object(obj_idx) => {
            ui.label(RichText::new(format!("Object #{}", obj_idx)).color(Color32::from_rgb(180, 210, 180)).size(10.5));
        }
        TaggedValue::Scale { scale, .. } => {
            ui.label(format!("Scale ({:.1}, {:.1}, {:.1})", scale.x, scale.y, scale.z));
        }
        TaggedValue::Raw(bytes) => {
            ui.label(RichText::new(format!("Raw [{} bytes]", bytes.len())).size(10.5).color(Color32::DARK_GRAY));
        }
    }
}
