use crate::piece::{Color, Piece, PieceKind};

/// The order of pieces on the back rank, from file a to file h.
const BACK_RANK: [PieceKind; 8] = [
    PieceKind::Rook,
    PieceKind::Knight,
    PieceKind::Bishop,
    PieceKind::Queen,
    PieceKind::King,
    PieceKind::Bishop,
    PieceKind::Knight,
    PieceKind::Rook,
];

/// An 8x8 chess board. Each square is empty (`None`) or holds a piece.
///
/// Indexed as `squares[rank][file]`:
/// rank 0 = rank 1 (White's side), file 0 = file a.
pub struct Board {
    squares: [[Option<Piece>; 8]; 8],
}

impl Board {
    /// Creates a board with no pieces on it.
    pub fn empty() -> Self {
        Self {
            squares: [[None; 8]; 8],
        }
    }

    /// Creates a board set up in the standard starting position.
    pub fn starting_position() -> Self {
        let mut board = Self::empty();

        for (file, kind) in BACK_RANK.iter().enumerate() {
            board.set(0, file, Some(Piece::new(Color::White, *kind)));
            board.set(7, file, Some(Piece::new(Color::Black, *kind)));
        }

        for file in 0..8 {
            board.set(1, file, Some(Piece::new(Color::White, PieceKind::Pawn)));
            board.set(6, file, Some(Piece::new(Color::Black, PieceKind::Pawn)));
        }

        board
    }

    /// Returns the piece on the given square, if any.
    pub fn get(&self, rank: usize, file: usize) -> Option<Piece> {
        self.squares[rank][file]
    }

    /// Places a piece on the given square, or clears it with `None`.
    pub fn set(&mut self, rank: usize, file: usize, piece: Option<Piece>) {
        self.squares[rank][file] = piece;
    }

    /// Prints the board with rank 8 at the top. Empty squares are shown as `.`.
    pub fn print(&self) {
        for rank in (0..8).rev() {
            for file in 0..8 {
                let c = match self.get(rank, file) {
                    Some(piece) => piece.symbol(),
                    None => '.',
                };
                print!("{c} ");
            }
            println!();
        }
    }
}
