use crate::{Board, Color, PieceType, Square, types::BitboardIter};

const MAX_PHASE: i32 = 24;
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

const MIDDLEGAME_VALUES: [i32; 6] = [100, 320, 335, 500, 900, 0];
const ENDGAME_VALUES: [i32; 6] = [120, 305, 325, 520, 900, 0];
const DOUBLED_PAWN_MG: i32 = -8;
const DOUBLED_PAWN_EG: i32 = -12;
const ISOLATED_PAWN_MG: i32 = -10;
const ISOLATED_PAWN_EG: i32 = -8;
const PASSED_PAWN_MG_BY_RANK: [i32; 8] = [0, 4, 8, 14, 24, 40, 70, 0];
const PASSED_PAWN_EG_BY_RANK: [i32; 8] = [0, 8, 16, 28, 48, 80, 130, 0];
const MOBILITY_MG: [i32; 6] = [0, 4, 4, 2, 1, 0];
const MOBILITY_EG: [i32; 6] = [0, 2, 3, 3, 2, 0];
const KNIGHT_OFFSETS: [(i8, i8); 8] = [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const ROOK_DIRECTIONS: [(i8, i8); 4] = [(-1, 0), (0, -1), (0, 1), (1, 0)];

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

/// Returns a tapered static evaluation from the side-to-move's perspective.
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
    let (pawn_middlegame, pawn_endgame) = pawn_structure(board);
    middlegame += pawn_middlegame;
    endgame += pawn_endgame;
    let (mobility_middlegame, mobility_endgame) = piece_mobility(board);
    middlegame += mobility_middlegame;
    endgame += mobility_endgame;

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

fn pawn_structure(board: &Board) -> (i32, i32) {
    let mut middlegame = 0;
    let mut endgame = 0;

    for color in Color::ALL {
        let pawns = board.colored_pieces(color, PieceType::Pawn);
        let file_counts = pawn_file_counts(pawns);
        let sign = if color == Color::White { 1 } else { -1 };

        for count in file_counts {
            if count > 1 {
                let extra = i32::from(count - 1);
                middlegame += sign * DOUBLED_PAWN_MG * extra;
                endgame += sign * DOUBLED_PAWN_EG * extra;
            }
        }

        let enemy_pawns = board.colored_pieces(!color, PieceType::Pawn);
        for square in BitboardIter(pawns) {
            if is_isolated_pawn(square.file(), &file_counts) {
                middlegame += sign * ISOLATED_PAWN_MG;
                endgame += sign * ISOLATED_PAWN_EG;
            }
            if is_passed_pawn(color, square, enemy_pawns) {
                let rank = relative_square(color, square).rank() as usize;
                middlegame += sign * PASSED_PAWN_MG_BY_RANK[rank];
                endgame += sign * PASSED_PAWN_EG_BY_RANK[rank];
            }
        }
    }

    (middlegame, endgame)
}

fn piece_mobility(board: &Board) -> (i32, i32) {
    let mut middlegame = 0;
    let mut endgame = 0;

    for color in Color::ALL {
        let sign = if color == Color::White { 1 } else { -1 };
        for kind in [PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen] {
            let count = BitboardIter(board.colored_pieces(color, kind))
                .map(|from| mobility_count(board, color, kind, from))
                .sum::<i32>();
            middlegame += sign * MOBILITY_MG[kind.index()] * count;
            endgame += sign * MOBILITY_EG[kind.index()] * count;
        }
    }

    (middlegame, endgame)
}

fn mobility_count(board: &Board, color: Color, kind: PieceType, from: Square) -> i32 {
    match kind {
        PieceType::Knight => leaper_mobility(board, color, from, &KNIGHT_OFFSETS),
        PieceType::Bishop => slider_mobility(board, color, from, &BISHOP_DIRECTIONS),
        PieceType::Rook => slider_mobility(board, color, from, &ROOK_DIRECTIONS),
        PieceType::Queen => {
            slider_mobility(board, color, from, &BISHOP_DIRECTIONS)
                + slider_mobility(board, color, from, &ROOK_DIRECTIONS)
        }
        PieceType::Pawn | PieceType::King => 0,
    }
}

fn leaper_mobility(board: &Board, color: Color, from: Square, offsets: &[(i8, i8)]) -> i32 {
    offsets
        .iter()
        .filter(|&&(file_delta, rank_delta)| {
            from.offset(file_delta, rank_delta)
                .is_some_and(|to| board.piece_at(to).is_none_or(|piece| piece.color != color))
        })
        .count() as i32
}

fn slider_mobility(board: &Board, color: Color, from: Square, directions: &[(i8, i8)]) -> i32 {
    let mut count = 0;
    for &(file_delta, rank_delta) in directions {
        let mut cursor = from;
        while let Some(to) = cursor.offset(file_delta, rank_delta) {
            cursor = to;
            if let Some(piece) = board.piece_at(to) {
                if piece.color != color {
                    count += 1;
                }
                break;
            }
            count += 1;
        }
    }
    count
}

fn pawn_file_counts(pawns: u64) -> [u8; 8] {
    let mut counts = [0; 8];
    for square in BitboardIter(pawns) {
        counts[square.file() as usize] += 1;
    }
    counts
}

fn is_isolated_pawn(file: u8, file_counts: &[u8; 8]) -> bool {
    let left_empty = file == 0 || file_counts[file as usize - 1] == 0;
    let right_empty = file == 7 || file_counts[file as usize + 1] == 0;
    left_empty && right_empty
}

fn is_passed_pawn(color: Color, square: Square, enemy_pawns: u64) -> bool {
    BitboardIter(enemy_pawns).all(|enemy| {
        enemy.file().abs_diff(square.file()) > 1
            || match color {
                Color::White => enemy.rank() <= square.rank(),
                Color::Black => enemy.rank() >= square.rank(),
            }
    })
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
    fn black_pawn_structure_mirrors_white_pawn_structure() {
        let white = Board::from_fen("4k3/8/8/8/4P3/8/8/4K3 w - - 0 1").unwrap();
        let black = Board::from_fen("4k3/8/8/4p3/8/8/8/4K3 b - - 0 1").unwrap();
        assert_eq!(evaluate(&white), evaluate(&black));
    }

    #[test]
    fn black_mobility_mirrors_white_mobility() {
        let white = Board::from_fen("4k3/8/8/8/3B4/8/8/4K3 w - - 0 1").unwrap();
        let black = Board::from_fen("4k3/8/8/3b4/8/8/8/4K3 b - - 0 1").unwrap();
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

    #[test]
    fn knight_mobility_rewards_more_available_squares() {
        let open = Board::from_fen("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1").unwrap();
        let blocked = Board::from_fen("4k3/8/2P1P3/1P3P2/3N4/1P3P2/2P1P3/4K3 w - - 0 1").unwrap();

        assert!(piece_mobility(&open).0 > piece_mobility(&blocked).0);
        assert!(piece_mobility(&open).1 > piece_mobility(&blocked).1);
    }

    #[test]
    fn slider_mobility_counts_open_lines_until_blocked() {
        let open = Board::from_fen("4k3/8/8/8/3R4/8/8/4K3 w - - 0 1").unwrap();
        let blocked = Board::from_fen("4k3/8/8/3P4/2PRP3/8/8/4K3 w - - 0 1").unwrap();

        assert_eq!(piece_mobility(&open), (28, 42));
        assert!(piece_mobility(&open).0 > piece_mobility(&blocked).0);
        assert!(piece_mobility(&open).1 > piece_mobility(&blocked).1);
    }

    #[test]
    fn doubled_pawns_are_penalized() {
        let doubled = Board::from_fen("4k3/3ppp2/8/8/8/4P3/3PP3/4K3 w - - 0 1").unwrap();
        let spread = Board::from_fen("4k3/3ppp2/8/8/8/8/3PPP2/4K3 w - - 0 1").unwrap();

        assert!(pawn_structure(&doubled).0 < pawn_structure(&spread).0);
        assert!(pawn_structure(&doubled).1 < pawn_structure(&spread).1);
    }

    #[test]
    fn isolated_pawns_are_penalized() {
        let isolated = Board::from_fen("4k3/3ppp2/8/8/8/8/3P1P2/4K3 w - - 0 1").unwrap();
        let connected = Board::from_fen("4k3/3ppp2/8/8/8/8/3PP3/4K3 w - - 0 1").unwrap();

        assert!(pawn_structure(&isolated).0 < pawn_structure(&connected).0);
        assert!(pawn_structure(&isolated).1 < pawn_structure(&connected).1);
    }

    #[test]
    fn passed_pawns_gain_value_as_they_advance() {
        let fourth_rank = Board::from_fen("4k3/8/8/8/4P3/8/8/4K3 w - - 0 1").unwrap();
        let fifth_rank = Board::from_fen("4k3/8/8/4P3/8/8/8/4K3 w - - 0 1").unwrap();

        assert!(pawn_structure(&fifth_rank).0 > pawn_structure(&fourth_rank).0);
        assert!(pawn_structure(&fifth_rank).1 > pawn_structure(&fourth_rank).1);
    }
}
