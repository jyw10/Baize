# Baize

Baize is an open-source competitive UCI chess engine written in Rust. The
current development engine identifies itself as `Baize-v0.3.0`.

Baize does not include a graphical user interface. Use it with a UCI-compatible
chess GUI or tournament manager.

## Features

- Complete standard-chess move generation and game-state handling
- Fail-soft negamax with alpha-beta pruning
- Iterative deepening with depth, node, clock, and move-time limits
- MVV-LVA capture move ordering
- Quiescence search over captures, promotions, and check evasions
- Material evaluation
- Asynchronous UCI `go`, `stop`, and `quit` handling

Baize is under active development. It does not yet include a transposition
table or additional pruning heuristics.

## Getting started

Precompiled builds are available from the [GitHub Releases page][releases]. To
build the current source, install the stable Rust toolchain and run:

```console
cargo build --release
```

The executable is written to `target/release/baize` on Unix-like systems and
`target/release/baize.exe` on Windows. Launch it through a UCI-compatible GUI,
or run it directly and communicate using UCI commands.

## Documentation

The [project wiki][wiki] is the home for architecture notes, the complete UCI
command reference, platform-specific build guidance, and reproducible engine
testing methodology and results.

## Development

```console
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release
```

Engine changes are measured against a fixed baseline with paired SPRT matches.

## License

Baize is available under either the [MIT License](LICENSE-MIT) or
[Apache License 2.0](LICENSE-APACHE), at your option.

[releases]: https://github.com/jyw10/Baize/releases
[wiki]: https://github.com/jyw10/Baize/wiki
