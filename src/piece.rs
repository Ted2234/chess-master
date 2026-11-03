use std::fmt;

/// The color of a piece or a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    /// Returns the other color (White -> Black, Black -> White).
    pub fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

/// The type of a chess piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

/// A chess piece: a color and a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl Piece {
    /// Creates a new piece.
    pub fn new(color: Color, kind: PieceKind) -> Self {
        Self { color, kind }
    }

    /// Returns the piece's letter: uppercase for White, lowercase for Black.
    pub fn symbol(self) -> char {
        // Step A: pick the uppercase letter based on the kind.
        let letter = match self.kind {
            PieceKind::Pawn => 'P',
            PieceKind::Knight => 'N',
            PieceKind::Bishop => 'B',
            PieceKind::Rook => 'R',
            PieceKind::Queen => 'Q',
            PieceKind::King => 'K',
        };
        // Step B: keep the letter for White, lowercase it for Black.
        match self.color {
            Color::White => letter,
            Color::Black => letter.to_ascii_lowercase(),
        }
    }
}

impl fmt::Display for Piece {
    ///Shows the piece as its letter
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}
