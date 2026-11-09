use eframe::egui::{self, RichText, Vec2};

use crate::ui::theme;

/// Shows `add_contents` in a column of at most `width` points, centered in the free space.
///
/// Returns whatever `add_contents` returns.
pub fn centered_column<R>(
    ui: &mut egui::Ui,
    width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let width = width.min(ui.available_width());
    let margin = (ui.available_width() - width) / 2.0;
    ui.horizontal_top(|ui| {
        ui.add_space(margin);
        ui.vertical(|ui| {
            ui.set_width(width);
            add_contents(ui)
        })
        .inner
    })
    .inner
}

/// Draws `add_contents` inside a rounded card.
///
/// Returns whatever `add_contents` returns.
pub fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(theme::card_color(ui.visuals()))
        .corner_radius(10)
        .inner_margin(12)
        .show(ui, add_contents)
        .inner
}

/// A button as wide as the space it's in, with its text in the middle.
/// Returns true when it was clicked.
pub fn wide_button(ui: &mut egui::Ui, enabled: bool, text: &str) -> bool {
    let button = egui::Button::new(text).min_size(Vec2::new(0.0, 34.0));
    // "Justified" stretches the button to the full width; "centered" centers its text.
    ui.vertical_centered_justified(|ui| ui.add_enabled(enabled, button))
        .inner
        .clicked()
}

/// The main button of a screen: wide, big and in the accent color.
/// Returns true when it was clicked.
pub fn accent_button(ui: &mut egui::Ui, text: &str) -> bool {
    let text = RichText::new(text)
        .size(20.0)
        .strong()
        .color(theme::ON_ACCENT);
    let button = egui::Button::new(text)
        .fill(theme::ACCENT)
        .min_size(Vec2::new(0.0, 46.0));
    ui.vertical_centered_justified(|ui| ui.add(button))
        .inner
        .clicked()
}
