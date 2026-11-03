mod board;
mod piece;

use board::Board;

fn main() {
    let board = Board::starting_position();
    println!("{board}");
}
