use std::fmt;

use crate::piece::Piece;
use crate::square::Square;

/// An 8x8 chess board. Each square is empty, or hold a piece.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    ///Indexed as 'square[rank][file]'.
    squares: [[Option<Piece>; 8]; 8],
}

impl Board {
    /// Creates a board with no pieces.
    pub fn empty() -> Self {
        Self {
            squares: [[None; 8]; 8],
        }
    }

    /// Returns the piece on the given square, if any.
    pub fn get(&self, square: Square) -> Option<Piece> {
        self.squares[square.rank][square.file]
    }

    /// Places a piece on the given square, or clears it with 'None'.
    pub fn set(&mut self, square: Square, piece: Option<Piece>) {
        self.squares[square.rank][square.file] = piece;
    }

    ///Moves whatever is on the 'from' to 'to' replacing anything that was there.
    pub fn move_piece(&mut self, from: Square, to: Square) {
        let piece = self.get(from);
        self.set(from, None);
        self.set(to, piece);
    }
}

impl fmt::Display for Board {
    ///Draws the board as a text, rank 8 at the top, with rank and file labels.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{}", rank + 1)?;
            for file in 0..8 {
                match self.get(Square::new(rank, file)) {
                    Some(piece) => write!(f, "{piece}")?,
                    None => write!(f, ". ")?,
                }
            }
            writeln!(f)?;
        }
        write!(f, " a b c d e f g h")
    }
}
