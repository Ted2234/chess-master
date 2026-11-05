use eframe::egui::{self, Color32, RichText};

use crate::clock::TimeControl;

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
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            mode: GameMode::Local,
            time_control: TimeControl::FiveMinutes,
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
        ui.selectable_value(&mut settings.mode, GameMode::Local, GameMode::Local.label());
        ui.add_enabled(false, egui::Button::new(GameMode::VsBot.label()))
            .on_disabled_hover_text("Coming in lab 14");
        ui.add_space(20.0);

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
