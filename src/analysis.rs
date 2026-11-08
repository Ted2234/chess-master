use crate::engine::{Analysis, Score};
use crate::game::PlayedMove;
use crate::moves::Move;
use crate::piece::{Color, PieceKind};
use crate::position::Position;
use crate::san;

/// A forced mate counts as this many centipawns (minus the moves needed).
const MATE_VALUE: i32 = 10_000;
/// When measuring a mistake, evaluations are capped at ±10 pawns, so going from
/// "winning by 15" to "winning by 12" doesn't count as a blunder.
const EVAL_CAP: i32 = 1_000;

/// How good a move was, measured by how much it gave away compared to the best move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MoveQuality {
    Best,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

impl MoveQuality {
    /// Every quality, from best to worst.
    pub const ALL: [MoveQuality; 5] = [
        MoveQuality::Best,
        MoveQuality::Good,
        MoveQuality::Inaccuracy,
        MoveQuality::Mistake,
        MoveQuality::Blunder,
    ];

    /// Classifies a move by its loss in centipawns (100 = one pawn).
    pub fn from_loss(loss: i32) -> Self {
        match loss {
            ..=10 => MoveQuality::Best,
            11..=50 => MoveQuality::Good,
            51..=100 => MoveQuality::Inaccuracy,
            101..=300 => MoveQuality::Mistake,
            _ => MoveQuality::Blunder,
        }
    }

    /// The name shown in the review.
    pub fn label(self) -> &'static str {
        match self {
            MoveQuality::Best => "Best",
            MoveQuality::Good => "Good",
            MoveQuality::Inaccuracy => "Inaccuracy",
            MoveQuality::Mistake => "Mistake",
            MoveQuality::Blunder => "Blunder",
        }
    }

    /// The symbol chess players write after a move: `?!`, `?`, `??`.
    pub fn symbol(self) -> &'static str {
        match self {
            MoveQuality::Best | MoveQuality::Good => "",
            MoveQuality::Inaccuracy => "?!",
            MoveQuality::Mistake => "?",
            MoveQuality::Blunder => "??",
        }
    }
}

/// Turns an engine score (which is for the side to move) into centipawns for White.
pub fn white_centipawns(score: Score, side_to_move: Color) -> i32 {
    let for_side_to_move = match score {
        Score::Centipawns(cp) => cp,
        Score::Mate(moves) if moves > 0 => MATE_VALUE - moves,
        // 0 or negative: the side to move is (getting) mated.
        Score::Mate(moves) => -MATE_VALUE - moves,
    };
    match side_to_move {
        Color::White => for_side_to_move,
        Color::Black => -for_side_to_move,
    }
}

/// Formats centipawns for White like a chess site: `+0.8`, `-2.3`, `#3`, `#-2`.
pub fn format_eval(white_cp: i32) -> String {
    if white_cp.abs() > MATE_VALUE - 1_000 {
        let moves = MATE_VALUE - white_cp.abs();
        match (moves, white_cp > 0) {
            (0, _) => "checkmate".to_string(),
            (_, true) => format!("#{moves}"),
            (_, false) => format!("#-{moves}"),
        }
    } else {
        format!("{:+.1}", white_cp as f32 / 100.0)
    }
}

/// The review of one move.
#[derive(Debug, Clone)]
pub struct MoveReview {
    pub mover: Color,
    /// The move in standard notation, e.g. `Nf3`.
    pub san: String,
    pub quality: MoveQuality,
    /// How many centipawns the move gave away compared to the best move.
    pub loss: i32,
    /// Evaluation for White before and after the move.
    pub eval_before: i32,
    pub eval_after: i32,
    /// What the engine would have played instead.
    pub best_move: Option<Move>,
    /// A few sentences about what happened.
    pub explanation: String,
}

/// Reviews every move of a game.
///
/// `analyses[i]` is the engine's analysis of the position *before* move `i`,
/// and the last one is the position after the final move, so there is one
/// more analysis than there are moves.
pub fn review_game(history: &[PlayedMove], analyses: &[Analysis]) -> Vec<MoveReview> {
    history
        .iter()
        .zip(analyses.windows(2))
        .map(|(played, pair)| review_move(played, &pair[0], &pair[1]))
        .collect()
}

/// Compares the played move with the engine's best move.
fn review_move(played: &PlayedMove, before: &Analysis, after: &Analysis) -> MoveReview {
    let mover = played.before.side_to_move;
    let eval_before = white_centipawns(before.score, mover);
    let eval_after = white_centipawns(after.score, mover.opposite());
    let best_move = Move::from_uci(&before.best_move);

    let loss = if best_move == Some(played.mv) {
        0
    } else {
        // Positive = the position got worse for the player who moved.
        let change = eval_before.clamp(-EVAL_CAP, EVAL_CAP) - eval_after.clamp(-EVAL_CAP, EVAL_CAP);
        let for_mover = match mover {
            Color::White => change,
            Color::Black => -change,
        };
        for_mover.max(0)
    };
    let quality = MoveQuality::from_loss(loss);

    MoveReview {
        mover,
        san: played.san.clone(),
        quality,
        loss,
        eval_before,
        eval_after,
        best_move,
        explanation: explain(played, quality, loss, before, after, best_move),
    }
}

/// Writes a short explanation: how bad the move was, what it allows, and what was better.
fn explain(
    played: &PlayedMove,
    quality: MoveQuality,
    loss: i32,
    before: &Analysis,
    after: &Analysis,
    best_move: Option<Move>,
) -> String {
    let position = &played.before;
    let mut sentences = Vec::new();

    match quality {
        MoveQuality::Best => sentences.push(format!("{} is the best move here.", played.san)),
        MoveQuality::Good => sentences.push(format!("{} is a good move.", played.san)),
        _ => {
            let article = if quality == MoveQuality::Inaccuracy {
                "an"
            } else {
                "a"
            };
            sentences.push(format!(
                "{} is {article} {}: it gives away about {:.1} pawns.",
                played.san,
                quality.label().to_lowercase(),
                loss as f32 / 100.0
            ));
        }
    }
    if quality < MoveQuality::Inaccuracy {
        return sentences.join(" ");
    }

    // What does the opponent get now?
    let mut after_position = position.clone();
    after_position.make_move(played.mv);
    if let Score::Mate(moves) = after.score
        && moves > 0
    {
        sentences.push(format!("It allows a forced mate in {moves}."));
    } else if let Some(reply) = Move::from_uci(&after.best_move)
        && let Some(kind) = captured_kind(&after_position, reply)
    {
        sentences.push(format!(
            "Your opponent can answer {}, taking your {}.",
            san::san(&after_position, reply),
            kind.name()
        ));
    }

    // What was better?
    if let Some(best) = best_move {
        let best_san = san::san(position, best);
        if let Score::Mate(moves) = before.score
            && moves > 0
        {
            sentences.push(format!(
                "You had a forced mate in {moves}, starting with {best_san}."
            ));
        } else if let Some(kind) = captured_kind(position, best) {
            sentences.push(format!("Better was {best_san}, taking a {}.", kind.name()));
        } else {
            sentences.push(format!("Better was {best_san}."));
        }
    }

    sentences.join(" ")
}

/// The kind of piece a move captures, if it captures anything (including en passant).
fn captured_kind(position: &Position, mv: Move) -> Option<PieceKind> {
    if let Some(piece) = position.board.get(mv.to) {
        return Some(piece.kind);
    }
    let is_pawn = position
        .board
        .get(mv.from)
        .is_some_and(|piece| piece.kind == PieceKind::Pawn);
    (is_pawn && Some(mv.to) == position.en_passant).then_some(PieceKind::Pawn)
}

/// Totals for one player's moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// How many moves of each quality, in the order of `MoveQuality::ALL`.
    pub counts: [usize; 5],
    /// The average loss per move in centipawns ("ACPL"). Lower is better.
    pub average_loss: i32,
    /// Indexes (into the reviews) of the player's worst moves, worst first.
    pub worst_moves: Vec<usize>,
}

/// Adds up the reviews of the moves made by `color`.
pub fn summarize(reviews: &[MoveReview], color: Color) -> Summary {
    let mine: Vec<(usize, &MoveReview)> = reviews
        .iter()
        .enumerate()
        .filter(|(_, review)| review.mover == color)
        .collect();

    let mut counts = [0; 5];
    for (_, review) in &mine {
        counts[review.quality as usize] += 1;
    }

    let total_loss: i32 = mine.iter().map(|(_, review)| review.loss).sum();
    let average_loss = if mine.is_empty() {
        0
    } else {
        total_loss / mine.len() as i32
    };

    let mut worst: Vec<(usize, &MoveReview)> = mine
        .into_iter()
        .filter(|(_, review)| review.quality >= MoveQuality::Mistake)
        .collect();
    worst.sort_by_key(|(_, review)| std::cmp::Reverse(review.loss));
    let worst_moves = worst.iter().take(3).map(|(index, _)| *index).collect();

    Summary {
        counts,
        average_loss,
        worst_moves,
    }
}

/// One piece of advice, based on the kind of mistakes made.
pub fn advice(summary: &Summary) -> &'static str {
    let blunders = summary.counts[MoveQuality::Blunder as usize];
    let mistakes = summary.counts[MoveQuality::Mistake as usize];
    if blunders > 0 {
        "Before every move, check: which of my pieces are attacked, and is each one defended? \
         Then look at every check and capture your opponent could answer with."
    } else if mistakes > 0 {
        "Good, no blunders! To cut down on mistakes, compare two or three candidate \
         moves before you play, instead of the first one you see."
    } else if summary.average_loss > 30 {
        "Solid game. Look at the better moves the engine found to sharpen your plans."
    } else {
        "Excellent accuracy. Try a higher bot rating!"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn analysis(score: Score, best_move: &str) -> Analysis {
        Analysis {
            score,
            best_move: best_move.to_string(),
        }
    }

    /// Plays UCI moves and returns the game.
    fn game_after(moves: &[&str]) -> Game {
        let mut game = Game::new();
        for text in moves {
            game.play(Move::from_uci(text).unwrap()).unwrap();
        }
        game
    }

    #[test]
    fn quality_thresholds() {
        assert_eq!(MoveQuality::from_loss(0), MoveQuality::Best);
        assert_eq!(MoveQuality::from_loss(30), MoveQuality::Good);
        assert_eq!(MoveQuality::from_loss(80), MoveQuality::Inaccuracy);
        assert_eq!(MoveQuality::from_loss(200), MoveQuality::Mistake);
        assert_eq!(MoveQuality::from_loss(900), MoveQuality::Blunder);
    }

    #[test]
    fn scores_are_converted_to_whites_view() {
        assert_eq!(white_centipawns(Score::Centipawns(50), Color::White), 50);
        assert_eq!(white_centipawns(Score::Centipawns(50), Color::Black), -50);
        assert_eq!(white_centipawns(Score::Mate(2), Color::White), 9_998);
        assert_eq!(white_centipawns(Score::Mate(-1), Color::Black), 9_999);
        assert_eq!(white_centipawns(Score::Mate(0), Color::White), -10_000);
    }

    #[test]
    fn evals_are_formatted() {
        assert_eq!(format_eval(80), "+0.8");
        assert_eq!(format_eval(-230), "-2.3");
        assert_eq!(format_eval(9_997), "#3");
        assert_eq!(format_eval(-9_998), "#-2");
        assert_eq!(format_eval(-10_000), "checkmate");
    }

    #[test]
    fn fools_mate_blunder_is_explained() {
        // 1. f3 e5 2. g4?? Qh4#
        let game = game_after(&["f2f3", "e7e5", "g2g4", "d8h4"]);
        let analyses = [
            analysis(Score::Centipawns(30), "e2e4"),  // before f3
            analysis(Score::Centipawns(60), "e7e5"),  // before e5
            analysis(Score::Centipawns(-80), "e2e4"), // before g4 (White to move)
            analysis(Score::Mate(1), "d8h4"),         // before Qh4# (Black mates in 1)
            analysis(Score::Mate(0), "(none)"),       // final position: White is mated
        ];
        let reviews = review_game(game.history(), &analyses);
        assert_eq!(reviews.len(), 4);

        let g4 = &reviews[2];
        assert_eq!(g4.san, "g4");
        assert_eq!(g4.quality, MoveQuality::Blunder);
        assert!(
            g4.explanation.contains("forced mate in 1"),
            "{}",
            g4.explanation
        );
        assert!(
            g4.explanation.contains("Better was e4"),
            "{}",
            g4.explanation
        );

        let qh4 = &reviews[3];
        assert_eq!(qh4.quality, MoveQuality::Best);
        assert_eq!(qh4.loss, 0);
    }

    #[test]
    fn hanging_piece_is_explained() {
        // 1. e4 e5 2. Qh5 Nc6 3. Qxf7+?? (the queen is simply taken by the king)
        let game = game_after(&["e2e4", "e7e5", "d1h5", "b8c6", "h5f7"]);
        let analyses = [
            analysis(Score::Centipawns(30), "e2e4"),
            analysis(Score::Centipawns(-30), "e7e5"),
            analysis(Score::Centipawns(20), "g1f3"),
            analysis(Score::Centipawns(-10), "g7g6"),
            analysis(Score::Centipawns(40), "f1c4"), // before Qxf7+
            analysis(Score::Centipawns(800), "e8f7"), // Black to move: Kxf7 wins the queen
        ];
        let reviews = review_game(game.history(), &analyses);
        let blunder = &reviews[4];
        assert_eq!(blunder.quality, MoveQuality::Blunder);
        assert!(
            blunder.explanation.contains("Kxf7, taking your queen"),
            "{}",
            blunder.explanation
        );
        assert!(
            blunder.explanation.contains("Better was Bc4"),
            "{}",
            blunder.explanation
        );
    }

    #[test]
    fn summary_counts_one_side() {
        let game = game_after(&["f2f3", "e7e5", "g2g4", "d8h4"]);
        let analyses = [
            analysis(Score::Centipawns(30), "e2e4"),
            analysis(Score::Centipawns(60), "e7e5"),
            analysis(Score::Centipawns(-80), "e2e4"),
            analysis(Score::Mate(1), "d8h4"),
            analysis(Score::Mate(0), "(none)"),
        ];
        let reviews = review_game(game.history(), &analyses);
        let white = summarize(&reviews, Color::White);
        assert_eq!(white.counts[MoveQuality::Blunder as usize], 1);
        assert_eq!(white.worst_moves, [2]);
        assert!(advice(&white).contains("attacked"));
    }
}
