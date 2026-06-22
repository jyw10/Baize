use crate::types::Square;

pub type Bitboard = u64;

#[derive(Clone, Copy, Debug)]
pub struct BitboardIter(pub(crate) Bitboard);

impl Iterator for BitboardIter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }

        let index = self.0.trailing_zeros() as u8;
        self.0 &= self.0 - 1;
        Some(Square::new(index).expect("trailing zero index is a square"))
    }
}
