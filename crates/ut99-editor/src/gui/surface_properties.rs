use crate::editor_state::{EditorState, SurfaceFlagsPreset};
use ut99_bsp::{
    PF_AUTO_UPAN, PF_AUTO_VPAN, PF_BIG_WAVY, PF_BRIGHT_CORNERS, PF_DIRTY_SHADOWS,
    PF_FAKE_BACKDROP, PF_FORCE_VIEW_ZONE, PF_HIGH_SHADOW_DETAIL, PF_INVISIBLE,
    PF_LOW_SHADOW_DETAIL, PF_MASKED, PF_MIRRORED, PF_MODULATED, PF_NO_BOUNDS_REJECT,
    PF_NO_SMOOTH, PF_NOT_SOLID, PF_PORTAL, PF_SEMISOLID, PF_SMALL_WAVY, PF_SPECIAL_LIT,
    PF_SPECIAL_POLY, PF_TRANSLUCENT, PF_TWO_SIDED, PF_UNLIT,
};

pub fn render_surface_properties_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_surface_properties {
        return;
    }

    let mut open = state.show_surface_properties;
    crate::gui::modal::standard_dialog_window(ctx, "Surface Properties (F5)", &mut open, [420.0, 560.0], [340.0, 320.0])
        .show(ctx, |ui| {
            // Target information
            let target_desc = if !state.selected_surfaces.is_empty() {
                format!("{} Selected Surface(s)", state.selected_surfaces.len())
            } else if let Some(idx) = state.selected_brush_index {
                format!("World Brush #{} ({})", idx, state.world_brushes.get(idx).map(|b| b.name.as_str()).unwrap_or("Brush"))
            } else {
                "Active Red Builder Brush".to_string()
            };

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Target:").strong());
                ui.label(egui::RichText::new(&target_desc).color(egui::Color32::from_rgb(100, 200, 255)));
                if !state.selected_surfaces.is_empty() {
                    if ui.small_button("Clear Selection").clicked() {
                        state.deselect_all_surfaces();
                    }
                    if ui.small_button("✂ Extract to Builder").on_hover_text("Clone selected surface(s) as a planar sheet into Red Builder Brush").clicked() {
                        state.extract_selected_surfaces_to_builder_brush();
                    }
                }
            });

            ui.separator();

            // 0. Surface Selection Controls (UnrealEd Shortcuts grouped by category)
            ui.label(egui::RichText::new("Surface Selection Filters").strong());

            // Sub-row 1: Adjacency & Coplanarity
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Adjacency:").size(11.0).color(egui::Color32::from_rgb(160, 175, 195)));
                if ui.small_button("Coplanar (Shift+C)").on_hover_text("Select coplanar adjacent surfaces sharing edges (Shift+C)").clicked() {
                    state.select_surfaces_coplanar_adjacent();
                }
                if ui.small_button("All Coplanar").on_hover_text("Select all surfaces in level on the same plane").clicked() {
                    state.select_surfaces_coplanar_all();
                }
                if ui.small_button("Adj All (Shift+J)").on_hover_text("Select all adjacent edge-sharing surfaces (Shift+J)").clicked() {
                    state.select_surfaces_adjacent_all();
                }
                if ui.small_button("Walls (Shift+W)").on_hover_text("Select adjacent vertical wall surfaces (Shift+W)").clicked() {
                    state.select_surfaces_adjacent_walls();
                }
                if ui.small_button("Floors (Shift+F)").on_hover_text("Select adjacent floor/ceiling surfaces (Shift+F)").clicked() {
                    state.select_surfaces_adjacent_floors();
                }
                if ui.small_button("Slants (Shift+S)").on_hover_text("Select adjacent sloped surfaces (Shift+S)").clicked() {
                    state.select_surfaces_adjacent_slants();
                }
            });

            // Sub-row 2: Matching Properties
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Matching: ").size(11.0).color(egui::Color32::from_rgb(160, 175, 195)));
                if ui.small_button("Texture (Shift+T)").on_hover_text("Select all surfaces in level with matching texture (Shift+T)").clicked() {
                    state.select_surfaces_matching_texture();
                }
                if ui.small_button("Brush (Shift+B)").on_hover_text("Select all surfaces on the currently selected brush(es) (Shift+B)").clicked() {
                    state.select_surfaces_on_selected_brushes();
                }
                if ui.small_button("Zone (Shift+Z)").on_hover_text("Select all surfaces belonging to the same 3D BSP Zone (POLY SELECT ZONE)").clicked() {
                    state.select_surfaces_matching_zone();
                }
                if ui.small_button("Groups (Shift+G)").on_hover_text("Select all surfaces in matching brush group(s) (Shift+G)").clicked() {
                    state.select_surfaces_matching_groups();
                }
                if ui.small_button("Items (Shift+I)").on_hover_text("Select all surfaces with matching CSG/PolyFlags item archetype (Shift+I)").clicked() {
                    state.select_surfaces_matching_items();
                }
            });

            // Sub-row 3: Scope & Inversion
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Scope:    ").size(11.0).color(egui::Color32::from_rgb(160, 175, 195)));
                if ui.small_button("Select All").on_hover_text("Select all surfaces in level").clicked() {
                    state.select_all_surfaces();
                }
                if ui.small_button("Invert (Shift+Q)").on_hover_text("Invert surface selection (Shift+Q)").clicked() {
                    state.invert_surface_selection();
                }
                if ui.small_button("Clear (None)").on_hover_text("Deselect all surfaces (Shift+N)").clicked() {
                    state.deselect_all_surfaces();
                }
                if ui.small_button("🔁 Replace Textures...").on_hover_text("Open Find & Replace Textures dialog (Ctrl+R)").clicked() {
                    state.show_replace_textures_dialog = true;
                    if !state.selected_surfaces.is_empty() {
                        state.replace_texture_scope = crate::editor_state::TextureReplaceScope::SelectedSurfaces;
                    }
                }
            });

            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(format!("Memory ({}):", state.memorized_surfaces.len())).size(11.0).strong());
                if ui.small_button("M (Memorize)").on_hover_text("Memorize current surface selection (Shift+M)").clicked() {
                    state.memorize_surface_selection();
                }
                if ui.small_button("R (Recall)").on_hover_text("Recall memorized surface selection (Shift+R)").clicked() {
                    state.recall_surface_selection();
                }
                if ui.small_button("O (OR ∪)").on_hover_text("Union current selection with memorized surfaces (Shift+O)").clicked() {
                    state.or_surface_selection();
                }
                if ui.small_button("U (AND ∩)").on_hover_text("Intersect current selection with memorized surfaces (Shift+U)").clicked() {
                    state.and_surface_selection();
                }
                if ui.small_button("X (XOR Δ)").on_hover_text("Symmetric difference with memorized surfaces (Shift+X)").clicked() {
                    state.xor_surface_selection();
                }
            });

            ui.separator();

            // 1. Material Assignment
            ui.label(egui::RichText::new("Texture Material").strong());
            ui.horizontal(|ui| {
                if let Some(ref tex) = state.active_texture {
                    ui.label(egui::RichText::new(tex).strong().color(egui::Color32::from_rgb(255, 215, 0)));
                } else {
                    ui.label("None selected");
                }
                if ui.button("🖼 Browse...").clicked() {
                    state.show_texture_browser = true;
                }
                if ui.button("Apply Material").clicked() {
                    state.apply_active_texture_to_selected_brush();
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .button("📋 Propagate Texture & UV (Ctrl+Alt+LMB)")
                    .on_hover_text("Copies texture name, TextureU, TextureV, and world-aligned Base from the first selected surface to all other selected surfaces (REQ-D02)")
                    .clicked()
                {
                    state.propagate_first_selected_surface_uv_to_all();
                }
                if ui
                    .button("💡 Apply Lights to Selected")
                    .on_hover_text("Bakes raytraced lighting strictly onto currently selected surfaces without touching unselected surfaces (REQ-F07)")
                    .clicked()
                {
                    state.build_lighting_for_selected_surfaces();
                }
            });

            ui.separator();

            // 2. Alignment Presets
            ui.label(egui::RichText::new("Texture Alignment").strong());
            ui.horizontal(|ui| {
                if ui.button("Floor / Ceiling").clicked() {
                    state.align_selected_brush_texture("floor");
                }
                if ui.button("Wall X").clicked() {
                    state.align_selected_brush_texture("wall_x");
                }
                if ui.button("Wall Y").clicked() {
                    state.align_selected_brush_texture("wall_y");
                }
                if ui.button("🧱 Auto Wall Direction").on_hover_text("Aligns texture along the natural wall plane without angle distortion (UnrealEd Standard)").clicked() {
                    state.align_selected_brush_texture("wall_auto");
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Planar").clicked() {
                    state.align_selected_brush_texture("planar");
                }
                if ui.button("Box (Auto)").clicked() {
                    state.align_selected_brush_texture("box");
                }
                if ui.button("📐 Fit 1x1 (Shift+F)").on_hover_text("Auto-fit texture 1x1 across surface bounding span (Shift+F)").clicked() {
                    state.fit_selected_brush_texture(1.0, 1.0);
                }
                if ui.button("📐 Fit 2x2").on_hover_text("Auto-fit texture with 2x2 repetitions across span").clicked() {
                    state.fit_selected_brush_texture(2.0, 2.0);
                }
                if ui.button("📐 Fit 4x4").on_hover_text("Auto-fit texture with 4x4 repetitions across span").clicked() {
                    state.fit_selected_brush_texture(4.0, 4.0);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("🌀 Cylinder (1x)").on_hover_text("Unwraps texture around circular/curved walls seamlessly (1 repetition)").clicked() {
                    state.align_selected_brush_cylinder(1.0, 1.0);
                }
                if ui.button("🌀 Cylinder (2x)").on_hover_text("Unwraps texture around circular/curved walls seamlessly (2 repetitions)").clicked() {
                    state.align_selected_brush_cylinder(2.0, 1.0);
                }
                if ui.button("🌀 Cylinder (4x)").on_hover_text("Unwraps texture around circular/curved walls seamlessly (4 repetitions)").clicked() {
                    state.align_selected_brush_cylinder(4.0, 1.0);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("🔄 Continuous Strip (1x)").on_hover_text("Continuously aligns adjacent wall faces across curved stairs, arches, and corridors with zero UV seams").clicked() {
                    state.align_selected_brush_continuous_strip(1.0, 1.0);
                }
                if ui.button("🔄 Continuous Strip (2x)").on_hover_text("Continuously aligns adjacent wall faces with 2 horizontal repeats").clicked() {
                    state.align_selected_brush_continuous_strip(2.0, 1.0);
                }
            });

            ui.separator();

            // 3. Pan Controls (WRK-02 Continuous Decimal & Differential Undo + UnrealEd 2.0 1/4/16/64 buttons)
            ui.label(egui::RichText::new("Pan (Translation)").strong());
            let prev_pan_u = state.uv_drag_pan_u;
            let prev_pan_v = state.uv_drag_pan_v;

            ui.horizontal(|ui| {
                ui.label("Pan U:");
                let r_u = ui.add(
                    egui::DragValue::new(&mut state.uv_drag_pan_u)
                        .speed(0.5)
                        .suffix(" px")
                        .fixed_decimals(1)
                ).on_hover_text("Continuous decimal UV Pan along U axis (drag or type value)");

                if r_u.drag_started() || r_u.gained_focus() {
                    if state.uv_edit_initial_polys.is_none() {
                        state.uv_edit_initial_polys = Some(state.snapshot_selected_brush_polys());
                    }
                }
                if r_u.changed() {
                    let delta = state.uv_drag_pan_u - prev_pan_u;
                    if delta != 0.0 {
                        state.pan_selected_brush_texture_raw(delta, 0.0);
                    }
                }
                if r_u.drag_stopped() || r_u.lost_focus() {
                    if let Some(initial) = state.uv_edit_initial_polys.take() {
                        state.commit_uv_modification(&initial, format!("Continuous Pan U (total: {:+.1}px)", state.uv_drag_pan_u));
                    }
                    state.uv_drag_pan_u = 0.0;
                }

                for step in [-64.0, -16.0, -4.0, -1.0, 1.0, 4.0, 16.0, 64.0] {
                    let lbl = if step > 0.0 { format!("+{}", step as i32) } else { format!("{}", step as i32) };
                    if ui.small_button(lbl).clicked() {
                        state.pan_selected_brush_texture(step, 0.0);
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Pan V:");
                let r_v = ui.add(
                    egui::DragValue::new(&mut state.uv_drag_pan_v)
                        .speed(0.5)
                        .suffix(" px")
                        .fixed_decimals(1)
                ).on_hover_text("Continuous decimal UV Pan along V axis (drag or type value)");

                if r_v.drag_started() || r_v.gained_focus() {
                    if state.uv_edit_initial_polys.is_none() {
                        state.uv_edit_initial_polys = Some(state.snapshot_selected_brush_polys());
                    }
                }
                if r_v.changed() {
                    let delta = state.uv_drag_pan_v - prev_pan_v;
                    if delta != 0.0 {
                        state.pan_selected_brush_texture_raw(0.0, delta);
                    }
                }
                if r_v.drag_stopped() || r_v.lost_focus() {
                    if let Some(initial) = state.uv_edit_initial_polys.take() {
                        state.commit_uv_modification(&initial, format!("Continuous Pan V (total: {:+.1}px)", state.uv_drag_pan_v));
                    }
                    state.uv_drag_pan_v = 0.0;
                }

                for step in [-64.0, -16.0, -4.0, -1.0, 1.0, 4.0, 16.0, 64.0] {
                    let lbl = if step > 0.0 { format!("+{}", step as i32) } else { format!("{}", step as i32) };
                    if ui.small_button(lbl).clicked() {
                        state.pan_selected_brush_texture(0.0, step);
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("🎯 Reset Pan (0,0)").on_hover_text("Align UV texture origin directly to surface first vertex").clicked() {
                    state.reset_selected_surfaces_pan();
                }
                if ui.button("▦ Snap UV 16px").on_hover_text("Snap UV texture pan to nearest 16-pixel grid").clicked() {
                    state.snap_selected_surfaces_uv_to_grid(16.0);
                }
                if ui.button("▦ Snap UV 64px").on_hover_text("Snap UV texture pan to nearest 64-pixel grid").clicked() {
                    state.snap_selected_surfaces_uv_to_grid(64.0);
                }
            });

            ui.separator();

            // 4. Scale & Rotation (WRK-02 Continuous Precision & Differential Undo)
            ui.label(egui::RichText::new("Scale & Rotation").strong());
            let prev_scale_u = state.uv_drag_scale_u;
            let prev_scale_v = state.uv_drag_scale_v;
            let prev_rot = state.uv_drag_rot;

            ui.horizontal(|ui| {
                ui.label("Scale U:");
                let r_su = ui.add(
                    egui::DragValue::new(&mut state.uv_drag_scale_u)
                        .speed(0.01)
                        .range(0.01..=100.0)
                        .fixed_decimals(3)
                ).on_hover_text("Continuous decimal UV Scale along U axis");

                if r_su.drag_started() || r_su.gained_focus() {
                    if state.uv_edit_initial_polys.is_none() {
                        state.uv_edit_initial_polys = Some(state.snapshot_selected_brush_polys());
                    }
                }
                if r_su.changed() && prev_scale_u > 0.0 {
                    let factor = state.uv_drag_scale_u / prev_scale_u;
                    state.scale_selected_brush_texture_raw(factor, 1.0);
                }
                if r_su.drag_stopped() || r_su.lost_focus() {
                    if let Some(initial) = state.uv_edit_initial_polys.take() {
                        state.commit_uv_modification(&initial, format!("Continuous Scale U (factor: {:.3})", state.uv_drag_scale_u));
                    }
                    state.uv_drag_scale_u = 1.0;
                }

                if ui.button("0.5x").clicked() { state.scale_selected_brush_texture(0.5, 1.0); }
                if ui.button("2.0x").clicked() { state.scale_selected_brush_texture(2.0, 1.0); }
            });

            ui.horizontal(|ui| {
                ui.label("Scale V:");
                let r_sv = ui.add(
                    egui::DragValue::new(&mut state.uv_drag_scale_v)
                        .speed(0.01)
                        .range(0.01..=100.0)
                        .fixed_decimals(3)
                ).on_hover_text("Continuous decimal UV Scale along V axis");

                if r_sv.drag_started() || r_sv.gained_focus() {
                    if state.uv_edit_initial_polys.is_none() {
                        state.uv_edit_initial_polys = Some(state.snapshot_selected_brush_polys());
                    }
                }
                if r_sv.changed() && prev_scale_v > 0.0 {
                    let factor = state.uv_drag_scale_v / prev_scale_v;
                    state.scale_selected_brush_texture_raw(1.0, factor);
                }
                if r_sv.drag_stopped() || r_sv.lost_focus() {
                    if let Some(initial) = state.uv_edit_initial_polys.take() {
                        state.commit_uv_modification(&initial, format!("Continuous Scale V (factor: {:.3})", state.uv_drag_scale_v));
                    }
                    state.uv_drag_scale_v = 1.0;
                }

                if ui.button("0.5x").clicked() { state.scale_selected_brush_texture(1.0, 0.5); }
                if ui.button("2.0x").clicked() { state.scale_selected_brush_texture(1.0, 2.0); }
            });

            ui.horizontal(|ui| {
                ui.label("Rotate:");
                let r_rot = ui.add(
                    egui::DragValue::new(&mut state.uv_drag_rot)
                        .speed(1.0)
                        .suffix("°")
                        .fixed_decimals(1)
                ).on_hover_text("Continuous UV Angle rotation (drag or type angle)");

                if r_rot.drag_started() || r_rot.gained_focus() {
                    if state.uv_edit_initial_polys.is_none() {
                        state.uv_edit_initial_polys = Some(state.snapshot_selected_brush_polys());
                    }
                }
                if r_rot.changed() {
                    let delta = state.uv_drag_rot - prev_rot;
                    if delta != 0.0 {
                        state.rotate_selected_brush_texture_raw(delta);
                    }
                }
                if r_rot.drag_stopped() || r_rot.lost_focus() {
                    if let Some(initial) = state.uv_edit_initial_polys.take() {
                        state.commit_uv_modification(&initial, format!("Continuous Rotate (angle: {:+.1}°)", state.uv_drag_rot));
                    }
                    state.uv_drag_rot = 0.0;
                }

                if ui.button("-45°").clicked() { state.rotate_selected_brush_texture(-45.0); }
                if ui.button("+45°").clicked() { state.rotate_selected_brush_texture(45.0); }
                if ui.button("+90°").clicked() { state.rotate_selected_brush_texture(90.0); }
            });

            ui.horizontal(|ui| {
                ui.label("Mirror / Skew:");
                if ui.button("↔ Flip U").clicked() { state.flip_selected_brush_texture_u(); }
                if ui.button("↕ Flip V").clicked() { state.flip_selected_brush_texture_v(); }
                if ui.button("⟋ Small Diag").on_hover_text("UnrealEd 2.0 Small Diagonal UV Skew (UU=1, VV=1, UV=1, VU=-1)").clicked() {
                    state.skew_selected_brush_texture_diagonal(true);
                }
                if ui.button("⟍ Big Diag").on_hover_text("UnrealEd 2.0 Big Diagonal UV Skew (UU=1, VV=0.5, UV=0.5, VU=-0.5)").clicked() {
                    state.skew_selected_brush_texture_diagonal(false);
                }
            });

            ui.separator();

            // 5. Surface Flags Presets & Canonical PolyFlags (UI-04 Clean Studio Icons)
            ui.label(egui::RichText::new("⚡ Quick Presets (1-Click)").strong());
            ui.horizontal_wrapped(|ui| {
                if ui.button("≋ Water").on_hover_text("Translucent + SmallWavy + NotSolid (Water surfaces)").clicked() {
                    state.apply_surface_flags_preset(SurfaceFlagsPreset::Water);
                }
                if ui.button("♨ Lava/Slime").on_hover_text("SmallWavy + Unlit (Hazard liquid surfaces)").clicked() {
                    state.apply_surface_flags_preset(SurfaceFlagsPreset::LavaSlime);
                }
                if ui.button("⬚ Glass").on_hover_text("Translucent + TwoSided (Windows and transparent barriers)").clicked() {
                    state.apply_surface_flags_preset(SurfaceFlagsPreset::GlassWindow);
                }
                if ui.button("▦ Grate").on_hover_text("Masked + TwoSided (Gratings, fences, wire mesh)").clicked() {
                    state.apply_surface_flags_preset(SurfaceFlagsPreset::MaskedGrate);
                }
                if ui.button("☁ Sky").on_hover_text("FakeBackdrop + Unlit (Skybox projection surface)").clicked() {
                    state.apply_surface_flags_preset(SurfaceFlagsPreset::SkyBackdrop);
                }
                if ui.button("⬡ Invisible").on_hover_text("Invisible (Solid invisible blocking collision)").clicked() {
                    state.apply_surface_flags_preset(SurfaceFlagsPreset::InvisibleBarrier);
                }
                if ui.button("☼ Unlit").on_hover_text("Unlit (Fullbright, ignores world shadow casting)").clicked() {
                    state.apply_surface_flags_preset(SurfaceFlagsPreset::FullbrightUnlit);
                }
                if ui.button("✕ Clear").on_hover_text("Clear all flags back to standard solid opaque surface").clicked() {
                    state.apply_surface_flags_preset(SurfaceFlagsPreset::ClearAll);
                }
            });

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Surface Flags (21 Canonical UnrealEd 2.0 PolyFlags)").strong());

            let (all_flags, any_flags, selection_count) = state.selected_surface_flags_summary();

            ui.horizontal_wrapped(|ui| {
                let mut toggle_flag = |ui: &mut egui::Ui, name: &str, flag: u32, tip: &str| {
                    let is_all = (all_flags & flag) != 0;
                    let is_any = (any_flags & flag) != 0;
                    let is_mixed = is_any && !is_all;

                    let label_text = if is_mixed {
                        egui::RichText::new(format!("≈ {}", name)).color(egui::Color32::from_rgb(245, 185, 75))
                    } else if is_all {
                        egui::RichText::new(name).strong()
                    } else {
                        egui::RichText::new(name)
                    };

                    let full_tip = if is_mixed {
                        format!("{} [Mixed state across {} surfaces: click to enable on all]", tip, selection_count)
                    } else {
                        format!("{} [Click to {} on all {} surfaces]", tip, if is_all { "disable" } else { "enable" }, selection_count)
                    };

                    let resp = if is_mixed {
                        ui.add_enabled(selection_count > 0, egui::Button::new(label_text).fill(egui::Color32::from_rgb(55, 45, 20)))
                    } else {
                        ui.add_enabled(selection_count > 0, egui::SelectableLabel::new(is_all, label_text))
                    };

                    if resp.on_hover_text(full_tip).clicked() {
                        if is_all { state.set_selected_surfaces_flags(0, flag); }
                        else { state.set_selected_surfaces_flags(flag, 0); }
                    }
                };

                toggle_flag(ui, "Invisible", PF_INVISIBLE, "PF_Invisible: Geometry is collision-only, surface invisible");
                toggle_flag(ui, "Masked", PF_MASKED, "PF_Masked: 1-bit alpha transparency via palette index 0");
                toggle_flag(ui, "Translucent", PF_TRANSLUCENT, "PF_Translucent: Additive blending (water, glass)");
                toggle_flag(ui, "Force View Zone", PF_FORCE_VIEW_ZONE, "PF_ForceViewZone: Force rendering in current view zone");
                toggle_flag(ui, "Modulated", PF_MODULATED, "PF_Modulated: Multiplicative blending (shadows, stains)");
                toggle_flag(ui, "Fake Backdrop", PF_FAKE_BACKDROP, "PF_FakeBackdrop: SkyBox projection anchor");
                toggle_flag(ui, "Two-Sided", PF_TWO_SIDED, "PF_TwoSided: Render surface from both sides");
                toggle_flag(ui, "U-Pan", PF_AUTO_UPAN, "PF_AutoUPan: Automatic horizontal texture scrolling");
                toggle_flag(ui, "V-Pan", PF_AUTO_VPAN, "PF_AutoVPan: Automatic vertical texture scrolling");
                toggle_flag(ui, "High Shadow Detail", PF_HIGH_SHADOW_DETAIL, "PF_HighShadowDetail: 2x finer lightmap luxel density (sharper shadows)");
                toggle_flag(ui, "Low Shadow Detail", PF_LOW_SHADOW_DETAIL, "PF_LowShadowDetail: 2x coarser lightmap luxel density (saves memory)");
                toggle_flag(ui, "No Smooth", PF_NO_SMOOTH, "PF_NoSmooth: Nearest-neighbor pixelated texture filtering");
                toggle_flag(ui, "Special Poly", PF_SPECIAL_POLY, "PF_SpecialPoly: Game-specific special polygon flag");
                toggle_flag(ui, "Small Wavy", PF_SMALL_WAVY, "PF_SmallWavy: High-frequency surface ripple animation");
                toggle_flag(ui, "Big Wavy", PF_BIG_WAVY, "PF_BigWavy: Low-frequency undulating waves");
                toggle_flag(ui, "Dirty Shadows", PF_DIRTY_SHADOWS, "PF_DirtyShadows: Gritty/noisy shadowmap modulation");
                toggle_flag(ui, "Bright Corners", PF_BRIGHT_CORNERS, "PF_BrightCorners: Prevent dark corner shading on acute angles");
                toggle_flag(ui, "Special Lit", PF_SPECIAL_LIT, "PF_SpecialLit: Only illuminated by lights marked bSpecialLit");
                toggle_flag(ui, "No Bounds Reject", PF_NO_BOUNDS_REJECT, "PF_NoBoundsReject: Never reject surface by bounding box culling");
                toggle_flag(ui, "Unlit", PF_UNLIT, "PF_Unlit: Fullbright, unaffected by world lighting/shadows");
                toggle_flag(ui, "Portal", PF_PORTAL, "PF_Portal: Zone boundary portal for occlusion culling");
                toggle_flag(ui, "Mirror", PF_MIRRORED, "PF_Mirrored: Flat planar mirror reflection");
                toggle_flag(ui, "Semisolid", PF_SEMISOLID, "PF_Semisolid: Non-BSP-splitting additive brush");
                toggle_flag(ui, "Not Solid", PF_NOT_SOLID, "PF_NotSolid: Pass-through geometry with no collision");
            });
        });

    state.show_surface_properties = open;
}
