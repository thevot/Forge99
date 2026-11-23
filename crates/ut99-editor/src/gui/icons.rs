//! High-Precision Vector Icon Engine for Unreal Precision Slate Dark
//!
//! Provides subpixel anti-aliased procedural vector rendering for Gizmos,
//! Viewport layouts, Shading modes, CSG operations, Snapping, and Browsers.
//! Rendered natively via `egui::Painter` with zero external dependencies.

use egui::{pos2, Color32, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

/// Vector icon identifiers for editor tools, modes, and dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    // Gizmo Modes
    GizmoTranslate,
    GizmoRotate,
    GizmoScale,

    // Viewport Types & Layouts
    ViewPerspective,
    ViewTop,
    ViewFront,
    ViewSide,
    LayoutSingle,
    LayoutQuad,

    // Viewport Shading Modes
    ShadingWireframe,
    ShadingZonePortal,
    ShadingUnlit,
    ShadingLit,
    ShadingDepthComplexity,
    ShadingTexUsage,
    ShadingBspCuts,

    // CSG Geometry Operations
    CsgAdd,
    CsgSubtract,
    CsgIntersect,
    CsgDeintersect,
    PolyMerge,
    PolySeparate,
    InvertNormals,
    ExtractSurface,

    // Navigation & Snapping
    CameraSpeed,
    Radar,
    SnapGrid,
    SnapRotate,
    AlignFloor,
    AlignGrid,
    AlignDistribution,

    // Panels & Inspector
    Outliner,
    Inspector,
    Split,
    TargetFocus,
    Duplicate,
    FloorSnap,
    Delete,

    // Visibility & Lock
    EyeVisible,
    EyeHidden,
    LockClosed,
    LockOpen,

    // File & Clipboard
    FileNew,
    FileOpen,
    FileSave,
    FileExport,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,

    // Builder Shapes / Primitives
    ShapeCube,
    ShapeCylinder,
    ShapeCone,
    ShapeSphere,
    ShapeSheet,
    ShapeStairs,
    ShapeCurvedStairs,
    ShapeSpiralStairs,
    ShapeArch,
    ShapeTorus,
    ShapeTerrain,

    // Actors & Gameplay
    ActorLight,
    ActorPlayerStart,
    ActorPathNode,
    ActorTrigger,
    ActorTeleporter,
    ActorWeapon,
    ActorPickup,
    ActorMover,
    StaticMesh,

    // Surface & Materials
    TextureEyedropper,
    TextureFit,
    TextureAlign,
    MaterialPreset,

    // Rebuild, Studios & Compilation
    BuildLighting,
    BuildPaths,
    BuildGeometry,
    BuildAll,
    PlayGame,
    Settings,
    Wizard,
    PresetCatalog,
    LevelDoctor,

    // Browsers & Studios
    TextureBrowser,
    ActorBrowser,
    MeshBrowser,
    SoundBrowser,
    MusicBrowser,
    GroupBrowser,
    SurfaceProperties,
    ActorProperties,
    RadiiView,
    CommandPalette,

    // General UI & Diagnostics (Zero-Emoji Vector Set)
    Search,
    Filter,
    CheckCircle,
    AlertTriangle,
    InfoCircle,
    Sliders,
    Folder,
    Sparkles,
    Compass,
}

/// Paints the specified vector icon into `rect` using `color` for primary strokes.
pub fn paint_icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let center = rect.center();
    let min_dim = rect.width().min(rect.height());
    let r = (min_dim / 2.0) * 0.85;
    let stroke = Stroke::new(1.3_f32, color);
    let thin_stroke = Stroke::new(1.0_f32, color);

    match icon {
        Icon::GizmoTranslate => {
            // 4-way orthogonal coordinate cross with arrowheads
            let d = r * 0.95;
            painter.line_segment([pos2(center.x - d, center.y), pos2(center.x + d, center.y)], stroke);
            painter.line_segment([pos2(center.x, center.y - d), pos2(center.x, center.y + d)], stroke);

            // Left arrow
            painter.line_segment([pos2(center.x - d + 2.5, center.y - 2.0), pos2(center.x - d, center.y)], stroke);
            painter.line_segment([pos2(center.x - d + 2.5, center.y + 2.0), pos2(center.x - d, center.y)], stroke);
            // Right arrow
            painter.line_segment([pos2(center.x + d - 2.5, center.y - 2.0), pos2(center.x + d, center.y)], stroke);
            painter.line_segment([pos2(center.x + d - 2.5, center.y + 2.0), pos2(center.x + d, center.y)], stroke);
            // Top arrow
            painter.line_segment([pos2(center.x - 2.0, center.y - d + 2.5), pos2(center.x, center.y - d)], stroke);
            painter.line_segment([pos2(center.x + 2.0, center.y - d + 2.5), pos2(center.x, center.y - d)], stroke);
            // Bottom arrow
            painter.line_segment([pos2(center.x - 2.0, center.y + d - 2.5), pos2(center.x, center.y + d)], stroke);
            painter.line_segment([pos2(center.x + 2.0, center.y + d - 2.5), pos2(center.x, center.y + d)], stroke);

            // Center pivot dot
            painter.circle_filled(center, 1.5, color);
        }

        Icon::GizmoRotate => {
            // 270-degree rotation arc with arrowhead
            let arc_r = r * 0.85;
            let segments = 16;
            for i in 0..segments {
                let a1 = (i as f32 / segments as f32) * std::f32::consts::PI * 1.55;
                let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::PI * 1.55;
                let p1 = pos2(center.x + arc_r * a1.cos(), center.y + arc_r * a1.sin());
                let p2 = pos2(center.x + arc_r * a2.cos(), center.y + arc_r * a2.sin());
                painter.line_segment([p1, p2], stroke);
            }
            // Arrowhead at the tip of the arc
            let tip = pos2(center.x + arc_r * (std::f32::consts::PI * 1.55).cos(), center.y + arc_r * (std::f32::consts::PI * 1.55).sin());
            painter.line_segment([pos2(tip.x - 3.5, tip.y), tip], stroke);
            painter.line_segment([pos2(tip.x, tip.y + 3.5), tip], stroke);

            // Center pivot
            painter.circle_filled(center, 1.5, color);
        }

        Icon::GizmoScale => {
            // Anchor square bottom-left and scaling vector to top-right box
            let b_size = r * 0.55;
            let b_rect = Rect::from_min_size(pos2(center.x - r, center.y + r - b_size), Vec2::splat(b_size));
            painter.rect_filled(b_rect, 1.0, color.gamma_multiply(0.4));
            painter.rect_stroke(b_rect, 1.0, stroke, StrokeKind::Inside);

            let t_rect = Rect::from_min_size(pos2(center.x + r - b_size, center.y - r), Vec2::splat(b_size));
            painter.rect_filled(t_rect, 1.0, color);

            // Diagonal vector
            painter.line_segment([b_rect.center(), t_rect.center()], thin_stroke);
        }

        Icon::ViewPerspective => {
            // Isometric 3D wireframe cube
            let dx = r * 0.85;
            let dy = r * 0.50;
            let dz = r * 0.85;

            let c = center;
            let top = pos2(c.x, c.y - dz);
            let top_l = pos2(c.x - dx, c.y - dz + dy);
            let top_r = pos2(c.x + dx, c.y - dz + dy);
            let bot_m = pos2(c.x, c.y + dz - dy);
            let bot_l = pos2(c.x - dx, c.y + dy);
            let bot_r = pos2(c.x + dx, c.y + dy);

            // Top rhombus
            painter.line_segment([c, top_l], stroke);
            painter.line_segment([top_l, top], stroke);
            painter.line_segment([top, top_r], stroke);
            painter.line_segment([top_r, c], stroke);

            // Vertical front pillar
            painter.line_segment([c, bot_m], stroke);

            // Outer contour
            painter.line_segment([top_l, bot_l], stroke);
            painter.line_segment([bot_l, bot_m], stroke);
            painter.line_segment([bot_m, bot_r], stroke);
            painter.line_segment([bot_r, top_r], stroke);
        }

        Icon::ViewTop => {
            // Flat 2D orthographic top plane with compass cross
            let box_r = Rect::from_center_size(center, Vec2::splat(r * 1.6));
            painter.rect_stroke(box_r, 2.0, stroke, StrokeKind::Inside);
            painter.line_segment([pos2(center.x, box_r.min.y), pos2(center.x, box_r.max.y)], thin_stroke);
            painter.line_segment([pos2(box_r.min.x, center.y), pos2(box_r.max.x, center.y)], thin_stroke);
            painter.circle_filled(center, 2.0, color);
        }

        Icon::ViewFront => {
            // Front elevation view with horizontal groundline
            let box_r = Rect::from_center_size(center, Vec2::new(r * 1.6, r * 1.3));
            painter.rect_stroke(box_r, 2.0, stroke, StrokeKind::Inside);
            // X and Z axis ticks
            painter.line_segment([pos2(box_r.min.x + 3.0, box_r.max.y - 3.0), pos2(box_r.max.x - 3.0, box_r.max.y - 3.0)], stroke);
            painter.line_segment([pos2(box_r.min.x + 3.0, box_r.max.y - 3.0), pos2(box_r.min.x + 3.0, box_r.min.y + 3.0)], stroke);
        }

        Icon::ViewSide => {
            // Side elevation view with Y and Z indicators
            let box_r = Rect::from_center_size(center, Vec2::new(r * 1.3, r * 1.6));
            painter.rect_stroke(box_r, 2.0, stroke, StrokeKind::Inside);
            painter.line_segment([pos2(box_r.min.x + 3.0, box_r.max.y - 3.0), pos2(box_r.max.x - 3.0, box_r.max.y - 3.0)], stroke);
            painter.line_segment([pos2(box_r.min.x + 3.0, box_r.max.y - 3.0), pos2(box_r.min.x + 3.0, box_r.min.y + 3.0)], stroke);
        }

        Icon::LayoutSingle => {
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.6));
            painter.rect_filled(b, 2.0, color.gamma_multiply(0.2));
            painter.rect_stroke(b, 2.0, stroke, StrokeKind::Inside);
        }

        Icon::LayoutQuad => {
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.6));
            painter.rect_stroke(b, 2.0, stroke, StrokeKind::Inside);
            painter.line_segment([pos2(center.x, b.min.y), pos2(center.x, b.max.y)], stroke);
            painter.line_segment([pos2(b.min.x, center.y), pos2(b.max.x, center.y)], stroke);
        }

        Icon::ShadingWireframe => {
            // Wireframe mesh lines
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.5));
            painter.rect_stroke(b, 1.0, stroke, StrokeKind::Inside);
            painter.line_segment([b.min, b.max], thin_stroke);
            painter.line_segment([pos2(b.min.x, b.max.y), pos2(b.max.x, b.min.y)], thin_stroke);
        }

        Icon::ShadingZonePortal => {
            // Zone Portal Arch
            let b = Rect::from_center_size(center, Vec2::new(r * 1.4, r * 1.7));
            let cyan = Color32::from_rgb(40, 220, 240);
            painter.rect_filled(b, 3.0, cyan.gamma_multiply(0.25));
            painter.rect_stroke(b, 3.0, Stroke::new(1.3_f32, cyan), StrokeKind::Inside);
            // Portal threshold line
            painter.line_segment([pos2(b.min.x - 2.0, b.max.y), pos2(b.max.x + 2.0, b.max.y)], Stroke::new(2.0_f32, cyan));
        }

        Icon::ShadingUnlit => {
            // Fullbright sun outline
            painter.circle_stroke(center, r * 0.75, stroke);
            painter.circle_filled(center, r * 0.35, color.gamma_multiply(0.3));
        }

        Icon::ShadingLit => {
            // Lit mode: Sun with emission rays
            painter.circle_filled(center, r * 0.45, color);
            let ray_in = r * 0.65;
            let ray_out = r * 0.95;
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::PI / 4.0;
                let p1 = pos2(center.x + ray_in * angle.cos(), center.y + ray_in * angle.sin());
                let p2 = pos2(center.x + ray_out * angle.cos(), center.y + ray_out * angle.sin());
                painter.line_segment([p1, p2], stroke);
            }
        }

        Icon::ShadingDepthComplexity => {
            // 3 stacked heatmap layers
            let w = r * 1.5;
            let h = r * 0.40;
            let colors = [
                Color32::from_rgb(60, 220, 100),  // Green (low overdraw)
                Color32::from_rgb(240, 200, 40),  // Yellow (medium)
                Color32::from_rgb(240, 60, 60),   // Red (high complexity)
            ];
            for i in 0..3 {
                let y = center.y - r * 0.75 + (i as f32) * (h + 2.0);
                let bar = Rect::from_min_size(pos2(center.x - w / 2.0, y), Vec2::new(w, h));
                painter.rect_filled(bar, 1.0, colors[i]);
            }
        }

        Icon::ShadingTexUsage => {
            // 2x2 Material Usage Swatch Grid
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.5));
            let half = b.width() * 0.5;
            let q1 = Rect::from_min_size(b.min, Vec2::splat(half));
            let q2 = Rect::from_min_size(pos2(center.x, b.min.y), Vec2::splat(half));
            let q3 = Rect::from_min_size(pos2(b.min.x, center.y), Vec2::splat(half));
            let q4 = Rect::from_min_size(center, Vec2::splat(half));
            painter.rect_filled(q1, 1.0, Color32::from_rgb(90, 185, 255).gamma_multiply(0.85));
            painter.rect_filled(q2, 1.0, Color32::from_rgb(255, 165, 70).gamma_multiply(0.85));
            painter.rect_filled(q3, 1.0, Color32::from_rgb(165, 110, 255).gamma_multiply(0.85));
            painter.rect_filled(q4, 1.0, Color32::from_rgb(80, 225, 150).gamma_multiply(0.85));
            painter.rect_stroke(b, 1.5, stroke, StrokeKind::Inside);
        }

        Icon::ShadingBspCuts => {
            // Polygon split by a diagonal BSP hyperplane cut
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.5));
            painter.rect_stroke(b, 1.5, thin_stroke, StrokeKind::Inside);
            let cut_stroke = Stroke::new(1.8_f32, Color32::from_rgb(255, 110, 195));
            painter.line_segment(
                [pos2(b.min.x - 1.5, b.max.y - 2.0), pos2(b.max.x + 1.5, b.min.y + 2.0)],
                cut_stroke,
            );
            painter.circle_filled(center, 1.6, Color32::from_rgb(255, 110, 195));
        }

        Icon::CsgAdd => {
            // Solid additive box with + sign
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.5));
            let add_blue = Color32::from_rgb(70, 150, 255);
            painter.rect_filled(b, 2.0, add_blue.gamma_multiply(0.3));
            painter.rect_stroke(b, 2.0, Stroke::new(1.3_f32, add_blue), StrokeKind::Inside);

            let plus_stroke = Stroke::new(2.0_f32, Color32::WHITE);
            let len = r * 0.55;
            painter.line_segment([pos2(center.x - len, center.y), pos2(center.x + len, center.y)], plus_stroke);
            painter.line_segment([pos2(center.x, center.y - len), pos2(center.x, center.y + len)], plus_stroke);
        }

        Icon::CsgSubtract => {
            // Subtractive brush contour with − sign
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.5));
            let sub_amber = Color32::from_rgb(255, 140, 70);
            painter.rect_filled(b, 2.0, sub_amber.gamma_multiply(0.2));
            painter.rect_stroke(b, 2.0, Stroke::new(1.3_f32, sub_amber), StrokeKind::Inside);

            let minus_stroke = Stroke::new(2.0_f32, Color32::WHITE);
            let len = r * 0.55;
            painter.line_segment([pos2(center.x - len, center.y), pos2(center.x + len, center.y)], minus_stroke);
        }

        Icon::CameraSpeed => {
            // Fast-forward double chevron ▶▶
            let d = r * 0.65;
            let c1 = pos2(center.x - d * 0.5, center.y);
            let c2 = pos2(center.x + d * 0.5, center.y);

            let poly1 = vec![pos2(c1.x - d * 0.5, c1.y - d), pos2(c1.x + d * 0.5, c1.y), pos2(c1.x - d * 0.5, c1.y + d)];
            let poly2 = vec![pos2(c2.x - d * 0.5, c2.y - d), pos2(c2.x + d * 0.5, c2.y), pos2(c2.x - d * 0.5, c2.y + d)];
            painter.add(egui::Shape::convex_polygon(poly1, color, Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(poly2, color, Stroke::NONE));
        }

        Icon::Radar => {
            // Radar concentric rings + blip
            painter.circle_stroke(center, r * 0.9, thin_stroke);
            painter.circle_stroke(center, r * 0.45, thin_stroke);
            painter.line_segment([pos2(center.x - r * 0.9, center.y), pos2(center.x + r * 0.9, center.y)], thin_stroke);
            painter.line_segment([pos2(center.x, center.y - r * 0.9), pos2(center.x, center.y + r * 0.9)], thin_stroke);
            // Blip
            painter.circle_filled(pos2(center.x + r * 0.45, center.y - r * 0.35), 2.0, Color32::from_rgb(255, 180, 40));
        }

        Icon::SnapGrid => {
            // 3x3 point grid matrix
            let step = r * 0.65;
            for ix in -1..=1 {
                for iy in -1..=1 {
                    let p = pos2(center.x + (ix as f32) * step, center.y + (iy as f32) * step);
                    let col = if ix == 0 && iy == 0 { Color32::from_rgb(100, 200, 255) } else { color };
                    painter.circle_filled(p, if ix == 0 && iy == 0 { 2.0 } else { 1.2 }, col);
                }
            }
        }

        Icon::SnapRotate => {
            // Angle arc corner
            let d = r * 0.85;
            painter.line_segment([pos2(center.x - d, center.y + d), pos2(center.x + d, center.y + d)], stroke);
            painter.line_segment([pos2(center.x - d, center.y + d), pos2(center.x - d, center.y - d)], stroke);
            // Diagonal arc
            painter.line_segment([pos2(center.x - d + 4.0, center.y), pos2(center.x, center.y + d - 4.0)], thin_stroke);
            painter.circle_filled(pos2(center.x + 2.0, center.y - 2.0), 1.5, color);
        }

        Icon::Outliner => {
            // Hierarchical list lines with indent
            let w = r * 1.5;
            let y1 = center.y - r * 0.65;
            let y2 = center.y;
            let y3 = center.y + r * 0.65;

            // Line 1: Root
            painter.circle_filled(pos2(center.x - w / 2.0 + 2.0, y1), 2.0, color);
            painter.line_segment([pos2(center.x - w / 2.0 + 7.0, y1), pos2(center.x + w / 2.0, y1)], stroke);

            // Line 2: Child (indented)
            painter.circle_filled(pos2(center.x - w / 2.0 + 8.0, y2), 1.5, color);
            painter.line_segment([pos2(center.x - w / 2.0 + 13.0, y2), pos2(center.x + w / 2.0, y2)], thin_stroke);

            // Line 3: Child (indented)
            painter.circle_filled(pos2(center.x - w / 2.0 + 8.0, y3), 1.5, color);
            painter.line_segment([pos2(center.x - w / 2.0 + 13.0, y3), pos2(center.x + w / 2.0 - 4.0, y3)], thin_stroke);
        }

        Icon::Inspector => {
            // 3 vertical mixer fader tracks with handles
            let xs = [center.x - r * 0.65, center.x, center.x + r * 0.65];
            let handle_ys = [center.y - r * 0.35, center.y + r * 0.40, center.y - r * 0.10];

            for (&x, &hy) in xs.iter().zip(handle_ys.iter()) {
                painter.line_segment([pos2(x, center.y - r * 0.85), pos2(x, center.y + r * 0.85)], thin_stroke);
                let handle = Rect::from_center_size(pos2(x, hy), Vec2::new(4.5, 3.0));
                painter.rect_filled(handle, 1.0, color);
            }
        }

        Icon::Split => {
            // Split panel icon (top & bottom rects)
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.6));
            painter.rect_stroke(b, 2.0, stroke, StrokeKind::Inside);
            painter.line_segment([pos2(b.min.x, center.y), pos2(b.max.x, center.y)], Stroke::new(1.8_f32, color));
        }

        Icon::TargetFocus => {
            // Precision crosshairs
            painter.circle_stroke(center, r * 0.85, stroke);
            painter.line_segment([pos2(center.x - r, center.y), pos2(center.x - r * 0.45, center.y)], stroke);
            painter.line_segment([pos2(center.x + r * 0.45, center.y), pos2(center.x + r, center.y)], stroke);
            painter.line_segment([pos2(center.x, center.y - r), pos2(center.x, center.y - r * 0.45)], stroke);
            painter.line_segment([pos2(center.x, center.y + r * 0.45), pos2(center.x, center.y + r)], stroke);
            painter.circle_filled(center, 1.5, color);
        }

        Icon::Duplicate => {
            // Two overlapping cascading rects
            let size = Vec2::splat(r * 1.15);
            let b1 = Rect::from_min_size(pos2(center.x - r * 0.8, center.y - r * 0.8), size);
            let b2 = Rect::from_min_size(pos2(center.x - r * 0.3, center.y - r * 0.3), size);

            painter.rect_stroke(b1, 1.5, thin_stroke, StrokeKind::Inside);
            painter.rect_filled(b2, 1.5, Color32::from_rgb(25, 30, 38));
            painter.rect_stroke(b2, 1.5, stroke, StrokeKind::Inside);
        }

        Icon::FloorSnap => {
            // Downward arrow to ground plane
            let ground_y = center.y + r * 0.75;
            painter.line_segment([pos2(center.x - r * 0.9, ground_y), pos2(center.x + r * 0.9, ground_y)], Stroke::new(2.0_f32, color));

            // Downward arrow shaft
            painter.line_segment([pos2(center.x, center.y - r * 0.85), pos2(center.x, ground_y - 2.5)], stroke);
            // Arrowhead
            painter.line_segment([pos2(center.x - 3.5, ground_y - 6.0), pos2(center.x, ground_y - 2.5)], stroke);
            painter.line_segment([pos2(center.x + 3.5, ground_y - 6.0), pos2(center.x, ground_y - 2.5)], stroke);
        }

        Icon::Delete => {
            // Trash bin with lid
            let lid_y = center.y - r * 0.55;
            painter.line_segment([pos2(center.x - r * 0.75, lid_y), pos2(center.x + r * 0.75, lid_y)], stroke);
            // Lid handle
            painter.line_segment([pos2(center.x - r * 0.35, lid_y - 2.5), pos2(center.x + r * 0.35, lid_y - 2.5)], thin_stroke);

            // Bin body
            let body_poly = vec![
                pos2(center.x - r * 0.60, lid_y + 1.0),
                pos2(center.x - r * 0.45, center.y + r * 0.85),
                pos2(center.x + r * 0.45, center.y + r * 0.85),
                pos2(center.x + r * 0.60, lid_y + 1.0),
            ];
            painter.add(egui::Shape::convex_polygon(body_poly, Color32::TRANSPARENT, stroke));
        }

        Icon::BuildLighting => {
            // Lightbulb with radiating rays
            let bulb_center = pos2(center.x, center.y - 1.5);
            painter.circle_filled(bulb_center, r * 0.55, Color32::from_rgb(255, 230, 80));
            // Base
            painter.rect_filled(Rect::from_center_size(pos2(center.x, center.y + r * 0.55), Vec2::new(r * 0.5, 3.0)), 1.0, color);
        }

        Icon::BuildPaths => {
            // Bot navigation path nodes
            let p1 = pos2(center.x - r * 0.65, center.y + r * 0.45);
            let p2 = pos2(center.x + r * 0.65, center.y - r * 0.45);
            painter.line_segment([p1, p2], stroke);
            painter.circle_filled(p1, 3.0, Color32::from_rgb(80, 220, 120));
            painter.circle_filled(p2, 3.0, Color32::from_rgb(100, 200, 255));
        }

        Icon::BuildAll => {
            // Lightning build star
            let star_poly = vec![
                pos2(center.x - 1.0, center.y - r * 0.95),
                pos2(center.x + r * 0.45, center.y - r * 0.15),
                pos2(center.x, center.y - r * 0.10),
                pos2(center.x + r * 0.25, center.y + r * 0.95),
                pos2(center.x - r * 0.45, center.y + r * 0.15),
                pos2(center.x - 1.0, center.y + r * 0.10),
            ];
            painter.add(egui::Shape::convex_polygon(star_poly, Color32::from_rgb(255, 215, 40), stroke));
        }

        Icon::TextureBrowser => {
            // 2x2 checkerboard pattern
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.5));
            painter.rect_stroke(b, 2.0, stroke, StrokeKind::Inside);
            let mid = center;
            // Top-left and bottom-right filled squares
            painter.rect_filled(Rect::from_min_max(b.min, mid), 0.0, color);
            painter.rect_filled(Rect::from_min_max(mid, b.max), 0.0, color);
        }

        Icon::ActorBrowser => {
            // Humanoid pawn silhouette
            let head_center = pos2(center.x, center.y - r * 0.45);
            painter.circle_filled(head_center, r * 0.40, color);
            // Torso arc
            let torso_poly = vec![
                pos2(center.x - r * 0.70, center.y + r * 0.85),
                pos2(center.x - r * 0.35, center.y),
                pos2(center.x + r * 0.35, center.y),
                pos2(center.x + r * 0.70, center.y + r * 0.85),
            ];
            painter.add(egui::Shape::convex_polygon(torso_poly, color, Stroke::NONE));
        }

        Icon::MeshBrowser => {
            // Diamond / 3D polyhedron
            let poly = vec![
                pos2(center.x, center.y - r * 0.90),
                pos2(center.x + r * 0.85, center.y),
                pos2(center.x, center.y + r * 0.90),
                pos2(center.x - r * 0.85, center.y),
            ];
            painter.add(egui::Shape::convex_polygon(poly, color.gamma_multiply(0.25), stroke));
            painter.line_segment([pos2(center.x - r * 0.85, center.y), pos2(center.x + r * 0.85, center.y)], thin_stroke);
            painter.line_segment([pos2(center.x, center.y - r * 0.90), pos2(center.x, center.y + r * 0.90)], thin_stroke);
        }

        Icon::SoundBrowser => {
            // Speaker cone with sound waves
            let cone = vec![
                pos2(center.x - r * 0.75, center.y - r * 0.35),
                pos2(center.x - r * 0.30, center.y - r * 0.35),
                pos2(center.x + r * 0.15, center.y - r * 0.75),
                pos2(center.x + r * 0.15, center.y + r * 0.75),
                pos2(center.x - r * 0.30, center.y + r * 0.35),
                pos2(center.x - r * 0.75, center.y + r * 0.35),
            ];
            painter.add(egui::Shape::convex_polygon(cone, color, Stroke::NONE));
            // Sound wave arc
            painter.line_segment([pos2(center.x + r * 0.45, center.y - r * 0.45), pos2(center.x + r * 0.70, center.y)], stroke);
            painter.line_segment([pos2(center.x + r * 0.70, center.y), pos2(center.x + r * 0.45, center.y + r * 0.45)], stroke);
        }

        Icon::MusicBrowser => {
            // Musical note pair
            let note1 = pos2(center.x - r * 0.45, center.y + r * 0.45);
            let note2 = pos2(center.x + r * 0.45, center.y + r * 0.20);
            painter.circle_filled(note1, 2.5, color);
            painter.circle_filled(note2, 2.5, color);
            painter.line_segment([note1, pos2(note1.x, center.y - r * 0.65)], stroke);
            painter.line_segment([note2, pos2(note2.x, center.y - r * 0.90)], stroke);
            painter.line_segment([pos2(note1.x, center.y - r * 0.65), pos2(note2.x, center.y - r * 0.90)], Stroke::new(2.5_f32, color));
        }

        Icon::GroupBrowser => {
            // Overlapping folder / tag tabs
            let b1 = Rect::from_min_size(pos2(center.x - r * 0.85, center.y - r * 0.55), Vec2::new(r * 1.3, r * 1.1));
            let b2 = Rect::from_min_size(pos2(center.x - r * 0.45, center.y - r * 0.85), Vec2::new(r * 1.3, r * 1.1));
            painter.rect_stroke(b2, 2.0, thin_stroke, StrokeKind::Inside);
            painter.rect_filled(b1, 2.0, Color32::from_rgb(25, 30, 38));
            painter.rect_stroke(b1, 2.0, stroke, StrokeKind::Inside);
        }

        Icon::SurfaceProperties => {
            // UV grid quad
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.5));
            painter.rect_stroke(b, 2.0, stroke, StrokeKind::Inside);
            painter.line_segment([pos2(b.min.x + b.width() * 0.33, b.min.y), pos2(b.min.x + b.width() * 0.33, b.max.y)], thin_stroke);
            painter.line_segment([pos2(b.min.x + b.width() * 0.66, b.min.y), pos2(b.min.x + b.width() * 0.66, b.max.y)], thin_stroke);
            painter.line_segment([pos2(b.min.x, b.min.y + b.height() * 0.33), pos2(b.max.x, b.min.y + b.height() * 0.33)], thin_stroke);
            painter.line_segment([pos2(b.min.x, b.min.y + b.height() * 0.66), pos2(b.max.x, b.min.y + b.height() * 0.66)], thin_stroke);
        }

        Icon::ActorProperties => {
            // Mechanical cog gear
            painter.circle_stroke(center, r * 0.60, Stroke::new(2.0_f32, color));
            painter.circle_filled(center, r * 0.25, Color32::from_rgb(22, 25, 30));
            // 4 teeth
            for i in 0..4 {
                let angle = (i as f32) * std::f32::consts::PI / 2.0;
                let p1 = pos2(center.x + r * 0.55 * angle.cos(), center.y + r * 0.55 * angle.sin());
                let p2 = pos2(center.x + r * 0.90 * angle.cos(), center.y + r * 0.90 * angle.sin());
                painter.line_segment([p1, p2], Stroke::new(2.5_f32, color));
            }
        }

        Icon::RadiiView => {
            // Concentric dashed radii circles
            painter.circle_stroke(center, r * 0.90, thin_stroke);
            painter.circle_stroke(center, r * 0.50, thin_stroke);
            painter.circle_filled(center, 2.0, Color32::from_rgb(100, 220, 255));
        }

        Icon::CommandPalette => {
            // Lightning bolt ⚡
            let bolt = vec![
                pos2(center.x + 1.0, center.y - r * 0.95),
                pos2(center.x - r * 0.55, center.y + 0.5),
                pos2(center.x - 0.5, center.y + 0.5),
                pos2(center.x - 2.0, center.y + r * 0.95),
                pos2(center.x + r * 0.55, center.y - 0.5),
                pos2(center.x + 0.5, center.y - 0.5),
            ];
            painter.add(egui::Shape::convex_polygon(bolt, Color32::from_rgb(245, 182, 45), Stroke::NONE));
        }

        Icon::CsgIntersect => {
            // Two intersecting circles with shaded lens
            let offset = r * 0.45;
            let c_r = r * 0.70;
            painter.circle_stroke(pos2(center.x - offset, center.y), c_r, stroke);
            painter.circle_stroke(pos2(center.x + offset, center.y), c_r, stroke);
            painter.circle_filled(center, r * 0.28, Color32::from_rgb(100, 220, 255));
        }

        Icon::CsgDeintersect => {
            // Cutout circle (boolean minus)
            let c_r = r * 0.80;
            painter.circle_stroke(center, c_r, stroke);
            painter.circle_stroke(pos2(center.x + r * 0.50, center.y - r * 0.35), r * 0.45, thin_stroke);
        }

        Icon::PolyMerge => {
            // Two adjacent squares fusing
            let w = r * 0.75;
            let h = r * 0.80;
            painter.rect_stroke(Rect::from_center_size(center, Vec2::new(w * 2.0, h)), 1.0, stroke, StrokeKind::Inside);
            painter.line_segment([pos2(center.x, center.y - h * 0.4), pos2(center.x, center.y + h * 0.4)], thin_stroke);
            painter.line_segment([pos2(center.x - w * 0.5, center.y), pos2(center.x - w * 0.2, center.y)], stroke);
            painter.line_segment([pos2(center.x + w * 0.5, center.y), pos2(center.x + w * 0.2, center.y)], stroke);
        }

        Icon::PolySeparate => {
            // Polygon with diagonal cut line
            let w = r * 0.85;
            let b = Rect::from_center_size(center, Vec2::splat(w * 1.5));
            painter.rect_stroke(b, 1.0, stroke, StrokeKind::Inside);
            painter.line_segment([b.left_bottom(), b.right_top()], stroke);
        }

        Icon::InvertNormals => {
            // Surface segment with a curved flip arrow
            painter.line_segment([pos2(center.x - r * 0.8, center.y + r * 0.4), pos2(center.x + r * 0.8, center.y + r * 0.4)], Stroke::new(1.8_f32, color));
            let top = pos2(center.x, center.y - r * 0.7);
            painter.line_segment([pos2(center.x, center.y + r * 0.2), top], stroke);
            painter.line_segment([pos2(top.x - 3.0, top.y + 3.0), top], stroke);
            painter.line_segment([pos2(top.x + 3.0, top.y + 3.0), top], stroke);
        }

        Icon::ExtractSurface => {
            // Base cube with detached floating top surface and upward arrow
            let b_w = r * 0.9;
            let b_h = r * 0.45;
            painter.rect_stroke(Rect::from_center_size(pos2(center.x, center.y + r * 0.4), Vec2::new(b_w, b_h)), 1.0, thin_stroke, StrokeKind::Inside);
            painter.line_segment([pos2(center.x - b_w * 0.55, center.y - r * 0.4), pos2(center.x + b_w * 0.55, center.y - r * 0.4)], Stroke::new(2.0_f32, Color32::from_rgb(100, 220, 255)));
            painter.line_segment([pos2(center.x, center.y + r * 0.1), pos2(center.x, center.y - r * 0.3)], stroke);
        }

        Icon::AlignFloor => {
            // Heavy floor line and down arrow
            let f_y = center.y + r * 0.7;
            painter.line_segment([pos2(center.x - r * 0.9, f_y), pos2(center.x + r * 0.9, f_y)], Stroke::new(2.0_f32, color));
            let a_tip = pos2(center.x, f_y - 2.0);
            painter.line_segment([pos2(center.x, center.y - r * 0.75), a_tip], stroke);
            painter.line_segment([pos2(a_tip.x - 3.5, a_tip.y - 4.0), a_tip], stroke);
            painter.line_segment([pos2(a_tip.x + 3.5, a_tip.y - 4.0), a_tip], stroke);
        }

        Icon::AlignGrid => {
            // 2x2 grid cross with magnet circle in center
            painter.line_segment([pos2(center.x - r * 0.85, center.y), pos2(center.x + r * 0.85, center.y)], thin_stroke);
            painter.line_segment([pos2(center.x, center.y - r * 0.85), pos2(center.x, center.y + r * 0.85)], thin_stroke);
            painter.circle_filled(center, 2.5, Color32::from_rgb(100, 220, 255));
        }

        Icon::AlignDistribution => {
            // 3 vertical bars with horizontal spacing
            let bar_h = r * 1.2;
            let d = r * 0.65;
            painter.line_segment([pos2(center.x - d, center.y - bar_h * 0.5), pos2(center.x - d, center.y + bar_h * 0.5)], stroke);
            painter.line_segment([pos2(center.x, center.y - bar_h * 0.5), pos2(center.x, center.y + bar_h * 0.5)], stroke);
            painter.line_segment([pos2(center.x + d, center.y - bar_h * 0.5), pos2(center.x + d, center.y + bar_h * 0.5)], stroke);
            painter.line_segment([pos2(center.x - d, center.y), pos2(center.x + d, center.y)], thin_stroke);
        }

        Icon::EyeVisible => {
            // Almond eye with pupil
            let ey_r = r * 0.85;
            let segments = 8;
            for i in 0..segments {
                let x0 = center.x - ey_r + (2.0 * ey_r * (i as f32 / segments as f32));
                let x1 = center.x - ey_r + (2.0 * ey_r * ((i + 1) as f32 / segments as f32));
                let y0_t = center.y - (1.0 - ((x0 - center.x) / ey_r).powi(2)).max(0.0).sqrt() * r * 0.55;
                let y1_t = center.y - (1.0 - ((x1 - center.x) / ey_r).powi(2)).max(0.0).sqrt() * r * 0.55;
                let y0_b = center.y + (1.0 - ((x0 - center.x) / ey_r).powi(2)).max(0.0).sqrt() * r * 0.55;
                let y1_b = center.y + (1.0 - ((x1 - center.x) / ey_r).powi(2)).max(0.0).sqrt() * r * 0.55;
                painter.line_segment([pos2(x0, y0_t), pos2(x1, y1_t)], stroke);
                painter.line_segment([pos2(x0, y0_b), pos2(x1, y1_b)], stroke);
            }
            painter.circle_filled(center, 2.5, color);
        }

        Icon::EyeHidden => {
            // EyeVisible + diagonal slash
            paint_icon(painter, rect, Icon::EyeVisible, color.gamma_multiply(0.55));
            painter.line_segment([pos2(center.x - r * 0.85, center.y + r * 0.75), pos2(center.x + r * 0.85, center.y - r * 0.75)], Stroke::new(1.8_f32, Color32::from_rgb(235, 75, 75)));
        }

        Icon::LockClosed => {
            // Rectangular body with curved shackle
            let b_w = r * 0.95;
            let b_h = r * 0.75;
            let body = Rect::from_min_size(pos2(center.x - b_w * 0.5, center.y - 1.0), Vec2::new(b_w, b_h));
            painter.rect_filled(body, 2.0, color.gamma_multiply(0.2));
            painter.rect_stroke(body, 2.0, stroke, StrokeKind::Inside);
            let shackle_r = r * 0.38;
            for i in 0..8 {
                let a1 = std::f32::consts::PI + (i as f32 / 8.0) * std::f32::consts::PI;
                let a2 = std::f32::consts::PI + ((i + 1) as f32 / 8.0) * std::f32::consts::PI;
                let p1 = pos2(center.x + shackle_r * a1.cos(), center.y - 1.0 + shackle_r * a1.sin());
                let p2 = pos2(center.x + shackle_r * a2.cos(), center.y - 1.0 + shackle_r * a2.sin());
                painter.line_segment([p1, p2], Stroke::new(1.6_f32, color));
            }
            painter.circle_filled(pos2(center.x, body.center().y), 1.5, color);
        }

        Icon::LockOpen => {
            // Rectangular body with open lifted shackle
            let b_w = r * 0.95;
            let b_h = r * 0.75;
            let body = Rect::from_min_size(pos2(center.x - b_w * 0.5, center.y - 1.0), Vec2::new(b_w, b_h));
            painter.rect_filled(body, 2.0, color.gamma_multiply(0.2));
            painter.rect_stroke(body, 2.0, stroke, StrokeKind::Inside);
            let shackle_r = r * 0.38;
            let s_center = pos2(center.x - 2.0, center.y - 4.5);
            for i in 0..8 {
                let a1 = std::f32::consts::PI + (i as f32 / 8.0) * std::f32::consts::PI;
                let a2 = std::f32::consts::PI + ((i + 1) as f32 / 8.0) * std::f32::consts::PI;
                let p1 = pos2(s_center.x + shackle_r * a1.cos(), s_center.y + shackle_r * a1.sin());
                let p2 = pos2(s_center.x + shackle_r * a2.cos(), s_center.y + shackle_r * a2.sin());
                painter.line_segment([p1, p2], Stroke::new(1.6_f32, color));
            }
            painter.circle_filled(pos2(center.x, body.center().y), 1.5, color);
        }

        Icon::FileNew => {
            // Document with dog-ear fold
            let w = r * 0.85;
            let h = r * 1.15;
            let top_right = pos2(center.x + w * 0.5, center.y - h * 0.5);
            let fold = r * 0.35;
            let p1 = pos2(center.x - w * 0.5, center.y - h * 0.5);
            let p2 = pos2(top_right.x - fold, top_right.y);
            let p3 = pos2(top_right.x, top_right.y + fold);
            let p4 = pos2(center.x + w * 0.5, center.y + h * 0.5);
            let p5 = pos2(center.x - w * 0.5, center.y + h * 0.5);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            painter.line_segment([p4, p5], stroke);
            painter.line_segment([p5, p1], stroke);
            let fold_corner = pos2(top_right.x - fold, top_right.y + fold);
            painter.line_segment([p2, fold_corner], stroke);
            painter.line_segment([fold_corner, p3], stroke);
        }

        Icon::FileOpen => {
            // Folder outline with tab
            let f_w = r * 1.2;
            let f_h = r * 0.85;
            let b = Rect::from_center_size(pos2(center.x, center.y + r * 0.1), Vec2::new(f_w, f_h));
            painter.rect_stroke(b, 2.0, stroke, StrokeKind::Inside);
            let tab_w = f_w * 0.45;
            let tab = Rect::from_min_size(pos2(b.min.x, b.min.y - r * 0.28), Vec2::new(tab_w, r * 0.30));
            painter.rect_stroke(tab, 2.0, stroke, StrokeKind::Inside);
        }

        Icon::FileSave => {
            // 3.5" floppy disk
            let sz = r * 1.15;
            let b = Rect::from_center_size(center, Vec2::splat(sz));
            painter.rect_stroke(b, 2.0, stroke, StrokeKind::Inside);
            let shutter = Rect::from_min_size(pos2(center.x - sz * 0.25, b.min.y), Vec2::new(sz * 0.50, sz * 0.38));
            painter.rect_filled(shutter, 1.0, color.gamma_multiply(0.3));
            painter.rect_stroke(shutter, 1.0, thin_stroke, StrokeKind::Inside);
            let label_rect = Rect::from_min_size(pos2(b.min.x + 2.0, center.y + sz * 0.1), Vec2::new(sz - 4.0, sz * 0.32));
            painter.rect_stroke(label_rect, 1.0, thin_stroke, StrokeKind::Inside);
        }

        Icon::FileExport => {
            // Box with outward arrow pointing top-right
            let b_w = r * 0.9;
            let b = Rect::from_min_size(pos2(center.x - b_w * 0.5, center.y - b_w * 0.2), Vec2::splat(b_w * 0.8));
            painter.rect_stroke(b, 2.0, thin_stroke, StrokeKind::Inside);
            let tip = pos2(center.x + r * 0.8, center.y - r * 0.8);
            painter.line_segment([center, tip], Stroke::new(1.8_f32, color));
            painter.line_segment([pos2(tip.x - 4.0, tip.y), tip], stroke);
            painter.line_segment([pos2(tip.x, tip.y + 4.0), tip], stroke);
        }

        Icon::Undo => {
            // Curved arrow pointing left
            let arc_r = r * 0.65;
            for i in 0..12 {
                let a1 = -0.3 + (i as f32 / 12.0) * std::f32::consts::PI * 1.1;
                let a2 = -0.3 + ((i + 1) as f32 / 12.0) * std::f32::consts::PI * 1.1;
                let p1 = pos2(center.x + arc_r * a1.cos(), center.y + arc_r * a1.sin());
                let p2 = pos2(center.x + arc_r * a2.cos(), center.y + arc_r * a2.sin());
                painter.line_segment([p1, p2], stroke);
            }
            let tip = pos2(center.x - arc_r, center.y);
            painter.line_segment([pos2(tip.x + 3.5, tip.y - 3.5), tip], stroke);
            painter.line_segment([pos2(tip.x + 3.5, tip.y + 3.5), tip], stroke);
        }

        Icon::Redo => {
            // Curved arrow pointing right
            let arc_r = r * 0.65;
            for i in 0..12 {
                let a1 = std::f32::consts::PI + 0.3 - (i as f32 / 12.0) * std::f32::consts::PI * 1.1;
                let a2 = std::f32::consts::PI + 0.3 - ((i + 1) as f32 / 12.0) * std::f32::consts::PI * 1.1;
                let p1 = pos2(center.x + arc_r * a1.cos(), center.y + arc_r * a1.sin());
                let p2 = pos2(center.x + arc_r * a2.cos(), center.y + arc_r * a2.sin());
                painter.line_segment([p1, p2], stroke);
            }
            let tip = pos2(center.x + arc_r, center.y);
            painter.line_segment([pos2(tip.x - 3.5, tip.y - 3.5), tip], stroke);
            painter.line_segment([pos2(tip.x - 3.5, tip.y + 3.5), tip], stroke);
        }

        Icon::Cut => {
            // Scissors
            let tip1 = pos2(center.x + r * 0.7, center.y - r * 0.7);
            let tip2 = pos2(center.x - r * 0.7, center.y - r * 0.7);
            let loop1 = pos2(center.x - r * 0.45, center.y + r * 0.55);
            let loop2 = pos2(center.x + r * 0.45, center.y + r * 0.55);
            painter.line_segment([tip1, loop1], stroke);
            painter.line_segment([tip2, loop2], stroke);
            painter.circle_stroke(loop1, r * 0.25, stroke);
            painter.circle_stroke(loop2, r * 0.25, stroke);
            painter.circle_filled(center, 1.5, color);
        }

        Icon::Copy => {
            // Two overlapping sheets
            let sz = Vec2::new(r * 0.85, r * 1.05);
            let b1 = Rect::from_center_size(pos2(center.x + r * 0.25, center.y - r * 0.25), sz);
            let b2 = Rect::from_center_size(pos2(center.x - r * 0.25, center.y + r * 0.25), sz);
            painter.rect_stroke(b1, 1.0, thin_stroke, StrokeKind::Inside);
            painter.rect_filled(b2, 1.0, Color32::from_rgb(22, 25, 30));
            painter.rect_stroke(b2, 1.0, stroke, StrokeKind::Inside);
        }

        Icon::Paste => {
            // Clipboard with top clip
            let sz = Vec2::new(r * 1.0, r * 1.2);
            let b = Rect::from_center_size(pos2(center.x, center.y + r * 0.1), sz);
            painter.rect_stroke(b, 2.0, stroke, StrokeKind::Inside);
            let clip = Rect::from_center_size(pos2(center.x, b.min.y), Vec2::new(sz.x * 0.45, r * 0.25));
            painter.rect_filled(clip, 1.0, color);
            painter.line_segment([pos2(b.min.x + 3.0, center.y), pos2(b.max.x - 3.0, center.y)], thin_stroke);
            painter.line_segment([pos2(b.min.x + 3.0, center.y + 4.0), pos2(b.max.x - 5.0, center.y + 4.0)], thin_stroke);
        }

        Icon::SelectAll => {
            // Dashed bounding rectangle
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.5));
            painter.rect_stroke(b, 1.0, thin_stroke, StrokeKind::Inside);
            let d = r * 0.35;
            for (cx, cy) in [(b.min.x, b.min.y), (b.max.x, b.min.y), (b.min.x, b.max.y), (b.max.x, b.max.y)] {
                painter.rect_filled(Rect::from_center_size(pos2(cx, cy), Vec2::splat(d)), 1.0, color);
            }
        }

        Icon::ShapeCube => {
            // Isometric 3D wireframe cube
            let hex_r = r * 0.95;
            let mut pts = Vec::with_capacity(6);
            for i in 0..6 {
                let a = (i as f32) * std::f32::consts::PI / 3.0 - std::f32::consts::PI / 6.0;
                pts.push(pos2(center.x + hex_r * a.cos(), center.y + hex_r * a.sin()));
            }
            for i in 0..6 {
                painter.line_segment([pts[i], pts[(i + 1) % 6]], stroke);
            }
            painter.line_segment([center, pts[1]], stroke);
            painter.line_segment([center, pts[3]], stroke);
            painter.line_segment([center, pts[5]], stroke);
        }

        Icon::ShapeCylinder => {
            // Top ellipse, vertical edges, bottom arc
            let ell_w = r * 0.85;
            let ell_h = r * 0.32;
            let top_y = center.y - r * 0.55;
            let bot_y = center.y + r * 0.55;
            for i in 0..16 {
                let a1 = (i as f32 / 16.0) * std::f32::consts::PI * 2.0;
                let a2 = ((i + 1) as f32 / 16.0) * std::f32::consts::PI * 2.0;
                painter.line_segment([pos2(center.x + ell_w * a1.cos(), top_y + ell_h * a1.sin()), pos2(center.x + ell_w * a2.cos(), top_y + ell_h * a2.sin())], stroke);
                if a1 >= 0.0 && a1 <= std::f32::consts::PI {
                    painter.line_segment([pos2(center.x + ell_w * a1.cos(), bot_y + ell_h * a1.sin()), pos2(center.x + ell_w * a2.cos(), bot_y + ell_h * a2.sin())], stroke);
                }
            }
            painter.line_segment([pos2(center.x - ell_w, top_y), pos2(center.x - ell_w, bot_y)], stroke);
            painter.line_segment([pos2(center.x + ell_w, top_y), pos2(center.x + ell_w, bot_y)], stroke);
        }

        Icon::ShapeCone => {
            // Triangle with curved bottom arc
            let tip = pos2(center.x, center.y - r * 0.85);
            let bot_y = center.y + r * 0.65;
            let base_w = r * 0.85;
            let ell_h = r * 0.28;
            painter.line_segment([tip, pos2(center.x - base_w, bot_y)], stroke);
            painter.line_segment([tip, pos2(center.x + base_w, bot_y)], stroke);
            for i in 0..12 {
                let a1 = (i as f32 / 12.0) * std::f32::consts::PI;
                let a2 = ((i + 1) as f32 / 12.0) * std::f32::consts::PI;
                painter.line_segment([pos2(center.x + base_w * a1.cos(), bot_y + ell_h * a1.sin()), pos2(center.x + base_w * a2.cos(), bot_y + ell_h * a2.sin())], stroke);
            }
        }

        Icon::ShapeSphere => {
            // Circle with equator ellipse
            painter.circle_stroke(center, r * 0.85, stroke);
            let eq_h = r * 0.32;
            for i in 0..16 {
                let a1 = (i as f32 / 16.0) * std::f32::consts::PI * 2.0;
                let a2 = ((i + 1) as f32 / 16.0) * std::f32::consts::PI * 2.0;
                painter.line_segment([pos2(center.x + r * 0.85 * a1.cos(), center.y + eq_h * a1.sin()), pos2(center.x + r * 0.85 * a2.cos(), center.y + eq_h * a2.sin())], thin_stroke);
            }
        }

        Icon::ShapeSheet => {
            // Planar quad with normal arrow
            let p1 = pos2(center.x - r * 0.8, center.y + r * 0.4);
            let p2 = pos2(center.x - r * 0.2, center.y - r * 0.4);
            let p3 = pos2(center.x + r * 0.8, center.y - r * 0.4);
            let p4 = pos2(center.x + r * 0.2, center.y + r * 0.4);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            painter.line_segment([p4, p1], stroke);
            let arrow_tip = pos2(center.x, center.y - r * 0.7);
            painter.line_segment([center, arrow_tip], Stroke::new(1.8_f32, color));
            painter.line_segment([pos2(arrow_tip.x - 2.5, arrow_tip.y + 2.5), arrow_tip], stroke);
            painter.line_segment([pos2(arrow_tip.x + 2.5, arrow_tip.y + 2.5), arrow_tip], stroke);
        }

        Icon::ShapeStairs => {
            // Stepped profile
            let steps = 4;
            let start = pos2(center.x - r * 0.8, center.y + r * 0.7);
            let step_w = (r * 1.6) / (steps as f32);
            let step_h = (r * 1.4) / (steps as f32);
            let mut cur = start;
            for _ in 0..steps {
                let up = pos2(cur.x, cur.y - step_h);
                let over = pos2(cur.x + step_w, cur.y - step_h);
                painter.line_segment([cur, up], stroke);
                painter.line_segment([up, over], stroke);
                cur = over;
            }
            painter.line_segment([cur, pos2(cur.x, start.y)], thin_stroke);
            painter.line_segment([pos2(cur.x, start.y), start], thin_stroke);
        }

        Icon::ShapeCurvedStairs => {
            // Curved steps arc
            for i in 0..5 {
                let angle = (i as f32 / 4.0) * std::f32::consts::PI * 0.6 - 0.2;
                let p1 = pos2(center.x + r * 0.4 * angle.cos(), center.y + r * 0.4 * angle.sin());
                let p2 = pos2(center.x + r * 0.9 * angle.cos(), center.y + r * 0.9 * angle.sin());
                painter.line_segment([p1, p2], stroke);
            }
            painter.circle_stroke(center, r * 0.9, thin_stroke);
            painter.circle_stroke(center, r * 0.4, thin_stroke);
        }

        Icon::ShapeSpiralStairs => {
            // Central pole + helical step tiers
            painter.line_segment([pos2(center.x, center.y - r * 0.9), pos2(center.x, center.y + r * 0.9)], Stroke::new(2.2_f32, color));
            for i in 0..5 {
                let y = center.y - r * 0.6 + (i as f32 * r * 0.3);
                let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
                painter.line_segment([pos2(center.x, y), pos2(center.x + dir * r * 0.65, y - 2.0)], stroke);
            }
        }

        Icon::ShapeArch => {
            // Roman rounded arch with opening
            let arch_w = r * 0.8;
            let arch_top_y = center.y - r * 0.3;
            let bot_y = center.y + r * 0.8;
            for i in 0..12 {
                let a1 = std::f32::consts::PI + (i as f32 / 12.0) * std::f32::consts::PI;
                let a2 = std::f32::consts::PI + ((i + 1) as f32 / 12.0) * std::f32::consts::PI;
                painter.line_segment([pos2(center.x + arch_w * a1.cos(), arch_top_y + arch_w * a1.sin()), pos2(center.x + arch_w * a2.cos(), arch_top_y + arch_w * a2.sin())], stroke);
            }
            painter.line_segment([pos2(center.x - arch_w, arch_top_y), pos2(center.x - arch_w, bot_y)], stroke);
            painter.line_segment([pos2(center.x + arch_w, arch_top_y), pos2(center.x + arch_w, bot_y)], stroke);
            let inner_w = arch_w * 0.55;
            for i in 0..12 {
                let a1 = std::f32::consts::PI + (i as f32 / 12.0) * std::f32::consts::PI;
                let a2 = std::f32::consts::PI + ((i + 1) as f32 / 12.0) * std::f32::consts::PI;
                painter.line_segment([pos2(center.x + inner_w * a1.cos(), arch_top_y + inner_w * a1.sin()), pos2(center.x + inner_w * a2.cos(), arch_top_y + inner_w * a2.sin())], thin_stroke);
            }
            painter.line_segment([pos2(center.x - inner_w, arch_top_y), pos2(center.x - inner_w, bot_y)], thin_stroke);
            painter.line_segment([pos2(center.x + inner_w, arch_top_y), pos2(center.x + inner_w, bot_y)], thin_stroke);
        }

        Icon::ShapeTorus => {
            // Concentric outer and inner circles
            painter.circle_stroke(center, r * 0.85, stroke);
            painter.circle_stroke(center, r * 0.38, stroke);
        }

        Icon::ShapeTerrain => {
            // Two wavy contour curves
            let w = r * 0.9;
            for i in 0..16 {
                let x0 = center.x - w + (2.0 * w * (i as f32 / 16.0));
                let x1 = center.x - w + (2.0 * w * ((i + 1) as f32 / 16.0));
                let y0_1 = center.y - r * 0.2 + (i as f32 * 0.7).sin() * 3.5;
                let y1_1 = center.y - r * 0.2 + ((i + 1) as f32 * 0.7).sin() * 3.5;
                let y0_2 = center.y + r * 0.35 + ((i as f32 + 2.0) * 0.6).sin() * 3.5;
                let y1_2 = center.y + r * 0.35 + ((i as f32 + 3.0) * 0.6).sin() * 3.5;
                painter.line_segment([pos2(x0, y0_1), pos2(x1, y1_1)], stroke);
                painter.line_segment([pos2(x0, y0_2), pos2(x1, y1_2)], stroke);
            }
        }

        Icon::ActorLight => {
            // Lightbulb profile with filament and rays
            let bulb_r = r * 0.55;
            let bulb_center = pos2(center.x, center.y - r * 0.15);
            painter.circle_stroke(bulb_center, bulb_r, stroke);
            painter.circle_filled(bulb_center, bulb_r * 0.5, Color32::from_rgb(255, 220, 80));
            let base_w = bulb_r * 0.6;
            painter.line_segment([pos2(center.x - base_w, bulb_center.y + bulb_r), pos2(center.x + base_w, bulb_center.y + bulb_r)], stroke);
            painter.line_segment([pos2(center.x - base_w * 0.7, bulb_center.y + bulb_r + 3.0), pos2(center.x + base_w * 0.7, bulb_center.y + bulb_r + 3.0)], thin_stroke);
        }

        Icon::ActorPlayerStart => {
            // Flagpole with waving pennant
            let pole_x = center.x - r * 0.4;
            painter.line_segment([pos2(pole_x, center.y - r * 0.85), pos2(pole_x, center.y + r * 0.85)], Stroke::new(1.8_f32, color));
            let flag = vec![
                pos2(pole_x, center.y - r * 0.85),
                pos2(pole_x + r * 0.85, center.y - r * 0.50),
                pos2(pole_x, center.y - r * 0.15),
            ];
            painter.add(egui::Shape::convex_polygon(flag, Color32::from_rgb(45, 180, 95), Stroke::NONE));
            painter.circle_stroke(pos2(pole_x, center.y + r * 0.85), 3.0, thin_stroke);
        }

        Icon::ActorPathNode => {
            // Bot navigation rhombus / diamond
            let d = vec![
                pos2(center.x, center.y - r * 0.85),
                pos2(center.x + r * 0.75, center.y),
                pos2(center.x, center.y + r * 0.85),
                pos2(center.x - r * 0.75, center.y),
            ];
            painter.add(egui::Shape::convex_polygon(d, Color32::from_rgb(47, 128, 237).gamma_multiply(0.2), stroke));
            painter.circle_filled(center, 2.0, Color32::from_rgb(100, 220, 255));
        }

        Icon::ActorTrigger => {
            // Lightning / spark polygon
            let spark = vec![
                pos2(center.x + 1.0, center.y - r * 0.90),
                pos2(center.x - r * 0.50, center.y),
                pos2(center.x - 0.5, center.y),
                pos2(center.x - 1.5, center.y + r * 0.90),
                pos2(center.x + r * 0.50, center.y - 1.0),
                pos2(center.x + 0.5, center.y - 1.0),
            ];
            painter.add(egui::Shape::convex_polygon(spark, Color32::from_rgb(255, 180, 45), Stroke::NONE));
        }

        Icon::ActorTeleporter => {
            // Portal rings
            painter.circle_stroke(center, r * 0.85, stroke);
            painter.circle_stroke(center, r * 0.52, thin_stroke);
            painter.circle_stroke(center, r * 0.22, thin_stroke);
        }

        Icon::ActorWeapon => {
            // Stylized weapon rifle
            painter.line_segment([pos2(center.x - r * 0.85, center.y + 1.0), pos2(center.x + r * 0.85, center.y + 1.0)], Stroke::new(2.2_f32, color));
            painter.line_segment([pos2(center.x + r * 0.3, center.y - r * 0.35), pos2(center.x + r * 0.3, center.y + 1.0)], stroke);
            painter.line_segment([pos2(center.x - r * 0.3, center.y + 1.0), pos2(center.x - r * 0.45, center.y + r * 0.55)], Stroke::new(2.0_f32, color));
        }

        Icon::ActorPickup => {
            // Medical cross
            let arm = r * 0.28;
            let len = r * 0.85;
            let cross = vec![
                pos2(center.x - arm, center.y - len),
                pos2(center.x + arm, center.y - len),
                pos2(center.x + arm, center.y - arm),
                pos2(center.x + len, center.y - arm),
                pos2(center.x + len, center.y + arm),
                pos2(center.x + arm, center.y + arm),
                pos2(center.x + arm, center.y + len),
                pos2(center.x - arm, center.y + len),
                pos2(center.x - arm, center.y + arm),
                pos2(center.x - len, center.y + arm),
                pos2(center.x - len, center.y - arm),
                pos2(center.x - arm, center.y - arm),
            ];
            painter.add(egui::Shape::convex_polygon(cross, Color32::from_rgb(235, 75, 75), Stroke::new(1.0_f32, Color32::WHITE)));
        }

        Icon::ActorMover => {
            // Sliding double doors with outward arrows
            let d_w = r * 0.55;
            let d_h = r * 1.2;
            let d1 = Rect::from_min_size(pos2(center.x - d_w - 1.0, center.y - d_h * 0.5), Vec2::new(d_w, d_h));
            let d2 = Rect::from_min_size(pos2(center.x + 1.0, center.y - d_h * 0.5), Vec2::new(d_w, d_h));
            painter.rect_stroke(d1, 1.0, stroke, StrokeKind::Inside);
            painter.rect_stroke(d2, 1.0, stroke, StrokeKind::Inside);
            painter.line_segment([pos2(d1.center().x + 2.0, d1.center().y), pos2(d1.center().x - 2.0, d1.center().y)], thin_stroke);
            painter.line_segment([pos2(d2.center().x - 2.0, d2.center().y), pos2(d2.center().x + 2.0, d2.center().y)], thin_stroke);
        }

        Icon::StaticMesh => {
            // Faceted 3D diamond mesh
            let top = pos2(center.x, center.y - r * 0.85);
            let bot = pos2(center.x, center.y + r * 0.85);
            let mid_l = pos2(center.x - r * 0.85, center.y - r * 0.1);
            let mid_r = pos2(center.x + r * 0.85, center.y - r * 0.1);
            let mid_c = pos2(center.x, center.y + r * 0.15);
            painter.line_segment([top, mid_l], stroke);
            painter.line_segment([top, mid_r], stroke);
            painter.line_segment([top, mid_c], stroke);
            painter.line_segment([mid_l, mid_c], stroke);
            painter.line_segment([mid_r, mid_c], stroke);
            painter.line_segment([mid_l, bot], stroke);
            painter.line_segment([mid_r, bot], stroke);
            painter.line_segment([mid_c, bot], stroke);
        }

        Icon::TextureEyedropper => {
            // 45° pipette with tip pointing down-left
            let tip = pos2(center.x - r * 0.7, center.y + r * 0.7);
            let bulb = pos2(center.x + r * 0.65, center.y - r * 0.65);
            painter.line_segment([tip, bulb], Stroke::new(2.5_f32, color));
            painter.circle_filled(bulb, 3.5, color);
            painter.circle_filled(pos2(tip.x - 3.0, tip.y + 3.0), 1.5, Color32::from_rgb(100, 220, 255));
        }

        Icon::TextureFit => {
            // Frame with 4 corner arrows
            let b = Rect::from_center_size(center, Vec2::splat(r * 1.2));
            painter.rect_stroke(b, 1.0, stroke, StrokeKind::Inside);
            painter.line_segment([b.left_top(), pos2(b.left_top().x + 3.0, b.left_top().y + 3.0)], stroke);
            painter.line_segment([b.right_top(), pos2(b.right_top().x - 3.0, b.right_top().y + 3.0)], stroke);
            painter.line_segment([b.left_bottom(), pos2(b.left_bottom().x + 3.0, b.left_bottom().y - 3.0)], stroke);
            painter.line_segment([b.right_bottom(), pos2(b.right_bottom().x - 3.0, b.right_bottom().y - 3.0)], stroke);
        }

        Icon::TextureAlign => {
            // Three parallel alignment lines with axis tick
            let w = r * 0.8;
            for i in 0..3 {
                let y = center.y - r * 0.5 + (i as f32 * r * 0.5);
                painter.line_segment([pos2(center.x - w, y), pos2(center.x + w, y)], stroke);
            }
            painter.line_segment([pos2(center.x - w, center.y - r * 0.6), pos2(center.x - w, center.y + r * 0.6)], Stroke::new(2.0_f32, color));
        }

        Icon::MaterialPreset => {
            // Painter's palette
            painter.circle_stroke(center, r * 0.85, stroke);
            painter.circle_filled(pos2(center.x + r * 0.4, center.y + r * 0.35), 2.5, Color32::from_rgb(22, 25, 30));
            painter.circle_filled(pos2(center.x - r * 0.4, center.y - r * 0.3), 2.0, Color32::from_rgb(235, 75, 75));
            painter.circle_filled(pos2(center.x, center.y - r * 0.5), 2.0, Color32::from_rgb(245, 182, 45));
            painter.circle_filled(pos2(center.x + r * 0.4, center.y - r * 0.25), 2.0, Color32::from_rgb(47, 128, 237));
        }

        Icon::PlayGame => {
            // Solid rightward pointing play triangle ▶
            let tri = vec![
                pos2(center.x - r * 0.55, center.y - r * 0.85),
                pos2(center.x + r * 0.85, center.y),
                pos2(center.x - r * 0.55, center.y + r * 0.85),
            ];
            painter.add(egui::Shape::convex_polygon(tri, Color32::from_rgb(45, 180, 95), Stroke::NONE));
        }

        Icon::BuildGeometry => {
            // Mason's hammer at 45°
            let head = Rect::from_center_size(pos2(center.x + r * 0.35, center.y - r * 0.35), Vec2::new(r * 0.85, r * 0.45));
            painter.rect_filled(head, 1.0, color);
            let handle_start = pos2(center.x + r * 0.15, center.y - r * 0.15);
            let handle_end = pos2(center.x - r * 0.75, center.y + r * 0.75);
            painter.line_segment([handle_start, handle_end], Stroke::new(2.5_f32, color));
        }

        Icon::Settings => {
            // 6-tooth mechanical cog
            painter.circle_stroke(center, r * 0.60, Stroke::new(2.0_f32, color));
            painter.circle_filled(center, r * 0.25, Color32::from_rgb(22, 25, 30));
            for i in 0..6 {
                let a = (i as f32) * std::f32::consts::PI / 3.0;
                let p1 = pos2(center.x + r * 0.55 * a.cos(), center.y + r * 0.55 * a.sin());
                let p2 = pos2(center.x + r * 0.90 * a.cos(), center.y + r * 0.90 * a.sin());
                painter.line_segment([p1, p2], Stroke::new(2.2_f32, color));
            }
        }

        Icon::Wizard => {
            // Magic wand at 45° with 4-point star at tip
            let wand_end = pos2(center.x - r * 0.75, center.y + r * 0.75);
            let wand_tip = pos2(center.x + r * 0.35, center.y - r * 0.35);
            painter.line_segment([wand_end, wand_tip], Stroke::new(2.2_f32, color));
            let star_c = pos2(center.x + r * 0.55, center.y - r * 0.55);
            painter.line_segment([pos2(star_c.x - 3.5, star_c.y), pos2(star_c.x + 3.5, star_c.y)], Stroke::new(1.5_f32, Color32::from_rgb(255, 230, 80)));
            painter.line_segment([pos2(star_c.x, star_c.y - 3.5), pos2(star_c.x, star_c.y + 3.5)], Stroke::new(1.5_f32, Color32::from_rgb(255, 230, 80)));
        }

        Icon::PresetCatalog => {
            // Stacked preset cards
            let sz = Vec2::new(r * 1.1, r * 0.75);
            painter.rect_stroke(Rect::from_center_size(pos2(center.x + 2.0, center.y - 2.0), sz), 2.0, thin_stroke, StrokeKind::Inside);
            let front = Rect::from_center_size(pos2(center.x - 2.0, center.y + 2.0), sz);
            painter.rect_filled(front, 2.0, Color32::from_rgb(24, 27, 34));
            painter.rect_stroke(front, 2.0, stroke, StrokeKind::Inside);
        }

        Icon::LevelDoctor => {
            // Medical cross
            let arm = r * 0.3;
            let len = r * 0.85;
            let cross = vec![
                pos2(center.x - arm, center.y - len),
                pos2(center.x + arm, center.y - len),
                pos2(center.x + arm, center.y - arm),
                pos2(center.x + len, center.y - arm),
                pos2(center.x + len, center.y + arm),
                pos2(center.x + arm, center.y + arm),
                pos2(center.x + arm, center.y + len),
                pos2(center.x - arm, center.y + len),
                pos2(center.x - arm, center.y + arm),
                pos2(center.x - len, center.y + arm),
                pos2(center.x - len, center.y - arm),
                pos2(center.x - arm, center.y - arm),
            ];
            painter.add(egui::Shape::convex_polygon(cross, Color32::from_rgb(40, 180, 95).gamma_multiply(0.25), stroke));
            painter.circle_filled(center, 2.0, Color32::from_rgb(110, 240, 160));
        }

        Icon::Search => {
            let lens_c = pos2(center.x - r * 0.15, center.y - r * 0.15);
            let lens_r = r * 0.55;
            painter.circle_stroke(lens_c, lens_r, stroke);
            painter.line_segment(
                [
                    pos2(lens_c.x + lens_r * 0.7, lens_c.y + lens_r * 0.7),
                    pos2(center.x + r * 0.75, center.y + r * 0.75),
                ],
                Stroke::new(2.0_f32, color),
            );
        }

        Icon::Filter => {
            let funnel = vec![
                pos2(center.x - r * 0.8, center.y - r * 0.65),
                pos2(center.x + r * 0.8, center.y - r * 0.65),
                pos2(center.x + r * 0.18, center.y + r * 0.05),
                pos2(center.x + r * 0.18, center.y + r * 0.75),
                pos2(center.x - r * 0.18, center.y + r * 0.55),
                pos2(center.x - r * 0.18, center.y + r * 0.05),
            ];
            painter.add(egui::Shape::closed_line(funnel, stroke));
        }

        Icon::CheckCircle => {
            painter.circle_stroke(center, r * 0.82, stroke);
            painter.line_segment(
                [pos2(center.x - r * 0.38, center.y + r * 0.02), pos2(center.x - r * 0.08, center.y + r * 0.32)],
                Stroke::new(1.8_f32, color),
            );
            painter.line_segment(
                [pos2(center.x - r * 0.08, center.y + r * 0.32), pos2(center.x + r * 0.42, center.y - r * 0.26)],
                Stroke::new(1.8_f32, color),
            );
        }

        Icon::AlertTriangle => {
            let tri = vec![
                pos2(center.x, center.y - r * 0.8),
                pos2(center.x + r * 0.82, center.y + r * 0.72),
                pos2(center.x - r * 0.82, center.y + r * 0.72),
            ];
            painter.add(egui::Shape::closed_line(tri, stroke));
            painter.line_segment(
                [pos2(center.x, center.y - r * 0.25), pos2(center.x, center.y + r * 0.25)],
                Stroke::new(1.8_f32, color),
            );
            painter.circle_filled(pos2(center.x, center.y + r * 0.48), 1.4, color);
        }

        Icon::InfoCircle => {
            painter.circle_stroke(center, r * 0.82, stroke);
            painter.circle_filled(pos2(center.x, center.y - r * 0.35), 1.4, color);
            painter.line_segment(
                [pos2(center.x, center.y - r * 0.08), pos2(center.x, center.y + r * 0.45)],
                Stroke::new(1.8_f32, color),
            );
        }

        Icon::Sliders => {
            for (i, knob_x) in [(-0.55_f32, -0.25_f32), (0.0, 0.3), (0.55, -0.1)].into_iter() {
                let y = center.y + r * i;
                painter.line_segment([pos2(center.x - r * 0.8, y), pos2(center.x + r * 0.8, y)], thin_stroke);
                painter.circle_filled(pos2(center.x + r * knob_x, y), 2.3, color);
            }
        }

        Icon::Folder => {
            let folder = vec![
                pos2(center.x - r * 0.82, center.y - r * 0.55),
                pos2(center.x - r * 0.15, center.y - r * 0.55),
                pos2(center.x + r * 0.05, center.y - r * 0.32),
                pos2(center.x + r * 0.82, center.y - r * 0.32),
                pos2(center.x + r * 0.82, center.y + r * 0.65),
                pos2(center.x - r * 0.82, center.y + r * 0.65),
            ];
            painter.add(egui::Shape::closed_line(folder, stroke));
        }

        Icon::Sparkles => {
            for (cx, cy, s) in [(0.0_f32, 0.0_f32, 0.75_f32), (-0.52, -0.48, 0.35), (0.52, 0.45, 0.32)] {
                let c = pos2(center.x + r * cx, center.y + r * cy);
                let arm = r * s;
                painter.line_segment([pos2(c.x - arm, c.y), pos2(c.x + arm, c.y)], stroke);
                painter.line_segment([pos2(c.x, c.y - arm), pos2(c.x, c.y + arm)], stroke);
            }
        }

        Icon::Compass => {
            painter.circle_stroke(center, r * 0.82, stroke);
            let diamond = vec![
                pos2(center.x + r * 0.42, center.y - r * 0.42),
                pos2(center.x + r * 0.14, center.y + r * 0.14),
                pos2(center.x - r * 0.42, center.y + r * 0.42),
                pos2(center.x - r * 0.14, center.y - r * 0.14),
            ];
            painter.add(egui::Shape::closed_line(diamond, thin_stroke));
        }
    }
}

/// Standalone icon display widget.
pub fn icon(ui: &mut Ui, icon: Icon, size: Vec2, color: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_icon(ui.painter(), rect, icon, color);
    }
    response
}

/// Square icon button widget (for segmented pills and compact toolbars).
/// Enforces WCAG 2.2 SC 2.5.8 minimum hit target (`>= 24x24` logical px) and SC 2.4.7 keyboard focus ring.
pub fn icon_button(
    ui: &mut Ui,
    icon: Icon,
    is_selected: bool,
    height: f32,
) -> Response {
    let palette = crate::gui::theme::SemanticPalette::current(ui.ctx());
    let target_side = height.max(24.0);
    let size = Vec2::new(target_side, target_side);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            ui.is_enabled(),
            is_selected,
            format!("{:?}", icon),
        )
    });
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let bg = if is_selected {
            palette.bg_widget_active
        } else if response.hovered() {
            palette.bg_widget_hover
        } else {
            Color32::TRANSPARENT
        };
        painter.rect_filled(rect, 4.0, bg);
        if is_selected {
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, palette.accent_azure), StrokeKind::Inside);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect.expand(1.0),
                5.0,
                Stroke::new(2.0_f32, palette.border_focus),
                StrokeKind::Outside,
            );
        }
        let fg = if is_selected || response.hovered() {
            palette.text_primary
        } else {
            palette.text_secondary
        };
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat((height - 6.0).clamp(13.0, 18.0)));
        paint_icon(painter, icon_rect, icon, fg);
    }
    response
}

/// Responsive icon button that renders an icon with an optional text label.
/// Enforces WCAG 2.2 SC 2.5.8 minimum target height (`24.0` logical px) and visible keyboard focus ring.
pub fn icon_label_button(
    ui: &mut Ui,
    icon: Icon,
    label: Option<&str>,
    is_selected: bool,
) -> Response {
    let palette = crate::gui::theme::SemanticPalette::current(ui.ctx());
    let height = 24.0;
    let icon_size = 13.5;
    let padding_x = 6.0;

    let text_width = if let Some(txt) = label {
        let font_id = egui::FontId::proportional(11.5);
        ui.fonts(|f| f.layout_no_wrap(txt.to_string(), font_id, palette.text_primary).size().x) + 5.0
    } else {
        0.0
    };
    let total_width = (padding_x * 2.0 + icon_size + text_width).max(24.0);
    let size = Vec2::new(total_width, height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let accessible_name = label.map(|s| s.to_string()).unwrap_or_else(|| format!("{:?}", icon));
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            ui.is_enabled(),
            is_selected,
            accessible_name.clone(),
        )
    });

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let bg = if is_selected {
            palette.bg_widget_active
        } else if response.hovered() {
            palette.bg_widget_hover
        } else {
            Color32::TRANSPARENT
        };
        painter.rect_filled(rect, 4.0, bg);
        if is_selected {
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, palette.accent_azure), StrokeKind::Inside);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect.expand(1.0),
                5.0,
                Stroke::new(2.0_f32, palette.border_focus),
                StrokeKind::Outside,
            );
        }
        let fg = if is_selected || response.hovered() {
            palette.text_primary
        } else {
            palette.text_secondary
        };

        let icon_rect = if label.is_some() {
            Rect::from_min_size(
                pos2(rect.min.x + padding_x, rect.center().y - icon_size / 2.0),
                Vec2::splat(icon_size),
            )
        } else {
            Rect::from_center_size(rect.center(), Vec2::splat(icon_size))
        };
        paint_icon(painter, icon_rect, icon, fg);

        if let Some(txt) = label {
            let text_pos = pos2(icon_rect.max.x + 5.0, rect.center().y);
            painter.text(
                text_pos,
                egui::Align2::LEFT_CENTER,
                txt,
                egui::FontId::proportional(11.5),
                fg,
            );
        }
    }
    response
}

/// Renders a full-width clickable menu or context menu item with vector icon, label, and right-aligned shortcut.
pub fn context_menu_item(
    ui: &mut Ui,
    icon: Option<Icon>,
    label: &str,
    shortcut: Option<&str>,
    text_color: Option<Color32>,
) -> Response {
    let palette = crate::gui::theme::SemanticPalette::current(ui.ctx());
    let height = 24.0;
    let icon_size = 13.5;
    let padding_x = 6.0;

    let available_w = ui.available_width().max(200.0);
    let size = Vec2::new(available_w, height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let item_label = label.to_string();
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            item_label.clone(),
        )
    });

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let is_hovered = response.hovered();

        if is_hovered {
            painter.rect_filled(rect, 3.0, palette.bg_widget_hover);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                3.0,
                Stroke::new(1.5_f32, palette.border_focus),
                StrokeKind::Inside,
            );
        }

        let primary_color = text_color.unwrap_or(palette.text_primary);

        // 1. Icon (if provided)
        let text_start_x = if let Some(ico) = icon {
            let icon_rect = Rect::from_min_size(
                pos2(rect.min.x + padding_x, rect.center().y - icon_size / 2.0),
                Vec2::splat(icon_size),
            );
            paint_icon(painter, icon_rect, ico, primary_color);
            icon_rect.max.x + 8.0
        } else {
            rect.min.x + padding_x
        };

        // 2. Text label
        painter.text(
            pos2(text_start_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            primary_color,
        );

        // 3. Right-aligned shortcut
        if let Some(sc) = shortcut {
            let sc_color = if is_hovered {
                palette.text_secondary
            } else {
                palette.text_muted
            };
            painter.text(
                pos2(rect.max.x - padding_x, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                sc,
                egui::FontId::proportional(11.0),
                sc_color,
            );
        }
    }

    response
}

/// Danger/delete context menu item rendered in semantic error accent.
pub fn context_menu_item_danger(
    ui: &mut Ui,
    icon: Option<Icon>,
    label: &str,
    shortcut: Option<&str>,
) -> Response {
    let palette = crate::gui::theme::SemanticPalette::current(ui.ctx());
    context_menu_item(ui, icon, label, shortcut, Some(palette.error))
}

