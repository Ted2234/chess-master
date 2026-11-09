use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, Vec2, load::SizedTexture};

use crate::bot;
use crate::clock::{self, Clock};
use crate::engine::{EngineHandle, EngineReply, EngineRequest, SearchLimit};
use crate::game::{Game, GameResult};
use crate::moves::Move;
use crate::piece::{Color, Piece, PieceKind};
use crate::rules;
use crate::san;
use crate::settings::Settings;
use crate::square::Square;
use crate::ui::board_view::{self, BoardInput, HINT_ARROW, Highlights, Slide};
use crate::ui::menu::{self, GameMode, GameSettings, MenuAction};
use crate::ui::piece_textures::PieceTextures;
use crate::ui::review::Review;
use crate::ui::settings_screen;
use crate::ui::theme;
use crate::ui::widgets;

/// How long a piece takes to slide to its new square.
const SLIDE_TIME: Duration = Duration::from_millis(160);

/// Which screen the app is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Settings,
    Game,
    Review,
}

/// A button on the game-over pop-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameOverAction {
    Review,
    NewGame,
    MainMenu,
    Close,
}

/// What we asked the engine to do, so we know what to do with its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineJob {
    /// Suggest a move to the player.
    Hint,
    /// Choose the bot's next move.
    BestMove,
}

/// The whole application: everything the window needs to remember.
pub struct ChessApp {
    screen: Screen,
    /// The choices made on the main menu.
    settings: GameSettings,
    /// The choices made on the Settings screen: how the app looks.
    app_settings: Settings,
    /// Why the settings could not be saved, shown on the Settings screen.
    settings_error: Option<String>,
    /// Your color in a game against the bot (picked when the game starts, for "Random").
    player_color: Color,
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
    /// Stockfish, started the first time it's needed.
    engine: Option<EngineHandle>,
    /// The last problem with the engine. Shown in the side panel.
    engine_error: Option<String>,
    /// The engine request we're waiting for (its id and what it's for), if any.
    waiting_for: Option<(u64, EngineJob)>,
    /// The id the next engine request will get.
    next_request_id: u64,
    /// The move Stockfish suggested for the current position.
    hint: Option<Move>,
    /// The analysis of a finished game, shown on the review screen.
    review: Option<Review>,
    /// The last move and when it was played, for the slide animation.
    animation: Option<(Move, Instant)>,
    /// True once the player has closed the game-over pop-up of this game.
    result_closed: bool,
}

impl ChessApp {
    /// Creates the app. Called once by eframe when the window opens.
    ///
    /// Fails if one of the piece images can't be decoded.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, image::ImageError> {
        let settings = GameSettings::default();
        let app_settings = Settings::load();
        theme::install_style(&cc.egui_ctx);
        theme::apply_app_theme(&cc.egui_ctx, app_settings.app_theme);
        Ok(Self {
            screen: Screen::Menu,
            settings,
            app_settings,
            settings_error: None,
            player_color: Color::White,
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
            review: None,
            animation: None,
            result_closed: false,
        })
    }

    /// Starts a new game with the settings from the menu.
    fn new_game(&mut self) {
        let vs_bot = self.settings.mode == GameMode::VsBot;
        if vs_bot && !self.ensure_engine() {
            self.screen = Screen::Menu; // can't play the bot without Stockfish.
            return;
        }

        self.player_color = self.settings.color_choice.pick();
        self.game = Game::new();
        self.selected = None;
        self.pending_promotion = None;
        self.clock = Clock::new(self.settings.time_control);
        self.hint = None;
        self.waiting_for = None;
        self.animation = None;
        self.result_closed = false;
        // Playing Black against the bot will flip the board so Black is at the bottom.
        self.flipped = vs_bot && self.player_color == Color::Black;

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
            && self.game.position().side_to_move != self.player_color
    }

    /// The name on a player card: "White" / "Black", or "You" / "Bot (1500)" against the bot.
    fn player_name(&self, color: Color) -> String {
        match self.settings.mode {
            GameMode::Local => format!("{color:?}"),
            GameMode::VsBot if color == self.player_color => "You".to_string(),
            GameMode::VsBot => format!("Bot ({})", self.settings.bot_rating),
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

    /// Reacts to a click or a drag on the board.
    fn handle_board_input(&mut self, input: BoardInput) {
        match input {
            BoardInput::Click(square) => self.handle_click(square),
            BoardInput::DragStart(square) => {
                // Pick the piece up, even if it was selected already (a click would put it down).
                self.selected = None;
                self.handle_click(square);
            }
            BoardInput::Drop(square) => {
                // Dropping it back on its own square keeps it selected, like a click.
                if self.selected.is_some_and(|from| from != square) {
                    self.handle_click(square);
                    self.animation = None; // the piece is already where it was dropped
                }
            }
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
            last_move: self
                .game
                .history()
                .last()
                .map(|played| (played.mv.from, played.mv.to)),
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
            arrows: self
                .hint
                .map(|mv| vec![(mv.from, mv.to, HINT_ARROW)])
                .unwrap_or_default(),
            slide: self.slide(),
            eval: None,
        }
    }

    /// The piece of the last move while it's still sliding into place, if any.
    fn slide(&self) -> Option<Slide> {
        let (mv, start) = self.animation?;
        let progress = start.elapsed().as_secs_f32() / SLIDE_TIME.as_secs_f32();
        (progress < 1.0).then_some(Slide {
            from: mv.from,
            to: mv.to,
            progress,
        })
    }

    /// Plays a move and passes the clock to the other player.
    fn play_move(&mut self, mv: Move) {
        self.selected = None;
        self.pending_promotion = None;
        if let Err(error) = self.game.play(mv) {
            eprintln!("could not play {mv}: {error}");
            return;
        }
        self.animation = self
            .app_settings
            .animate_moves
            .then_some((mv, Instant::now()));

        if self.game.result().is_some() {
            self.clock.stop();
        } else {
            self.clock.start(self.game.position().side_to_move);
        }
    }

    /// Takes back the last move (against the bot: the bot's reply and your move.)
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
            self.animation = None;
            if self.game.history().is_empty() {
                self.clock.stop(); // back at the start: nobody's time runs
            } else {
                self.clock.start(self.game.position().side_to_move);
            }
        }
    }

    /// Starts Stockfish if it isn't running yet. Returns false if that failed.
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

    /// Asks Stockfish for the best move in the current position.
    fn request_hint(&mut self) {
        if !self.ensure_engine() {
            return;
        }
        self.hint = None;
        self.search(EngineJob::Hint, SearchLimit::Millis(500));
    }

    /// If it's the bot's turn and it isn't thinking yet, asks it for a move.
    fn start_bot_move(&mut self) {
        if !self.is_bot_turn() || self.waiting_for.is_some() {
            return;
        }
        let mut limit = bot::strength(self.settings.bot_rating).limit;
        // Low on time? Then think faster.
        let bot_color = self.player_color.opposite();
        if self.clock.remaining(bot_color) < clock::LOW_TIME {
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

    /// Picks up any answers from the engine thread. Called every frame; never waits.
    fn poll_engine(&mut self) {
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
                // Analysis for the review screen?
                if let Some(review) = &mut self.review
                    && review.accepts(id)
                {
                    review.store(id, analysis);
                    return;
                }
                let Some((waiting_id, job)) = self.waiting_for else {
                    return;
                };
                if id != waiting_id {
                    return; // an old answer: the position has changed since.
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

    /// Sends every position of the finished game to Stockfish and opens the review screen.
    fn start_review(&mut self) {
        if !self.ensure_engine() {
            return;
        }
        let review = Review::new(
            self.game.history().to_vec(),
            self.game.position().clone(),
            self.next_request_id,
        );
        let fens = review.fens();
        self.next_request_id += fens.len() as u64;

        if let Some(engine) = &self.engine {
            // Analyse at full strength, even if the bot was playing weaker.
            engine.send(EngineRequest::SetOptions(bot::full_strength_options()));
            for (index, fen) in fens.into_iter().enumerate() {
                engine.send(EngineRequest::Search {
                    id: review.first_id() + index as u64,
                    fen,
                    limit: SearchLimit::Depth(12),
                });
            }
        }
        self.review = Some(review);
        self.screen = Screen::Review;
    }

    /// Draws the review screen: the analysis panel and the board at the chosen move.
    fn review_screen(&mut self, ui: &mut egui::Ui) {
        self.poll_engine();
        let player = match self.settings.mode {
            GameMode::Local => None,
            GameMode::VsBot => Some(self.player_color),
        };
        let Some(review) = &mut self.review else {
            self.screen = Screen::Menu;
            return;
        };
        if !review.is_finished() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        let mut back_to_menu = false;
        egui::Panel::right("review_panel")
            .resizable(false)
            .exact_size(360.0)
            .show(ui, |ui| back_to_menu = review.side_panel(ui, player));

        let position = review.position();
        let highlights = Highlights {
            last_move: review.last_move(),
            eval: Some(review.eval()),
            arrows: review.arrows(),
            check: if rules::is_in_check(position, position.side_to_move) {
                rules::king_square(position, position.side_to_move)
            } else {
                None
            },
            ..Highlights::default()
        };
        egui::CentralPanel::default().show(ui, |ui| {
            // The review board only shows positions, so clicks are ignored.
            board_view::draw_board(
                ui,
                &position.board,
                &self.textures,
                &highlights,
                self.flipped,
                self.app_settings.board_theme,
            );
        });

        if back_to_menu {
            if self
                .review
                .as_ref()
                .is_some_and(|review| !review.is_finished())
            {
                // Stockfish still has analysis jobs queued. Dropping the handle makes
                // its thread stop after the current one; a fresh engine starts when needed.
                self.engine = None;
            }
            self.review = None;
            self.screen = Screen::Menu;
        }
    }

    /// Gives up the game: against the bot that's you, otherwise the side to move.
    fn resign(&mut self) {
        let loser = match self.settings.mode {
            GameMode::Local => self.game.position().side_to_move,
            GameMode::VsBot => self.player_color,
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

    /// Draws the Settings screen. Every change is used and saved right away.
    fn settings_screen(&mut self, ui: &mut egui::Ui) {
        let before = self.app_settings;
        egui::CentralPanel::default().show(ui, |ui| {
            if settings_screen::draw_settings(ui, &mut self.app_settings) {
                self.settings_error = None;
                self.screen = Screen::Menu;
            }
            if let Some(error) = &self.settings_error {
                ui.vertical_centered(|ui| ui.colored_label(egui::Color32::RED, error));
            }
        });

        if self.app_settings != before {
            theme::apply_app_theme(ui.ctx(), self.app_settings.app_theme);
            self.settings_error = self
                .app_settings
                .save()
                .err()
                .map(|error| error.to_string());
        }
    }

    /// Draws the game screen: the side panel, the board and the promotion picker.
    fn game_screen(&mut self, ui: &mut egui::Ui) {
        self.check_time();
        self.poll_engine();
        self.start_bot_move();
        if self.slide().is_some() {
            ui.ctx().request_repaint(); // every frame, for a smooth slide
        } else if self.clock.is_running() || self.waiting_for.is_some() {
            // Redraw regularly so the clock visibly counts down.
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        egui::Panel::right("side_panel")
            .resizable(false)
            .exact_size(320.0)
            .show(ui, |ui| self.side_panel(ui));

        let highlights = self.highlights();
        egui::CentralPanel::default().show(ui, |ui| {
            let input = board_view::draw_board(
                ui,
                &self.game.position().board,
                &self.textures,
                &highlights,
                self.flipped,
                self.app_settings.board_theme,
            );
            if let Some(input) = input {
                self.handle_board_input(input);
            }
        });

        if self.pending_promotion.is_some() {
            self.promotion_picker(ui);
        }
        // Wait until the last move has finished sliding, so you see it before the pop-up.
        if self.game.result().is_some() && !self.result_closed && self.slide().is_none() {
            self.game_over_popup(ui);
        }
    }

    /// A pop-up when the game ends: who won (or a draw), how, and what to do next.
    fn game_over_popup(&mut self, ui: &mut egui::Ui) {
        let Some(result) = self.game.result() else {
            return;
        };
        let has_moves = !self.game.history().is_empty();
        let mut action = None;

        let modal = egui::Modal::new(egui::Id::unique("game_over")).show(ui.ctx(), |ui| {
            ui.set_width(300.0);
            ui.vertical_centered(|ui| {
                // The winner's king, or both kings for a draw.
                let kings: Vec<Color> = match result.winner() {
                    Some(winner) => vec![winner],
                    None => vec![Color::White, Color::Black],
                };
                let size = 64.0;
                ui.horizontal(|ui| {
                    let row_width = size * kings.len() as f32;
                    ui.add_space((ui.available_width() - row_width) / 2.0);
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for color in kings {
                        let king = self.textures.get(Piece::new(color, PieceKind::King));
                        ui.add(egui::Image::new(SizedTexture::new(king, Vec2::splat(size))));
                    }
                });

                let title = match result.winner() {
                    Some(winner) => format!("{winner:?} wins!"),
                    None => "Draw".to_string(),
                };
                ui.label(RichText::new(title).size(32.0).strong());
                // For a checkmate, also name the move that did it.
                let reason = match (result, self.game.history().last()) {
                    (GameResult::Checkmate { .. }, Some(last)) => {
                        format!("{} ({})", result.reason(), last.san)
                    }
                    _ => result.reason().to_string(),
                };
                ui.label(RichText::new(reason).size(16.0));
                ui.weak(result.score());
                ui.add_space(12.0);

                if has_moves && widgets::accent_button(ui, "Review game") {
                    action = Some(GameOverAction::Review);
                }
                ui.columns(2, |columns| {
                    if widgets::wide_button(&mut columns[0], true, "New game") {
                        action = Some(GameOverAction::NewGame);
                    }
                    if widgets::wide_button(&mut columns[1], true, "Main menu") {
                        action = Some(GameOverAction::MainMenu);
                    }
                });
                ui.add_space(4.0);
                if ui.link("Close").clicked() {
                    action = Some(GameOverAction::Close);
                }
            });
        });
        // A click outside the pop-up or `Esc` closes it too.
        if action.is_none() && modal.should_close() {
            action = Some(GameOverAction::Close);
        }

        let Some(action) = action else {
            return;
        };
        self.result_closed = true;
        match action {
            GameOverAction::Review => self.start_review(),
            GameOverAction::NewGame => self.new_game(),
            GameOverAction::MainMenu => self.back_to_menu(),
            GameOverAction::Close => {}
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

    /// Draws the panel on the right: a card for each player, the moves, and the buttons.
    fn side_panel(&mut self, ui: &mut egui::Ui) {
        // The player at the top of the board gets the top card.
        let (top, bottom) = if self.flipped {
            (Color::White, Color::Black)
        } else {
            (Color::Black, Color::White)
        };

        egui::Panel::top("top_player")
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_space(8.0);
                self.player_card(ui, top);
                ui.add_space(4.0);
            });
        egui::Panel::bottom("controls")
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_space(4.0);
                self.status(ui);
                self.player_card(ui, bottom);
                ui.add_space(4.0);
                self.buttons(ui);
                ui.collapsing("FEN", |ui| self.fen(ui));
                ui.add_space(4.0);
            });
        // The moves get all the space that is left in the middle.
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.move_list(ui));
    }

    /// A card for one player: their king, name, captured pieces and clock.
    fn player_card(&self, ui: &mut egui::Ui, color: Color) {
        widgets::card(ui, |ui| {
            ui.horizontal(|ui| {
                let king = self.textures.get(Piece::new(color, PieceKind::King));
                ui.add(egui::Image::new(SizedTexture::new(king, Vec2::splat(40.0))));
                ui.vertical(|ui| {
                    ui.label(RichText::new(self.player_name(color)).size(18.0).strong());
                    self.captured_pieces(ui, color);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.clock_display(ui, color);
                });
            });
        });
    }

    /// The pieces `color` has captured, small, and how many points ahead `color` is.
    fn captured_pieces(&self, ui: &mut egui::Ui, color: Color) {
        ui.horizontal(|ui| {
            ui.set_min_height(18.0); // keeps the card the same height with nothing captured
            ui.spacing_mut().item_spacing.x = 0.0;
            for piece in self.game.captured_by(color) {
                let image = SizedTexture::new(self.textures.get(piece), Vec2::splat(18.0));
                ui.add(egui::Image::new(image));
            }

            let board = &self.game.position().board;
            let ours = board.material(color);
            let theirs = board.material(color.opposite());
            if let Some(lead) = ours.checked_sub(theirs).filter(|&lead| lead > 0) {
                ui.add_space(4.0);
                ui.weak(format!("+{lead}"));
            }
        });
    }

    /// One player's clock: in the accent color while it's their turn, red when time is low.
    fn clock_display(&self, ui: &mut egui::Ui, color: Color) {
        let remaining = self.clock.remaining(color);
        let their_turn = color == self.game.position().side_to_move && self.game.result().is_none();
        let (fill, text_color) = if remaining < clock::LOW_TIME {
            (theme::DANGER, Color32::WHITE)
        } else if their_turn {
            (theme::ACCENT, theme::ON_ACCENT)
        } else {
            (
                ui.visuals().extreme_bg_color,
                ui.visuals().weak_text_color(),
            )
        };

        egui::Frame::new()
            .fill(fill)
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(10, 4))
            .show(ui, |ui| {
                let time = RichText::new(clock::format_time(remaining))
                    .monospace()
                    .size(26.0)
                    .strong()
                    .color(text_color);
                ui.label(time);
            });
    }

    /// One line about the game (whose turn, check, the result), then engine messages.
    fn status(&self, ui: &mut egui::Ui) {
        match self.game.result() {
            Some(result) => {
                ui.label(RichText::new(result.to_string()).size(16.0).strong());
            }
            None => {
                let position = self.game.position();
                let side = position.side_to_move;
                if rules::is_in_check(position, side) {
                    ui.colored_label(Color32::ORANGE, format!("{side:?} is in check!"));
                } else {
                    ui.label(format!("{side:?} to move"));
                }
            }
        }

        if self.is_bot_turn() {
            ui.weak("The bot is thinking...");
        } else if self.waiting_for.is_some() {
            ui.weak("Stockfish is thinking...");
        }
        if let Some(hint) = self.hint {
            let text = san::san(self.game.position(), hint);
            ui.label(format!("Stockfish suggests {text}"));
        }
        if let Some(error) = &self.engine_error {
            ui.colored_label(theme::DANGER, error);
        }
        ui.add_space(4.0);
    }

    /// The buttons under your card. All buttons in a row have the same width.
    fn buttons(&mut self, ui: &mut egui::Ui) {
        let playing = self.game.result().is_none();
        let has_moves = !self.game.history().is_empty();
        let can_undo = playing && has_moves && !self.is_bot_turn();
        let can_hint = playing && self.waiting_for.is_none();
        // Hints are for local games; against the bot they would be cheating.
        let local = self.settings.mode == GameMode::Local;

        ui.columns(if local { 4 } else { 3 }, |columns| {
            if widgets::wide_button(&mut columns[0], can_undo, "Undo") {
                self.undo();
            }
            if widgets::wide_button(&mut columns[1], playing, "Resign") {
                self.resign();
            }
            if widgets::wide_button(&mut columns[2], true, "Flip") {
                self.flipped = !self.flipped;
            }
            if local && widgets::wide_button(&mut columns[3], can_hint, "Hint") {
                self.request_hint();
            }
        });
        if !playing && has_moves && widgets::accent_button(ui, "Review game") {
            self.start_review();
        }
        ui.columns(2, |columns| {
            if widgets::wide_button(&mut columns[0], true, "New game") {
                self.new_game();
            }
            if widgets::wide_button(&mut columns[1], true, "Main menu") {
                self.back_to_menu();
            }
        });
    }

    /// The position as FEN text, with a button to copy it.
    fn fen(&self, ui: &mut egui::Ui) {
        let fen = self.game.position().to_fen();
        if ui.small_button("Copy").clicked() {
            ui.ctx().copy_text(fen.clone());
        }
        ui.add(egui::Label::new(RichText::new(fen).monospace().small()).wrap());
    }

    /// Every move so far as a striped table. The last move is in the accent color.
    fn move_list(&self, ui: &mut egui::Ui) {
        widgets::card(ui, |ui| {
            ui.set_min_size(ui.available_size()); // fill the space between the two cards
            ui.horizontal(|ui| {
                ui.strong("Moves");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(self.settings.summary());
                });
            });

            let history = self.game.history();
            let last = history.len().checked_sub(1);
            // Three equal columns: number, White's move, Black's move.
            let column_width = (ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 3.0;
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink(false)
                .show(ui, |ui| {
                    egui::Grid::new("moves")
                        .striped(true)
                        .num_columns(3)
                        .min_col_width(column_width)
                        .show(ui, |ui| {
                            for (number, pair) in history.chunks(2).enumerate() {
                                ui.weak(format!("{}.", number + 1));
                                for (offset, played) in pair.iter().enumerate() {
                                    let color = if Some(number * 2 + offset) == last {
                                        theme::ACCENT
                                    } else {
                                        ui.visuals().strong_text_color()
                                    };
                                    let text = RichText::new(&played.san)
                                        .monospace()
                                        .size(15.0)
                                        .color(color);
                                    ui.label(text);
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    }
}

impl eframe::App for ChessApp {
    /// Called every frame to draw the window.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        match self.screen {
            Screen::Menu => {
                egui::CentralPanel::default().show(ui, |ui| {
                    match menu::draw_menu(ui, &mut self.settings, &self.textures) {
                        MenuAction::StartGame => {
                            self.engine_error = None;
                            self.new_game();
                        }
                        MenuAction::OpenSettings => self.screen = Screen::Settings,
                        MenuAction::None => {}
                    }
                    if let Some(error) = &self.engine_error {
                        ui.vertical_centered(|ui| {
                            ui.colored_label(egui::Color32::RED, error);
                        });
                    }
                });
            }
            Screen::Settings => self.settings_screen(ui),
            Screen::Game => self.game_screen(ui),
            Screen::Review => self.review_screen(ui),
        }
    }
}
