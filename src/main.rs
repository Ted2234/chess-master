mod piece;

use piece::{Color, Piece, PieceKind};

fn main() {
    let me = Color::White;
    println!("I play {me:?}");
    println!("Opponent plays {:?}", me.opposite());

    let knight = Piece::new(Color::White, PieceKind::Knight);
    let queen = Piece::new(Color::Black, PieceKind::Queen);

    println!("{knight:?}  {}", knight.symbol());
    println!("{queen:?}  {}", queen.symbol());
}
