use eframe::egui::{self, Color32, CornerRadius, Theme, Vec2, Visuals};

use crate::settings::{AppTheme, BoardTheme};

/// The one accent color of the app: selected things, the active clock, the main button.
pub const ACCENT: Color32 = Color32::from_rgb(129, 182, 76);
/// Dark text, readable on the accent color.
pub const ON_ACCENT: Color32 = Color32::from_rgb(24, 32, 16);
/// Only for bad news: low time, errors.
pub const DANGER: Color32 = Color32::from_rgb(200, 60, 50);

/// Returns the (light, dark) square colors of a board theme.
pub fn board_colors(theme: BoardTheme) -> (Color32, Color32) {
    match theme {
        BoardTheme::Green => (
            Color32::from_rgb(238, 238, 210),
            Color32::from_rgb(118, 150, 86),
        ),
        BoardTheme::Wood => (
            Color32::from_rgb(240, 217, 181),
            Color32::from_rgb(181, 136, 99),
        ),
        BoardTheme::Ice => (
            Color32::from_rgb(222, 233, 242),
            Color32::from_rgb(122, 156, 192),
        ),
        BoardTheme::Slate => (
            Color32::from_rgb(169, 180, 194),
            Color32::from_rgb(75, 90, 107),
        ),
    }
}

/// Sets the spacing, roundness and accent color of every widget, in dark and light mode.
/// Called once, when the app starts.
pub fn install_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 6.0);

        style.visuals.selection.bg_fill = ACCENT;
        style.visuals.selection.stroke.color = Color32::WHITE;

        let widgets = &mut style.visuals.widgets;
        for state in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            state.corner_radius = CornerRadius::same(8);
        }
    });

    // Warm dark greys instead of egui's default blue-grey.
    ctx.style_mut_of(Theme::Dark, |style| {
        style.visuals.panel_fill = Color32::from_rgb(38, 36, 33);
        style.visuals.window_fill = Color32::from_rgb(48, 46, 43);
        style.visuals.faint_bg_color = Color32::from_rgb(62, 59, 55); // striped rows
        style.visuals.extreme_bg_color = Color32::from_rgb(28, 27, 25);
    });
    ctx.style_mut_of(Theme::Light, |style| {
        style.visuals.faint_bg_color = Color32::from_rgb(226, 224, 220);
    });
}

/// Returns the background color of a card: a bit lighter than the panel in dark mode,
/// a bit darker in light mode.
pub fn card_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(50, 47, 44)
    } else {
        Color32::from_rgb(237, 235, 231)
    }
}

/// Switches the whole app to dark or light mode.
pub fn apply_app_theme(ctx: &egui::Context, app_theme: AppTheme) {
    let theme = match app_theme {
        AppTheme::Dark => Theme::Dark,
        AppTheme::Light => Theme::Light,
    };
    ctx.set_theme(theme);
}
