# chess-master ♟️

A chess app written in **Rust**, built as a learning project. All the chess rules are written from scratch. From the main menu you can play a friend on the same computer, or play the **Stockfish** bot at a rating you choose. After a game, Stockfish reviews every move and explains what went wrong and which move was better. Every game has a **chess clock**.

> **Status:** playable on Linux (version 0.1.0). Learning project.

## Features

- **Graphical interface**: click a piece and dots show where it can move
- Chess rules written from scratch, with full **illegal move** detection
- **Local 2-player** mode on the same computer
- **Main menu**: play a friend on the same computer, or the bot
- **Play vs the bot** (Stockfish) at a **rating you choose** (400–3000)
- **Game review**: after a game, every move is rated and explained (what went wrong, and the better move)
- **Chess clock** for each player: **5 min** or **10 min**
- **Settings**: four board themes, dark or light mode, move animation (saved between runs)
- Drag and drop or click to move, last-move highlight, player cards with captured pieces, an evaluation bar in the review
- A **game-over pop-up**: who won (or a draw), how, and the score

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (stable, installed via `rustup`)
- [Stockfish](https://stockfishchess.org/download/) installed and on your `PATH`, for development (the Linux package ships its own Stockfish)

  ```sh
  # Arch / CachyOS
  sudo pacman -S stockfish
  ```

## Usage

```sh
cargo run            # build and run
cargo test           # run tests
cargo fmt            # format code
cargo clippy         # lint code
./scripts/package-linux.sh   # release build + Stockfish -> target/dist/chess-master-<version>-linux-x86_64.tar.gz
```

## Status

Built step by step in labs: the window first, then the chess rules, then the Stockfish bot, a polished look, and packaging.

- [x] Foundations: pieces, board, a window with your own pixel-art pieces
- [x] Playing: click to move, side panel, chess clock, main menu
- [x] Chess rules: FEN, piece movement with move dots, legal moves, check, checkmate, draws, promotion, perft tests, undo, move notation
- [x] The bot: Stockfish over UCI on a background thread, hints, a bot at a chosen rating, game review with explanations
- [x] The look: board themes, Settings screen, dark/light mode, player cards, drag and drop, animations, eval bar, game-over pop-up
- [x] Linux package with Stockfish bundled
- [ ] Releases with GitHub Actions: Linux and Windows downloads for every version tag
- [ ] Android: APK with Stockfish inside, portrait layout
- [ ] "Update available" notice: the desktop app checks GitHub Releases at startup

## Project Structure

```
assets/pieces/       # 32x32 pixel-art piece images (drawn in Aseprite)
scripts/             # package-linux.sh: the Linux download
src/
├── main.rs          # entry point: opens the window
├── piece.rs         # Color, PieceKind, Piece
├── board.rs         # 8x8 board
├── square.rs        # Square (rank + file)
├── clock.rs         # chess clock
├── position.rs      # side to move, castling, en passant, FEN
├── moves.rs         # Move type and move generation
├── rules.rs         # legal moves, check, checkmate, draws (+ perft tests)
├── san.rs           # move notation: e4, Nf3, O-O, e8=Q+
├── game.rs          # game state, history, undo, result
├── engine.rs        # Stockfish (UCI) on a background thread
├── bot.rs           # bot rating -> Stockfish settings
├── analysis.rs      # move ratings, explanations, summary
├── settings.rs      # board theme, dark/light, animation; saved to a file
└── ui/
    ├── app.rs            # app state, screens, side panel
    ├── board_view.rs     # drawing the board, dots, arrows, clicks and drags
    ├── menu.rs           # main menu: opponent, color, rating, time
    ├── piece_textures.rs # loads the piece images
    ├── review.rs         # game review screen
    ├── settings_screen.rs # the Settings screen
    ├── theme.rs          # board colors, accent color, dark/light style
    └── widgets.rs        # cards, buttons, a centered column
```

## Learning Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [egui](https://www.egui.rs/) · [eframe docs](https://docs.rs/eframe/)
- [Chess Programming Wiki](https://www.chessprogramming.org/)
- [UCI Protocol](https://backscattering.de/chess/uci/)

## Art

The chess pieces are 32×32 pixel art in `assets/pieces/`, drawn in [Aseprite](https://www.aseprite.org/). The starting placeholders were generated from the DejaVu Sans chess symbols, and they get replaced one by one with hand-drawn art.
