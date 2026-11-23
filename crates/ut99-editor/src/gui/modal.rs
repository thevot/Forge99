// crates/ut99-editor/src/gui/modal.rs
//
// Centralized window and modal management for Forge99.
// Guarantees responsive sizing, viewport containment, screen clamping,
// and consistent visual hierarchy across all studios, wizards, and dialogs.

use egui::{Align2, Context, Rect, ScrollArea, WidgetText, Window};

/// Standard semantic color tokens for Forge99 menus, buttons, and studio accents.
pub mod colors {
    use egui::Color32;

    /// Emerald Mint (#6ef0a0): Creation, Wizards, Build All, Commit Actions
    pub const ACCENT_MINT: Color32 = Color32::from_rgb(110, 240, 160);
    /// Sky Blue (#64d2ff): Geometry, CSG Add, BSP Rebuild
    pub const ACCENT_SKY: Color32 = Color32::from_rgb(100, 210, 255);
    /// Radiant Gold (#ffd750): Lighting, Radiosity, Presets, Bookmarks
    pub const ACCENT_GOLD: Color32 = Color32::from_rgb(255, 215, 80);
    /// Coral Orange (#ffaa46): CSG Subtract, Movers, Hazard, Physics
    pub const ACCENT_CORAL: Color32 = Color32::from_rgb(255, 170, 70);
    /// Cyber Green (#78eb96): Bot AI, Pathing, Simulator
    pub const ACCENT_CYBER: Color32 = Color32::from_rgb(120, 235, 150);
    /// Lavender Violet (#cda5ff): Audio, EAX Reverb, Music
    pub const ACCENT_VIOLET: Color32 = Color32::from_rgb(205, 165, 255);
    /// Tournament Crimson (#ff8282): Game Modes (CTF, DOM, AS), Weapons
    pub const ACCENT_CRIMSON: Color32 = Color32::from_rgb(255, 130, 130);
    /// Neon Cyan (#6ee6f0): Scripting, UC Wizard, FX, Decals
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(110, 230, 240);
    /// Danger Red (#ff7373): Destructive actions, Reset, Clear, Exit
    pub const DANGER_RED: Color32 = Color32::from_rgb(255, 115, 115);
}

/// Creates a standard editor window that:
/// 1. Clamps max size strictly within `ctx.screen_rect()`, leaving margins for toolbars and borders.
/// 2. Scales `default_size` so it never overflows smaller resolutions (e.g. 1366x768, 720p, or high DPI scaling).
/// 3. Enables `.constrain(true)` and `.resizable(true)` so the window cannot be lost off-screen.
/// 4. Centers the window by default.
pub fn standard_dialog_window<'a>(
    ctx: &Context,
    title: impl Into<WidgetText>,
    open: &'a mut bool,
    preferred_size: [f32; 2],
    min_size: [f32; 2],
) -> Window<'a> {
    let screen: Rect = ctx.screen_rect();
    let max_w = (screen.width() - 28.0).max(320.0);
    let max_h = (screen.height() - 44.0).max(240.0);

    let default_w = preferred_size[0].min(max_w);
    let default_h = preferred_size[1].min(max_h);

    let min_w = min_size[0].min(max_w);
    let min_h = min_size[1].min(max_h);

    Window::new(title)
        .open(open)
        .default_size([default_w, default_h])
        .max_size([max_w, max_h])
        .min_width(min_w)
        .min_height(min_h)
        .default_pos(screen.center())
        .pivot(Align2::CENTER_CENTER)
        .collapsible(false)
        .resizable(true)
        .constrain(true)
}

/// Creates a studio dialog window with responsive defaults suitable for large 2-column or tabbed studios.
pub fn studio_dialog_window<'a>(
    ctx: &Context,
    title: impl Into<WidgetText>,
    open: &'a mut bool,
) -> Window<'a> {
    let screen = ctx.screen_rect();
    let preferred_w = (screen.width() * 0.88).clamp(740.0, 1080.0);
    let preferred_h = (screen.height() * 0.84).clamp(520.0, 750.0);
    standard_dialog_window(ctx, title, open, [preferred_w, preferred_h], [560.0, 380.0])
}

/// Creates a compact utility dialog window (e.g. alignment, bookmarks, simple property adjusters).
pub fn compact_dialog_window<'a>(
    ctx: &Context,
    title: impl Into<WidgetText>,
    open: &'a mut bool,
) -> Window<'a> {
    let screen = ctx.screen_rect();
    let preferred_w = (screen.width() * 0.5).clamp(420.0, 620.0);
    let preferred_h = (screen.height() * 0.6).clamp(340.0, 520.0);
    standard_dialog_window(ctx, title, open, [preferred_w, preferred_h], [360.0, 260.0])
}

/// Renders the body of a dialog inside a full-height vertical scroll area.
/// This prevents any button, control, or footer from getting clipped on smaller screens.
pub fn dialog_scroll_body<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, add_contents)
        .inner
}

/// Renders a responsive two-column studio layout threading mutable `state`:
/// - When `ui.available_width() >= breakpoint_px`, renders side-by-side columns using `left_ratio` (clamped `0.25..=0.75`).
/// - When `ui.available_width() < breakpoint_px` (compact windows, 720p screens, or high DPI scale),
///   stacks the two panels vertically with a subtle separator so neither column gets crushed.
pub fn responsive_two_columns<S>(
    ui: &mut egui::Ui,
    state: &mut S,
    breakpoint_px: f32,
    left_ratio: f32,
    left_fn: impl FnOnce(&mut egui::Ui, &mut S),
    right_fn: impl FnOnce(&mut egui::Ui, &mut S),
) {
    let total_w = ui.available_width();
    if total_w >= breakpoint_px {
        let ratio = left_ratio.clamp(0.25, 0.75);
        let gap = 12.0_f32;
        let left_w = ((total_w - gap) * ratio).max(200.0);
        let right_w = (total_w - gap - left_w).max(200.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| left_fn(ui, state),
            );
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.allocate_ui_with_layout(
                egui::vec2(right_w, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| right_fn(ui, state),
            );
        });
    } else {
        ui.vertical(|ui| {
            left_fn(ui, state);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            right_fn(ui, state);
        });
    }
}

/// Closes a dialog (`*open = false`) when the user presses `Escape` and no text box has keyboard focus.
pub fn close_on_escape_if_unfocused(ctx: &Context, open: &mut bool) {
    if *open && !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        *open = false;
    }
}

/// Renders a full-screen semi-transparent backdrop in `egui::Order::Middle` (above all background
/// panels and standard `Order::Middle` windows, but strictly below `egui::Order::Foreground` modal
/// windows and cards).
///
/// Always pair this with `.order(egui::Order::Foreground)` on the modal `egui::Window` or `egui::Area`
/// so that the modal itself is never dimmed by the backdrop layer.
pub fn render_modal_backdrop(ctx: &Context, id_source: impl std::hash::Hash, color: egui::Color32) {
    let screen_rect = ctx.screen_rect();
    if screen_rect.width() <= 1.0 || screen_rect.height() <= 1.0 {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new(id_source),
    ));
    painter.rect_filled(screen_rect, 0.0, color);
}


