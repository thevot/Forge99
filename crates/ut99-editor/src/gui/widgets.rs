//! Reusable, Theme-Aware, WCAG 2.2 AA Accessible UI Component Library (`gui/widgets.rs`)
//!
//! Centralizes common layout patterns (section cards, property rows, segmented pill selectors,
//! status badges, search filter bars, and empty-state placeholders) so dialogs and studios
//! inherit [`SemanticPalette`] automatically without hardcoded RGB values.

use egui::{CornerRadius, Frame, Margin, Response, RichText, Stroke, Ui, Vec2};
use crate::gui::icons::{self, Icon};
use crate::gui::theme::{ProTooltipExt, SemanticPalette};

/// Semantic status tone for badges, banners, and telemetry indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Success,
    Warning,
    Danger,
    Info,
    Neutral,
}

impl StatusTone {
    pub fn resolve_colors(self, palette: &SemanticPalette) -> (egui::Color32, egui::Color32, egui::Color32) {
        let fg = match self {
            Self::Success => palette.success,
            Self::Warning => palette.warning,
            Self::Danger => palette.error,
            Self::Info => palette.accent_azure,
            Self::Neutral => palette.text_secondary,
        };
        let fill_alpha = if palette.theme.is_light() { 0.14 } else { 0.18 };
        let stroke_alpha = if palette.theme.is_light() { 0.45 } else { 0.55 };
        (
            fg,
            fg.gamma_multiply(fill_alpha),
            fg.gamma_multiply(stroke_alpha),
        )
    }
}

/// Renders a clean, bordered section card with an optional vector icon and strong header title.
pub fn section_card<R>(
    ui: &mut Ui,
    icon: Option<Icon>,
    title: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let palette = SemanticPalette::current(ui.ctx());
    Frame::NONE
        .fill(palette.bg_panel)
        .stroke(Stroke::new(1.0_f32, palette.border_subtle))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            if !title.is_empty() || icon.is_some() {
                ui.horizontal(|ui| {
                    if let Some(ico) = icon {
                        icons::icon(ui, ico, Vec2::splat(14.0), palette.accent_azure);
                    }
                    if !title.is_empty() {
                        ui.label(
                            RichText::new(title)
                                .strong()
                                .size(12.5)
                                .color(palette.text_primary),
                        );
                    }
                });
                ui.add_space(6.0);
            }
            add_contents(ui)
        })
        .inner
}

/// Renders a standardized two-part property row (`label` on the left with minimum width and optional tooltip,
/// interactive control on the right).
pub fn property_row<R>(
    ui: &mut Ui,
    label: &str,
    tooltip: Option<&str>,
    label_min_width: f32,
    add_control: impl FnOnce(&mut Ui) -> R,
) -> R {
    let palette = SemanticPalette::current(ui.ctx());
    ui.horizontal(|ui| {
        let lbl_resp = ui.add_sized(
            [label_min_width, 22.0],
            egui::Label::new(RichText::new(label).size(12.0).color(palette.text_secondary)),
        );
        if let Some(tip) = tooltip {
            if !tip.is_empty() {
                lbl_resp.pro_tooltip(label, None, tip);
            }
        }
        add_control(ui)
    })
    .inner
}

/// Renders a compact, accessible segmented pill bar where each option meets the `24px` minimum height.
pub fn segmented_pills<T: Copy + PartialEq>(
    ui: &mut Ui,
    current: &mut T,
    options: &[(T, Option<Icon>, &str, &str)],
) -> bool {
    let palette = SemanticPalette::current(ui.ctx());
    let mut changed = false;

    Frame::NONE
        .fill(palette.bg_canvas)
        .stroke(Stroke::new(1.0_f32, palette.border_subtle))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(3, 2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.horizontal(|ui| {
                for &(val, maybe_icon, label, tip) in options {
                    let is_active = *current == val;
                    let resp = if let Some(ico) = maybe_icon {
                        icons::icon_label_button(ui, ico, Some(label), is_active)
                    } else {
                        let btn = egui::Button::new(
                            RichText::new(label).size(11.5).color(if is_active {
                                palette.text_primary
                            } else {
                                palette.text_secondary
                            }),
                        )
                        .min_size(Vec2::new(32.0, 24.0))
                        .fill(if is_active {
                            palette.bg_widget_active
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .stroke(if is_active {
                            Stroke::new(1.0_f32, palette.accent_azure)
                        } else {
                            Stroke::NONE
                        });
                        ui.add(btn)
                    };
                    let resp = if !tip.is_empty() {
                        resp.pro_tooltip(label, None, tip)
                    } else {
                        resp
                    };
                    if resp.clicked() && !is_active {
                        *current = val;
                        changed = true;
                    }
                }
            });
        });

    changed
}

/// Renders a semantic pill badge (`Success`, `Warning`, `Danger`, `Info`, `Neutral`).
pub fn status_badge(ui: &mut Ui, tone: StatusTone, label: &str) -> Response {
    let palette = SemanticPalette::current(ui.ctx());
    let (fg, bg, border) = tone.resolve_colors(&palette);
    Frame::NONE
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(11.0).strong().color(fg))
        })
        .inner
}

/// Renders a search filter input bar with a vector search icon and a 24x24px clear button when non-empty.
pub fn search_filter_bar(ui: &mut Ui, query: &mut String, hint: &str) -> bool {
    let palette = SemanticPalette::current(ui.ctx());
    let mut changed = false;
    ui.horizontal(|ui| {
        icons::icon(ui, Icon::Search, Vec2::splat(14.0), palette.text_muted);
        let desired_w = (ui.available_width() - if query.is_empty() { 4.0 } else { 32.0 }).max(80.0);
        let resp = ui.add(
            egui::TextEdit::singleline(query)
                .hint_text(hint)
                .desired_width(desired_w),
        );
        if resp.changed() {
            changed = true;
        }
        if !query.is_empty() {
            let clear_btn = ui.add_sized(
                [24.0, 24.0],
                egui::Button::new(RichText::new("×").size(13.0).color(palette.text_secondary)),
            );
            if clear_btn
                .pro_tooltip("Clear Filter", Some("Esc"), "Reset the current search filter.")
                .clicked()
            {
                query.clear();
                changed = true;
            }
        }
    });
    changed
}

/// Renders a clean, centered empty-state message when a list, outliner, or studio has no items.
pub fn empty_state_card(ui: &mut Ui, icon: Icon, title: &str, subtitle: &str) {
    let palette = SemanticPalette::current(ui.ctx());
    Frame::NONE
        .fill(palette.bg_canvas)
        .stroke(Stroke::new(1.0_f32, palette.border_subtle))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(16, 18))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                icons::icon(ui, icon, Vec2::splat(22.0), palette.text_muted);
                ui.add_space(6.0);
                ui.label(RichText::new(title).strong().size(12.5).color(palette.text_secondary));
                if !subtitle.is_empty() {
                    ui.add_space(2.0);
                    ui.label(RichText::new(subtitle).size(11.0).color(palette.text_muted));
                }
            });
        });
}
