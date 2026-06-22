use crate::{Board, Move};

/// Counts legal leaf nodes at `depth`.
#[must_use]
pub fn perft(board: &mut Board, depth: u8) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = board.legal_moves();
    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0;
    for mv in moves {
        let undo = board
            .make_move_unchecked(mv)
            .expect("legal move must be structurally valid");
        nodes += perft(board, depth - 1);
        board.unmake_move(undo);
    }
    nodes
}

#[must_use]
pub fn perft_divide(board: &mut Board, depth: u8) -> Vec<(Move, u64)> {
    if depth == 0 {
        return Vec::new();
    }
    let moves = board.legal_moves();
    let mut result = Vec::with_capacity(moves.len());
    for mv in moves {
        let undo = board
            .make_move_unchecked(mv)
            .expect("legal move must be structurally valid");
        let nodes = perft(board, depth - 1);
        board.unmake_move(undo);
        result.push((mv, nodes));
    }
    result
}
