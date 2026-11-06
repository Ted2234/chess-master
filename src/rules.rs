use crate::moves::{self, BISHOP_DIRECTIONS, KING_STEPS, KNIGHT_JUMPS, Move, ROOK_DIRECTIONS};
use crate::piece::{Color, Piece, PieceKind};
use crate::position::Position;
use crate::square::Square;

/// How the game stands after a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Ongoing,
    Checkmate { winner: Color },
    Stalemate,
    Draw(DrawReason),
}

/// Why a game ended in a draw (besides Stalemate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawReason {
    /// 50 moves by each side without a capture or a pawn move.
    FiftyMoveRule,
    /// Neither side has enough pieces left to ever Checkmate.
    InsufficientMaterial,
    /// The same position appears three times.
    ThreefoldRepetition,
}

/// Every legal move for the side to move.
pub fn legal_moves(position: &Position) -> Vec<Move> {
    moves::pseudo_legal_moves(position)
        .into_iter()
        .filter(|&mv| is_legal(position, mv))
        .collect()
}

/// A move is legal if your own king isn't in check after it.
/// Castling has two extra rules: not out of check, and not through an attacked square.
fn is_legal(position: &Position, mv: Move) -> bool {
    let us = position.side_to_move;
    let them = us.opposite();

    if is_castling(position, mv) {
        let passed_square = Square::new(mv.from.rank, (mv.from.file + mv.to.file) / 2);
        if is_square_attacked(position, mv.from, them)
            || is_square_attacked(position, passed_square, them)
        {
            return false;
        }
    }

    // Try the move on a copy, then look at our king.
    let mut after = position.clone();
    after.make_move(mv);
    !is_in_check(&after, us)
}

/// True for a king move of two squares.
fn is_castling(position: &Position, mv: Move) -> bool {
    let is_king = position
        .board
        .get(mv.from)
        .is_some_and(|piece| piece.kind == PieceKind::King);
    is_king && mv.from.file.abs_diff(mv.to.file) == 2
}

/// True if any piece of color 'by' could capture on 'square'.
///
/// Instead of generatingg all enemy moves, wee look outward *from* the square: "if a king stood
/// here, could it reach an enemy knight?" and so on.
pub fn is_square_attacked(position: &Position, square: Square, by: Color) -> bool {
    let attacker_on = |from: Option<Square>, kind: PieceKind| {
        from.is_some_and(|from| position.board.get(from) == Some(Piece::new(by, kind)))
    };

    // A white pawn attacks upwards, so it would stand  one rank *below* the square.
    let pawn_rank_delta = match by {
        Color::White => -1,
        Color::Black => 1,
    };
    let pawn = [-1, 1]
        .into_iter()
        .any(|file_delta| attacker_on(square.offset(pawn_rank_delta, file_delta), PieceKind::Pawn));
    let knight = KNIGHT_JUMPS
        .iter()
        .any(|&(dr, df)| attacker_on(square.offset(dr, df), PieceKind::Knight));
    let king = KING_STEPS
        .iter()
        .any(|&(dr, df)| attacker_on(square.offset(dr, df), PieceKind::King));

    pawn || knight
        || king
        || slider_attacks(position, square, by, &ROOK_DIRECTIONS, PieceKind::Rook)
        || slider_attacks(position, square, by, &BISHOP_DIRECTIONS, PieceKind::Bishop)
}

/// Looks along each direction for the first piece. It attacks the square if it's on an enemy 'kind'
/// (rook or bishop) or an enemy queen.
fn slider_attacks(
    position: &Position,
    square: Square,
    by: Color,
    directions: &[(isize, isize)],
    kind: PieceKind,
) -> bool {
    for &(rank_delta, file_delta) in directions {
        let mut current = square;
        while let Some(next) = current.offset(rank_delta, file_delta) {
            if let Some(piece) = position.board.get(next) {
                if piece.color == by && (piece.kind == kind || piece.kind == PieceKind::Queen) {
                    return true;
                }
                break;
            }
            current = next;
        }
    }
    false
}

/// Where the king of 'color' stands.
pub fn king_square(position: &Position, color: Color) -> Option<Square> {
    let king = Some(Piece::new(color, PieceKind::King));
    Square::all().find(|&square| position.board.get(square) == king)
}

/// True if the king of 'color' is attacked.
pub fn is_in_check(position: &Position, color: Color) -> bool {
    king_square(position, color)
        .is_some_and(|king| is_square_attacked(position, king, color.opposite()))
}

/// Checkmate, stalemate, a draw, or still going.
pub fn status(position: &Position) -> GameStatus {
    let us = position.side_to_move;
    if legal_moves(position).is_empty() {
        return if is_in_check(position, us) {
            GameStatus::Checkmate {
                winner: us.opposite(),
            }
        } else {
            GameStatus::Stalemate
        };
    }
    if position.halfmove_clock >= 100 {
        return GameStatus::Draw(DrawReason::FiftyMoveRule);
    }
    if has_insufficient_material(position) {
        return GameStatus::Draw(DrawReason::InsufficientMaterial);
    }
    GameStatus::Ongoing
}

/// True when only kings are left, plus at most one mishop or knight in total.
/// cannot checkmmate with that.
fn has_insufficient_material(position: &Position) -> bool {
    let mut minor_pieces = 0;
    for square in Square::all() {
        if let Some(piece) = position.board.get(square) {
            match piece.kind {
                PieceKind::King => {}
                PieceKind::Bishop | PieceKind::Knight => minor_pieces += 1,
                PieceKind::Pawn | PieceKind::Rook | PieceKind::Queen => return false,
            }
        }
    }
    minor_pieces <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::START_FEN;

    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    const POSITION_3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    const POSITION_4: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    const POSITION_5: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";

    fn position(fen: &str) -> Position {
        fen.parse().unwrap()
    }

    /// Counts every possible move sequence `depth` moves deep ("perft").
    ///
    /// The numbers for well-known positions are published, so comparing against
    /// them is the best way to prove move generation is correct.
    fn perft(position: &Position, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        legal_moves(position)
            .into_iter()
            .map(|mv| {
                let mut next = position.clone();
                next.make_move(mv);
                perft(&next, depth - 1)
            })
            .sum()
    }

    /// Checks perft for depths 1, 2, 3, ... against the expected counts.
    fn check_perft(fen: &str, expected: &[u64]) {
        let position = position(fen);
        for (depth, &count) in (1..).zip(expected) {
            assert_eq!(perft(&position, depth), count, "{fen} at depth {depth}");
        }
    }

    #[test]
    fn perft_start_position() {
        check_perft(START_FEN, &[20, 400, 8_902]);
    }

    #[test]
    fn perft_kiwipete() {
        check_perft(KIWIPETE, &[48, 2_039]);
    }

    #[test]
    fn perft_position_3() {
        check_perft(POSITION_3, &[14, 191, 2_812]);
    }

    #[test]
    fn perft_position_4() {
        check_perft(POSITION_4, &[6, 264, 9_467]);
    }

    #[test]
    fn perft_position_5() {
        check_perft(POSITION_5, &[44, 1_486]);
    }

    /// Deeper and slower. Run with: cargo test --release -- --ignored
    #[test]
    #[ignore]
    fn perft_deep() {
        check_perft(START_FEN, &[20, 400, 8_902, 197_281]);
        check_perft(KIWIPETE, &[48, 2_039, 97_862]);
        check_perft(POSITION_3, &[14, 191, 2_812, 43_238]);
        check_perft(POSITION_5, &[44, 1_486, 62_379]);
    }

    #[test]
    fn pinned_piece_cannot_move() {
        // The knight on e2 is pinned to the king by the rook on e8.
        let moves = legal_moves(&position("4r2k/8/8/8/8/8/4N3/4K3 w - - 0 1"));
        assert!(
            moves
                .iter()
                .all(|mv| mv.from != Square::from_name("e2").unwrap())
        );
    }

    #[test]
    fn cannot_castle_through_check() {
        // The rook on f8 attacks f1, which the king would pass through.
        let moves = legal_moves(&position("5r1k/8/8/8/8/8/8/4K2R w K - 0 1"));
        let castle = Move::new(
            Square::from_name("e1").unwrap(),
            Square::from_name("g1").unwrap(),
        );
        assert!(!moves.contains(&castle));
    }

    #[test]
    fn fools_mate_is_checkmate() {
        let fen = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
        assert_eq!(
            status(&position(fen)),
            GameStatus::Checkmate {
                winner: Color::Black
            }
        );
    }

    #[test]
    fn stalemate_is_detected() {
        assert_eq!(
            status(&position("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1")),
            GameStatus::Stalemate
        );
    }

    #[test]
    fn draws_are_detected() {
        assert_eq!(
            status(&position("8/8/4k3/8/8/3NK3/8/8 w - - 0 1")),
            GameStatus::Draw(DrawReason::InsufficientMaterial)
        );
        assert_eq!(
            status(&position("8/8/4k3/8/8/3RK3/8/8 w - - 100 80")),
            GameStatus::Draw(DrawReason::FiftyMoveRule)
        );
    }
}
