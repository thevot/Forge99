use serde::{Deserialize, Serialize};
use egui::{Color32, FontFamily, FontId, Frame, Margin, Response, RichText, Stroke, TextStyle, Ui, Visuals};

/// Visual UI themes available in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppTheme {
    /// Unreal Precision Slate Dark (Modern dark slate, UT Gold & Azure accents)
    #[default]
    UnrealSlateDark,
    /// Unreal Precision Slate Light (Daylight studio CAD palette with high-contrast ink)
    UnrealSlateLight,
    /// Classic UnrealEd 2.0 Retro (Windows 98/2000 industrial slate-gray with classic accents)
    ClassicUnrealEd,
    /// High-Contrast Dark OLED (Deep black canvas, pure white text, electric cyan/amber, WCAG AAA)
    HighContrastDark,
    /// Cyberpunk Tournament Neon (Dark carbon, neon tournament green/cyan matrix)
    CyberTournament,
}

impl AppTheme {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::UnrealSlateDark => "Unreal Precision Slate Dark (Default)",
            Self::UnrealSlateLight => "Unreal Precision Slate Light (Daylight CAD)",
            Self::ClassicUnrealEd => "Classic UnrealEd 2.0 Retro",
            Self::HighContrastDark => "High-Contrast Dark (OLED / WCAG AAA)",
            Self::CyberTournament => "Cyberpunk Tournament Neon",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::UnrealSlateDark => "Modern obsidian canvas with slate panels and iconic UT Gold / Azure focus rings.",
            Self::UnrealSlateLight => "Clean daylight studio theme with crisp cool-slate surfaces and deep high-contrast ink.",
            Self::ClassicUnrealEd => "Industrial slate-gray look inspired by UnrealEd 2.0 and Windows 98/2000 ergonomics.",
            Self::HighContrastDark => "True pitch-black canvas with ultra-high contrast typography and bright focus borders.",
            Self::CyberTournament => "High-tech combat arena theme with emerald carbon panels and cyan neon highlights.",
        }
    }

    pub fn is_light(&self) -> bool {
        matches!(self, Self::UnrealSlateLight)
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::UnrealSlateDark,
            Self::UnrealSlateLight,
            Self::ClassicUnrealEd,
            Self::HighContrastDark,
            Self::CyberTournament,
        ]
    }
}

/// Dynamic, theme-aware semantic design token palette (WCAG 2.2 AA verified).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticPalette {
    pub theme: AppTheme,
    pub bg_canvas: Color32,
    pub bg_panel: Color32,
    pub bg_panel_alt: Color32,
    pub bg_widget: Color32,
    pub bg_widget_hover: Color32,
    pub bg_widget_active: Color32,
    pub border_subtle: Color32,
    pub border_focus: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent_azure: Color32,
    pub accent_gold: Color32,
    pub csg_add: Color32,
    pub csg_sub: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
}

impl Default for SemanticPalette {
    fn default() -> Self {
        Self::for_theme(AppTheme::UnrealSlateDark)
    }
}

impl SemanticPalette {
    /// Returns the semantic color palette for the requested [`AppTheme`].
    pub const fn for_theme(theme: AppTheme) -> Self {
        match theme {
            AppTheme::UnrealSlateDark => Self {
                theme,
                bg_canvas: Color32::from_rgb(14, 16, 20),
                bg_panel: Color32::from_rgb(22, 25, 30),
                bg_panel_alt: Color32::from_rgb(28, 32, 38),
                bg_widget: Color32::from_rgb(36, 41, 50),
                bg_widget_hover: Color32::from_rgb(48, 55, 68),
                bg_widget_active: Color32::from_rgb(60, 69, 84),
                border_subtle: Color32::from_rgb(48, 55, 68),
                border_focus: Color32::from_rgb(56, 139, 253),
                text_primary: Color32::from_rgb(240, 244, 248),
                text_secondary: Color32::from_rgb(176, 186, 201),
                text_muted: Color32::from_rgb(142, 152, 168),
                accent_azure: Color32::from_rgb(56, 139, 253),
                accent_gold: Color32::from_rgb(245, 182, 45),
                csg_add: Color32::from_rgb(45, 124, 232),
                csg_sub: Color32::from_rgb(230, 160, 35),
                success: Color32::from_rgb(46, 194, 106),
                warning: Color32::from_rgb(245, 158, 42),
                error: Color32::from_rgb(240, 82, 82),
            },
            AppTheme::UnrealSlateLight => Self {
                theme,
                bg_canvas: Color32::from_rgb(228, 233, 240),
                bg_panel: Color32::from_rgb(244, 246, 250),
                bg_panel_alt: Color32::from_rgb(255, 255, 255),
                bg_widget: Color32::from_rgb(230, 235, 242),
                bg_widget_hover: Color32::from_rgb(214, 222, 234),
                bg_widget_active: Color32::from_rgb(196, 208, 224),
                border_subtle: Color32::from_rgb(190, 199, 212),
                border_focus: Color32::from_rgb(18, 94, 204),
                text_primary: Color32::from_rgb(18, 22, 29),
                text_secondary: Color32::from_rgb(52, 60, 74),
                text_muted: Color32::from_rgb(80, 90, 106),
                accent_azure: Color32::from_rgb(18, 94, 204),
                accent_gold: Color32::from_rgb(164, 98, 0),
                csg_add: Color32::from_rgb(18, 94, 204),
                csg_sub: Color32::from_rgb(176, 88, 0),
                success: Color32::from_rgb(18, 122, 62),
                warning: Color32::from_rgb(176, 88, 0),
                error: Color32::from_rgb(192, 34, 34),
            },
            AppTheme::ClassicUnrealEd => Self {
                theme,
                bg_canvas: Color32::from_rgb(24, 28, 34),
                bg_panel: Color32::from_rgb(36, 40, 46),
                bg_panel_alt: Color32::from_rgb(44, 49, 56),
                bg_widget: Color32::from_rgb(52, 58, 68),
                bg_widget_hover: Color32::from_rgb(66, 74, 86),
                bg_widget_active: Color32::from_rgb(80, 90, 104),
                border_subtle: Color32::from_rgb(68, 76, 88),
                border_focus: Color32::from_rgb(235, 175, 50),
                text_primary: Color32::from_rgb(242, 245, 248),
                text_secondary: Color32::from_rgb(190, 198, 210),
                text_muted: Color32::from_rgb(158, 168, 182),
                accent_azure: Color32::from_rgb(90, 168, 255),
                accent_gold: Color32::from_rgb(235, 175, 50),
                csg_add: Color32::from_rgb(72, 152, 245),
                csg_sub: Color32::from_rgb(235, 175, 50),
                success: Color32::from_rgb(64, 204, 118),
                warning: Color32::from_rgb(245, 168, 55),
                error: Color32::from_rgb(245, 95, 95),
            },
            AppTheme::HighContrastDark => Self {
                theme,
                bg_canvas: Color32::from_rgb(0, 0, 0),
                bg_panel: Color32::from_rgb(10, 10, 12),
                bg_panel_alt: Color32::from_rgb(16, 16, 20),
                bg_widget: Color32::from_rgb(24, 24, 30),
                bg_widget_hover: Color32::from_rgb(40, 40, 50),
                bg_widget_active: Color32::from_rgb(56, 56, 70),
                border_subtle: Color32::from_rgb(110, 120, 138),
                border_focus: Color32::from_rgb(80, 210, 255),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_secondary: Color32::from_rgb(224, 232, 240),
                text_muted: Color32::from_rgb(185, 196, 210),
                accent_azure: Color32::from_rgb(80, 210, 255),
                accent_gold: Color32::from_rgb(255, 215, 55),
                csg_add: Color32::from_rgb(80, 210, 255),
                csg_sub: Color32::from_rgb(255, 215, 55),
                success: Color32::from_rgb(75, 240, 135),
                warning: Color32::from_rgb(255, 195, 50),
                error: Color32::from_rgb(255, 105, 105),
            },
            AppTheme::CyberTournament => Self {
                theme,
                bg_canvas: Color32::from_rgb(8, 14, 12),
                bg_panel: Color32::from_rgb(16, 24, 20),
                bg_panel_alt: Color32::from_rgb(22, 32, 28),
                bg_widget: Color32::from_rgb(28, 42, 36),
                bg_widget_hover: Color32::from_rgb(38, 58, 50),
                bg_widget_active: Color32::from_rgb(50, 76, 64),
                border_subtle: Color32::from_rgb(48, 76, 64),
                border_focus: Color32::from_rgb(0, 235, 165),
                text_primary: Color32::from_rgb(235, 252, 244),
                text_secondary: Color32::from_rgb(176, 208, 194),
                text_muted: Color32::from_rgb(140, 174, 160),
                accent_azure: Color32::from_rgb(0, 235, 165),
                accent_gold: Color32::from_rgb(245, 196, 55),
                csg_add: Color32::from_rgb(45, 200, 245),
                csg_sub: Color32::from_rgb(245, 180, 45),
                success: Color32::from_rgb(45, 235, 145),
                warning: Color32::from_rgb(250, 175, 45),
                error: Color32::from_rgb(250, 90, 95),
            },
        }
    }

    /// Retrieves the active [`SemanticPalette`] stored in the [`egui::Context`].
    pub fn current(ctx: &egui::Context) -> Self {
        ctx.data(|d| d.get_temp::<Self>(egui::Id::new("forge99_semantic_palette")))
            .unwrap_or_default()
    }
}

/// Core design tokens for Unreal Precision Slate Dark theme (and backward-compatible constants).
pub struct ThemeColors;

impl ThemeColors {
    pub const BG_CANVAS: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).bg_canvas;
    pub const BG_PANEL: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).bg_panel;
    pub const BG_PANEL_ALT: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).bg_panel_alt;
    pub const BG_WIDGET: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).bg_widget;
    pub const BG_WIDGET_HOVER: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).bg_widget_hover;
    pub const BG_WIDGET_ACTIVE: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).bg_widget_active;

    pub const BORDER_SUBTLE: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).border_subtle;
    pub const BORDER_FOCUS: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).border_focus;

    pub const TEXT_PRIMARY: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).text_primary;
    pub const TEXT_SECONDARY: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).text_secondary;
    pub const TEXT_MUTED: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).text_muted;

    // Functional accents
    pub const ACCENT_GOLD: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).accent_gold;
    pub const ACCENT_AZURE: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).accent_azure;
    pub const CSG_ADD: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).csg_add;
    pub const CSG_SUB: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).csg_sub;
    pub const SUCCESS: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).success;
    pub const WARNING: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).warning;
    pub const ERROR: Color32 = SemanticPalette::for_theme(AppTheme::UnrealSlateDark).error;
}

pub fn configure_system_fallback_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut candidates: Vec<(&'static str, std::path::PathBuf)> = Vec::new();

    // Dynamically resolve Windows system directory if present
    if let Ok(sys_root) = std::env::var("SystemRoot").or_else(|_| std::env::var("WINDIR")) {
        let root = std::path::PathBuf::from(sys_root).join("Fonts");
        candidates.push(("segoe_ui_symbol", root.join("seguisym.ttf")));
        candidates.push(("segoe_ui_emoji", root.join("seguiemj.ttf")));
    } else {
        candidates.push(("segoe_ui_symbol", std::path::PathBuf::from(r"C:\Windows\Fonts\seguisym.ttf")));
        candidates.push(("segoe_ui_emoji", std::path::PathBuf::from(r"C:\Windows\Fonts\seguiemj.ttf")));
    }
    // Unix / macOS common fallback locations
    candidates.push(("noto_color_emoji", std::path::PathBuf::from("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf")));
    candidates.push(("apple_color_emoji", std::path::PathBuf::from("/System/Library/Fonts/Apple Color Emoji.ttc")));

    let mut loaded_any = false;
    for (font_key, font_path) in candidates {
        if font_path.exists() {
            if let Ok(bytes) = std::fs::read(&font_path) {
                fonts.font_data.insert(
                    font_key.to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(bytes)),
                );
                if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
                    prop.push(font_key.to_owned());
                }
                if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
                    mono.push(font_key.to_owned());
                }
                loaded_any = true;
            }
        }
    }
    if loaded_any {
        ctx.set_fonts(fonts);
    }
}

/// Applies a specific theme variant to the egui context and stores its [`SemanticPalette`].
pub fn apply_theme_variant(ctx: &egui::Context, theme: AppTheme) {
    configure_system_fallback_fonts(ctx);

    let palette = SemanticPalette::for_theme(theme);
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("forge99_semantic_palette"), palette);
    });

    let mut visuals = if theme.is_light() {
        Visuals::light()
    } else {
        Visuals::dark()
    };

    let active_text = if theme.is_light() {
        palette.text_primary
    } else {
        Color32::WHITE
    };

    // Backgrounds
    visuals.panel_fill = palette.bg_panel;
    visuals.window_fill = palette.bg_panel_alt;
    visuals.extreme_bg_color = palette.bg_canvas;
    visuals.faint_bg_color = palette.bg_panel;
    visuals.override_text_color = None;

    // Window shadow & borders
    visuals.window_stroke = Stroke::new(1.0_f32, palette.border_subtle);
    visuals.window_corner_radius = 6.into();

    // Widgets noninteractive
    visuals.widgets.noninteractive.bg_fill = palette.bg_panel;
    visuals.widgets.noninteractive.weak_bg_fill = palette.bg_panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, palette.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.text_primary);
    visuals.widgets.noninteractive.corner_radius = 4.into();

    // Widgets inactive
    visuals.widgets.inactive.bg_fill = palette.bg_widget;
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.border_subtle);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.text_primary);
    visuals.widgets.inactive.corner_radius = 4.into();

    // Widgets hovered
    visuals.widgets.hovered.bg_fill = palette.bg_widget_hover;
    visuals.widgets.hovered.weak_bg_fill = palette.bg_widget_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.accent_azure);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, active_text);
    visuals.widgets.hovered.corner_radius = 4.into();

    // Widgets active / clicked
    visuals.widgets.active.bg_fill = palette.bg_widget_active;
    visuals.widgets.active.weak_bg_fill = palette.bg_widget_active;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, palette.accent_azure);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, active_text);
    visuals.widgets.active.corner_radius = 4.into();

    // Widgets open
    visuals.widgets.open.bg_fill = palette.bg_widget_hover;
    visuals.widgets.open.weak_bg_fill = palette.bg_widget_hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, palette.border_subtle);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, palette.text_primary);
    visuals.widgets.open.corner_radius = 4.into();

    // Selection & Focus Rings (WCAG 2.4.7 / 2.4.11 / 2.4.13)
    visuals.selection.bg_fill = palette.accent_azure.gamma_multiply(if theme.is_light() { 0.22 } else { 0.35 });
    visuals.selection.stroke = Stroke::new(1.5_f32, palette.border_focus);

    ctx.set_visuals(visuals);

    // Layout Style & 4px/8px Grid Spacings
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.interact_size = egui::vec2(40.0, 22.0);
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.indent = 16.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.bar_inner_margin = 2.0;

    // Typography hierarchy
    style.text_styles = [
        (TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Name("Subheading".into()), FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(11.5, FontFamily::Monospace)),
    ].into();

    ctx.set_style(style);
}

/// Backward compatible theme application defaulting to Unreal Precision Slate Dark.
pub fn apply_theme(ctx: &egui::Context) {
    apply_theme_variant(ctx, AppTheme::UnrealSlateDark);
}

/// Computes the relative luminance of a color according to W3C WCAG 2.1 formula.
pub fn relative_luminance(c: Color32) -> f32 {
    fn srgb_to_linear(val: u8) -> f32 {
        let v = val as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * srgb_to_linear(c.r()) + 0.7152 * srgb_to_linear(c.g()) + 0.0722 * srgb_to_linear(c.b())
}

/// Computes the WCAG contrast ratio between two colors (range: 1.0 to 21.0).
pub fn wcag_contrast_ratio(c1: Color32, c2: Color32) -> f32 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

/// Checks whether the contrast ratio meets WCAG AA criteria.
/// - Normal text: >= 4.5
/// - Large text or UI components / graphical borders: >= 3.0
pub fn check_wcag_aa(fg: Color32, bg: Color32, is_large_or_ui: bool) -> bool {
    let ratio = wcag_contrast_ratio(fg, bg);
    if is_large_or_ui {
        ratio >= 3.0
    } else {
        ratio >= 4.5
    }
}

/// Standardized rich tooltip with keycap shortcut badge and high contrast typography (01)
pub fn pro_tooltip(ui: &mut Ui, title: &str, shortcut: Option<&str>, description: &str) {
    let palette = SemanticPalette::current(ui.ctx());
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).strong().color(palette.text_primary));
            if let Some(sc) = shortcut {
                if !sc.is_empty() {
                    Frame::NONE
                        .fill(palette.bg_canvas)
                        .stroke(Stroke::new(1.0_f32, palette.border_subtle))
                        .corner_radius(3.0)
                        .inner_margin(Margin::symmetric(4, 1))
                        .show(ui, |ui| {
                            ui.label(RichText::new(sc).monospace().size(10.5).color(palette.accent_azure));
                        });
                }
            }
        });
        if !description.is_empty() {
            ui.add_space(2.0);
            ui.label(RichText::new(description).size(11.0).color(palette.text_secondary));
        }
    });
}

pub fn pro_tooltip_ext(
    ui: &mut Ui,
    title: &str,
    shortcut: Option<&str>,
    description: &str,
    modifiers_hint: Option<&str>,
    status: Option<&str>,
) {
    let palette = SemanticPalette::current(ui.ctx());
    pro_tooltip(ui, title, shortcut, description);
    if let Some(mods) = modifiers_hint {
        if !mods.is_empty() {
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!("⌨ {mods}"))
                    .size(10.5)
                    .color(palette.accent_gold),
            );
        }
    }
    if let Some(st) = status {
        if !st.is_empty() {
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!("● {st}"))
                    .monospace()
                    .size(10.5)
                    .color(palette.success),
            );
        }
    }
}

/// 4-Level comprehensive tooltip: Title/Keycap, Description, Modifiers/Status, and Historical UnrealEd Gotchas
pub fn pro_tooltip_4level(
    ui: &mut Ui,
    title: &str,
    shortcut: Option<&str>,
    description: &str,
    modifiers_hint: Option<&str>,
    status: Option<&str>,
    unrealed2_gotcha: Option<&str>,
) {
    let palette = SemanticPalette::current(ui.ctx());
    pro_tooltip_ext(ui, title, shortcut, description, modifiers_hint, status);
    if let Some(gotcha) = unrealed2_gotcha {
        if !gotcha.is_empty() {
            ui.add_space(4.0);
            let (box_fill, box_stroke, title_col, body_col) = if palette.theme.is_light() {
                (
                    Color32::from_rgb(255, 246, 224),
                    Color32::from_rgb(204, 142, 24),
                    Color32::from_rgb(138, 82, 0),
                    Color32::from_rgb(68, 44, 8),
                )
            } else {
                (
                    Color32::from_rgba_premultiplied(40, 32, 10, 220),
                    Color32::from_rgb(200, 150, 40),
                    Color32::from_rgb(255, 205, 90),
                    Color32::from_rgb(240, 225, 185),
                )
            };
            Frame::NONE
                .fill(box_fill)
                .stroke(Stroke::new(1.0_f32, box_stroke))
                .corner_radius(3.0)
                .inner_margin(Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.label(RichText::new("UnrealEd 2.0 Best Practice:").strong().size(10.5).color(title_col));
                    ui.label(RichText::new(gotcha).italics().size(10.0).color(body_col));
                });
        }
    }
}

pub trait ProTooltipExt {
    fn pro_tooltip(self, title: &str, shortcut: Option<&str>, description: &str) -> Self;
    fn pro_tooltip_ext(
        self,
        title: &str,
        shortcut: Option<&str>,
        description: &str,
        modifiers_hint: Option<&str>,
        status: Option<&str>,
    ) -> Self;
    fn pro_tooltip_4level(
        self,
        title: &str,
        shortcut: Option<&str>,
        description: &str,
        modifiers_hint: Option<&str>,
        status: Option<&str>,
        unrealed2_gotcha: Option<&str>,
    ) -> Self;
}

impl ProTooltipExt for Response {
    fn pro_tooltip(self, title: &str, shortcut: Option<&str>, description: &str) -> Self {
        self.on_hover_ui(|ui| {
            pro_tooltip(ui, title, shortcut, description);
        })
    }

    fn pro_tooltip_ext(
        self,
        title: &str,
        shortcut: Option<&str>,
        description: &str,
        modifiers_hint: Option<&str>,
        status: Option<&str>,
    ) -> Self {
        self.on_hover_ui(|ui| {
            pro_tooltip_ext(ui, title, shortcut, description, modifiers_hint, status);
        })
    }

    fn pro_tooltip_4level(
        self,
        title: &str,
        shortcut: Option<&str>,
        description: &str,
        modifiers_hint: Option<&str>,
        status: Option<&str>,
        unrealed2_gotcha: Option<&str>,
    ) -> Self {
        self.on_hover_ui(|ui| {
            pro_tooltip_4level(ui, title, shortcut, description, modifiers_hint, status, unrealed2_gotcha);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_themes_wcag_2_2_aa_text_and_focus_contrast() {
        for &theme in AppTheme::all() {
            let p = SemanticPalette::for_theme(theme);
            let name = theme.display_name();

            // Primary, Secondary, and Muted text must pass WCAG AA (>= 4.5:1) against panel and canvas backgrounds
            for (bg_name, bg) in [("bg_panel", p.bg_panel), ("bg_panel_alt", p.bg_panel_alt), ("bg_canvas", p.bg_canvas)] {
                let r_pri = wcag_contrast_ratio(p.text_primary, bg);
                let r_sec = wcag_contrast_ratio(p.text_secondary, bg);
                let r_mut = wcag_contrast_ratio(p.text_muted, bg);

                assert!(
                    r_pri >= 4.5,
                    "[{name}] text_primary on {bg_name} failed WCAG AA: {r_pri:.2}:1"
                );
                assert!(
                    r_sec >= 4.5,
                    "[{name}] text_secondary on {bg_name} failed WCAG AA: {r_sec:.2}:1"
                );
                assert!(
                    r_mut >= 4.5,
                    "[{name}] text_muted on {bg_name} failed WCAG AA: {r_mut:.2}:1"
                );
            }

            // Focus ring (WCAG 2.4.11 / 1.4.11) must have >= 3.0:1 contrast against panel backgrounds
            let r_focus = wcag_contrast_ratio(p.border_focus, p.bg_panel);
            assert!(
                r_focus >= 3.0,
                "[{name}] border_focus on bg_panel failed WCAG 2.2 AA (3:1): {r_focus:.2}:1"
            );

            // Functional status colors must be legible (>= 3.5:1 for UI/badges) on panel background
            for (st_name, col) in [("success", p.success), ("warning", p.warning), ("error", p.error)] {
                let r_st = wcag_contrast_ratio(col, p.bg_panel);
                assert!(
                    r_st >= 3.5,
                    "[{name}] status '{st_name}' on bg_panel too low contrast: {r_st:.2}:1"
                );
            }
        }
    }

    #[test]
    fn test_high_contrast_theme_meets_wcag_aaa() {
        let hc = SemanticPalette::for_theme(AppTheme::HighContrastDark);
        assert!(wcag_contrast_ratio(hc.text_primary, hc.bg_panel) >= 15.0);
        assert!(wcag_contrast_ratio(hc.text_secondary, hc.bg_panel) >= 10.0);
        assert!(wcag_contrast_ratio(hc.text_muted, hc.bg_panel) >= 7.0);
    }
}



