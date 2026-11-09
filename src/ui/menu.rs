use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextureId, Vec2,
};

use crate::bot::{MAX_RATING, MIN_RATING};
use crate::clock::TimeControl;
use crate::piece::{Color, Piece, PieceKind};
use crate::ui::piece_textures::PieceTextures;
use crate::ui::theme;
use crate::ui::widgets;

/// The width of the menu, centered in the window.
const MENU_WIDTH: f32 = 560.0;

/// Who you play against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Local,
    VsBot,
}

impl GameMode {
    /// Returns the title of the opponent card.
    pub fn label(self) -> &'static str {
        match self {
            GameMode::Local => "Play a friend",
            GameMode::VsBot => "Play the bot",
        }
    }

    /// Returns the small text under the label.
    pub fn description(self) -> &'static str {
        match self {
            GameMode::Local => "Same computer, take turns",
            GameMode::VsBot => "Stockfish, 400 – 3000",
        }
    }
}

/// The color you want to play against the bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    White,
    Random,
    Black,
}

impl ColorChoice {
    /// Every choice, in the order shown on the menu.
    pub const ALL: [ColorChoice; 3] = [ColorChoice::White, ColorChoice::Random, ColorChoice::Black];

    /// Returns the color to play this game. `Random` picks again every time.
    pub fn pick(self) -> Color {
        match self {
            ColorChoice::White => Color::White,
            ColorChoice::Black => Color::Black,
            ColorChoice::Random => random_color(),
        }
    }
}

/// Picks White or Black using the clock. The microseconds of the current time are
/// different every time you click, which is random enough for choosing a color.
fn random_color() -> Color {
    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.subsec_micros())
        .unwrap_or(0);
    if micros.is_multiple_of(2) {
        Color::White
    } else {
        Color::Black
    }
}

/// What the player clicked on the main menu this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    None,
    StartGame,
    OpenSettings,
}

/// Everything chosen on the main menu before a game starts.
#[derive(Debug, Clone, Copy)]
pub struct GameSettings {
    pub mode: GameMode,
    pub time_control: TimeControl,
    /// The color you want to play against the bot.
    pub color_choice: ColorChoice,
    /// How strong the bot plays (an approximate rating).
    pub bot_rating: u32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            mode: GameMode::Local,
            time_control: TimeControl::FiveMinutes,
            color_choice: ColorChoice::White,
            bot_rating: 1200,
        }
    }
}

impl GameSettings {
    /// A short description for the side panel, e.g. "Bot (1500) · 5 min".
    pub fn summary(&self) -> String {
        match self.mode {
            GameMode::Local => format!("Local game · {}", self.time_control.label()),
            GameMode::VsBot => format!("Bot ({}) · {}", self.bot_rating, self.time_control.label()),
        }
    }
}

/// Draws the main menu and returns what was clicked.
pub fn draw_menu(
    ui: &mut egui::Ui,
    settings: &mut GameSettings,
    textures: &PieceTextures,
) -> MenuAction {
    let mut action = MenuAction::None;
    let piece = |color, kind| textures.get(Piece::new(color, kind));

    egui::ScrollArea::vertical().show(ui, |ui| {
        widgets::centered_column(ui, MENU_WIDTH, |ui| {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("chess-master").size(44.0).strong());
                ui.weak("Pick an opponent, a color and a clock");
            });
            ui.add_space(20.0);

            // Two big cards, one per opponent.
            ui.columns(2, |columns| {
                for (column, mode) in columns.iter_mut().zip([GameMode::Local, GameMode::VsBot]) {
                    let icons = match mode {
                        GameMode::Local => vec![
                            piece(Color::White, PieceKind::Pawn),
                            piece(Color::Black, PieceKind::Pawn),
                        ],
                        GameMode::VsBot => vec![piece(Color::Black, PieceKind::King)],
                    };
                    let card = ChoiceCard {
                        icons: &icons,
                        icon_size: 64.0,
                        title: mode.label(),
                        subtitle: mode.description(),
                    };
                    let size = Vec2::new(column.available_width(), 140.0);
                    if card.show(column, size, settings.mode == mode).clicked() {
                        settings.mode = mode;
                    }
                }
            });
            ui.add_space(16.0);

            ui.columns(2, |columns| {
                let [left, right] = columns else {
                    return;
                };
                if settings.mode == GameMode::VsBot {
                    color_picker(left, settings, textures);
                    left.add_space(16.0);
                    rating_slider(right, settings);
                    right.add_space(20.0);
                }
                time_picker(left, settings);
                if widgets::accent_button(right, "Start game") {
                    action = MenuAction::StartGame;
                }
                if widgets::wide_button(right, true, "Settings") {
                    action = MenuAction::OpenSettings;
                }
            });

            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                // Quitting is rare, so it's a quiet link, not a big red button.
                if ui.link("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    });
    action
}

/// "You play": White, Random or Black, as three small cards.
fn color_picker(ui: &mut egui::Ui, settings: &mut GameSettings, textures: &PieceTextures) {
    let white_king = textures.get(Piece::new(Color::White, PieceKind::King));
    let black_king = textures.get(Piece::new(Color::Black, PieceKind::King));
    ui.strong("You play");
    ui.horizontal(|ui| {
        // Three cards of the same width fill the column.
        let width = (ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 3.0;
        for choice in ColorChoice::ALL {
            let icons = match choice {
                ColorChoice::White => vec![white_king],
                ColorChoice::Random => vec![white_king, black_king],
                ColorChoice::Black => vec![black_king],
            };
            let title = format!("{choice:?}");
            let card = ChoiceCard {
                icons: &icons,
                icon_size: 32.0,
                title: &title,
                subtitle: "",
            };
            let selected = settings.color_choice == choice;
            if card.show(ui, Vec2::new(width, 80.0), selected).clicked() {
                settings.color_choice = choice;
            }
        }
    });
}

/// "Bot rating": the rating and what it means, a slider, and its two ends.
fn rating_slider(ui: &mut egui::Ui, settings: &mut GameSettings) {
    ui.horizontal(|ui| {
        ui.strong("Bot rating");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let rating = settings.bot_rating;
            ui.strong(format!("{rating} · {}", rating_description(rating)));
        });
    });
    // The slider is as wide as the column.
    ui.spacing_mut().slider_width = ui.available_width();
    ui.add(
        egui::Slider::new(&mut settings.bot_rating, MIN_RATING..=MAX_RATING)
            .step_by(50.0)
            .show_value(false)
            .trailing_fill(true),
    );
    ui.horizontal(|ui| {
        ui.weak(MIN_RATING.to_string());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak(MAX_RATING.to_string());
        });
    });
}

/// "Time": one round chip per time control.
fn time_picker(ui: &mut egui::Ui, settings: &mut GameSettings) {
    ui.strong("Time");
    ui.horizontal(|ui| {
        for time_control in TimeControl::ALL {
            let text = RichText::new(time_control.label()).size(18.0);
            ui.selectable_value(&mut settings.time_control, time_control, text);
        }
    });
}

/// A big clickable card: piece pictures, a title and a small subtitle.
struct ChoiceCard<'a> {
    icons: &'a [TextureId],
    icon_size: f32,
    title: &'a str,
    subtitle: &'a str,
}

impl ChoiceCard<'_> {
    /// Draws the card. The selected card gets an accent-colored border.
    fn show(&self, ui: &mut egui::Ui, size: Vec2, selected: bool) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let visuals = ui.visuals();
        let painter = ui.painter();

        let fill = if response.hovered() {
            visuals.widgets.hovered.weak_bg_fill
        } else {
            theme::card_color(visuals)
        };
        painter.rect_filled(rect, 10, fill);
        if selected {
            painter.rect_stroke(
                rect,
                10,
                Stroke::new(3.0, theme::ACCENT),
                StrokeKind::Inside,
            );
        }

        // Pictures, title and subtitle are stacked, and the stack is centered in the card.
        let title_size = if self.subtitle.is_empty() { 15.0 } else { 22.0 };
        let subtitle_height = if self.subtitle.is_empty() { 0.0 } else { 22.0 };
        let stack_height = self.icon_size + 8.0 + title_size + subtitle_height;
        let mut y = rect.center().y - stack_height / 2.0;

        let whole_image = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let mut x = rect.center().x - self.icon_size * self.icons.len() as f32 / 2.0;
        for &icon in self.icons {
            let icon_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(self.icon_size));
            painter.image(icon, icon_rect, whole_image, Color32::WHITE);
            x += self.icon_size;
        }
        y += self.icon_size + 8.0;

        let text = |y: f32, text: &str, size: f32, color: Color32| {
            let pos = Pos2::new(rect.center().x, y);
            painter.text(
                pos,
                Align2::CENTER_TOP,
                text,
                FontId::proportional(size),
                color,
            );
        };
        text(y, self.title, title_size, visuals.strong_text_color());
        y += title_size + 6.0;
        text(y, self.subtitle, 14.0, visuals.weak_text_color());
        response
    }
}

/// A rough idea of what a rating means.
fn rating_description(rating: u32) -> &'static str {
    match rating {
        0..=799 => "Beginner",
        800..=1199 => "Casual player",
        1200..=1599 => "Club player",
        1600..=1999 => "Strong club player",
        2000..=2399 => "Expert / master",
        _ => "Grandmaster",
    }
}
