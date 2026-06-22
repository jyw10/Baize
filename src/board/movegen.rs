use crate::{
    board::Board,
    types::{BitboardIter, Color, Move, Piece, PieceType, Square},
};

const KNIGHT_OFFSETS: [(i8, i8); 8] = [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];
const KING_OFFSETS: [(i8, i8); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const ROOK_DIRECTIONS: [(i8, i8); 4] = [(-1, 0), (0, -1), (0, 1), (1, 0)];

impl Board {
    #[must_use]
    pub fn legal_moves(&self) -> Vec<Move> {
        let side = self.side_to_move();
        let pseudo = self.pseudo_legal_moves();
        let mut board = self.clone();
        let mut legal = Vec::with_capacity(pseudo.len());
        for mv in pseudo {
            let undo = board
                .make_move_unchecked(mv)
                .expect("pseudo-legal move must satisfy structural move requirements");
            if !board.is_in_check(side) {
                legal.push(mv);
            }
            board.unmake_move(undo);
        }
        legal
    }

    #[must_use]
    pub fn is_in_check(&self, color: Color) -> bool {
        self.king_square(color)
            .is_some_and(|king| self.is_square_attacked(king, !color))
    }

    #[must_use]
    pub fn is_square_attacked(&self, target: Square, by: Color) -> bool {
        let pawn_source_rank = if by == Color::White { -1 } else { 1 };
        for file_delta in [-1, 1] {
            if target
                .offset(file_delta, pawn_source_rank)
                .is_some_and(|square| self.piece_at(square) == Some(Piece::new(by, PieceType::Pawn)))
            {
                return true;
            }
        }

        if KNIGHT_OFFSETS.into_iter().any(|(file, rank)| {
            target
                .offset(file, rank)
                .is_some_and(|square| self.piece_at(square) == Some(Piece::new(by, PieceType::Knight)))
        }) {
            return true;
        }

        if KING_OFFSETS.into_iter().any(|(file, rank)| {
            target
                .offset(file, rank)
                .is_some_and(|square| self.piece_at(square) == Some(Piece::new(by, PieceType::King)))
        }) {
            return true;
        }

        self.attacked_on_rays(target, by, &BISHOP_DIRECTIONS, PieceType::Bishop)
            || self.attacked_on_rays(target, by, &ROOK_DIRECTIONS, PieceType::Rook)
    }

    fn attacked_on_rays(&self, target: Square, by: Color, directions: &[(i8, i8)], slider: PieceType) -> bool {
        for &(file_delta, rank_delta) in directions {
            let mut cursor = target;
            while let Some(square) = cursor.offset(file_delta, rank_delta) {
                cursor = square;
                if let Some(piece) = self.piece_at(square) {
                    if piece.color == by && (piece.kind == slider || piece.kind == PieceType::Queen) {
                        return true;
                    }
                    break;
                }
            }
        }
        false
    }

    fn pseudo_legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);
        let side = self.side_to_move();
        self.generate_pawns(side, &mut moves);
        self.generate_leapers(side, PieceType::Knight, &KNIGHT_OFFSETS, &mut moves);
        self.generate_sliders(side, PieceType::Bishop, &BISHOP_DIRECTIONS, &mut moves);
        self.generate_sliders(side, PieceType::Rook, &ROOK_DIRECTIONS, &mut moves);
        self.generate_sliders(side, PieceType::Queen, &BISHOP_DIRECTIONS, &mut moves);
        self.generate_sliders(side, PieceType::Queen, &ROOK_DIRECTIONS, &mut moves);
        self.generate_leapers(side, PieceType::King, &KING_OFFSETS, &mut moves);
        self.generate_castling(side, &mut moves);
        moves
    }

    fn generate_pawns(&self, side: Color, moves: &mut Vec<Move>) {
        for from in BitboardIter(self.colored_pieces(side, PieceType::Pawn)) {
            let rank_delta = side.pawn_push() / 8;
            if let Some(to) = from.offset(0, rank_delta)
                && self.piece_at(to).is_none()
            {
                self.push_pawn_move(from, to, side, moves);
                let start_rank = if side == Color::White { 1 } else { 6 };
                if from.rank() == start_rank
                    && let Some(double_to) = from.offset(0, rank_delta * 2)
                    && self.piece_at(double_to).is_none()
                {
                    moves.push(Move::new(from, double_to));
                }
            }

            for file_delta in [-1, 1] {
                let Some(to) = from.offset(file_delta, rank_delta) else {
                    continue;
                };
                if self
                    .piece_at(to)
                    .is_some_and(|piece| piece.color != side && piece.kind != PieceType::King)
                {
                    self.push_pawn_move(from, to, side, moves);
                } else if Some(to) == self.en_passant() {
                    moves.push(Move::new_en_passant(from, to));
                }
            }
        }
    }

    fn push_pawn_move(&self, from: Square, to: Square, side: Color, moves: &mut Vec<Move>) {
        let promotion_rank = if side == Color::White { 7 } else { 0 };
        if to.rank() == promotion_rank {
            moves.extend(
                PieceType::PROMOTIONS
                    .into_iter()
                    .map(|piece| Move::new_promotion(from, to, piece)),
            );
        } else {
            moves.push(Move::new(from, to));
        }
    }

    fn generate_leapers(&self, side: Color, kind: PieceType, offsets: &[(i8, i8)], moves: &mut Vec<Move>) {
        for from in BitboardIter(self.colored_pieces(side, kind)) {
            for &(file_delta, rank_delta) in offsets {
                let Some(to) = from.offset(file_delta, rank_delta) else {
                    continue;
                };
                if self
                    .piece_at(to)
                    .is_none_or(|piece| piece.color != side && piece.kind != PieceType::King)
                {
                    moves.push(Move::new(from, to));
                }
            }
        }
    }

    fn generate_sliders(&self, side: Color, kind: PieceType, directions: &[(i8, i8)], moves: &mut Vec<Move>) {
        for from in BitboardIter(self.colored_pieces(side, kind)) {
            for &(file_delta, rank_delta) in directions {
                let mut cursor = from;
                while let Some(to) = cursor.offset(file_delta, rank_delta) {
                    cursor = to;
                    match self.piece_at(to) {
                        None => moves.push(Move::new(from, to)),
                        Some(piece) if piece.color != side && piece.kind != PieceType::King => {
                            moves.push(Move::new(from, to));
                            break;
                        }
                        Some(_) => break,
                    }
                }
            }
        }
    }

    fn generate_castling(&self, side: Color, moves: &mut Vec<Move>) {
        let (king_from, kingside_rook, queenside_rook, rank) = match side {
            Color::White => (Square::E1, Square::H1, Square::A1, 0),
            Color::Black => (Square::E8, Square::H8, Square::A8, 7),
        };
        if self.piece_at(king_from) != Some(Piece::new(side, PieceType::King)) || self.is_in_check(side) {
            return;
        }

        if self.castling_rights().has_kingside(side)
            && self.piece_at(kingside_rook) == Some(Piece::new(side, PieceType::Rook))
        {
            let f = Square::from_file_rank(5, rank).expect("castling square");
            let g = Square::from_file_rank(6, rank).expect("castling square");
            if self.piece_at(f).is_none()
                && self.piece_at(g).is_none()
                && !self.is_square_attacked(f, !side)
                && !self.is_square_attacked(g, !side)
            {
                moves.push(Move::new_castling(king_from, kingside_rook));
            }
        }

        if self.castling_rights().has_queenside(side)
            && self.piece_at(queenside_rook) == Some(Piece::new(side, PieceType::Rook))
        {
            let b = Square::from_file_rank(1, rank).expect("castling square");
            let c = Square::from_file_rank(2, rank).expect("castling square");
            let d = Square::from_file_rank(3, rank).expect("castling square");
            if self.piece_at(b).is_none()
                && self.piece_at(c).is_none()
                && self.piece_at(d).is_none()
                && !self.is_square_attacked(d, !side)
                && !self.is_square_attacked(c, !side)
            {
                moves.push(Move::new_castling(king_from, queenside_rook));
            }
        }
    }
}
