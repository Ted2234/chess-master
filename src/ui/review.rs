use std::iter;

use eframe::egui::{self, Align2, Color32, FontId, RichText, Sense, Stroke, Vec2};

use crate::analysis::{self, MoveQuality, MoveReview};
use crate::engine::Analysis;
use crate::game::PlayedMove;
use crate::piece::Color;
use crate::position::Position;
use crate::san;
use crate::square::Square;
use crate::ui::board_view::{BEST_MOVE_ARROW, PLAYED_MOVE_ARROW};
use crate::ui::{theme, widgets};

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

    /// The squares of the move that led to the position on the board, if any.
    pub fn last_move(&self) -> Option<(Square, Square)> {
        let previous = self.current.checked_sub(1)?;
        let played = self.history.get(previous)?;
        Some((played.mv.from, played.mv.to))
    }

    /// The evaluation for White before the move being looked at, for the eval bar.
    /// Shows 0 (equal) until the analysis is done.
    pub fn eval(&self) -> i32 {
        self.reviews
            .get(self.current)
            .map_or(0, |review| review.eval_before)
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
        let mut back = false;
        let ready = self.is_finished() && !self.reviews.is_empty();

        egui::Panel::top("review_top")
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Game review").size(24.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // The same height as the other buttons, but only as wide as its text.
                        let button = egui::Button::new("Main menu").min_size(Vec2::new(0.0, 34.0));
                        back = ui.add(button).clicked();
                    });
                });
                ui.add_space(4.0);

                if !self.is_finished() {
                    let done = self.analyses.iter().filter(|slot| slot.is_some()).count();
                    let total = self.analyses.len();
                    ui.label("Stockfish is analysing every move...");
                    ui.add(
                        egui::ProgressBar::new(done as f32 / total as f32)
                            .text(format!("{done} / {total} positions")),
                    );
                } else if self.reviews.is_empty() {
                    ui.label("No moves were played.");
                } else {
                    self.summaries(ui, player);
                    self.current_move_card(ui);
                    ui.add_space(4.0);
                }
            });
        if !ready {
            return back;
        }

        egui::Panel::bottom("review_navigation")
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_space(4.0);
                self.navigation(ui);
                ui.add_space(8.0);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.move_list(ui));
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
        ui.columns(2, |columns| {
            let previous = widgets::wide_button(&mut columns[0], self.current > 0, "◀ Previous");
            if (previous || left) && self.current > 0 {
                self.current -= 1;
            }
            let next = widgets::wide_button(&mut columns[1], self.current < last, "Next ▶");
            if (next || right) && self.current < last {
                self.current += 1;
            }
        });
        ui.vertical_centered(|ui| ui.weak("or use the ← → keys"));
    }

    /// A card about the move being looked at, with a border in its quality color:
    /// the move, the explanation, the evaluation and the better move.
    fn current_move_card(&self, ui: &mut egui::Ui) {
        let review = &self.reviews[self.current];
        let color = quality_color(review.quality);
        let number = self.current / 2 + 1;
        let dots = if review.mover == Color::White {
            "."
        } else {
            "..."
        };

        egui::Frame::new()
            .fill(theme::card_color(ui.visuals()))
            .stroke(Stroke::new(2.0, color))
            .corner_radius(10)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    badge(ui, review.quality, BADGE_SIZE);
                    let title = format!("{number}{dots} {}{}", review.san, review.quality.symbol());
                    ui.label(RichText::new(title).size(18.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(color, review.quality.label());
                    });
                });
                ui.add(egui::Label::new(&review.explanation).wrap());
                ui.weak(format!(
                    "Evaluation {} → {}",
                    analysis::format_eval(review.eval_before),
                    analysis::format_eval(review.eval_after)
                ));

                // The better move (the green arrow), in a green "pill".
                if review.quality >= MoveQuality::Inaccuracy
                    && let Some(best) = review.best_move
                {
                    let better = san::san(self.position(), best);
                    egui::Frame::new()
                        .fill(BEST_MOVE_ARROW.gamma_multiply(0.25))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            let text = RichText::new(format!("Better: {better}")).strong();
                            ui.colored_label(quality_color(MoveQuality::Best), text);
                        });
                }
            });
    }

    /// A card per player: the average loss as a big number, a badge with the count for each
    /// quality, the key moments and a piece of advice.
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

            widgets::card(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.weak(name);
                        let average = RichText::new(summary.average_loss.to_string()).size(34.0);
                        ui.label(average.strong());
                        ui.weak("avg. loss (cp)");
                    });
                    // Push the five badges to the right edge of the card.
                    let spacing = ui.spacing().item_spacing.x;
                    let badges_width = 5.0 * BADGE_SIZE + 4.0 * spacing;
                    ui.add_space((ui.available_width() - badges_width).max(0.0));
                    for quality in MoveQuality::ALL {
                        let count = summary.counts[quality as usize];
                        // A narrow column: the badge, and the count centered under it.
                        let size = Vec2::new(BADGE_SIZE, 2.0 * BADGE_SIZE);
                        let layout = egui::Layout::top_down(egui::Align::Center);
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            badge(ui, quality, BADGE_SIZE);
                            ui.label(count.to_string());
                        });
                    }
                });

                if !summary.worst_moves.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak("Key moments:");
                        for &index in &summary.worst_moves {
                            let review = &self.reviews[index];
                            let label = format!("{}{}", review.san, review.quality.symbol());
                            if ui.link(label).clicked() {
                                self.current = index;
                            }
                        }
                    });
                }
                let advice = RichText::new(analysis::advice(&summary)).italics();
                ui.add(egui::Label::new(advice).wrap());
            });
            ui.add_space(6.0);
        }
    }

    /// Every move with a dot in its quality color. Click one to jump to it.
    fn move_list(&mut self, ui: &mut egui::Ui) {
        widgets::card(ui, |ui| {
            ui.set_min_size(ui.available_size()); // fill the space between the cards and buttons
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    egui::Grid::new("review_moves")
                        .num_columns(3)
                        .show(ui, |ui| {
                            for row in 0..self.reviews.len().div_ceil(2) {
                                ui.weak(format!("{}.", row + 1));
                                self.move_cell(ui, row * 2);
                                if row * 2 + 1 < self.reviews.len() {
                                    self.move_cell(ui, row * 2 + 1);
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    }

    /// One move in the list: a dot in its quality color, then the move. Click it to jump to it.
    fn move_cell(&mut self, ui: &mut egui::Ui, index: usize) {
        let review = &self.reviews[index];
        let quality = review.quality;
        let selected = index == self.current;
        // Only inaccuracies and worse get colored text, like on chess sites.
        let color = if selected {
            theme::ON_ACCENT
        } else if quality >= MoveQuality::Inaccuracy {
            quality_color(quality)
        } else {
            ui.visuals().strong_text_color()
        };
        let text = RichText::new(format!("{}{}", review.san, quality.symbol()))
            .monospace()
            .size(15.0)
            .color(color);

        let clicked = ui
            .horizontal(|ui| {
                badge(ui, quality, 10.0);
                ui.selectable_label(selected, text).clicked()
            })
            .inner;
        if clicked {
            self.current = index;
        }
    }
}

/// The size of the quality badges in the summary and the move card.
const BADGE_SIZE: f32 = 26.0;

/// A colored circle for a move quality, with its symbol inside when it's big enough.
fn badge(ui: &mut egui::Ui, quality: MoveQuality, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter();
    painter.circle_filled(rect.center(), size / 2.0, quality_color(quality));
    if size >= 20.0 {
        let symbol = match quality {
            MoveQuality::Best => "★",
            MoveQuality::Good => "✓",
            _ => quality.symbol(),
        };
        let font = FontId::proportional(size * 0.5);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            symbol,
            font,
            Color32::BLACK,
        );
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
