use std::{
    fmt,
    io::{self, BufRead, Write},
    sync::{Arc, Mutex, atomic::AtomicBool},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    Board, Color, Move, MoveKind, PieceType, Square,
    search::{DEFAULT_HASH_MB, MAX_HASH_MB, MIN_HASH_MB, SearchInfo, SearchLimits, iterative_deepening, mate_moves},
    time::{Clock, allocate_time, fixed_move_time},
};

pub fn run_stdio() {
    let stdin = io::stdin();
    let output = Arc::new(Mutex::new(io::stdout()));
    let mut engine = UciEngine::new(output);

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if !engine.handle_command(line.trim()) {
            return;
        }
    }
    engine.stop_search();
}

struct SearchHandle {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

struct UciEngine<W: Write + Send + 'static> {
    board: Board,
    hash_size_mb: usize,
    output: Arc<Mutex<W>>,
    search: Option<SearchHandle>,
}

impl<W: Write + Send + 'static> UciEngine<W> {
    fn new(output: Arc<Mutex<W>>) -> Self {
        Self {
            board: Board::default(),
            hash_size_mb: DEFAULT_HASH_MB,
            output,
            search: None,
        }
    }

    fn handle_command(&mut self, command: &str) -> bool {
        let Some(name) = command.split_ascii_whitespace().next() else {
            return true;
        };
        match name {
            "uci" => {
                emit(
                    &self.output,
                    format_args!("id name Baize-v{}", env!("CARGO_PKG_VERSION")),
                );
                emit(&self.output, format_args!("id author Baize contributors"));
                emit(
                    &self.output,
                    format_args!(
                        "option name Hash type spin default {DEFAULT_HASH_MB} min {MIN_HASH_MB} max {MAX_HASH_MB}"
                    ),
                );
                emit(&self.output, format_args!("uciok"));
            }
            "isready" => emit(&self.output, format_args!("readyok")),
            "ucinewgame" => {
                self.stop_search();
                self.board = Board::default();
            }
            "position" => {
                self.stop_search();
                if let Err(error) = self.set_position(command) {
                    emit(&self.output, format_args!("info string {error}"));
                }
            }
            "go" => self.start_search(GoParams::parse(command), self.board.clone()),
            "stop" => self.stop_search(),
            "quit" => {
                self.stop_search();
                return false;
            }
            "setoption" => {
                self.stop_search();
                self.set_option(command);
            }
            "debug" | "register" | "ponderhit" => {}
            _ => {}
        }
        true
    }

    fn set_option(&mut self, command: &str) {
        let tokens = command.split_ascii_whitespace().skip(1).collect::<Vec<_>>();
        let Some(value_index) = tokens.iter().position(|&token| token.eq_ignore_ascii_case("value")) else {
            return;
        };
        if tokens.first().is_none_or(|token| !token.eq_ignore_ascii_case("name")) {
            return;
        }
        let name = tokens[1..value_index].join(" ");
        let value = tokens.get(value_index + 1).copied();
        if name.eq_ignore_ascii_case("Hash")
            && let Some(size) = value.and_then(|text| text.parse::<usize>().ok())
        {
            self.hash_size_mb = size.clamp(MIN_HASH_MB, MAX_HASH_MB);
        }
    }

    fn set_position(&mut self, command: &str) -> Result<(), UciError> {
        let tokens = command.split_ascii_whitespace().skip(1).collect::<Vec<_>>();
        let (mut board, mut cursor) = match tokens.first().copied() {
            Some("startpos") => (Board::default(), 1),
            Some("fen") => {
                if tokens.len() < 7 {
                    return Err(UciError("position fen requires all six FEN fields".to_owned()));
                }
                (
                    Board::from_fen(&tokens[1..7].join(" ")).map_err(|error| UciError(error.to_string()))?,
                    7,
                )
            }
            _ => return Err(UciError("position requires 'startpos' or 'fen'".to_owned())),
        };

        if tokens.get(cursor).copied() == Some("moves") {
            cursor += 1;
            for text in &tokens[cursor..] {
                let mv = parse_uci_move(&board, text)
                    .ok_or_else(|| UciError(format!("illegal move '{text}' in position command")))?;
                board.make_move(mv).map_err(|error| UciError(error.to_string()))?;
            }
        } else if cursor != tokens.len() {
            return Err(UciError("unexpected token in position command".to_owned()));
        }

        self.board = board;
        Ok(())
    }

    fn start_search(&mut self, params: GoParams, board: Board) {
        self.stop_search();
        let mut limits = params.limits(board.side_to_move());
        limits.hash_size_mb = self.hash_size_mb;
        let stop = Arc::new(AtomicBool::new(false));
        let search_stop = Arc::clone(&stop);
        let output = Arc::clone(&self.output);
        let thread = thread::spawn(move || {
            let info_output = Arc::clone(&output);
            let outcome = iterative_deepening(&board, limits, &search_stop, move |info| {
                emit_search_info(&info_output, &info);
            });
            let bestmove = outcome.best_move.map_or_else(|| "0000".to_owned(), format_uci_move);
            emit(&output, format_args!("bestmove {bestmove}"));
        });
        self.search = Some(SearchHandle { stop, thread });
    }

    fn stop_search(&mut self) {
        if let Some(search) = self.search.take() {
            search.stop.store(true, std::sync::atomic::Ordering::Relaxed);
            let _ = search.thread.join();
        }
    }
}

fn emit<W: Write>(output: &Arc<Mutex<W>>, arguments: fmt::Arguments<'_>) {
    let mut output = output.lock().expect("UCI output mutex poisoned");
    let _ = writeln!(output, "{arguments}");
    let _ = output.flush();
}

fn emit_search_info<W: Write>(output: &Arc<Mutex<W>>, info: &SearchInfo) {
    let millis = info.elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
    let nps = if millis == 0 {
        info.nodes.saturating_mul(1_000)
    } else {
        info.nodes.saturating_mul(1_000) / millis
    };
    let score = mate_moves(info.score).map_or_else(|| format!("cp {}", info.score), |moves| format!("mate {moves}"));
    let pv = info
        .pv
        .iter()
        .copied()
        .map(format_uci_move)
        .collect::<Vec<_>>()
        .join(" ");
    emit(
        output,
        format_args!(
            "info depth {} score {} nodes {} time {} nps {} pv {}",
            info.depth, score, info.nodes, millis, nps, pv
        ),
    );
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct GoParams {
    wtime: Option<Duration>,
    btime: Option<Duration>,
    winc: Duration,
    binc: Duration,
    moves_to_go: Option<u32>,
    move_time: Option<Duration>,
    depth: Option<u8>,
    nodes: Option<u64>,
    infinite: bool,
}

impl GoParams {
    fn parse(command: &str) -> Self {
        let mut params = Self::default();
        let mut tokens = command.split_ascii_whitespace().skip(1);
        while let Some(token) = tokens.next() {
            match token {
                "wtime" => params.wtime = parse_millis(tokens.next()),
                "btime" => params.btime = parse_millis(tokens.next()),
                "winc" => params.winc = parse_millis(tokens.next()).unwrap_or_default(),
                "binc" => params.binc = parse_millis(tokens.next()).unwrap_or_default(),
                "movestogo" => params.moves_to_go = tokens.next().and_then(|value| value.parse().ok()),
                "movetime" => params.move_time = parse_millis(tokens.next()),
                "depth" => {
                    params.depth = tokens
                        .next()
                        .and_then(|value| value.parse::<u16>().ok())
                        .map(|depth| depth.clamp(1, (crate::search::MAX_PLY - 1) as u16) as u8);
                }
                "nodes" => {
                    params.nodes = tokens
                        .next()
                        .and_then(|value| value.parse::<u64>().ok())
                        .map(|v| v.max(1))
                }
                "infinite" => params.infinite = true,
                _ => {}
            }
        }
        params
    }

    fn limits(self, side: Color) -> SearchLimits {
        let budget = if self.infinite {
            None
        } else if let Some(move_time) = self.move_time {
            Some(fixed_move_time(move_time))
        } else {
            let (remaining, increment) = match side {
                Color::White => (self.wtime, self.winc),
                Color::Black => (self.btime, self.binc),
            };
            remaining.map(|remaining| {
                allocate_time(Clock {
                    remaining,
                    increment,
                    moves_to_go: self.moves_to_go,
                })
            })
        };
        SearchLimits {
            depth: self.depth,
            nodes: self.nodes,
            soft_time: budget.map(|value| value.soft),
            hard_time: budget.map(|value| value.hard),
            ..SearchLimits::default()
        }
    }
}

fn parse_millis(value: Option<&str>) -> Option<Duration> {
    value?.parse::<u64>().ok().map(Duration::from_millis)
}

#[must_use]
pub fn format_uci_move(mv: Move) -> String {
    let from = mv.from();
    let to = if mv.kind() == MoveKind::Castling {
        Square::from_file_rank(if mv.to().file() > from.file() { 6 } else { 2 }, from.rank())
            .expect("castling target is on the board")
    } else {
        mv.to()
    };
    let mut text = format!("{from}{to}");
    if let Some(piece) = mv.promotion() {
        text.push(match piece {
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Rook => 'r',
            PieceType::Queen => 'q',
            PieceType::Pawn | PieceType::King => unreachable!(),
        });
    }
    text
}

#[must_use]
pub fn parse_uci_move(board: &Board, text: &str) -> Option<Move> {
    board.legal_moves().into_iter().find(|&mv| format_uci_move(mv) == text)
}

#[derive(Clone, Debug)]
struct UciError(String);

impl fmt::Display for UciError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuffer {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn position_command_applies_moves() {
        let output = Arc::new(Mutex::new(SharedBuffer::default()));
        let mut engine = UciEngine::new(output);
        engine.set_position("position startpos moves e2e4 e7e5 g1f3").unwrap();
        assert_eq!(
            engine.board.to_fen(),
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2"
        );
    }

    #[test]
    fn castling_converts_between_uci_and_internal_encoding() {
        let board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let mv = parse_uci_move(&board, "e1g1").unwrap();
        assert_eq!(mv.kind(), MoveKind::Castling);
        assert_eq!(mv.to(), Square::H1);
        assert_eq!(format_uci_move(mv), "e1g1");
    }

    #[test]
    fn go_parser_builds_expected_limits() {
        let params = GoParams::parse("go wtime 10000 btime 20000 winc 100 binc 200 movestogo 20 depth 8 nodes 999");
        let limits = params.limits(Color::White);
        assert_eq!(limits.depth, Some(8));
        assert_eq!(limits.nodes, Some(999));
        assert!(limits.soft_time.is_some());
        assert!(limits.hard_time > limits.soft_time);
        assert_eq!(limits.hash_size_mb, DEFAULT_HASH_MB);
    }

    #[test]
    fn hash_option_updates_and_clamps_the_table_size() {
        let output = Arc::new(Mutex::new(SharedBuffer::default()));
        let mut engine = UciEngine::new(output);

        assert_eq!(engine.hash_size_mb, DEFAULT_HASH_MB);
        assert!(engine.handle_command("setoption name Hash value 64"));
        assert_eq!(engine.hash_size_mb, 64);
        assert!(engine.handle_command("setoption name hash value 0"));
        assert_eq!(engine.hash_size_mb, MIN_HASH_MB);
        assert!(engine.handle_command("setoption name Hash value 999999"));
        assert_eq!(engine.hash_size_mb, MAX_HASH_MB);
        assert!(engine.handle_command("setoption name Unknown value 32"));
        assert_eq!(engine.hash_size_mb, MAX_HASH_MB);
    }

    #[test]
    fn protocol_handshake_and_search_emit_required_lines() {
        let buffer = SharedBuffer::default();
        let bytes = Arc::clone(&buffer.0);
        let output = Arc::new(Mutex::new(buffer));
        let mut engine = UciEngine::new(output);
        assert!(engine.handle_command("uci"));
        assert!(engine.handle_command("isready"));
        assert!(engine.handle_command("go depth 1"));
        engine.stop_search();

        let text = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
        assert!(text.contains("id name Baize-v0.6.0\n"));
        assert!(text.contains("option name Hash type spin default 16 min 1 max 65536\n"));
        assert!(text.contains("uciok\n"));
        assert!(text.contains("readyok\n"));
        assert!(text.contains("bestmove "));
    }
}
