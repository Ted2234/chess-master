mod piece;

use piece::Color;

fn main() {
    let me = Color::White;
    println!("I play {:?}", me);
    println!("Opponent plays {:?}", me.opposite());
}
