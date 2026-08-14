# Donkey Run

A fast, neon-soaked arcade dodger inspired by the classic `DONKEY.BAS`, rebuilt in Rust with SDL2.

Drive into the night, dodge oncoming traffic, chase near-miss combos, and collect cassette tapes as the game becomes endlessly faster. Everything—including the pixel art, animation, and chiptune audio—is generated in code.

## Features

- Endless difficulty with a gentle logarithmic speed curve
- Randomized traffic lanes and rare lane-crossing red cars
- Three lives, near-miss combos, and persistent high scores
- Collectible cassette tapes with score bonuses and temporary music boosts
- Procedural 1980s-style sound effects and adaptive chiptune music
- Perspective-scaled sprites, animated roadside scenery, and a flickering neon skyline
- Keyboard and hot-pluggable game-controller support
- Fullscreen mode and automatic pause when the window loses focus

## Controls

| Action | Keyboard | Controller |
| --- | --- | --- |
| Steer | Left / Right arrows | D-pad or left stick |
| Start | Space | A or Start |
| Retry | R | A or Start |
| Pause / resume | P | Start or Back |
| Toggle sound | M | Y |
| Toggle fullscreen | F11 | — |
| Quit | Escape or Q | — |

Red cars cross into the opposite lane. Follow the arrow and move into the lane they just vacated. Cassette tapes appear ahead of some straight-moving cars; collect the tape, then move away before the car arrives.

## Build and run

You need a recent Rust toolchain with Rust 2024 edition support, plus CMake and a working C/C++ compiler. SDL2 is built automatically by the crate.

```sh
cargo run --release
```

Run the tests with:

```sh
cargo test
```

## High scores

The best score is saved locally at `~/.donkey-bas/high-score`.

For development or isolated runs, override that location with `DONKEY_BAS_DATA_DIR`:

```sh
DONKEY_BAS_DATA_DIR=/tmp/donkey-bas cargo run
```

## Project structure

- `src/main.rs` — SDL setup, input handling, and the main loop
- `src/game.rs` — gameplay state, movement, collisions, scoring, and difficulty
- `src/render.rs` — software-rendered pixel graphics and animation
- `src/audio.rs` — procedural sound effects and adaptive music
- `src/score.rs` — persistent high-score storage
