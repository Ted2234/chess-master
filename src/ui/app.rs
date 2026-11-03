use eframe::egui;

use crate::board::Board;
use crate::ui::board_view;
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
    ///Called every frame to draw the window
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            board_view::draw_board(ui, &self.board);
        });
    }
}
