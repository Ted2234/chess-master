//! chess-master as a library. The desktop program (`main.rs`) and the Android app both
//! start it with [`run`]; only the window setup around it differs.

mod analysis;
#[cfg(target_os = "android")]
mod android;
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

use ui::app::ChessApp;

/// Opens the app with these window options and runs it until it's closed.
pub fn run(options: eframe::NativeOptions) -> eframe::Result {
    eframe::run_native(
        "chess-master",
        options,
        Box::new(|cc| Ok(Box::new(ChessApp::new(cc)?))),
    )
}
