# chess-master ♟️

A chess app written in **Rust**, built as a learning project. You play against the **Stockfish** engine and learn from its analysis of every move.

> **Status:** early development (learning project)

## Features (planned)

- Chess rules written from scratch: board, move generation, check, checkmate, stalemate
- Play against Stockfish in the terminal
- Adjustable difficulty
- Move analysis: evaluation after each move, blunder and mistake detection
- Hints on request
- Graphical interface (later)

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (stable, installed via `rustup`)
- [Stockfish](https://stockfishchess.org/download/) installed and on your `PATH`

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

## Project Structure (planned)

```
src/
├── main.rs      # entry point
├── piece.rs     # pieces and colors
├── board.rs     # board representation and FEN
├── moves.rs     # move generation and legality
├── game.rs      # game state and rules
├── engine.rs    # Stockfish (UCI) communication
└── ui/          # terminal UI, then GUI
```

## Learning Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Chess Programming Wiki](https://www.chessprogramming.org/)
- [UCI Protocol](https://backscattering.de/chess/uci/)
