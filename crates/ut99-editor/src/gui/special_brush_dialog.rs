use crate::editor_state::{EditorState, SpecialBrushPreset};
use egui::Color32;
use ut99_bsp::{
    PF_ANTI_PORTAL, PF_FAKE_BACKDROP, PF_INVISIBLE, PF_MASKED, PF_MIRRORED, PF_MODULATED,
    PF_NOT_SOLID, PF_PORTAL, PF_SEMISOLID, PF_TRANSLUCENT, PF_TWO_SIDED, PF_UNLIT,
};

pub fn render_special_brush_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_add_special_dialog {
        return;
    }

    let mut open = state.show_add_special_dialog;
    let mut close_requested = false;
    let mut add_requested = false;
    crate::gui::modal::compact_dialog_window(ctx, "Add Special Brush - UnrealEd 2 Paradigm", &mut open)
        .show(ctx, |ui| {
            ui.heading("Prefabs (UnrealEd 2.0 Canonical)");
            ui.separator();

            egui::Grid::new("special_presets_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    for (idx, preset) in SpecialBrushPreset::ALL.iter().enumerate() {
                        let is_selected = state.special_brush_flags == preset.poly_flags()
                            && !state.special_is_subtractive;
                        if ui
                            .selectable_label(is_selected, preset.label())
                            .clicked()
                        {
                            state.apply_special_brush_preset(*preset);
                        }
                        if idx % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });

            ui.add_space(8.0);
            ui.heading("Solidity");
            ui.separator();
            let mut flags = state.special_brush_flags;
            let current_solidity = if (flags & PF_NOT_SOLID) != 0 {
                2u8 // Non-Solid
            } else if (flags & PF_SEMISOLID) != 0 {
                1u8 // Semi-Solid
            } else {
                0u8 // Solid
            };
            let mut sel_solidity = current_solidity;
            ui.horizontal(|ui| {
                ui.radio_value(&mut sel_solidity, 0, "🟦 Solid (Cuts BSP)");
                ui.radio_value(&mut sel_solidity, 1, "🟪 Semi-Solid (No BSP Cuts)");
                ui.radio_value(&mut sel_solidity, 2, "🟩 Non-Solid (Pass-Through)");
            });
            if sel_solidity != current_solidity {
                flags &= !(PF_SEMISOLID | PF_NOT_SOLID);
                match sel_solidity {
                    1 => flags |= PF_SEMISOLID,
                    2 => flags |= PF_NOT_SOLID,
                    _ => {}
                }
            }

            ui.add_space(8.0);
            ui.heading("CSG Operation");
            ui.separator();
            ui.horizontal(|ui| {
                ui.radio_value(&mut state.special_is_subtractive, false, "Add (Solid)");
                ui.radio_value(&mut state.special_is_subtractive, true, "Subtract (Hollow)");
            });

            ui.add_space(8.0);
            ui.heading("Polygon Surface Flags");
            ui.separator();

            fn flag_checkbox(ui: &mut egui::Ui, flags: &mut u32, label: &str, bit: u32) {
                let mut checked = (*flags & bit) != 0;
                if ui.checkbox(&mut checked, label).changed() {
                    if checked {
                        *flags |= bit;
                    } else {
                        *flags &= !bit;
                    }
                }
            }

            egui::Grid::new("poly_flags_grid").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                flag_checkbox(ui, &mut flags, "Invisible (PF_Invisible)", PF_INVISIBLE);
                flag_checkbox(ui, &mut flags, "Masked (PF_Masked)", PF_MASKED);
                ui.end_row();

                flag_checkbox(ui, &mut flags, "Translucent (PF_Translucent)", PF_TRANSLUCENT);
                flag_checkbox(ui, &mut flags, "Modulated (PF_Modulated)", PF_MODULATED);
                ui.end_row();

                flag_checkbox(ui, &mut flags, "Zone Portal (PF_Portal)", PF_PORTAL);
                flag_checkbox(ui, &mut flags, "Mirrored (PF_Mirrored)", PF_MIRRORED);
                ui.end_row();

                flag_checkbox(ui, &mut flags, "Two-Sided (PF_TwoSided)", PF_TWO_SIDED);
                flag_checkbox(ui, &mut flags, "Fake Backdrop (PF_FakeBackdrop)", PF_FAKE_BACKDROP);
                ui.end_row();

                flag_checkbox(ui, &mut flags, "Unlit (PF_Unlit)", PF_UNLIT);
                flag_checkbox(ui, &mut flags, "Anti-Portal (PF_AntiPortal)", PF_ANTI_PORTAL);
                ui.end_row();

                ui.label(format!("Flags: {:#010X}", flags));
                ui.end_row();
            });

            state.special_brush_flags = flags;

            ui.add_space(12.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("Add Special Brush").color(Color32::from_rgb(100, 220, 255)).strong()).clicked() {
                    add_requested = true;
                }
                if ui.button("Cancel").clicked() {
                    close_requested = true;
                }
            });
        });

    if add_requested {
        state.add_special_brush(state.special_brush_flags, state.special_is_subtractive);
        close_requested = true;
    }
    state.show_add_special_dialog = open && !close_requested;
}
