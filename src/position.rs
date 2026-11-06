use std::fmt;
use std::str::FromStr;

use crate::board::Board;
use crate::moves::Move;
use crate::piece::{Color, Piece, PieceKind};
use crate::square::Square;

/// The FEN of the standard starting position.
pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Which castling moves are still allowed (the king and that rook haven't moved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastlingRights {
    pub white_king_side: bool,
    pub white_queen_side: bool,
    pub black_king_side: bool,
    pub black_queen_side: bool,
}

impl CastlingRights {
    /// No castling allowed for anyone.
    pub const NONE: CastlingRights = CastlingRights {
        white_king_side: false,
        white_queen_side: false,
        black_king_side: false,
        black_queen_side: false,
    };

    /// The castling field of a FEN string, e.g. `KQkq`, `Kq` or `-`.
    pub fn to_fen(self) -> String {
        let mut text = String::new();
        if self.white_king_side {
            text.push('K');
        }
        if self.white_queen_side {
            text.push('Q');
        }
        if self.black_king_side {
            text.push('k');
        }
        if self.black_queen_side {
            text.push('q');
        }
        if text.is_empty() {
            text.push('-');
        }
        text
    }
}

/// A complete chess position: the board plus everything else the rules need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub board: Board,
    pub side_to_move: Color,
    pub castling: CastlingRights,
    /// The square a pawn can capture onto "en passant", right after a double pawn push.
    pub en_passant: Option<Square>,
    /// Half-moves since the last capture or pawn move (for the 50-move rule).
    pub halfmove_clock: u32,
    /// Starts at 1 and goes up by one after each Black move.
    pub fullmove_number: u32,
}

impl Position {
    /// Returns the standard starting position.
    pub fn starting() -> Self {
        START_FEN.parse().expect("START_FEN is a valid FEN")
    }

    /// Plays a move, updating the board and everything else in the position.
    ///
    /// `mv` must be a move generated for this position (see 'move.rs').
    pub fn make_move(&mut self, mv: Move) {
        let us = self.side_to_move;
        let Some(piece) = self.board.get(mv.from) else {
            return; //no piece to move.
        };
        let captured = self.board.get(mv.to);
        let is_pawn = piece.kind == PieceKind::Pawn;

        // En passasnt: the captured pawn is next to the target square, not on it.
        if is_pawn && Some(mv.to) == self.en_passant && captured.is_none() {
            self.board.set(Square::new(mv.from.rank, mv.to.file), None);
        }

        self.board.move_piece(mv.from, mv.to);

        if let Some(kind) = mv.promotion {
            self.board.set(mv.to, Some(Piece::new(us, kind)));
        }

        // Castliing: the king moves two squares, so the rook jumps over it.
        if piece.kind == PieceKind::King && mv.from.file.abs_diff(mv.to.file) == 2 {
            let (rook_from, rook_to) = if mv.to.file == 6 { (7, 5) } else { (0, 3) };
            let rank = mv.from.rank;
            self.board
                .move_piece(Square::new(rank, rook_from), Square::new(rank, rook_to));
        }

        // After a double pawn push, the square it skipped can be captured en passant.
        self.en_passant = if is_pawn && mv.from.rank.abs_diff(mv.to.rank) == 2 {
            Some(Square::new((mv.from.rank + mv.to.rank) / 2, mv.from.file))
        } else {
            None
        };

        self.update_castling_rights(mv);

        if is_pawn || captured.is_some() {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }
        if us == Color::Black {
            self.fullmove_number += 1;
        }
        self.side_to_move = us.opposite();
    }

    /// A move from or to a king's or rook's starting square removes those castling rights.
    fn update_castling_rights(&mut self, mv: Move) {
        for square in [mv.from, mv.to] {
            match (square.rank, square.file) {
                (0, 4) => {
                    self.castling.white_king_side = false;
                    self.castling.white_queen_side = false;
                }
                (0, 7) => self.castling.white_king_side = false,
                (0, 0) => self.castling.white_queen_side = false,
                (7, 4) => {
                    self.castling.black_king_side = false;
                    self.castling.black_queen_side = false;
                }
                (7, 7) => self.castling.black_king_side = false,
                (7, 0) => self.castling.black_queen_side = false,
                _ => {}
            }
        }
    }

    /// Writes the position as a FEN string.
    pub fn to_fen(&self) -> String {
        let side = match self.side_to_move {
            Color::White => "w",
            Color::Black => "b",
        };
        let en_passant = match self.en_passant {
            Some(square) => square.to_string(),
            None => "-".to_string(),
        };
        format!(
            "{} {side} {} {en_passant} {} {}",
            board_to_fen(&self.board),
            self.castling.to_fen(),
            self.halfmove_clock,
            self.fullmove_number
        )
    }
}

/// Everything that can be wrong with a FEN string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenError {
    /// A FEN has exactly 6 fields separated by spaces.
    WrongFieldCount(usize),
    /// The board part has exactly 8 ranks separated by `/`.
    WrongRankCount(usize),
    /// A rank doesn't describe exactly 8 squares.
    BadRank(String),
    /// A character that isn't a piece letter or a digit.
    BadPiece(char),
    BadSideToMove(String),
    BadCastling(String),
    BadEnPassant(String),
    BadNumber(String),
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FenError::WrongFieldCount(n) => write!(f, "expected 6 fields, found {n}"),
            FenError::WrongRankCount(n) => write!(f, "expected 8 ranks, found {n}"),
            FenError::BadRank(rank) => write!(f, "rank '{rank}' doesn't have 8 squares"),
            FenError::BadPiece(c) => write!(f, "'{c}' is not a piece"),
            FenError::BadSideToMove(text) => write!(f, "side to move must be w or b, not '{text}'"),
            FenError::BadCastling(text) => write!(f, "invalid castling rights '{text}'"),
            FenError::BadEnPassant(text) => write!(f, "invalid en passant square '{text}'"),
            FenError::BadNumber(text) => write!(f, "'{text}' is not a valid number"),
        }
    }
}

impl std::error::Error for FenError {}

impl FromStr for Position {
    type Err = FenError;

    /// Reads a position from FEN, e.g. `"8/8/8/8/8/8/8/K6k w - - 0 1".parse()`.
    fn from_str(fen: &str) -> Result<Self, Self::Err> {
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(FenError::WrongFieldCount(fields.len()));
        }

        Ok(Position {
            board: parse_board(fields[0])?,
            side_to_move: parse_side_to_move(fields[1])?,
            castling: parse_castling(fields[2])?,
            en_passant: parse_en_passant(fields[3])?,
            halfmove_clock: parse_number(fields[4])?,
            fullmove_number: parse_number(fields[5])?,
        })
    }
}

/// Reads the board part, e.g. `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR`.
fn parse_board(text: &str) -> Result<Board, FenError> {
    let rows: Vec<&str> = text.split('/').collect();
    if rows.len() != 8 {
        return Err(FenError::WrongRankCount(rows.len()));
    }

    let mut board = Board::empty();
    // FEN lists rank 8 first, so row 0 is rank index 7.
    for (row, rank_text) in rows.iter().enumerate() {
        let rank = 7 - row;
        let mut file = 0;
        for c in rank_text.chars() {
            if let Some(empty_squares) = c.to_digit(10) {
                file += empty_squares as usize;
            } else {
                let piece = Piece::from_symbol(c).ok_or(FenError::BadPiece(c))?;
                if file >= 8 {
                    return Err(FenError::BadRank(rank_text.to_string()));
                }
                board.set(Square::new(rank, file), Some(piece));
                file += 1;
            }
        }
        if file != 8 {
            return Err(FenError::BadRank(rank_text.to_string()));
        }
    }
    Ok(board)
}

fn parse_side_to_move(text: &str) -> Result<Color, FenError> {
    match text {
        "w" => Ok(Color::White),
        "b" => Ok(Color::Black),
        _ => Err(FenError::BadSideToMove(text.to_string())),
    }
}

fn parse_castling(text: &str) -> Result<CastlingRights, FenError> {
    let mut rights = CastlingRights::NONE;
    if text == "-" {
        return Ok(rights);
    }
    for c in text.chars() {
        match c {
            'K' => rights.white_king_side = true,
            'Q' => rights.white_queen_side = true,
            'k' => rights.black_king_side = true,
            'q' => rights.black_queen_side = true,
            _ => return Err(FenError::BadCastling(text.to_string())),
        }
    }
    Ok(rights)
}

fn parse_en_passant(text: &str) -> Result<Option<Square>, FenError> {
    if text == "-" {
        return Ok(None);
    }
    match Square::from_name(text) {
        Some(square) => Ok(Some(square)),
        None => Err(FenError::BadEnPassant(text.to_string())),
    }
}

fn parse_number(text: &str) -> Result<u32, FenError> {
    text.parse()
        .map_err(|_| FenError::BadNumber(text.to_string()))
}

/// Writes the board part of a FEN string.
fn board_to_fen(board: &Board) -> String {
    let mut fen = String::new();
    for rank in (0..8).rev() {
        let mut empty_squares = 0;
        for file in 0..8 {
            match board.get(Square::new(rank, file)) {
                Some(piece) => {
                    if empty_squares > 0 {
                        fen.push_str(&empty_squares.to_string());
                        empty_squares = 0;
                    }
                    fen.push(piece.symbol());
                }
                None => empty_squares += 1,
            }
        }
        if empty_squares > 0 {
            fen.push_str(&empty_squares.to_string());
        }
        if rank > 0 {
            fen.push('/');
        }
    }
    fen
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A famous test position with castling, pins and en passant chances.
    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

    #[test]
    fn fen_round_trips() {
        let fens = [
            START_FEN,
            KIWIPETE,
            "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2",
            "8/8/8/8/8/8/8/K6k b - - 42 99",
        ];
        for fen in fens {
            let position: Position = fen.parse().unwrap();
            assert_eq!(position.to_fen(), fen);
        }
    }

    #[test]
    fn starting_position_is_correct() {
        let position = Position::starting();
        assert_eq!(position.side_to_move, Color::White);
        assert_eq!(
            position.board.get(Square::new(0, 4)),
            Some(Piece::new(Color::White, PieceKind::King))
        );
        assert_eq!(
            position.board.get(Square::new(6, 0)),
            Some(Piece::new(Color::Black, PieceKind::Pawn))
        );
        assert_eq!(position.board.get(Square::new(3, 3)), None);
        assert!(position.castling.white_queen_side);
        assert_eq!(position.en_passant, None);
    }

    /// Plays moves given in UCI style ("e2e4", "e7e8q") and returns the FEN.
    fn fen_after(start: &str, moves: &[&str]) -> String {
        let mut position: Position = start.parse().unwrap();
        for text in moves {
            let from = Square::from_name(&text[0..2]).unwrap();
            let to = Square::from_name(&text[2..4]).unwrap();
            let promotion = text
                .chars()
                .nth(4)
                .and_then(Piece::from_symbol)
                .map(|piece| piece.kind);
            position.make_move(Move {
                from,
                to,
                promotion,
            });
        }
        position.to_fen()
    }

    #[test]
    fn double_push_sets_en_passant_square() {
        assert_eq!(
            fen_after(START_FEN, &["e2e4"]),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        );
    }

    #[test]
    fn counters_update() {
        // Knight moves don't reset the half-move clock; Black's move ends move 1.
        assert_eq!(
            fen_after(START_FEN, &["g1f3", "g8f6"]),
            "rnbqkb1r/pppppppp/5n2/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 2 2"
        );
    }

    #[test]
    fn en_passant_removes_the_pawn() {
        let start = "k7/8/8/3pP3/8/8/8/K7 w - d6 0 1";
        assert_eq!(fen_after(start, &["e5d6"]), "k7/8/3P4/8/8/8/8/K7 b - - 0 1");
    }

    #[test]
    fn castling_moves_the_rook() {
        let start = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        assert_eq!(
            fen_after(start, &["e1g1"]),
            "r3k2r/8/8/8/8/8/8/R4RK1 b kq - 1 1"
        );
        assert_eq!(
            fen_after(start, &["e1g1", "e8c8"]),
            "2kr3r/8/8/8/8/8/8/R4RK1 w - - 2 2"
        );
    }

    #[test]
    fn moving_a_rook_loses_that_castling_right() {
        let start = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        assert_eq!(
            fen_after(start, &["h1h5"]),
            "r3k2r/8/8/7R/8/8/8/R3K3 b Qkq - 1 1"
        );
    }

    #[test]
    fn promotion_places_the_new_piece() {
        let start = "k7/4P3/8/8/8/8/8/K7 w - - 0 1";
        assert_eq!(
            fen_after(start, &["e7e8n"]),
            "k3N3/8/8/8/8/8/8/K7 b - - 0 1"
        );
    }
    #[test]
    fn bad_fens_give_the_right_error() {
        let cases = [
            ("8/8/8/8/8/8/8/8 w - -", FenError::WrongFieldCount(4)),
            ("8/8/8/8/8/8/8 w - - 0 1", FenError::WrongRankCount(7)),
            (
                "8/8/8/8/8/8/8/7 w - - 0 1",
                FenError::BadRank("7".to_string()),
            ),
            (
                "8/8/8/8/8/8/8/ppppppppp w - - 0 1",
                FenError::BadRank("ppppppppp".to_string()),
            ),
            ("8/8/8/8/8/8/8/x7 w - - 0 1", FenError::BadPiece('x')),
            (
                "8/8/8/8/8/8/8/8 x - - 0 1",
                FenError::BadSideToMove("x".to_string()),
            ),
            (
                "8/8/8/8/8/8/8/8 w KX - 0 1",
                FenError::BadCastling("KX".to_string()),
            ),
            (
                "8/8/8/8/8/8/8/8 w - z9 0 1",
                FenError::BadEnPassant("z9".to_string()),
            ),
            (
                "8/8/8/8/8/8/8/8 w - - x 1",
                FenError::BadNumber("x".to_string()),
            ),
        ];
        for (fen, expected) in cases {
            assert_eq!(fen.parse::<Position>(), Err(expected), "FEN: {fen}");
        }
    }
}
