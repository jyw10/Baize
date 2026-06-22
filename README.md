# Baize

Baize is an early-stage UCI chess engine written in Rust. Version 0.1.0 is an
intentionally minimal baseline for measuring later engine improvements.

The baseline contains:

- complete standard-chess move generation and game-state handling;
- fail-soft negamax with alpha-beta pruning;
- iterative deepening with depth, node, clock, and move-time limits;
- material-only evaluation; and
- asynchronous UCI `go`, `stop`, and `quit` handling.

It deliberately does not yet include quiescence search, a transposition table,
move ordering, or additional pruning heuristics.

## Build and run

```console
cargo build --release
target/release/baize
```

Baize communicates through UCI and is intended to be launched by a chess GUI
or tournament manager.

## Verify

```console
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release
```

## License

Baize is available under either the MIT License or Apache License 2.0, at your
option.
