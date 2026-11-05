use std::time::Duration;

use eframe::egui;

use crate::board::Board;
use crate::clock::{self, Clock};
use crate::piece::Color;
use crate::square::Square;
use crate::ui::board_view;
use crate::ui::menu::{self, GameSettings};
use crate::ui::piece_textures::PieceTextures;

/// Which screen the app is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Game,
}

/// The whole application: everything the window needs to remember.
pub struct ChessApp {
    screen: Screen,
    /// The choices made on the main menu.
    settings: GameSettings,
    textures: PieceTextures,
    board: Board,
    /// Whose turn it is.
    turn: Color,
    /// The square of the piece the player picked up, if any.
    selected: Option<Square>,
    /// Every move played so far, as (from, to) pairs.
    moves: Vec<(Square, Square)>,
    /// When true, the board is drawn with Black at the bottom.
    flipped: bool,
    clock: Clock,
    /// Why the game ended, or `None` while it's still being played.
    game_over: Option<String>,
}

impl ChessApp {
    /// Creates the app. Called once by eframe when the window opens.
    ///
    /// Fails if one of the piece images can't be decoded.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, image::ImageError> {
        let settings = GameSettings::default();
        Ok(Self {
            screen: Screen::Menu,
            settings,
            textures: PieceTextures::load(&cc.egui_ctx)?,
            board: Board::starting_position(),
            turn: Color::White,
            selected: None,
            moves: Vec::new(),
            flipped: false,
            clock: Clock::new(settings.time_control),
            game_over: None,
        })
    }

    /// Starts a new game with the settings from the menu.
    fn new_game(&mut self) {
        self.board = Board::starting_position();
        self.turn = Color::White;
        self.selected = None;
        self.moves.clear();
        self.clock = Clock::new(self.settings.time_control);
        self.game_over = None;
        self.screen = Screen::Game;
    }

    /// Leaves the game and goes back to the main menu.
    fn back_to_menu(&mut self) {
        self.clock.stop();
        self.screen = Screen::Menu;
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

    /// Moves a piece, records it, and passes the turn (and the clock) to the other player.
    fn make_move(&mut self, from: Square, to: Square) {
        self.board.move_piece(from, to);
        self.moves.push((from, to));
        self.turn = self.turn.opposite();
        self.selected = None;
        self.clock.start(self.turn);
    }

    /// Ends the game if a player has run out of time.
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

    /// Draws the game screen: the side panel and the board.
    fn game_screen(&mut self, ui: &mut egui::Ui) {
        self.check_time();
        if self.clock.is_running() {
            // Redraw regularly so the clock visibly counts down.
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        egui::Panel::right("side_panel")
            .resizable(false)
            .exact_size(260.0)
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

    /// Draws the panel on the right: clocks, game status, buttons and the move list.
    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("chess-master");
        ui.label(format!(
            "{} · {}",
            self.settings.mode.label(),
            self.settings.time_control.label()
        ));
        ui.separator();

        // The clock of the player at the top of the board is shown first.
        let (top, bottom) = if self.flipped {
            (Color::White, Color::Black)
        } else {
            (Color::Black, Color::White)
        };
        self.clock_label(ui, top);
        self.clock_label(ui, bottom);
        ui.separator();

        match &self.game_over {
            Some(message) => {
                ui.colored_label(egui::Color32::RED, message);
            }
            None => {
                ui.label(format!("{:?} to move", self.turn));
            }
        }

        ui.horizontal(|ui| {
            if ui.button("New game").clicked() {
                self.new_game();
            }
            if ui.button("Flip board").clicked() {
                self.flipped = !self.flipped;
            }
            if ui.button("Main menu").clicked() {
                self.back_to_menu();
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
    }

    /// Shows one player's clock. The clock of the side to move is highlighted.
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
        match self.screen {
            Screen::Menu => {
                egui::CentralPanel::default().show(ui, |ui| {
                    if menu::draw_menu(ui, &mut self.settings) {
                        self.new_game();
                    }
                });
            }
            Screen::Game => self.game_screen(ui),
        }
    }
}
