use eframe::egui::{self, Color32, RichText};

use crate::bot::{MAX_RATING, MIN_RATING};
use crate::clock::TimeControl;
use crate::piece::Color;

/// Who you play againts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Local,
    VsBot,
}

impl GameMode {
    pub fn label(self) -> &'static str {
        match self {
            GameMode::Local => "Play a frient (same computer)",
            GameMode::VsBot => "Play the bot",
        }
    }
}

/// Everything chosen on the main menu before a game starts.
#[derive(Debug, Clone, Copy)]
pub struct GameSettings {
    pub mode: GameMode,
    pub time_control: TimeControl,
    /// The color you play against the bot.
    pub player_color: Color,
    /// How strong the bot plays (an approximate rating).
    pub bot_rating: u32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            mode: GameMode::Local,
            time_control: TimeControl::FiveMinutes,
            player_color: Color::White,
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

/// Draws the main menu. Retirns true when pressing "Start Game".
pub fn draw_menu(ui: &mut egui::Ui, settings: &mut GameSettings) -> bool {
    let mut start = false;

    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new("chess-master").size(40.0).strong());
        ui.add_space(30.0);

        ui.label(RichText::new("Opponent").size(20.0));
        for mode in [GameMode::Local, GameMode::VsBot] {
            ui.selectable_value(&mut settings.mode, mode, mode.label());
        }
        ui.add_space(20.0);

        if settings.mode == GameMode::VsBot {
            ui.label(RichText::new("You play").size(20.0));
            for color in Color::ALL {
                ui.selectable_value(&mut settings.player_color, color, format!("{color:?}"));
            }
            ui.add_space(10.0);

            ui.label(RichText::new("Bot rating").size(20.0));
            let slider_size = [ui.spacing().slider_width, ui.spacing().interact_size.y];
            ui.add_sized(
                slider_size,
                egui::Slider::new(&mut settings.bot_rating, MIN_RATING..=MAX_RATING)
                    .step_by(50.0)
                    .show_value(false),
            );
            ui.add(
                egui::DragValue::new(&mut settings.bot_rating)
                    .range(MIN_RATING..=MAX_RATING)
                    .speed(18.0)
                    .update_while_editing(false),
            );
            ui.label(rating_description(settings.bot_rating));
            ui.add_space(20.0);
        }

        ui.label(RichText::new("Time per player").size(20.0));
        for time_control in TimeControl::ALL {
            ui.selectable_value(
                &mut settings.time_control,
                time_control,
                time_control.label(),
            );
        }
        ui.add_space(30.0);

        if ui.button(RichText::new("Start Game").size(24.0)).clicked() {
            start = true;
        }

        if ui
            .button(RichText::new("Quit").size(24.0).color(Color32::RED))
            .clicked()
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
    start
}

/// A rough idea of what a rating means.
fn rating_description(rating: u32) -> &'static str {
    match rating {
        0..=799 => "Beginner: just learned the rules",
        800..=1199 => "Casual player",
        1200..=1599 => "Club player",
        1600..=1999 => "Strong club player",
        2000..=2399 => "Expect / master",
        _ => "Grandmaster level and above",
    }
}
