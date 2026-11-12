use crate::moves::Move;
use crate::piece::{Color, Piece, PieceKind};
use crate::position::Position;
use crate::rules;
use crate::square::Square;

/// Writes a move in Standard Algebraic Notation, the way chess players write moves:
/// `e4`, `Nf3`, `exd5`, `O-O`, `e8=Q`, `Qh4#`.
///
/// `position` is the position *before* the move, and `mv` must be legal in it.
pub fn san(position: &Position, mv: Move) -> String {
    let Some(piece) = position.board.get(mv.from) else {
        return mv.to_string(); // not a real move here; fall back to UCI.
    };

    let mut text = if piece.kind == PieceKind::King && mv.from.file.abs_diff(mv.to.file) == 2 {
        if mv.to.file == 6 {
            "O-O".to_string()
        } else {
            "O-O-O".to_string()
        }
    } else {
        piece_move_text(position, mv, piece)
    };

    // A "+" for check, or "#" for checkmate.
    let mut after = position.clone();
    after.make_move(mv);
    if rules::is_in_check(&after, after.side_to_move) {
        let is_mate = rules::legal_moves(&after).is_empty();
        text.push(if is_mate { '#' } else { '+' });
    }
    text
}

/// Everything except castling: piece letter, disambiguation, capture, square, promotion.
fn piece_move_text(position: &Position, mv: Move, piece: Piece) -> String {
    let is_pawn = piece.kind == PieceKind::Pawn;
    let is_capture =
        position.board.get(mv.to).is_some() || (is_pawn && Some(mv.to) == position.en_passant);

    let mut text = String::new();
    if is_pawn {
        // Pawn captures name the file the pawn came from: edx5.
        if is_capture {
            text.push(mv.from.file_char());
        }
    } else {
        text.push(letter(piece.kind));
        text.push_str(&disambiguation(position, mv, piece.kind));
    }
    if is_capture {
        text.push('x');
    }
    text.push_str(&mv.to.to_string());
    if let Some(kind) = mv.promotion {
        text.push('=');
        text.push(letter(kind));
    }
    text
}

/// The uppercase letter of a piece kind: K, Q, R, B, N (P for pawns, though SAN omits it).
fn letter(kind: PieceKind) -> char {
    Piece::new(Color::White, kind).symbol()
}

/// When two pieces of the same kind can move to the same square, SAN adds the file they come from
/// (`Nbd2`), or the rank if the files are equal (`R1e2`), or both if neither alone is enough (`Qh4e1`).
fn disambiguation(position: &Position, mv: Move, kind: PieceKind) -> String {
    let rivals: Vec<Square> = rules::legal_moves(position)
        .into_iter()
        .filter(|other| other.to == mv.to && other.from != mv.from)
        .filter(|other| position.board.get(other.from).map(|p| p.kind) == Some(kind))
        .map(|other| other.from)
        .collect();

    if rivals.is_empty() {
        String::new()
    } else if rivals.iter().all(|rival| rival.file != mv.from.file) {
        mv.from.file_char().to_string()
    } else if rivals.iter().all(|rival| rival.rank != mv.from.rank) {
        mv.from.rank_char().to_string()
    } else {
        mv.from.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::START_FEN;

    /// The SAN of a move given in UCI style, in the position given as FEN.
    fn san_of(fen: &str, uci: &str) -> String {
        let position: Position = fen.parse().unwrap();
        let mv = rules::legal_moves(&position)
            .into_iter()
            .find(|mv| mv.to_string() == uci)
            .unwrap();
        san(&position, mv)
    }

    #[test]
    fn simple_moves() {
        assert_eq!(san_of(START_FEN, "e2e4"), "e4");
        assert_eq!(san_of(START_FEN, "g1f3"), "Nf3");
    }

    #[test]
    fn captures() {
        let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2";
        assert_eq!(san_of(fen, "e4d5"), "exd5");
        assert_eq!(san_of("7k/8/8/3p4/8/8/8/K2Q4 w - - 0 1", "d1d5"), "Qxd5");
    }

    #[test]
    fn castling() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        assert_eq!(san_of(fen, "e1g1"), "O-O");
        assert_eq!(san_of(fen, "e1c1"), "O-O-O");
    }

    #[test]
    fn promotion_and_check() {
        assert_eq!(san_of("8/4P3/8/8/8/8/8/K5k1 w - - 0 1", "e7e8q"), "e8=Q");
        assert_eq!(san_of("8/4P3/8/8/8/8/8/K6k w - - 0 1", "e7e8r"), "e8=R");
        assert_eq!(san_of("7k/4P3/8/8/8/8/8/K7 w - - 0 1", "e7e8q"), "e8=Q+");
    }

    #[test]
    fn checkmate_gets_a_hash() {
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 2";
        assert_eq!(san_of(fen, "d8h4"), "Qh4#");
    }

    #[test]
    fn disambiguation() {
        // Knights on b1 and f3 can both go to d2.
        let fen = "k7/8/8/8/8/5N2/8/KN6 w - - 0 1";
        assert_eq!(san_of(fen, "b1d2"), "Nbd2");
        // Rooks on a1 and a5 can both go to a3.
        let fen = "7k/8/8/R7/8/8/8/R6K w - - 0 1";
        assert_eq!(san_of(fen, "a1a3"), "R1a3");
    }
}
