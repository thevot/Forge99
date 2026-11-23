use crate::editor_state::EditorState;

pub struct PaletteCommand {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub shortcut: &'static str,
    pub icon: &'static str,
}

pub fn render_command_palette(ctx: &egui::Context, state: &mut EditorState) {
    // Temporary UI state distinguishes opening/query changes from ordinary idle frames.
    let interaction_id = egui::Id::new("command_palette_last_query");
    if !state.show_command_palette {
        ctx.data_mut(|data| data.remove::<String>(interaction_id));
        return;
    }

    let previous_query = ctx.data(|data| data.get_temp::<String>(interaction_id));
    let mut reveal_selection = previous_query.is_none();
    let mut window_open = true;
    let mut close_requested = false;
    let mut action_to_execute: Option<&'static str> = None;

    let commands: &[PaletteCommand] = &[
        // File & Project
        PaletteCommand { id: "new_map", title: "New Map", category: "File", shortcut: "Ctrl+N", icon: "📄" },
        PaletteCommand { id: "map_wizard", title: "New Valid UT99 Map Generator Wizard (Official DNA Cloner)", category: "File", shortcut: "Ctrl+Shift+N", icon: "🧙" },
        PaletteCommand { id: "open_map", title: "Open .unr Map", category: "File", shortcut: "Ctrl+O", icon: "📂" },
        PaletteCommand { id: "save_map", title: "Save .unr Map", category: "File", shortcut: "Ctrl+S", icon: "💾" },
        PaletteCommand { id: "save_map_as", title: "Save .unr Map As...", category: "File", shortcut: "Ctrl+Shift+S", icon: "💾" },
        PaletteCommand { id: "save_preferences", title: "Save User Preferences to JSON", category: "System", shortcut: "", icon: "💾" },
        PaletteCommand { id: "load_preferences", title: "Reload User Preferences from JSON", category: "System", shortcut: "", icon: "🔄" },
        PaletteCommand { id: "open_settings", title: "Preferences & Settings (Language, Theme, Paths, Snapping)", category: "System", shortcut: "Ctrl+,", icon: "⚙" },
        PaletteCommand { id: "open_welcome_wizard", title: "Welcome & Onboarding Wizard (First-Run Setup)", category: "Help", shortcut: "", icon: "✨" },
        PaletteCommand { id: "play_map", title: "Play Map in UT99 Engine", category: "Game", shortcut: "F8", icon: "🎮" },

        // Window & Workspace
        PaletteCommand { id: "close_all_windows", title: "Close All Floating Windows", category: "Window", shortcut: "Ctrl+Shift+W", icon: "✕" },
        PaletteCommand { id: "workspace_default", title: "Workspace Preset: Default", category: "Window", shortcut: "", icon: "⊞" },
        PaletteCommand { id: "workspace_level_design", title: "Workspace Preset: Level Design", category: "Window", shortcut: "", icon: "⊞" },
        PaletteCommand { id: "workspace_lighting", title: "Workspace Preset: Lighting & Atmosphere", category: "Window", shortcut: "", icon: "☼" },
        PaletteCommand { id: "workspace_bots", title: "Workspace Preset: Bot & Navigation", category: "Window", shortcut: "", icon: "◈" },

        // Viewports & Navigation
        PaletteCommand { id: "toggle_fullscreen", title: "Toggle Fullscreen Mode", category: "Viewport", shortcut: "F11 / Alt+Enter", icon: "⛶" },
        PaletteCommand { id: "toggle_quad_view", title: "Toggle 2x2 Multi-Viewport / Single", category: "Viewport", shortcut: "Space", icon: "⊞" },
        PaletteCommand { id: "toggle_console", title: "Toggle Bottom Console Drawer", category: "Viewport", shortcut: "Ctrl+~ / Ctrl+Shift+C", icon: "📟" },
        PaletteCommand { id: "view_persp", title: "Active View: 3D Perspective", category: "Viewport", shortcut: "F1", icon: "🎥" },
        PaletteCommand { id: "view_top", title: "Active View: Top (XY)", category: "Viewport", shortcut: "F2", icon: "📐" },
        PaletteCommand { id: "view_front", title: "Active View: Front (XZ)", category: "Viewport", shortcut: "F3", icon: "📐" },
        PaletteCommand { id: "view_side", title: "Active View: Side (YZ)", category: "Viewport", shortcut: "Shift+F4", icon: "📐" },
        PaletteCommand { id: "focus_selection", title: "Focus Camera on Selection", category: "Viewport", shortcut: "F", icon: "🎯" },
        PaletteCommand { id: "toggle_fog", title: "Toggle Real-Time Distance Fog", category: "Viewport", shortcut: "F10", icon: "🌫" },
        PaletteCommand { id: "toggle_luxmeter", title: "Toggle Luxmeter Lighting Heatmap", category: "Viewport", shortcut: "Shift+F6", icon: "💡" },
        PaletteCommand { id: "toggle_nav_coverage", title: "Toggle Bot Nav Coverage Heatmap", category: "Viewport", shortcut: "F9", icon: "🤖" },
        PaletteCommand { id: "shading_wire", title: "Viewport Shading: Wireframe Only", category: "Viewport", shortcut: "Alt+1", icon: "📐" },
        PaletteCommand { id: "shading_zone", title: "Viewport Shading: Zone/Portal View", category: "Viewport", shortcut: "Alt+2", icon: "🚪" },
        PaletteCommand { id: "shading_unlit", title: "Viewport Shading: Unlit (Full-Bright Diffuse)", category: "Viewport", shortcut: "Alt+3", icon: "☀" },
        PaletteCommand { id: "shading_lit", title: "Viewport Shading: Lit (Full Lighting)", category: "Viewport", shortcut: "Alt+4", icon: "💡" },
        PaletteCommand { id: "shading_depth", title: "Viewport Shading: Depth Complexity Heatmap", category: "Viewport", shortcut: "Alt+5", icon: "📊" },
        PaletteCommand { id: "shading_tex_usage", title: "Viewport Shading: Texture Usage Color-Coding", category: "Viewport", shortcut: "Alt+6", icon: "🎨" },
        PaletteCommand { id: "shading_bsp_cuts", title: "Viewport Shading: BSP Cuts Diagnostic", category: "Viewport", shortcut: "Alt+7", icon: "✂" },
        PaletteCommand { id: "cycle_cam_speed", title: "Cycle Camera Speed Preset (1-8)", category: "Viewport", shortcut: "1-8", icon: "🚀" },
        PaletteCommand { id: "toggle_minimap", title: "Toggle Map Navigation Radar", category: "Viewport", shortcut: "M", icon: "🗺" },
        PaletteCommand { id: "toggle_radii", title: "Toggle Radii View (Collision & Attenuation)", category: "Viewport", shortcut: "", icon: "⭕" },
        PaletteCommand { id: "toggle_doctor_beacons", title: "Toggle Level Doctor 3D Beacons", category: "Viewport", shortcut: "", icon: "📍" },
        PaletteCommand { id: "toggle_frustum_culling", title: "Toggle View Frustum Culling Engine", category: "Viewport", shortcut: "", icon: "👁" },

        // Texture Alignment & Manipulation
        PaletteCommand { id: "tex_pan_up", title: "Pan Selected Texture Up", category: "Texture", shortcut: "Shift+Up", icon: "🖼" },
        PaletteCommand { id: "tex_pan_down", title: "Pan Selected Texture Down", category: "Texture", shortcut: "Shift+Down", icon: "🖼" },
        PaletteCommand { id: "tex_pan_left", title: "Pan Selected Texture Left", category: "Texture", shortcut: "Shift+Left", icon: "🖼" },
        PaletteCommand { id: "tex_pan_right", title: "Pan Selected Texture Right", category: "Texture", shortcut: "Shift+Right", icon: "🖼" },
        PaletteCommand { id: "tex_rotate_45", title: "Rotate Selected Texture 45°", category: "Texture", shortcut: "Shift+R / Ctrl+Shift+R", icon: "🔄" },

        // Build & Compilers
        PaletteCommand { id: "build_geom", title: "Build Geometry (BSP Tree Rebuild)", category: "Build", shortcut: "Ctrl+Shift+G", icon: "🔨" },
        PaletteCommand { id: "build_light", title: "Build Lighting (Raytracing Radiosity)", category: "Build", shortcut: "Ctrl+Shift+L", icon: "💡" },
        PaletteCommand { id: "build_paths", title: "Build Bot AI Navigation Paths", category: "Build", shortcut: "Ctrl+Shift+P", icon: "🔷" },
        PaletteCommand { id: "build_all", title: "Build All (Geometry + Lighting + Paths + Decals)", category: "Build", shortcut: "Ctrl+F8", icon: "⚡" },
        PaletteCommand { id: "win_build_options", title: "Open Build Options & Compilers", category: "Build", shortcut: "Ctrl+Shift+B", icon: "⚙" },

        // Edit & Transform
        PaletteCommand { id: "undo", title: "Undo Last Action", category: "Edit", shortcut: "Ctrl+Z", icon: "↩" },
        PaletteCommand { id: "redo", title: "Redo Action", category: "Edit", shortcut: "Ctrl+Y", icon: "↪" },
        PaletteCommand { id: "cut", title: "Cut Selection to Clipboard", category: "Edit", shortcut: "Ctrl+X", icon: "✂" },
        PaletteCommand { id: "copy", title: "Copy Selection to Clipboard", category: "Edit", shortcut: "Ctrl+C", icon: "📄" },
        PaletteCommand { id: "paste", title: "Paste from Clipboard", category: "Edit", shortcut: "Ctrl+V", icon: "📋" },
        PaletteCommand { id: "duplicate", title: "Duplicate Selection", category: "Edit", shortcut: "Ctrl+D", icon: "📑" },
        PaletteCommand { id: "delete", title: "Delete Selection", category: "Edit", shortcut: "Del", icon: "🗑" },
        PaletteCommand { id: "select_all", title: "Select All Objects", category: "Edit", shortcut: "Ctrl+A", icon: "☑" },
        PaletteCommand { id: "select_all_brushes", title: "Select All Brushes", category: "Edit", shortcut: "", icon: "📦" },
        PaletteCommand { id: "select_all_actors", title: "Select All Actors", category: "Edit", shortcut: "", icon: "🎭" },
        PaletteCommand { id: "deselect_all", title: "Deselect All", category: "Edit", shortcut: "Esc", icon: "☐" },
        PaletteCommand { id: "invert_selection", title: "Invert Selection", category: "Edit", shortcut: "Ctrl+I", icon: "🔄" },
        PaletteCommand { id: "mirror_x", title: "Mirror Selection across X Axis", category: "Edit", shortcut: "", icon: "🪞" },
        PaletteCommand { id: "mirror_y", title: "Mirror Selection across Y Axis", category: "Edit", shortcut: "", icon: "🪞" },
        PaletteCommand { id: "mirror_z", title: "Mirror Selection across Z Axis", category: "Edit", shortcut: "", icon: "🪞" },
        PaletteCommand { id: "snap_floor", title: "Snap Selection to Floor", category: "Edit", shortcut: "End", icon: "⬇" },
        PaletteCommand { id: "snap_floor_aligned", title: "Snap Selection to Floor Aligned to Normal", category: "Edit", shortcut: "Shift+End", icon: "📐" },
        PaletteCommand { id: "snap_grid_objects", title: "Snap Selected Objects to Grid", category: "Edit", shortcut: "Ctrl+Alt+G", icon: "📐" },
        PaletteCommand { id: "snap_grid", title: "Snap Vertices to Grid", category: "Edit", shortcut: "Ctrl+Alt+S", icon: "▦" },
        PaletteCommand { id: "weld_vertices", title: "Weld Coincident Vertices (16u)", category: "Edit", shortcut: "Ctrl+Alt+W", icon: "🧲" },
        PaletteCommand { id: "fit_texture", title: "Fit Active Texture to Selection", category: "Edit", shortcut: "Shift+F / Ctrl+Shift+F", icon: "🖼" },
        PaletteCommand { id: "find_actors", title: "Find Actors in Level", category: "Edit", shortcut: "Ctrl+F", icon: "🔍" },
        PaletteCommand { id: "replace_textures", title: "Find and Replace Textures (Dialog)", category: "Edit", shortcut: "Ctrl+R", icon: "🔁" },
        PaletteCommand { id: "scale_brush", title: "Scale Brush Arbitrarily (Dialog)", category: "Edit", shortcut: "", icon: "📐" },
        PaletteCommand { id: "shear_brush", title: "Shear Brush Arbitrarily (Dialog)", category: "Edit", shortcut: "", icon: "📐" },
        PaletteCommand { id: "align_actors", title: "Align & Distribute Actors (Dialog)", category: "Edit", shortcut: "", icon: "📐" },
        PaletteCommand { id: "toggle_gizmo_space", title: "Toggle Gizmo Coordinate Space (World / Local)", category: "Edit", shortcut: "Ctrl+L", icon: "🌐" },
        PaletteCommand { id: "gizmo_translate", title: "Gizmo Mode: Translate (Move)", category: "Edit", shortcut: "W", icon: "✛" },
        PaletteCommand { id: "gizmo_rotate", title: "Gizmo Mode: Rotate", category: "Edit", shortcut: "E", icon: "🔄" },
        PaletteCommand { id: "gizmo_scale", title: "Gizmo Mode: Scale", category: "Edit", shortcut: "R", icon: "📐" },
        PaletteCommand { id: "open_action_wizards", title: "Open Map Creation Hub & Action Wizards", category: "Tools", shortcut: "Ctrl+Shift+A", icon: "✨" },
        PaletteCommand { id: "open_visual_presets", title: "Open Visual & Lighting Presets Dialog", category: "Tools", shortcut: "", icon: "🎨" },

        // Surface Selection (UnrealEd 2.0 Parity)
        PaletteCommand { id: "select_all_surfaces", title: "Select All Surfaces", category: "Select", shortcut: "Shift+S", icon: "🎨" },
        PaletteCommand { id: "deselect_all_surfaces", title: "Deselect All Surfaces", category: "Select", shortcut: "Shift+N", icon: "🎨" },
        PaletteCommand { id: "select_matching_texture", title: "Select Matching Texture", category: "Select", shortcut: "Shift+T", icon: "🎨" },
        PaletteCommand { id: "select_coplanar_adjacent", title: "Select Adjacent Coplanar Surfaces", category: "Select", shortcut: "Shift+C", icon: "🎨" },
        PaletteCommand { id: "select_coplanar_all", title: "Select All Coplanar Surfaces", category: "Select", shortcut: "", icon: "🎨" },
        PaletteCommand { id: "select_adjacent_walls", title: "Select Adjacent Walls", category: "Select", shortcut: "Shift+W", icon: "🎨" },
        PaletteCommand { id: "select_adjacent_slants", title: "Select Adjacent Slants / Floors", category: "Select", shortcut: "Shift+Q", icon: "🎨" },
        PaletteCommand { id: "invert_surface_selection", title: "Invert Surface Selection", category: "Select", shortcut: "Ctrl+Shift+I", icon: "🎨" },
        PaletteCommand { id: "select_surfaces_on_brush", title: "Select All Surfaces on Brush", category: "Select", shortcut: "Shift+B", icon: "🎨" },
        PaletteCommand { id: "align_continuous_strip", title: "Align Continuous Wall Strip / Wrap", category: "Surface", shortcut: "", icon: "🔄" },
        PaletteCommand { id: "align_wall_auto", title: "Align Texture along Wall Direction", category: "Surface", shortcut: "", icon: "🧱" },
        PaletteCommand { id: "align_fit_2x2", title: "Fit Texture 2x2 Repetitions", category: "Surface", shortcut: "", icon: "📐" },
        PaletteCommand { id: "align_fit_4x4", title: "Fit Texture 4x4 Repetitions", category: "Surface", shortcut: "", icon: "📐" },
        PaletteCommand { id: "snap_brush_pivot_vertex", title: "Snap Brush Pivot to Nearest Vertex", category: "Brush", shortcut: "Ctrl+End", icon: "📍" },
        PaletteCommand { id: "snap_brush_pivot_center", title: "Center Brush Pivot to Bounding Box", category: "Brush", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "extract_surface_to_builder", title: "Extract Selected Surface(s) to Builder Brush", category: "Brush", shortcut: "", icon: "✂" },
        PaletteCommand { id: "brush_make_hollow_16", title: "Make Builder Brush Hollow (16 uu walls)", category: "Brush", shortcut: "", icon: "🏠" },
        PaletteCommand { id: "brush_make_hollow_32", title: "Make Builder Brush Hollow (32 uu walls)", category: "Brush", shortcut: "", icon: "🏠" },
        PaletteCommand { id: "reset_surface_pan", title: "Reset Surface Pan to Origin (0,0)", category: "Surface", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "snap_surface_uv_16", title: "Snap Surface UV to 16px Grid", category: "Surface", shortcut: "", icon: "▦" },
        PaletteCommand { id: "snap_surface_uv_64", title: "Snap Surface UV to 64px Grid", category: "Surface", shortcut: "", icon: "▦" },

        // Actor Selection by Tag & Event
        PaletteCommand { id: "select_actors_matching_tag", title: "Select Actors Matching Tag", category: "Select", shortcut: "", icon: "👥" },
        PaletteCommand { id: "select_actors_matching_event", title: "Select Actors Matching Event", category: "Select", shortcut: "", icon: "👥" },
        PaletteCommand { id: "select_actors_targeted_by_event", title: "Select Actors Targeted by Event (Event -> Tag)", category: "Select", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "select_actors_triggering_tag", title: "Select Actors Triggering Tag (Event <- Tag)", category: "Select", shortcut: "", icon: "🔍" },

        // Surface Flag Presets
        PaletteCommand { id: "preset_flags_water", title: "Surface Preset: Water (Translucent, Wavy, NotSolid)", category: "Surface", shortcut: "", icon: "💧" },
        PaletteCommand { id: "preset_flags_lava", title: "Surface Preset: Lava / Slime (Wavy, Unlit)", category: "Surface", shortcut: "", icon: "🔥" },
        PaletteCommand { id: "preset_flags_glass", title: "Surface Preset: Glass / Window (Translucent, TwoSided)", category: "Surface", shortcut: "", icon: "🪟" },
        PaletteCommand { id: "preset_flags_grate", title: "Surface Preset: Masked Grate (Masked, TwoSided)", category: "Surface", shortcut: "", icon: "⛓" },
        PaletteCommand { id: "preset_flags_sky", title: "Surface Preset: Sky Backdrop (FakeBackdrop, Unlit)", category: "Surface", shortcut: "", icon: "🌌" },
        PaletteCommand { id: "preset_flags_invisible", title: "Surface Preset: Invisible Barrier", category: "Surface", shortcut: "", icon: "👻" },
        PaletteCommand { id: "preset_flags_unlit", title: "Surface Preset: Fullbright / Unlit", category: "Surface", shortcut: "", icon: "💡" },
        PaletteCommand { id: "preset_flags_clear", title: "Surface Preset: Clear All Flags (Default Solid)", category: "Surface", shortcut: "", icon: "❌" },

        // Vertex Edit & Mover Commands
        PaletteCommand { id: "vertex_split_edge", title: "Vertex Edit: Split Selected Edge", category: "Brush", shortcut: "Ctrl+Alt+E", icon: "✂" },
        PaletteCommand { id: "vertex_collapse_edge", title: "Vertex Edit: Collapse Edge / Merge Vertices", category: "Brush", shortcut: "Ctrl+Alt+M", icon: "💥" },
        PaletteCommand { id: "mover_next_key", title: "Mover: Switch to Next Keyframe (0..7)", category: "Actor", shortcut: "", icon: "🔑" },
        PaletteCommand { id: "mover_prev_key", title: "Mover: Switch to Previous Keyframe (0..7)", category: "Actor", shortcut: "", icon: "🔑" },

        // Grid & Camera & OBJ Commands
        PaletteCommand { id: "grid_step_up", title: "Increase Grid Step (Double)", category: "Edit", shortcut: "]", icon: "📐" },
        PaletteCommand { id: "grid_step_down", title: "Decrease Grid Step (Halve)", category: "Edit", shortcut: "[", icon: "📐" },
        PaletteCommand { id: "fov_cycle", title: "Cycle Camera Field of View (FOV)", category: "View", shortcut: "", icon: "📷" },
        PaletteCommand { id: "brush_export_obj", title: "Export Builder Brush to OBJ", category: "Brush", shortcut: "", icon: "📤" },

        // Brush Solidity & Order Commands
        PaletteCommand { id: "brush_solidity_solid", title: "Set Brush Solidity: Solid", category: "Brush", shortcut: "", icon: "🧱" },
        PaletteCommand { id: "brush_solidity_semisolid", title: "Set Brush Solidity: Semi-Solid", category: "Brush", shortcut: "", icon: "🌿" },
        PaletteCommand { id: "brush_solidity_nonsolid", title: "Set Brush Solidity: Non-Solid", category: "Brush", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "brush_order_first", title: "Brush Order: Send to First (Earliest)", category: "Brush", shortcut: "", icon: "⏮" },
        PaletteCommand { id: "brush_order_last", title: "Brush Order: Send to Last (Latest)", category: "Brush", shortcut: "", icon: "⏭" },
        PaletteCommand { id: "brush_order_up", title: "Brush Order: Move Up", category: "Brush", shortcut: "", icon: "▲" },
        PaletteCommand { id: "brush_order_down", title: "Brush Order: Move Down", category: "Brush", shortcut: "", icon: "▼" },

        // Brush Invert & Advanced Selection Commands
        PaletteCommand { id: "brush_invert_normals", title: "Brush: Invert Normals & Polys Winding", category: "Brush", shortcut: "Ctrl+Alt+I", icon: "🔄" },
        PaletteCommand { id: "surface_invert_normals", title: "Surface: Invert Normals", category: "Surface", shortcut: "", icon: "🔄" },
        PaletteCommand { id: "select_brushes_additive", title: "Select Additive Brushes", category: "Select", shortcut: "", icon: "➕" },
        PaletteCommand { id: "select_brushes_subtractive", title: "Select Subtractive Brushes", category: "Select", shortcut: "", icon: "➖" },
        PaletteCommand { id: "select_brushes_semisolid", title: "Select Semi-Solid Brushes", category: "Select", shortcut: "", icon: "🌿" },
        PaletteCommand { id: "select_brushes_nonsolid", title: "Select Non-Solid Brushes", category: "Select", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "select_actors_lights", title: "Select All Lights", category: "Select", shortcut: "", icon: "💡" },
        PaletteCommand { id: "select_actors_pathnodes", title: "Select All Navigation Points / PathNodes", category: "Select", shortcut: "", icon: "🚩" },

        // Brush Transform Reset & Baking
        PaletteCommand { id: "brush_transform_permanently", title: "Brush: Transform Permanently (Bake Rotation & Scale)", category: "Brush", shortcut: "", icon: "⚙" },
        PaletteCommand { id: "brush_reset_rotation", title: "Brush: Reset Rotation (0°)", category: "Brush", shortcut: "", icon: "↺" },
        PaletteCommand { id: "brush_reset_scaling", title: "Brush: Reset Scaling (1.0)", category: "Brush", shortcut: "", icon: "↔" },
        PaletteCommand { id: "brush_reset_all", title: "Brush: Reset All (Rotation + Scaling)", category: "Brush", shortcut: "", icon: "🔄" },

        // Inspectors & Windows
        PaletteCommand { id: "win_texture_browser", title: "Open Texture Browser", category: "Window", shortcut: "F4", icon: "🖼" },
        PaletteCommand { id: "win_surface_props", title: "Open Surface Properties", category: "Window", shortcut: "F5", icon: "🎨" },
        PaletteCommand { id: "win_actor_props", title: "Open Actor Properties", category: "Window", shortcut: "Enter", icon: "⚙" },
        PaletteCommand { id: "win_group_browser", title: "Open Groups Browser", category: "Window", shortcut: "", icon: "👥" },
        PaletteCommand { id: "win_level_doctor", title: "Open Level Doctor (Diagnostics)", category: "Window", shortcut: "", icon: "🩺" },
        PaletteCommand { id: "win_budget_profiler", title: "Open Engine Budget & Profiler", category: "Window", shortcut: "F7", icon: "📊" },
        PaletteCommand { id: "win_actor_browser", title: "Open Actor Class Browser", category: "Window", shortcut: "", icon: "⚙" },
        PaletteCommand { id: "win_sound_browser", title: "Open Sound Browser (.uax)", category: "Window", shortcut: "", icon: "🔊" },
        PaletteCommand { id: "win_music_browser", title: "Open Music Browser (.umx)", category: "Window", shortcut: "", icon: "🎵" },
        PaletteCommand { id: "win_bot_sim", title: "Open Bot AI Navigation Simulator", category: "Window", shortcut: "", icon: "🤖" },
        PaletteCommand { id: "win_bookmarks", title: "Open Camera Bookmarks Manager", category: "Window", shortcut: "Ctrl+B", icon: "🔖" },
        PaletteCommand { id: "win_acoustics", title: "Open Zone Acoustics & Distance Fog", category: "Window", shortcut: "F11", icon: "🌐" },
        PaletteCommand { id: "win_gameplay_balance", title: "Open Tournament Gameplay Balance", category: "Window", shortcut: "F12", icon: "⚖" },
        PaletteCommand { id: "win_zone_seal", title: "Open Zone Boundary Leak Detector", category: "Window", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "win_dispatcher", title: "Open Dispatcher Trigger Timeline", category: "Window", shortcut: "", icon: "⏱" },
        PaletteCommand { id: "win_ctf_symmetry", title: "Open Symmetrical CTF Base Generator", category: "Window", shortcut: "", icon: "🏰" },
        PaletteCommand { id: "win_prefab", title: "Open Prefab & Template Library", category: "Window", shortcut: "", icon: "📦" },
        PaletteCommand { id: "win_shape_editor", title: "Open 2D Shape Extruder / Lathe", category: "Window", shortcut: "", icon: "✏" },
        PaletteCommand { id: "win_packaging", title: "Open Map Packaging & Manifest", category: "Window", shortcut: "", icon: "📦" },
        PaletteCommand { id: "win_scale_brush", title: "Open Scale Brush Dialog", category: "Window", shortcut: "", icon: "📐" },
        PaletteCommand { id: "win_find_actors", title: "Open Find Actors Dialog", category: "Window", shortcut: "Ctrl+F", icon: "🔍" },
        PaletteCommand { id: "win_shear_brush", title: "Open Shear Brush Dialog", category: "Window", shortcut: "", icon: "📐" },
        PaletteCommand { id: "win_align_actors", title: "Open Align & Distribute Actors Dialog", category: "Window", shortcut: "", icon: "📐" },
        PaletteCommand { id: "win_map_statistics", title: "Open Map Statistics & Metrics", category: "Window", shortcut: "", icon: "📊" },
        PaletteCommand { id: "level_properties", title: "Level Properties (Title, Author, Music, Rules)", category: "Level", shortcut: "F6", icon: "📋" },
        PaletteCommand { id: "scale_entire_map", title: "Scale Entire Map (Brushes, Movers, Actors, Lights)", category: "Tools", shortcut: "", icon: "🌐" },
        PaletteCommand { id: "tex_export_active", title: "Export Active Texture to PNG...", category: "Texture", shortcut: "", icon: "💾" },
        PaletteCommand { id: "tex_export_package", title: "Export Current Package Textures...", category: "Texture", shortcut: "", icon: "📦" },
        PaletteCommand { id: "win_mesh_browser", title: "Open Mesh & Static Mesh Browser", category: "Window", shortcut: "", icon: "🗿" },
        PaletteCommand { id: "mesh_place_active", title: "Place Selected Mesh in Level (at Builder Pivot)", category: "Mesh", shortcut: "", icon: "🗿" },
        PaletteCommand { id: "win_procedural_texture", title: "Open FireEngine Procedural Texture Studio", category: "Window", shortcut: "", icon: "🔥" },
        PaletteCommand { id: "procedural_apply_active", title: "Apply Procedural Texture as Active Material", category: "Texture", shortcut: "", icon: "🔥" },
        PaletteCommand { id: "win_package_deps", title: "Open Package Dependencies & Distribution Manifest", category: "Window", shortcut: "", icon: "📦" },
        PaletteCommand { id: "win_decal_studio", title: "Decal Painting & Projector Studio", category: "Window", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "bake_decals", title: "Bake Projector Decals onto Level Geometry", category: "Build", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "win_uc_viewer", title: "UnrealScript Studio & DefaultProperties", category: "Window", shortcut: "", icon: "📜" },
        PaletteCommand { id: "win_skybox_studio", title: "SkyBox 3D Parallax & Multi-Zone Studio", category: "Window", shortcut: "", icon: "🌌" },
        PaletteCommand { id: "skybox_place_anchor", title: "Place SkyZoneInfo Anchor at Pivot", category: "Tools", shortcut: "", icon: "🌌" },
        PaletteCommand { id: "skybox_mark_surfaces", title: "Mark Selected Surfaces as Sky Backdrop (PF_FAKE_BACKDROP)", category: "Texture", shortcut: "", icon: "🌌" },
        PaletteCommand { id: "skybox_focus", title: "Focus Camera on SkyZone", category: "View", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "win_acoustic_studio", title: "3D Acoustic & Ambient Sound Studio", category: "Window", shortcut: "", icon: "🔊" },
        PaletteCommand { id: "acoustic_place_sound", title: "Place AmbientSound at Pivot", category: "Audio", shortcut: "", icon: "🔊" },
        PaletteCommand { id: "win_warp_zone_studio", title: "WarpZone & Non-Euclidean Portal Studio", category: "Window", shortcut: "", icon: "🌀" },
        PaletteCommand { id: "warp_create_pair", title: "Create Coordinated WarpZone Pair at Pivot", category: "Tools", shortcut: "", icon: "🌀" },
        PaletteCommand { id: "warp_teleport_camera", title: "Teleport Camera Through Active WarpZone", category: "View", shortcut: "", icon: "👁" },
        PaletteCommand { id: "win_lighting_studio", title: "Radiosity & Direct Light Baking Studio", category: "Window", shortcut: "", icon: "💡" },
        PaletteCommand { id: "lighting_bake_differential", title: "Bake Selected Lights Only (Differential Fast Bake)", category: "Build", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "lighting_apply_midnight", title: "Apply Facing Worlds Midnight Light Preset", category: "Lighting", shortcut: "", icon: "🌌" },
        PaletteCommand { id: "win_zone_fog", title: "Zone Fog & Volumetric Attenuation Studio", category: "Window", shortcut: "", icon: "🌫" },
        PaletteCommand { id: "fog_place_zone", title: "Place New Fog ZoneInfo at Pivot", category: "Tools", shortcut: "", icon: "🌫" },
        PaletteCommand { id: "fog_toggle_shells", title: "Toggle 3D Fog Attenuation Shells", category: "View", shortcut: "", icon: "🌐" },
        PaletteCommand { id: "win_dispatcher_studio", title: "Dispatcher Sequence & Trigger Timeline Studio", category: "Window", shortcut: "", icon: "⏱" },
        PaletteCommand { id: "dispatcher_place_pivot", title: "Place New Scripted Dispatcher at Pivot", category: "Tools", shortcut: "", icon: "⏱" },
        PaletteCommand { id: "dispatcher_play_toggle", title: "Play / Pause Dispatcher Timeline Simulation", category: "Tools", shortcut: "", icon: "▶" },
        PaletteCommand { id: "win_skeletal_studio", title: "Skeletal Rigging & Character Animation Studio", category: "Window", shortcut: "", icon: "🦴" },
        PaletteCommand { id: "win_skin_studio", title: "UT99 Skin & Character Authoring Studio", category: "Window", shortcut: "", icon: "🎨" },
        PaletteCommand { id: "win_package_extractor", title: "Package Asset Extractor & .uz Decompressor", category: "Window", shortcut: "", icon: "📦" },
        PaletteCommand { id: "win_uc_wizard", title: "UnrealScript Class Wizard & #exec Studio", category: "Window", shortcut: "", icon: "📜" },
        PaletteCommand { id: "skeletal_place_pivot", title: "Place Animated Humanoid at Pivot", category: "Tools", shortcut: "", icon: "🦴" },
        PaletteCommand { id: "skeletal_reset_pose", title: "Reset Skeleton to Bind Pose (T-Pose)", category: "Tools", shortcut: "", icon: "🔄" },
        PaletteCommand { id: "win_particle_studio", title: "Particle FX & Volumetric Emitter Studio", category: "Window", shortcut: "", icon: "✨" },
        PaletteCommand { id: "particle_place_pivot", title: "Place Particle Emitter at Pivot", category: "Tools", shortcut: "", icon: "📍" },
        PaletteCommand { id: "particle_sim_toggle", title: "Play / Pause Particle Simulation", category: "Tools", shortcut: "", icon: "▶" },
        PaletteCommand { id: "win_bot_nav_studio", title: "Bot AI Navigation & Reachability Studio", category: "Window", shortcut: "", icon: "🤖" },
        PaletteCommand { id: "nav_build_paths", title: "Build Level Navigation Paths", category: "Tools", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "nav_focus_node", title: "Focus Camera on Selected NavNode", category: "Tools", shortcut: "", icon: "📍" },
        PaletteCommand { id: "win_arch_studio", title: "Procedural Architectural Prefab Studio", category: "Window", shortcut: "", icon: "🏛" },
        PaletteCommand { id: "arch_to_builder", title: "Synthesize Architectural Prefab to Builder Brush", category: "Tools", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "arch_bake_add", title: "Bake Architectural Prefab to World (Additive)", category: "Tools", shortcut: "", icon: "➕" },
        PaletteCommand { id: "arch_bake_sub", title: "Bake Architectural Prefab to World (Subtractive)", category: "Tools", shortcut: "", icon: "➖" },
        PaletteCommand { id: "win_terrain_studio", title: "Terrain & Heightmap Sculptor Studio", category: "Window", shortcut: "", icon: "🏔" },
        PaletteCommand { id: "terrain_to_builder", title: "Synthesize Terrain Mesh to Builder Brush", category: "Tools", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "terrain_bake_add", title: "Bake Terrain Mesh to World (Additive)", category: "Tools", shortcut: "", icon: "➕" },
        PaletteCommand { id: "terrain_bake_sub", title: "Bake Terrain Mesh to World (Subtractive)", category: "Tools", shortcut: "", icon: "➖" },
        PaletteCommand { id: "win_lod_collision", title: "Static Mesh LOD & Convex Collision Studio", category: "Window", shortcut: "", icon: "📐" },
        PaletteCommand { id: "lod_bake_hull", title: "Bake Convex Collision Hull for Selection", category: "Tools", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "lod_apply_level", title: "Apply Selected LOD Level to Target Brush", category: "Tools", shortcut: "", icon: "📉" },
        PaletteCommand { id: "win_cinematic_studio", title: "Cinematic Matinee Camera Director Studio", category: "Window", shortcut: "", icon: "🎬" },
        PaletteCommand { id: "cinematic_capture_cam", title: "Capture Viewport Camera to Cinematic Spline", category: "Tools", shortcut: "", icon: "📸" },
        PaletteCommand { id: "cinematic_play_toggle", title: "Toggle Cinematic Camera Playback", category: "Tools", shortcut: "", icon: "▶" },
        PaletteCommand { id: "cinematic_export_actors", title: "Export Cinematic Spline to InterpolationPoints", category: "Tools", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "win_light_effects_studio", title: "Dynamic Light FX & Animating Spotlight Studio", category: "Window", shortcut: "", icon: "💡" },
        PaletteCommand { id: "light_place_animated", title: "Place Animated Dynamic Light at Pivot", category: "Lighting", shortcut: "", icon: "📍" },
        PaletteCommand { id: "light_apply_preset", title: "Apply Dynamic Light Preset to Selected", category: "Lighting", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "win_zone_occlusion_studio", title: "Zone Portals, Antiportals & PVS Occlusion Studio", category: "Window", shortcut: "", icon: "🌊" },
        PaletteCommand { id: "zone_recompute_pvs", title: "Recompute PVS & Watertightness Audit", category: "Tools", shortcut: "", icon: "🔄" },
        PaletteCommand { id: "zone_create_portal_sheet", title: "Create Portal Sheet at Pivot", category: "Tools", shortcut: "", icon: "🚪" },
        PaletteCommand { id: "zone_add_antiportal", title: "Add AntiPortal Occluder Volume at Pivot", category: "Tools", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "win_music_studio", title: "Music Event & UMX Track Sequencing Studio", category: "Window", shortcut: "", icon: "🎵" },
        PaletteCommand { id: "music_place_event", title: "Place MusicEvent at Pivot", category: "Audio", shortcut: "", icon: "📍" },
        PaletteCommand { id: "music_audit_level", title: "Run Level Music Configuration Audit", category: "Audio", shortcut: "", icon: "🔍" },
        PaletteCommand { id: "win_corona_studio", title: "Sun, Corona & Lens Flare Studio", category: "Window", shortcut: "", icon: "☀" },
        PaletteCommand { id: "corona_place_sun", title: "Place Celestial Corona / Sun Light at Pivot", category: "Lighting", shortcut: "", icon: "📍" },
        PaletteCommand { id: "corona_apply_preset", title: "Apply Corona Preset to Selected Lights", category: "Lighting", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "win_fluid_studio", title: "Fluid Physics & Hazard Volume Studio", category: "Window", shortcut: "", icon: "🏊" },
        PaletteCommand { id: "fluid_place_zone", title: "Place Fluid Physics Zone at Pivot", category: "Physics", shortcut: "", icon: "💧" },
        PaletteCommand { id: "fluid_apply_preset", title: "Apply Fluid Physics Preset to Selected", category: "Physics", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "win_ballistics_studio", title: "Weapon Ballistics & Combat Sightline Studio", category: "Window", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "ballistics_place_probe", title: "Place Ballistic Combat Probe at Pivot", category: "Weapons", shortcut: "", icon: "📍" },
        PaletteCommand { id: "ballistics_recalculate", title: "Recalculate Active Weapon Ballistics", category: "Weapons", shortcut: "", icon: "🚀" },
        PaletteCommand { id: "win_teleporter_studio", title: "Teleporter Destination & Network Link Studio", category: "Window", shortcut: "", icon: "🌀" },
        PaletteCommand { id: "teleporter_pair_spawn", title: "Spawn Paired Teleporter Link at Pivot", category: "Gameplay", shortcut: "", icon: "🔗" },
        PaletteCommand { id: "win_translocator_studio", title: "Translocator Trajectory & Beacon Studio", category: "Window", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "translocator_place_probe", title: "Place Translocator Test Probe at Pivot", category: "Weapons", shortcut: "", icon: "📍" },
        PaletteCommand { id: "translocator_recalculate", title: "Recalculate Translocator Flight Simulation", category: "Weapons", shortcut: "", icon: "🚀" },
        PaletteCommand { id: "win_lift_pathing_studio", title: "LiftCenter, LiftExit & Elevator Pathing Studio", category: "Window", shortcut: "", icon: "🛗" },
        PaletteCommand { id: "lift_pathing_sync_setup", title: "Generate Synchronized Lift Pathing Set for Mover", category: "Navigation", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "lift_pathing_autofix", title: "Auto-Fix Elevator Tags & Floor Discrepancies", category: "Navigation", shortcut: "", icon: "🔧" },
        PaletteCommand { id: "win_water_surface_studio", title: "Water Surface & Wave Reflection Studio", category: "Window", shortcut: "", icon: "🌊" },
        PaletteCommand { id: "water_surface_generate_sheet", title: "Generate Watertight Water Sheet Brush", category: "Geometry", shortcut: "", icon: "💧" },
        PaletteCommand { id: "water_surface_reset_sim", title: "Reset Water Wave Tank Heightfield", category: "Physics", shortcut: "", icon: "🔄" },
        PaletteCommand { id: "win_mover_studio", title: "Mover Encroachment & Door Studio", category: "Window", shortcut: "", icon: "🚪" },
        PaletteCommand { id: "mover_wire_proximity_trigger", title: "Wire Proximity Trigger for Selected Door", category: "Movers", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "mover_audit_hazards", title: "Audit Mover Pinch-Points & Crushing Hazards", category: "Movers", shortcut: "", icon: "⚠" },
        PaletteCommand { id: "win_team_objective_studio", title: "Team Objective, CTF Route & Defense AI Studio", category: "Window", shortcut: "", icon: "🚩" },
        PaletteCommand { id: "team_obj_deploy_defense", title: "Deploy 3 Coordinated DefensePoints for Active Objective", category: "Gameplay", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "team_obj_refresh_analysis", title: "Refresh CTF Routes & Asymmetry Analysis", category: "Gameplay", shortcut: "", icon: "🔄" },
        PaletteCommand { id: "win_destructible_studio", title: "Destructible Architecture & Fracture Studio", category: "Window", shortcut: "", icon: "💥" },
        PaletteCommand { id: "destructible_shatter_test", title: "Trigger Destructible Shatter & Blast Physics Test", category: "Physics", shortcut: "", icon: "💥" },
        PaletteCommand { id: "destructible_generate_barrier", title: "Generate Watertight Destructible Barrier & Triggers", category: "Geometry", shortcut: "", icon: "🧱" },
        PaletteCommand { id: "win_kicker_studio", title: "Jump Pad, Aerial Kicker & Low-G Studio", category: "Window", shortcut: "", icon: "🚀" },
        PaletteCommand { id: "kicker_solve_trajectory", title: "Auto-Solve Kicker Velocity for Target Platform", category: "Physics", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "kicker_generate_setup", title: "Generate JumpPad / Kicker Setup & JumpSpot", category: "Geometry", shortcut: "", icon: "🚀" },
        PaletteCommand { id: "win_bsp_quality_studio", title: "BSP Tree Quality, Cut Minimizer & Sliver Purger", category: "Window", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "bsp_quality_run_audit", title: "Run In-Depth BSP Quality & Sliver Audit", category: "Geometry", shortcut: "", icon: "🔍" },
        PaletteCommand { id: "bsp_quality_purge_slivers", title: "Purge All Sliver Micro-Triangles", category: "Geometry", shortcut: "", icon: "🧹" },
        PaletteCommand { id: "bsp_quality_convert_semisolid", title: "Convert Top 5 Cut Offenders to Semi-Solid", category: "Geometry", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "win_shape_loft_studio", title: "2D Shape Loft, Curved Pipe & Molding Studio", category: "Window", shortcut: "", icon: "📐" },
        PaletteCommand { id: "loft_commit_builder", title: "Commit Loft to Red Active Builder Brush", category: "Geometry", shortcut: "", icon: "🔴" },
        PaletteCommand { id: "loft_commit_world_solid", title: "Commit Loft as Additive World Brush", category: "Geometry", shortcut: "", icon: "➕" },
        PaletteCommand { id: "win_stairway_studio", title: "Curved Stairway, Spiral Ramp & Smooth Collision Studio", category: "Window", shortcut: "", icon: "🪜" },
        PaletteCommand { id: "stair_commit_builder", title: "Commit Stairway to Red Active Builder Brush", category: "Geometry", shortcut: "", icon: "🔴" },
        PaletteCommand { id: "stair_commit_combo", title: "Commit Stairs + Smooth Collision Wedge to World", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_scatter_studio", title: "Surface Clutter, Foliage & Scatter Studio", category: "Window", shortcut: "", icon: "🌿" },
        PaletteCommand { id: "scatter_commit", title: "Commit Scattered Clutter Instances to World", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "scatter_reroll_seed", title: "Re-Roll Clutter Random Seed", category: "Geometry", shortcut: "", icon: "🎲" },
        PaletteCommand { id: "win_light_shaft_studio", title: "Volumetric Light Shaft & God Ray Studio", category: "Window", shortcut: "", icon: "✨" },
        PaletteCommand { id: "light_shaft_commit_builder", title: "Commit Light Shaft to Red Active Builder Brush", category: "Geometry", shortcut: "", icon: "🔴" },
        PaletteCommand { id: "light_shaft_commit_world", title: "Commit Light Shaft as Non-Solid World Brush", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_track_studio", title: "Spline Rail & Track Mover Studio", category: "Window", shortcut: "", icon: "🎢" },
        PaletteCommand { id: "track_commit_world", title: "Commit Track, Mover & Bot Waypoints to Level", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_surveillance_studio", title: "Security CCTV & Video Monitor Studio", category: "Window", shortcut: "", icon: "📹" },
        PaletteCommand { id: "surveillance_commit_world", title: "Commit Surveillance Camera, Screen & Monitor", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_laser_studio", title: "Security Laser Grid & Tripwire Studio", category: "Window", shortcut: "", icon: "🚨" },
        PaletteCommand { id: "laser_commit_world", title: "Commit Laser Grid, Diodes & Alarm Triggers", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_ventilator_studio", title: "Environmental Ventilator & Wind Tunnel Studio", category: "Window", shortcut: "", icon: "🌀" },
        PaletteCommand { id: "ventilator_commit_world", title: "Commit Ventilator, Air Zone & Hazard Trigger", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_conveyor_studio", title: "Industrial Conveyor Belt Studio", category: "Window", shortcut: "", icon: "⚙" },
        PaletteCommand { id: "conveyor_commit_world", title: "Commit Conveyor, Rollers, Zone & Bot Waypoints", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_turret_studio", title: "Automated Defense Turret & Sentry Studio", category: "Window", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "turret_commit_world", title: "Commit Turret, Mount, Shield & Bot Evasion", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_soundscape_studio", title: "Ambient Soundscape & Acoustic Reverb Studio", category: "Window", shortcut: "", icon: "🔊" },
        PaletteCommand { id: "soundscape_commit_world", title: "Commit Soundscape, Zone Brush & Ambient Actor", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_hazard_chamber_studio", title: "Hazard Chamber & Pressure Airlock Studio", category: "Window", shortcut: "", icon: "☣" },
        PaletteCommand { id: "hazard_chamber_commit_world", title: "Commit Chamber, Blast Doors, Zone & Trigger", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_pipe_conduit_studio", title: "Industrial Pipe Network & Steam Valve Studio", category: "Window", shortcut: "", icon: "🔧" },
        PaletteCommand { id: "pipe_conduit_commit_world", title: "Commit Pipe Run, Flanges, Valve & Hazard", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_hologram_studio", title: "Holographic Signage & Sci-Fi Billboard Studio", category: "Window", shortcut: "", icon: "✨" },
        PaletteCommand { id: "hologram_commit_world", title: "Commit Hologram Display, Emitter & Mover", category: "Geometry", shortcut: "", icon: "✨" },
        PaletteCommand { id: "win_weather_studio", title: "Dynamic Weather & Lightning Storm Studio", category: "Window", shortcut: "", icon: "🌧" },
        PaletteCommand { id: "weather_commit_world", title: "Commit Weather Volume, Lightning & Audio", category: "Geometry", shortcut: "", icon: "🌧" },
        PaletteCommand { id: "win_arena_rules_studio", title: "Arena Match Rules & Tournament Mutator Studio", category: "Window", shortcut: "", icon: "🏆" },
        PaletteCommand { id: "arena_rules_commit_world", title: "Commit Arena Rules & Mutators to Level", category: "Gameplay", shortcut: "", icon: "🏆" },
        PaletteCommand { id: "win_armor_stacking_studio", title: "Armor, Health & ShieldBelt Stacking Studio", category: "Window", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "armor_stacking_commit_world", title: "Commit Armor Station & Pickups to Level", category: "Gameplay", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "win_bot_jump_routes_studio", title: "Bot Tactical JumpSpot & Translocator Studio", category: "Window", shortcut: "", icon: "🦘" },
        PaletteCommand { id: "bot_jump_routes_commit_world", title: "Commit Jump Route, Plate & Spot to Level", category: "Gameplay", shortcut: "", icon: "🦘" },
        PaletteCommand { id: "win_weapon_circuit_studio", title: "Tournament Weapon & Ammo Circuit Studio", category: "Window", shortcut: "", icon: "🔫" },
        PaletteCommand { id: "weapon_circuit_commit_world", title: "Commit Weapon Station & Ammo to Level", category: "Gameplay", shortcut: "", icon: "🔫" },
        PaletteCommand { id: "win_powerup_control_studio", title: "Tournament Powerup Control & Relic Altar Studio", category: "Window", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "powerup_control_commit_world", title: "Commit Powerup Altar & Actor to Level", category: "Gameplay", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "win_spawn_distribution_studio", title: "Tournament Spawn Point Balancing Studio", category: "Window", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "spawn_distribution_commit_world", title: "Commit Spawn Pad & PlayerStart to Level", category: "Gameplay", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "win_domination_studio", title: "Domination Control Point Studio", category: "Window", shortcut: "", icon: "🚩" },
        PaletteCommand { id: "domination_commit_world", title: "Commit Domination Pedestal & ControlPoint to Level", category: "Gameplay", shortcut: "", icon: "🚩" },
        PaletteCommand { id: "win_assault_studio", title: "Assault Objective & Blast Door Studio", category: "Window", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "assault_commit_world", title: "Commit Blast Door & Objective to Level", category: "Gameplay", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "win_radar_studio", title: "Team Base Radar & Jammer Studio", category: "Window", shortcut: "", icon: "📡" },
        PaletteCommand { id: "radar_commit_world", title: "Commit Radar Mast & Station to Level", category: "Gameplay", shortcut: "", icon: "📡" },
        PaletteCommand { id: "win_ctf_base_studio", title: "Capture The Flag Base & Routes Studio", category: "Window", shortcut: "", icon: "🚩" },
        PaletteCommand { id: "ctf_base_commit_world", title: "Commit Flag Dais & Base to Level", category: "Gameplay", shortcut: "", icon: "🚩" },
        PaletteCommand { id: "win_team_cannon_studio", title: "Team Cannon & Turret Defense Studio", category: "Window", shortcut: "", icon: "🔫" },
        PaletteCommand { id: "team_cannon_commit_world", title: "Commit Cannon Mount & Turret to Level", category: "Gameplay", shortcut: "", icon: "🔫" },
        PaletteCommand { id: "win_bio_trap_studio", title: "Bio-Rifle Slime Trap & Hazard Studio", category: "Window", shortcut: "", icon: "☣" },
        PaletteCommand { id: "bio_trap_commit_world", title: "Commit Chemical Drain & BioGel Trap to Level", category: "Gameplay", shortcut: "", icon: "☣" },
        PaletteCommand { id: "win_trigger_network_studio", title: "Pressure Plate & Trigger Network Studio", category: "Window", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "trigger_network_commit_world", title: "Commit Plate Frame & Trigger to Level", category: "Gameplay", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "win_elevator_studio", title: "Elevator & Vertical Transit Studio", category: "Window", shortcut: "", icon: "🛗" },
        PaletteCommand { id: "elevator_commit_world", title: "Commit Platform & Bot Nodes to Level", category: "Gameplay", shortcut: "", icon: "🛗" },
        PaletteCommand { id: "win_zone_velocity_studio", title: "Zone Velocity & Fluid Dynamics Studio", category: "Window", shortcut: "", icon: "🌊" },
        PaletteCommand { id: "zone_velocity_commit_world", title: "Commit Portal Sheet & ZoneInfo to Level", category: "Gameplay", shortcut: "", icon: "🌊" },
        PaletteCommand { id: "win_hazard_barrel_studio", title: "Explosive Barrel & Hazard Studio", category: "Window", shortcut: "", icon: "🛢" },
        PaletteCommand { id: "hazard_barrel_commit_world", title: "Commit Hazard Barrel & Actor to Level", category: "Gameplay", shortcut: "", icon: "🛢" },
        PaletteCommand { id: "win_special_lit_studio", title: "SpecialLit & Lighting Mask Studio", category: "Window", shortcut: "", icon: "💡" },
        PaletteCommand { id: "special_lit_commit_world", title: "Commit SpecialLit Fixture & Light to Level", category: "Gameplay", shortcut: "", icon: "💡" },
        PaletteCommand { id: "win_sniper_vantage_studio", title: "Sniper Vantage & Headshot Nest Studio", category: "Window", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "sniper_vantage_commit_world", title: "Commit Parapet & Sniper Weapon to Level", category: "Gameplay", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "win_secret_door_studio", title: "Secret Door & Concealed Passage Studio", category: "Window", shortcut: "", icon: "🚪" },
        PaletteCommand { id: "secret_door_commit_world", title: "Commit Secret Door Mover & Trigger to Level", category: "Gameplay", shortcut: "", icon: "🚪" },
        PaletteCommand { id: "win_teleporter_arch_studio", title: "Teleporter Portal Arch Studio", category: "Window", shortcut: "", icon: "🌀" },
        PaletteCommand { id: "teleporter_arch_commit_world", title: "Commit Portal Arch & Teleporter to Level", category: "Gameplay", shortcut: "", icon: "🌀" },
        PaletteCommand { id: "win_fluid_basin_studio", title: "Toxic Slime Basin & Pool Studio", category: "Window", shortcut: "", icon: "🧪" },
        PaletteCommand { id: "fluid_basin_commit_world", title: "Commit Basin Trough & Zone Actor to Level", category: "Gameplay", shortcut: "", icon: "🧪" },
        PaletteCommand { id: "win_alarm_beacon_studio", title: "Alarm Siren Beacon Studio", category: "Window", shortcut: "", icon: "🚨" },
        PaletteCommand { id: "alarm_beacon_commit_world", title: "Commit Alarm Beacon & Light to Level", category: "Gameplay", shortcut: "", icon: "🚨" },
        PaletteCommand { id: "win_supply_locker_studio", title: "Liandri Supply Locker Studio", category: "Window", shortcut: "", icon: "📦" },
        PaletteCommand { id: "supply_locker_commit_world", title: "Commit Supply Locker & Weapon to Level", category: "Gameplay", shortcut: "", icon: "📦" },
        PaletteCommand { id: "win_skylight_studio", title: "Glass Skylight & Dome Studio", category: "Window", shortcut: "", icon: "🏛" },
        PaletteCommand { id: "skylight_commit_world", title: "Commit Skylight & Sunlight to Level", category: "Gameplay", shortcut: "", icon: "🏛" },
        PaletteCommand { id: "win_vent_duct_studio", title: "Ventilation Duct & Crawlspace Studio", category: "Window", shortcut: "", icon: "💨" },
        PaletteCommand { id: "vent_duct_commit_world", title: "Commit Vent Duct & Grate to Level", category: "Gameplay", shortcut: "", icon: "💨" },
        PaletteCommand { id: "win_piston_studio", title: "Industrial Steam Piston & Compactor Studio", category: "Window", shortcut: "", icon: "⚙" },
        PaletteCommand { id: "piston_commit_world", title: "Commit Piston & Mover to Level", category: "Gameplay", shortcut: "", icon: "⚙" },
        PaletteCommand { id: "win_jump_boots_studio", title: "Jump Boots Cache & High-Vault Studio", category: "Window", shortcut: "", icon: "🦘" },
        PaletteCommand { id: "jump_boots_commit_world", title: "Commit Jump Boots & Cradle to Level", category: "Gameplay", shortcut: "", icon: "🦘" },
        PaletteCommand { id: "win_force_field_studio", title: "Force Field & Energy Barrier Studio", category: "Window", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "force_field_commit_world", title: "Commit Force Field & Pylons to Level", category: "Gameplay", shortcut: "", icon: "🛡" },
        PaletteCommand { id: "win_reactor_core_studio", title: "Reactor Core & Kinetic Ring Studio", category: "Window", shortcut: "", icon: "⚛" },
        PaletteCommand { id: "reactor_core_commit_world", title: "Commit Reactor Core & Rings to Level", category: "Gameplay", shortcut: "", icon: "⚛" },
        PaletteCommand { id: "toggle_experience_mode", title: "Toggle Experience Mode (Standard / Architect)", category: "System", shortcut: "", icon: "🎯" },
        PaletteCommand { id: "help_doc", title: "Open Documentation & Mapping Manual", category: "Help", shortcut: "F1", icon: "📖" },
        PaletteCommand { id: "help_shortcuts", title: "Open Keyboard Shortcuts Cheatsheet", category: "Help", shortcut: "? / Shift+F1", icon: "⌨" },
        PaletteCommand { id: "help_csg", title: "Documentation: CSG & Brush Geometry Reference", category: "Help", shortcut: "", icon: "🧱" },
        PaletteCommand { id: "help_lighting", title: "Documentation: Lighting & Radiosity Studio Guide", category: "Help", shortcut: "", icon: "💡" },
        PaletteCommand { id: "help_movers", title: "Documentation: Movers, Doors & Lifts Guide", category: "Help", shortcut: "", icon: "🚪" },
        PaletteCommand { id: "help_zones", title: "Documentation: Zones, Water & Atmosphere Guide", category: "Help", shortcut: "", icon: "🌊" },
        PaletteCommand { id: "help_surfaces", title: "Documentation: Surfaces, Materials & SkyBoxes Guide", category: "Help", shortcut: "", icon: "🎨" },
        PaletteCommand { id: "help_events", title: "Documentation: Event & Tag Scripting Guide", category: "Help", shortcut: "", icon: "⚡" },
        PaletteCommand { id: "help_bots", title: "Documentation: Bot AI & ReachSpecs Pathing Manual", category: "Help", shortcut: "", icon: "🤖" },
        PaletteCommand { id: "help_gamemodes", title: "Documentation: Tournament Game Modes Guide", category: "Help", shortcut: "", icon: "🎮" },
        PaletteCommand { id: "help_studios", title: "Documentation: Studios & Procedural Generators Guide", category: "Help", shortcut: "", icon: "✨" },
        PaletteCommand { id: "help_gotchas", title: "Documentation: UnrealEd 2.0 Historical Gotchas & FAQ", category: "Help", shortcut: "", icon: "⚠️" },
        PaletteCommand { id: "help_about", title: "About Forge99 Level Editor (Native 64-bit Rust)", category: "Help", shortcut: "", icon: "ℹ" },
    ];

    let screen_rect = ctx.input(|i| i.screen_rect());
    let window_width = 540.0;
    let window_height = 380.0;
    let pos_x = (screen_rect.width() - window_width) * 0.5;
    let pos_y = screen_rect.height() * 0.18;

    egui::Window::new("Quick Command Palette (Ctrl+P)")
        .open(&mut window_open)
        .fixed_pos(egui::pos2(pos_x, pos_y))
        .fixed_size(egui::vec2(window_width, window_height))
        .collapsible(false)
        .resizable(false)
        .title_bar(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔍").size(16.0));
                let search_box = ui.add(
                    egui::TextEdit::singleline(&mut state.command_palette_search)
                        .hint_text("Type a command or shortcut to search...")
                        .desired_width(window_width - 60.0)
                );
                // Auto-focus search input
                search_box.request_focus();

                if search_box.changed() {
                    state.command_palette_selected_index = 0;
                }

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_requested = true;
                }
            });

            if previous_query.as_deref() != Some(state.command_palette_search.as_str()) {
                state.command_palette_selected_index = 0;
                reveal_selection = true;
            }
            ctx.data_mut(|data| data.insert_temp(interaction_id, state.command_palette_search.clone()));

            ui.separator();

            let raw_search = state.command_palette_search.clone();
            let (cat_filter, query) = parse_query(&raw_search);

            let mut filtered_scored: Vec<(&PaletteCommand, i32, bool)> = Vec::new();

            if query.is_empty() {
                // Query is empty: show MRU recent commands first, then all commands
                for recent_id in &state.recent_palette_commands {
                    if let Some(cmd) = commands.iter().find(|c| c.id == recent_id) {
                        if let Some(cats) = cat_filter {
                            if !cats.iter().any(|&c| cmd.category.eq_ignore_ascii_case(c)) {
                                continue;
                            }
                        }
                        filtered_scored.push((cmd, 1000, true));
                    }
                }
                for cmd in commands {
                    if !filtered_scored.iter().any(|(c, _, _)| c.id == cmd.id) {
                        if let Some(cats) = cat_filter {
                            if !cats.iter().any(|&c| cmd.category.eq_ignore_ascii_case(c)) {
                                continue;
                            }
                        }
                        filtered_scored.push((cmd, 0, false));
                    }
                }
            } else {
                for cmd in commands {
                    if let Some(cats) = cat_filter {
                        if !cats.iter().any(|&c| cmd.category.eq_ignore_ascii_case(c)) {
                            continue;
                        }
                    }

                    let score_title = fuzzy_match(query, cmd.title);
                    let score_cat = fuzzy_match(query, cmd.category);
                    let score_sc = fuzzy_match(query, cmd.shortcut);
                    let score_id = fuzzy_match(query, cmd.id);

                    let max_score = score_title
                        .into_iter()
                        .chain(score_cat.map(|s| s - 10))
                        .chain(score_sc.map(|s| s - 5))
                        .chain(score_id.map(|s| s - 10))
                        .max();

                    if let Some(s) = max_score {
                        let is_recent = state.recent_palette_commands.iter().any(|id| id == cmd.id);
                        let final_score = if is_recent { s + 25 } else { s };
                        filtered_scored.push((cmd, final_score, is_recent));
                    }
                }

                // Sort descending by match score
                filtered_scored.sort_by(|a, b| b.1.cmp(&a.1));
            }

            let filtered_len = filtered_scored.len();

            if filtered_len == 0 {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("No matching commands found").color(egui::Color32::from_rgb(150, 150, 160)));
                });
            } else {
                if state.command_palette_selected_index >= filtered_len {
                    state.command_palette_selected_index = 0;
                    reveal_selection = true;
                }

                // Keyboard navigation: ArrowUp/ArrowDown, Tab/Shift+Tab, Enter
                let (down_pressed, up_pressed, enter_pressed) = ctx.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowDown) || (i.key_pressed(egui::Key::Tab) && !i.modifiers.shift),
                        i.key_pressed(egui::Key::ArrowUp) || (i.key_pressed(egui::Key::Tab) && i.modifiers.shift),
                        i.key_pressed(egui::Key::Enter),
                    )
                });

                if down_pressed {
                    state.command_palette_selected_index = (state.command_palette_selected_index + 1) % filtered_len;
                }
                if up_pressed {
                    state.command_palette_selected_index = (state.command_palette_selected_index + filtered_len - 1) % filtered_len;
                }
                reveal_selection |= down_pressed || up_pressed;
                if enter_pressed {
                    action_to_execute = Some(filtered_scored[state.command_palette_selected_index].0.id);
                }

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (idx, (cmd, _score, is_recent)) in filtered_scored.iter().enumerate() {
                            let is_selected = idx == state.command_palette_selected_index;
                            let prefix = if is_selected { "▶ " } else { "   " };
                            let cat_tag = if *is_recent && query.is_empty() {
                                "Recent".to_string()
                            } else {
                                cmd.category.to_string()
                            };
                            let item_label = format!("{}{cmd_icon} {title} [{cat}]", prefix, cmd_icon = cmd.icon, title = cmd.title, cat = cat_tag);
                            let response = ui.horizontal(|ui| {
                                let label_res = ui.selectable_label(is_selected, item_label);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if !cmd.shortcut.is_empty() {
                                        let text_color = if is_selected {
                                            egui::Color32::from_rgb(255, 230, 140)
                                        } else {
                                            egui::Color32::from_rgb(180, 185, 200)
                                        };
                                        ui.label(
                                            egui::RichText::new(cmd.shortcut)
                                                .monospace()
                                                .size(11.0)
                                                .color(text_color)
                                        );
                                    }
                                    if *is_recent && !query.is_empty() {
                                        ui.label(
                                            egui::RichText::new("★")
                                                .size(10.0)
                                                .color(egui::Color32::from_rgb(245, 182, 45))
                                        );
                                    }
                                });
                                label_res
                            }).inner;

                            if is_selected && reveal_selection {
                                response.scroll_to_me(None);
                            }

                            if response.clicked() {
                                state.command_palette_selected_index = idx;
                                action_to_execute = Some(cmd.id);
                            } else if response.hovered() && ctx.input(|i| i.pointer.delta().length_sq() > 0.0) {
                                state.command_palette_selected_index = idx;
                            }
                        }
                    });
            }
        });

    if close_requested || !window_open {
        ctx.data_mut(|data| data.remove::<String>(interaction_id));
        state.show_command_palette = false;
        state.command_palette_search.clear();
        state.command_palette_selected_index = 0;
    } else if let Some(action) = action_to_execute {
        ctx.data_mut(|data| data.remove::<String>(interaction_id));
        state.record_recent_palette_command(action);
        execute_palette_action(action, state);
        state.command_palette_search.clear();
        state.command_palette_selected_index = 0;
        state.show_command_palette = false;
    }
}

/// Subsequence fuzzy matcher with score bonuses (02):
/// - +25 for exact match
/// - +20 for exact prefix match
/// - +15 for word-boundary matches (start of string, after space, underscore, dash, dot, bracket, slash)
/// - +10 for contiguous matching characters
/// - Length penalty to rank concise and specific commands higher.
pub fn fuzzy_match(pattern: &str, target: &str) -> Option<i32> {
    let p_clean = pattern.trim();
    if p_clean.is_empty() {
        return Some(0);
    }

    let p_lower: Vec<char> = p_clean.to_lowercase().chars().collect();
    let t_lower: Vec<char> = target.to_lowercase().chars().collect();
    let t_orig: Vec<char> = target.chars().collect();

    if p_lower.len() > t_lower.len() {
        return None;
    }

    // Exact match bonus
    if p_lower == t_lower {
        return Some(1000);
    }

    // Check substring match
    let is_substr = target.to_lowercase().contains(&p_clean.to_lowercase());

    let mut score = if is_substr { 100 } else { 0 };
    let mut p_idx = 0;
    let mut last_match_idx: Option<usize> = None;

    for (t_idx, &tc) in t_lower.iter().enumerate() {
        if p_idx < p_lower.len() && tc == p_lower[p_idx] {
            score += 10;

            if t_idx == 0 {
                score += 20;
            }

            if t_idx > 0 {
                let prev_c = t_orig[t_idx - 1];
                if prev_c == ' ' || prev_c == '_' || prev_c == '-' || prev_c == '.' || prev_c == '(' || prev_c == '[' || prev_c == '/' {
                    score += 15;
                } else if prev_c.is_lowercase() && t_orig[t_idx].is_uppercase() {
                    score += 15;
                }
            }

            if let Some(last) = last_match_idx {
                if t_idx == last + 1 {
                    score += 10;
                }
            }

            last_match_idx = Some(t_idx);
            p_idx += 1;
        }
    }

    if p_idx == p_lower.len() {
        let len_penalty = (t_lower.len() as i32 - p_lower.len() as i32).min(50);
        Some(score - len_penalty)
    } else {
        None
    }
}

pub fn parse_query(raw_query: &str) -> (Option<&'static [&'static str]>, &str) {
    let q = raw_query.trim_start();
    if let Some(rest) = q.strip_prefix('>') {
        (None, rest.trim())
    } else if let Some(rest) = q.strip_prefix('@') {
        (Some(&["Window", "Tools", "Browser"]), rest.trim())
    } else if let Some(rest) = q.strip_prefix('#') {
        (Some(&["Texture", "Surface", "Brush", "Geometry"]), rest.trim())
    } else if let Some(rest) = q.strip_prefix('?') {
        (Some(&["Help", "System", "File"]), rest.trim())
    } else {
        (None, q)
    }
}


fn execute_palette_action(action: &str, state: &mut EditorState) {
    match action {
        "help_doc" => {
            state.open_help_tab(crate::editor_state::HelpTab::QuickStart);
            return;
        }
        "help_shortcuts" => {
            state.open_help_tab(crate::editor_state::HelpTab::Shortcuts);
            return;
        }
        "help_csg" => {
            state.open_help_tab(crate::editor_state::HelpTab::CsgGeometry);
            return;
        }
        "help_lighting" => {
            state.open_help_tab(crate::editor_state::HelpTab::Lighting);
            return;
        }
        "help_movers" => {
            state.open_help_tab(crate::editor_state::HelpTab::MoversAndTriggers);
            return;
        }
        "help_zones" => {
            state.open_help_tab(crate::editor_state::HelpTab::ZonesAndAtmosphere);
            return;
        }
        "help_surfaces" => {
            state.open_help_tab(crate::editor_state::HelpTab::SurfacesAndMaterials);
            return;
        }
        "help_events" => {
            state.open_help_tab(crate::editor_state::HelpTab::EventSystem);
            return;
        }
        "help_bots" => {
            state.open_help_tab(crate::editor_state::HelpTab::BotNavigation);
            return;
        }
        "help_gamemodes" => {
            state.open_help_tab(crate::editor_state::HelpTab::GameModes);
            return;
        }
        "help_studios" => {
            state.open_help_tab(crate::editor_state::HelpTab::StudiosAndTools);
            return;
        }
        "help_gotchas" => {
            state.open_help_tab(crate::editor_state::HelpTab::UnrealEdGotchas);
            return;
        }
        "help_about" => {
            state.open_help_tab(crate::editor_state::HelpTab::About);
            return;
        }
        _ => {}
    }

    match action {
        "new_map" => {
            if state.map_is_dirty {
                state.pending_dirty_action = Some(crate::editor_state::PendingDirtyAction::NewMap);
                state.show_unsaved_exit_modal = true;
            } else {
                state.new_map();
            }
        }
        "open_map" => {
            state.prompt_or_open_unr_dialog();
        }
        "save_map" => {
            if let Some(ref path) = state.map_path.clone() {
                state.trigger_save_unr(path);
            } else {
                let default_dir = state.default_maps_directory();
                let initial_name = state.map_path.as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("Untitled.unr");
                let dialog = rfd::FileDialog::new()
                    .add_filter("Unreal Map (*.unr)", &["unr"])
                    .set_directory(&default_dir)
                    .set_file_name(initial_name);
                if let Some(path) = dialog.save_file() {
                    state.trigger_save_unr(&path);
                }
            }
        }
        "save_map_as" => {
            let default_dir = state.default_maps_directory();
            let initial_name = state.map_path.as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Untitled.unr");
            let dialog = rfd::FileDialog::new()
                .add_filter("Unreal Map (*.unr)", &["unr"])
                .set_directory(&default_dir)
                .set_file_name(initial_name);
            if let Some(path) = dialog.save_file() {
                state.trigger_save_unr(&path);
            }
        }
        "toggle_console" => {
            state.show_bottom_console = !state.show_bottom_console;
        }
        "play_map" => {
            let _ = state.play_map_in_ut99();
        }
        "toggle_fullscreen" => {
            state.pending_toggle_fullscreen = true;
        }
        "toggle_quad_view" => {
            state.viewport_layout = match state.viewport_layout {
                crate::camera::ViewportLayout::Single => crate::camera::ViewportLayout::FourViewports,
                crate::camera::ViewportLayout::FourViewports => crate::camera::ViewportLayout::Single,
            };
            state.log(format!("Viewport Layout: {:?}", state.viewport_layout));
        }
        "view_persp" => {
            state.active_viewport_type = crate::camera::ViewportType::Perspective;
            state.log("Viewport: 3D Perspective");
        }
        "view_top" => {
            state.active_viewport_type = crate::camera::ViewportType::Top;
            state.log("Viewport: Top XY");
        }
        "view_front" => {
            state.active_viewport_type = crate::camera::ViewportType::Front;
            state.log("Viewport: Front XZ");
        }
        "view_side" => {
            state.active_viewport_type = crate::camera::ViewportType::Side;
            state.log("Viewport: Side YZ");
        }
        "focus_selection" => state.focus_camera_on_selection(),
        "toggle_frustum_culling" => state.toggle_frustum_culling(),
        "toggle_fog" => {
            state.distance_fog_enabled = !state.distance_fog_enabled;
            let status = if state.distance_fog_enabled { "ENABLED" } else { "DISABLED" };
            state.log(format!("Real-Time Distance Fog {status}"));
        }
        "toggle_luxmeter" => {
            state.render_lux_heatmap = !state.render_lux_heatmap;
            let status = if state.render_lux_heatmap { "ENABLED" } else { "DISABLED" };
            state.log(format!("Luxmeter Lighting Heatmap {status}"));
        }
        "toggle_nav_coverage" => {
            state.show_nav_coverage = !state.show_nav_coverage;
            let status = if state.show_nav_coverage { "ENABLED" } else { "DISABLED" };
            state.log(format!("Bot Navigation Coverage Heatmap {status}"));
        }
        "shading_wire" => {
            state.viewport_shading = crate::camera::ViewportShading::Wireframe;
            state.log("Viewport Shading: Wireframe (Alt+1)");
        }
        "shading_zone" => {
            state.viewport_shading = crate::camera::ViewportShading::ZonePortal;
            state.log("Viewport Shading: Zone/Portal View (Alt+2)");
        }
        "shading_unlit" => {
            state.viewport_shading = crate::camera::ViewportShading::Unlit;
            state.log("Viewport Shading: Unlit (Alt+3)");
        }
        "shading_lit" => {
            state.viewport_shading = crate::camera::ViewportShading::Lit;
            state.log("Viewport Shading: Lit (Alt+4)");
        }
        "shading_depth" => {
            state.viewport_shading = crate::camera::ViewportShading::DepthComplexity;
            state.log("Viewport Shading: Depth Complexity Heatmap (Alt+5)");
        }
        "shading_tex_usage" => {
            state.viewport_shading = crate::camera::ViewportShading::TextureUsage;
            state.log("Viewport Shading: Texture Usage Color-Coding (Alt+6)");
        }
        "shading_bsp_cuts" => {
            state.viewport_shading = crate::camera::ViewportShading::BspCuts;
            state.log("Viewport Shading: BSP Cuts Diagnostic (Alt+7)");
        }
        "cycle_cam_speed" => {
            state.cycle_camera_speed();
        }
        "toggle_minimap" => {
            state.show_minimap = !state.show_minimap;
            let status = if state.show_minimap { "ENABLED" } else { "DISABLED" };
            state.log(format!("Map Navigation Radar {status}"));
        }
        "toggle_radii" => {
            state.show_actor_radii = !state.show_actor_radii;
            let status = if state.show_actor_radii { "ENABLED" } else { "DISABLED" };
            state.log(format!("Radii View (Collision & Attenuation) {status}"));
        }
        "toggle_doctor_beacons" => {
            state.show_diagnostic_markers = !state.show_diagnostic_markers;
            let status = if state.show_diagnostic_markers { "ENABLED" } else { "DISABLED" };
            state.log(format!("Level Doctor 3D Beacons {status}"));
        }
        "tex_pan_up" => state.pan_selected_brush_texture(0.0, state.grid_step),
        "tex_pan_down" => state.pan_selected_brush_texture(0.0, -state.grid_step),
        "tex_pan_left" => state.pan_selected_brush_texture(-state.grid_step, 0.0),
        "tex_pan_right" => state.pan_selected_brush_texture(state.grid_step, 0.0),
        "tex_rotate_45" => state.rotate_selected_brush_texture(45.0),
        "build_geom" => {
            state.build_geometry();
        }
        "build_light" => {
            state.show_lighting_dialog = true;
        }
        "build_paths" => {
            state.build_ai_paths();
        }
        "build_all" => {
            state.build_all_complete();
        }
        "win_build_options" => {
            state.show_build_options_dialog = true;
        }
        "undo" => state.undo(),
        "redo" => state.redo(),
        "cut" => {
            state.cut_selected_to_clipboard();
        }
        "copy" => {
            state.copy_selected_to_clipboard();
        }
        "paste" => {
            state.paste_from_clipboard();
        }
        "delete" => {
            state.delete_selected_batch();
        }
        "select_all" => state.select_all(),
        "select_all_brushes" => state.select_all_brushes(),
        "select_all_actors" => state.select_all_actors(),
        "deselect_all" => state.deselect_all(),
        "invert_selection" => state.invert_selection(),
        "mirror_x" => state.mirror_selected(crate::editor_state::MirrorAxis::X),
        "mirror_y" => state.mirror_selected(crate::editor_state::MirrorAxis::Y),
        "mirror_z" => state.mirror_selected(crate::editor_state::MirrorAxis::Z),
        "duplicate" => {
            if state.has_multi_selection() {
                state.duplicate_selected_batch(glam::Vec3::new(state.grid_step, state.grid_step, 0.0));
            } else {
                state.duplicate_selected();
            }
        }
        "toggle_gizmo_space" => state.toggle_gizmo_space(),
        "gizmo_translate" => {
            state.gizmo_mode = crate::raycast::GizmoMode::Translate;
            state.log("Gizmo Mode: Translate (W)");
        }
        "gizmo_rotate" => {
            state.gizmo_mode = crate::raycast::GizmoMode::Rotate;
            state.log("Gizmo Mode: Rotate (E)");
        }
        "gizmo_scale" => {
            state.gizmo_mode = crate::raycast::GizmoMode::Scale;
            state.log("Gizmo Mode: Scale (R)");
        }
        "open_action_wizards" => {
            state.show_action_wizards_dialog = true;
        }
        "open_visual_presets" => {
            state.show_visual_presets_dialog = true;
        }
        "snap_floor" => {
            if state.has_multi_selection() {
                state.snap_multi_selected_to_floor();
            } else {
                state.snap_selected_to_floor();
            }
        }
        "snap_floor_aligned" => {
            if state.has_multi_selection() {
                state.snap_multi_selected_to_floor_aligned();
            } else {
                state.snap_selected_to_floor_aligned();
            }
        }
        "snap_grid_objects" => {
            state.snap_selected_to_grid();
        }
        "snap_grid" => {
            state.snap_selected_vertices_to_grid();
        }
        "weld_vertices" => {
            state.weld_selected_vertices_to_nearest(16.0);
        }
        "fit_texture" => {
            state.fit_selected_brush_texture(1.0, 1.0);
        }
        "close_all_windows" => {
            state.close_all_floating_dialogs();
        }
        "workspace_default" => {
            state.apply_workspace_preset(crate::editor_state::WorkspacePreset::Default);
        }
        "workspace_level_design" => {
            state.apply_workspace_preset(crate::editor_state::WorkspacePreset::LevelDesign);
        }
        "workspace_lighting" => {
            state.apply_workspace_preset(crate::editor_state::WorkspacePreset::LightingAndAtmosphere);
        }
        "workspace_bots" => {
            state.apply_workspace_preset(crate::editor_state::WorkspacePreset::BotAndGameplay);
        }
        "win_texture_browser" => state.show_texture_browser = true,
        "win_surface_props" => state.show_surface_properties = true,
        "win_actor_props" => state.show_actor_properties = true,
        "win_group_browser" => state.show_group_browser = true,
        "win_level_doctor" => state.show_level_doctor_window = true,
        "win_budget_profiler" => state.show_map_budget_dialog = true,
        "win_actor_browser" => state.show_actor_browser = true,
        "win_sound_browser" => state.show_sound_browser = true,
        "win_music_browser" => state.show_music_browser = true,
        "win_bot_sim" => state.show_bot_simulator_window = true,
        "win_bookmarks" => state.show_camera_bookmarks_dialog = true,
        "win_acoustics" => state.show_acoustics_dialog = true,
        "win_gameplay_balance" => state.show_gameplay_balance_dialog = true,
        "win_zone_seal" => state.show_zone_seal_dialog = true,
        "find_actors" | "win_find_actors" => state.show_find_actors_dialog = true,
        "replace_textures" | "win_replace_textures" => state.show_replace_textures_dialog = true,
        "scale_brush" | "win_scale_brush" => state.show_scale_brush_dialog = true,
        "shear_brush" | "win_shear_brush" => state.show_shear_brush_dialog = true,
        "align_actors" | "win_align_actors" => state.show_actor_align_dialog = true,
        "map_statistics" | "win_map_statistics" => state.show_map_statistics_dialog = true,
        "win_dispatcher" => state.show_dispatcher_dialog = true,
        "win_ctf_symmetry" => state.show_ctf_symmetry_dialog = true,
        "win_prefab" => state.show_prefab_browser = true,
        "win_shape_editor" => state.show_shape_editor = true,
        "win_packaging" => state.show_map_packaging_dialog = true,
        "win_package_deps" => {
            state.show_package_manifest_dialog = true;
            state.refresh_map_manifest();
        }
        "win_decal_studio" => state.show_decal_studio_dialog = true,
        "bake_decals" => state.bake_projector_decals(),
        "win_uc_viewer" => {
            let cls = state.selected_class_name.clone().unwrap_or_else(|| "Actor".to_string());
            state.open_uc_viewer_for_class(&cls);
        }
        "win_skybox_studio" => state.show_skybox_studio_dialog = true,
        "skybox_place_anchor" => { let _ = state.place_sky_zone_anchor_at_pivot(); }
        "skybox_mark_surfaces" => { state.mark_selected_surfaces_as_sky(true); }
        "skybox_focus" => state.focus_camera_on_sky_zone(),
        "win_acoustic_studio" => state.show_acoustics_dialog = true,
        "acoustic_place_sound" => {
            let snd = state.selected_sound_name.clone().unwrap_or_else(|| "AmbAncient.Loop.Hum".to_string());
            let _ = state.place_ambient_sound_at_pivot(&snd, 32, 128);
        }
        "win_warp_zone_studio" => state.show_warp_zone_studio_dialog = true,
        "warp_create_pair" => {
            let p = state.builder_brush.location;
            let (a, _) = state.create_coordinated_warp_pair(p, glam::Vec3::new(1024.0, 0.0, 0.0));
            state.warp_zone_settings.selected_warp_index = Some(a);
        }
        "warp_teleport_camera" => {
            let topology = state.collect_warpzone_topology();
            if let Some(target) = state.warp_zone_settings.selected_warp_index
                .or_else(|| topology.first().map(|e| e.actor_index)) {
                let _ = state.teleport_camera_through_warp_zone(target);
            }
        }
        "win_lighting_studio" => state.show_lighting_studio_dialog = true,
        "lighting_bake_differential" => { let _ = state.bake_selected_lights_only(); }
        "lighting_apply_midnight" => { state.apply_light_preset_to_selected(crate::editor_state::TournamentLightPreset::FacingWorldsMidnight); }
        "win_zone_fog" => state.show_zone_fog_studio_dialog = true,
        "fog_place_zone" => {
            let preset = state.zone_fog_settings.active_preset.unwrap_or(crate::editor_state::FogPresetId::MistyMorningSwamp);
            let _ = state.place_fog_zone_at_pivot(preset);
        }
        "fog_toggle_shells" => {
            state.zone_fog_settings.show_distance_shells = !state.zone_fog_settings.show_distance_shells;
            let status = if state.zone_fog_settings.show_distance_shells { "ENABLED" } else { "DISABLED" };
            state.log(format!("3D Fog Distance Shells {status}"));
        }
        "win_dispatcher_studio" => state.show_dispatcher_dialog = true,
        "dispatcher_place_pivot" => {
            let _ = state.place_dispatcher_at_pivot(Some(ut99_bsp::DispatcherPresetTemplate::TimedBlastAndBreach));
        }
        "dispatcher_play_toggle" => {
            state.dispatcher_settings.is_playing = !state.dispatcher_settings.is_playing;
            let status = if state.dispatcher_settings.is_playing { "PLAYING" } else { "PAUSED" };
            state.log(format!("Dispatcher Timeline Simulation {status}"));
        }
        "win_skeletal_studio" => state.show_skeletal_studio_dialog = true,
        "win_skin_studio" => state.show_skin_studio_dialog = true,
        "win_package_extractor" => state.show_package_extractor_dialog = true,
        "win_uc_wizard" => state.show_uc_class_wizard_dialog = true,
        "skeletal_place_pivot" => {
            let _ = state.place_skeletal_mesh_at_pivot(0);
        }
        "skeletal_reset_pose" => {
            let _ = state.reset_skeletal_mesh_to_bind_pose(0);
        }
        "win_particle_studio" => state.show_particle_studio_dialog = true,
        "particle_place_pivot" => {
            let _ = state.place_particle_emitter_at_pivot(None);
        }
        "particle_sim_toggle" => {
            state.particle_settings.is_simulating = !state.particle_settings.is_simulating;
            let status = if state.particle_settings.is_simulating { "RUNNING" } else { "PAUSED" };
            state.log(format!("Particle Simulation {status}"));
        }
        "win_bot_nav_studio" => state.show_bot_nav_studio_dialog = true,
        "nav_build_paths" => {
            let max_d = state.auto_path_max_distance;
            let max_s = state.auto_path_max_step;
            let max_j = state.auto_path_max_jump;
            state.build_auto_paths(max_d, max_s, max_j);
        }
        "nav_focus_node" => {
            if let Some(sel_id) = state.bot_nav_settings.selected_node_id {
                if let Some(node) = state.nav_graph.nodes.get(sel_id) {
                    state.camera.position = node.location - state.camera.forward() * 200.0;
                }
            }
        }
        "win_arch_studio" => state.show_architectural_studio_dialog = true,
        "arch_to_builder" => state.synthesize_prefab_to_builder(),
        "arch_bake_add" => {
            state.bake_prefab_to_world(ut99_bsp::CsgOperation::Add);
        }
        "arch_bake_sub" => {
            state.bake_prefab_to_world(ut99_bsp::CsgOperation::Subtract);
        }
        "win_terrain_studio" => state.show_terrain_studio_dialog = true,
        "terrain_to_builder" => state.synthesize_terrain_to_builder(),
        "terrain_bake_add" => {
            state.bake_terrain_to_world(ut99_bsp::CsgOperation::Add);
        }
        "terrain_bake_sub" => {
            state.bake_terrain_to_world(ut99_bsp::CsgOperation::Subtract);
        }
        "win_lod_collision" => state.show_lod_collision_dialog = true,
        "lod_bake_hull" => {
            let hull_type = state.lod_collision_settings.hull_type;
            let margin = state.lod_collision_settings.hull_margin;
            state.bake_collision_hull_as_brush(hull_type, margin);
        }
        "lod_apply_level" => {
            let level = state.lod_collision_settings.target_lod_level;
            state.apply_lod_to_selected_brush(level);
        }
        "win_cinematic_studio" => state.show_cinematic_studio_dialog = true,
        "cinematic_capture_cam" => state.add_keyframe_at_camera_position(),
        "cinematic_play_toggle" => {
            state.cinematic_settings.is_playing = !state.cinematic_settings.is_playing;
        }
        "cinematic_export_actors" => {
            state.export_cinematic_to_level_actors();
        }
        "win_light_effects_studio" => state.show_light_effects_studio_dialog = true,
        "light_place_animated" => {
            let p = state.light_effects_settings.selected_preset;
            state.place_dynamic_light_at_pivot(p);
        }
        "light_apply_preset" => {
            let p = state.light_effects_settings.selected_preset;
            state.apply_dynamic_light_preset_to_selected(p);
        }
        "win_zone_occlusion_studio" => state.show_zone_occlusion_studio_dialog = true,
        "zone_recompute_pvs" => {
            state.analyze_zone_occlusion();
        }
        "zone_create_portal_sheet" => {
            let w = state.zone_occlusion_settings.portal_sheet_width;
            let h = state.zone_occlusion_settings.portal_sheet_height;
            let n = state.zone_occlusion_settings.portal_sheet_normal;
            state.create_portal_sheet_at_pivot(w, h, n);
        }
        "zone_add_antiportal" => {
            let sz = state.zone_occlusion_settings.antiportal_size;
            state.add_antiportal_volume_at_pivot(sz);
        }
        "win_music_studio" => state.show_music_studio_dialog = true,
        "music_place_event" => {
            let song = state.music_studio_settings.current_track_info.package.clone();
            let section = state.music_studio_settings.selected_section;
            let trans = state.music_studio_settings.crossfade_transition;
            state.place_music_event_at_pivot(&song, section, trans);
        }
        "music_audit_level" => {
            state.audit_music_setup();
        }
        "win_corona_studio" => state.show_corona_studio_dialog = true,
        "corona_place_sun" => {
            let p = state.corona_studio_settings.selected_preset;
            state.place_corona_light_at_pivot(p);
        }
        "corona_apply_preset" => {
            let p = state.corona_studio_settings.selected_preset;
            state.apply_corona_preset_to_selected(p);
        }
        "win_fluid_studio" => state.show_fluid_studio_dialog = true,
        "fluid_place_zone" => {
            let p = state.fluid_studio_settings.selected_preset;
            state.place_physics_zone_at_pivot(p);
        }
        "fluid_apply_preset" => {
            let p = state.fluid_studio_settings.selected_preset;
            state.apply_fluid_preset_to_selected(p);
        }
        "win_ballistics_studio" => state.show_ballistics_studio_dialog = true,
        "ballistics_place_probe" => {
            let p = state.ballistics_studio_settings.selected_preset;
            state.place_ballistic_probe_at_pivot(p);
        }
        "ballistics_recalculate" => {
            state.calculate_active_weapon_trajectory();
        }
        "win_teleporter_studio" => state.show_teleporter_studio_dialog = true,
        "teleporter_pair_spawn" => {
            let offset = state.teleporter_studio_settings.auto_pair_offset;
            state.create_paired_teleporters_at_pivot(offset);
        }
        "win_translocator_studio" => state.show_translocator_studio_dialog = true,
        "translocator_place_probe" => {
            state.place_translocator_probe_at_pivot();
        }
        "translocator_recalculate" => {
            state.calculate_active_translocator_simulation();
        }
        "win_lift_pathing_studio" => state.show_lift_pathing_studio_dialog = true,
        "lift_pathing_sync_setup" => {
            let mover_idx = state.lift_pathing_studio_settings.selected_elevator_idx;
            state.create_synchronized_lift_setup_for_mover(mover_idx);
        }
        "lift_pathing_autofix" => {
            if let Some(ref audit) = state.lift_pathing_studio_settings.cached_audit {
                let idx = state.lift_pathing_studio_settings.selected_elevator_idx;
                if let Some(sys) = audit.systems.get(idx) {
                    let tag = sys.mover_tag.clone();
                    state.auto_fix_elevator_issues(&tag);
                }
            }
        }
        "win_water_surface_studio" => state.show_water_surface_studio_dialog = true,
        "water_surface_generate_sheet" => {
            state.generate_water_sheet_brush();
        }
        "water_surface_reset_sim" => state.reset_water_simulation(),
        "win_mover_studio" => state.show_mover_studio_dialog = true,
        "mover_wire_proximity_trigger" => {
            state.wire_proximity_trigger_for_active_mover();
        }
        "mover_audit_hazards" => state.audit_active_mover_hazards(),
        "win_team_objective_studio" => {
            state.show_team_objective_dialog = true;
            state.scan_and_refresh_team_objectives();
        }
        "team_obj_deploy_defense" => {
            state.generate_defense_points_for_active_objective();
        }
        "team_obj_refresh_analysis" => state.scan_and_refresh_team_objectives(),
        "win_destructible_studio" => state.show_destructible_studio_dialog = true,
        "destructible_shatter_test" => state.trigger_destructible_shatter(),
        "destructible_generate_barrier" => {
            state.generate_destructible_barrier();
        }
        "win_kicker_studio" => state.show_kicker_studio_dialog = true,
        "kicker_solve_trajectory" => state.solve_kicker_velocity_to_target(),
        "kicker_generate_setup" => {
            state.generate_kicker_setup();
        }
        "win_bsp_quality_studio" => {
            state.show_bsp_quality_studio_dialog = true;
            state.run_bsp_quality_audit();
        }
        "bsp_quality_run_audit" => state.run_bsp_quality_audit(),
        "bsp_quality_purge_slivers" => {
            state.purge_all_sliver_polygons();
        }
        "bsp_quality_convert_semisolid" => {
            state.convert_top_cut_offenders_to_semisolid(5);
        }
        "win_shape_loft_studio" => {
            state.show_loft_studio_dialog = true;
            state.recalculate_loft_mesh();
        }
        "loft_commit_builder" => state.commit_loft_to_builder_brush(),
        "loft_commit_world_solid" => {
            state.commit_loft_to_world_brush(ut99_bsp::CsgOperation::Add);
        }
        "win_stairway_studio" => {
            state.show_stairway_studio_dialog = true;
            state.recalculate_stair_geometry();
        }
        "stair_commit_builder" => state.commit_stair_to_builder_brush(),
        "stair_commit_combo" => {
            state.commit_stair_and_ramp_to_world(ut99_bsp::CsgOperation::Add);
        }
        "win_scatter_studio" => {
            state.show_scatter_studio_dialog = true;
            state.recalculate_scatter_preview();
        }
        "scatter_commit" => {
            state.commit_scatter_instances_to_world();
        }
        "scatter_reroll_seed" => state.reroll_scatter_seed(),
        "win_light_shaft_studio" => {
            state.show_light_shaft_dialog = true;
            state.recalculate_light_shaft_geometry();
        }
        "light_shaft_commit_builder" => state.commit_light_shaft_to_builder_brush(),
        "light_shaft_commit_world" => {
            state.commit_light_shaft_to_world();
        }
        "win_track_studio" => {
            state.show_track_studio_dialog = true;
            state.recalculate_track_preview();
        }
        "track_commit_world" => {
            state.commit_track_setup();
        }
        "win_surveillance_studio" => {
            state.show_surveillance_dialog = true;
            state.recalculate_surveillance_preview();
        }
        "surveillance_commit_world" => {
            state.commit_surveillance_setup_to_world();
        }
        "win_laser_studio" => {
            state.show_laser_grid_dialog = true;
            state.recalculate_laser_preview();
        }
        "laser_commit_world" => {
            state.commit_laser_grid_to_world();
        }
        "win_ventilator_studio" => {
            state.show_ventilator_dialog = true;
            state.recalculate_ventilator_preview();
        }
        "ventilator_commit_world" => {
            state.commit_ventilator_to_world();
        }
        "win_conveyor_studio" => {
            state.show_conveyor_dialog = true;
            state.recalculate_conveyor_preview();
        }
        "conveyor_commit_world" => {
            state.commit_conveyor_to_world();
        }
        "win_turret_studio" => {
            state.show_turret_dialog = true;
            state.recalculate_turret_preview();
        }
        "turret_commit_world" => {
            state.commit_turret_to_world();
        }
        "win_soundscape_studio" => {
            state.show_soundscape_dialog = true;
            state.recalculate_soundscape_preview();
        }
        "soundscape_commit_world" => {
            state.commit_soundscape_to_world();
        }
        "win_hazard_chamber_studio" => {
            state.show_hazard_chamber_dialog = true;
            state.recalculate_hazard_chamber_preview();
        }
        "hazard_chamber_commit_world" => {
            state.commit_hazard_chamber_to_world();
        }
        "win_pipe_conduit_studio" => {
            state.show_pipe_conduit_dialog = true;
            state.recalculate_pipe_preview();
        }
        "pipe_conduit_commit_world" => {
            state.commit_pipe_conduit_to_world();
        }
        "win_hologram_studio" => {
            state.show_hologram_dialog = true;
            state.recalculate_hologram_preview();
        }
        "hologram_commit_world" => {
            state.commit_hologram_to_world();
        }
        "win_weather_studio" => {
            state.show_weather_dialog = true;
            state.recalculate_weather_preview();
        }
        "weather_commit_world" => {
            state.commit_weather_to_world();
        }
        "win_arena_rules_studio" => {
            state.show_arena_rules_dialog = true;
            state.recalculate_arena_rules_preview();
        }
        "arena_rules_commit_world" => {
            state.commit_arena_rules_to_world();
        }
        "win_armor_stacking_studio" => {
            state.show_armor_stacking_dialog = true;
            state.recalculate_armor_stacking_preview();
        }
        "armor_stacking_commit_world" => {
            state.commit_armor_stacking_to_world();
        }
        "win_bot_jump_routes_studio" => {
            state.show_bot_jump_routes_dialog = true;
            state.recalculate_jump_route_preview();
        }
        "bot_jump_routes_commit_world" => {
            state.commit_jump_route_to_world();
        }
        "win_weapon_circuit_studio" => {
            state.show_weapon_circuit_dialog = true;
            state.recalculate_weapon_circuit_preview();
        }
        "weapon_circuit_commit_world" => {
            state.commit_weapon_station_to_world();
        }
        "win_powerup_control_studio" => {
            state.show_powerup_control_dialog = true;
            state.recalculate_powerup_preview();
        }
        "powerup_control_commit_world" => {
            state.commit_powerup_to_world();
        }
        "win_spawn_distribution_studio" => {
            state.show_spawn_distribution_dialog = true;
            state.recalculate_spawn_preview();
        }
        "spawn_distribution_commit_world" => {
            state.commit_spawn_pad_to_world();
        }
        "win_domination_studio" => {
            state.show_domination_dialog = true;
            state.recalculate_domination_preview();
        }
        "domination_commit_world" => {
            state.commit_domination_point_to_world();
        }
        "win_assault_studio" => {
            state.show_assault_dialog = true;
            state.recalculate_assault_preview();
        }
        "assault_commit_world" => {
            state.commit_assault_objective_to_world();
        }
        "win_radar_studio" => {
            state.show_radar_dialog = true;
            state.recalculate_radar_preview();
        }
        "radar_commit_world" => {
            state.commit_radar_station_to_world();
        }
        "win_ctf_base_studio" => {
            state.show_ctf_base_dialog = true;
            state.recalculate_ctf_base_preview();
        }
        "ctf_base_commit_world" => {
            state.commit_ctf_base_to_world();
        }
        "win_team_cannon_studio" => {
            state.show_team_cannon_dialog = true;
            state.recalculate_team_cannon_preview();
        }
        "team_cannon_commit_world" => {
            state.commit_team_cannon_to_world();
        }
        "win_bio_trap_studio" => {
            state.show_bio_trap_dialog = true;
            state.recalculate_bio_trap_preview();
        }
        "bio_trap_commit_world" => {
            state.commit_bio_trap_to_world();
        }
        "win_trigger_network_studio" => {
            state.show_trigger_network_dialog = true;
            state.recalculate_trigger_network_preview();
        }
        "trigger_network_commit_world" => {
            state.commit_trigger_network_to_world();
        }
        "win_elevator_studio" => {
            state.show_elevator_dialog = true;
            state.recalculate_elevator_preview();
        }
        "elevator_commit_world" => {
            state.commit_elevator_to_world();
        }
        "win_zone_velocity_studio" => {
            state.show_zone_velocity_dialog = true;
            state.recalculate_zone_velocity_preview();
        }
        "zone_velocity_commit_world" => {
            state.commit_zone_velocity_to_world();
        }
        "win_hazard_barrel_studio" => {
            state.show_hazard_barrel_dialog = true;
            state.recalculate_hazard_barrel_preview();
        }
        "hazard_barrel_commit_world" => {
            state.commit_hazard_barrel_to_world();
        }
        "win_special_lit_studio" => {
            state.show_special_lit_dialog = true;
            state.recalculate_special_lit_preview();
        }
        "special_lit_commit_world" => {
            state.commit_special_lit_to_world();
        }
        "win_sniper_vantage_studio" => {
            state.show_sniper_vantage_dialog = true;
            state.recalculate_sniper_vantage_preview();
        }
        "sniper_vantage_commit_world" => {
            state.commit_sniper_vantage_to_world();
        }
        "win_secret_door_studio" => {
            state.show_secret_door_dialog = true;
            state.recalculate_secret_door_preview();
        }
        "secret_door_commit_world" => {
            state.commit_secret_door_to_world();
        }
        "win_teleporter_arch_studio" => {
            state.show_teleporter_arch_dialog = true;
            state.recalculate_teleporter_arch_preview();
        }
        "teleporter_arch_commit_world" => {
            state.commit_teleporter_arch_to_world();
        }
        "win_fluid_basin_studio" => {
            state.show_fluid_basin_dialog = true;
            state.recalculate_fluid_basin_preview();
        }
        "fluid_basin_commit_world" => {
            state.commit_fluid_basin_to_world();
        }
        "win_alarm_beacon_studio" => {
            state.show_alarm_beacon_dialog = true;
            state.recalculate_alarm_beacon_preview();
        }
        "alarm_beacon_commit_world" => {
            state.commit_alarm_beacon_to_world();
        }
        "win_supply_locker_studio" => {
            state.show_supply_locker_dialog = true;
            state.recalculate_supply_locker_preview();
        }
        "supply_locker_commit_world" => {
            state.commit_supply_locker_to_world();
        }
        "win_skylight_studio" => {
            state.show_skylight_dialog = true;
            state.recalculate_skylight_preview();
        }
        "skylight_commit_world" => {
            state.commit_skylight_to_world();
        }
        "win_vent_duct_studio" => {
            state.show_vent_duct_dialog = true;
            state.recalculate_vent_duct_preview();
        }
        "vent_duct_commit_world" => {
            state.commit_vent_duct_to_world();
        }
        "win_piston_studio" => {
            state.show_piston_dialog = true;
            state.recalculate_piston_preview();
        }
        "piston_commit_world" => {
            state.commit_piston_to_world();
        }
        "win_jump_boots_studio" => {
            state.show_jump_boots_dialog = true;
            state.recalculate_jump_boots_preview();
        }
        "jump_boots_commit_world" => {
            state.commit_jump_boots_to_world();
        }
        "win_force_field_studio" => {
            state.show_force_field_dialog = true;
            state.recalculate_force_field_preview();
        }
        "force_field_commit_world" => {
            state.commit_force_field_to_world();
        }
        "win_reactor_core_studio" => {
            state.show_reactor_core_dialog = true;
            state.recalculate_reactor_core_preview();
        }
        "reactor_core_commit_world" => {
            state.commit_reactor_core_to_world();
        }
        "select_all_surfaces" => state.select_all_surfaces(),
        "deselect_all_surfaces" => state.deselect_all_surfaces(),
        "select_matching_texture" => state.select_surfaces_matching_texture(),
        "select_coplanar_adjacent" => state.select_surfaces_coplanar_adjacent(),
        "select_coplanar_all" => state.select_surfaces_coplanar_all(),
        "select_adjacent_walls" => state.select_surfaces_adjacent_walls(),
        "select_adjacent_slants" => state.select_surfaces_adjacent_slants(),
        "invert_surface_selection" => state.invert_surface_selection(),
        "select_surfaces_on_brush" => state.select_surfaces_on_selected_brushes(),
        "align_continuous_strip" => state.align_selected_surfaces_continuous(1.0, 1.0),
        "snap_brush_pivot_vertex" => state.snap_selected_brush_pivot_to_nearest_vertex(None),
        "snap_brush_pivot_center" => state.snap_brush_pivot_to_center(),
        "select_actors_matching_tag" => { let _ = state.select_actors_matching_tag(); }
        "select_actors_matching_event" => { let _ = state.select_actors_matching_event(); }
        "select_actors_targeted_by_event" => { let _ = state.select_actors_targeted_by_event(); }
        "select_actors_triggering_tag" => { let _ = state.select_actors_triggering_tag(); }
        "preset_flags_water" => state.apply_surface_flags_preset(crate::editor_state::SurfaceFlagsPreset::Water),
        "preset_flags_lava" => state.apply_surface_flags_preset(crate::editor_state::SurfaceFlagsPreset::LavaSlime),
        "preset_flags_glass" => state.apply_surface_flags_preset(crate::editor_state::SurfaceFlagsPreset::GlassWindow),
        "preset_flags_grate" => state.apply_surface_flags_preset(crate::editor_state::SurfaceFlagsPreset::MaskedGrate),
        "preset_flags_sky" => state.apply_surface_flags_preset(crate::editor_state::SurfaceFlagsPreset::SkyBackdrop),
        "preset_flags_invisible" => state.apply_surface_flags_preset(crate::editor_state::SurfaceFlagsPreset::InvisibleBarrier),
        "preset_flags_unlit" => state.apply_surface_flags_preset(crate::editor_state::SurfaceFlagsPreset::FullbrightUnlit),
        "preset_flags_clear" => state.apply_surface_flags_preset(crate::editor_state::SurfaceFlagsPreset::ClearAll),
        "extract_surface_to_builder" => { let _ = state.extract_selected_surfaces_to_builder_brush(); }
        "brush_make_hollow_16" => { let _ = state.make_builder_brush_hollow(16.0); }
        "brush_make_hollow_32" => { let _ = state.make_builder_brush_hollow(32.0); }
        "reset_surface_pan" => state.reset_selected_surfaces_pan(),
        "snap_surface_uv_16" => state.snap_selected_surfaces_uv_to_grid(16.0),
        "snap_surface_uv_64" => state.snap_selected_surfaces_uv_to_grid(64.0),
        "align_wall_auto" => state.align_selected_brush_texture("wall_auto"),
        "align_fit_2x2" => state.fit_selected_brush_texture(2.0, 2.0),
        "align_fit_4x4" => state.fit_selected_brush_texture(4.0, 4.0),
        "vertex_split_edge" => { let _ = state.split_selected_edge(); }
        "vertex_collapse_edge" => { let _ = state.collapse_selected_edge(); }
        "mover_next_key" => { let _ = state.cycle_selected_mover_key(true); }
        "mover_prev_key" => { let _ = state.cycle_selected_mover_key(false); }
        "grid_step_up" => { state.step_grid_up(); }
        "grid_step_down" => { state.step_grid_down(); }
        "fov_cycle" => {
            let cur = state.get_camera_fov_degrees();
            let next = if cur < 75.0 { 80.0 } else if cur < 85.0 { 90.0 } else if cur < 95.0 { 100.0 } else { 65.0 };
            state.set_camera_fov_degrees(next);
        }
        "brush_export_obj" => {
            let default_dir = state.default_maps_directory();
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Wavefront OBJ", &["obj"])
                .set_directory(&default_dir)
                .set_file_name("BuilderBrush.obj")
                .save_file()
            {
                state.trigger_export_obj(&path);
            }
        }
        "brush_solidity_solid" => { let _ = state.set_selected_brush_solidity(crate::editor_state::BrushSolidity::Solid); }
        "brush_solidity_semisolid" => { let _ = state.set_selected_brush_solidity(crate::editor_state::BrushSolidity::SemiSolid); }
        "brush_solidity_nonsolid" => { let _ = state.set_selected_brush_solidity(crate::editor_state::BrushSolidity::NonSolid); }
        "brush_order_first" => state.send_selected_brush_to_first(),
        "brush_order_last" => state.send_selected_brush_to_last(),
        "brush_order_up" => state.move_selected_brush_up(),
        "brush_order_down" => state.move_selected_brush_down(),
        "brush_invert_normals" => state.invert_selected_brush(),
        "surface_invert_normals" => { let _ = state.invert_selected_surfaces(); }
        "select_brushes_additive" => { let _ = state.select_additive_brushes(); }
        "select_brushes_subtractive" => { let _ = state.select_subtractive_brushes(); }
        "select_brushes_semisolid" => { let _ = state.select_semisolid_brushes(); }
        "select_brushes_nonsolid" => { let _ = state.select_nonsolid_brushes(); }
        "select_actors_lights" => { let _ = state.select_all_lights(); }
        "select_actors_pathnodes" => { let _ = state.select_all_pathnodes(); }
        "brush_transform_permanently" => { let _ = state.transform_permanently_selected(); }
        "brush_reset_rotation" => { let _ = state.reset_rotation_selected(); }
        "brush_reset_scaling" => { let _ = state.reset_scaling_selected(); }
        "brush_reset_all" => { let _ = state.reset_all_selected(); }
        "level_properties" => state.show_level_properties_dialog = true,
        "scale_entire_map" => state.show_scale_map_dialog = true,
        "tex_export_active" => {
            if let Some(ref active) = state.active_texture {
                let parts: Vec<&str> = active.split('.').collect();
                if parts.len() >= 2 {
                    let p_name = parts[0];
                    let t_name = parts[parts.len() - 1];
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Portable Network Graphics (*.png)", &["png"])
                        .set_file_name(&format!("{t_name}.png"))
                        .save_file()
                    {
                        match state.texture_manager.export_texture_to_file(p_name, t_name, &path) {
                            Ok(()) => state.log(format!("Exported texture '{active}' to '{}'", path.display())),
                            Err(e) => state.log(format!("Failed to export texture: {e}")),
                        }
                    }
                }
            } else {
                state.log("No active texture selected to export.");
            }
        }
        "tex_export_package" => {
            let pkg_name = state.selected_texture_package.clone();
            if let Some(dir) = rfd::FileDialog::new().set_title("Select Folder to Export All Textures").pick_folder() {
                match state.texture_manager.export_package_to_directory(&pkg_name, &dir) {
                    Ok(count) => {
                        state.log(format!("Exported {} texture(s) from package '{}' to '{}'", count, pkg_name, dir.display()));
                    }
                    Err(e) => {
                        state.log(format!("Failed to export package: {e}"));
                    }
                }
            }
        }
        "win_mesh_browser" => state.show_mesh_browser = true,
        "mesh_place_active" => {
            if let Some(ref name) = state.selected_mesh_name.clone() {
                let loc = state.builder_brush.location;
                if let Err(e) = state.place_mesh_from_library(name, loc) {
                    state.log(format!("Failed to place mesh: {e}"));
                }
            } else {
                state.log("No mesh selected in library to place.");
            }
        }
        "win_procedural_texture" => state.show_procedural_texture_dialog = true,
        "procedural_apply_active" => state.apply_procedural_texture_as_active(),
        "save_preferences" => { state.save_preferences(); },
        "load_preferences" => { state.load_preferences(); },
        "open_settings" | "preferences" => { state.show_settings_dialog = true; },
        "open_welcome_wizard" => {
            state.welcome_wizard_step = 0;
            state.show_welcome_wizard = true;
        },
        "win_action_wizards" => state.show_action_wizards_dialog = true,
        "map_wizard" | "win_map_wizard" => {
            state.show_map_wizard_dialog = true;
            if state.available_official_maps.is_empty() {
                state.scan_official_maps_for_wizard();
            }
        }
        "win_visual_presets" => state.show_visual_presets_dialog = true,
        "toggle_experience_mode" => {
            state.ui_experience_mode = match state.ui_experience_mode {
                crate::editor_state::UiExperienceMode::Standard => crate::editor_state::UiExperienceMode::Architect,
                crate::editor_state::UiExperienceMode::Architect => crate::editor_state::UiExperienceMode::Standard,
            };
            state.log(format!("Switched experience mode to {:?}", state.ui_experience_mode));
        }
        _ => {}
    }
}
