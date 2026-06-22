# Baize

Baize is an open-source competitive UCI chess engine written in Rust.

Baize does not include a graphical user interface. Use it with a UCI-compatible
chess GUI or tournament manager.

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

Baize is available under the [MIT License](LICENSE).

[releases]: https://github.com/jyw10/Baize/releases
[wiki]: https://github.com/jyw10/Baize/wiki
