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

    /// returns the square 'rank_delta' rnks up and 'file_delta' files to the fight, or None is that
    /// would be off the board.
    pub fn offset(self, rank_delta: isize, file_delta: isize) -> Option<Square> {
        let rank = self.rank as isize + rank_delta;
        let file = self.file as isize + file_delta;
        if (0..8).contains(&rank) && (0..8).contains(&file) {
            Some(Square::new(rank as usize, file as usize))
        } else {
            None
        }
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
    fn offset_stays_on_the_board() {
        let e4 = Square::from_name("e4").unwrap();
        assert_eq!(e4.offset(1, 1), Square::from_name("f5"));
        assert_eq!(e4.offset(-3, -4), Square::from_name("a1"));
        assert_eq!(e4.offset(5, 0), None);
        assert_eq!(e4.offset(0, -5), None);
    }

    #[test]
    fn there_are_64_squares() {
        assert_eq!(Square::all().count(), 64);
    }
}
