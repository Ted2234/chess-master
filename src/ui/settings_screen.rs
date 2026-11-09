use eframe::egui::{self, Align2, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

use crate::settings::{AppTheme, BoardTheme, Settings};
use crate::ui::theme;
use crate::ui::widgets;

/// The width of the column the settings are shown in.
const COLUMN_WIDTH: f32 = 540.0;
/// The width and height of the small board drawn for each theme.
const PREVIEW_SIZE: f32 = 120.0;
/// The room under each preview for the theme's name.
const NAME_HEIGHT: f32 = 28.0;

/// Draws the Settings screen. Returns true when the player wants to go back.
pub fn draw_settings(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut back = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        widgets::centered_column(ui, COLUMN_WIDTH, |ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("Settings").size(36.0).strong());
            ui.add_space(20.0);

            section_title(ui, "Board");
            ui.horizontal(|ui| {
                for board_theme in BoardTheme::ALL {
                    let selected = settings.board_theme == board_theme;
                    if theme_preview(ui, board_theme, selected).clicked() {
                        settings.board_theme = board_theme;
                    }
                }
            });

            section_title(ui, "App");
            ui.horizontal(|ui| {
                for app_theme in AppTheme::ALL {
                    let text = RichText::new(app_theme.name()).size(16.0);
                    ui.selectable_value(&mut settings.app_theme, app_theme, text);
                }
            });

            section_title(ui, "Game");
            let text = RichText::new("Animate moves").size(16.0);
            ui.checkbox(&mut settings.animate_moves, text);
            ui.add_space(30.0);

            let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
            if widgets::wide_button(ui, true, "Back") || escape {
                back = true;
            }
        });
    });
    back
}

/// A heading above a group of settings.
fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).size(20.0).strong());
    ui.add_space(4.0);
}

/// Draws a small 4×4 board in the colors of a theme, with its name under it.
/// Returns the response, so the caller can see if it was clicked.
fn theme_preview(ui: &mut egui::Ui, board_theme: BoardTheme, selected: bool) -> egui::Response {
    let size = Vec2::new(PREVIEW_SIZE, PREVIEW_SIZE + NAME_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let board = Rect::from_min_size(rect.min, Vec2::splat(PREVIEW_SIZE));
    let painter = ui.painter();

    let (light, dark) = theme::board_colors(board_theme);
    let square = PREVIEW_SIZE / 4.0;
    for row in 0..4 {
        for col in 0..4 {
            let color = if (row + col) % 2 == 0 { light } else { dark };
            let min = board.min + Vec2::new(col as f32, row as f32) * square;
            painter.rect_filled(Rect::from_min_size(min, Vec2::splat(square)), 0.0, color);
        }
    }

    // The chosen theme gets an accent-colored frame; the others get one when hovered.
    let outline = if selected {
        Stroke::new(4.0, theme::ACCENT)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.fg_stroke
    } else {
        Stroke::NONE
    };
    painter.rect_stroke(board, 0.0, outline, StrokeKind::Outside);

    painter.text(
        rect.center_bottom(),
        Align2::CENTER_BOTTOM,
        board_theme.name(),
        FontId::proportional(16.0),
        ui.visuals().text_color(),
    );
    response
}
