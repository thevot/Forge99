use crate::editor_state::EditorState;
use egui::{Color32, DragValue, RichText, Ui, Window};
use ut99_bsp::CtfSymmetryMode;

/// Interactive Automated Symmetrical CTF Base Generator Dialog
pub fn show_ctf_symmetry_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_ctf_symmetry_dialog {
        return;
    }

    let mut is_open = state.show_ctf_symmetry_dialog;
    let mut close_requested = false;
    Window::new("Symmetrical CTF Base Generator")
        .open(&mut is_open)
        .default_width(520.0)
        .default_height(400.0)
        .resizable(true)
        .show(ctx, |ui| {
            render_ctf_symmetry_content(ui, state, &mut close_requested);
        });

    state.show_ctf_symmetry_dialog = is_open && !close_requested;
}

fn render_ctf_symmetry_content(ui: &mut Ui, state: &mut EditorState, close_requested: &mut bool) {
    ui.horizontal(|ui| {
        ui.heading("🚩 Symmetrical CTF Base Generator");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✖ Close").clicked() {
                *close_requested = true;
            }
        });
    });

    ui.separator();

    ui.label(
        "Automatically duplicates and reflects the current Red Base geometry to form a mathematically \
        perfect, balanced Blue Base arena (or vice versa), with automated texture remapping and winding repair."
    );

    ui.add_space(8.0);

    // Symmetry Mode
    ui.group(|ui| {
        ui.label(RichText::new("Symmetry Transformation Mode:").strong());

        let mode = &mut state.ctf_symmetry_config.mode;
        ui.radio_value(mode, CtfSymmetryMode::Rotational180, "Rotational 180° (Point Reflection / CTF-Face style)");
        ui.radio_value(mode, CtfSymmetryMode::MirrorX, "Mirror X (Planar Reflection across X axis / CTF-November style)");
        ui.radio_value(mode, CtfSymmetryMode::MirrorY, "Mirror Y (Planar Reflection across Y axis)");
    });

    ui.add_space(8.0);

    // Pivot Point
    ui.group(|ui| {
        ui.label(RichText::new("Symmetry Pivot / Center Point (UU):").strong());
        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(DragValue::new(&mut state.ctf_symmetry_config.center.x).speed(16.0));
            ui.label("Y:");
            ui.add(DragValue::new(&mut state.ctf_symmetry_config.center.y).speed(16.0));
            ui.label("Z:");
            ui.add(DragValue::new(&mut state.ctf_symmetry_config.center.z).speed(16.0));

            if ui.button("🎯 Origin (0,0,0)").clicked() {
                state.ctf_symmetry_config.center = glam::Vec3::ZERO;
            }
        });
    });

    ui.add_space(8.0);

    // Options
    ui.group(|ui| {
        ui.label(RichText::new("Generation Options:").strong());

        ui.checkbox(
            &mut state.ctf_symmetry_config.remap_textures,
            "Auto-remap Team Textures (e.g. Red_* ↔ Blue_*, cf_red ↔ cf_blue)",
        );

        ui.horizontal(|ui| {
            ui.label("Brush Name Filter (Optional):");
            let mut filter_str = state.ctf_symmetry_config.prefix_filter.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut filter_str).changed() {
                state.ctf_symmetry_config.prefix_filter = if filter_str.trim().is_empty() {
                    None
                } else {
                    Some(filter_str.trim().to_string())
                };
            }
        });
    });

    ui.add_space(12.0);

    // Action button
    let total_brushes = state.world_brushes.len();
    if ui.button(RichText::new(format!("🚩 Duplicate & Generate Blue Base (From {} Brushes)", total_brushes)).strong().size(15.0)).clicked() {
        let config = state.ctf_symmetry_config.clone();
        let res = state.apply_symmetrical_ctf_base(&config);
        state.log(format!(
            "CTF Base Generated: {} symmetrical brushes added, {} textures remapped, {} polygons winding-inverted.",
            res.transformed_brush_count,
            res.remapped_texture_polys_count,
            res.inverted_winding_polys_count
        ));
    }

    ui.separator();
    ui.colored_label(
        Color32::from_rgb(140, 200, 140),
        "💡 Tip: Planar mirroring automatically inverts polygon vertex windings to guarantee outward-pointing normals.",
    );
}
