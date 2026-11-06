mod board;
mod clock;
mod moves;
mod piece;
mod position;
mod rules;
mod square;
mod ui;

use eframe::egui;

use ui::app::ChessApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("chess-master")
            .with_inner_size([900.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "chess-master",
        options,
        Box::new(|cc| Ok(Box::new(ChessApp::new(cc)?))),
    )
}
