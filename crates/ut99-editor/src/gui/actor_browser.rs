use crate::editor_state::EditorState;
use egui::{Color32, RichText};
use ut99_format::package::ClassTreeNode;

pub fn render_actor_browser(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_actor_browser {
        return;
    }

    let mut open = state.show_actor_browser;

    crate::gui::modal::standard_dialog_window(ctx, "Actor Class Browser", &mut open, [380.0, 520.0], [300.0, 300.0])
        .show(ctx, |ui| {
            crate::gui::widgets::search_filter_bar(ui, &mut state.actor_browser_search, "Search Actor classes...");
            ui.separator();

            // Tree Scroll Area
            let search = state.actor_browser_search.to_lowercase();
            let mut selected_class = None;
            let mut place_class = None;
            let mut drag_payload = None;

            egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                if let Some(ref tree) = state.class_tree.clone() {
                    render_class_tree_node(ui, tree, &search, state.selected_class_name.as_deref(), &mut selected_class, &mut place_class, &mut drag_payload);
                } else {
                    ui.label("Loading class hierarchy...");
                }
            });

            if let Some(payload) = drag_payload {
                state.active_drag_drop = Some(payload);
            }

            if let Some((cls, pkg)) = selected_class {
                state.selected_class_name = Some(cls);
                state.selected_class_package = Some(pkg);
            }

            ui.add_space(8.0);
            ui.separator();

            // Selected Class Details and Placement
            if let Some(ref cls) = state.selected_class_name.clone() {
                let pkg = state.selected_class_package.as_deref().unwrap_or("Engine");
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Selected Class:").strong());
                    ui.label(RichText::new(cls).color(Color32::from_rgb(100, 220, 255)).strong());
                    ui.label(format!("({})", pkg));
                });

                ui.add_space(6.0);
                let place_btn = egui::Button::new(
                    RichText::new(format!("➕ Place '{}' at Builder Brush", cls))
                        .color(Color32::from_rgb(120, 240, 160))
                        .strong(),
                );
                if ui.add_sized([ui.available_width(), 28.0], place_btn).clicked() {
                    place_class = Some(cls.clone());
                }

                let replace_btn = egui::Button::new(
                    RichText::new(format!("🔄 Replace Selected Actor(s) With '{}'", cls))
                        .color(Color32::from_rgb(255, 200, 110))
                        .strong(),
                );
                if ui
                    .add_sized([ui.available_width(), 26.0], replace_btn)
                    .on_hover_text("UnrealEd 2.0 Replace With (REQ-C05): Replaces selected actor(s) with this class while preserving Location, Rotation, Tag, and Event")
                    .clicked()
                {
                    let full_cls = if cls.contains('.') {
                        cls.clone()
                    } else {
                        format!("{}.{}", pkg, cls)
                    };
                    state.replace_selected_actors_with_class(&full_cls);
                }

                let drag_btn = egui::Button::new(
                    RichText::new(format!("🎯 Drag '{}' to 3D Viewport", cls))
                        .color(Color32::from_rgb(100, 210, 255))
                        .strong(),
                );
                let drag_res = ui.add_sized([ui.available_width(), 26.0], drag_btn)
                    .on_hover_text("Drag this button directly into the 3D Viewport to drop and place this actor at the cursor location");
                if drag_res.drag_started() || (drag_res.is_pointer_button_down_on() && drag_res.hovered()) {
                    state.active_drag_drop = Some(crate::editor_state::DragDropPayload::ActorClass(cls.clone()));
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("📜 View Script").on_hover_text("Open UnrealScript Class Source Code Viewer").clicked() {
                        state.open_uc_viewer_for_class(cls);
                        state.uc_viewer_tab = crate::unrealscript::UcViewerTab::Source;
                    }
                    if ui.button("⚙ Default Properties").on_hover_text("Open Live DefaultProperties Inspector & Hot-Patcher").clicked() {
                        state.open_uc_viewer_for_class(cls);
                        state.uc_viewer_tab = crate::unrealscript::UcViewerTab::DefaultProperties;
                    }
                });
            } else {
                ui.label("Select a class from the hierarchy tree above.");
            }

            if let Some(cls) = place_class {
                state.place_actor_of_class(&cls);
            }
        });

    state.show_actor_browser = open;
}

fn class_icon(name: &str) -> &'static str {
    if name.contains("Light") {
        "💡"
    } else if name.contains("PlayerStart") {
        "🚩"
    } else if name.contains("PathNode") || name.contains("Teleporter") {
        "🔷"
    } else if name.contains("Weapon") {
        "🔫"
    } else if name.contains("Pickup") || name.contains("MedBox") || name.contains("Armor") {
        "💊"
    } else if name.contains("Pawn") || name.contains("Bot") || name.contains("Player") {
        "👾"
    } else if name.contains("ZoneInfo") {
        "🌐"
    } else if name.contains("Mover") {
        "🚪"
    } else if name.contains("Trigger") {
        "⚡"
    } else {
        "📄"
    }
}

fn render_class_tree_node(
    ui: &mut egui::Ui,
    node: &ClassTreeNode,
    search: &str,
    selected_name: Option<&str>,
    selected_out: &mut Option<(String, String)>,
    place_out: &mut Option<String>,
    drag_out: &mut Option<crate::editor_state::DragDropPayload>,
) {
    let name_matches = search.is_empty() || node.name.to_lowercase().contains(search);
    let child_matches = node.children.iter().any(|c| has_matching_child(c, search));

    if !name_matches && !child_matches {
        return;
    }

    let is_selected = selected_name == Some(&node.name);
    let icon = class_icon(&node.name);
    let label_text = format!("{} {}", icon, node.name);

    if node.children.is_empty() {
        let text = if is_selected {
            RichText::new(label_text).color(Color32::from_rgb(255, 230, 80)).strong()
        } else {
            RichText::new(label_text)
        };
        let res = ui.selectable_label(is_selected, text);
        if res.drag_started() {
            *drag_out = Some(crate::editor_state::DragDropPayload::ActorClass(node.name.clone()));
            *selected_out = Some((node.name.clone(), node.package.clone()));
        } else if res.double_clicked() {
            *selected_out = Some((node.name.clone(), node.package.clone()));
            *place_out = Some(node.name.clone());
        } else if res.clicked() {
            *selected_out = Some((node.name.clone(), node.package.clone()));
        }
    } else {
        let id = ui.make_persistent_id(&node.name);
        let default_open = !search.is_empty() || node.name == "Object" || node.name == "Actor";

        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
            .show_header(ui, |ui| {
                let text = if is_selected {
                    RichText::new(label_text).color(Color32::from_rgb(255, 230, 80)).strong()
                } else {
                    RichText::new(label_text)
                };
                let res = ui.selectable_label(is_selected, text);
                if res.drag_started() {
                    *drag_out = Some(crate::editor_state::DragDropPayload::ActorClass(node.name.clone()));
                    *selected_out = Some((node.name.clone(), node.package.clone()));
                } else if res.double_clicked() {
                    *selected_out = Some((node.name.clone(), node.package.clone()));
                    *place_out = Some(node.name.clone());
                } else if res.clicked() {
                    *selected_out = Some((node.name.clone(), node.package.clone()));
                }
            })
            .body(|ui| {
                for child in &node.children {
                    render_class_tree_node(ui, child, search, selected_name, selected_out, place_out, drag_out);
                }
            });
    }
}

fn has_matching_child(node: &ClassTreeNode, search: &str) -> bool {
    if node.name.to_lowercase().contains(search) {
        return true;
    }
    node.children.iter().any(|c| has_matching_child(c, search))
}
