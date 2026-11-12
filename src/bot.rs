use std::time::Duration;

use crate::engine::SearchLimit;

/// The lowest rating you can choose for the bot.
pub const MIN_RATING: u32 = 400;
/// The highest rating you can choose for the bot.
pub const MAX_RATING: u32 = 3000;
/// Stockfish can limit itself to a rating from this value up (`UCI_Elo`).
const STOCKFISH_MIN_ELO: u32 = 1320;
/// The bot never moves faster than this, so you can see its move coming.
pub const MIN_MOVE_TIME: Duration = Duration::from_millis(800);

/// The engine settings and thinking limit that make Stockfish play at about a rating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotStrength {
    /// UCI options as (name, value) pairs.
    pub options: Vec<(String, String)>,
    pub limit: SearchLimit,
}

/// How to make Stockfish play at roughly `rating`.
///
/// From 1320 up, Stockfish has a built-in rating limit. Below that, we use its
/// weakest "Skill Level" and let it look only a few moves ahead. That's an
/// approximation: real ratings depend on time control and opponent pool.
pub fn strength(rating: u32) -> BotStrength {
    let rating = rating.clamp(MIN_RATING, MAX_RATING);
    let option = |name: &str, value: String| (name.to_string(), value);

    if rating >= STOCKFISH_MIN_ELO {
        BotStrength {
            options: vec![
                option("Skill Level", "20".to_string()),
                option("UCI_LimitStrength", "true".to_string()),
                option("UCI_Elo", rating.to_string()),
            ],
            limit: SearchLimit::Millis(500),
        }
    } else {
        // 400 -> depth 1, 650 -> 2, 900 -> 3, 1150 -> 4
        let depth = 1 + (rating - MIN_RATING) / 250;
        BotStrength {
            options: vec![
                option("UCI_LimitStrength", "false".to_string()),
                option("Skill Level", "0".to_string()),
            ],
            limit: SearchLimit::Depth(depth),
        }
    }
}

/// Settings for full engine strength, used for hints and game analysis.
pub fn full_strength_options() -> Vec<(String, String)> {
    vec![
        ("UCI_LimitStrength".to_string(), "false".to_string()),
        ("Skill Level".to_string(), "20".to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_ratings_use_uci_elo() {
        let bot = strength(1800);
        assert!(
            bot.options
                .contains(&("UCI_Elo".to_string(), "1800".to_string()))
        );
        assert_eq!(bot.limit, SearchLimit::Millis(500));
    }

    #[test]
    fn low_ratings_search_shallow() {
        assert_eq!(strength(400).limit, SearchLimit::Depth(1));
        assert_eq!(strength(1300).limit, SearchLimit::Depth(4));
        assert!(
            strength(800)
                .options
                .contains(&("Skill Level".to_string(), "0".to_string()))
        );
    }

    /// Needs Stockfish. Two bots (600 and 2000) play up to 60 moves through the
    /// engine thread; every move must be legal. Run with: cargo test -- --ignored
    #[test]
    #[ignore]
    fn bots_play_a_legal_game() {
        use crate::engine::{EngineHandle, EngineReply, EngineRequest};
        use crate::game::Game;
        use crate::moves::Move;
        use crate::piece::Color;

        let white = EngineHandle::spawn();
        let black = EngineHandle::spawn();
        white.send(EngineRequest::SetOptions(strength(600).options));
        black.send(EngineRequest::SetOptions(strength(2000).options));

        let mut game = Game::new();
        for id in 0..60 {
            if game.result().is_some() {
                break;
            }
            let (engine, rating) = match game.position().side_to_move {
                Color::White => (&white, 600),
                Color::Black => (&black, 2000),
            };
            engine.send(EngineRequest::Search {
                id,
                fen: game.position().to_fen(),
                limit: strength(rating).limit,
                min_time: Duration::ZERO,
            });
            // In the app we never wait like this; in a test it's fine.
            let reply = loop {
                if let Some(reply) = engine.try_receive() {
                    break reply;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            };
            let EngineReply::Searched { analysis, .. } = reply else {
                panic!("engine failed: {reply:?}");
            };
            let mv = Move::from_uci(&analysis.best_move).unwrap();
            game.play(mv).unwrap();
        }
        assert!(game.history().len() >= 10);
    }

    #[test]
    fn ratings_are_clamped() {
        assert_eq!(strength(0), strength(MIN_RATING));
        assert_eq!(strength(9999), strength(MAX_RATING));
    }
}
