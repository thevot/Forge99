use egui::{self, Color32, RichText};
use crate::editor_state::{EditorState, FindActorCategory};

#[derive(Clone, Default)]
struct FindActorsInteraction {
    highlighted: Option<(usize, String, String)>,
    document_path: Option<std::path::PathBuf>,
    actor_storage: usize,
    query: String,
    category: FindActorCategory,
    keyboard_owned: bool,
    reveal_highlight: bool,
}

pub fn render_find_actors_dialog(ctx: &egui::Context, state: &mut EditorState) {
    let interaction_id = egui::Id::new("find_actors_interaction");
    let search_id = egui::Id::new("find_actors_search");
    if !state.show_find_actors_dialog {
        ctx.data_mut(|data| data.remove::<FindActorsInteraction>(interaction_id));
        return;
    }
    let previous = ctx.data(|data| data.get_temp::<FindActorsInteraction>(interaction_id));
    let opening = previous.is_none();
    let mut interaction = previous.unwrap_or_default();
    // A storage token is compared only; no pointer is retained or dereferenced.
    let actor_storage = state.loaded_unr.as_ref().map_or(0, |level| level.actors.as_ptr() as usize);
    if interaction.document_path != state.map_path || interaction.actor_storage != actor_storage {
        interaction.highlighted = None;
    }
    interaction.document_path = state.map_path.clone();
    interaction.actor_storage = actor_storage;

    let mut open = state.show_find_actors_dialog;
    let mut close_requested = false;

    crate::gui::modal::standard_dialog_window(
        ctx,
        RichText::new("Find Actors (UnrealEd 2.0 Parity — Ctrl+F)").strong(),
        &mut open,
        [640.0, 480.0],
        [480.0, 360.0],
    )
    .show(ctx, |ui| {
            // Capture before TextEdit consumes Enter or navigation keys. Only this
            // window's active search/list may handle them.
            let focused = ctx.memory(|memory| memory.focused());
            let focused_layer = focused.and_then(|id| ctx.read_response(id)).map(|response| response.layer_id);
            if opening { interaction.keyboard_owned = true; }
            if let Some(layer) = focused_layer {
                interaction.keyboard_owned = layer == ui.layer_id();
            } else if ui.ui_contains_pointer() && ctx.input(|input| input.pointer.any_pressed()) {
                interaction.keyboard_owned = true;
            }
            let owns_window = ui.is_enabled() && interaction.keyboard_owned
                && ctx.top_layer_id() == Some(ui.layer_id());
            let owns_search = owns_window && (focused == Some(search_id) || focused.is_none());
            let (down, up, enter, escape) = ctx.input(|input| (
                input.key_pressed(egui::Key::ArrowDown), input.key_pressed(egui::Key::ArrowUp),
                input.key_pressed(egui::Key::Enter), input.key_pressed(egui::Key::Escape),
            ));
            if owns_window && escape { close_requested = true; }
            ui.add_space(4.0);

            // Search Filter Bar
            ui.horizontal(|ui| {
                ui.label(RichText::new("Search:").strong());
                let search = ui.add(
                    egui::TextEdit::singleline(&mut state.find_actors_query)
                        .id(search_id)
                        .hint_text("Filter by name, class, tag, event, or group...")
                        .desired_width(320.0),
                );
                if opening { search.request_focus(); }
                if ui.button("✖").on_hover_text("Clear search query").clicked() {
                    state.find_actors_query.clear();
                    search.request_focus();
                }
            });

            ui.add_space(4.0);

            // Category Filter Pills
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Category:").size(11.0).color(Color32::from_rgb(170, 180, 195)));
                let categories = [
                    FindActorCategory::All,
                    FindActorCategory::Lights,
                    FindActorCategory::Navigation,
                    FindActorCategory::Triggers,
                    FindActorCategory::Inventory,
                    FindActorCategory::Movers,
                    FindActorCategory::Sounds,
                    FindActorCategory::Other,
                ];

                for cat in categories {
                    let is_active = state.find_actors_category == cat;
                    if ui.selectable_label(is_active, cat.label()).clicked() {
                        state.find_actors_category = cat;
                        ctx.memory_mut(|memory| memory.request_focus(search_id));
                    }
                }
            });

            ui.separator();

            // Derive both displayed results and actions from this frame's filters.
            let matches = state.find_actors_matching(&state.find_actors_query, state.find_actors_category);
            let match_count = matches.len();
            interaction.reveal_highlight |= opening || interaction.query != state.find_actors_query
                || interaction.category != state.find_actors_category;
            interaction.query = state.find_actors_query.clone();
            interaction.category = state.find_actors_category;
            let mut highlighted_position = interaction.highlighted.as_ref().and_then(|(index, name, class)|
                matches.iter().position(|actor| actor.actor_index == *index && actor.name == *name && actor.class_name == *class));
            if highlighted_position.is_none() {
                highlighted_position = if opening {
                    matches.iter().position(|actor| Some(actor.actor_index) == state.selected_actor_index)
                } else { None }.or_else(|| (!matches.is_empty()).then_some(0));
                interaction.reveal_highlight = true;
            }
            if owns_search && !matches.is_empty() {
                let position = highlighted_position.unwrap_or(0);
                if down { highlighted_position = Some((position + 1).min(matches.len() - 1)); }
                else if up { highlighted_position = Some(position.saturating_sub(1)); }
                interaction.reveal_highlight |= down || up;
            }
            interaction.highlighted = highlighted_position.map(|position| {
                let actor = &matches[position];
                (actor.actor_index, actor.name.clone(), actor.class_name.clone())
            });
            let mut to_focus = if owns_search && enter && !close_requested {
                interaction.highlighted.as_ref().map(|actor| actor.0)
            } else { None };
            let mut to_properties = None;

            // Match Summary & Batch Actions
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Matches found: {}", match_count)).strong().color(Color32::from_rgb(100, 200, 255)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Deselect All").clicked() {
                        state.deselect_all();
                    }
                    if ui.add_enabled(match_count > 0, egui::Button::new("⭐ Select All Matches")).on_hover_text("Multi-selects all matching actors").clicked() {
                        let query = state.find_actors_query.clone();
                        let cat = state.find_actors_category;
                        state.select_all_matching_actors(&query, cat);
                    }
                });
            });

            ui.separator();

            // Table / List of matching actors
            egui::ScrollArea::vertical()
                .max_height((ui.available_height() - 56.0).max(120.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                if ui.ui_contains_pointer() && ui.input(|input| input.raw_scroll_delta != egui::Vec2::ZERO) {
                    interaction.reveal_highlight = false;
                }
                if matches.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No matching actors found in this level.").italics().color(Color32::GRAY));
                    });
                } else {
                    egui::Grid::new("find_actors_grid")
                        .striped(true)
                        .min_col_width(50.0)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            // Table Headers
                            ui.label(RichText::new("#").strong());
                            ui.label(RichText::new("Actor Name").strong());
                            ui.label(RichText::new("Class").strong());
                            ui.label(RichText::new("Tag").strong());
                            ui.label(RichText::new("Event").strong());
                            ui.label(RichText::new("Location (X, Y, Z)").strong());
                            ui.label(RichText::new("Group").strong());
                            ui.label(RichText::new("Actions").strong());
                            ui.end_row();

                            for m in &matches {
                                let is_selected = interaction.highlighted.as_ref().is_some_and(|actor| actor.0 == m.actor_index);

                                let row_color = if is_selected {
                                    Color32::from_rgb(245, 182, 45)
                                } else {
                                    Color32::from_rgb(215, 220, 230)
                                };

                                ui.label(RichText::new(format!("{}", m.actor_index)).color(row_color));

                                let name_label = ui.selectable_label(is_selected, RichText::new(&m.name).strong().color(row_color));
                                if is_selected && interaction.reveal_highlight {
                                    name_label.scroll_to_me(None);
                                    if ui.clip_rect().contains_rect(name_label.rect) {
                                        interaction.reveal_highlight = false;
                                    }
                                }
                                if name_label.clicked() {
                                    interaction.highlighted = Some((m.actor_index, m.name.clone(), m.class_name.clone()));
                                    ctx.memory_mut(|memory| memory.request_focus(search_id));
                                }
                                if name_label.double_clicked() {
                                    to_focus = Some(m.actor_index);
                                }

                                ui.label(RichText::new(&m.class_name).color(Color32::from_rgb(120, 190, 240)));
                                ui.label(RichText::new(if m.tag.is_empty() { "-" } else { &m.tag }).color(Color32::from_rgb(230, 160, 60)));
                                ui.label(RichText::new(if m.event.is_empty() { "-" } else { &m.event }).color(Color32::from_rgb(240, 200, 80)));
                                ui.label(RichText::new(format!("({:.0}, {:.0}, {:.0})", m.location.x, m.location.y, m.location.z)).monospace().size(11.0));
                                ui.label(RichText::new(if m.group.is_empty() { "-" } else { &m.group }).color(Color32::from_rgb(170, 180, 195)));

                                ui.horizontal(|ui| {
                                    if ui.small_button("🎯 Focus").on_hover_text("Select and center camera on this actor").clicked() {
                                        interaction.highlighted = Some((m.actor_index, m.name.clone(), m.class_name.clone()));
                                        to_focus = Some(m.actor_index);
                                    }
                                    if ui.small_button("⚙ Props").on_hover_text("Open Actor Properties (F4)").clicked() {
                                        interaction.highlighted = Some((m.actor_index, m.name.clone(), m.class_name.clone()));
                                        to_properties = Some(m.actor_index);
                                    }
                                });

                                ui.end_row();
                            }

                        });
                }
            });

            if !close_requested {
                if let Some(index) = to_focus { state.select_and_focus_actor(index); }
                if let Some(index) = to_properties {
                    if state.select_and_focus_actor(index) { state.show_actor_properties = true; }
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new("💡 Tip: Type to search; ↑/↓ or click to highlight. Enter, double-click or Focus centers the camera. Esc closes; Props opens properties.").size(10.5).italics().color(Color32::from_rgb(140, 150, 165)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    if !open || close_requested {
        state.show_find_actors_dialog = false;
        ctx.data_mut(|data| data.remove::<FindActorsInteraction>(interaction_id));
    } else {
        ctx.data_mut(|data| data.insert_temp(interaction_id, interaction));
    }
}
