use eframe::egui;

use crate::board::Board;
use crate::ui::board_view;
use crate::ui::piece_textures::PieceTextures;

/// The whole application sits here.
pub struct ChessApp {
    board: Board,
    textures: PieceTextures,
}

impl ChessApp {
    ///Creates the app.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, image::ImageError> {
        Ok(Self {
            board: Board::starting_position(),
            textures: PieceTextures::load(&cc.egui_ctx)?,
        })
    }
}

impl eframe::App for ChessApp {
    ///Called every frame to draw the window
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            board_view::draw_board(ui, &self.board, &self.textures);
        });
    }
}
