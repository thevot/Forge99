// crates/ut99-editor/src/gui/action_wizards.rs
//
// Creation Hub & Action Wizards
// Consolidates complex multi-step game design tasks into intuitive guided wizards:
// 1. Mover & Elevator Wizard (sliding doors, lifts, vertical transit + trigger wiring)
// 2. Teleporter Pair Wizard (entrance/exit teleporters linked by URL -> Tag)
// 3. Water & Hazard Pool Wizard (carves fluid pit, adds portal water sheet, WaterZone)
// 4. Jump Pad / Kicker Wizard (kicker pad + landmark target + parabolic physics)
// 5. Room & Corridor Architect Wizard (rapid room carving + atmospheric lights)

use egui::{Color32, RichText, Ui};
use crate::editor_state::{EditorState, WizardTab};
use crate::gui::theme::{ProTooltipExt, ThemeColors};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_action_wizards_dialog {
        return;
    }

    let mut open = state.show_action_wizards_dialog;

    crate::gui::modal::standard_dialog_window(
        ctx,
        "✨ Creation Hub & Action Wizards",
        &mut open,
        [740.0, 580.0],
        [560.0, 380.0],
    )
    .show(ctx, |ui| {
        crate::gui::modal::dialog_scroll_body(ui, |ui| {
            render_wizard_hub(ui, state);
        });
    });

    state.show_action_wizards_dialog = open;
}

fn render_wizard_hub(ui: &mut Ui, state: &mut EditorState) {
    // 0. Quick Banner for Full Valid UT99 Map Generator Wizard
    ui.horizontal(|ui| {
        let btn = ui.button(
            RichText::new("🧙 Open Full Valid UT99 Map Generator Wizard (Ctrl+Shift+N)")
                .strong()
                .color(Color32::from_rgb(130, 220, 255)),
        );
        if btn.clicked() {
            state.show_map_wizard_dialog = true;
            if state.available_official_maps.is_empty() {
                state.scan_official_maps_for_wizard();
            }
        }
        btn.pro_tooltip(
            "New Valid UT99 Map Generator Wizard",
            Some("Ctrl+Shift+N"),
            "Generate complete, watertight, playtest-ready DM / CTF / DOM / AS UT99 maps with Official .unr DNA Cloning",
        );
    });
    ui.separator();

    // 1. Wizard Tabs Header
    ui.horizontal(|ui| {
        let tabs = [
            (WizardTab::MoverLift, "🚪 Mover & Lift", "Create animated doors, elevators and platforms with automatic trigger wiring"),
            (WizardTab::TeleporterPair, "🌀 Teleporters", "Create matched pairs of teleporters linked by URL and Tag"),
            (WizardTab::WaterBasin, "🌊 Water & Hazards", "Carve pools of water, toxic slime or lava with fluid zones"),
            (WizardTab::JumpPad, "🎯 Jump Pad", "Place kicker pads calibrated to land players at exact destinations"),
            (WizardTab::RoomCorridor, "🏗 Room Architect", "Carve complete rooms and corridors with lights and texture themes"),
        ];

        for (tab, label, tip) in tabs {
            let is_sel = state.active_wizard_tab == tab;
            let btn = ui.selectable_label(is_sel, RichText::new(label).strong());
            if btn.clicked() {
                state.active_wizard_tab = tab;
            }
            btn.pro_tooltip(label, None, tip);
        }
    });

    ui.separator();

    // 2. Active Wizard Content
    match state.active_wizard_tab {
        WizardTab::MoverLift => render_mover_lift_wizard(ui, state),
        WizardTab::TeleporterPair => render_teleporter_pair_wizard(ui, state),
        WizardTab::WaterBasin => render_water_basin_wizard(ui, state),
        WizardTab::JumpPad => render_jump_pad_wizard(ui, state),
        WizardTab::RoomCorridor => render_room_architect_wizard(ui, state),
    }
}

// ---------------------------------------------------------------------------
// 1. Mover & Lift Wizard
// ---------------------------------------------------------------------------
fn render_mover_lift_wizard(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🚪 Interactive Mover & Lift Wizard");
    ui.label(RichText::new("Configure and generate animated doors, moving platforms, or multi-floor elevators. All keyframes and Trigger wiring are generated automatically.").color(ThemeColors::TEXT_SECONDARY));
    ui.add_space(8.0);

    // Style Presets
    ui.group(|ui| {
        ui.label(RichText::new("1. Choose Mover Archetype:").strong());
        ui.horizontal_wrapped(|ui| {
            if ui.button("↕ Hydraulic Lift (+256uu)").clicked() {
                state.wizard_mover_height = 256.0;
                state.wizard_mover_horizontal = 0.0;
                state.wizard_mover_delay = 4.0;
                state.wizard_mover_tag = "Elevator_01".into();
            }
            if ui.button("↕ Tower Express Lift (+512uu)").clicked() {
                state.wizard_mover_height = 512.0;
                state.wizard_mover_horizontal = 0.0;
                state.wizard_mover_delay = 5.0;
                state.wizard_mover_tag = "TowerLift".into();
            }
            if ui.button("↔ Sliding Door (+192uu X)").clicked() {
                state.wizard_mover_height = 0.0;
                state.wizard_mover_horizontal = 192.0;
                state.wizard_mover_delay = 3.0;
                state.wizard_mover_tag = "Door_Slide".into();
            }
            if ui.button("⏹ Trapdoor / Drop Floor (-256uu)").clicked() {
                state.wizard_mover_height = -256.0;
                state.wizard_mover_horizontal = 0.0;
                state.wizard_mover_delay = 6.0;
                state.wizard_mover_tag = "Trapdoor".into();
            }
        });
    });

    ui.add_space(8.0);

    // Parameters
    ui.group(|ui| {
        ui.label(RichText::new("2. Movement & Motion Parameters:").strong());
        ui.horizontal(|ui| {
            ui.label("Vertical Travel (Z):");
            ui.add(egui::DragValue::new(&mut state.wizard_mover_height).speed(16.0).suffix(" uu"));
            ui.separator();
            ui.label("Horizontal Travel (X/Y):");
            ui.add(egui::DragValue::new(&mut state.wizard_mover_horizontal).speed(16.0).suffix(" uu"));
        });
        ui.horizontal(|ui| {
            ui.label("Stay Open / Return Delay:");
            ui.add(egui::DragValue::new(&mut state.wizard_mover_delay).speed(0.5).range(0.5..=30.0).suffix(" sec"));
            ui.separator();
            ui.label("Identifier Tag:");
            ui.text_edit_singleline(&mut state.wizard_mover_tag);
        });
    });

    ui.add_space(12.0);

    // Execution Banner
    ui.horizontal(|ui| {
        let spawn_btn = ui.button(RichText::new("🚀 Generate Mover & Trigger Network").strong().color(Color32::WHITE));
        if spawn_btn.clicked() {
            state.generate_wizard_lift();
        }
        ui.label(RichText::new("Creates platform brush, keyframes 0/1, and wires Trigger actor.").size(11.0).color(ThemeColors::TEXT_MUTED));
    });
}

// ---------------------------------------------------------------------------
// 2. Teleporter Pair Wizard
// ---------------------------------------------------------------------------
fn render_teleporter_pair_wizard(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🌀 Teleporter Pair Wizard");
    ui.label(RichText::new("Create an entrance and exit teleporter pair with automatic URL -> Tag destination link.").color(ThemeColors::TEXT_SECONDARY));
    ui.add_space(8.0);

    ui.group(|ui| {
        ui.label(RichText::new("1. Connection Settings:").strong());
        ui.horizontal(|ui| {
            ui.label("Entrance Tag:");
            ui.text_edit_singleline(&mut state.wizard_teleport_enter_tag);
            ui.separator();
            ui.label("Exit Destination Tag (URL):");
            ui.text_edit_singleline(&mut state.wizard_teleport_exit_tag);
        });
        ui.horizontal(|ui| {
            ui.label("Separation Offset (X, Y, Z):");
            ui.add(egui::DragValue::new(&mut state.wizard_teleport_offset_x).speed(64.0).suffix(" uu"));
            ui.add(egui::DragValue::new(&mut state.wizard_teleport_offset_y).speed(64.0).suffix(" uu"));
            ui.add(egui::DragValue::new(&mut state.wizard_teleport_offset_z).speed(32.0).suffix(" uu"));
        });
    });

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        let spawn_btn = ui.button(RichText::new("✨ Generate Linked Teleporter Pair").strong().color(Color32::WHITE));
        if spawn_btn.clicked() {
            state.generate_wizard_teleporters();
        }
        ui.label(RichText::new("Spawns Entrance at builder brush, Exit at destination offset.").size(11.0).color(ThemeColors::TEXT_MUTED));
    });
}

// ---------------------------------------------------------------------------
// 3. Water & Hazard Basin Wizard
// ---------------------------------------------------------------------------
fn render_water_basin_wizard(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🌊 Water & Hazard Pool Wizard");
    ui.label(RichText::new("Carve a fluid basin into the floor, place the translucent surface sheet, and inject the physics ZoneInfo.").color(ThemeColors::TEXT_SECONDARY));
    ui.add_space(8.0);

    ui.group(|ui| {
        ui.label(RichText::new("1. Fluid Type:").strong());
        ui.horizontal(|ui| {
            if ui.button("💧 Clear Water (Swimming)").clicked() {
                state.wizard_basin_type = 0; // Water
            }
            if ui.button("🧪 Toxic Slime (10 dmg/sec)").clicked() {
                state.wizard_basin_type = 1; // Slime
            }
            if ui.button("🔥 Molten Lava (50 dmg/sec)").clicked() {
                state.wizard_basin_type = 2; // Lava
            }
        });
    });

    ui.add_space(8.0);

    ui.group(|ui| {
        ui.label(RichText::new("2. Pool Dimensions:").strong());
        ui.horizontal(|ui| {
            ui.label("Width (X):");
            ui.add(egui::DragValue::new(&mut state.wizard_basin_width).speed(32.0).range(128.0..=4096.0).suffix(" uu"));
            ui.separator();
            ui.label("Length (Y):");
            ui.add(egui::DragValue::new(&mut state.wizard_basin_length).speed(32.0).range(128.0..=4096.0).suffix(" uu"));
            ui.separator();
            ui.label("Depth (Z):");
            ui.add(egui::DragValue::new(&mut state.wizard_basin_depth).speed(16.0).range(32.0..=2048.0).suffix(" uu"));
        });
    });

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        let spawn_btn = ui.button(RichText::new("🌊 Carve Basin & Setup Fluid Zone").strong().color(Color32::WHITE));
        if spawn_btn.clicked() {
            state.generate_wizard_water_basin();
        }
        ui.label(RichText::new("CSG subtracts pool, adds water portal sheet, and configures WaterZone.").size(11.0).color(ThemeColors::TEXT_MUTED));
    });
}

// ---------------------------------------------------------------------------
// 4. Jump Pad / Kicker Wizard
// ---------------------------------------------------------------------------
fn render_jump_pad_wizard(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🎯 Jump Pad / Kicker Physics Wizard");
    ui.label(RichText::new("Place a tournament JumpPad or Kicker actor with launch velocity calibrated for perfect parabolic flight.").color(ThemeColors::TEXT_SECONDARY));
    ui.add_space(8.0);

    ui.group(|ui| {
        ui.label(RichText::new("1. Trajectory Targets:").strong());
        ui.horizontal(|ui| {
            ui.label("Apex Jump Height (Z):");
            ui.add(egui::DragValue::new(&mut state.wizard_jump_height).speed(16.0).range(64.0..=4096.0).suffix(" uu"));
            ui.separator();
            ui.label("Forward Travel (X):");
            ui.add(egui::DragValue::new(&mut state.wizard_jump_dist).speed(32.0).range(0.0..=8192.0).suffix(" uu"));
        });

        // Compute simulated launch vector
        let gravity = 950.0_f32; // UT99 standard gravity
        let vz = (2.0 * gravity * state.wizard_jump_height).sqrt();
        let time_apex = vz / gravity;
        let vx = if time_apex > 0.001 { state.wizard_jump_dist / (time_apex * 2.0) } else { 0.0 };

        ui.add_space(4.0);
        ui.label(RichText::new(format!("Calculated KickVelocity: ({:.0}, 0, {:.0}) | Flight Time: {:.2}s", vx, vz, time_apex * 2.0)).color(ThemeColors::ACCENT_AZURE));
    });

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        let spawn_btn = ui.button(RichText::new("🎯 Place Kicker & Calibrate Physics").strong().color(Color32::WHITE));
        if spawn_btn.clicked() {
            state.generate_wizard_jump_pad();
        }
        ui.label(RichText::new("Spawns Kicker pad at builder location with target PathNode landmark.").size(11.0).color(ThemeColors::TEXT_MUTED));
    });
}

// ---------------------------------------------------------------------------
// 5. Room & Corridor Generator Wizard
// ---------------------------------------------------------------------------
fn render_room_architect_wizard(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🏗 Room & Corridor Architect Wizard");
    ui.label(RichText::new("Instantly carve hollow arena rooms or connecting corridors with walls and balanced light placement.").color(ThemeColors::TEXT_SECONDARY));
    ui.add_space(8.0);

    ui.group(|ui| {
        ui.label(RichText::new("1. Room Dimensions:").strong());
        ui.horizontal(|ui| {
            ui.label("Width (X):");
            ui.add(egui::DragValue::new(&mut state.wizard_room_width).speed(64.0).range(256.0..=8192.0).suffix(" uu"));
            ui.separator();
            ui.label("Length (Y):");
            ui.add(egui::DragValue::new(&mut state.wizard_room_length).speed(64.0).range(256.0..=8192.0).suffix(" uu"));
            ui.separator();
            ui.label("Height (Z):");
            ui.add(egui::DragValue::new(&mut state.wizard_room_height).speed(32.0).range(128.0..=4096.0).suffix(" uu"));
        });
    });

    ui.add_space(8.0);

    ui.group(|ui| {
        ui.label(RichText::new("2. Atmospheric Lighting:").strong());
        ui.horizontal(|ui| {
            ui.radio_value(&mut state.wizard_room_lights, 0, "No Lights");
            ui.radio_value(&mut state.wizard_room_lights, 1, "Center Chandelier (1 Light)");
            ui.radio_value(&mut state.wizard_room_lights, 2, "Corner Grid (4 Symmetrical Lights)");
        });
    });

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        let spawn_btn = ui.button(RichText::new("🏗 Carve Room & Place Atmospheric Lights").strong().color(Color32::WHITE));
        if spawn_btn.clicked() {
            state.generate_wizard_room();
        }
        ui.label(RichText::new("Performs CSG subtract and places atmospheric lights in level.").size(11.0).color(ThemeColors::TEXT_MUTED));
    });
}
