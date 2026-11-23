use egui::{self, Color32, RichText};
use crate::editor_state::{EditorState, TextureMatchMode, TextureReplaceScope};

/// Renders the Find and Replace Textures Dialog (UnrealEd 2.0 Parity & Supremacy).
pub fn render_replace_textures_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_replace_textures_dialog {
        return;
    }

    let mut open = state.show_replace_textures_dialog;
    let mut close_requested = false;

    crate::gui::modal::standard_dialog_window(
        ctx,
        RichText::new("Find & Replace Textures (UnrealEd 2.0 Parity)").strong(),
        &mut open,
        [460.0, 500.0],
        [380.0, 340.0],
    )
    .show(ctx, |ui| {
            ui.add_space(4.0);

            // 1. Target Scope Description
            let scope_info = match state.replace_texture_scope {
                TextureReplaceScope::EntireLevel => {
                    format!("Scope: Entire Level ({} world brushes)", state.world_brushes.len())
                }
                TextureReplaceScope::SelectedBrushes => {
                    let sel_count = state.get_selected_brush_indices().len();
                    format!("Scope: Selected Brushes ({} selected)", sel_count)
                }
                TextureReplaceScope::SelectedSurfaces => {
                    format!("Scope: Selected Surfaces ({} selected)", state.selected_surfaces.len())
                }
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(scope_info).strong().color(Color32::from_rgb(100, 200, 255)));
            });
            ui.separator();

            // 2. Texture Inputs
            ui.label(RichText::new("Search & Replacement Criteria:").strong());

            ui.horizontal(|ui| {
                ui.label("Find:");
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut state.replace_texture_find)
                        .hint_text("Texture to find (e.g. GenEarth.Dirt1 or Dirt1)")
                        .desired_width(240.0)
                );
                if ui.button("Use Active").on_hover_text("Use texture currently selected in Texture Browser").clicked() {
                    if let Some(ref tex) = state.active_texture {
                        state.replace_texture_find = tex.clone();
                    }
                }
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Enter pressed in find
                }
            });

            // Quick Swap Button
            ui.horizontal(|ui| {
                ui.add_space(38.0);
                if ui.button("⇅ Swap Find / Replace").on_hover_text("Invert Find and Replace textures").clicked() {
                    std::mem::swap(&mut state.replace_texture_find, &mut state.replace_texture_with);
                }
            });

            ui.horizontal(|ui| {
                ui.label("With:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.replace_texture_with)
                        .hint_text("Replacement texture name or package")
                        .desired_width(240.0)
                );
                if ui.button("Use Active").on_hover_text("Use texture currently selected in Texture Browser").clicked() {
                    if let Some(ref tex) = state.active_texture {
                        state.replace_texture_with = tex.clone();
                    }
                }
            });

            ui.separator();

            // 3. Scope & Mode Selectors
            ui.columns(2, |columns| {
                columns[0].label(RichText::new("Target Scope:").strong());
                columns[0].radio_value(&mut state.replace_texture_scope, TextureReplaceScope::EntireLevel, "Entire Level");
                columns[0].radio_value(&mut state.replace_texture_scope, TextureReplaceScope::SelectedBrushes, "Selected Brushes");
                columns[0].radio_value(&mut state.replace_texture_scope, TextureReplaceScope::SelectedSurfaces, "Selected Surfaces");

                columns[1].label(RichText::new("Matching Mode:").strong());
                columns[1].radio_value(&mut state.replace_texture_match_mode, TextureMatchMode::ExactName, "Exact Name");
                columns[1].radio_value(&mut state.replace_texture_match_mode, TextureMatchMode::PackageOnly, "Package Only");
                columns[1].radio_value(&mut state.replace_texture_match_mode, TextureMatchMode::ContainsSubstring, "Substring Match");
            });

            ui.separator();

            // 4. Live Preview Banner
            let preview_count = state.preview_replace_textures_count(
                &state.replace_texture_find,
                state.replace_texture_scope,
                state.replace_texture_match_mode,
            );

            ui.horizontal(|ui| {
                if preview_count > 0 {
                    ui.label(
                        RichText::new(format!("⚡ {} surface(s) match criteria and will be modified", preview_count))
                            .strong()
                            .color(Color32::from_rgb(255, 205, 50))
                    );
                } else {
                    ui.label(
                        RichText::new("ℹ No surfaces match current search criteria.")
                            .italics()
                            .color(Color32::from_rgb(160, 160, 160))
                    );
                }
            });

            ui.separator();

            // 5. Action Buttons
            ui.horizontal(|ui| {
                let can_replace = !state.replace_texture_find.trim().is_empty()
                    && !state.replace_texture_with.trim().is_empty()
                    && preview_count > 0;

                let replace_btn = ui.add_enabled(
                    can_replace,
                    egui::Button::new(RichText::new("🔁 Replace All Matching").strong())
                );

                if replace_btn.clicked() {
                    let find = state.replace_texture_find.clone();
                    let replace = state.replace_texture_with.clone();
                    state.replace_textures(&find, &replace, state.replace_texture_scope, state.replace_texture_match_mode);
                }

                if ui.button("Close").clicked() {
                    close_requested = true;
                }
            });
        });

    state.show_replace_textures_dialog = open && !close_requested;
}
