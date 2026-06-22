use crate::types::{Color, Square};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceType {
    pub const ALL: [Self; 6] = [
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];

    pub const PROMOTIONS: [Self; 4] = [Self::Knight, Self::Bishop, Self::Rook, Self::Queen];

    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[must_use]
    pub const fn fen(self, color: Color) -> char {
        let ch = match self {
            Self::Pawn => 'p',
            Self::Knight => 'n',
            Self::Bishop => 'b',
            Self::Rook => 'r',
            Self::Queen => 'q',
            Self::King => 'k',
        };
        match color {
            Color::White => ch.to_ascii_uppercase(),
            Color::Black => ch,
        }
    }

    #[must_use]
    pub const fn from_fen(ch: char) -> Option<Self> {
        match ch.to_ascii_lowercase() {
            'p' => Some(Self::Pawn),
            'n' => Some(Self::Knight),
            'b' => Some(Self::Bishop),
            'r' => Some(Self::Rook),
            'q' => Some(Self::Queen),
            'k' => Some(Self::King),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceType,
}

impl Piece {
    #[must_use]
    pub const fn new(color: Color, kind: PieceType) -> Self {
        Self { color, kind }
    }

    #[must_use]
    pub const fn zobrist_index(self) -> usize {
        self.color.index() * 6 + self.kind.index()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PieceOnSquare {
    pub piece: Piece,
    pub square: Square,
}

impl PieceOnSquare {
    #[must_use]
    pub const fn new(piece: Piece, square: Square) -> Self {
        Self { piece, square }
    }
}
