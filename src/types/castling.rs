use std::fmt;

use crate::types::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CastlingRights(u8);

impl CastlingRights {
    pub const WHITE_KINGSIDE: u8 = 1;
    pub const WHITE_QUEENSIDE: u8 = 2;
    pub const BLACK_KINGSIDE: u8 = 4;
    pub const BLACK_QUEENSIDE: u8 = 8;
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(15);

    #[must_use]
    pub const fn new(bits: u8) -> Option<Self> {
        if bits & !15 == 0 { Some(Self(bits)) } else { None }
    }

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn has_kingside(self, color: Color) -> bool {
        self.0
            & match color {
                Color::White => Self::WHITE_KINGSIDE,
                Color::Black => Self::BLACK_KINGSIDE,
            }
            != 0
    }

    #[must_use]
    pub const fn has_queenside(self, color: Color) -> bool {
        self.0
            & match color {
                Color::White => Self::WHITE_QUEENSIDE,
                Color::Black => Self::BLACK_QUEENSIDE,
            }
            != 0
    }

    pub const fn remove_color(&mut self, color: Color) {
        self.0 &= match color {
            Color::White => !(Self::WHITE_KINGSIDE | Self::WHITE_QUEENSIDE),
            Color::Black => !(Self::BLACK_KINGSIDE | Self::BLACK_QUEENSIDE),
        };
    }

    pub const fn remove_kingside(&mut self, color: Color) {
        self.0 &= !match color {
            Color::White => Self::WHITE_KINGSIDE,
            Color::Black => Self::BLACK_KINGSIDE,
        };
    }

    pub const fn remove_queenside(&mut self, color: Color) {
        self.0 &= !match color {
            Color::White => Self::WHITE_QUEENSIDE,
            Color::Black => Self::BLACK_QUEENSIDE,
        };
    }
}

impl fmt::Display for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::NONE {
            return f.write_str("-");
        }
        if self.has_kingside(Color::White) {
            f.write_str("K")?;
        }
        if self.has_queenside(Color::White) {
            f.write_str("Q")?;
        }
        if self.has_kingside(Color::Black) {
            f.write_str("k")?;
        }
        if self.has_queenside(Color::Black) {
            f.write_str("q")?;
        }
        Ok(())
    }
}
