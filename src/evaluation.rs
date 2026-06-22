use crate::{Board, Color, PieceType, Square};

const MAX_PHASE: i32 = 24;
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

const MIDDLEGAME_VALUES: [i32; 6] = [100, 320, 335, 500, 900, 0];
const ENDGAME_VALUES: [i32; 6] = [120, 305, 325, 520, 900, 0];

type HalfBoardTable = [[i32; 4]; 8];

// Tables are indexed from White's first rank and mirrored across the files.
const MIDDLEGAME_TABLES: [HalfBoardTable; 6] = [
    [
        [0, 0, 0, 0],
        [-8, -2, 4, 10],
        [-6, 0, 8, 14],
        [-2, 6, 16, 24],
        [4, 12, 24, 34],
        [12, 22, 34, 44],
        [30, 40, 50, 60],
        [0, 0, 0, 0],
    ],
    [
        [-50, -35, -25, -20],
        [-38, -20, -8, -2],
        [-26, -8, 8, 14],
        [-20, 0, 16, 24],
        [-18, 2, 18, 26],
        [-24, -6, 10, 16],
        [-36, -18, -6, 0],
        [-50, -36, -26, -20],
    ],
    [
        [-22, -12, -14, -16],
        [-14, -4, -2, 2],
        [-10, 2, 8, 10],
        [-8, 4, 10, 14],
        [-6, 6, 12, 16],
        [-8, 2, 8, 12],
        [-12, -2, 2, 6],
        [-20, -12, -10, -14],
    ],
    [
        [0, 2, 4, 6],
        [4, 8, 10, 12],
        [-4, 0, 2, 4],
        [-2, 2, 4, 6],
        [0, 4, 6, 8],
        [4, 8, 10, 12],
        [14, 18, 22, 24],
        [6, 8, 10, 12],
    ],
    [
        [-12, -8, -6, -10],
        [-10, -4, -2, 0],
        [-8, -2, 4, 6],
        [-6, 0, 6, 10],
        [-4, 2, 8, 12],
        [-6, 0, 6, 8],
        [-10, -4, 0, 2],
        [-14, -10, -8, -10],
    ],
    [
        [18, 28, 12, -24],
        [4, 8, -4, -18],
        [-12, -16, -20, -26],
        [-24, -28, -32, -38],
        [-30, -34, -38, -44],
        [-34, -38, -42, -48],
        [-36, -40, -44, -50],
        [-38, -42, -46, -52],
    ],
];

const ENDGAME_TABLES: [HalfBoardTable; 6] = [
    [
        [0, 0, 0, 0],
        [0, 4, 8, 12],
        [4, 8, 14, 20],
        [8, 14, 22, 30],
        [16, 24, 34, 44],
        [28, 40, 54, 68],
        [52, 66, 82, 96],
        [0, 0, 0, 0],
    ],
    [
        [-38, -26, -18, -14],
        [-28, -14, -4, 2],
        [-20, -4, 10, 16],
        [-16, 2, 16, 24],
        [-16, 2, 16, 24],
        [-20, -4, 10, 16],
        [-28, -14, -4, 2],
        [-38, -26, -18, -14],
    ],
    [
        [-18, -10, -6, -4],
        [-12, -2, 4, 6],
        [-8, 4, 10, 14],
        [-6, 6, 14, 18],
        [-6, 6, 14, 18],
        [-8, 4, 10, 14],
        [-12, -2, 4, 6],
        [-18, -10, -6, -4],
    ],
    [
        [-4, 0, 4, 6],
        [-2, 2, 6, 8],
        [0, 4, 8, 10],
        [2, 6, 10, 12],
        [2, 6, 10, 12],
        [0, 4, 8, 10],
        [-2, 2, 6, 8],
        [-4, 0, 4, 6],
    ],
    [
        [-12, -8, -4, -2],
        [-8, -2, 2, 4],
        [-4, 2, 6, 10],
        [-2, 4, 10, 14],
        [-2, 4, 10, 14],
        [-4, 2, 6, 10],
        [-8, -2, 2, 4],
        [-12, -8, -4, -2],
    ],
    [
        [-48, -32, -24, -20],
        [-34, -18, -8, -4],
        [-24, -8, 8, 14],
        [-18, -2, 14, 24],
        [-18, -2, 14, 24],
        [-24, -8, 8, 14],
        [-34, -18, -8, -4],
        [-48, -32, -24, -20],
    ],
];

/// Returns a tapered material and piece-square score from the side-to-move's perspective.
#[must_use]
pub fn evaluate(board: &Board) -> i32 {
    let mut middlegame = 0;
    let mut endgame = 0;

    for square in board.occupied_squares() {
        let piece = board.piece_at(square).expect("occupied square contains a piece");
        let relative_square = relative_square(piece.color, square);
        let sign = if piece.color == Color::White { 1 } else { -1 };
        let kind = piece.kind.index();
        middlegame += sign * (MIDDLEGAME_VALUES[kind] + table_value(&MIDDLEGAME_TABLES[kind], relative_square));
        endgame += sign * (ENDGAME_VALUES[kind] + table_value(&ENDGAME_TABLES[kind], relative_square));
    }

    let phase = game_phase(board);
    let white_relative = (middlegame * phase + endgame * (MAX_PHASE - phase)) / MAX_PHASE;
    if board.side_to_move() == Color::White {
        white_relative
    } else {
        -white_relative
    }
}

fn game_phase(board: &Board) -> i32 {
    PieceType::ALL
        .into_iter()
        .map(|kind| PHASE_WEIGHTS[kind.index()] * board.pieces(kind).count_ones() as i32)
        .sum::<i32>()
        .min(MAX_PHASE)
}

const fn relative_square(color: Color, square: Square) -> Square {
    match color {
        Color::White => square,
        Color::Black => square.flip_vertical(),
    }
}

fn table_value(table: &HalfBoardTable, square: Square) -> i32 {
    let file = square.file().min(7 - square.file()) as usize;
    table[square.rank() as usize][file]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_is_side_relative() {
        let white = Board::from_fen("4k3/8/8/8/8/8/4Q3/4K3 w - - 0 1").unwrap();
        let black = Board::from_fen("4k3/8/8/8/8/8/4Q3/4K3 b - - 0 1").unwrap();
        assert_eq!(evaluate(&white), -evaluate(&black));
        assert!(evaluate(&white) > 0);
    }

    #[test]
    fn starting_position_is_balanced() {
        assert_eq!(evaluate(&Board::starting_position()), 0);
    }

    #[test]
    fn black_tables_are_vertical_mirrors_of_white_tables() {
        let white = Board::from_fen("4k3/8/8/8/8/2N5/8/4K3 w - - 0 1").unwrap();
        let black = Board::from_fen("4k3/8/2n5/8/8/8/8/4K3 b - - 0 1").unwrap();
        assert_eq!(evaluate(&white), evaluate(&black));
    }

    #[test]
    fn phase_is_bounded_by_normal_starting_material() {
        assert_eq!(game_phase(&Board::starting_position()), MAX_PHASE);
        let kings = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(game_phase(&kings), 0);

        let promoted = Board::from_fen("4k3/QQQQQQQQ/8/8/8/8/qqqqqqqq/4K3 w - - 0 1").unwrap();
        assert_eq!(game_phase(&promoted), MAX_PHASE);
    }

    #[test]
    fn knight_prefers_the_center_to_the_corner() {
        let center = Board::from_fen("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1").unwrap();
        let corner = Board::from_fen("4k3/8/8/8/8/8/8/N3K3 w - - 0 1").unwrap();
        assert!(evaluate(&center) > evaluate(&corner));
    }

    #[test]
    fn king_prefers_the_center_in_the_endgame() {
        let center = Board::from_fen("4k3/8/8/8/3K4/8/8/8 w - - 0 1").unwrap();
        let corner = Board::from_fen("4k3/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        assert!(evaluate(&center) > evaluate(&corner));
    }
}
