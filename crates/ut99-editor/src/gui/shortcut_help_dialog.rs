use crate::editor_state::EditorState;

pub struct ShortcutEntry {
    pub category: &'static str,
    pub title: &'static str,
    pub keys: &'static [&'static str],
    pub icon: &'static str,
    pub description: &'static str,
}

pub const SHORTCUTS: &[ShortcutEntry] = &[
    // Viewport & Camera
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Fly / Mouselook",
        keys: &["RMB", "Drag"],
        icon: "🎥",
        description: "Freely rotate 3D camera look direction in real time",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "First-Person Fly Navigation",
        keys: &["RMB", "W / A / S / D"],
        icon: "🚀",
        description: "Move viewport camera forward, backward, strafe left and right",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Pan Viewport Camera",
        keys: &["MMB", "Drag"],
        icon: "✋",
        description: "Pan camera parallel to viewport view plane",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Orthogonal Camera Drag",
        keys: &["LMB", "Drag"],
        icon: "📐",
        description: "Move orthogonal (Top / Front / Side) camera view",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Focus on Selection",
        keys: &["F"],
        icon: "🎯",
        description: "Frame active camera onto the bounding sphere of selected objects",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Toggle 2x2 Multi-Viewport / Single",
        keys: &["Space"],
        icon: "⊞",
        description: "Switch between 4-way split view (Top, Front, Side, 3D) and full single viewport",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Active View: 3D Perspective",
        keys: &["F1"],
        icon: "🎥",
        description: "Switch active viewport to 3D Perspective mode",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Active View: Top (XY)",
        keys: &["F2"],
        icon: "📐",
        description: "Switch active viewport to Orthogonal Top (XY) mode",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Active View: Front (XZ)",
        keys: &["F3"],
        icon: "📐",
        description: "Switch active viewport to Orthogonal Front (XZ) mode",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Active View: Side (YZ)",
        keys: &["Shift", "F4"],
        icon: "📐",
        description: "Switch active viewport to Orthogonal Side (YZ) mode",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Camera Speed Preset",
        keys: &["1 .. 8"],
        icon: "⚡",
        description: "Set camera travel velocity preset (1 = slowest fine-tuning, 8 = fastest overview)",
    },
    ShortcutEntry {
        category: "Viewport & Camera",
        title: "Navigation Radar Minimap",
        keys: &["M"],
        icon: "🗺",
        description: "Toggle real-time 2D radar overlay with player starts, lights, and goals",
    },

    // Shading & Diagnostics
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Wireframe Shading",
        keys: &["Alt", "1"],
        icon: "📐",
        description: "Render all brushes and BSP geometry in wireframe outlines",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Zone / Portal View",
        keys: &["Alt", "2"],
        icon: "🚪",
        description: "Color-code geometry by zone partitions and show portal boundaries",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Unlit Shading",
        keys: &["Alt", "3"],
        icon: "☀",
        description: "Display surfaces at full diffuse brightness without shadows or lightmaps",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Lit Shading",
        keys: &["Alt", "4"],
        icon: "💡",
        description: "Full lighting mode with raytraced lightmaps and dynamic lights",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Depth Complexity Heatmap",
        keys: &["Alt", "5"],
        icon: "📊",
        description: "Overdraw heatmap shader showing pixel fill cost and occluder efficiency",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Texture Usage Color-Coding",
        keys: &["Alt", "6"],
        icon: "🎨",
        description: "Color-code BSP surfaces by distinct texture package and material assignment",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "BSP Cuts Diagnostic",
        keys: &["Alt", "7"],
        icon: "✂",
        description: "Highlight BSP tree split cuts and polygon fragmentation across world brushes",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Luxmeter Lighting Heatmap",
        keys: &["Shift", "F6"],
        icon: "💡",
        description: "False-color luminance diagnostic heatmap for competitive UT99 balance",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Bot Navigation Heatmap",
        keys: &["F9"],
        icon: "🤖",
        description: "Overlay bot AI reachability, optimal routes, and dead zone diagnostics",
    },
    ShortcutEntry {
        category: "Shading & Diagnostics",
        title: "Distance Fog Toggle",
        keys: &["F10"],
        icon: "🌫",
        description: "Toggle atmospheric volumetric depth fog simulation in 3D perspective",
    },

    // Selection & Transform
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Gizmo: Translate",
        keys: &["W"],
        icon: "↔",
        description: "Set active 3D gizmo to translation axes",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Gizmo: Rotate",
        keys: &["E"],
        icon: "🔄",
        description: "Set active 3D gizmo to pitch/yaw/roll rotation rings",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Gizmo: Scale",
        keys: &["R"],
        icon: "📐",
        description: "Set active 3D gizmo to non-uniform scaling handles",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Select Object / Actor",
        keys: &["LMB"],
        icon: "👆",
        description: "Click to select a brush, actor, light, mover, or mesh under cursor",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Multi-Select Toggle",
        keys: &["Ctrl", "LMB"],
        icon: "➕",
        description: "Add or toggle clicked actor or brush into the multi-selection set",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Select All Objects",
        keys: &["Ctrl", "A"],
        icon: "☑",
        description: "Select all actors, brushes, movers, and static meshes in level",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Invert Selection",
        keys: &["Ctrl", "I"],
        icon: "🔄",
        description: "Invert selection set across current level actors and brushes",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Progressive Escape / Deselect",
        keys: &["Esc"],
        icon: "⎋",
        description: "Progressively closes overlays, dismisses dialogs, then drops selection",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Lock / Unlock Selection",
        keys: &["Shift", "Space"],
        icon: "🔒",
        description: "Lock active selection to protect it from accidental viewport clicks or drags",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Cancel Drag Transform",
        keys: &["Esc", "RMB"],
        icon: "↩",
        description: "Cancel in-progress gizmo drag or move, instantly restoring original positions",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Constrain Drag Axis",
        keys: &["X", "Y", "Z"],
        icon: "↔",
        description: "Lock live translation to X, Y, or Z axis during drag; press again to unlock",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Invert Snap / Fine Step",
        keys: &["Ctrl", "Shift"],
        icon: "⚡",
        description: "Hold Ctrl to invert grid/rot snap during drag; hold Shift for fine 1/4x micro-stepping",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Snap to Floor",
        keys: &["End"],
        icon: "⬇",
        description: "Raycast downward to snap selected object to the nearest solid floor",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Snap Floor Aligned to Normal",
        keys: &["Shift", "End"],
        icon: "📐",
        description: "Snap selection to floor and orient rotation aligned to surface normal",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Snap Pivot to Nearest Vertex",
        keys: &["Ctrl", "End"],
        icon: "📍",
        description: "Snap brush transform pivot point to its closest geometry vertex",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Snap Objects to Grid",
        keys: &["Ctrl", "Alt", "G"],
        icon: "▦",
        description: "Snap selected object positions to active grid resolution",
    },
    ShortcutEntry {
        category: "Selection & Transform",
        title: "Grid Step: Halve / Double",
        keys: &["[", "]"],
        icon: "▦",
        description: "Halve ([) or double (]) spatial snapping grid resolution (1u .. 256u)",
    },

    // Editing & History
    ShortcutEntry {
        category: "Editing & History",
        title: "Undo",
        keys: &["Ctrl", "Z"],
        icon: "↩",
        description: "Undo the last brush, actor, texture, or vertex modification",
    },
    ShortcutEntry {
        category: "Editing & History",
        title: "Redo",
        keys: &["Ctrl", "Y"],
        icon: "↪",
        description: "Redo previously undone action",
    },
    ShortcutEntry {
        category: "Editing & History",
        title: "Duplicate Selection",
        keys: &["Ctrl", "D"],
        icon: "📑",
        description: "Duplicate selected brush, mover, static mesh, or actor offset by grid",
    },
    ShortcutEntry {
        category: "Editing & History",
        title: "Delete Selection",
        keys: &["Del"],
        icon: "🗑",
        description: "Delete currently selected actor(s) or brush(es)",
    },
    ShortcutEntry {
        category: "Editing & History",
        title: "Cut to Clipboard",
        keys: &["Ctrl", "X"],
        icon: "✂",
        description: "Cut selection to internal editor clipboard",
    },
    ShortcutEntry {
        category: "Editing & History",
        title: "Copy to Clipboard",
        keys: &["Ctrl", "C"],
        icon: "📄",
        description: "Copy selected actor or brush to clipboard",
    },
    ShortcutEntry {
        category: "Editing & History",
        title: "Paste from Clipboard",
        keys: &["Ctrl", "V"],
        icon: "📋",
        description: "Paste clipboard contents into level at current cursor / grid offset",
    },
    ShortcutEntry {
        category: "Editing & History",
        title: "Open Properties Inspector",
        keys: &["Enter"],
        icon: "⚙",
        description: "Open Actor Properties or Surface Properties for current selection",
    },

    // Surfaces & Texturing
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Apply Active Texture to Surface",
        keys: &["Alt", "RMB"],
        icon: "🎨",
        description: "Instantly apply selected package texture to clicked 3D surface",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Select All Surfaces",
        keys: &["Shift", "S"],
        icon: "🎨",
        description: "Select all polygon surfaces across the entire BSP world",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Deselect All Surfaces",
        keys: &["Shift", "N"],
        icon: "☐",
        description: "Clear active polygon surface selection",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Select Matching Texture",
        keys: &["Shift", "T"],
        icon: "🔍",
        description: "Select all surfaces in the level sharing the same texture name",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Select All Surfaces on Brush",
        keys: &["Shift", "B"],
        icon: "📦",
        description: "Select all polygon surfaces belonging to the currently selected brush(es)",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Select Coplanar Adjacent Surfaces",
        keys: &["Shift", "C"],
        icon: "📐",
        description: "Select contiguous adjacent coplanar surfaces sharing the same normal",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Select Adjacent Wall Surfaces",
        keys: &["Shift", "W"],
        icon: "🧱",
        description: "Select contiguous vertical wall surfaces",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Select Adjacent Slants / Floors",
        keys: &["Shift", "Q"],
        icon: "📐",
        description: "Select contiguous non-vertical floor and ramp surfaces",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Invert Surface Selection",
        keys: &["Ctrl", "Shift", "I"],
        icon: "🔄",
        description: "Invert selected surfaces across active world brushes",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Pan Texture UV (Up / Down / Left / Right)",
        keys: &["Shift", "Arrows"],
        icon: "🖼",
        description: "Pan texture coordinate alignment along U and V axes by grid step",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Rotate Texture 45°",
        keys: &["Shift+R", "Ctrl+Shift+R"],
        icon: "🔄",
        description: "Rotate texture UV mapping by 45 degrees clockwise",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Scale Texture UV (Zoom In / Out)",
        keys: &["Shift", "PgUp / PgDn"],
        icon: "↔",
        description: "Scale texture UV coordinates by 1.25x or 0.8x",
    },
    ShortcutEntry {
        category: "Surfaces & Texturing",
        title: "Fit Texture to Surface (1x1)",
        keys: &["Shift+F", "Ctrl+Shift+F"],
        icon: "🎯",
        description: "Automatically scale and pan texture to fit 1 repetition on polygon (or select adjacent floors when surfaces are active with Shift+F)",
    },

    // CSG & Brush Geometry
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "CSG Add (Additive Brush)",
        keys: &["CSG Add", "Left Toolbar"],
        icon: "➕",
        description: "Commit the Red Builder Brush as a solid additive world brush",
    },
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "CSG Subtract (Carve Space)",
        keys: &["CSG Sub", "Left Toolbar"],
        icon: "➖",
        description: "Commit the Red Builder Brush to carve hollow playable room space",
    },
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "CSG Intersect Builder",
        keys: &["CSG Intersect"],
        icon: "✂",
        description: "Trim the Red Builder Brush to match the intersection volume of world geometry",
    },
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "CSG Deintersect Builder",
        keys: &["CSG Deintersect"],
        icon: "✂",
        description: "Trim the Red Builder Brush to remove volume overlapping solid world brushes",
    },
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "Split Selected Edge (Vertex Edit)",
        keys: &["Ctrl", "Alt", "E"],
        icon: "✂",
        description: "Insert a new midpoint vertex on selected edge for precise bevels and chamfers",
    },
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "Collapse Edge / Merge Vertices",
        keys: &["Ctrl", "Alt", "M"],
        icon: "💥",
        description: "Collapse selected edge by welding its endpoints together",
    },
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "Snap Vertices to Grid",
        keys: &["Ctrl", "Alt", "S"],
        icon: "▦",
        description: "Snap all vertices of selected brush to current grid snapping resolution",
    },
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "Weld Coincident Vertices",
        keys: &["Ctrl", "Alt", "W"],
        icon: "🧲",
        description: "Merge coincident vertices within 16 units tolerance",
    },
    ShortcutEntry {
        category: "CSG & Brush Geometry",
        title: "Invert Brush Normals",
        keys: &["Ctrl", "Alt", "I"],
        icon: "🔄",
        description: "Flip poly winding order and invert normals of selected brush",
    },

    // System & Windows
    ShortcutEntry {
        category: "System & Windows",
        title: "Quick Command Palette",
        keys: &["Ctrl", "P"],
        icon: "⚡",
        description: "Fuzzy-search any editor action, tool studio, or inspector",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Keyboard Shortcuts Cheatsheet",
        keys: &["?", "Shift+F1"],
        icon: "❓",
        description: "Open this interactive keyboard shortcuts reference overlay",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Toggle Bottom Console Drawer",
        keys: &["Ctrl+~", "Ctrl+Shift+C"],
        icon: "📟",
        description: "Show or hide the collapsible execution log console drawer (+96px space)",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "New Valid UT99 Map Generator Wizard",
        keys: &["Ctrl", "Shift", "N"],
        icon: "🧙",
        description: "Open interactive step-by-step UT99 map generator & Official .unr DNA Cloner (DM, CTF, DOM, AS)",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Open .unr Map",
        keys: &["Ctrl", "O"],
        icon: "📂",
        description: "Launch native file picker to open Unreal Tournament map",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Save Map",
        keys: &["Ctrl", "S"],
        icon: "💾",
        description: "Save changes to current .unr map file",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Save Map As...",
        keys: &["Ctrl", "Shift", "S"],
        icon: "💾",
        description: "Save current map to a new .unr file path",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Build Options & Compilers",
        keys: &["Ctrl", "Shift", "B"],
        icon: "⚙",
        description: "Configure BSP tree compiler, radiosity lighting, and AI path settings",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Play Map in UT99 Engine",
        keys: &["F8"],
        icon: "🎮",
        description: "Launch map directly into the native 64-bit Unreal Tournament 99 engine",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Texture Browser",
        keys: &["F4"],
        icon: "🖼",
        description: "Open package texture browser and material library",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Surface Properties",
        keys: &["F5"],
        icon: "🎨",
        description: "Open surface flags, panning, scaling, and alignment inspector",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Level Properties",
        keys: &["F6"],
        icon: "📋",
        description: "Configure map title, author, song, ambient lighting, and game rules",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Performance & Budget Profiler",
        keys: &["F7"],
        icon: "📊",
        description: "View real-time polygon, draw call, and memory budget metrics",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Toggle Fullscreen",
        keys: &["F11", "Alt+Enter"],
        icon: "⛶",
        description: "Toggle between borderless fullscreen and maximized window",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Zone Acoustics & Reverb",
        keys: &["Shift", "F11"],
        icon: "🔊",
        description: "Configure zone audio resonance, EAX presets, and ambient soundscapes",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Tournament Item Balance",
        keys: &["F12"],
        icon: "⚖",
        description: "Inspect weapon spawn timers, health density, and CTF travel balance",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Camera Bookmarks Manager",
        keys: &["Ctrl", "B"],
        icon: "🔖",
        description: "Manage 10 saved camera viewpoints (Ctrl+0..9 to save, Shift+0..9 to recall)",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Find Actors in Level",
        keys: &["Ctrl", "F"],
        icon: "🔍",
        description: "Filter and locate actors by name, class, tag, or event",
    },
    ShortcutEntry {
        category: "System & Windows",
        title: "Find and Replace Textures",
        keys: &["Ctrl", "R"],
        icon: "🔁",
        description: "Batch replace texture references across level geometry",
    },
];

pub fn render_shortcut_help_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_shortcut_help_dialog {
        return;
    }

    let mut open = true;
    let screen_rect = ctx.input(|i| i.screen_rect());
    let window_width = 720.0_f32.min(screen_rect.width() - 40.0);
    let window_height = 560.0_f32.min(screen_rect.height() - 60.0);

    egui::Window::new("Keyboard Shortcuts & Controls Cheatsheet (? / Shift+F1)")
        .open(&mut open)
        .default_size(egui::vec2(window_width, window_height))
        .collapsible(false)
        .resizable(true)
        .show(ctx, |ui| {
            // Header / Search Bar
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔍").size(16.0));
                let search_box = ui.add(
                    egui::TextEdit::singleline(&mut state.shortcut_help_search)
                        .hint_text("Filter shortcuts by key, action, or category...")
                        .desired_width(ui.available_width() - 70.0)
                );
                search_box.request_focus();

                if !state.shortcut_help_search.is_empty() && ui.button("✖").on_hover_text("Clear filter").clicked() {
                    state.shortcut_help_search.clear();
                }
            });

            ui.separator();

            let query = state.shortcut_help_search.trim().to_lowercase();
            let filtered: Vec<&ShortcutEntry> = SHORTCUTS
                .iter()
                .filter(|entry| {
                    if query.is_empty() {
                        true
                    } else {
                        entry.title.to_lowercase().contains(&query)
                            || entry.category.to_lowercase().contains(&query)
                            || entry.description.to_lowercase().contains(&query)
                            || entry.keys.iter().any(|k| k.to_lowercase().contains(&query))
                    }
                })
                .collect();

            if filtered.is_empty() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("No matching shortcuts found.").color(egui::Color32::from_rgb(160, 165, 180)));
                    if ui.button("Clear Search").clicked() {
                        state.shortcut_help_search.clear();
                    }
                });
            } else {
                // Group by category
                let mut current_category = "";
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for entry in filtered {
                            if entry.category != current_category {
                                current_category = entry.category;
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(current_category)
                                            .strong()
                                            .size(13.0)
                                            .color(egui::Color32::from_rgb(120, 200, 255))
                                    );
                                });
                                ui.separator();
                            }

                            ui.horizontal(|ui| {
                                ui.set_min_height(26.0);
                                // Left icon and title
                                ui.label(egui::RichText::new(entry.icon).size(14.0));
                                ui.label(egui::RichText::new(entry.title).strong().color(egui::Color32::from_rgb(230, 235, 245)));

                                // Right-aligned keycaps
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let mut first = true;
                                    for key in entry.keys.iter().rev() {
                                        if !first {
                                            ui.label(egui::RichText::new("+").size(11.0).color(egui::Color32::from_rgb(130, 135, 150)));
                                        }
                                        first = false;

                                        egui::Frame::new()
                                            .fill(egui::Color32::from_rgb(32, 35, 45))
                                            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(70, 78, 96)))
                                            .corner_radius(4.0_f32)
                                            .inner_margin(egui::Margin::symmetric(6, 2))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(*key)
                                                        .monospace()
                                                        .size(11.5)
                                                        .strong()
                                                        .color(egui::Color32::from_rgb(250, 252, 255))
                                                );
                                            });
                                    }
                                });
                            });

                            // Subtitle description
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.label(
                                    egui::RichText::new(entry.description)
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(160, 168, 185))
                                );
                            });

                            ui.add_space(4.0);
                        }
                    });
            }
        });

    if !open {
        state.show_shortcut_help_dialog = false;
        state.shortcut_help_search.clear();
    }
}
