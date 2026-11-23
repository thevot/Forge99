//! Responsive Multi-Tier & Atomic Cluster Toolbar Engine (`gui/toolbar.rs`)
//!
//! Ensures 100% of toolbar controls remain visible and accessible in 1 click regardless of
//! window resolution, aspect ratio, or OS DPI scaling (from `800px` split-screen up to `4K`).
//!
//! ### Architecture
//! 1. **Real Width Measurement**: Measures `ui.available_width()` *outside* any horizontal `ScrollArea`
//!    (avoiding `egui`'s `f32::INFINITY` inside horizontal scroll containers).
//! 2. **4 Adaptive Density Tiers (`ToolbarDensityTier`)**:
//!    - `Expanded`: Full labels with keyboard shortcuts (`Move (W)`, `3D (F1)`, `Wire (Alt+1)`).
//!    - `Standard`: Clean full labels without redundant shortcut suffixes (shortcuts remain in rich tooltips).
//!    - `Compact`: Concise labels without outer group title prefixes (`Gizmo:`, `View:`, `Shading:`, `Snap:`).
//!    - `UltraCompact`: High-density icon-first pills (icons on all segments + text label on active segment).
//! 3. **Atomic Cluster Row Wrapping (`plan_toolbar_layout`)**:
//!    - Groups the 31 toolbar controls into 7 indivisible functional clusters (`ToolbarCluster`).
//!    - Fits all 7 clusters onto a single row whenever possible (`Expanded` -> `Standard` -> `Compact`).
//!    - Otherwise selects the richest tier that fits into **2 balanced rows** (minimizing `max(row1_w, row2_w)`),
//!      or falls back to multi-row greedy wrapping on ultra-narrow viewports (`< 720pt`).

use egui::{Color32, Ui};
use crate::editor_state::EditorState;
use crate::gui::icons::{self, Icon};
use crate::gui::theme::{ProTooltipExt, SemanticPalette};

/// Adaptive visual density tier for the main editor toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolbarDensityTier {
    /// Full labels including keyboard shortcut suffixes (e.g. `Move (W)`, `Wire (Alt+1)`).
    Expanded,
    /// Full descriptive labels without inline shortcut suffixes (e.g. `Move`, `Wire`).
    Standard,
    /// Compact labels and omitted outer group header labels (`W`, `3D`, `+Add`, `Build`).
    Compact,
    /// Ultra-compact icon-first mode for narrow/split-screen windows (`< 1000pt`).
    UltraCompact,
}

impl ToolbarDensityTier {
    pub const ALL: [Self; 4] = [
        Self::Expanded,
        Self::Standard,
        Self::Compact,
        Self::UltraCompact,
    ];
}

/// Indivisible functional clusters of the main editor toolbar.
/// Wrapping only ever occurs at cluster boundaries so related controls never split across lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ToolbarCluster {
    ModeAndLaunchers = 0,
    GizmoAndTransform = 1,
    ViewportAndLayout = 2,
    ViewportShading = 3,
    SnappingControls = 4,
    CsgOperations = 5,
    BuildAndProperties = 6,
}

impl ToolbarCluster {
    pub const COUNT: usize = 7;
    pub const ALL: [Self; Self::COUNT] = [
        Self::ModeAndLaunchers,
        Self::GizmoAndTransform,
        Self::ViewportAndLayout,
        Self::ViewportShading,
        Self::SnappingControls,
        Self::CsgOperations,
        Self::BuildAndProperties,
    ];
}

/// Computed layout plan for the toolbar at a given available width.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolbarLayoutPlan {
    pub tier: ToolbarDensityTier,
    pub rows: Vec<Vec<ToolbarCluster>>,
    pub row_widths: Vec<f32>,
    pub available_width: f32,
}

/// Exact width of an `icons::icon_label_button` matching `gui/icons.rs` metrics.
fn measure_icon_btn_width(ui: &Ui, label: Option<&str>) -> f32 {
    let padding_x = 6.0_f32;
    let icon_size = 13.5_f32;
    let text_width = if let Some(txt) = label {
        let font_id = egui::FontId::proportional(11.5);
        ui.fonts(|f| f.layout_no_wrap(txt.to_string(), font_id, Color32::WHITE).size().x) + 5.0
    } else {
        0.0
    };
    (padding_x * 2.0 + icon_size + text_width).max(24.0)
}

/// Exact width of a bold `11.0px` group header label (e.g. `"Gizmo:"`).
fn measure_group_label_width(ui: &Ui, text: &str) -> f32 {
    let font_id = egui::FontId::proportional(11.0);
    ui.fonts(|f| f.layout_no_wrap(text.to_string(), font_id, Color32::WHITE).size().x) + 4.0
}

/// Width of a segmented pill frame containing buttons with `button_labels`.
fn measure_pill_width(ui: &Ui, button_labels: &[Option<&str>]) -> f32 {
    let btns_sum: f32 = button_labels.iter().map(|&lbl| measure_icon_btn_width(ui, lbl)).sum();
    let inner_spacing = (button_labels.len().saturating_sub(1) as f32) * 2.0;
    // Frame symmetric margin (2*2=4) + border stroke (1*2=2) + safety pad (4)
    btns_sum + inner_spacing + 10.0
}

/// Computes the exact width (in logical points) required by each of the 7 [`ToolbarCluster`]s
/// under the given [`ToolbarDensityTier`].
pub fn measure_cluster_widths(
    ui: &Ui,
    state: &EditorState,
    tier: ToolbarDensityTier,
) -> [f32; ToolbarCluster::COUNT] {
    let item_gap = match tier {
        ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => 5.0_f32,
        ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => 3.5_f32,
    };
    let safety = 3.0_f32;

    // 0. ModeAndLaunchers
    let mode_lbl = match (tier, state.ui_experience_mode) {
        (ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact, crate::editor_state::UiExperienceMode::Standard) => Some("Std"),
        (ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact, crate::editor_state::UiExperienceMode::Architect) => Some("Arch"),
        (_, crate::editor_state::UiExperienceMode::Standard) => Some("Standard"),
        (_, crate::editor_state::UiExperienceMode::Architect) => Some("Architect"),
    };
    let wiz_lbl = match tier {
        ToolbarDensityTier::UltraCompact => None,
        _ => Some("Wizards"),
    };
    let pre_lbl = match tier {
        ToolbarDensityTier::UltraCompact => None,
        _ => Some("Presets"),
    };
    let c0 = measure_icon_btn_width(ui, mode_lbl)
        + measure_icon_btn_width(ui, wiz_lbl)
        + measure_icon_btn_width(ui, pre_lbl)
        + item_gap * 2.0
        + safety;

    // 1. GizmoAndTransform
    let gizmo_hdr = match tier {
        ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => {
            measure_group_label_width(ui, "Gizmo:") + item_gap
        }
        ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => 0.0,
    };
    let gizmo_pill_lbls: [Option<&str>; 3] = match tier {
        ToolbarDensityTier::Expanded => [Some("Move (W)"), Some("Rotate (E)"), Some("Scale (R)")],
        ToolbarDensityTier::Standard => [Some("Move"), Some("Rotate"), Some("Scale")],
        ToolbarDensityTier::Compact => [Some("W"), Some("E"), Some("R")],
        ToolbarDensityTier::UltraCompact => [
            if state.gizmo_mode == crate::raycast::GizmoMode::Translate { Some("W") } else { None },
            if state.gizmo_mode == crate::raycast::GizmoMode::Rotate { Some("E") } else { None },
            if state.gizmo_mode == crate::raycast::GizmoMode::Scale { Some("R") } else { None },
        ],
    };
    let space_lbl = match (tier, state.gizmo_space) {
        (ToolbarDensityTier::Expanded, crate::editor_state::GizmoCoordinateSpace::World) => Some("World (Ctrl+L)"),
        (ToolbarDensityTier::Expanded, crate::editor_state::GizmoCoordinateSpace::Local) => Some("Local (Ctrl+L)"),
        (ToolbarDensityTier::UltraCompact, _) => None,
        (_, crate::editor_state::GizmoCoordinateSpace::World) => Some("World"),
        (_, crate::editor_state::GizmoCoordinateSpace::Local) => Some("Local"),
    };
    let lock_lbl = match (tier, state.selection_locked) {
        (ToolbarDensityTier::Expanded, true) => Some("Locked (Shift+Space)"),
        (ToolbarDensityTier::Expanded, false) => Some("Unlock"),
        (ToolbarDensityTier::Standard, true) => Some("Locked"),
        (ToolbarDensityTier::Standard, false) => Some("Unlock"),
        (ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact, _) => None,
    };
    let c1 = gizmo_hdr
        + measure_pill_width(ui, &gizmo_pill_lbls)
        + measure_icon_btn_width(ui, space_lbl)
        + measure_icon_btn_width(ui, lock_lbl)
        + item_gap * 2.0
        + safety;

    // 2. ViewportAndLayout (includes Radar toggle)
    let view_hdr = match tier {
        ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => {
            measure_group_label_width(ui, "View:") + item_gap
        }
        ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => 0.0,
    };
    let vt = state.active_viewport_type;
    let view_pill_lbls: [Option<&str>; 4] = match tier {
        ToolbarDensityTier::Expanded => [
            Some("3D (F1)"),
            Some("Top (F2)"),
            Some("Front (F3)"),
            Some("Side (Shift+F4)"),
        ],
        ToolbarDensityTier::Standard => [Some("3D"), Some("Top"), Some("Front"), Some("Side")],
        ToolbarDensityTier::Compact => [Some("3D"), Some("Top"), Some("Frt"), Some("Side")],
        ToolbarDensityTier::UltraCompact => [
            if vt == crate::camera::ViewportType::Perspective { Some("3D") } else { None },
            if vt == crate::camera::ViewportType::Top { Some("Top") } else { None },
            if vt == crate::camera::ViewportType::Front { Some("Frt") } else { None },
            if vt == crate::camera::ViewportType::Side { Some("Side") } else { None },
        ],
    };
    let layout_lbl = match (tier, state.viewport_layout) {
        (ToolbarDensityTier::Expanded, crate::camera::ViewportLayout::FourViewports) => Some("Single (Space)"),
        (ToolbarDensityTier::Expanded, crate::camera::ViewportLayout::Single) => Some("2x2 (Space)"),
        (ToolbarDensityTier::UltraCompact, _) => None,
        (_, crate::camera::ViewportLayout::FourViewports) => Some("Single"),
        (_, crate::camera::ViewportLayout::Single) => Some("2x2"),
    };
    let radar_lbl = match tier {
        ToolbarDensityTier::Expanded => Some("Radar (M)"),
        ToolbarDensityTier::Standard => Some("Radar"),
        ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => None,
    };
    let c2 = view_hdr
        + measure_pill_width(ui, &view_pill_lbls)
        + measure_icon_btn_width(ui, layout_lbl)
        + measure_icon_btn_width(ui, radar_lbl)
        + item_gap * 2.0
        + safety;

    // 3. ViewportShading
    let shading_hdr = match tier {
        ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => {
            measure_group_label_width(ui, "Shading:") + item_gap
        }
        ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => 0.0,
    };
    let sh = state.viewport_shading;
    let shading_pill_lbls: [Option<&str>; 7] = match tier {
        ToolbarDensityTier::Expanded => [
            Some("Wire (Alt+1)"),
            Some("Zone (Alt+2)"),
            Some("Unlit (Alt+3)"),
            Some("Lit (Alt+4)"),
            Some("Depth (Alt+5)"),
            Some("Tex (Alt+6)"),
            Some("Cuts (Alt+7)"),
        ],
        ToolbarDensityTier::Standard | ToolbarDensityTier::Compact => [
            Some("Wire"),
            Some("Zone"),
            Some("Unlit"),
            Some("Lit"),
            Some("Depth"),
            Some("Tex"),
            Some("Cuts"),
        ],
        ToolbarDensityTier::UltraCompact => [
            if sh == crate::camera::ViewportShading::Wireframe { Some("Wire") } else { None },
            if sh == crate::camera::ViewportShading::ZonePortal { Some("Zone") } else { None },
            if sh == crate::camera::ViewportShading::Unlit { Some("Unlit") } else { None },
            if sh == crate::camera::ViewportShading::Lit { Some("Lit") } else { None },
            if sh == crate::camera::ViewportShading::DepthComplexity { Some("Depth") } else { None },
            if sh == crate::camera::ViewportShading::TextureUsage { Some("Tex") } else { None },
            if sh == crate::camera::ViewportShading::BspCuts { Some("Cuts") } else { None },
        ],
    };
    let c3 = shading_hdr + measure_pill_width(ui, &shading_pill_lbls) + safety;

    // 4. SnappingControls
    let snap_hdr = match tier {
        ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => {
            measure_group_label_width(ui, "Snap:") + item_gap
        }
        ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => 0.0,
    };
    let (grid_cb_lbl, rot_cb_lbl) = match tier {
        ToolbarDensityTier::UltraCompact => ("G", "R"),
        _ => ("Grid", "Rot"),
    };
    let cb_grid_w = 22.0 + measure_group_label_width(ui, grid_cb_lbl);
    let cb_rot_w = 22.0 + measure_group_label_width(ui, rot_cb_lbl);
    let drag_w = 48.0;
    let c4 = snap_hdr + cb_grid_w + drag_w + cb_rot_w + drag_w + item_gap * 3.0 + safety;

    // 5. CsgOperations
    let builder_diag = state.audit_builder_brush_csg_safety();
    let non_convex_w = if !builder_diag.is_convex {
        let nc_lbl = match tier {
            ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => Some("Non-Convex"),
            ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => Some("!Convex"),
        };
        measure_icon_btn_width(ui, nc_lbl) + item_gap
    } else {
        0.0
    };
    let (add_lbl, sub_lbl) = match tier {
        ToolbarDensityTier::Expanded => (Some("+ Add (Ctrl+A)"), Some("− Sub (Ctrl+S)")),
        ToolbarDensityTier::Standard => (Some("+ Add"), Some("− Sub")),
        ToolbarDensityTier::Compact => (Some("+Add"), Some("−Sub")),
        ToolbarDensityTier::UltraCompact => (Some("+"), Some("−")),
    };
    let c5 = non_convex_w
        + measure_icon_btn_width(ui, add_lbl)
        + measure_icon_btn_width(ui, sub_lbl)
        + item_gap
        + safety;

    // 6. BuildAndProperties
    let (b_all_lbl, b_lit_lbl, b_pth_lbl, surf_lbl, act_lbl, rad_lbl) = match tier {
        ToolbarDensityTier::Expanded => (
            Some("Build All"),
            Some("Lights"),
            Some("Paths"),
            Some("Surface (F5)"),
            Some("Actor (Enter)"),
            Some("Radii"),
        ),
        ToolbarDensityTier::Standard => (
            Some("Build All"),
            Some("Lights"),
            Some("Paths"),
            Some("Surface"),
            Some("Actor"),
            Some("Radii"),
        ),
        ToolbarDensityTier::Compact => (
            Some("Build"),
            Some("Lights"),
            Some("Paths"),
            Some("Surf"),
            Some("Actor"),
            None,
        ),
        ToolbarDensityTier::UltraCompact => (
            None,
            None,
            None,
            None,
            None,
            None,
        ),
    };
    let c6 = measure_icon_btn_width(ui, b_all_lbl)
        + measure_icon_btn_width(ui, b_lit_lbl)
        + measure_icon_btn_width(ui, b_pth_lbl)
        + measure_icon_btn_width(ui, surf_lbl)
        + measure_icon_btn_width(ui, act_lbl)
        + measure_icon_btn_width(ui, rad_lbl)
        + item_gap * 5.0
        + safety;

    [c0, c1, c2, c3, c4, c5, c6]
}

/// Computes the total width of a slice of clusters separated by vertical separators.
fn slice_row_width(widths: &[f32], sep_width: f32) -> f32 {
    if widths.is_empty() {
        return 0.0;
    }
    let sum: f32 = widths.iter().sum();
    let seps = (widths.len().saturating_sub(1) as f32) * sep_width;
    sum + seps
}

/// Partitions the 7 toolbar clusters into rows for a given `tier` and `avail_width`.
///
/// - If all 7 fit in 1 row (`<= avail_width`), returns 1 row.
/// - If a 2-row split exists where both rows are `<= avail_width`, chooses the split point `1..7`
///   that minimizes `row1_width.max(row2_width)` (balanced 2-row layout).
/// - Otherwise uses greedy row-by-row packing so every multi-cluster row respects `<= avail_width`.
pub fn partition_clusters_for_width(
    cluster_widths: &[f32; ToolbarCluster::COUNT],
    avail_width: f32,
) -> (Vec<Vec<ToolbarCluster>>, Vec<f32>) {
    let sep_w = 11.0_f32;
    let all_clusters = ToolbarCluster::ALL;

    // 1. Check single row fit
    let single_w = slice_row_width(cluster_widths, sep_w);
    if single_w <= avail_width {
        return (vec![all_clusters.to_vec()], vec![single_w]);
    }

    // 2. Check balanced 2-row split
    let mut best_split: Option<(usize, f32, f32)> = None;
    for split_idx in 1..ToolbarCluster::COUNT {
        let w1 = slice_row_width(&cluster_widths[..split_idx], sep_w);
        let w2 = slice_row_width(&cluster_widths[split_idx..], sep_w);
        if w1 <= avail_width && w2 <= avail_width {
            let max_w = w1.max(w2);
            let is_better = match best_split {
                None => true,
                Some((_, prev_w1, prev_w2)) => max_w < prev_w1.max(prev_w2),
            };
            if is_better {
                best_split = Some((split_idx, w1, w2));
            }
        }
    }

    if let Some((split_idx, w1, w2)) = best_split {
        let row1 = all_clusters[..split_idx].to_vec();
        let row2 = all_clusters[split_idx..].to_vec();
        return (vec![row1, row2], vec![w1, w2]);
    }

    // 3. Greedy multi-row fallback for very narrow windows
    let mut rows: Vec<Vec<ToolbarCluster>> = Vec::new();
    let mut row_widths: Vec<f32> = Vec::new();
    let mut cur_row: Vec<ToolbarCluster> = Vec::new();
    let mut cur_w = 0.0_f32;

    for (i, &cluster) in all_clusters.iter().enumerate() {
        let cw = cluster_widths[i];
        let next_w = if cur_row.is_empty() { cw } else { cur_w + sep_w + cw };
        if !cur_row.is_empty() && next_w > avail_width {
            rows.push(cur_row);
            row_widths.push(cur_w);
            cur_row = vec![cluster];
            cur_w = cw;
        } else {
            cur_row.push(cluster);
            cur_w = next_w;
        }
    }
    if !cur_row.is_empty() {
        rows.push(cur_row);
        row_widths.push(cur_w);
    }

    (rows, row_widths)
}

/// Selects the optimal [`ToolbarDensityTier`] and row partitioning for `avail_width`.
///
/// Strategy:
/// 1. First attempts to fit all 7 clusters onto a **single row** using `Expanded`, `Standard`, or `Compact`.
/// 2. If a single row cannot hold all 7 clusters, selects the richest tier (`Expanded` -> `Standard` -> `Compact` -> `UltraCompact`)
///    that fits all 7 clusters into **at most 2 balanced rows** within `avail_width`.
/// 3. For ultra-narrow viewports where even `UltraCompact` needs >2 rows, uses `UltraCompact` greedy multi-row.
pub fn plan_toolbar_layout(ui: &Ui, state: &EditorState, avail_width: f32) -> ToolbarLayoutPlan {
    let effective_avail = (avail_width - 8.0).max(200.0);

    // Pass 1: Can we fit everything on 1 single row in Expanded, Standard, or Compact?
    for tier in [
        ToolbarDensityTier::Expanded,
        ToolbarDensityTier::Standard,
        ToolbarDensityTier::Compact,
    ] {
        let widths = measure_cluster_widths(ui, state, tier);
        let (rows, row_widths) = partition_clusters_for_width(&widths, effective_avail);
        if rows.len() == 1 {
            return ToolbarLayoutPlan {
                tier,
                rows,
                row_widths,
                available_width: avail_width,
            };
        }
    }

    // Pass 2: Pick the richest tier that fits cleanly into 2 balanced rows (every row <= effective_avail)
    for tier in ToolbarDensityTier::ALL {
        let widths = measure_cluster_widths(ui, state, tier);
        let (rows, row_widths) = partition_clusters_for_width(&widths, effective_avail);
        if rows.len() <= 2 && row_widths.iter().all(|&w| w <= effective_avail) {
            return ToolbarLayoutPlan {
                tier,
                rows,
                row_widths,
                available_width: avail_width,
            };
        }
    }

    // Pass 3: Ultra-narrow fallback (< ~750pt) using UltraCompact multi-row
    let widths = measure_cluster_widths(ui, state, ToolbarDensityTier::UltraCompact);
    let (rows, row_widths) = partition_clusters_for_width(&widths, effective_avail);
    ToolbarLayoutPlan {
        tier: ToolbarDensityTier::UltraCompact,
        rows,
        row_widths,
        available_width: avail_width,
    }
}

/// Renders the responsive main toolbar (`sub_toolbar`) panel.
pub fn render_sub_toolbar(ctx: &egui::Context, state: &mut EditorState) {
    egui::TopBottomPanel::top("sub_toolbar").show(ctx, |ui| {
        let palette = SemanticPalette::current(ui.ctx());
        // Measure true available width directly from the panel (never inside a ScrollArea)
        let avail_width = ui.available_width();
        let plan = plan_toolbar_layout(ui, state, avail_width);

        ui.spacing_mut().item_spacing.y = 3.0;
        for row_clusters in &plan.rows {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = match plan.tier {
                    ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => 5.0,
                    ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => 3.5,
                };
                for (c_idx, &cluster) in row_clusters.iter().enumerate() {
                    if c_idx > 0 {
                        ui.separator();
                    }
                    render_toolbar_cluster(ui, state, &palette, plan.tier, cluster);
                }
            });
        }
    });
}

pub fn render_toolbar_cluster(
    ui: &mut Ui,
    state: &mut EditorState,
    palette: &SemanticPalette,
    tier: ToolbarDensityTier,
    cluster: ToolbarCluster,
) {
    let show_group_headers = matches!(
        tier,
        ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard
    );

    match cluster {
        ToolbarCluster::ModeAndLaunchers => {
            // UI Experience Mode Toggle (Standard vs Architect)
            let (mode_ico, mode_lbl, mode_tip) = match state.ui_experience_mode {
                crate::editor_state::UiExperienceMode::Standard => (
                    Icon::TargetFocus,
                    if matches!(tier, ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact) {
                        "Std"
                    } else {
                        "Standard"
                    },
                    "Standard Mode: Streamlined interface with essential tools, guided wizards, and progressive disclosure. Click to switch to Architect mode.",
                ),
                crate::editor_state::UiExperienceMode::Architect => (
                    Icon::Sliders,
                    if matches!(tier, ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact) {
                        "Arch"
                    } else {
                        "Architect"
                    },
                    "Architect Mode: Full low-level technical access to all 13 primitives, 100+ studios, math evaluator, and raw vertex tools. Click to switch to Standard mode.",
                ),
            };
            if icons::icon_label_button(
                ui,
                mode_ico,
                Some(mode_lbl),
                state.ui_experience_mode == crate::editor_state::UiExperienceMode::Architect,
            )
            .pro_tooltip("UI Experience Mode", None, mode_tip)
            .clicked()
            {
                state.ui_experience_mode = match state.ui_experience_mode {
                    crate::editor_state::UiExperienceMode::Standard => {
                        crate::editor_state::UiExperienceMode::Architect
                    }
                    crate::editor_state::UiExperienceMode::Architect => {
                        crate::editor_state::UiExperienceMode::Standard
                    }
                };
            }

            // Quick Wizards & Presets Launchers (Vector Icons)
            let wiz_lbl = if tier == ToolbarDensityTier::UltraCompact {
                None
            } else {
                Some("Wizards")
            };
            if icons::icon_label_button(ui, Icon::Sparkles, wiz_lbl, state.show_action_wizards_dialog)
                .pro_tooltip(
                    "Creation Hub & Wizards",
                    Some("Ctrl+Shift+A"),
                    "Guided step-by-step setup for complex game elements: Movers, Teleporters, Jump Pads, Basins, Rooms",
                )
                .clicked()
            {
                state.show_action_wizards_dialog = !state.show_action_wizards_dialog;
            }

            let pre_lbl = if tier == ToolbarDensityTier::UltraCompact {
                None
            } else {
                Some("Presets")
            };
            if icons::icon_label_button(ui, Icon::PresetCatalog, pre_lbl, state.show_visual_presets_dialog)
                .pro_tooltip(
                    "Visual Presets Catalog",
                    None,
                    "1-Click curated lighting themes, surface material presets, and gameplay combat stations",
                )
                .clicked()
            {
                state.show_visual_presets_dialog = !state.show_visual_presets_dialog;
            }
        }

        ToolbarCluster::GizmoAndTransform => {
            if show_group_headers {
                ui.label(
                    egui::RichText::new("Gizmo:")
                        .strong()
                        .size(11.0)
                        .color(palette.text_secondary),
                );
            }
            {
                let frame = egui::Frame::NONE
                    .fill(palette.bg_canvas)
                    .stroke(egui::Stroke::new(1.0_f32, palette.border_subtle))
                    .corner_radius(egui::CornerRadius::same(5))
                    .inner_margin(egui::Margin::symmetric(2, 2));
                frame.show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let snap_status = if state.snap_to_grid {
                        format!("Grid Snap: {:.0} UU", state.grid_step)
                    } else {
                        "Grid Snap: Off".to_string()
                    };
                    let modes = [
                        (
                            Icon::GizmoTranslate,
                            match tier {
                                ToolbarDensityTier::Expanded => Some("Move (W)"),
                                ToolbarDensityTier::Standard => Some("Move"),
                                ToolbarDensityTier::Compact => Some("W"),
                                ToolbarDensityTier::UltraCompact => {
                                    if state.gizmo_mode == crate::raycast::GizmoMode::Translate {
                                        Some("W")
                                    } else {
                                        None
                                    }
                                }
                            },
                            "Translate Gizmo",
                            Some("W"),
                            "Translate selected objects along X, Y, or Z axes",
                            "Alt+Drag: Instant Duplicate | Shift+Click: Multi-Select",
                            crate::raycast::GizmoMode::Translate,
                        ),
                        (
                            Icon::GizmoRotate,
                            match tier {
                                ToolbarDensityTier::Expanded => Some("Rotate (E)"),
                                ToolbarDensityTier::Standard => Some("Rotate"),
                                ToolbarDensityTier::Compact => Some("E"),
                                ToolbarDensityTier::UltraCompact => {
                                    if state.gizmo_mode == crate::raycast::GizmoMode::Rotate {
                                        Some("E")
                                    } else {
                                        None
                                    }
                                }
                            },
                            "Rotate Gizmo",
                            Some("E"),
                            "Rotate selected objects around yaw, pitch, or roll",
                            "Angle Snap: 15° / 45° | Ctrl+L: World/Local",
                            crate::raycast::GizmoMode::Rotate,
                        ),
                        (
                            Icon::GizmoScale,
                            match tier {
                                ToolbarDensityTier::Expanded => Some("Scale (R)"),
                                ToolbarDensityTier::Standard => Some("Scale"),
                                ToolbarDensityTier::Compact => Some("R"),
                                ToolbarDensityTier::UltraCompact => {
                                    if state.gizmo_mode == crate::raycast::GizmoMode::Scale {
                                        Some("R")
                                    } else {
                                        None
                                    }
                                }
                            },
                            "Scale Gizmo",
                            Some("R"),
                            "Scale selected brush along axes",
                            "Uniform or Per-Axis Brush Scaling",
                            crate::raycast::GizmoMode::Scale,
                        ),
                    ];
                    for (ico, lbl, tip_title, sc, tip_desc, mods_hint, mode) in modes {
                        let is_sel = state.gizmo_mode == mode;
                        if icons::icon_label_button(ui, ico, lbl, is_sel)
                            .pro_tooltip_ext(tip_title, sc, tip_desc, Some(mods_hint), Some(&snap_status))
                            .clicked()
                            && !is_sel
                        {
                            state.gizmo_mode = mode;
                        }
                    }
                });
            }

            // Coordinate Space (World vs Local)
            {
                let is_local = state.gizmo_space == crate::editor_state::GizmoCoordinateSpace::Local;
                let (space_lbl, space_tip) = match state.gizmo_space {
                    crate::editor_state::GizmoCoordinateSpace::World => (
                        match tier {
                            ToolbarDensityTier::Expanded => Some("World (Ctrl+L)"),
                            ToolbarDensityTier::UltraCompact => None,
                            _ => Some("World"),
                        },
                        "Gizmo transforms along absolute map axes (World Space). Click or Ctrl+L to switch to Local Space.",
                    ),
                    crate::editor_state::GizmoCoordinateSpace::Local => (
                        match tier {
                            ToolbarDensityTier::Expanded => Some("Local (Ctrl+L)"),
                            ToolbarDensityTier::UltraCompact => None,
                            _ => Some("Local"),
                        },
                        "Gizmo transforms along object's rotated orientation (Local Space). Click or Ctrl+L to switch to World Space.",
                    ),
                };
                if icons::icon_label_button(ui, Icon::Compass, space_lbl, is_local)
                    .pro_tooltip("Coordinate Space", Some("Ctrl+L"), space_tip)
                    .clicked()
                {
                    state.toggle_gizmo_space();
                }
            }

            // Selection Lock Toggle (LOCK)
            {
                let is_locked = state.selection_locked;
                let (lock_ico, lock_lbl, lock_tip) = if is_locked {
                    (
                        Icon::LockClosed,
                        match tier {
                            ToolbarDensityTier::Expanded => Some("Locked (Shift+Space)"),
                            ToolbarDensityTier::Standard => Some("Locked"),
                            ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => None,
                        },
                        "Selection is LOCKED. Viewport clicks cannot change selection or start direct drags. Click or Shift+Space to unlock.",
                    )
                } else {
                    (
                        Icon::LockOpen,
                        match tier {
                            ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => Some("Unlock"),
                            ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => None,
                        },
                        "Selection is UNLOCKED. Viewport picking and direct drag active. Click or Shift+Space to lock.",
                    )
                };
                if icons::icon_label_button(ui, lock_ico, lock_lbl, is_locked)
                    .pro_tooltip("Selection Lock", Some("Shift+Space"), lock_tip)
                    .clicked()
                {
                    state.toggle_selection_lock();
                }
            }
        }

        ToolbarCluster::ViewportAndLayout => {
            if show_group_headers {
                ui.label(
                    egui::RichText::new("View:")
                        .strong()
                        .size(11.0)
                        .color(palette.text_secondary),
                );
            }
            {
                let frame = egui::Frame::NONE
                    .fill(palette.bg_canvas)
                    .stroke(egui::Stroke::new(1.0_f32, palette.border_subtle))
                    .corner_radius(egui::CornerRadius::same(5))
                    .inner_margin(egui::Margin::symmetric(2, 2));
                frame.show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let views = [
                        (
                            Icon::ViewPerspective,
                            match tier {
                                ToolbarDensityTier::Expanded => Some("3D (F1)"),
                                ToolbarDensityTier::UltraCompact => {
                                    if state.active_viewport_type == crate::camera::ViewportType::Perspective {
                                        Some("3D")
                                    } else {
                                        None
                                    }
                                }
                                _ => Some("3D"),
                            },
                            "Perspective 3D View",
                            Some("F1"),
                            "Switch active viewport to 3D perspective camera view",
                            crate::camera::ViewportType::Perspective,
                        ),
                        (
                            Icon::ViewTop,
                            match tier {
                                ToolbarDensityTier::Expanded => Some("Top (F2)"),
                                ToolbarDensityTier::UltraCompact => {
                                    if state.active_viewport_type == crate::camera::ViewportType::Top {
                                        Some("Top")
                                    } else {
                                        None
                                    }
                                }
                                _ => Some("Top"),
                            },
                            "2D Top View (XY)",
                            Some("F2"),
                            "Switch active viewport to 2D orthographic top view",
                            crate::camera::ViewportType::Top,
                        ),
                        (
                            Icon::ViewFront,
                            match tier {
                                ToolbarDensityTier::Expanded => Some("Front (F3)"),
                                ToolbarDensityTier::Standard => Some("Front"),
                                ToolbarDensityTier::Compact => Some("Frt"),
                                ToolbarDensityTier::UltraCompact => {
                                    if state.active_viewport_type == crate::camera::ViewportType::Front {
                                        Some("Frt")
                                    } else {
                                        None
                                    }
                                }
                            },
                            "2D Front View (XZ)",
                            Some("F3"),
                            "Switch active viewport to 2D orthographic front view",
                            crate::camera::ViewportType::Front,
                        ),
                        (
                            Icon::ViewSide,
                            match tier {
                                ToolbarDensityTier::Expanded => Some("Side (Shift+F4)"),
                                ToolbarDensityTier::UltraCompact => {
                                    if state.active_viewport_type == crate::camera::ViewportType::Side {
                                        Some("Side")
                                    } else {
                                        None
                                    }
                                }
                                _ => Some("Side"),
                            },
                            "2D Side View (YZ)",
                            Some("Shift+F4"),
                            "Switch active viewport to 2D orthographic side view",
                            crate::camera::ViewportType::Side,
                        ),
                    ];
                    for (ico, lbl, tip_title, sc, tip_desc, vt) in views {
                        let is_sel = state.active_viewport_type == vt;
                        if icons::icon_label_button(ui, ico, lbl, is_sel)
                            .pro_tooltip(tip_title, sc, tip_desc)
                            .clicked()
                            && !is_sel
                        {
                            state.active_viewport_type = vt;
                        }
                    }
                });
            }

            let (layout_ico, layout_lbl) = if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
                (
                    Icon::LayoutSingle,
                    match tier {
                        ToolbarDensityTier::Expanded => Some("Single (Space)"),
                        ToolbarDensityTier::UltraCompact => None,
                        _ => Some("Single"),
                    },
                )
            } else {
                (
                    Icon::LayoutQuad,
                    match tier {
                        ToolbarDensityTier::Expanded => Some("2x2 (Space)"),
                        ToolbarDensityTier::UltraCompact => None,
                        _ => Some("2x2"),
                    },
                )
            };
            if icons::icon_label_button(ui, layout_ico, layout_lbl, false)
                .pro_tooltip(
                    "Viewport Layout",
                    Some("Space"),
                    "Toggle between single maximized viewport and 2x2 four-viewport grid",
                )
                .clicked()
            {
                state.viewport_layout = match state.viewport_layout {
                    crate::camera::ViewportLayout::Single => crate::camera::ViewportLayout::FourViewports,
                    crate::camera::ViewportLayout::FourViewports => crate::camera::ViewportLayout::Single,
                };
            }

            // Mini-Map Radar Navigation (M)
            let radar_lbl = match tier {
                ToolbarDensityTier::Expanded => Some("Radar (M)"),
                ToolbarDensityTier::Standard => Some("Radar"),
                ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => None,
            };
            if icons::icon_label_button(ui, Icon::Radar, radar_lbl, state.show_minimap)
                .pro_tooltip(
                    "Minimap Radar Overlay",
                    Some("M"),
                    "Toggle 2D overview tactical minimap radar overlay",
                )
                .clicked()
            {
                state.show_minimap = !state.show_minimap;
            }
        }

        ToolbarCluster::ViewportShading => {
            if show_group_headers {
                ui.label(
                    egui::RichText::new("Shading:")
                        .strong()
                        .size(11.0)
                        .color(palette.text_secondary),
                );
            }
            {
                let frame = egui::Frame::NONE
                    .fill(palette.bg_canvas)
                    .stroke(egui::Stroke::new(1.0_f32, palette.border_subtle))
                    .corner_radius(egui::CornerRadius::same(5))
                    .inner_margin(egui::Margin::symmetric(2, 2));
                frame.show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let shadings = [
                        (
                            Icon::ShadingWireframe,
                            "Wire (Alt+1)",
                            "Wire",
                            "Wireframe Mode",
                            Some("Alt+1"),
                            "Display wireframe geometry mesh lines without textures",
                            crate::camera::ViewportShading::Wireframe,
                        ),
                        (
                            Icon::ShadingZonePortal,
                            "Zone (Alt+2)",
                            "Zone",
                            "Zone / Portal Mode",
                            Some("Alt+2"),
                            "Highlight zone portals (cyan) and distinct room zones",
                            crate::camera::ViewportShading::ZonePortal,
                        ),
                        (
                            Icon::ShadingUnlit,
                            "Unlit (Alt+3)",
                            "Unlit",
                            "Unlit Fullbright",
                            Some("Alt+3"),
                            "Fullbright textured surface rendering without dynamic lighting",
                            crate::camera::ViewportShading::Unlit,
                        ),
                        (
                            Icon::ShadingLit,
                            "Lit (Alt+4)",
                            "Lit",
                            "Lit Mode",
                            Some("Alt+4"),
                            "Realistic lighting with dynamic point lights, radiosity and vertex shading",
                            crate::camera::ViewportShading::Lit,
                        ),
                        (
                            Icon::ShadingDepthComplexity,
                            "Depth (Alt+5)",
                            "Depth",
                            "Depth Complexity",
                            Some("Alt+5"),
                            "Overdraw and BSP depth complexity heatmap inspection (Green = low, Red = high)",
                            crate::camera::ViewportShading::DepthComplexity,
                        ),
                        (
                            Icon::ShadingTexUsage,
                            "Tex (Alt+6)",
                            "Tex",
                            "Texture Usage Mode",
                            Some("Alt+6"),
                            "Color-code BSP surfaces by texture package and material",
                            crate::camera::ViewportShading::TextureUsage,
                        ),
                        (
                            Icon::ShadingBspCuts,
                            "Cuts (Alt+7)",
                            "Cuts",
                            "BSP Cuts Diagnostic",
                            Some("Alt+7"),
                            "Highlight BSP tree split cuts and polygon fragmentation",
                            crate::camera::ViewportShading::BspCuts,
                        ),
                    ];
                    for (ico, full_lbl, short_lbl, tip_title, sc, tip_desc, mode) in shadings {
                        let is_sel = state.viewport_shading == mode;
                        let lbl = match tier {
                            ToolbarDensityTier::Expanded => Some(full_lbl),
                            ToolbarDensityTier::Standard | ToolbarDensityTier::Compact => Some(short_lbl),
                            ToolbarDensityTier::UltraCompact => {
                                if is_sel {
                                    Some(short_lbl)
                                } else {
                                    None
                                }
                            }
                        };
                        if icons::icon_label_button(ui, ico, lbl, is_sel)
                            .pro_tooltip(tip_title, sc, tip_desc)
                            .clicked()
                            && !is_sel
                        {
                            state.viewport_shading = mode;
                        }
                    }
                });
            }
        }

        ToolbarCluster::SnappingControls => {
            if show_group_headers {
                ui.label(
                    egui::RichText::new("Snap:")
                        .strong()
                        .size(11.0)
                        .color(palette.text_secondary),
                );
            }
            let (grid_lbl, rot_lbl) = match tier {
                ToolbarDensityTier::UltraCompact => ("G", "R"),
                _ => ("Grid", "Rot"),
            };
            ui.checkbox(&mut state.snap_to_grid, grid_lbl)
                .pro_tooltip("Grid Snapping", Some("Ctrl+G"), "Snap brush vertices and actor positions to grid step");
            let mut gs = state.grid_step;
            if ui
                .add(egui::DragValue::new(&mut gs).speed(1.0).range(1.0..=512.0).suffix("u"))
                .pro_tooltip("Grid Step", Some("[ / ]"), "Grid step size in Unreal Units (1 to 512 uu)")
                .changed()
            {
                state.set_grid_step(gs);
            }

            ui.checkbox(&mut state.snap_to_rotation, rot_lbl)
                .pro_tooltip("Rotation Snapping", None, "Snap actor and brush rotations to fixed degree intervals");
            ui.add(egui::DragValue::new(&mut state.rotation_snap_deg).speed(1.0).range(1.0..=90.0).suffix("°"))
                .pro_tooltip("Rotation Snap Interval", None, "Rotation snap interval angle (1° to 90°)");
        }

        ToolbarCluster::CsgOperations => {
            let builder_diag = state.audit_builder_brush_csg_safety();
            if !builder_diag.is_convex {
                let nc_lbl = match tier {
                    ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => "Non-Convex",
                    ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => "!Convex",
                };
                icons::icon_label_button(ui, Icon::AlertTriangle, Some(nc_lbl), true).on_hover_text(format!(
                    "{}\n\nLevel Design Tip: Decompose into convex components or use Intersect / Deintersect to prevent BSP holes and HOMs.",
                    builder_diag.summary_message
                ));
            }

            let add_lbl = match tier {
                ToolbarDensityTier::Expanded => "+ Add (Ctrl+A)",
                ToolbarDensityTier::Standard => "+ Add",
                ToolbarDensityTier::Compact => "+Add",
                ToolbarDensityTier::UltraCompact => "+",
            };
            let add_tooltip = if builder_diag.is_convex {
                "Add active red builder brush to world geometry (Convex & Watertight)"
            } else {
                "Add active red builder brush to world geometry\nWARNING: Builder brush is non-convex! High risk of BSP cuts and HOM errors."
            };
            if icons::icon_label_button(ui, Icon::CsgAdd, Some(add_lbl), false)
                .pro_tooltip_4level(
                    "CSG Add",
                    Some("Ctrl+A"),
                    add_tooltip,
                    Some("Shift+Click: Keep builder in place"),
                    Some("Operation: Additive Solid"),
                    Some("UnrealEd 2.0 Gotcha: Always subtract the room first, then add pillars, crates, and decorations. Never add overlapping solid brushes directly without clipping or semi-solids to prevent BSP leaf explosion."),
                )
                .clicked()
            {
                state.csg_add();
            }

            let sub_lbl = match tier {
                ToolbarDensityTier::Expanded => "− Sub (Ctrl+S)",
                ToolbarDensityTier::Standard => "− Sub",
                ToolbarDensityTier::Compact => "−Sub",
                ToolbarDensityTier::UltraCompact => "−",
            };
            let sub_tooltip = if builder_diag.is_convex {
                "Subtract active red builder brush from world geometry (Convex & Watertight)"
            } else {
                "Subtract active red builder brush from world geometry\nWARNING: Builder brush is non-convex! High risk of BSP cuts and HOM errors."
            };
            if icons::icon_label_button(ui, Icon::CsgSubtract, Some(sub_lbl), false)
                .pro_tooltip_4level(
                    "CSG Subtract",
                    Some("Ctrl+S"),
                    sub_tooltip,
                    Some("Shift+Click: Keep builder in place"),
                    Some("Operation: Subtractive Hollow"),
                    Some("UnrealEd 2.0 Gotcha: In Unreal Engine 1, the world begins as solid infinite rock. Every playable space must be carved out with Subtract before adding objects inside."),
                )
                .clicked()
            {
                state.csg_subtract();
            }
        }

        ToolbarCluster::BuildAndProperties => {
            let b_all_lbl = match tier {
                ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => Some("Build All"),
                ToolbarDensityTier::Compact => Some("Build"),
                ToolbarDensityTier::UltraCompact => None,
            };
            if icons::icon_label_button(ui, Icon::BuildAll, b_all_lbl, false)
                .pro_tooltip(
                    "Build All Complete",
                    Some("Ctrl+F8"),
                    "Execute full rebuild: Geometry + Lighting + Bot AI Paths + Decals",
                )
                .clicked()
            {
                state.build_all_complete();
            }

            let lit_lbl = match tier {
                ToolbarDensityTier::UltraCompact => None,
                _ => Some("Lights"),
            };
            if icons::icon_label_button(ui, Icon::BuildLighting, lit_lbl, false)
                .pro_tooltip_4level(
                    "Build Lighting",
                    Some("Ctrl+Shift+L"),
                    "Rebuild dynamic illumination and shadows",
                    Some("Ctrl+Shift+L: Quick Lighting Rebuild"),
                    None,
                    Some("UnrealEd 2.0 Gotcha: Never place Light actors inside solid geometry or exactly coplanar on a wall, which causes black lightmap bleeding and HOM errors."),
                )
                .clicked()
            {
                state.build_lighting();
            }

            let pth_lbl = match tier {
                ToolbarDensityTier::UltraCompact => None,
                _ => Some("Paths"),
            };
            if icons::icon_label_button(ui, Icon::BuildPaths, pth_lbl, false)
                .pro_tooltip_4level(
                    "Build Paths",
                    Some("Ctrl+Shift+P"),
                    "Generate bot navigation graph, reachability network and jump spots",
                    Some("F8: Rebuild Geometry & Paths"),
                    None,
                    Some("UnrealEd 2.0 Gotcha: PathNodes must be within 1000 Unreal Units of each other with unobstructed line-of-sight. If bots get stuck on corners, add a PathNode at the apex."),
                )
                .clicked()
            {
                state.build_ai_paths();
            }

            let surf_lbl = match tier {
                ToolbarDensityTier::Expanded => Some("Surface (F5)"),
                ToolbarDensityTier::Standard => Some("Surface"),
                ToolbarDensityTier::Compact => Some("Surf"),
                ToolbarDensityTier::UltraCompact => None,
            };
            if icons::icon_label_button(ui, Icon::SurfaceProperties, surf_lbl, state.show_surface_properties)
                .pro_tooltip_4level(
                    "Surface Properties",
                    Some("F5"),
                    "Open Surface Properties window for UV alignment, panning, scaling, and PolyFlags",
                    Some("Shift+B: Select all matching brush surfaces"),
                    None,
                    Some("UnrealEd 2.0 Gotcha: When aligning textures across multiple coplanar surfaces, use Align Wall Direction/Pan rather than manual scaling to preserve consistent texel density."),
                )
                .clicked()
            {
                state.show_surface_properties = !state.show_surface_properties;
            }

            let act_lbl = match tier {
                ToolbarDensityTier::Expanded => Some("Actor (Enter)"),
                ToolbarDensityTier::Standard | ToolbarDensityTier::Compact => Some("Actor"),
                ToolbarDensityTier::UltraCompact => None,
            };
            if icons::icon_label_button(ui, Icon::ActorProperties, act_lbl, state.show_actor_properties)
                .pro_tooltip_4level(
                    "Actor Properties",
                    Some("Enter"),
                    "Open Actor Properties window for transform, lighting, tags, and tagged properties",
                    Some("Double-click any actor in 3D viewport"),
                    None,
                    Some("UnrealEd 2.0 Gotcha: To trigger a mover or event, ensure the triggering Actor's Event matches the target Actor's Tag exactly (case-insensitive in UT99)."),
                )
                .clicked()
            {
                state.show_actor_properties = !state.show_actor_properties;
            }

            let rad_lbl = match tier {
                ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => Some("Radii"),
                ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => None,
            };
            if icons::icon_label_button(ui, Icon::RadiiView, rad_lbl, state.show_actor_radii)
                .pro_tooltip(
                    "Actor Radii View",
                    None,
                    "Toggle visual collision cylinders, light attenuation and sound spheres for all actors",
                )
                .clicked()
            {
                state.show_actor_radii = !state.show_actor_radii;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Pos2, Rect, Vec2, ViewportId, ViewportInfo};

    fn make_ctx(width: f32, height: f32) -> Context {
        let ctx = Context::default();
        let mut raw = egui::RawInput::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height));
        raw.screen_rect = Some(screen);
        raw.viewports.insert(
            ViewportId::ROOT,
            ViewportInfo {
                inner_rect: Some(screen),
                ..Default::default()
            },
        );
        let _ = ctx.run(raw, |_| {});
        ctx
    }

    #[test]
    fn test_sub_toolbar_never_swallows_viewport_and_fits_width() {
        let widths = [800.0_f32, 960.0, 1024.0, 1152.0, 1280.0, 1366.0, 1440.0, 1536.0, 1600.0, 1920.0, 2560.0];
        for &w in &widths {
            let mut state = EditorState::default();
            let ctx = make_ctx(w, 900.0);

            for frame in 0..3 {
                let mut raw = egui::RawInput::default();
                raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, 900.0)));
                let _ = ctx.run(raw, |ctx| {
                    crate::gui::render_ui(ctx, &mut state);
                    let avail = ctx.available_rect();
                    assert!(
                        avail.height() >= 700.0,
                        "width={w} frame={frame}: central viewport height collapsed to {}",
                        avail.height()
                    );
                    assert!(
                        avail.width() >= 200.0,
                        "width={w} frame={frame}: central viewport width collapsed to {}",
                        avail.width()
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        let palette = SemanticPalette::current(ui.ctx());
                        let plan = plan_toolbar_layout(ui, &state, w);
                        let flattened: Vec<ToolbarCluster> =
                            plan.rows.iter().flatten().copied().collect();
                        assert_eq!(flattened, ToolbarCluster::ALL.to_vec());
                        assert!(plan.rows.len() <= 2, "width={w}: rows={}", plan.rows.len());

                        for (r_idx, row_clusters) in plan.rows.iter().enumerate() {
                            let planned_rw = plan.row_widths[r_idx];
                            assert!(
                                planned_rw <= w,
                                "width={w} row={r_idx}: planned_rw={planned_rw} > {w}"
                            );
                            let resp = ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = match plan.tier {
                                    ToolbarDensityTier::Expanded
                                    | ToolbarDensityTier::Standard => 5.0,
                                    ToolbarDensityTier::Compact
                                    | ToolbarDensityTier::UltraCompact => 3.5,
                                };
                                for (c_idx, &cluster) in row_clusters.iter().enumerate() {
                                    if c_idx > 0 {
                                        ui.separator();
                                    }
                                    render_toolbar_cluster(
                                        ui, &mut state, &palette, plan.tier, cluster,
                                    );
                                }
                            });
                            let actual_rw = resp.response.rect.width();
                            assert!(
                                actual_rw <= w,
                                "width={w} row={r_idx} tier={:?}: actual_rw={actual_rw:.1} > {w} (planned={planned_rw:.1})",
                                plan.tier
                            );
                        }
                    });
                });
            }
        }
    }
}

