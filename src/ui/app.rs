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
    moves: Vec<(Square, Square)>,
    flipped: bool,
}

impl ChessApp {
    ///Creates the app.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, image::ImageError> {
        Ok(Self {
            board: Board::starting_position(),
            textures: PieceTextures::load(&cc.egui_ctx)?,
            turn: Color::White,
            selected: None,
            moves: Vec::new(),
            flipped: false,
        })
    }

    /// Resets the board for a new game.
    fn new_game(&mut self) {
        self.board = Board::starting_position();
        self.turn = Color::White;
        self.selected = None;
        self.moves.clear();
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
        self.moves.push((from, to));
        self.turn = self.turn.opposite();
        self.selected = None;
        println!("{from} -> {to}");
    }

    /// Draws the pannel in the right. Turn, buttons and move, list.
    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("chess-master");
        ui.separator();

        ui.label(format!("{:?} to move", self.turn));
        ui.horizontal(|ui| {
            if ui.button("New game").clicked() {
                self.new_game();
            }
            if ui.button("Flip board").clicked() {
                self.flipped = !self.flipped;
            }
        });

        ui.separator();
        ui.strong("Moves");
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (number, pair) in self.moves.chunks(2).enumerate() {
                let white = format!("{}-{}", pair[0].0, pair[0].1);
                let black = pair
                    .get(1)
                    .map(|(from, to)| format!("{from}-{to}"))
                    .unwrap_or_default();
                ui.monospace(format!("{:>3}. {white:<6} {black}", number + 1));
            }
        });
        ui.label(format!("Moves played: {}", self.moves.len()));
    }
}

impl eframe::App for ChessApp {
    /// Called every frame to draw the window.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::right("side_panel")
            .resizable(false)
            .exact_size(220.0)
            .show(ui, |ui| self.side_panel(ui));

        egui::CentralPanel::default().show(ui, |ui| {
            let clicked = board_view::draw_board(
                ui,
                &self.board,
                &self.textures,
                self.selected,
                self.flipped,
            );
            if let Some(square) = clicked {
                self.handle_click(square);
            }
        });
    }
}
