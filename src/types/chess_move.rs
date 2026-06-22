use std::fmt;

use crate::types::{PieceType, Square};

const FROM_MASK: u16 = 0x3f;
const TO_SHIFT: u16 = 6;
const PROMOTION_SHIFT: u16 = 12;
const KIND_SHIFT: u16 = 14;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MoveKind {
    #[default]
    Normal = 0,
    EnPassant = 1,
    Castling = 2,
    Promotion = 3,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Move(u16);

impl Move {
    #[must_use]
    pub const fn new(from: Square, to: Square) -> Self {
        Self(from.raw() as u16 | ((to.raw() as u16) << TO_SHIFT))
    }

    #[must_use]
    pub const fn new_en_passant(from: Square, to: Square) -> Self {
        Self::with_kind(from, to, 0, MoveKind::EnPassant)
    }

    /// Creates an internal castling move. `rook_square` is the rook's source
    /// square, matching the Viriformat king-takes-rook representation.
    #[must_use]
    pub const fn new_castling(king_square: Square, rook_square: Square) -> Self {
        Self::with_kind(king_square, rook_square, 0, MoveKind::Castling)
    }

    #[must_use]
    pub const fn new_promotion(from: Square, to: Square, promotion: PieceType) -> Self {
        let code = match promotion {
            PieceType::Knight => 0,
            PieceType::Bishop => 1,
            PieceType::Rook => 2,
            PieceType::Queen => 3,
            PieceType::Pawn | PieceType::King => panic!("invalid promotion piece"),
        };
        Self::with_kind(from, to, code, MoveKind::Promotion)
    }

    const fn with_kind(from: Square, to: Square, promotion: u16, kind: MoveKind) -> Self {
        Self(
            from.raw() as u16
                | ((to.raw() as u16) << TO_SHIFT)
                | (promotion << PROMOTION_SHIFT)
                | ((kind as u16) << KIND_SHIFT),
        )
    }

    #[must_use]
    pub const fn from_raw(raw: u16) -> Option<Self> {
        if raw == 0 {
            return None;
        }
        let kind = (raw >> KIND_SHIFT) & 3;
        let from = raw & FROM_MASK;
        let to = (raw >> TO_SHIFT) & FROM_MASK;
        if from == to || kind > 3 { None } else { Some(Self(raw)) }
    }

    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    #[must_use]
    pub fn from(self) -> Square {
        Square::new((self.0 & FROM_MASK) as u8).expect("move source is masked to a square")
    }

    #[must_use]
    pub fn to(self) -> Square {
        Square::new(((self.0 >> TO_SHIFT) & FROM_MASK) as u8).expect("move target is masked to a square")
    }

    #[must_use]
    pub const fn kind(self) -> MoveKind {
        match (self.0 >> KIND_SHIFT) & 3 {
            0 => MoveKind::Normal,
            1 => MoveKind::EnPassant,
            2 => MoveKind::Castling,
            _ => MoveKind::Promotion,
        }
    }

    #[must_use]
    pub const fn promotion(self) -> Option<PieceType> {
        if !matches!(self.kind(), MoveKind::Promotion) {
            return None;
        }
        Some(match (self.0 >> PROMOTION_SHIFT) & 3 {
            0 => PieceType::Knight,
            1 => PieceType::Bishop,
            2 => PieceType::Rook,
            _ => PieceType::Queen,
        })
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.from(), self.to())?;
        if let Some(piece) = self.promotion() {
            let suffix = match piece {
                PieceType::Knight => 'n',
                PieceType::Bishop => 'b',
                PieceType::Rook => 'r',
                PieceType::Queen => 'q',
                PieceType::Pawn | PieceType::King => unreachable!(),
            };
            write!(f, "{suffix}")?;
        }
        Ok(())
    }
}
