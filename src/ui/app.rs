use eframe::egui;

use crate::board::Board;
use crate::piece::Color;
use crate::square::Square;
use crate::ui::board_view;
use crate::ui::piece_textures::PieceTextures;

/// The whole application sits here.
pub struct ChessApp {
    board: Board,
    textures: PieceTextures,
    turn: Color,
    selected: Option<Square>,
}

impl ChessApp {
    ///Creates the app.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, image::ImageError> {
        Ok(Self {
            board: Board::starting_position(),
            textures: PieceTextures::load(&cc.egui_ctx)?,
            turn: Color::White,
            selected: None,
        })
    }

    /// Selects a piece, or moves the selected piece, depending on what was clicked.
    fn handle_click(&mut self, square: Square) {
        let clicked_own_piece = self
            .board
            .get(square.rank, square.file)
            .is_some_and(|piece| piece.color == self.turn);

        match self.selected {
            None if clicked_own_piece => self.selected = Some(square),
            None => {}
            Some(from) if from == square => self.selected = None,
            Some(_) if clicked_own_piece => self.selected = Some(square),
            Some(from) => self.make_move(from, square),
        }
    }

    /// Moves a piece and passes the turn to the other player.
    fn make_move(&mut self, from: Square, to: Square) {
        self.board.move_piece(from, to);
        self.turn = self.turn.opposite();
        self.selected = None;
    }
}

impl eframe::App for ChessApp {
    ///Called every frame to draw the window
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(square) =
                board_view::draw_board(ui, &self.board, &self.textures, self.selected)
            {
                self.handle_click(square);
            }
        });
    }
}
