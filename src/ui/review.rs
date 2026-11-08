use std::iter;

use eframe::egui::{self, Color32, RichText};

use crate::analysis::{self, MoveQuality, MoveReview};
use crate::engine::Analysis;
use crate::game::PlayedMove;
use crate::piece::Color;
use crate::position::Position;
use crate::square::Square;
use crate::ui::board_view::{BEST_MOVE_ARROW, PLAYED_MOVE_ARROW};

/// The review screen: a finished game, the engine's analysis of every position,
/// and which move the player is looking at.
pub struct Review {
    history: Vec<PlayedMove>,
    final_position: Position,
    /// Engine request ids are `first_id`, `first_id + 1`, ... one per position.
    first_id: u64,
    /// One slot per position; filled in as the engine answers.
    analyses: Vec<Option<Analysis>>,
    /// One review per move, computed when all analyses have arrived.
    reviews: Vec<MoveReview>,
    /// The index of the move being looked at.
    current: usize,
}

impl Review {
    /// Starts a review of a finished game.
    pub fn new(history: Vec<PlayedMove>, final_position: Position, first_id: u64) -> Self {
        let positions = history.len() + 1;
        Review {
            history,
            final_position,
            first_id,
            analyses: vec![None; positions],
            reviews: Vec::new(),
            current: 0,
        }
    }

    /// Every position to analyse, as FEN, in order: before each move, then the final one.
    pub fn fens(&self) -> Vec<String> {
        self.history
            .iter()
            .map(|played| played.before.to_fen())
            .chain(iter::once(self.final_position.to_fen()))
            .collect()
    }

    /// The id of the request for the first position.
    pub fn first_id(&self) -> u64 {
        self.first_id
    }

    /// True if an engine answer with this id belongs to this review.
    pub fn accepts(&self, id: u64) -> bool {
        id.checked_sub(self.first_id)
            .is_some_and(|index| (index as usize) < self.analyses.len())
    }

    /// Stores an engine answer. When the last one arrives, every move gets reviewed.
    pub fn store(&mut self, id: u64, analysis: Analysis) {
        let index = (id - self.first_id) as usize;
        self.analyses[index] = Some(analysis);

        if self.is_finished() {
            let analyses: Vec<Analysis> = self.analyses.iter().flatten().cloned().collect();
            self.reviews = analysis::review_game(&self.history, &analyses);
        }
    }

    /// True once the engine has answered for every position.
    pub fn is_finished(&self) -> bool {
        self.analyses.iter().all(Option::is_some)
    }

    /// The position shown on the board: just before the move being looked at.
    pub fn position(&self) -> &Position {
        match self.history.get(self.current) {
            Some(played) => &played.before,
            None => &self.final_position,
        }
    }

    /// Arrows for the board: the move played, and the engine's move if it was better.
    pub fn arrows(&self) -> Vec<(Square, Square, Color32)> {
        let mut arrows = Vec::new();
        if let Some(played) = self.history.get(self.current) {
            arrows.push((played.mv.from, played.mv.to, PLAYED_MOVE_ARROW));
        }
        if let Some(review) = self.reviews.get(self.current)
            && review.quality >= MoveQuality::Inaccuracy
            && let Some(best) = review.best_move
        {
            arrows.push((best.from, best.to, BEST_MOVE_ARROW));
        }
        arrows
    }

    /// Draws the side panel. `player` is your color against the bot, `None` in a
    /// local game. Returns true when "Main menu" is clicked.
    pub fn side_panel(&mut self, ui: &mut egui::Ui, player: Option<Color>) -> bool {
        ui.heading("Game review");
        let back = ui.button("Main menu").clicked();
        ui.separator();

        if !self.is_finished() {
            let done = self.analyses.iter().filter(|slot| slot.is_some()).count();
            let total = self.analyses.len();
            ui.label("Stockfish is analysing every move...");
            ui.add(
                egui::ProgressBar::new(done as f32 / total as f32)
                    .text(format!("{done} / {total} positions")),
            );
            return back;
        }
        if self.reviews.is_empty() {
            ui.label("No moves were played.");
            return back;
        }

        self.navigation(ui);
        ui.separator();
        self.current_move_details(ui);
        ui.separator();
        self.summaries(ui, player);
        ui.separator();
        self.move_list(ui);
        back
    }

    /// ◀ ▶ buttons and the arrow keys.
    fn navigation(&mut self, ui: &mut egui::Ui) {
        let last = self.reviews.len() - 1;
        let (left, right) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::ArrowLeft),
                input.key_pressed(egui::Key::ArrowRight),
            )
        });
        ui.horizontal(|ui| {
            if (ui.button("◀ Previous").clicked() || left) && self.current > 0 {
                self.current -= 1;
            }
            if (ui.button("Next ▶").clicked() || right) && self.current < last {
                self.current += 1;
            }
            ui.label("(or use ← →)");
        });
    }

    /// The move being looked at: its quality, the evaluation and the explanation.
    fn current_move_details(&self, ui: &mut egui::Ui) {
        let review = &self.reviews[self.current];
        let number = self.current / 2 + 1;
        let dots = if review.mover == Color::White {
            "."
        } else {
            "..."
        };
        ui.label(
            RichText::new(format!(
                "{number}{dots} {}{}  {}",
                review.san,
                review.quality.symbol(),
                review.quality.label()
            ))
            .size(20.0)
            .color(quality_color(review.quality)),
        );
        ui.label(format!(
            "Evaluation: {} → {}",
            analysis::format_eval(review.eval_before),
            analysis::format_eval(review.eval_after)
        ));
        ui.add(egui::Label::new(&review.explanation).wrap());
    }

    /// Move-quality totals and advice for each player.
    fn summaries(&mut self, ui: &mut egui::Ui, player: Option<Color>) {
        for color in Color::ALL {
            if player.is_some_and(|player| player != color) {
                continue; // against the bot, only your own summary
            }
            let summary = analysis::summarize(&self.reviews, color);
            let name = if player == Some(color) {
                "You".to_string()
            } else {
                format!("{color:?}")
            };
            ui.strong(format!(
                "{name}: average loss {} centipawns per move",
                summary.average_loss
            ));
            ui.horizontal_wrapped(|ui| {
                for quality in MoveQuality::ALL {
                    let count = summary.counts[quality as usize];
                    ui.colored_label(
                        quality_color(quality),
                        format!("{}: {count}", quality.label()),
                    );
                }
            });
            if !summary.worst_moves.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Key moments:");
                    for &index in &summary.worst_moves {
                        let label = format!(
                            "{}{}",
                            self.reviews[index].san,
                            self.reviews[index].quality.symbol()
                        );
                        if ui.link(label).clicked() {
                            self.current = index;
                        }
                    }
                });
            }
            ui.add(egui::Label::new(RichText::new(analysis::advice(&summary)).italics()).wrap());
            ui.add_space(6.0);
        }
    }

    /// Every move, colored by quality. Click one to jump to it.
    fn move_list(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (row, pair) in self.reviews.chunks(2).enumerate() {
                ui.horizontal(|ui| {
                    ui.monospace(format!("{:>3}.", row + 1));
                    for (offset, review) in pair.iter().enumerate() {
                        let index = row * 2 + offset;
                        let text =
                            RichText::new(format!("{}{}", review.san, review.quality.symbol()))
                                .monospace()
                                .color(quality_color(review.quality));
                        if ui.selectable_label(index == self.current, text).clicked() {
                            self.current = index;
                        }
                    }
                });
            }
        });
    }
}

/// Green for good moves, then yellow, orange and red as moves get worse.
fn quality_color(quality: MoveQuality) -> Color32 {
    match quality {
        MoveQuality::Best => Color32::from_rgb(90, 200, 90),
        MoveQuality::Good => Color32::from_rgb(160, 200, 160),
        MoveQuality::Inaccuracy => Color32::from_rgb(230, 200, 60),
        MoveQuality::Mistake => Color32::from_rgb(240, 140, 40),
        MoveQuality::Blunder => Color32::from_rgb(230, 60, 60),
    }
}
