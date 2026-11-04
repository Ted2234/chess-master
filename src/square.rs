use std::fmt;

/// Creates a square on the board
/// 'rank' 0 = rank 1 (White's side)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square {
    pub rank: usize,
    pub file: usize,
}

impl Square {
    /// Creates a square from a rank and file. both 0 to 7
    pub fn new(rank: usize, file: usize) -> Self {
        Self { rank, file }
    }

    /// Returns true for light square
    pub fn is_light(self) -> bool {
        !(self.rank + self.file).is_multiple_of(2)
    }
}

impl fmt::Display for Square {
    /// Shows the square in chess notation e.g 'e4'
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = (b'a' + self.file as u8) as char;
        write!(f, "{file}{}", self.rank + 1)
    }
}
