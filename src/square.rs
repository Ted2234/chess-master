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

    /// Parses a square name like "e4". Returns `None` if it isn't a valid square
    pub fn from_name(name: &str) -> Option<Square> {
        let mut chars = name.chars();
        let file_char = chars.next()?;
        let rank_char = chars.next()?;
        if chars.next().is_some() {
            return None; // too long, like "e44"
        }
        if !('a'..='h').contains(&file_char) || !('1'..='8').contains(&rank_char) {
            return None;
        }
        let file = file_char as usize - 'a' as usize;
        let rank = rank_char as usize - '1' as usize;
        Some(Square::new(rank, file))
    }

    /// All 64 squares: a1, b1, ..., h1,  a2, ..., h8.
    pub fn all() -> impl Iterator<Item = Square> {
        (0..8).flat_map(|rank| (0..8).map(move |file| Square::new(rank, file)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for square in Square::all() {
            let name = square.to_string();
            assert_eq!(Square::from_name(&name), Some(square));
        }
    }

    #[test]
    fn bad_names_are_rejected() {
        for name in ["", "e", "i1", "a9", "a0", "e44", "E4"] {
            assert_eq!(Square::from_name(name), None, "{name} should be invalid");
        }
    }

    #[test]
    fn there_are_64_squares() {
        assert_eq!(Square::all().count(), 64);
    }
}
