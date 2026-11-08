use std::time::Duration;

use eframe::egui;

use crate::bot;
use crate::clock::{self, Clock};
use crate::engine::{EngineHandle, EngineReply, EngineRequest, SearchLimit};
use crate::game::{Game, GameResult};
use crate::moves::Move;
use crate::piece::{Color, Piece, PieceKind};
use crate::rules;
use crate::san;
use crate::square::Square;
use crate::ui::board_view::{self, Highlights};
use crate::ui::menu::{self, GameMode, GameSettings};
use crate::ui::piece_textures::PieceTextures;

/// Which screen the app is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Game,
}

/// What we asked the engine to do, so we know what to do with its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineJob {
    /// Suggest a move to the player.
    Hint,
    /// Chose the bot's next move.
    BestMove,
}

/// The whole application: everything the window needs to remember.
pub struct ChessApp {
    screen: Screen,
    /// The choices made on the main menu.
    settings: GameSettings,
    textures: PieceTextures,
    /// The position, the move history and the result.
    game: Game,
    /// The square of the piece the player picked up, if any.
    selected: Option<Square>,
    /// A pawn move waiting for the player to pick a promotion piece.
    pending_promotion: Option<(Square, Square)>,
    /// When true, the board is drawn with Black at the bottom.
    flipped: bool,
    clock: Clock,
    /// Stockfish, started the first time it's unimplemented!
    engine: Option<EngineHandle>,
    /// The last problem with the engine. Shown in the side panel.
    engine_error: Option<String>,
    /// The engine request we're waiting for (its id and what it's for), if any.
    waiting_for: Option<(u64, EngineJob)>,
    /// The id the next engine request will get.
    next_request_id: u64,
    /// The move Stockfish suggested for the current position.
    hint: Option<Move>,
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
            game: Game::new(),
            selected: None,
            pending_promotion: None,
            flipped: false,
            clock: Clock::new(settings.time_control),
            engine: None,
            engine_error: None,
            waiting_for: None,
            next_request_id: 0,
            hint: None,
        })
    }

    /// Starts a new game with the settings from the menu.
    fn new_game(&mut self) {
        let vs_bot = self.settings.mode == GameMode::VsBot;
        if vs_bot && !self.ensure_engine() {
            self.screen = Screen::Menu; // can't play the bot without Stockfish.
            return;
        }

        self.game = Game::new();
        self.selected = None;
        self.pending_promotion = None;
        self.clock = Clock::new(self.settings.time_control);
        self.hint = None;
        self.waiting_for = None;
        // Playing Black against the bot will flip the board so Black is at the bottom.
        self.flipped = vs_bot && self.settings.player_color == Color::Black;

        if let Some(engine) = &self.engine {
            let options = if vs_bot {
                bot::strength(self.settings.bot_rating).options
            } else {
                bot::full_strength_options()
            };
            engine.send(EngineRequest::NewGame);
            engine.send(EngineRequest::SetOptions(options));
        }
        self.screen = Screen::Game;
    }

    /// True when the bot should move now.
    fn is_bot_turn(&self) -> bool {
        self.settings.mode == GameMode::VsBot
            && self.game.result().is_none()
            && self.game.position().side_to_move != self.settings.player_color
    }

    /// The name shown next to a clock: "White", or "You (White)" / "Bot (Black)"
    fn player_name(&self, color: Color) -> String {
        match self.settings.mode {
            GameMode::Local => format!("{color:?}"),
            GameMode::VsBot if color == self.settings.player_color => "You".to_string(),
            GameMode::VsBot => "Bot".to_string(),
        }
    }

    /// Leaves the game and goes back to the main menu.
    fn back_to_menu(&mut self) {
        self.clock.stop();
        self.screen = Screen::Menu;
    }

    /// Selects a piece, or moves the selected piece, depending on what was clicked.
    fn handle_click(&mut self, square: Square) {
        if self.game.result().is_some() || self.pending_promotion.is_some() || self.is_bot_turn() {
            return;
        }

        let position = self.game.position();
        let clicked_own_piece = position
            .board
            .get(square)
            .is_some_and(|piece| piece.color == position.side_to_move);

        match self.selected {
            Some(from) if from == square => self.selected = None,
            _ if clicked_own_piece => self.selected = Some(square),
            Some(from) => {
                // Only squares the selected piece can legally move to are accepted.
                let options: Vec<Move> = self
                    .moves_from(from)
                    .into_iter()
                    .filter(|mv| mv.to == square)
                    .collect();
                match options.as_slice() {
                    [] => self.selected = None,
                    [mv] => self.play_move(*mv),
                    // Several moves to the same square means a promotion: ask which piece.
                    _ => self.pending_promotion = Some((from, square)),
                }
            }
            None => {}
        }
    }

    /// The legal moves of the piece on `from`.
    fn moves_from(&self, from: Square) -> Vec<Move> {
        self.game
            .legal_moves()
            .into_iter()
            .filter(|mv| mv.from == from)
            .collect()
    }

    /// What to draw on top of the board this frame.
    fn highlights(&self) -> Highlights {
        let position = self.game.position();
        let side = position.side_to_move;
        Highlights {
            selected: self.selected,
            targets: match self.selected {
                Some(from) => self.moves_from(from).iter().map(|mv| mv.to).collect(),
                None => Vec::new(),
            },
            check: if rules::is_in_check(position, side) {
                rules::king_square(position, side)
            } else {
                None
            },
            arrow: self.hint.map(|mv| (mv.from, mv.to)),
        }
    }

    /// Plays a move and passes the clock to the other player.
    fn play_move(&mut self, mv: Move) {
        self.selected = None;
        self.pending_promotion = None;
        if let Err(error) = self.game.play(mv) {
            eprintln!("could not play {mv}: {error}");
            return;
        }

        if self.game.result().is_some() {
            self.clock.stop();
        } else {
            self.clock.start(self.game.position().side_to_move);
        }
    }

    /// Takes back the last move (againts the bot: the bot's reply and your move.)
    /// The clock keeps the time already used.
    fn undo(&mut self) {
        let moves_to_undo = match self.settings.mode {
            GameMode::Local => 1,
            GameMode::VsBot => 2,
        };
        let mut undone = false;
        for _ in 0..moves_to_undo {
            undone |= self.game.undo().is_some();
        }

        if undone {
            self.selected = None;
            self.hint = None;
            self.waiting_for = None;
            if self.game.history().is_empty() {
                self.clock.stop(); // back at the start: nobody's time runs
            } else {
                self.clock.start(self.game.position().side_to_move);
            }
        }
    }

    /// Starts Stockfish if it isn't running yet. Retuens false if that failed.
    fn ensure_engine(&mut self) -> bool {
        if self.engine.is_none() {
            match EngineHandle::spawn() {
                Ok(engine) => self.engine = Some(engine),
                Err(error) => {
                    self.engine_error = Some(error.to_string());
                    return false;
                }
            }
        }
        true
    }

    /// Asks Stockfish for the best move in he current position.
    fn request_hint(&mut self) {
        if !self.ensure_engine() {
            return;
        }
        self.hint = None;
        self.search(EngineJob::Hint, SearchLimit::Millis(500));
    }

    /// If it's the bot's turn it isn't thinking yet, asks it for a move.
    fn start_bot_move(&mut self) {
        if !self.is_bot_turn() || self.waiting_for.is_some() {
            return;
        }
        let mut limit = bot::strength(self.settings.bot_rating).limit;
        // Low on time, then tink faster.
        let bot_color = self.settings.player_color.opposite();
        if self.clock.remaining(bot_color) < Duration::from_secs(20) {
            limit = SearchLimit::Millis(100);
        }
        self.search(EngineJob::BestMove, limit);
    }

    /// Sends a search for the current position to the engine thread.
    fn search(&mut self, job: EngineJob, limit: SearchLimit) {
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.waiting_for = Some((id, job));

        if let Some(engine) = &self.engine {
            engine.send(EngineRequest::Search {
                id,
                fen: self.game.position().to_fen(),
                limit,
            });
        }
    }

    /// Picks up any answers from the engine thread. called every frame; never waits.
    fn pool_engine(&mut self) {
        // Collect the replies first: we can't call `&mut self` methods while `self.engine` is
        // borrowed.
        let replies: Vec<EngineReply> = match &self.engine {
            Some(engine) => std::iter::from_fn(|| engine.try_receive()).collect(),
            None => return,
        };
        for reply in replies {
            self.handle_reply(reply);
        }
    }

    /// Uses one answer from the engine.
    fn handle_reply(&mut self, reply: EngineReply) {
        match reply {
            EngineReply::Searched { id, analysis } => {
                let Some((waiting_id, job)) = self.waiting_for else {
                    return;
                };
                if id != waiting_id {
                    return; // an lod answer: The position has changed since.
                }
                self.waiting_for = None;
                let best_move = Move::from_uci(&analysis.best_move);
                match job {
                    EngineJob::Hint => self.hint = best_move,
                    EngineJob::BestMove => {
                        if let Some(mv) = best_move {
                            self.play_move(mv);
                        }
                    }
                }
            }
            EngineReply::Failed(error) => {
                self.waiting_for = None;
                self.engine_error = Some(error);
            }
        }
    }

    /// Gives up the game: against the bot that's you, otherwise the side to move.
    fn resign(&mut self) {
        let loser = match self.settings.mode {
            GameMode::Local => self.game.position().side_to_move,
            GameMode::VsBot => self.settings.player_color,
        };
        self.game.end(GameResult::Resignation {
            winner: loser.opposite(),
        });
        self.clock.stop();
    }

    /// Ends the game if a player has run out of time.
    fn check_time(&mut self) {
        if self.game.result().is_some() {
            return;
        }
        if let Some(loser) = self.clock.flagged() {
            self.clock.stop();
            self.game.end(GameResult::Timeout {
                winner: loser.opposite(),
            });
        }
    }

    /// Draws the game screen: the side panel, the board and the promotion picker.
    fn game_screen(&mut self, ui: &mut egui::Ui) {
        self.check_time();
        self.pool_engine();
        self.start_bot_move();
        if self.clock.is_running() || self.waiting_for.is_some() {
            // Redraw regularly so the clock visibly counts down.
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        egui::Panel::right("side_panel")
            .resizable(false)
            .exact_size(260.0)
            .show(ui, |ui| self.side_panel(ui));

        let highlights = self.highlights();
        egui::CentralPanel::default().show(ui, |ui| {
            let clicked = board_view::draw_board(
                ui,
                &self.game.position().board,
                &self.textures,
                &highlights,
                self.flipped,
            );
            if let Some(square) = clicked {
                self.handle_click(square);
            }
        });

        if self.pending_promotion.is_some() {
            self.promotion_picker(ui);
        }
    }

    /// A pop-up window with the four pieces a pawn can become.
    fn promotion_picker(&mut self, ui: &mut egui::Ui) {
        let Some((from, to)) = self.pending_promotion else {
            return;
        };
        let color = self.game.position().side_to_move;
        let mut chosen = None;

        egui::Modal::new(egui::Id::unique("promotion")).show(ui.ctx(), |ui| {
            ui.label("Promote to:");
            ui.horizontal(|ui| {
                for kind in [
                    PieceKind::Queen,
                    PieceKind::Rook,
                    PieceKind::Bishop,
                    PieceKind::Knight,
                ] {
                    let texture = self.textures.get(Piece::new(color, kind));
                    let image = egui::load::SizedTexture::new(texture, egui::vec2(64.0, 64.0));
                    if ui.add(egui::Button::image(image)).clicked() {
                        chosen = Some(kind);
                    }
                }
            });
            if ui.button("Cancel").clicked() {
                self.pending_promotion = None;
                self.selected = None;
            }
        });

        if let Some(kind) = chosen {
            self.play_move(Move {
                from,
                to,
                promotion: Some(kind),
            });
        }
    }

    /// Draws the panel on the right: clocks, game status, buttons and the move list.
    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("chess-master");
        ui.label(self.settings.summary());
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

        match self.game.result() {
            Some(result) => {
                ui.colored_label(egui::Color32::RED, result.to_string());
            }
            None => {
                let position = self.game.position();
                let side = position.side_to_move;
                if rules::is_in_check(position, side) {
                    ui.colored_label(egui::Color32::ORANGE, format!("{side:?} is in check!"));
                } else {
                    ui.label(format!("{side:?} to move"));
                }
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
        let playing = self.game.result().is_none();
        ui.horizontal(|ui| {
            let can_undo = playing && !self.game.history().is_empty() && !self.is_bot_turn();
            if ui
                .add_enabled(can_undo, egui::Button::new("Undo move"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(playing, egui::Button::new("Resign"))
                .clicked()
            {
                self.resign();
            }
        });

        ui.separator();
        let thinking = self.waiting_for.is_some();
        match self.settings.mode {
            GameMode::Local => {
                let hint_button = egui::Button::new("Hint (ask Stockfish)");
                if ui.add_enabled(playing && !thinking, hint_button).clicked() {
                    self.request_hint();
                }
                if thinking {
                    ui.label("Stockfish is thinking...");
                }
                if let Some(hint) = self.hint {
                    let text = san::san(self.game.position(), hint);
                    ui.label(format!("Stockfish suggests {text}"));
                }
            }
            GameMode::VsBot => {
                if self.is_bot_turn() {
                    ui.label("The bot is thinking...");
                }
            }
        }
        if let Some(error) = &self.engine_error {
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.separator();

        let fen = self.game.position().to_fen();
        ui.horizontal(|ui| {
            ui.strong("FEN");
            if ui.small_button("Copy").clicked() {
                ui.ctx().copy_text(fen.clone());
            }
        });
        ui.add(egui::Label::new(egui::RichText::new(fen).monospace().small()).wrap());

        ui.separator();
        ui.strong("Moves");
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (number, pair) in self.game.history().chunks(2).enumerate() {
                let white = &pair[0].san;
                let black = pair.get(1).map(|played| played.san.as_str()).unwrap_or("");
                ui.monospace(format!("{:>3}. {white:<8} {black}", number + 1));
            }
        });
    }

    /// Shows one player's clock. The clock of the side to move is highlighted.
    fn clock_label(&self, ui: &mut egui::Ui, color: Color) {
        let time = clock::format_time(self.clock.remaining(color));
        let name = self.player_name(color);
        let mut text = egui::RichText::new(format!("{name:<6}{time}"))
            .size(28.0)
            .monospace();
        if color == self.game.position().side_to_move && self.game.result().is_none() {
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
                        self.engine_error = None;
                        self.new_game();
                    }
                    if let Some(error) = &self.engine_error {
                        ui.vertical_centered(|ui| {
                            ui.colored_label(egui::Color32::RED, error);
                        });
                    }
                });
            }
            Screen::Game => self.game_screen(ui),
        }
    }
}
