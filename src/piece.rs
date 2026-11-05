use std::fmt;

/// The color of a piece or a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    /// Both colors, White first.
    pub const ALL: [Color; 2] = [Color::White, Color::Black];

    /// Returns the other color (White -> Black, Black -> White).
    pub fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

/// The type of a chess piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    ///All six kind of pieces.
    pub const ALL: [PieceKind; 6] = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];
}

/// A chess piece: a color and a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl Piece {
    /// Creates a new piece.
    pub fn new(color: Color, kind: PieceKind) -> Self {
        Self { color, kind }
    }

    /// Reads a piece from it's letter (the reverse of a symbol): `'N'` is a white Knight
    /// `'q'` is a black Queen. Returns `None` for any other character.
    pub fn from_symbol(symbol: char) -> Option<Piece> {
        let kind = match symbol.to_ascii_uppercase() {
            'P' => PieceKind::Pawn,
            'N' => PieceKind::Knight,
            'B' => PieceKind::Bishop,
            'R' => PieceKind::Rook,
            'Q' => PieceKind::Queen,
            'K' => PieceKind::King,
            _ => return None,
        };
        let color = if symbol.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        Some(Piece::new(color, kind))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_round_trip() {
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                let piece = Piece::new(color, kind);
                assert_eq!(Piece::from_symbol(piece.symbol()), Some(piece));
            }
        }
    }

    #[test]
    fn unknown_symbols_are_rejected() {
        assert_eq!(Piece::from_symbol('x'), None);
        assert_eq!(Piece::from_symbol('1'), None);
    }
}
