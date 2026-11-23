// crates/ut99-editor/src/gui/visual_presets_dialog.rs
//
// Visual Presets Catalog
// 1-click curated visual & gameplay presets for instant application:
// - Lighting Styles (Cyberpunk Neon, Gothic Cathedral, Industrial Tech, Alien Hive, Red Alert)
// - Surface & Material Shaders (Water, Grate Masked, Translucent Glass, Unlit Lava, Non-Solid Portal)
// - Gameplay Node Setups (Powerup Station, Sniper Roost, Hazard Cluster)

use egui::{RichText, Ui};
use crate::editor_state::EditorState;
use crate::gui::theme::{ProTooltipExt, ThemeColors};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_visual_presets_dialog {
        return;
    }

    let mut open = state.show_visual_presets_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "🎨 Visual Presets Catalog", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_presets_content(ui, state);
            });
        });

    state.show_visual_presets_dialog = open;
}

fn render_presets_content(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🎨 Visual & Material Presets");
    ui.label(RichText::new("Apply curated artistic themes and physical material shaders to selected lights and surfaces with 1 click.").color(ThemeColors::TEXT_SECONDARY));
    ui.add_space(8.0);

    // Section 1: Lighting Presets
    ui.group(|ui| {
        ui.label(RichText::new("💡 Lighting Atmosphere Presets:").strong());
        ui.label(RichText::new("Select any Light actor and click a preset to apply colors, radius, and animation effects.").size(11.0).color(ThemeColors::TEXT_MUTED));
        ui.add_space(4.0);

        let light_presets = [
            ("🔷 Neon Cyberpunk (Cyan)", 135, 10, 180, 40, 0, "Cyan neon glow for modern industrial and urban maps"),
            ("🔶 Gothic Cathedral (Warm Amber)", 28, 40, 140, 36, 1, "Warm torch/candlelight with subtle organic flicker"),
            ("⚪ Industrial Tech (Cool White)", 150, 15, 160, 48, 0, "Clean high-intensity warehouse and laboratory illumination"),
            ("🟢 Skaarj Alien Hive (Toxic Green)", 85, 0, 190, 32, 2, "Alien bio-luminescence with slow pulsing strobe"),
            ("🔴 Red Alert (Emergency Beacon)", 0, 0, 210, 44, 3, "Urgent crimson strobe for reactor cores and hazard chambers"),
        ];

        for (name, hue, sat, bright, rad, effect, desc) in light_presets {
            ui.horizontal(|ui| {
                let btn = ui.button(name);
                if btn.clicked() {
                    state.apply_lighting_preset(hue, sat, bright, rad, effect);
                }
                btn.pro_tooltip(name, None, desc);
                ui.label(RichText::new(desc).size(10.5).color(ThemeColors::TEXT_MUTED));
            });
        }
    });

    ui.add_space(8.0);

    // Section 2: Surface Material Presets
    ui.group(|ui| {
        ui.label(RichText::new("🧱 Surface & Material Shader Presets:").strong());
        ui.label(RichText::new("Select any brush surface(s) (Shift+Click or F5) and click to apply canonical Unreal flags.").size(11.0).color(ThemeColors::TEXT_MUTED));
        ui.add_space(4.0);

        let surface_presets: [(&str, u32, &str); 6] = [
            ("🌊 Water Surface", ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_SMALL_WAVY | ut99_bsp::PF_NOT_SOLID, "Translucent animated water sheet with wave disturbance"),
            ("⛓ Masked Grate / Catwalk", ut99_bsp::PF_MASKED | ut99_bsp::PF_TWO_SIDED, "Two-sided metallic grating with 1-bit alpha transparency"),
            ("🪟 Translucent Glass", ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_TWO_SIDED, "Translucent reflective window pane"),
            ("🔥 Molten Lava / Magma", ut99_bsp::PF_UNLIT, "Full-bright self-illuminated volcanic surface"),
            ("🌫 Non-Solid Zone Portal", ut99_bsp::PF_PORTAL | ut99_bsp::PF_NOT_SOLID, "Invisible boundary dividing level acoustic and fog zones"),
            ("🚫 Invisible Collision Barrier", ut99_bsp::PF_INVISIBLE, "Walk-blocking invisible wall to prevent map exploits"),
        ];

        for (name, flags, desc) in surface_presets {
            ui.horizontal(|ui| {
                let btn = ui.button(name);
                if btn.clicked() {
                    state.apply_surface_material_preset(flags);
                }
                btn.pro_tooltip(name, None, desc);
                ui.label(RichText::new(desc).size(10.5).color(ThemeColors::TEXT_MUTED));
            });
        }
    });

    ui.add_space(8.0);

    // Section 3: Gameplay Station Presets
    ui.group(|ui| {
        ui.label(RichText::new("⚡ Gameplay Station Presets:").strong());
        ui.label(RichText::new("Spawns complete tournament gameplay hubs at the red builder brush.").size(11.0).color(ThemeColors::TEXT_MUTED));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("💊 MedKit & Armor Station").clicked() {
                state.spawn_gameplay_station_preset("HealthAndArmor");
            }
            if ui.button("🔫 Flak Cannon + Ammo Station").clicked() {
                state.spawn_gameplay_station_preset("FlakStation");
            }
            if ui.button("⚡ UDamage Powerup Altar").clicked() {
                state.spawn_gameplay_station_preset("UDamageAltar");
            }
        });
    });
}
