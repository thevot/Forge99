// crates/ut99-editor/src/gui/help_dialog.rs
//
// Comprehensive Documentation & Help Center for Forge99 Map Editor
// Provides:
// 1. Quick Start Guide (The Unreal Subtractive Philosophy, 5-step tutorial & pre-flight checklist)
// 2. Interactive Keyboard & Mouse Shortcuts Cheatsheet with search
// 3. CSG & Brush Geometry Reference (Solid, Semi-Solid, Non-Solid, Builders, Vertex Edit, CSG Order)
// 4. Lighting & Radiosity Studio Guide (LightEffects, LightTypes, Raytraced Radiosity, Luxmeter, SpecialLit)
// 5. Movers, Doors & Lifts Manual (Keyframing, InitialState, Sound FX, Encroach Types, Lift Navigation)
// 6. Zones, Water & Atmosphere Guide (ZonePortals, ZoneInfo, Water Physics, Low-G, Distance Fog, SkyBoxes)
// 7. Surfaces, Materials & PolyFlags (Surface Properties F5, Unlit, Translucent, Masked, FakeBackdrop, UV Align)
// 8. Event & Tag Scripting Guide (Triggers, Dispatcher Chains, Counters, SpecialEvents, Teleporters)
// 9. Bot AI & Navigation Pathing Manual (PathNodes, ReachSpecs, LiftCenter/LiftExit, AlternatePath, LevelDoctor)
// 10. Tournament Game Modes Authoring (DM, CTF, DOM, AS design patterns and item balancing)
// 11. Forge99 Studios & Procedural Generators (Creation Hub, Architectural Prefabs, Bio Traps, LevelDoctor)
// 12. Historical UnrealEd 2.0 Gotchas & FAQ (BSP holes, HOM, Zone leaks, Mover pivot drift, MyLevel)
// 13. Engine Specifications & System About (64-bit Rust architecture, WGPU 24, Live Telemetry)

use egui::{Color32, Frame, Margin, RichText, Stroke, Ui};
use crate::editor_state::{EditorState, HelpTab};
use crate::gui::shortcut_help_dialog::{ShortcutEntry, SHORTCUTS};
use crate::gui::theme::{ProTooltipExt, ThemeColors};
use crate::i18n::tr;

pub fn render_help_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_help_dialog && !state.show_shortcut_help_dialog {
        return;
    }

    // If opened via shortcut_help_dialog trigger, ensure Shortcuts tab is selected
    if state.show_shortcut_help_dialog && !state.show_help_dialog {
        state.help_active_tab = HelpTab::Shortcuts;
        state.show_help_dialog = true;
    }

    let mut open = true;
    let screen_rect = ctx.input(|i| i.screen_rect());
    let window_width = 980.0_f32.min(screen_rect.width() - 40.0).max(540.0);
    let window_height = 740.0_f32.min(screen_rect.height() - 50.0).max(440.0);

    let window_title = format!("📖 {} (? / Shift+F1)", tr(state.language, "help.title"));
    egui::Window::new(window_title)
        .open(&mut open)
        .default_size([window_width, window_height])
        .collapsible(false)
        .resizable(true)
        .show(ctx, |ui| {
            render_help_content(ui, state);
        });

    if !open {
        state.show_help_dialog = false;
        state.show_shortcut_help_dialog = false;
        state.help_search_query.clear();
        state.shortcut_help_search.clear();
    }
}

fn render_help_content(ui: &mut Ui, state: &mut EditorState) {
    // -----------------------------------------------------------------
    // 1. TOP HEADER & SEARCH BAR
    // -----------------------------------------------------------------
    ui.horizontal(|ui| {
        ui.label(RichText::new("🔍").size(16.0));
        let _search_box = ui.add(
            egui::TextEdit::singleline(&mut state.help_search_query)
                .hint_text(tr(state.language, "help.search_hint"))
                .desired_width(ui.available_width() - 100.0),
        );
        if !state.help_search_query.is_empty() {
            if ui.button(format!("✖ {}", tr(state.language, "help.clear_search"))).on_hover_text("Clear search filter").clicked() {
                state.help_search_query.clear();
            }
        } else {
            ui.label(RichText::new(tr(state.language, "help.esc_close")).size(10.5).color(ThemeColors::TEXT_MUTED));
        }
    });

    ui.add_space(4.0);

    // -----------------------------------------------------------------
    // 2. TAB SELECTOR BAR
    // -----------------------------------------------------------------
    let tabs = [
        (HelpTab::QuickStart, "🚀 Quick Start", "First playable map in 5 minutes & Unreal subtractive philosophy"),
        (HelpTab::Shortcuts, "⌨ Shortcuts", "Comprehensive keyboard, mouse and viewport navigation controls"),
        (HelpTab::CsgGeometry, "🧱 CSG Geometry", "Solid, Semi-Solid, Non-Solid brushes, primitives & 2D shape editor"),
        (HelpTab::Lighting, "💡 Lighting", "Dynamic lights, LightEffects, LightTypes & radiosity baking"),
        (HelpTab::MoversAndTriggers, "🚪 Movers & Lifts", "Doors, elevators, keyframes, InitialState, audio FX and crush rules"),
        (HelpTab::ZonesAndAtmosphere, "🌊 Zones & Atmosphere", "ZonePortals, ZoneInfo, water swimming, low gravity, fog and skyboxes"),
        (HelpTab::SurfacesAndMaterials, "🎨 Surfaces & Textures", "Surface Properties (F5), Unlit, Translucent, Masked, FakeBackdrop & UV align"),
        (HelpTab::EventSystem, "⚡ Event & Tag Logic", "Tag/Event triggers, Dispatcher sequences, SpecialEvents, Counters & Teleporters"),
        (HelpTab::BotNavigation, "🤖 Bot AI & Paths", "PathNodes, ReachSpecs, lifts, jumps & LevelDoctor diagnostics"),
        (HelpTab::GameModes, "🎮 Game Modes", "Design guides for DeathMatch, CTF, Domination, and Assault"),
        (HelpTab::StudiosAndTools, "✨ Studios & Tools", "Creation Hub, Architectural Prefabs, Bio Traps, Armory & LevelDoctor"),
        (HelpTab::UnrealEdGotchas, "⚠️ Gotchas & FAQ", "BSP hole elimination, HOM prevention, limits & package hygiene"),
        (HelpTab::About, "ℹ About & Specs", "64-bit Rust architecture, WGPU 24 rendering engine & live telemetry"),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (tab, label, tooltip) in tabs {
            let is_selected = state.help_active_tab == tab;
            let text_color = if is_selected {
                ThemeColors::ACCENT_AZURE
            } else {
                ThemeColors::TEXT_PRIMARY
            };
            let clicked = ui.selectable_label(
                is_selected,
                RichText::new(label).strong().color(text_color),
            )
            .pro_tooltip(label, None, tooltip)
            .clicked();
            if clicked {
                state.help_active_tab = tab;
            }
        }
    });

    ui.separator();

    // -----------------------------------------------------------------
    // 3. TAB CONTENT ROUTING (WITH SCROLLAREA)
    // -----------------------------------------------------------------
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let query = state.help_search_query.trim().to_lowercase();
            if !query.is_empty() {
                render_search_results(ui, state, &query);
            } else {
                match state.help_active_tab {
                    HelpTab::QuickStart => render_tab_quick_start(ui, state),
                    HelpTab::Shortcuts => render_tab_shortcuts(ui, state),
                    HelpTab::CsgGeometry => render_tab_csg_geometry(ui, state),
                    HelpTab::Lighting => render_tab_lighting(ui, state),
                    HelpTab::MoversAndTriggers => render_tab_movers_and_triggers(ui, state),
                    HelpTab::ZonesAndAtmosphere => render_tab_zones_and_atmosphere(ui, state),
                    HelpTab::SurfacesAndMaterials => render_tab_surfaces_and_materials(ui, state),
                    HelpTab::EventSystem => render_tab_event_system(ui, state),
                    HelpTab::BotNavigation => render_tab_bot_navigation(ui, state),
                    HelpTab::GameModes => render_tab_game_modes(ui, state),
                    HelpTab::StudiosAndTools => render_tab_studios_and_tools(ui, state),
                    HelpTab::UnrealEdGotchas => render_tab_unrealed_gotchas(ui, state),
                    HelpTab::About => render_tab_about(ui, state),
                }
            }
        });
}

// -----------------------------------------------------------------------------
// TAB 1: QUICK START GUIDE
// -----------------------------------------------------------------------------
fn render_tab_quick_start(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🚀 Quick Start: Creating Your First UT99 Map");
    ui.add_space(4.0);

    // Callout: The Subtractive Universe
    render_info_box(
        ui,
        "🌌 The Unreal Subtractive Architecture",
        "Unlike modern game engines (Unity, Unreal Engine 5, Godot) where you start in an empty additive void, Unreal Engine 1 is fundamentally SUBTRACTIVE. The universe begins as an infinite block of solid stone. To create rooms, corridors, and arenas, you must CARVE out hollow volumes using CSG Subtract (Ctrl+S). Only once void space is created can you add solid objects (crates, pillars, bridges) inside it.",
        ThemeColors::ACCENT_AZURE,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("Build a Fully Playable Room in 5 Steps:").strong().size(13.0));
    ui.add_space(4.0);

    // Step 1
    render_step_card(
        ui,
        1,
        "Carve the Playable Room Volume (CSG Subtract)",
        "Select the Red Builder Brush in the Left Toolbar, click 'Cube', set dimensions to 1024 x 1024 x 512, and press Ctrl+S (CSG Subtract). You now have a clean, hollow, walkable room carved out of the solid universe.",
    );

    // Step 2
    render_step_card(
        ui,
        2,
        "Apply Textures & Materials (Texture Browser F4 & Surface F5)",
        "Press F4 to open the Texture Browser. Open an official UT99 package (e.g. GenEarth, Starship, MetalFX) and select a texture. In the 3D viewport, Left-Click any wall surface, press Shift+B to select all matching brush surfaces, and press F5 to align UV coordinates.",
    );

    // Step 3
    render_step_card(
        ui,
        3,
        "Add Illumination (Light Actor)",
        "Right-click anywhere on the floor or wall in the 3D viewport and choose 'Add Light Here' (or Menu -> Add -> Light). Select the light and press Enter (Actor Properties) to adjust LightBrightness (default 64) and LightRadius (default 32 to 64).",
    );

    // Step 4
    render_step_card(
        ui,
        4,
        "Place Player Spawns & Pickups (PlayerStart & Weapons)",
        "Right-click on the floor and choose 'Add PlayerStart Here'. Ensure the PlayerStart hovers at least 16-32 UU above the floor so players do not spawn embedded into geometry. Add a weapon using Quick Place Actor -> Add Weapon (Shock Rifle / Flak Cannon).",
    );

    // Step 5
    render_step_card(
        ui,
        5,
        "Rebuild All & Launch Map (Ctrl+F8)",
        "Press Ctrl+F8 (Build All) or click Build -> Build All. The editor will compute the BSP tree, bake static lighting & shadows, and compute bot reachability paths. Save with Ctrl+S as 'DM-MyFirstMap.unr' and test in Unreal Tournament 99!",
    );

    ui.add_space(10.0);
    render_card(
        ui,
        "📋 Pre-Flight Map Quality Checklist:",
        "• Sealed Perimeter: Ensure no hollow space opens directly to the infinite black exterior (prevents HOM glitches).
• Floor Clearance: Verify all PlayerStart actors and weapon pickups hover 16-32 UU above floor geometry.
• Light Placement: Never embed Light actors inside solid brush walls; keep them 16-32 UU out in open air.
• Reachability Graph: Run Build AI Paths (Ctrl+Shift+P) and check that green path lines connect all pickups.
• Automated Audit: Run LevelDoctor (Ctrl+Shift+D) to verify 0 BSP holes and 0 orphaned navigation nodes.",
        ThemeColors::SUCCESS,
    );

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);

    // Interactive Action Launchers
    ui.label(RichText::new("⚡ Quick Jump Launchers:").strong().color(ThemeColors::ACCENT_GOLD));
    ui.horizontal_wrapped(|ui| {
        if ui.button("🧙 Open New Map Wizard (Ctrl+Shift+N)").clicked() {
            state.show_map_wizard_dialog = true;
            state.show_help_dialog = false;
        }
        if ui.button("✨ Open Creation Hub & Wizards (Ctrl+Shift+A)").clicked() {
            state.show_action_wizards_dialog = true;
            state.show_help_dialog = false;
        }
        if ui.button("🎨 Open Visual Presets Catalog").clicked() {
            state.show_visual_presets_dialog = true;
            state.show_help_dialog = false;
        }
        if ui.button("🛠 Build All Now (Ctrl+F8)").clicked() {
            state.build_all_complete();
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 2: KEYBOARD & MOUSE SHORTCUTS
// -----------------------------------------------------------------------------
fn render_tab_shortcuts(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("⌨ Keyboard Shortcuts & Viewport Navigation Controls");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("🔍").size(14.0));
        ui.add(
            egui::TextEdit::singleline(&mut state.shortcut_help_search)
                .hint_text("Filter shortcuts by key or action (e.g. WASD, MMB, Gizmo, CSG)...")
                .desired_width(ui.available_width() - 80.0),
        );
        if !state.shortcut_help_search.is_empty() && ui.button("✖").clicked() {
            state.shortcut_help_search.clear();
        }
    });

    ui.separator();

    let query = state.shortcut_help_search.trim().to_lowercase();
    let query_clean = query.replace(' ', "").replace('/', "");
    let filtered: Vec<&ShortcutEntry> = SHORTCUTS
        .iter()
        .filter(|entry| {
            if query.is_empty() {
                true
            } else {
                entry.title.to_lowercase().contains(&query)
                    || entry.category.to_lowercase().contains(&query)
                    || entry.description.to_lowercase().contains(&query)
                    || entry.keys.iter().any(|k| {
                        let kl = k.to_lowercase();
                        kl.contains(&query) || kl.replace(' ', "").replace('/', "").contains(&query_clean)
                    })
            }
        })
        .collect();

    if filtered.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No matching shortcuts found.").color(ThemeColors::TEXT_MUTED));
            if ui.button("Clear Search").clicked() {
                state.shortcut_help_search.clear();
            }
        });
        return;
    }

    let mut current_category = "";
    for entry in filtered {
        if entry.category != current_category {
            current_category = entry.category;
            ui.add_space(8.0);
            ui.label(
                RichText::new(current_category)
                    .strong()
                    .size(13.0)
                    .color(ThemeColors::ACCENT_AZURE),
            );
            ui.separator();
        }

        ui.horizontal(|ui| {
            ui.set_min_height(24.0);
            ui.label(RichText::new(entry.icon).size(13.0));
            ui.label(RichText::new(entry.title).strong().color(ThemeColors::TEXT_PRIMARY));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut first = true;
                for key in entry.keys.iter().rev() {
                    if !first {
                        ui.label(RichText::new("+").size(10.5).color(ThemeColors::TEXT_MUTED));
                    }
                    first = false;
                    render_keycap(ui, key);
                }
            });
        });

        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(RichText::new(entry.description).size(11.0).color(ThemeColors::TEXT_SECONDARY));
        });
        ui.add_space(3.0);
    }
}

// -----------------------------------------------------------------------------
// TAB 3: CSG & BRUSH GEOMETRY
// -----------------------------------------------------------------------------
fn render_tab_csg_geometry(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🧱 CSG Geometry & Brush Engineering");
    ui.add_space(4.0);

    ui.label(RichText::new("The 4 Fundamental Brush Archetypes:").strong().size(13.0));
    ui.add_space(4.0);

    // Grid of Brush Archetypes
    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            render_card(
                ui,
                "🔷 Solid Brushes (CSG Add)",
                "Standard structural geometry. Solid brushes block player movement, cast opaque shadows, and slice all surrounding polygons in the BSP tree. Use for primary walls, outer barriers, and floors.",
                ThemeColors::CSG_ADD,
            );
            ui.add_space(6.0);
            render_card(
                ui,
                "🔶 Subtractive Brushes (CSG Subtract)",
                "Carves walkable interior void out of solid rock. Every room, corridor, or arena starts with a subtractive brush. Always keep subtractive brushes aligned to the grid to prevent micro-gaps.",
                ThemeColors::CSG_SUB,
            );
        });

        cols[1].vertical(|ui| {
            render_card(
                ui,
                "🌟 Semi-Solid Brushes (PF_SEMISOLID)",
                "THE GOLDEN RULE OF UT99 LEVEL DESIGN! Semi-solids block players but DO NOT cut surrounding BSP polygons. Crucial for pillars, ceiling rafters, arches, floating catwalks, and decorative trims. Eliminates BSP cuts and HOM glitches.",
                ThemeColors::ACCENT_GOLD,
            );
            ui.add_space(6.0);
            render_card(
                ui,
                "💠 Non-Solid Brushes (Sheets & Portals)",
                "Zero collision and zero BSP cuts. Used for Zone Portals (cyan sheets dividing rooms), water/slime sheets, fire sheets, and trigger bounds. Players can pass freely through non-solids.",
                ThemeColors::ACCENT_AZURE,
            );
        });
    });

    ui.add_space(10.0);
    render_gotcha_box(
        ui,
        "Why Semi-Solids are Essential:",
        "If you add 10 solid pillars inside a room, each pillar slices the floor and ceiling into dozens of complex fragments, causing BSP leaf explosion and polygon lag. Converting those 10 pillars to Semi-Solids (Brush -> Add Special -> Semi-Solid) retains collision while leaving the floor as a single clean polygon!",
    );

    ui.add_space(8.0);
    render_card(
        ui,
        "⏳ The Law of CSG Chronological Order:",
        "CSG operations execute strictly in chronological sequence (Brush 0, Brush 1, Brush 2...).
• If a Subtractive brush is placed AFTER an Additive brush inside the same volume, it carves away the additive brush.
• To change operation order: Right-click brush -> Order -> Send to First (Ctrl+F) or Send to Last (Ctrl+L).
• Semi-Solids are evaluated after structural solids and clip against preceding geometry without fragmenting outer walls.",
        ThemeColors::ACCENT_AZURE,
    );

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    ui.label(RichText::new("CSG Boolean Operations:").strong().size(13.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("• CSG Add (Ctrl+A):").strong().color(ThemeColors::CSG_ADD));
        ui.label("Adds the Red Builder Brush geometry to the world as solid matter.");
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("• CSG Subtract (Ctrl+S):").strong().color(ThemeColors::CSG_SUB));
        ui.label("Carves the Red Builder Brush volume out of existing solid geometry.");
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("• CSG Intersect:").strong().color(ThemeColors::ACCENT_AZURE));
        ui.label("Trims the Builder Brush to only keep the volume overlapping existing solid geometry.");
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("• CSG Deintersect:").strong().color(ThemeColors::WARNING));
        ui.label("Hollows out the Builder Brush by deleting any volume overlapping solid geometry.");
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("📐 Open 2D Shape Editor").clicked() {
            state.show_shape_editor = true;
            state.show_help_dialog = false;
        }
        if ui.button("✨ Open Add Special Brush Dialog").clicked() {
            state.show_add_special_dialog = true;
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 4: LIGHTING & RADIOSITY
// -----------------------------------------------------------------------------
fn render_tab_lighting(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("💡 Lighting, Radiosity & Atmosphere");
    ui.add_space(4.0);

    render_info_box(
        ui,
        "Static Baked Lightmaps + Raytraced Radiosity",
        "Unreal Tournament 99 bakes static illumination directly onto surface lightmaps in the .unr level file. Forge99 enhances this with a multi-bounce radiosity engine that computes indirect color bleeding and soft penumbra shadows.",
        ThemeColors::ACCENT_GOLD,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("Core Light Actor Properties (Actor Properties -> Lighting):").strong().size(13.0));
    ui.add_space(4.0);

    let props = [
        ("LightBrightness (0..255)", "Luminous intensity of the light source. Standard interior lights use 40 to 90."),
        ("LightRadius (0..255)", "Radial attenuation boundary in Unreal Units. Radius 32 ≈ 256 UU, Radius 64 ≈ 512 UU."),
        ("LightHue (0..255)", "Color hue spectrum (0 = Red, 42 = Yellow, 85 = Green, 128 = Cyan, 170 = Blue, 213 = Magenta)."),
        ("LightSaturation (0..255)", "Saturation amount: 0 = fully saturated vivid color, 255 = pure desaturated white."),
        ("bSpecialLit (Bool)", "When true, this light only affects surfaces marked with PolyFlags: SpecialLit. Ideal for stage spotlights."),
    ];
    for (prop, desc) in props {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {prop}:")).strong().color(ThemeColors::TEXT_PRIMARY));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Canonical LightEffect & LightType Styles:").strong().size(13.0));
    ui.add_space(4.0);

    let effects = [
        ("LE_None", "Standard spherical isotropic light falloff."),
        ("LE_TorchWaver", "Gentle organic flickering simulating burning medieval wall torches."),
        ("LE_FireWaver", "High-frequency energetic flame wavering for bonfires and furnaces."),
        ("LE_WateryShimmer", "Underwater caustics refraction shimmer effect."),
        ("LE_Spotlight", "Directional cone projection oriented by the Light actor's Pitch and Yaw rotation."),
        ("LE_Searchlight", "Rotating beacon beam sweeping across the environment."),
        ("LT_Pulse / LT_Blink", "Dynamic periodic pulsing or blinking animation for emergency lights and indicators."),
        ("LT_Flicker / LT_Strobe", "Strobe frequency flashing for damaged electrical fixtures and rave lighting."),
    ];
    for (eff, desc) in effects {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {eff}:")).monospace().color(ThemeColors::ACCENT_AZURE));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(10.0);
    render_gotcha_box(
        ui,
        "Critical Light Placement Gotcha:",
        "Never place Light actors inside solid geometry or exactly coplanar flush on a wall surface! Doing so causes raycast failures that produce pitch-black lightmap bleeding and ugly artifacts. Always position lights at least 16 to 32 Unreal Units out from the wall or ceiling.",
    );

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("💡 Build Lighting (Ctrl+Shift+L)").clicked() {
            state.build_lighting();
            state.show_help_dialog = false;
        }
        if ui.button("🌡 Toggle Luxmeter Heatmap (Shift+F6)").clicked() {
            state.render_lux_heatmap = !state.render_lux_heatmap;
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 5: MOVERS, DOORS & LIFTS
// -----------------------------------------------------------------------------
fn render_tab_movers_and_triggers(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🚪 Movers, Doors, Lifts & Dynamic Brushes");
    ui.add_space(4.0);

    render_info_box(
        ui,
        "From Static Geometry to Dynamic Interactive Worlds",
        "Unlike static CSG brushes carved permanently into the BSP tree, a Mover is a dynamic polyhedral brush compiled into its own independent UModel sub-tree attached to a Mover actor. Movers smoothly interpolate between spatial keyframes to create sliding blast doors, multi-floor elevators, spinning turbine fans, crushers, and triggered bridge traps.",
        ThemeColors::ACCENT_AZURE,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("The 3 Golden Rules of Mover Authoring:").strong().size(13.0));
    ui.add_space(4.0);

    let mover_rules = [
        ("1. Shape with Builder Brush", "Model the door, platform or trap using the Red Builder Brush in its resting position (Key 0)."),
        ("2. Add Mover (Yellow Wireframe)", "Click the yellow 'Add Mover' button (or Brush -> Add Mover). A yellow wireframe brush appears."),
        ("3. Set Keyframes with Right-Click", "Right-click Mover -> Movers -> Key 1. Translate or rotate the mover to its open/raised position. Then return to Key 0!"),
    ];
    for (step, desc) in mover_rules {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {step}:")).strong().color(ThemeColors::TEXT_PRIMARY));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Core InitialState Dispatcher Modes (Object -> InitialState):").strong().size(13.0));
    ui.add_space(4.0);

    let states = [
        ("BumpOpenTimed", "Opens when physically bumped by player or bot; waits StayOpenTime, then smoothly closes. Ideal for automatic doors."),
        ("StandOpenTimed", "Activates when a player steps on top of the mover. Essential for elevators, lifts, and vertical jump platforms."),
        ("TriggerOpenTimed", "Remains closed until triggered by an external actor (switch, terminal, or proximity Trigger) with matching Event/Tag."),
        ("TriggerToggle", "Toggles state on each trigger pulse: first trigger opens and holds; second trigger closes and holds."),
        ("ConstantLoop", "Continuously cycles through all keyframes without stopping. Perfect for rotating turbine blades, gears, and hazard pendulums."),
    ];
    for (st, desc) in states {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {st}:")).monospace().color(ThemeColors::ACCENT_GOLD));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    render_card(
        ui,
        "🤖 Bot Navigation for Elevators (LiftCenter & LiftExit):",
        "Bots cannot navigate moving platforms without explicit guidance nodes!
1. Place a LiftExit actor on the floor outside the elevator door on each floor.
2. Place a LiftCenter actor in the center of the elevator platform.
3. In LiftCenter properties, set LiftTag matching the Mover actor's Tag.
Bots will wait at LiftExit for the elevator to arrive, step onto LiftCenter, ride, and exit smoothly!",
        ThemeColors::SUCCESS,
    );

    ui.add_space(8.0);
    render_gotcha_box(
        ui,
        "Mover Pivot Origin Gotcha:",
        "Always position the Builder Brush pivot at the intended rotation axis BEFORE adding the mover! If you scale or translate brush vertices after adding the mover, the pivot will drift, causing doors to swing around an off-center axis.",
    );

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("🧙 Open Action Wizards (Elevator / Door)").clicked() {
            state.show_action_wizards_dialog = true;
            state.show_help_dialog = false;
        }
        if ui.button("🎯 Open Actor Properties (Enter)").clicked() {
            state.show_actor_properties = true;
            state.show_help_dialog = false;
        }
        if ui.button("✨ Open Add Special Brush Dialog").clicked() {
            state.show_add_special_dialog = true;
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 6: ZONES, WATER & ATMOSPHERE
// -----------------------------------------------------------------------------
fn render_tab_zones_and_atmosphere(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🌊 Zones, Water, Gravity & Environmental Atmosphere");
    ui.add_space(4.0);

    render_info_box(
        ui,
        "BSP Portals & Isolated Thermodynamic Sectors",
        "In Unreal Tournament 99, a Zone is an isolated partition of space separated by ZonePortal sheets. Zones deliver two massive advantages:
1. Occlusion Culling: The engine skips rendering rooms that cannot be seen through open portals, keeping FPS high.
2. Environmental Isolation: Each zone can have custom gravity, water swimming physics, wind currents, distance fog, and reverb.",
        ThemeColors::ACCENT_AZURE,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("ZoneInfo Configuration Matrix (Actor Properties -> ZoneInfo):").strong().size(13.0));
    ui.add_space(4.0);

    let zone_props = [
        ("ZoneVelocity (Vector)", "Applies continuous linear force (UU/sec). (0, 400, 0) creates rushing rivers, wind tunnels, or conveyor belts."),
        ("ZoneGravity (Vector)", "Gravity acceleration. Default UT99 is (0, 0, -950). Low-G space arenas use Z = -200 (like DM-Phobos or DM-Morpheus)."),
        ("bWaterZone (Bool)", "Instantly converts zone into swimmable water! Activates buoyancy, splash particles, muffled audio, and lung breath timers."),
        ("DamagePerSec (Int)", "Continuous environmental hazard damage (e.g. 40 for boiling lava, 15 for toxic bio-slime, 5 for vacuum asphyxiation)."),
        ("DamageType (Name)", "Attribution string: 'Burned', 'Corroded', 'Drowned', 'Fell'."),
        ("bDistanceFog (Bool)", "Activates volumetric atmospheric distance fog (DistanceFogColor, DistanceFogStart, DistanceFogEnd)."),
    ];
    for (prop, desc) in zone_props {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {prop}:")).strong().color(ThemeColors::TEXT_PRIMARY));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    render_card(
        ui,
        "🌌 The Classic SkyBox Architecture:",
        "UT99 skies are rendered via virtual projection from a SkyZoneInfo actor:
1. Carve a small 512x512x512 subtractive cube outside the playable map (e.g. at (0, 0, 30000)).
2. Texture its interior walls with nebula, mountains, or cloud textures.
3. Place a SkyZoneInfo actor at the exact center of this cube.
4. In your playable arena, select outdoor ceilings, press F5 (Surface Properties), and check 'FakeBackdrop'.
The engine automatically projects the skybox camera through those polygons with parallax motion!",
        ThemeColors::ACCENT_GOLD,
    );

    ui.add_space(8.0);
    render_gotcha_box(
        ui,
        "The Zero-Leakage ZonePortal Rule:",
        "ZonePortals MUST intersect solid geometry all the way around their perimeter! If a portal sheet has even a 1-UU gap with a doorframe, the zone 'leaks' into neighboring rooms, causing the entire map to collapse into Zone 0. Use Viewport Shading -> Zone Portal View to visually verify seals.",
    );

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("👁 Switch to Zone Portal View").clicked() {
            state.viewport_shading = crate::camera::ViewportShading::ZonePortal;
            state.show_help_dialog = false;
        }
        if ui.button("🎨 Open Visual Presets & Fog Catalog").clicked() {
            state.show_visual_presets_dialog = true;
            state.show_help_dialog = false;
        }
        if ui.button("✨ Open Add Special (Zone Portal)").clicked() {
            state.show_add_special_dialog = true;
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 7: SURFACES, MATERIALS & POLYFLAGS
// -----------------------------------------------------------------------------
fn render_tab_surfaces_and_materials(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🎨 Surfaces, Materials, PolyFlags & UV Mapping");
    ui.add_space(4.0);

    render_info_box(
        ui,
        "Surface Properties Inspector (F5) & PolyFlags",
        "Every geometry surface in Unreal Tournament 99 carries specialized rendering flags (PolyFlags) and planar UV texture coordinates. Mastering surface properties lets you create glowing neon monitors, translucent energy shields, perforated metal grates, and reflective floor mirrors.",
        ThemeColors::ACCENT_AZURE,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("3D Surface Selection Hotkeys:").strong().size(13.0));
    ui.add_space(4.0);

    let sel_keys = [
        ("Left-Click", "Select a single polygon surface in the 3D viewport."),
        ("Ctrl + Left-Click", "Add or remove surface to/from current multi-selection."),
        ("Shift + B", "Select all surfaces belonging to the currently selected brush."),
        ("Shift + T", "Select all surfaces across the ENTIRE level sharing the active texture."),
        ("Shift + C", "Select all coplanar adjacent surfaces (seamless floor or ceiling planes)."),
        ("Shift + W", "Select all contiguous wall surfaces around room perimeters."),
        ("Shift + S", "Select all surfaces in the entire level."),
    ];
    for (k, desc) in sel_keys {
        ui.horizontal(|ui| {
            render_keycap(ui, k);
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    ui.label(RichText::new("PolyFlags Matrix Decoded:").strong().size(13.0));
    ui.add_space(4.0);

    let flags = [
        ("Unlit", "Ignores baked lightmaps; renders at 100% full bright. Essential for monitors, neon signs, lava, and glowing cores."),
        ("Translucent", "Additive blending (Color = Surface + Background; black is invisible). Perfect for forcefields, glass, lasers."),
        ("Masked", "1-bit alpha transparency based on palette index 0. Used for chainlink fences, railings, ladders, and scaffolding."),
        ("Modulated", "Multiplicative blending (Color = Background * Surface * 2). Used for shadow decals, scorch marks, and stained glass."),
        ("TwoSided", "Disables backface culling; renders both sides of the polygon. Vital for hanging banners, sheets, and thin grates."),
        ("FakeBackdrop", "Renders the skybox camera through this polygon, simulating infinite outdoor vistas and ceilings."),
        ("SpecialLit", "Surface only receives light from Light actors marked with bSpecialLit = true."),
        ("Mirrored", "Renders real-time planar mirror reflections of players and combat projectiles."),
    ];
    for (fl, desc) in flags {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {fl}:")).strong().color(ThemeColors::ACCENT_GOLD));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    render_card(
        ui,
        "🖌 UV Alignment & The Eyedropper Tool:",
        "• Align Wall Direction: Wraps textures seamlessly around contiguous room corners without horizontal stretching.
• Align Wall Pan: Smooths out vertical and horizontal offsets across adjacent coplanar surfaces.
• Eyedropper Workflow: Press Alt+RMB on a source surface to sample its texture and UV transform, then Alt+LMB on target surfaces to stamp it!",
        ThemeColors::SUCCESS,
    );

    ui.add_space(8.0);
    render_gotcha_box(
        ui,
        "The MyLevel Package Miracle:",
        "Never save custom textures or audio into official game packages (Botpack.u, GenEarth.utx)! Always save custom assets into package 'MyLevel'. Textures in MyLevel are embedded directly inside the .unr level file, ensuring players can download and play without 'Package Missing' crashes.",
    );

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("🖼 Open Texture Browser (F4)").clicked() {
            state.show_texture_browser = true;
            state.show_help_dialog = false;
        }
        if ui.button("🎨 Open Surface Properties (F5)").clicked() {
            state.show_surface_properties = true;
            state.show_help_dialog = false;
        }
        if ui.button("🔍 Find and Replace Textures (Ctrl+R)").clicked() {
            state.show_replace_textures_dialog = true;
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 8: EVENT & TAG SCRIPTING
// -----------------------------------------------------------------------------
fn render_tab_event_system(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("⚡ Event & Tag Logic, Triggers & Scripting Mechanics");
    ui.add_space(4.0);

    render_info_box(
        ui,
        "Visual Scripting Without Writing Code",
        "Unreal Tournament 99 features an elegant string-based messaging bus. Actors communicate by matching Event and Tag strings. When an actor broadcasts its Event (e.g. Event = 'DoorAlpha'), the engine searches all actors in the level whose Tag matches (Tag = 'DoorAlpha') and triggers their activation routine!",
        ThemeColors::ACCENT_AZURE,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("Essential Logic Actors (Actor Browser -> Triggers):").strong().size(13.0));
    ui.add_space(4.0);

    let logic_actors = [
        (
            "Trigger",
            "Proximity & Shootable Switches",
            "• TriggerType: TT_PlayerProximity (player enters radius), TT_PawnProximity (players/bots), TT_Shoot (weapon damage).
• Message: HUD text displayed to player (e.g. 'Blast Doors Opening').
• bTriggerOnceOnly: Fires once and deactivates permanently.
• ReTriggerDelay: Cooldown seconds before trigger can fire again.",
            ThemeColors::ACCENT_GOLD,
        ),
        (
            "Dispatcher",
            "Multi-Phase Timed Sequences",
            "• OutEvents[0..7]: Array of target Tags to trigger sequentially.
• OutDelays[0..7]: Floating-point delays in seconds between steps.
• Example: Delay 0.0s: Sound siren -> Delay 1.5s: Dim red lights -> Delay 3.0s: Open blast door -> Delay 4.5s: Spawn ambush!",
            ThemeColors::CSG_ADD,
        ),
        (
            "SpecialEvent",
            "Sensory & Damage Execution",
            "• Damage: Inflicts custom damage onto activating pawn.
• Kill: Instantly kills activator with tournament killfeed message.
• PlaySound: Plays localized or map-wide broadcast audio clips.
• DisplayMessage: Broadcasts on-screen announcements.",
            ThemeColors::WARNING,
        ),
        (
            "Counter",
            "Objective Countdown Switch",
            "• NumToCount: Number of trigger pulses required before firing output Event.
• CountMessage: Message displayed on each pulse ('%i more power nodes remain!').
• CompleteMessage: Broadcast when final count is reached ('Core Overloaded!').",
            ThemeColors::SUCCESS,
        ),
        (
            "Teleporter",
            "Spatial Quantum Portals",
            "• URL: Set to the Tag of the destination Teleporter.
• Bot navigation automatically calculates valid reachability through linked teleporters!",
            ThemeColors::ACCENT_AZURE,
        ),
    ];

    for (name, sub, body, col) in logic_actors {
        Frame::NONE
            .fill(Color32::from_rgb(20, 24, 30))
            .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
            .corner_radius(4.0)
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).strong().size(13.0).color(col));
                    ui.label(RichText::new(format!("— {sub}")).size(11.5).color(ThemeColors::TEXT_MUTED));
                });
                ui.add_space(2.0);
                ui.label(RichText::new(body).size(11.0).color(ThemeColors::TEXT_PRIMARY));
            });
        ui.add_space(6.0);
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("🎯 Open Actor Properties (Enter)").clicked() {
            state.show_actor_properties = true;
            state.show_help_dialog = false;
        }
        if ui.button("🔍 Find Actors by Tag/Event (Ctrl+F)").clicked() {
            state.show_find_actors_dialog = true;
            state.show_help_dialog = false;
        }
        if ui.button("🧙 Open Action Wizards (Ctrl+Shift+A)").clicked() {
            state.show_action_wizards_dialog = true;
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 9: BOT AI & NAVIGATION PATHING
// -----------------------------------------------------------------------------
fn render_tab_bot_navigation(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🤖 Bot AI Navigation & ReachSpecs Network");
    ui.add_space(4.0);

    render_info_box(
        ui,
        "The Precomputed Reachability Network",
        "Unreal Tournament 99 bots do not use runtime navigation mesh. Instead, they traverse a precomputed graph of ReachSpecs connecting navigation actors. When you click 'Build Paths' (Ctrl+Shift+P), the compiler tests collision cylinders and lines-of-sight between nodes to determine walk, jump, and swim connections.",
        ThemeColors::SUCCESS,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("The 4 Golden Rules of Bot Pathing:").strong().size(13.0));
    ui.add_space(4.0);

    let rules = [
        ("Rule 1: Spacing", "Keep PathNodes spaced 400 to 800 Unreal Units apart. Never exceed 1000 UU between nodes."),
        ("Rule 2: Line of Sight", "Adjacent PathNodes must have unobstructed line of sight. Geometry blocking the connection breaks the link."),
        ("Rule 3: Corner Apexes", "Always place a PathNode directly at the apex of every corner or hallway intersection so bots don't bump into walls."),
        ("Rule 4: Automatic Nodes", "PlayerStarts, InventorySpots (weapons, ammo, armor), and Flags automatically act as navigation nodes!"),
    ];
    for (rule, desc) in rules {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {rule}:")).strong().color(ThemeColors::TEXT_PRIMARY));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Specialized Navigation Actors:").strong().size(13.0));
    ui.add_space(4.0);

    let special = [
        ("LiftCenter & LiftExit", "Essential for elevators and movers! Place LiftExit on each floor, and attach LiftCenter to the moving platform."),
        ("JumpSpot", "Marks jump pad landing points or high ledges accessible via Jump Boots or Impact Hammer jumps."),
        ("TranslocatorTarget", "Marks elevated sniper perches or secret alcoves that bots should throw their Translocator beacon toward."),
        ("AlternatePath", "Capture The Flag routing! Place in flanking lanes with SelectionWeight (1.0 to 2.5) to distribute bot attack routes."),
        ("DefensePoint", "CTF & Assault defensive positioning! Set Priority (1 = sniping, 2 = corridor guard, 3 = flag base sentinel)."),
    ];
    for (node, desc) in special {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {node}:")).monospace().color(ThemeColors::ACCENT_AZURE));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    render_gotcha_box(
        ui,
        "Orphaned Node Prevention:",
        "A PathNode with no connections is an 'Orphan'. Bots will refuse to walk toward it and may get stuck in loops. Always run LevelDoctor (Ctrl+Shift+D) to automatically detect orphaned nodes and unlinked sectors.",
    );

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("🤖 Build AI Paths (Ctrl+Shift+P)").clicked() {
            state.build_ai_paths();
            state.show_help_dialog = false;
        }
        if ui.button("🏥 Run LevelDoctor Audit (Ctrl+Shift+D)").clicked() {
            state.run_level_doctor_audit();
            state.show_help_dialog = false;
        }
        if ui.button("👁 Toggle Path Network Lines").clicked() {
            state.show_ai_paths = !state.show_ai_paths;
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 10: GAME MODES AUTHORING
// -----------------------------------------------------------------------------
fn render_tab_game_modes(ui: &mut Ui, _state: &mut EditorState) {
    ui.heading("🎮 Tournament Game Modes Authoring Guide");
    ui.add_space(4.0);

    let modes = [
        (
            "🏆 DeathMatch (DM-)",
            "Arena & Duel Combat",
            "• Provide 8 to 16 PlayerStart actors distributed across rooms (never in direct line of sight of each other).
• Design triangular or circular weapon loops (e.g. Flak Cannon, Shock Rifle, Rocket Launcher at opposing vertices).
• Place high-value pickups (Damage Amplifier, ShieldBelt) in high-exposure, contested center zones.",
            ThemeColors::ACCENT_GOLD,
        ),
        (
            "🚩 Capture The Flag (CTF-)",
            "Polar Bases & Multi-Lane Assault",
            "• Red Base: FlagBase with Team = 0 | Blue Base: FlagBase with Team = 1. Maintain Euclidean distance ≥ 2400 UU.
• Place AlternatePath actors with SelectionWeight (1.0 = normal, 2.0 = heavy) along left/right flanking routes.
• Add DefensePoint actors: Priority 3 (guard flag), Priority 2 (hall sentinels), Priority 1 (bSniping snipers).
• Place symmetric armor and a contested central super-item at X = 0.",
            ThemeColors::CSG_ADD,
        ),
        (
            "⚡ Domination (DOM-)",
            "Triad Circuit Control",
            "• Place 3 ControlPoint actors (Points A, B, C) in a balanced equilateral or isosceles triangle layout.
• Ensure minimum standoff distance between points is ≥ 480 UU to eliminate proximity spam.
• Neutral respawn areas must be placed in secondary perimeter tunnels, never directly on top of control points.",
            ThemeColors::WARNING,
        ),
        (
            "🛡 Assault (AS-)",
            "Sequential Mission Infiltration",
            "• Add AssaultInfo actor and configure ObjDesc[0..n] sequential briefing texts.
• Use FortPath actors for attacker waypoint guidance and DefensePoints for defenders.
• Use Movers with TriggerOpenTimed as blast doors sealing passages until objectives are completed.
• Place SpectatorCam actors (AS_StartCam and AS_EndCam) for cinematic introductory and victory flybys.",
            ThemeColors::SUCCESS,
        ),
    ];

    for (title, sub, body, color) in modes {
        Frame::NONE
            .fill(Color32::from_rgb(20, 24, 30))
            .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
            .corner_radius(4.0)
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).strong().size(13.0).color(color));
                    ui.label(RichText::new(format!("— {sub}")).size(11.5).color(ThemeColors::TEXT_MUTED));
                });
                ui.add_space(2.0);
                ui.label(RichText::new(body).size(11.0).color(ThemeColors::TEXT_PRIMARY));
            });
        ui.add_space(6.0);
    }
}

// -----------------------------------------------------------------------------
// TAB 11: STUDIOS & PROCEDURAL GENERATORS
// -----------------------------------------------------------------------------
fn render_tab_studios_and_tools(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("✨ Forge99 Studios, Procedural Generators & Diagnostics");
    ui.add_space(4.0);

    render_info_box(
        ui,
        "Next-Generation Level Authoring Superpowers",
        "Forge99 goes far beyond classic UnrealEd 2.0 by providing integrated Procedural Studios and real-time automated diagnostics. Complex architecture, mission chains, hazard traps, and bot balancing that once took days of manual drafting are created in seconds.",
        ThemeColors::ACCENT_AZURE,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("Integrated Studios Directory:").strong().size(13.0));
    ui.add_space(4.0);

    let studios = [
        ("Creation Hub (Ctrl+Shift+A)", "One-click generation of procedural arenas, modular corridors, staircases, elevators, and hazard pits."),
        ("Architectural Prefab Studio", "Parametric Roman colonnades, fluted pillars, gothic arches, vaulted ceilings, and bunker bulkheads."),
        ("Assault Mission Studio", "Complete AS mission authoring: objective chains, blast doors with fortpaths, defense points, and flyby cams."),
        ("Armory & Pickup Balancer", "Evaluates weapon distribution, ammo clusters, armor stacking metrics, and CTF base travel-time symmetry."),
        ("Bio Trap & Hazard Studio", "Automated acid pools, toxic sluices, crushing hydraulic pistons, and laser tripwires with pre-wired Triggers."),
        ("Alarm Beacon & Siren Studio", "Emergency oscillating beacon lights synchronized with klaxon sound effects and pulsing light styles."),
        ("Cinematic Camera Studio", "Spline-interpolated camera sequences for spectator intros (SpectatorCam) and tournament matches."),
        ("2D Shape Editor", "Precision CAD drafting: Bezier curves, concave ear-clipping triangulator, 3D extrusion, and lathe revolving."),
        ("LevelDoctor Map Audit (Ctrl+Shift+D)", "1-click automated diagnosis of BSP holes, non-planar polygons, orphaned PathNodes, and budget overruns."),
        ("Command Palette (Ctrl+P)", "Instant fuzzy search and execution for all 100+ editor commands, studios, and actions."),
    ];

    for (name, desc) in studios {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {name}:")).strong().color(ThemeColors::ACCENT_GOLD));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);

    ui.label(RichText::new("⚡ Quick Studio Launchers:").strong().color(ThemeColors::ACCENT_AZURE));
    ui.horizontal_wrapped(|ui| {
        if ui.button("🧙 Creation Hub (Ctrl+Shift+A)").clicked() {
            state.show_action_wizards_dialog = true;
            state.show_help_dialog = false;
        }
        if ui.button("📐 2D Shape Editor").clicked() {
            state.show_shape_editor = true;
            state.show_help_dialog = false;
        }
        if ui.button("🏥 LevelDoctor Audit (Ctrl+Shift+D)").clicked() {
            state.run_level_doctor_audit();
            state.show_help_dialog = false;
        }
        if ui.button("🎨 Visual Presets Catalog").clicked() {
            state.show_visual_presets_dialog = true;
            state.show_help_dialog = false;
        }
        if ui.button("⚡ Command Palette (Ctrl+P)").clicked() {
            state.show_command_palette = true;
            state.show_help_dialog = false;
        }
    });
}

// -----------------------------------------------------------------------------
// TAB 12: UNREALED 2.0 GOTCHAS & FAQ
// -----------------------------------------------------------------------------
fn render_tab_unrealed_gotchas(ui: &mut Ui, _state: &mut EditorState) {
    ui.heading("⚠️ UnrealEd 2.0 Historical Gotchas & Level Design FAQ");
    ui.add_space(4.0);

    let gotchas = [
        (
            "1. What causes BSP Holes and HOMs (Hall of Mirrors)?",
            "BSP holes occur when polygon cuts produce microscopic mathematical gaps between adjacent surfaces, causing the camera to view the unrendered void behind geometry. 

Causes & Cures:
• Off-grid vertices: Always keep Grid Snap ENABLED (Ctrl+G). Vertices moved off-grid introduce floating-point rounding errors.
• Non-planar polygons: Faces with 4+ vertices where points do not lie on the exact same mathematical plane. Cure: select brush and choose Polygons -> Separate (Triangulate).
• Complex solid intersections: Excessive overlapping solid brushes. Cure: convert decorative details to Semi-Solids!",
        ),
        (
            "2. The 65,536 Unreal Units World Coordinate Barrier",
            "Unreal Tournament 99 engine physics and netcode use signed 16-bit integer space for actor coordinate replication. 
Geometry placed farther than ±65,000 UU from the origin (0, 0, 0) will experience player jitter, collision failure, and renderer clipping. 
Always keep your map centered near the world origin.",
        ),
        (
            "3. Texture Package Management & The 'MyLevel' Miracle",
            "If you import custom textures or sounds, how do you prevent 'Package Missing' errors when sharing your map? 

Solution: Use the internal package name 'MyLevel'! Textures saved into package 'MyLevel' are embedded directly into the .unr map file itself. Players do not need to download separate .utx files.",
        ),
        (
            "4. Texture Alignment & Texel Density",
            "When aligning brick or panel textures across walls, never manually scale UVs arbitrarily. 
Instead, select the surfaces, press F5 (Surface Properties), and use 'Align Wall Direction' or 'Align Wall Pan'. 
This ensures seamless texture wrapping across corners without stretching.",
        ),
        (
            "5. Zone Leakage & The Single-Zone Collapse",
            "If a ZonePortal sheet fails to intersect the boundary wall by even 1 Unreal Unit, the zone boundary leaks! When a leak occurs, all rooms bleed together into Zone 0, breaking room-specific gravity, distance fog, and occlusion culling. Cure: Switch to Zone Portal View (Shift+F5) to verify clean color separation across all doorways.",
        ),
        (
            "6. Mover Pivot Drift & Off-Center Rotations",
            "When creating rotating doors or swinging blades, always place the Red Builder Brush pivot at the intended hinge axis BEFORE adding the mover. If you transform or move vertices after mover creation, the rotation axis drifts away from the hinge.",
        ),
    ];

    for (q, a) in gotchas {
        render_gotcha_box(ui, q, a);
        ui.add_space(6.0);
    }
}

// -----------------------------------------------------------------------------
// TAB 13: ABOUT & SYSTEM SPECS
// -----------------------------------------------------------------------------
fn render_tab_about(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("ℹ About Forge99 Level Editor (Native 64-bit Rust)");
    ui.add_space(4.0);

    render_info_box(
        ui,
        "Modern High-Performance Engineering",
        "Forge99 is a ground-up 64-bit level authoring suite written in Rust for Unreal Tournament 99 and Unreal Engine 1 (UT99 v436, v451, v469, Unreal v227). It replaces the unstable 16/32-bit Visual Basic / C++ editor from 1999 with a crash-safe, parallel multithreaded architecture that renders at 60+ FPS via WGPU 24.",
        ThemeColors::ACCENT_AZURE,
    );

    ui.add_space(8.0);
    ui.label(RichText::new("Crate Architecture:").strong().size(13.0));
    ui.add_space(4.0);

    let crates = [
        ("crates/ut99-format", "Zero-copy binary parser and serializer for .unr, .utx, .uax, .umx, and .u packages with crash-safe atomic saving."),
        ("crates/ut99-bsp", "Polyhedral CSG boolean operations, Rayon parallel BSP tree compiler, radiosity lighting solver, and reachability graph."),
        ("crates/ut99-editor", "Hardware-accelerated WGPU 24 viewport, High-DPI procedural vector icons, and transactional TransformSession."),
    ];
    for (cr, desc) in crates {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("• {cr}:")).monospace().color(ThemeColors::ACCENT_GOLD));
            ui.label(RichText::new(desc).color(ThemeColors::TEXT_SECONDARY));
        });
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Live Telemetry & Diagnostics:").strong().size(13.0));
    ui.add_space(4.0);

    let map_name = state.map_path.as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled / In-Memory".to_string());

    ui.horizontal(|ui| {
        ui.label("Loaded Map:");
        ui.label(RichText::new(map_name).strong().color(ThemeColors::ACCENT_AZURE));
    });
    ui.horizontal(|ui| {
        ui.label("World Brushes:");
        ui.label(RichText::new(format!("{}", state.world_brushes.len())).strong());
        ui.label("| Selected:");
        ui.label(RichText::new(format!("{:?}", state.selected_brush_index)).strong());
    });
    ui.horizontal(|ui| {
        let actor_count = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);
        ui.label("Loaded Actors:");
        ui.label(RichText::new(format!("{}", actor_count)).strong());
    });
    ui.horizontal(|ui| {
        ui.label("Camera Position:");
        ui.label(RichText::new(format!("({:.0}, {:.0}, {:.0})", state.camera.position.x, state.camera.position.y, state.camera.position.z)).monospace());
    });
    ui.horizontal(|ui| {
        ui.label("Grid Snapping:");
        ui.label(RichText::new(format!("{} (Step: {:.0} UU)", if state.snap_to_grid { "ON" } else { "OFF" }, state.grid_step)).strong());
    });

    ui.add_space(8.0);
    render_info_box(
        ui,
        "Creator & Original Author",
        "Nicola \"thevot\" Selenu — Architectural concept, CSG engineering, and level authoring suite design.",
        ThemeColors::ACCENT_GOLD,
    );

    ui.add_space(6.0);
    render_info_box(
        ui,
        "Support Open-Source Development",
        "Forge99 is an independent, community-driven open-source project. If you find this editor valuable, consider becoming a sponsor to help maintain multi-platform releases, CI/CD infrastructure, and continuous improvements.",
        Color32::from_rgb(255, 130, 160),
    );
    ui.horizontal(|ui| {
        if ui.button(RichText::new("❤ Sponsor Forge99 on GitHub (@thevot)").strong().color(Color32::from_rgb(255, 140, 170))).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab("https://github.com/sponsors/thevot"));
        }
        if ui.button(RichText::new("🌐 GitHub Profile (@thevot)").color(ThemeColors::TEXT_PRIMARY)).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab("https://github.com/thevot"));
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(RichText::new("License: MIT OR Apache-2.0 | Version: v0.21.0 | Created by Nicola \"thevot\" Selenu & Forge99 Contributors").size(10.5).color(ThemeColors::TEXT_MUTED));
}

// -----------------------------------------------------------------------------
// SEARCH RESULTS VIEW
// -----------------------------------------------------------------------------
fn render_search_results(ui: &mut Ui, state: &mut EditorState, query: &str) {
    ui.label(RichText::new(format!("Search Results for \"{query}\":")).strong().size(13.0).color(ThemeColors::ACCENT_AZURE));
    ui.add_space(6.0);

    // 1. Search shortcuts
    let query_clean = query.replace(' ', "").replace('/', "");
    let matching_shortcuts: Vec<&ShortcutEntry> = SHORTCUTS
        .iter()
        .filter(|entry| {
            entry.title.to_lowercase().contains(query)
                || entry.category.to_lowercase().contains(query)
                || entry.description.to_lowercase().contains(query)
                || entry.keys.iter().any(|k| {
                    let kl = k.to_lowercase();
                    kl.contains(query) || kl.replace(' ', "").replace('/', "").contains(&query_clean)
                })
        })
        .collect();

    if !matching_shortcuts.is_empty() {
        ui.label(RichText::new(format!("⌨ Matching Shortcuts ({}):", matching_shortcuts.len())).strong().color(ThemeColors::ACCENT_GOLD));
        ui.add_space(2.0);
        for entry in matching_shortcuts.iter().take(6) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(entry.icon).size(12.0));
                ui.label(RichText::new(entry.title).strong().color(ThemeColors::TEXT_PRIMARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut first = true;
                    for key in entry.keys.iter().rev() {
                        if !first {
                            ui.label(RichText::new("+").size(10.0).color(ThemeColors::TEXT_MUTED));
                        }
                        first = false;
                        render_keycap(ui, key);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                ui.label(RichText::new(entry.description).size(10.5).color(ThemeColors::TEXT_SECONDARY));
            });
            ui.add_space(2.0);
        }
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);
    }

    // 2. Matching Topics in Documentation
    let topics = [
        (HelpTab::QuickStart, "Quick Start Guide", "Subtractive architecture, room carving, lights, playerstarts, texture browser, checklist"),
        (HelpTab::CsgGeometry, "CSG Geometry & Brushes", "Solid, Semi-Solid (PF_SEMISOLID), Non-Solid, Add, Subtract, Intersect, Deintersect, 2D Shape Editor, CSG order"),
        (HelpTab::Lighting, "Lighting & Radiosity", "LightBrightness, LightRadius, LightEffect, TorchWaver, FireWaver, Spotlight, bSpecialLit, raytracing, luxmeter"),
        (HelpTab::MoversAndTriggers, "Movers, Doors & Lifts", "Mover creation, Keyframes 0..7, BumpOpenTimed, StandOpenTimed, TriggerOpenTimed, sound FX, crush rules, lift navigation"),
        (HelpTab::ZonesAndAtmosphere, "Zones, Water & Atmosphere", "ZonePortal sheets, zero-leakage, ZoneInfo, ZoneVelocity, low gravity, water swimming, distance fog, skybox"),
        (HelpTab::SurfacesAndMaterials, "Surfaces & Materials", "Surface Properties (F5), Unlit, Translucent, Masked, Modulated, TwoSided, FakeBackdrop, UV align, eyedropper, MyLevel"),
        (HelpTab::EventSystem, "Event & Tag Scripting", "Tag and Event pairing, Trigger types, Dispatcher multi-phase sequences, SpecialEvent, Counter, Teleporters"),
        (HelpTab::BotNavigation, "Bot AI & Pathing", "PathNode, ReachSpecs, 1000 UU rule, corner apexes, LiftCenter, LiftExit, JumpSpot, AlternatePath, DefensePoint"),
        (HelpTab::GameModes, "Tournament Game Modes", "DeathMatch (DM), Capture The Flag (CTF), Domination (DOM), Assault (AS) rules and objectives"),
        (HelpTab::StudiosAndTools, "Studios & Procedural Generators", "Creation Hub (Ctrl+Shift+A), Architectural Prefabs, Assault Studio, Bio Traps, Armory, LevelDoctor, 2D Shape Editor"),
        (HelpTab::UnrealEdGotchas, "UnrealEd Gotchas & FAQ", "BSP holes, Hall of Mirrors (HOM), off-grid vertices, 65,536 UU limit, zone leaks, mover pivot drift, MyLevel"),
        (HelpTab::About, "Engine Specifications", "64-bit Rust architecture, WGPU 24, multi-threaded Rayon, format compatibility, telemetry"),
    ];

    let matching_topics: Vec<_> = topics
        .iter()
        .filter(|(_, title, desc)| {
            title.to_lowercase().contains(query) || desc.to_lowercase().contains(query)
        })
        .collect();

    if !matching_topics.is_empty() {
        ui.label(RichText::new(format!("📖 Matching Documentation Sections ({}):", matching_topics.len())).strong().color(ThemeColors::ACCENT_AZURE));
        ui.add_space(4.0);
        for (tab, title, desc) in matching_topics {
            Frame::NONE
                .fill(Color32::from_rgb(22, 26, 32))
                .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
                .corner_radius(4.0)
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(*title).strong().size(12.5).color(ThemeColors::TEXT_PRIMARY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Open Section →").clicked() {
                                state.help_active_tab = *tab;
                                state.help_search_query.clear();
                            }
                        });
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(*desc).size(10.5).color(ThemeColors::TEXT_SECONDARY));
                });
            ui.add_space(4.0);
        }
    } else if matching_shortcuts.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(RichText::new("No matching topics or shortcuts found.").color(ThemeColors::TEXT_MUTED));
            if ui.button("Clear Search").clicked() {
                state.help_search_query.clear();
            }
        });
    }
}

// -----------------------------------------------------------------------------
// REUSABLE UI WIDGET HELPERS
// -----------------------------------------------------------------------------
fn render_keycap(ui: &mut Ui, key: &str) {
    Frame::NONE
        .fill(Color32::from_rgb(28, 32, 40))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 68, 85)))
        .corner_radius(3.0)
        .inner_margin(Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.label(
                RichText::new(key)
                    .monospace()
                    .size(11.0)
                    .strong()
                    .color(ThemeColors::ACCENT_AZURE),
            );
        });
}

fn render_info_box(ui: &mut Ui, title: &str, body: &str, accent_color: Color32) {
    Frame::NONE
        .fill(Color32::from_rgba_premultiplied(20, 26, 36, 230))
        .stroke(Stroke::new(1.0_f32, accent_color))
        .corner_radius(4.0)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().size(12.5).color(accent_color));
            ui.add_space(2.0);
            ui.label(RichText::new(body).size(11.0).color(ThemeColors::TEXT_PRIMARY));
        });
}

fn render_gotcha_box(ui: &mut Ui, title: &str, body: &str) {
    Frame::NONE
        .fill(Color32::from_rgba_premultiplied(40, 32, 10, 230))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(200, 150, 40)))
        .corner_radius(4.0)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("💡 UnrealEd 2.0 Gotcha: {title}"))
                    .strong()
                    .size(11.5)
                    .color(Color32::from_rgb(255, 205, 90)),
            );
            ui.add_space(2.0);
            ui.label(RichText::new(body).italics().size(10.5).color(Color32::from_rgb(240, 225, 190)));
        });
}

fn render_card(ui: &mut Ui, title: &str, body: &str, color: Color32) {
    Frame::NONE
        .fill(Color32::from_rgb(20, 24, 30))
        .stroke(Stroke::new(1.0_f32, color))
        .corner_radius(4.0)
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().size(11.5).color(color));
            ui.add_space(2.0);
            ui.label(RichText::new(body).size(10.5).color(ThemeColors::TEXT_SECONDARY));
        });
}

fn render_step_card(ui: &mut Ui, step: usize, title: &str, body: &str) {
    Frame::NONE
        .fill(Color32::from_rgb(22, 26, 34))
        .stroke(Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE))
        .corner_radius(4.0)
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                Frame::NONE
                    .fill(ThemeColors::ACCENT_AZURE)
                    .corner_radius(10.0)
                    .inner_margin(Margin::symmetric(6, 1))
                    .show(ui, |ui| {
                        ui.label(RichText::new(format!("{step}")).strong().color(Color32::WHITE).size(11.0));
                    });
                ui.label(RichText::new(title).strong().size(12.0).color(ThemeColors::TEXT_PRIMARY));
            });
            ui.add_space(2.0);
            ui.label(RichText::new(body).size(11.0).color(ThemeColors::TEXT_SECONDARY));
        });
    ui.add_space(4.0);
}
