// In release builds on Windows, start without a console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analysis;
mod board;
mod bot;
mod clock;
mod engine;
mod game;
mod moves;
mod piece;
mod position;
mod rules;
mod san;
mod settings;
mod square;
mod ui;

use eframe::egui;

use ui::app::ChessApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("chess-master")
            .with_inner_size([1100.0, 820.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "chess-master",
        options,
        Box::new(|cc| Ok(Box::new(ChessApp::new(cc)?))),
    )
}
