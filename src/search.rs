use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use crate::{Board, Move, evaluation};

pub const MATE_SCORE: i32 = 30_000;
pub const MAX_PLY: usize = 128;
const INFINITY: i32 = 32_000;

#[derive(Clone, Copy, Debug, Default)]
pub struct SearchLimits {
    pub depth: Option<u8>,
    pub nodes: Option<u64>,
    pub soft_time: Option<Duration>,
    pub hard_time: Option<Duration>,
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

#[derive(Clone, Copy, Debug)]
struct Aborted;

/// Runs deterministic iterative deepening and reports each completed depth.
pub fn iterative_deepening(
    board: &Board,
    limits: SearchLimits,
    stop: &AtomicBool,
    mut on_info: impl FnMut(SearchInfo),
) -> SearchOutcome {
    let start = Instant::now();
    let root_moves = board.legal_moves();
    let fallback = root_moves.first().copied();
    let max_depth = limits
        .depth
        .unwrap_or((MAX_PLY - 1) as u8)
        .clamp(1, (MAX_PLY - 1) as u8);
    let mut context = SearchContext {
        stop,
        start,
        hard_time: limits.hard_time,
        max_nodes: limits.nodes,
        nodes: 0,
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
        let result = negamax(&mut position, depth, 0, -INFINITY, INFINITY, &mut context, &mut pv);
        let Ok(score) = result else {
            break;
        };

        let elapsed = start.elapsed();
        let pv = pv.to_vec();
        outcome.best_move = pv.first().copied().or(fallback);
        outcome.score = score;
        outcome.completed_depth = depth;
        outcome.nodes = context.nodes;
        outcome.elapsed = elapsed;
        outcome.pv.clone_from(&pv);
        on_info(SearchInfo {
            depth,
            score,
            nodes: context.nodes,
            elapsed,
            pv,
        });

        if context.should_stop_between_iterations(limits.soft_time) {
            break;
        }
    }

    outcome.nodes = context.nodes;
    outcome.elapsed = start.elapsed();
    outcome
}

fn negamax(
    board: &mut Board,
    depth: u8,
    ply: usize,
    mut alpha: i32,
    beta: i32,
    context: &mut SearchContext<'_>,
    pv: &mut PvLine,
) -> Result<i32, Aborted> {
    context.enter_node()?;
    pv.len = 0;

    let moves = board.legal_moves();
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
    if depth == 0 || ply + 1 >= MAX_PLY {
        return Ok(evaluation::evaluate(board));
    }

    let mut best = -INFINITY;
    for mv in moves {
        let undo = board
            .make_move_unchecked(mv)
            .expect("legal move must be structurally valid");
        let mut child_pv = PvLine::default();
        let child = negamax(board, depth - 1, ply + 1, -beta, -alpha, context, &mut child_pv);
        board.unmake_move(undo);
        let score = -child?;

        if score > best {
            best = score;
        }
        if score > alpha {
            alpha = score;
            pv.prepend(mv, &child_pv);
        }
        if alpha >= beta {
            break;
        }
    }

    Ok(best)
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
        let mut context = SearchContext {
            stop: &stop,
            start: Instant::now(),
            hard_time: None,
            max_nodes: None,
            nodes: 0,
        };
        let mut pv = PvLine::default();
        let score = negamax(&mut board, 1, 0, -50, 50, &mut context, &mut pv).unwrap();
        assert_eq!(score, 900, "fail-soft search returned a bound instead of the score");
    }
}
