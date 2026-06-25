use std::fmt;

use crate::{
    evaluation::{self, EvalState},
    types::{Bitboard, BitboardIter, CastlingRights, Color, Piece, PieceOnSquare, PieceType, Square},
};

mod fen;
mod makemove;
mod movegen;
mod zobrist;

#[cfg(test)]
mod tests;

pub use fen::FenError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PositionState {
    side_to_move: Color,
    castling: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u16,
    fullmove_number: u16,
    hash: u64,
}

impl Default for PositionState {
    fn default() -> Self {
        Self {
            side_to_move: Color::White,
            castling: CastlingRights::NONE,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            hash: 0,
        }
    }
}

/// Complete chess position state, excluding search-specific data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    pieces: [Bitboard; 6],
    colors: [Bitboard; 2],
    mailbox: [Option<Piece>; 64],
    state: PositionState,
    eval: EvalState,
    hash_history: Vec<u64>,
}

impl Default for Board {
    fn default() -> Self {
        Self::starting_position()
    }
}

impl Board {
    pub const STARTING_FEN: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[must_use]
    pub fn starting_position() -> Self {
        Self::from_fen(Self::STARTING_FEN).expect("the built-in starting FEN is valid")
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            pieces: [0; 6],
            colors: [0; 2],
            mailbox: [None; 64],
            state: PositionState::default(),
            eval: EvalState::default(),
            hash_history: Vec::new(),
        }
    }

    #[must_use]
    pub const fn side_to_move(&self) -> Color {
        self.state.side_to_move
    }

    #[must_use]
    pub const fn castling_rights(&self) -> CastlingRights {
        self.state.castling
    }

    #[must_use]
    pub const fn en_passant(&self) -> Option<Square> {
        self.state.en_passant
    }

    #[must_use]
    pub const fn halfmove_clock(&self) -> u16 {
        self.state.halfmove_clock
    }

    #[must_use]
    pub const fn fullmove_number(&self) -> u16 {
        self.state.fullmove_number
    }

    #[must_use]
    pub const fn hash(&self) -> u64 {
        self.state.hash
    }

    #[must_use]
    pub const fn piece_at(&self, square: Square) -> Option<Piece> {
        self.mailbox[square.index()]
    }

    #[must_use]
    pub const fn pieces(&self, kind: PieceType) -> Bitboard {
        self.pieces[kind.index()]
    }

    #[must_use]
    pub const fn colors(&self, color: Color) -> Bitboard {
        self.colors[color.index()]
    }

    #[must_use]
    pub const fn occupied(&self) -> Bitboard {
        self.colors[Color::White.index()] | self.colors[Color::Black.index()]
    }

    #[must_use]
    pub const fn colored_pieces(&self, color: Color, kind: PieceType) -> Bitboard {
        self.colors(color) & self.pieces(kind)
    }

    #[must_use]
    pub fn occupied_squares(&self) -> BitboardIter {
        BitboardIter(self.occupied())
    }

    #[must_use]
    pub fn king_square(&self, color: Color) -> Option<Square> {
        let kings = self.colored_pieces(color, PieceType::King);
        if kings.count_ones() == 1 {
            Square::new(kings.trailing_zeros() as u8)
        } else {
            None
        }
    }

    #[must_use]
    pub(crate) const fn core_eval(&self) -> EvalState {
        self.eval
    }

    fn add_piece_raw(&mut self, piece: Piece, square: Square) {
        debug_assert!(self.mailbox[square.index()].is_none());
        self.mailbox[square.index()] = Some(piece);
        self.pieces[piece.kind.index()] |= square.bit();
        self.colors[piece.color.index()] |= square.bit();
        self.state.hash ^= zobrist::piece(piece, square);
        self.eval.add(evaluation::piece_core_eval(piece, square));
    }

    fn remove_piece_raw(&mut self, square: Square) -> Option<Piece> {
        let piece = self.mailbox[square.index()].take()?;
        self.pieces[piece.kind.index()] &= !square.bit();
        self.colors[piece.color.index()] &= !square.bit();
        self.state.hash ^= zobrist::piece(piece, square);
        self.eval.subtract(evaluation::piece_core_eval(piece, square));
        Some(piece)
    }

    fn ep_hash(&self) -> u64 {
        let Some(target) = self.state.en_passant else {
            return 0;
        };
        let source_rank_delta = match self.state.side_to_move {
            Color::White => -1,
            Color::Black => 1,
        };
        let capturable = [-1, 1].into_iter().any(|file_delta| {
            target.offset(file_delta, source_rank_delta).is_some_and(|source| {
                self.piece_at(source) == Some(Piece::new(self.state.side_to_move, PieceType::Pawn))
            })
        });
        if capturable {
            zobrist::en_passant_file(target.file())
        } else {
            0
        }
    }

    fn recompute_hash(&self) -> u64 {
        let mut hash = 0;
        for square in self.occupied_squares() {
            hash ^= zobrist::piece(self.piece_at(square).expect("occupied square contains a piece"), square);
        }
        if self.side_to_move() == Color::Black {
            hash ^= zobrist::side();
        }
        hash ^= zobrist::castling(self.castling_rights().bits());
        hash ^ self.ep_hash()
    }

    pub(crate) fn reset_history_and_hash(&mut self) {
        self.state.hash = self.recompute_hash();
        self.eval = evaluation::recompute_core_eval(self);
        self.hash_history.clear();
        self.hash_history.push(self.state.hash);
    }

    #[must_use]
    pub fn is_threefold_repetition(&self) -> bool {
        let reversible = usize::from(self.halfmove_clock()).saturating_add(1);
        self.hash_history
            .iter()
            .rev()
            .take(reversible)
            .filter(|&&hash| hash == self.hash())
            .count()
            >= 3
    }

    #[must_use]
    pub const fn is_fifty_move_draw(&self) -> bool {
        self.state.halfmove_clock >= 100
    }

    #[must_use]
    pub fn is_insufficient_material(&self) -> bool {
        if self.pieces(PieceType::Pawn) | self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen) != 0 {
            return false;
        }

        let knights = self.pieces(PieceType::Knight).count_ones();
        let bishops = self.pieces(PieceType::Bishop);
        let bishop_count = bishops.count_ones();
        if knights + bishop_count <= 1 {
            return true;
        }
        if knights != 0 {
            return false;
        }

        let light_squares = 0x55aa_55aa_55aa_55aa_u64;
        bishops & light_squares == 0 || bishops & !light_squares == 0
    }

    #[must_use]
    pub fn game_status(&self) -> GameStatus {
        let moves = self.legal_moves();
        if moves.is_empty() {
            return if self.is_in_check(self.side_to_move()) {
                GameStatus::Checkmate {
                    winner: !self.side_to_move(),
                }
            } else {
                GameStatus::Stalemate
            };
        }
        if self.is_fifty_move_draw() {
            return GameStatus::DrawFiftyMove;
        }
        if self.is_threefold_repetition() {
            return GameStatus::DrawThreefold;
        }
        if self.is_insufficient_material() {
            return GameStatus::DrawInsufficientMaterial;
        }
        GameStatus::Ongoing
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        let mut pieces = [0; 6];
        let mut colors = [0; 2];
        for (index, entry) in self.mailbox.iter().enumerate() {
            if let Some(piece) = entry {
                let bit = 1_u64 << index;
                pieces[piece.kind.index()] |= bit;
                colors[piece.color.index()] |= bit;
            }
        }
        pieces == self.pieces
            && colors == self.colors
            && colors[0] & colors[1] == 0
            && self.king_square(Color::White).is_some()
            && self.king_square(Color::Black).is_some()
            && self.recompute_hash() == self.hash()
            && evaluation::recompute_core_eval(self) == self.core_eval()
            && self.hash_history.last().copied() == Some(self.hash())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PieceDelta {
    removed: [Option<PieceOnSquare>; 2],
    added: [Option<PieceOnSquare>; 2],
    removed_len: u8,
    added_len: u8,
    old_kings: [Square; 2],
    new_kings: [Square; 2],
}

impl PieceDelta {
    fn new(old_kings: [Square; 2]) -> Self {
        Self {
            removed: [None; 2],
            added: [None; 2],
            removed_len: 0,
            added_len: 0,
            old_kings,
            new_kings: old_kings,
        }
    }

    fn remove(&mut self, piece: Piece, square: Square) {
        let index = usize::from(self.removed_len);
        assert!(index < self.removed.len(), "a chess move removes at most two pieces");
        self.removed[index] = Some(PieceOnSquare::new(piece, square));
        self.removed_len += 1;
    }

    fn add(&mut self, piece: Piece, square: Square) {
        let index = usize::from(self.added_len);
        assert!(index < self.added.len(), "a chess move adds at most two pieces");
        self.added[index] = Some(PieceOnSquare::new(piece, square));
        self.added_len += 1;
        if piece.kind == PieceType::King {
            self.new_kings[piece.color.index()] = square;
        }
    }

    pub fn removed(&self) -> impl Iterator<Item = PieceOnSquare> + '_ {
        self.removed.iter().copied().flatten()
    }

    pub fn added(&self) -> impl Iterator<Item = PieceOnSquare> + '_ {
        self.added.iter().copied().flatten()
    }

    #[must_use]
    pub const fn old_king(&self, color: Color) -> Square {
        self.old_kings[color.index()]
    }

    #[must_use]
    pub const fn new_king(&self, color: Color) -> Square {
        self.new_kings[color.index()]
    }

    #[must_use]
    pub const fn king_moved(&self, color: Color) -> bool {
        self.old_king(color).raw() != self.new_king(color).raw()
    }
}

#[derive(Clone, Debug)]
pub struct Undo {
    previous: PositionState,
    previous_history_len: usize,
    delta: PieceDelta,
}

impl Undo {
    #[must_use]
    pub const fn delta(&self) -> &PieceDelta {
        &self.delta
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    Ongoing,
    Checkmate { winner: Color },
    Stalemate,
    DrawFiftyMove,
    DrawThreefold,
    DrawInsufficientMaterial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    MissingPiece,
    WrongSide,
    OccupiedByFriendlyPiece,
    InvalidSpecialMove,
    IllegalMove,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingPiece => "move source is empty",
            Self::WrongSide => "move source contains the wrong side's piece",
            Self::OccupiedByFriendlyPiece => "move target contains a friendly piece",
            Self::InvalidSpecialMove => "special move does not match the position",
            Self::IllegalMove => "move is not legal in this position",
        })
    }
}

impl std::error::Error for MoveError {}
