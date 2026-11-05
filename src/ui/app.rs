use std::time::Duration;

use eframe::egui;

use crate::board::Board;
use crate::clock::{self, Clock, TimeControl};
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
    time_control: TimeControl,
    clock: Clock,
    game_over: Option<String>,
}

impl ChessApp {
    ///Creates the app.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, image::ImageError> {
        let time_control = TimeControl::FiveMinutes;
        Ok(Self {
            board: Board::starting_position(),
            textures: PieceTextures::load(&cc.egui_ctx)?,
            turn: Color::White,
            selected: None,
            moves: Vec::new(),
            flipped: false,
            time_control,
            clock: Clock::new(time_control),
            game_over: None,
        })
    }

    /// Resets the board for a new game.
    fn new_game(&mut self) {
        self.board = Board::starting_position();
        self.turn = Color::White;
        self.selected = None;
        self.moves.clear();
        self.clock = Clock::new(self.time_control);
        self.game_over = None;
    }

    /// Selects a piece, or moves the selected piece, depending on what was clicked.
    fn handle_click(&mut self, square: Square) {
        if self.game_over.is_some() {
            return;
        }

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
        self.clock.start(self.turn);
    }

    /// Ends game if a player ran out of time.
    fn check_time(&mut self) {
        if self.game_over.is_some() {
            return;
        }
        if let Some(loser) = self.clock.flagged() {
            self.clock.stop();
            self.game_over = Some(format!(
                "{:?} ran out of time. {:?} wins!",
                loser,
                loser.opposite()
            ));
        }
    }

    /// Draws the pannel in the right. Turn, buttons and move, list.
    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("chess-master");
        ui.separator();

        ui.label("Time control (applies on New Game):");
        ui.horizontal(|ui| {
            for time_control in TimeControl::ALL {
                ui.selectable_value(&mut self.time_control, time_control, time_control.label());
            }
        });
        ui.separator();

        // The clock of the player at the top of the board is shown first.
        let (top, bottom) = if self.flipped {
            (Color::White, Color::Black)
        } else {
            (Color::Black, Color::White)
        };
        self.clock_label(ui, top);
        self.clock_label(ui, bottom);

        match &self.game_over {
            Some(message) => {
                ui.colored_label(egui::Color32::RED, message);
            }
            None => {
                ui.label(format!("{:?} to move", self.turn));
            }
        }

        ui.horizontal(|ui| {
            if ui.button("New Game").clicked() {
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

    /// Shows one player's clock. The clock of the side move is highlighted.
    fn clock_label(&self, ui: &mut egui::Ui, color: Color) {
        let time = clock::format_time(self.clock.remaining(color));
        let mut text = egui::RichText::new(format!("{color:?}  {time}"))
            .size(28.0)
            .monospace();
        if color == self.turn && self.game_over.is_none() {
            text = text.strong().color(egui::Color32::LIGHT_GREEN);
        }
        ui.label(text);
    }
}

impl eframe::App for ChessApp {
    /// Called every frame to draw the window.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.check_time();
        if self.clock.is_running() {
            // Redraw regurarly so the clock counts down.
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

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
