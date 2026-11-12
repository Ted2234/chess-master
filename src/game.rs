use std::fmt;

use crate::moves::Move;
use crate::piece::{Color, Piece, PieceKind};
use crate::position::Position;
use crate::rules::{self, DrawReason, GameStatus};
use crate::san;

/// How a finished game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResult {
    Checkmate { winner: Color },
    Timeout { winner: Color },
    Resignation { winner: Color },
    Stalemate,
    Draw(DrawReason),
}

impl fmt::Display for GameResult {
    /// A sentence for the player, e.g. "Checkmate! White wins."
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameResult::Checkmate { winner } => write!(f, "Checkmate! {winner:?} wins."),
            GameResult::Timeout { winner } => write!(
                f,
                "{:?} ran out of time. {winner:?} wins!",
                winner.opposite()
            ),
            GameResult::Resignation { winner } => {
                write!(f, "{:?} resigned. {winner:?} wins.", winner.opposite())
            }
            GameResult::Stalemate => write!(f, "Stalemate. It's a draw."),
            GameResult::Draw(DrawReason::FiftyMoveRule) => write!(f, "Draw by the 50-move rule."),
            GameResult::Draw(DrawReason::InsufficientMaterial) => {
                write!(f, "Draw: not enough pieces left to checkmate.")
            }
            GameResult::Draw(DrawReason::ThreefoldRepetition) => {
                write!(f, "Draw by threefold repetition.")
            }
        }
    }
}

impl GameResult {
    /// Returns the winner, or `None` for a draw.
    pub fn winner(self) -> Option<Color> {
        match self {
            GameResult::Checkmate { winner }
            | GameResult::Timeout { winner }
            | GameResult::Resignation { winner } => Some(winner),
            GameResult::Stalemate | GameResult::Draw(_) => None,
        }
    }

    /// Returns how the game ended, as a short line for under the result: "by checkmate", ...
    pub fn reason(self) -> &'static str {
        match self {
            GameResult::Checkmate { .. } => "by checkmate",
            GameResult::Timeout { .. } => "on time",
            GameResult::Resignation { .. } => "by resignation",
            GameResult::Stalemate => "by stalemate",
            GameResult::Draw(DrawReason::FiftyMoveRule) => "by the 50-move rule",
            GameResult::Draw(DrawReason::InsufficientMaterial) => "not enough pieces to checkmate",
            GameResult::Draw(DrawReason::ThreefoldRepetition) => "by threefold repetition",
        }
    }

    /// Returns the score the way chess players write it: `1-0`, `0-1` or `½-½`.
    pub fn score(self) -> &'static str {
        match self.winner() {
            Some(Color::White) => "1-0",
            Some(Color::Black) => "0-1",
            None => "½-½",
        }
    }
}

/// Why `Game::play` refused a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// The game has ended.
    GameOver,
    /// The move breaks the rules of this position.
    Illegal,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveError::GameOver => write!(f, "the game is over"),
            MoveError::Illegal => write!(f, "that move is not legal"),
        }
    }
}

impl std::error::Error for MoveError {}

/// One move from the game's history.
#[derive(Debug, Clone)]
pub struct PlayedMove {
    pub mv: Move,
    /// The move in standard notation, e.g. `Nf3`.
    pub san: String,
    /// The position just before the move was played.
    pub before: Position,
}

impl PlayedMove {
    /// Returns the piece this move captured, if any.
    pub fn captured(&self) -> Option<Piece> {
        let board = &self.before.board;
        if let Some(piece) = board.get(self.mv.to) {
            return Some(piece);
        }
        // En passant: the pawn moves to an empty square and takes the pawn beside it.
        let mover = board.get(self.mv.from)?;
        let en_passant =
            mover.kind == PieceKind::Pawn && Some(self.mv.to) == self.before.en_passant;
        en_passant.then_some(Piece::new(mover.color.opposite(), PieceKind::Pawn))
    }
}

/// A game of chess: the current position, every move played, and the result.
#[derive(Debug, Clone)]
pub struct Game {
    position: Position,
    history: Vec<PlayedMove>,
    result: Option<GameResult>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// A new game from the starting position.
    pub fn new() -> Self {
        Self {
            position: Position::starting(),
            history: Vec::new(),
            result: None,
        }
    }

    /// The current position.
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// Every move played so far, oldest first.
    pub fn history(&self) -> &[PlayedMove] {
        &self.history
    }

    /// How the game ended, or `None` while it's still going.
    pub fn result(&self) -> Option<GameResult> {
        self.result
    }

    /// The legal moves in the current position (none once the game is over).
    pub fn legal_moves(&self) -> Vec<Move> {
        if self.result.is_some() {
            Vec::new()
        } else {
            rules::legal_moves(&self.position)
        }
    }

    /// Plays a move if it's legal, and checks whether the game has ended.
    pub fn play(&mut self, mv: Move) -> Result<(), MoveError> {
        if self.result.is_some() {
            return Err(MoveError::GameOver);
        }
        if !rules::legal_moves(&self.position).contains(&mv) {
            return Err(MoveError::Illegal);
        }

        let san = san::san(&self.position, mv);
        let before = self.position.clone();
        self.position.make_move(mv);
        self.history.push(PlayedMove { mv, san, before });
        self.result = self.result_by_rules();
        Ok(())
    }

    /// Takes back the last move. Returns it, or `None` if no moves were played.
    pub fn undo(&mut self) -> Option<Move> {
        let last = self.history.pop()?;
        self.position = last.before;
        self.result = None;
        Some(last.mv)
    }

    /// Returns the pieces `color` has captured so far, cheapest first.
    pub fn captured_by(&self, color: Color) -> Vec<Piece> {
        let mut pieces: Vec<Piece> = self
            .history
            .iter()
            .filter(|played| played.before.side_to_move == color)
            .filter_map(PlayedMove::captured)
            .collect();
        pieces.sort_by_key(|piece| piece.kind.value());
        pieces
    }

    /// Ends the game for a reason outside the board: time ran out or a resignation.
    pub fn end(&mut self, result: GameResult) {
        if self.result.is_none() {
            self.result = Some(result);
        }
    }

    /// Checkmate, stalemate or a draw in the current position, if any.
    fn result_by_rules(&self) -> Option<GameResult> {
        match rules::status(&self.position) {
            GameStatus::Checkmate { winner } => Some(GameResult::Checkmate { winner }),
            GameStatus::Stalemate => Some(GameResult::Stalemate),
            GameStatus::Draw(reason) => Some(GameResult::Draw(reason)),
            GameStatus::Ongoing if self.is_threefold_repetition() => {
                Some(GameResult::Draw(DrawReason::ThreefoldRepetition))
            }
            GameStatus::Ongoing => None,
        }
    }

    /// True if the current position has now appeared for the third time.
    fn is_threefold_repetition(&self) -> bool {
        let current = repetition_key(&self.position);
        let earlier = self
            .history
            .iter()
            .filter(|played| repetition_key(&played.before) == current)
            .count();
        earlier >= 2
    }
}

/// What makes two positions "the same" for repetition: the pieces, the side to move, castling
/// rights, and en passant. That's FEN without the two counters.
///
/// The en passant square only counts if a pawn can really capture there; FEN writes it
/// after every double pawn push.
fn repetition_key(position: &Position) -> String {
    let mut position = position.clone();
    if !can_capture_en_passant(&position) {
        position.en_passant = None;
    }
    position
        .to_fen()
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}

/// True if the side to move has a legal en passant capture.
fn can_capture_en_passant(position: &Position) -> bool {
    let Some(target) = position.en_passant else {
        return false;
    };
    // The square behind a pawn that just moved two squares is empty, so the only way a pawn
    // can get there is by taking en passant.
    rules::legal_moves(position).iter().any(|mv| {
        mv.to == target
            && position
                .board
                .get(mv.from)
                .is_some_and(|piece| piece.kind == PieceKind::Pawn)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::START_FEN;

    /// Plays moves given in UCI style ("e2e4").
    fn play(game: &mut Game, moves: &[&str]) {
        for text in moves {
            let mv = game
                .legal_moves()
                .into_iter()
                .find(|mv| mv.to_string() == *text)
                .unwrap_or_else(|| panic!("{text} is not legal"));
            game.play(mv).unwrap();
        }
    }

    #[test]
    fn history_records_san() {
        let mut game = Game::new();
        play(&mut game, &["e2e4", "e7e5", "g1f3"]);
        let san: Vec<&str> = game.history().iter().map(|m| m.san.as_str()).collect();
        assert_eq!(san, ["e4", "e5", "Nf3"]);
    }

    #[test]
    fn illegal_moves_are_rejected() {
        let mut game = Game::new();
        let e2 = crate::square::Square::from_name("e2").unwrap();
        let e5 = crate::square::Square::from_name("e5").unwrap();
        assert_eq!(game.play(Move::new(e2, e5)), Err(MoveError::Illegal));
        assert!(game.history().is_empty());
    }

    #[test]
    fn undo_restores_the_position() {
        let mut game = Game::new();
        play(&mut game, &["e2e4", "d7d5", "e4d5"]);
        game.undo();
        game.undo();
        game.undo();
        assert_eq!(game.position().to_fen(), START_FEN);
        assert_eq!(game.undo(), None);
    }

    #[test]
    fn checkmate_ends_the_game() {
        let mut game = Game::new();
        play(&mut game, &["f2f3", "e7e5", "g2g4", "d8h4"]);
        assert_eq!(
            game.result(),
            Some(GameResult::Checkmate {
                winner: Color::Black
            })
        );
        assert_eq!(game.history()[3].san, "Qh4#");
        assert!(game.legal_moves().is_empty());
    }

    #[test]
    fn results_know_winner_reason_and_score() {
        let mate = GameResult::Checkmate {
            winner: Color::Black,
        };
        assert_eq!(mate.winner(), Some(Color::Black));
        assert_eq!(mate.reason(), "by checkmate");
        assert_eq!(mate.score(), "0-1");

        let draw = GameResult::Draw(DrawReason::ThreefoldRepetition);
        assert_eq!(draw.winner(), None);
        assert_eq!(draw.score(), "½-½");
    }

    #[test]
    fn captured_pieces_are_listed() {
        let mut game = Game::new();
        // exd5 takes a pawn, Qxd5 takes it back.
        play(&mut game, &["e2e4", "d7d5", "e4d5", "d8d5", "b1c3"]);
        let white_pawn = Piece::new(Color::White, PieceKind::Pawn);
        let black_pawn = Piece::new(Color::Black, PieceKind::Pawn);
        assert_eq!(game.captured_by(Color::White), [black_pawn]);
        assert_eq!(game.captured_by(Color::Black), [white_pawn]);
    }

    #[test]
    fn en_passant_captures_a_pawn() {
        let mut game = Game::new();
        // e5 waits on the fifth rank; after d7-d5, exd6 takes the d-pawn en passant.
        play(&mut game, &["e2e4", "a7a6", "e4e5", "d7d5", "e5d6"]);
        let captured = game.history()[4].captured();
        assert_eq!(captured, Some(Piece::new(Color::Black, PieceKind::Pawn)));
    }

    #[test]
    fn repetition_ignores_an_en_passant_square_nobody_can_use() {
        let mut game = Game::new();
        // After 1. e4 the FEN says "e3", but no black pawn can take en passant, so the
        // position is the same as after the knights have gone out and back twice.
        let out_and_back = ["g8f6", "g1f3", "f6g8", "f3g1"];
        play(&mut game, &["e2e4"]);
        play(&mut game, &out_and_back);
        assert_eq!(game.result(), None); // seen twice
        play(&mut game, &out_and_back);
        assert_eq!(
            game.result(),
            Some(GameResult::Draw(DrawReason::ThreefoldRepetition))
        );
    }

    #[test]
    fn en_passant_counts_only_when_a_pawn_can_take() {
        let can_take: Position = "k7/8/8/3pP3/8/8/8/K7 w - d6 0 1".parse().unwrap();
        assert!(can_capture_en_passant(&can_take));
        let no_pawn: Position = "k7/8/8/3p4/8/8/8/K7 w - d6 0 1".parse().unwrap();
        assert!(!can_capture_en_passant(&no_pawn));
    }

    #[test]
    fn threefold_repetition_is_a_draw() {
        let mut game = Game::new();
        let shuffle = ["g1f3", "g8f6", "f3g1", "f6g8"];
        play(&mut game, &shuffle);
        assert_eq!(game.result(), None); // start position seen twice
        play(&mut game, &shuffle);
        assert_eq!(
            game.result(),
            Some(GameResult::Draw(DrawReason::ThreefoldRepetition))
        );
    }
}
