use std::time::{Duration, Instant};

use crate::piece::Color;

///The time controls a player can choose from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeControl {
    FiveMinutes,
    TenMinutes,
}

impl TimeControl {
    /// Every time control, in the order they appear in the menu.
    pub const ALL: [TimeControl; 2] = [TimeControl::FiveMinutes, TimeControl::TenMinutes];

    /// The starting time for each player.
    pub fn duration(self) -> Duration {
        match self {
            TimeControl::FiveMinutes => Duration::from_secs(5 * 60),
            TimeControl::TenMinutes => Duration::from_secs(10 * 60),
        }
    }

    /// The text shown in the menu.
    pub fn label(self) -> &'static str {
        match self {
            TimeControl::FiveMinutes => "5 min",
            TimeControl::TenMinutes => "10 min",
        }
    }
}

/// A chess clock: the time left for each player.
///
/// Only one side's time runs at once: the side that has the turn.
pub struct Clock {
    white: Duration,
    black: Duration,
    running: Option<(Color, Instant)>,
}

impl Clock {
    /// Create a stopped clock with the full time for each player.
    pub fn new(time_control: TimeControl) -> Self {
        let time = time_control.duration();
        Self {
            white: time,
            black: time,
            running: None,
        }
    }

    ///Returns how much time 'color' has left.
    pub fn remaining(&self, color: Color) -> Duration {
        let saved = match color {
            Color::White => self.white,
            Color::Black => self.black,
        };
        match self.running {
            Some((running, since)) if running == color => saved.saturating_sub(since.elapsed()),
            _ => saved,
        }
    }

    /// Starts `color`'s time. If the other side's time was running, it stops.
    pub fn start(&mut self, color: Color) {
        self.stop();
        self.running = Some((color, Instant::now()));
    }

    /// Stops the clock and saves the time used so far.
    pub fn stop(&mut self) {
        if let Some((color, _)) = self.running {
            let left = self.remaining(color);
            *self.time_mut(color) = left;
            self.running = None;
        }
    }

    /// Returns true wjile one side's time is counting down.
    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// returns the side whose time has run out, if any.
    pub fn flagged(&self) -> Option<Color> {
        [Color::White, Color::Black]
            .into_iter()
            .find(|&color| self.remaining(color).is_zero())
    }

    /// Gives mutable access to one side's saved time.
    fn time_mut(&mut self, color: Color) -> &mut Duration {
        match color {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        }
    }
}

/// Formats a dration as `mm:ss`.
pub fn format_time(time: Duration) -> String {
    let seconds = time.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_shows_minutes_and_seconds() {
        assert_eq!(format_time(Duration::from_secs(300)), "05:00");
        assert_eq!(format_time(Duration::from_secs(59)), "00:59");
        assert_eq!(format_time(Duration::from_secs(61)), "01:01");
    }

    #[test]
    fn new_clock_is_stopped_with_full_time() {
        let clock = Clock::new(TimeControl::TenMinutes);
        assert!(!clock.is_running());
        assert_eq!(clock.remaining(Color::White), Duration::from_secs(600));
        assert_eq!(clock.remaining(Color::Black), Duration::from_secs(600));
        assert_eq!(clock.flagged(), None);
    }

    #[test]
    fn only_the_running_side_loses_time() {
        let mut clock = Clock::new(TimeControl::FiveMinutes);
        clock.start(Color::Black);
        std::thread::sleep(Duration::from_millis(50));
        clock.stop();
        assert!(clock.remaining(Color::Black) < Duration::from_secs(300));
        assert_eq!(clock.remaining(Color::White), Duration::from_secs(300));
    }
}
