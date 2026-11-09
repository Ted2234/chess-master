# chess-master ♟️

A chess app written in **Rust**, built as a learning project. All the chess rules are written from scratch. From the main menu you can play a friend on the same computer, or play the **Stockfish** bot at a rating you choose. After a game, Stockfish reviews every move and explains what went wrong and which move was better. Every game has a **chess clock**.

> **Status:** early development (learning project)

## Features (planned)

- **Graphical interface**: click a piece and dots show where it can move
- Chess rules written from scratch, with full **illegal move** detection
- **Local 2-player** mode on the same computer
- **Main menu**: play a friend on the same computer, or the bot
- **Play vs the bot** (Stockfish) at a **rating you choose** (400–3000)
- **Game review**: after a game, every move is rated and explained (what went wrong, and the better move)
- **Chess clock** for each player: **5 min** or **10 min**
- **Settings**: four board themes, dark or light mode, move animation (saved between runs)
- Drag and drop or click to move, last-move highlight, player cards with captured pieces, an evaluation bar in the review

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (stable, installed via `rustup`)
- [Stockfish](https://stockfishchess.org/download/) installed and on your `PATH` (needed from Lab 13)

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
```

## Roadmap

The app is built step by step by following the **[lab](docs/lab/README.md)**. Every lab has tested code, an explanation of each part, checkpoints and exercises. The UI comes first, then the chess rules, then the bot.

### Part 1: Foundations
- [x] [Lab 00: Setup](docs/lab/lab-00-setup.md): Rust, cargo, git, Stockfish, LazyVim
- [x] [Lab 01: Pieces, board and the `Display` trait](docs/lab/lab-01-pieces-and-board.md)
  - [x] `Color` + `opposite()`, `PieceKind`
  - [x] `Piece` + `new()` + `symbol()`
  - [x] `Board`: 8×8 grid, `get`/`set`, starting position
  - [x] `Display` for `Piece` and `Board`, with rank and file labels

### Part 2: The window (local game, no rules yet)
- [x] [Lab 02: Your first window](docs/lab/lab-02-first-window.md): eframe, the `App` trait
- [x] [Lab 03: Draw the board](docs/lab/lab-03-draw-the-board.md): squares, colors, coordinates
- [x] [Lab 04: Draw the pieces](docs/lab/lab-04-draw-the-pieces.md): your own pixel-art pieces (Aseprite PNGs)
- [x] [Lab 05: Click to select and move](docs/lab/lab-05-click-to-move.md): `Square`, selection, turns
- [x] [Lab 06: Side panel](docs/lab/lab-06-side-panel.md): turn, New game, Flip board, move list
- [x] [Lab 07: Chess clock](docs/lab/lab-07-chess-clock.md): 5 / 10 min, countdown, loss on time
- [x] [Lab 08: Main menu](docs/lab/lab-08-main-menu.md): play a friend or the bot, time per player

### Part 3: The chess rules
- [x] [Lab 09: Position and FEN](docs/lab/lab-09-position-and-fen.md): side to move, castling, en passant, FEN
- [x] [Lab 10: Piece movement](docs/lab/lab-10-piece-movement.md): how each piece moves, **dots on the available moves**, castling, en passant
- [x] [Lab 11: Legal moves](docs/lab/lab-11-legal-moves.md): no illegal moves, check, checkmate, stalemate, draws, promotion picker, perft tests
- [x] [Lab 12: Game state](docs/lab/lab-12-game-state.md): history, undo, resign, repetition, `Nf3`-style move list

### Part 4: The bot
- [x] [Lab 13: Talk to Stockfish](docs/lab/lab-13-stockfish.md): UCI, background thread, hints
- [x] [Lab 14: Play vs the bot](docs/lab/lab-14-play-vs-bot.md): choose your color and the **bot's rating** (400–3000)
- [x] [Lab 15: Game analysis](docs/lab/lab-15-bot-analysis.md): every move rated, **what went wrong and the better move**, summary and advice

### Part 5: Make it look good
- [ ] [Lab 16: Make it look good](docs/lab/lab-16-make-it-look-good.md): board themes, a **Settings screen**, dark and light mode, a new look
  - [ ] Fix the board painting, the capture rings and the typos
  - [ ] Board themes and a Settings screen
  - [ ] Dark and light mode with one accent color
  - [ ] Save the settings to a file
  - [ ] Last move, check glow, rounded board, sharp pixels
  - [ ] Side panel with player cards
  - [ ] Main menu as cards, with a Random color
  - [ ] Slide animation and the "Animate moves" setting
  - [ ] Drag and drop
  - [ ] Review: eval bar, badges and cards

### Part 6: Ship it
- [ ] [Lab 17: Bundle Stockfish](docs/lab/lab-17-bundle-stockfish.md): Stockfish next to the app, no console windows, version number, Linux `.tar.gz`
- [ ] Lab 18: Releases with GitHub Actions: Linux and Windows downloads for every version tag
- [ ] Lab 19: Android: APK with Stockfish inside, portrait layout
- [ ] Lab 20: "Update available" notice: the desktop app checks GitHub Releases at startup

## Project Structure (planned)

```
assets/pieces/       # 32x32 pixel-art piece images (drawn in Aseprite)
docs/lab/            # the step-by-step lab
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
