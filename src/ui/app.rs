use eframe::egui;

use crate::board::Board;

/// The whole application sits here.
pub struct ChessApp {
    board: Board,
}

impl ChessApp {
    ///Creates the app.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            board: Board::starting_position(),
        }
    }
}

impl eframe::App for ChessApp {
    ///Called every frame to draw the window.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("chess-master");
            ui.label("Hello from egui!");
            ui.separator();
            ui.monospace(self.board.to_string());
            ui.separator();
            if ui.button("Click me").clicked() {
                println!("Clicked!");
            }
        });
    }
}
