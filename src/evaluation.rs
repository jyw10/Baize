use crate::{Board, Color, PieceType};

const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 0];

/// Returns a static material score from the side-to-move's perspective.
#[must_use]
pub fn evaluate(board: &Board) -> i32 {
    let white_relative = PieceType::ALL.into_iter().fold(0, |score, kind| {
        let value = PIECE_VALUES[kind.index()];
        let white = board.colored_pieces(Color::White, kind).count_ones() as i32;
        let black = board.colored_pieces(Color::Black, kind).count_ones() as i32;
        score + value * (white - black)
    });
    if board.side_to_move() == Color::White {
        white_relative
    } else {
        -white_relative
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_is_side_relative() {
        let white = Board::from_fen("4k3/8/8/8/8/8/4Q3/4K3 w - - 0 1").unwrap();
        let black = Board::from_fen("4k3/8/8/8/8/8/4Q3/4K3 b - - 0 1").unwrap();
        assert_eq!(evaluate(&white), 900);
        assert_eq!(evaluate(&black), -900);
    }
}
