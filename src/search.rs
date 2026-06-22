use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use crate::{Board, Color, Move, MoveKind, PieceType, evaluation};

mod tt;

use tt::{Bound, Entry, TranspositionTable, score_from_tt, score_to_tt};

pub const MATE_SCORE: i32 = 30_000;
pub const MAX_PLY: usize = 128;
pub const DEFAULT_HASH_MB: usize = 16;
pub const MIN_HASH_MB: usize = 1;
pub const MAX_HASH_MB: usize = 65_536;
const INFINITY: i32 = 32_000;
const HISTORY_MAX: i32 = 16_384;

#[derive(Clone, Copy, Debug)]
pub struct SearchLimits {
    pub depth: Option<u8>,
    pub nodes: Option<u64>,
    pub soft_time: Option<Duration>,
    pub hard_time: Option<Duration>,
    pub hash_size_mb: usize,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            depth: None,
            nodes: None,
            soft_time: None,
            hard_time: None,
            hash_size_mb: DEFAULT_HASH_MB,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchInfo {
    pub depth: u8,
    pub score: i32,
    pub nodes: u64,
    pub elapsed: Duration,
    pub pv: Vec<Move>,
}

#[derive(Clone, Debug)]
pub struct SearchOutcome {
    pub best_move: Option<Move>,
    pub score: i32,
    pub completed_depth: u8,
    pub nodes: u64,
    pub elapsed: Duration,
    pub pv: Vec<Move>,
}

#[derive(Clone)]
struct PvLine {
    moves: [Move; MAX_PLY],
    len: usize,
}

impl Default for PvLine {
    fn default() -> Self {
        Self {
            moves: [Move::default(); MAX_PLY],
            len: 0,
        }
    }
}

impl PvLine {
    fn prepend(&mut self, mv: Move, child: &Self) {
        self.moves[0] = mv;
        let child_len = child.len.min(MAX_PLY - 1);
        self.moves[1..=child_len].copy_from_slice(&child.moves[..child_len]);
        self.len = child_len + 1;
    }

    fn to_vec(&self) -> Vec<Move> {
        self.moves[..self.len].to_vec()
    }
}

struct SearchContext<'a> {
    stop: &'a AtomicBool,
    start: Instant,
    hard_time: Option<Duration>,
    max_nodes: Option<u64>,
    nodes: u64,
}

impl SearchContext<'_> {
    fn enter_node(&mut self) -> Result<(), Aborted> {
        if self.stop.load(Ordering::Relaxed)
            || self.max_nodes.is_some_and(|limit| self.nodes >= limit)
            || self.hard_time.is_some_and(|limit| self.start.elapsed() >= limit)
        {
            return Err(Aborted);
        }
        self.nodes += 1;
        Ok(())
    }

    fn should_stop_between_iterations(&self, soft_time: Option<Duration>) -> bool {
        self.stop.load(Ordering::Relaxed)
            || self.max_nodes.is_some_and(|limit| self.nodes >= limit)
            || soft_time.is_some_and(|limit| self.start.elapsed() >= limit)
            || self.hard_time.is_some_and(|limit| self.start.elapsed() >= limit)
    }
}

struct SearchState<'a> {
    context: SearchContext<'a>,
    table: TranspositionTable,
    history: ButterflyHistory,
}

struct ButterflyHistory {
    scores: [[[i32; 64]; 64]; 2],
}

impl Default for ButterflyHistory {
    fn default() -> Self {
        Self {
            scores: [[[0; 64]; 64]; 2],
        }
    }
}

impl ButterflyHistory {
    fn score(&self, color: Color, mv: Move) -> i32 {
        self.scores[color.index()][mv.from().index()][mv.to().index()]
    }

    fn reward(&mut self, color: Color, mv: Move, depth: u8) {
        let depth = i32::from(depth);
        let bonus = (depth * depth).min(HISTORY_MAX);
        let entry = &mut self.scores[color.index()][mv.from().index()][mv.to().index()];
        *entry += bonus - *entry * bonus / HISTORY_MAX;
    }
}

#[derive(Clone, Copy, Debug)]
struct Aborted;

/// Runs deterministic iterative deepening and reports each completed depth.
pub fn iterative_deepening(
    board: &Board,
    limits: SearchLimits,
    stop: &AtomicBool,
    mut on_info: impl FnMut(SearchInfo),
) -> SearchOutcome {
    let table_bytes = limits.hash_size_mb.saturating_mul(1024 * 1024);
    let table = TranspositionTable::new(table_bytes);
    let start = Instant::now();
    let root_moves = board.legal_moves();
    let fallback = root_moves.first().copied();
    let max_depth = limits
        .depth
        .unwrap_or((MAX_PLY - 1) as u8)
        .clamp(1, (MAX_PLY - 1) as u8);
    let mut search = SearchState {
        context: SearchContext {
            stop,
            start,
            hard_time: limits.hard_time,
            max_nodes: limits.nodes,
            nodes: 0,
        },
        table,
        history: ButterflyHistory::default(),
    };
    let mut outcome = SearchOutcome {
        best_move: fallback,
        score: 0,
        completed_depth: 0,
        nodes: 0,
        elapsed: Duration::ZERO,
        pv: fallback.into_iter().collect(),
    };

    for depth in 1..=max_depth {
        let mut position = board.clone();
        let mut pv = PvLine::default();
        let result = negamax(&mut position, depth, 0, -INFINITY, INFINITY, &mut search, &mut pv);
        let Ok(score) = result else {
            break;
        };

        let elapsed = start.elapsed();
        let pv = pv.to_vec();
        outcome.best_move = pv.first().copied().or(fallback);
        outcome.score = score;
        outcome.completed_depth = depth;
        outcome.nodes = search.context.nodes;
        outcome.elapsed = elapsed;
        outcome.pv.clone_from(&pv);
        on_info(SearchInfo {
            depth,
            score,
            nodes: search.context.nodes,
            elapsed,
            pv,
        });

        if search.context.should_stop_between_iterations(limits.soft_time) {
            break;
        }
    }

    outcome.nodes = search.context.nodes;
    outcome.elapsed = start.elapsed();
    outcome
}

fn negamax(
    board: &mut Board,
    depth: u8,
    ply: usize,
    mut alpha: i32,
    beta: i32,
    search: &mut SearchState<'_>,
    pv: &mut PvLine,
) -> Result<i32, Aborted> {
    if depth == 0 {
        return quiescence(board, ply, alpha, beta, search, pv);
    }

    search.context.enter_node()?;
    pv.len = 0;

    let mut moves = board.legal_moves();
    if moves.is_empty() {
        return Ok(if board.is_in_check(board.side_to_move()) {
            -MATE_SCORE + ply as i32
        } else {
            0
        });
    }
    if board.is_fifty_move_draw() || board.is_threefold_repetition() || board.is_insufficient_material() {
        return Ok(0);
    }
    if ply + 1 >= MAX_PLY {
        return Ok(evaluation::evaluate(board));
    }

    let key = board.hash();
    let entry = search.table.probe(key);
    if let Some(score) = entry.and_then(|stored| tt_cutoff(stored, depth, ply, alpha, beta)) {
        return Ok(score);
    }
    let alpha_start = alpha;

    let side_to_move = board.side_to_move();
    order_moves(
        board,
        &mut moves,
        entry.and_then(|stored| stored.best_move),
        &search.history,
    );

    let mut best = -INFINITY;
    let mut best_move = None;
    for mv in moves {
        let undo = board
            .make_move_unchecked(mv)
            .expect("legal move must be structurally valid");
        let mut child_pv = PvLine::default();
        let child = negamax(board, depth - 1, ply + 1, -beta, -alpha, search, &mut child_pv);
        board.unmake_move(undo);
        let score = -child?;

        if score > best {
            best = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
            pv.prepend(mv, &child_pv);
        }
        if alpha >= beta {
            if is_quiet(board, mv) {
                search.history.reward(side_to_move, mv, depth);
            }
            break;
        }
    }

    search.table.store(Entry {
        key,
        score: score_to_tt(best, ply),
        best_move,
        depth,
        bound: classify_bound(best, alpha_start, beta),
    });
    Ok(best)
}

fn quiescence(
    board: &mut Board,
    ply: usize,
    mut alpha: i32,
    beta: i32,
    search: &mut SearchState<'_>,
    pv: &mut PvLine,
) -> Result<i32, Aborted> {
    search.context.enter_node()?;
    pv.len = 0;

    let in_check = board.is_in_check(board.side_to_move());
    let mut moves = board.legal_moves();
    if moves.is_empty() {
        return Ok(if in_check { -MATE_SCORE + ply as i32 } else { 0 });
    }
    if board.is_fifty_move_draw() || board.is_threefold_repetition() || board.is_insufficient_material() {
        return Ok(0);
    }
    if ply + 1 >= MAX_PLY {
        return Ok(evaluation::evaluate(board));
    }

    let key = board.hash();
    let entry = search.table.probe(key);
    if let Some(score) = entry.and_then(|stored| tt_cutoff(stored, 0, ply, alpha, beta)) {
        return Ok(score);
    }
    let alpha_start = alpha;
    let mut best = -INFINITY;
    let mut best_move = None;
    if !in_check {
        let stand_pat = evaluation::evaluate(board);
        best = stand_pat;
        if stand_pat >= beta {
            search.table.store(Entry {
                key,
                score: score_to_tt(stand_pat, ply),
                best_move: None,
                depth: 0,
                bound: Bound::Lower,
            });
            return Ok(stand_pat);
        }
        alpha = alpha.max(stand_pat);
        moves.retain(|&mv| is_tactical(board, mv));
    }

    order_moves(
        board,
        &mut moves,
        entry.and_then(|stored| stored.best_move),
        &search.history,
    );
    for mv in moves {
        let undo = board
            .make_move_unchecked(mv)
            .expect("legal move must be structurally valid");
        let mut child_pv = PvLine::default();
        let child = quiescence(board, ply + 1, -beta, -alpha, search, &mut child_pv);
        board.unmake_move(undo);
        let score = -child?;

        if score > best {
            best = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
            pv.prepend(mv, &child_pv);
        }
        if alpha >= beta {
            break;
        }
    }

    search.table.store(Entry {
        key,
        score: score_to_tt(best, ply),
        best_move,
        depth: 0,
        bound: classify_bound(best, alpha_start, beta),
    });
    Ok(best)
}

fn is_tactical(board: &Board, mv: Move) -> bool {
    mv.promotion().is_some() || mvv_lva_score(board, mv) > 0
}

fn is_quiet(board: &Board, mv: Move) -> bool {
    mv.promotion().is_none() && mvv_lva_score(board, mv) == 0
}

fn tt_cutoff(entry: Entry, depth: u8, ply: usize, alpha: i32, beta: i32) -> Option<i32> {
    if entry.depth < depth {
        return None;
    }
    let score = score_from_tt(entry.score, ply);
    match entry.bound {
        Bound::Exact => Some(score),
        Bound::Lower if score >= beta => Some(score),
        Bound::Upper if score <= alpha => Some(score),
        Bound::Lower | Bound::Upper => None,
    }
}

fn classify_bound(score: i32, alpha: i32, beta: i32) -> Bound {
    if score <= alpha {
        Bound::Upper
    } else if score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    }
}

fn order_moves(board: &Board, moves: &mut [Move], tt_move: Option<Move>, history: &ButterflyHistory) {
    let color = board.side_to_move();
    moves.sort_by_key(|&mv| {
        let capture_score = mvv_lva_score(board, mv);
        let history_score = if is_quiet(board, mv) {
            history.score(color, mv)
        } else {
            0
        };
        std::cmp::Reverse((Some(mv) == tt_move, capture_score > 0, capture_score, history_score))
    });
}

fn mvv_lva_score(board: &Board, mv: Move) -> i32 {
    let victim = match mv.kind() {
        MoveKind::EnPassant => Some(PieceType::Pawn),
        MoveKind::Castling => None,
        MoveKind::Normal | MoveKind::Promotion => board.piece_at(mv.to()).map(|piece| piece.kind),
    };
    let Some(victim) = victim else {
        return 0;
    };
    let attacker = board
        .piece_at(mv.from())
        .expect("generated move source must contain a piece")
        .kind;

    64 + piece_order_value(victim) * 8 - piece_order_value(attacker)
}

const fn piece_order_value(kind: PieceType) -> i32 {
    match kind {
        PieceType::Pawn => 1,
        PieceType::Knight => 2,
        PieceType::Bishop => 3,
        PieceType::Rook => 4,
        PieceType::Queen => 5,
        PieceType::King => 6,
    }
}

#[must_use]
pub const fn mate_moves(score: i32) -> Option<i32> {
    if score >= MATE_SCORE - MAX_PLY as i32 {
        Some((MATE_SCORE - score + 1) / 2)
    } else if score <= -MATE_SCORE + MAX_PLY as i32 {
        Some(-((MATE_SCORE + score + 1) / 2))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use super::*;

    fn unlimited_search(stop: &AtomicBool) -> SearchState<'_> {
        SearchState {
            context: SearchContext {
                stop,
                start: Instant::now(),
                hard_time: None,
                max_nodes: None,
                nodes: 0,
            },
            table: TranspositionTable::new(64 * 1024),
            history: ButterflyHistory::default(),
        }
    }

    #[test]
    fn finds_mate_in_one() {
        let board = Board::from_fen("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1").unwrap();
        let stop = AtomicBool::new(false);
        let result = iterative_deepening(
            &board,
            SearchLimits {
                depth: Some(2),
                ..SearchLimits::default()
            },
            &stop,
            |_| {},
        );
        assert_eq!(mate_moves(result.score), Some(1));
        assert!(result.best_move.is_some());
        assert_eq!(result.completed_depth, 2);
    }

    #[test]
    fn node_limit_returns_a_legal_fallback() {
        let board = Board::default();
        let stop = AtomicBool::new(false);
        let result = iterative_deepening(
            &board,
            SearchLimits {
                nodes: Some(1),
                ..SearchLimits::default()
            },
            &stop,
            |_| {},
        );
        assert!(result.best_move.is_some_and(|mv| board.legal_moves().contains(&mv)));
        assert_eq!(result.nodes, 1);
    }

    #[test]
    fn fail_soft_returns_score_beyond_beta() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/4Q3/4K3 w - - 0 1").unwrap();
        let stop = AtomicBool::new(false);
        let mut search = unlimited_search(&stop);
        let mut pv = PvLine::default();
        let score = negamax(&mut board, 1, 0, -50, 50, &mut search, &mut pv).unwrap();
        assert!(
            score > 50,
            "fail-soft search returned the beta bound instead of the score"
        );
        assert!(
            search.history.scores[Color::White.index()]
                .iter()
                .flatten()
                .any(|&score| score > 0)
        );
    }

    #[test]
    fn exact_tt_entry_avoids_researching_a_position() {
        let mut board = Board::default();
        let stop = AtomicBool::new(false);
        let mut search = unlimited_search(&stop);
        let mut first_pv = PvLine::default();
        let first_score = negamax(&mut board, 3, 0, -INFINITY, INFINITY, &mut search, &mut first_pv).unwrap();
        assert!(search.context.nodes > 1);

        search.context.nodes = 0;
        let mut second_pv = PvLine::default();
        let second_score = negamax(&mut board, 3, 0, -INFINITY, INFINITY, &mut search, &mut second_pv).unwrap();

        assert_eq!(second_score, first_score);
        assert_eq!(search.context.nodes, 1);
    }

    #[test]
    fn quiescence_avoids_a_poisoned_capture_at_the_horizon() {
        let board = Board::from_fen("3q3k/8/8/3r4/8/8/8/3Q3K w - - 0 1").unwrap();
        let poisoned = Move::new("d1".parse().unwrap(), "d5".parse().unwrap());
        let stop = AtomicBool::new(false);
        let result = iterative_deepening(
            &board,
            SearchLimits {
                depth: Some(1),
                ..SearchLimits::default()
            },
            &stop,
            |_| {},
        );

        assert_ne!(result.best_move, Some(poisoned));
        assert!(result.score < 0);
    }

    #[test]
    fn quiescence_searches_quiet_promotions() {
        let mut board = Board::from_fen("7k/P7/8/8/8/8/8/7K w - - 0 1").unwrap();
        let stop = AtomicBool::new(false);
        let mut search = unlimited_search(&stop);
        let mut pv = PvLine::default();
        let stand_pat = evaluation::evaluate(&board);

        let score = quiescence(&mut board, 0, -INFINITY, INFINITY, &mut search, &mut pv).unwrap();

        assert!(score > stand_pat);
        assert_eq!(pv.moves[0].promotion(), Some(PieceType::Queen));
    }

    #[test]
    fn quiescence_searches_all_legal_evasions_while_in_check() {
        let mut board = Board::from_fen("4r1k1/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let stop = AtomicBool::new(false);
        let mut search = unlimited_search(&stop);
        let mut pv = PvLine::default();

        let score = quiescence(&mut board, 0, -INFINITY, INFINITY, &mut search, &mut pv).unwrap();

        assert!(score < 0);
        assert!(search.context.nodes > 1, "quiet check evasions were not searched");
        assert_eq!(pv.len, 1);
    }

    #[test]
    fn quiescence_stand_pat_cutoff_is_fail_soft() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/4Q3/4K3 w - - 0 1").unwrap();
        let stop = AtomicBool::new(false);
        let mut search = unlimited_search(&stop);
        let mut pv = PvLine::default();

        let score = quiescence(&mut board, 0, -50, 50, &mut search, &mut pv).unwrap();

        assert_eq!(score, evaluation::evaluate(&board));
        assert!(score > 50);
    }

    #[test]
    fn orders_tt_move_before_mvv_lva_captures_and_quiet_moves() {
        let board = Board::from_fen("k7/8/8/3q3r/4P3/8/8/K2Q4 w - - 0 1").unwrap();
        let pawn_takes_queen = Move::new("e4".parse().unwrap(), "d5".parse().unwrap());
        let queen_takes_queen = Move::new("d1".parse().unwrap(), "d5".parse().unwrap());
        let queen_takes_rook = Move::new("d1".parse().unwrap(), "h5".parse().unwrap());
        let quiet = Move::new("d1".parse().unwrap(), "d2".parse().unwrap());
        let mut moves = [quiet, queen_takes_rook, queen_takes_queen, pawn_takes_queen];

        let history = ButterflyHistory::default();
        order_moves(&board, &mut moves, None, &history);

        assert_eq!(moves, [pawn_takes_queen, queen_takes_queen, queen_takes_rook, quiet]);

        order_moves(&board, &mut moves, Some(quiet), &history);
        assert_eq!(moves, [quiet, pawn_takes_queen, queen_takes_queen, queen_takes_rook]);
    }

    #[test]
    fn orders_rewarded_quiets_by_butterfly_history() {
        let board = Board::starting_position();
        let a3 = Move::new("a2".parse().unwrap(), "a3".parse().unwrap());
        let nc3 = Move::new("b1".parse().unwrap(), "c3".parse().unwrap());
        let mut history = ButterflyHistory::default();
        history.reward(Color::White, nc3, 6);
        let mut moves = [a3, nc3];

        order_moves(&board, &mut moves, None, &history);

        assert_eq!(moves, [nc3, a3]);
    }

    #[test]
    fn captures_stay_ahead_of_high_history_quiets() {
        let board = Board::from_fen("k7/8/8/3q4/4P3/8/8/K7 w - - 0 1").unwrap();
        let capture = Move::new("e4".parse().unwrap(), "d5".parse().unwrap());
        let quiet = Move::new("e4".parse().unwrap(), "e5".parse().unwrap());
        let mut history = ButterflyHistory::default();
        for _ in 0..1_000 {
            history.reward(Color::White, quiet, 127);
        }
        let mut moves = [quiet, capture];

        order_moves(&board, &mut moves, None, &history);

        assert_eq!(moves, [capture, quiet]);
    }

    #[test]
    fn recognizes_en_passant_but_not_internal_castling_as_a_capture() {
        let en_passant_board = Board::from_fen("k7/8/8/3pP3/8/8/8/K7 w - d6 0 1").unwrap();
        let en_passant = Move::new_en_passant("e5".parse().unwrap(), "d6".parse().unwrap());
        assert!(mvv_lva_score(&en_passant_board, en_passant) > 0);

        let castling_board = Board::default();
        let castling = Move::new_castling("e1".parse().unwrap(), "h1".parse().unwrap());
        assert_eq!(mvv_lva_score(&castling_board, castling), 0);
    }
}
