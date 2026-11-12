use std::fmt;

use crate::piece::{Color, Piece, PieceKind};
use crate::position::Position;
use crate::square::Square;

/// A move from one square to another.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
}

impl Move {
    /// Creates a normal (non-promotional) move.
    pub fn new(from: Square, to: Square) -> Self {
        Self {
            from,
            to,
            promotion: None,
        }
    }

    /// Reads a move in UCI style (`e2e4`, `e7e8q`), the format Stockfish uses.
    pub fn from_uci(text: &str) -> Option<Move> {
        let from = Square::from_name(text.get(0..2)?)?;
        let to = Square::from_name(text.get(2..4)?)?;
        let promotion = match text.len() {
            4 => None,
            5 => Some(Piece::from_symbol(text.chars().nth(4)?)?.kind),
            _ => return None,
        };
        Some(Move {
            from,
            to,
            promotion,
        })
    }
}

impl fmt::Display for Move {
    /// Writes the move in UCI style: `e2e4` or `e7e8q` for a promotion.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        if let Some(kind) = self.promotion {
            write!(f, "{}", Piece::new(Color::Black, kind).symbol())?;
        }
        Ok(())
    }
}

/// The 8 jumps a knight can make as (rank, file) steps.
pub const KNIGHT_JUMPS: [(isize, isize); 8] = [
    (2, 1),
    (1, 2),
    (-1, 2),
    (-2, 1),
    (-2, -1),
    (-1, -2),
    (1, -2),
    (2, -1),
];

/// The 8 squares around a king.
pub const KING_STEPS: [(isize, isize); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

/// Directions a rook can slide in: up, down, right, left.
pub const ROOK_DIRECTIONS: [(isize, isize); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Directions a bishop slides in: the four diagonals.
pub const BISHOP_DIRECTIONS: [(isize, isize); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

/// The pieces a pawn can become, the strongest first.
pub const PROMOTION_KINDS: [PieceKind; 4] = [
    PieceKind::Queen,
    PieceKind::Rook,
    PieceKind::Bishop,
    PieceKind::Knight,
];

/// Every move the side to move can make by the movement rules of each piece.
///
/// "Pseudo-legal": this doesn't check yet whether a move leaves your king in check.
pub fn pseudo_legal_moves(position: &Position) -> Vec<Move> {
    let mut moves = Vec::new();

    for from in Square::all() {
        let Some(piece) = position.board.get(from) else {
            continue; // empty square
        };
        if piece.color != position.side_to_move {
            continue; // not our piece
        }

        match piece.kind {
            PieceKind::Pawn => pawn_moves(position, from, &mut moves),
            PieceKind::Knight => step_moves(position, from, &KNIGHT_JUMPS, &mut moves),
            PieceKind::Bishop => slide_moves(position, from, &BISHOP_DIRECTIONS, &mut moves),
            PieceKind::Rook => slide_moves(position, from, &ROOK_DIRECTIONS, &mut moves),
            PieceKind::Queen => {
                slide_moves(position, from, &ROOK_DIRECTIONS, &mut moves);
                slide_moves(position, from, &BISHOP_DIRECTIONS, &mut moves);
            }
            PieceKind::King => {
                step_moves(position, from, &KING_STEPS, &mut moves);
                castling_moves(position, &mut moves);
            }
        }
    }

    moves
}

/// True if the side to move may end a move on `square`: it's empty or holds an enemy piece.
fn can_land_on(position: &Position, square: Square) -> bool {
    match position.board.get(square) {
        None => true,
        Some(piece) => piece.color != position.side_to_move,
    }
}

/// Moves for pieces that jump a fixed distance: the knight and the king.
fn step_moves(position: &Position, from: Square, steps: &[(isize, isize)], moves: &mut Vec<Move>) {
    for &(rank_delta, file_delta) in steps {
        if let Some(to) = from.offset(rank_delta, file_delta)
            && can_land_on(position, to)
        {
            moves.push(Move::new(from, to));
        }
    }
}

/// Moves for pieces that slide until something blocks them: bishop, rook, queen.
fn slide_moves(
    position: &Position,
    from: Square,
    directions: &[(isize, isize)],
    moves: &mut Vec<Move>,
) {
    for &(rank_delta, file_delta) in directions {
        let mut current = from;
        while let Some(to) = current.offset(rank_delta, file_delta) {
            match position.board.get(to) {
                None => moves.push(Move::new(from, to)),
                Some(piece) => {
                    if piece.color != position.side_to_move {
                        moves.push(Move::new(from, to)); // capture
                    }
                    break; // can't jump over pieces
                }
            }
            current = to;
        }
    }
}

/// Pawn moves: one step, two steps from the start, diagonal captures, en passant, promotion.
fn pawn_moves(position: &Position, from: Square, moves: &mut Vec<Move>) {
    let us = position.side_to_move;
    let (forward, start_rank) = match us {
        Color::White => (1, 1),
        Color::Black => (-1, 6),
    };

    // Straight ahead, only onto empty squares.
    if let Some(one_step) = from.offset(forward, 0)
        && position.board.get(one_step).is_none()
    {
        push_pawn_move(from, one_step, moves);

        if from.rank == start_rank
            && let Some(two_steps) = from.offset(2 * forward, 0)
            && position.board.get(two_steps).is_none()
        {
            moves.push(Move::new(from, two_steps));
        }
    }

    // Diagonally, only to capture (or en passant).
    for file_delta in [-1, 1] {
        if let Some(to) = from.offset(forward, file_delta) {
            let enemy_there = position
                .board
                .get(to)
                .is_some_and(|piece| piece.color != us);
            if enemy_there || position.en_passant == Some(to) {
                push_pawn_move(from, to, moves);
            }
        }
    }
}

/// Adds a pawn move, or all four promotions if the pawn reaches the last rank.
fn push_pawn_move(from: Square, to: Square, moves: &mut Vec<Move>) {
    if to.rank == 0 || to.rank == 7 {
        for kind in PROMOTION_KINDS {
            moves.push(Move {
                from,
                to,
                promotion: Some(kind),
            });
        }
    } else {
        moves.push(Move::new(from, to));
    }
}

/// Castling: the king moves two squares towards a rook.
///
/// Here we only check the castling rights and that the squares in between are
/// empty. The "not out of or through check" rules are in `rules::is_legal`.
fn castling_moves(position: &Position, moves: &mut Vec<Move>) {
    let us = position.side_to_move;
    let rights = position.castling;
    let (rank, king_side, queen_side) = match us {
        Color::White => (0, rights.white_king_side, rights.white_queen_side),
        Color::Black => (7, rights.black_king_side, rights.black_queen_side),
    };

    let king_from = Square::new(rank, 4);
    if position.board.get(king_from) != Some(Piece::new(us, PieceKind::King)) {
        return;
    }
    let rook = Some(Piece::new(us, PieceKind::Rook));
    let empty = |file: usize| position.board.get(Square::new(rank, file)).is_none();

    if king_side && empty(5) && empty(6) && position.board.get(Square::new(rank, 7)) == rook {
        moves.push(Move::new(king_from, Square::new(rank, 6)));
    }
    if queen_side
        && empty(1)
        && empty(2)
        && empty(3)
        && position.board.get(Square::new(rank, 0)) == rook
    {
        moves.push(Move::new(king_from, Square::new(rank, 2)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::START_FEN;

    fn position(fen: &str) -> Position {
        fen.parse().unwrap()
    }

    /// The moves of the piece on `from`, as UCI strings, sorted.
    fn moves_from(fen: &str, from: &str) -> Vec<String> {
        let from = Square::from_name(from).unwrap();
        let mut moves: Vec<String> = pseudo_legal_moves(&position(fen))
            .into_iter()
            .filter(|mv| mv.from == from)
            .map(|mv| mv.to_string())
            .collect();
        moves.sort();
        moves
    }

    #[test]
    fn uci_round_trips() {
        for text in ["e2e4", "g8f7", "e7e8q", "a2a1n"] {
            assert_eq!(Move::from_uci(text).unwrap().to_string(), text);
        }
        for bad in ["", "e2", "e2e9", "e7e8x", "e2e4e5"] {
            assert_eq!(Move::from_uci(bad), None, "{bad} should be invalid");
        }
    }

    #[test]
    fn twenty_moves_from_the_start() {
        assert_eq!(pseudo_legal_moves(&position(START_FEN)).len(), 20);
    }

    #[test]
    fn knight_in_the_corner_has_two_moves() {
        assert_eq!(
            moves_from("8/8/8/8/8/8/8/N3K2k w - - 0 1", "a1"),
            ["a1b3", "a1c2"]
        );
    }

    #[test]
    fn rook_on_empty_board_has_fourteen_moves() {
        assert_eq!(moves_from("8/8/8/8/3R4/8/8/K6k w - - 0 1", "d4").len(), 14);
    }

    #[test]
    fn sliding_stops_at_pieces() {
        // The rook can capture the black pawn on d6 but not go past it,
        // and is blocked by its own pawn on d2.
        let moves = moves_from("8/8/3p4/8/3R4/8/3P4/K6k w - - 0 1", "d4");
        assert!(moves.contains(&"d4d6".to_string()));
        assert!(!moves.contains(&"d4d7".to_string()));
        assert!(!moves.contains(&"d4d2".to_string()));
    }

    #[test]
    fn pawn_pushes_and_captures() {
        let fen = "k7/8/8/8/8/2p5/3P4/K7 w - - 0 1";
        assert_eq!(moves_from(fen, "d2"), ["d2c3", "d2d3", "d2d4"]);
    }

    #[test]
    fn pawn_promotes_to_four_pieces() {
        assert_eq!(
            moves_from("k7/4P3/8/8/8/8/8/K7 w - - 0 1", "e7"),
            ["e7e8b", "e7e8n", "e7e8q", "e7e8r"]
        );
    }

    #[test]
    fn en_passant_is_generated() {
        let fen = "k7/8/8/3pP3/8/8/8/K7 w - d6 0 1";
        assert!(moves_from(fen, "e5").contains(&"e5d6".to_string()));
    }

    #[test]
    fn castling_needs_empty_squares() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K1NR w KQkq - 0 1";
        let moves = moves_from(fen, "e1");
        assert!(moves.contains(&"e1c1".to_string())); // queen side is free
        assert!(!moves.contains(&"e1g1".to_string())); // knight on g1 blocks
    }
}
