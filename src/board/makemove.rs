use crate::{
    board::{Board, MoveError, PieceDelta, Undo, zobrist},
    types::{Color, Move, MoveKind, Piece, PieceType, Square},
};

impl Board {
    /// Validates and plays a legal move.
    pub fn make_move(&mut self, mv: Move) -> Result<Undo, MoveError> {
        if !self.legal_moves().contains(&mv) {
            return Err(MoveError::IllegalMove);
        }
        self.make_move_unchecked(mv)
    }

    pub(crate) fn make_move_unchecked(&mut self, mv: Move) -> Result<Undo, MoveError> {
        let from = mv.from();
        let encoded_to = mv.to();
        let mover = self.piece_at(from).ok_or(MoveError::MissingPiece)?;
        if mover.color != self.side_to_move() {
            return Err(MoveError::WrongSide);
        }
        if mv.kind() != MoveKind::Castling
            && self
                .piece_at(encoded_to)
                .is_some_and(|piece| piece.color == mover.color)
        {
            return Err(MoveError::OccupiedByFriendlyPiece);
        }
        self.validate_special_move(mv, mover)?;

        let previous = self.state;
        let previous_history_len = self.hash_history.len();
        let old_kings = [
            self.king_square(Color::White).expect("legal board has a white king"),
            self.king_square(Color::Black).expect("legal board has a black king"),
        ];
        let mut delta = PieceDelta::new(old_kings);

        self.state.hash ^= zobrist::castling(self.state.castling.bits());
        self.state.hash ^= self.ep_hash();
        self.state.en_passant = None;

        let mut was_capture = false;
        match mv.kind() {
            MoveKind::Normal => {
                if let Some(captured) = self.remove_piece_raw(encoded_to) {
                    delta.remove(captured, encoded_to);
                    was_capture = true;
                }
                self.move_piece_with_delta(mover, from, encoded_to, &mut delta);

                if mover.kind == PieceType::Pawn && from.rank().abs_diff(encoded_to.rank()) == 2 {
                    self.state.en_passant = from.offset(0, mover.color.pawn_push() / 8);
                }
            }
            MoveKind::EnPassant => {
                let captured_square = encoded_to
                    .offset(0, -mover.color.pawn_push() / 8)
                    .expect("validated en-passant target has a capture square");
                let captured = self
                    .remove_piece_raw(captured_square)
                    .expect("validated en-passant target contains a pawn");
                delta.remove(captured, captured_square);
                was_capture = true;
                self.move_piece_with_delta(mover, from, encoded_to, &mut delta);
            }
            MoveKind::Promotion => {
                if let Some(captured) = self.remove_piece_raw(encoded_to) {
                    delta.remove(captured, encoded_to);
                    was_capture = true;
                }
                let removed = self
                    .remove_piece_raw(from)
                    .expect("validated mover remains on its source square");
                delta.remove(removed, from);
                let promoted = Piece::new(
                    mover.color,
                    mv.promotion().expect("promotion move has a promotion piece"),
                );
                self.add_piece_raw(promoted, encoded_to);
                delta.add(promoted, encoded_to);
            }
            MoveKind::Castling => {
                let kingside = encoded_to.file() > from.file();
                let rank = from.rank();
                let king_to =
                    Square::from_file_rank(if kingside { 6 } else { 2 }, rank).expect("castling target is valid");
                let rook_to =
                    Square::from_file_rank(if kingside { 5 } else { 3 }, rank).expect("castling target is valid");

                let removed_king = self
                    .remove_piece_raw(from)
                    .expect("validated king remains on its source square");
                let removed_rook = self
                    .remove_piece_raw(encoded_to)
                    .expect("validated rook remains on its source square");
                delta.remove(removed_king, from);
                delta.remove(removed_rook, encoded_to);
                self.add_piece_raw(removed_king, king_to);
                self.add_piece_raw(removed_rook, rook_to);
                delta.add(removed_king, king_to);
                delta.add(removed_rook, rook_to);
            }
        }

        self.update_castling_rights(mover, from, encoded_to);
        self.state.halfmove_clock = if mover.kind == PieceType::Pawn || was_capture {
            0
        } else {
            self.state.halfmove_clock.saturating_add(1)
        };
        if mover.color == Color::Black {
            self.state.fullmove_number = self.state.fullmove_number.saturating_add(1);
        }
        self.state.side_to_move = !self.state.side_to_move;
        self.state.hash ^= zobrist::side();
        self.state.hash ^= zobrist::castling(self.state.castling.bits());
        self.state.hash ^= self.ep_hash();
        self.hash_history.push(self.state.hash);

        debug_assert!(self.validate());
        Ok(Undo {
            previous,
            previous_history_len,
            delta,
        })
    }

    pub(crate) fn make_null_move(&mut self) -> Undo {
        debug_assert!(!self.is_in_check(self.side_to_move()));
        let previous = self.state;
        let previous_history_len = self.hash_history.len();
        let old_kings = [
            self.king_square(Color::White).expect("legal board has a white king"),
            self.king_square(Color::Black).expect("legal board has a black king"),
        ];

        self.state.hash ^= self.ep_hash();
        self.state.en_passant = None;
        self.state.side_to_move = !self.state.side_to_move;
        self.state.hash ^= zobrist::side();
        self.hash_history.push(self.state.hash);

        debug_assert!(self.validate());
        Undo {
            previous,
            previous_history_len,
            delta: PieceDelta::new(old_kings),
        }
    }

    pub fn unmake_move(&mut self, undo: Undo) {
        for entry in undo.delta.added() {
            let removed = self.remove_piece_raw(entry.square);
            debug_assert_eq!(removed, Some(entry.piece));
        }
        for entry in undo.delta.removed() {
            self.add_piece_raw(entry.piece, entry.square);
        }
        self.state = undo.previous;
        self.hash_history.truncate(undo.previous_history_len);
        debug_assert_eq!(self.hash_history.last().copied(), Some(self.state.hash));
        debug_assert!(self.validate());
    }

    fn move_piece_with_delta(&mut self, piece: Piece, from: Square, to: Square, delta: &mut PieceDelta) {
        let removed = self
            .remove_piece_raw(from)
            .expect("validated mover remains on its source square");
        debug_assert_eq!(piece, removed);
        delta.remove(removed, from);
        self.add_piece_raw(removed, to);
        delta.add(removed, to);
    }

    fn validate_special_move(&self, mv: Move, mover: Piece) -> Result<(), MoveError> {
        match mv.kind() {
            MoveKind::Normal => Ok(()),
            MoveKind::EnPassant => {
                let target = mv.to();
                let captured_square = target
                    .offset(0, -mover.color.pawn_push() / 8)
                    .ok_or(MoveError::InvalidSpecialMove)?;
                if mover.kind == PieceType::Pawn
                    && Some(target) == self.en_passant()
                    && self.piece_at(target).is_none()
                    && self.piece_at(captured_square) == Some(Piece::new(!mover.color, PieceType::Pawn))
                {
                    Ok(())
                } else {
                    Err(MoveError::InvalidSpecialMove)
                }
            }
            MoveKind::Promotion => {
                if mover.kind == PieceType::Pawn
                    && matches!((mover.color, mv.to().rank()), (Color::White, 7) | (Color::Black, 0))
                    && mv.promotion().is_some()
                {
                    Ok(())
                } else {
                    Err(MoveError::InvalidSpecialMove)
                }
            }
            MoveKind::Castling => {
                if mover.kind == PieceType::King
                    && self.piece_at(mv.to()) == Some(Piece::new(mover.color, PieceType::Rook))
                {
                    Ok(())
                } else {
                    Err(MoveError::InvalidSpecialMove)
                }
            }
        }
    }

    fn update_castling_rights(&mut self, mover: Piece, from: Square, captured_or_to: Square) {
        if mover.kind == PieceType::King {
            self.state.castling.remove_color(mover.color);
        }
        Self::remove_rook_right(&mut self.state.castling, from);
        Self::remove_rook_right(&mut self.state.castling, captured_or_to);
    }

    fn remove_rook_right(rights: &mut crate::types::CastlingRights, square: Square) {
        match square {
            Square::A1 => rights.remove_queenside(Color::White),
            Square::H1 => rights.remove_kingside(Color::White),
            Square::A8 => rights.remove_queenside(Color::Black),
            Square::H8 => rights.remove_kingside(Color::Black),
            _ => {}
        }
    }
}
