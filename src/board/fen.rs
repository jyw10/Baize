use std::fmt;

use crate::{
    board::Board,
    types::{CastlingRights, Color, Piece, PieceType, Square},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FenError(String);

impl FenError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FenError {}

impl Board {
    pub fn from_fen(fen: &str) -> Result<Self, FenError> {
        let fields = fen.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() != 6 {
            return Err(FenError::new("FEN must contain six fields"));
        }

        let mut board = Self::empty();
        let ranks = fields[0].split('/').collect::<Vec<_>>();
        if ranks.len() != 8 {
            return Err(FenError::new("piece placement must contain eight ranks"));
        }

        for (fen_rank, rank_text) in ranks.iter().enumerate() {
            let rank = 7 - fen_rank as u8;
            let mut file = 0_u8;
            for ch in rank_text.chars() {
                if let Some(empty) = ch.to_digit(10) {
                    if empty == 0 || empty > 8 {
                        return Err(FenError::new("empty-square run must be between 1 and 8"));
                    }
                    file = file
                        .checked_add(empty as u8)
                        .ok_or_else(|| FenError::new("rank contains too many squares"))?;
                    continue;
                }

                let kind = PieceType::from_fen(ch).ok_or_else(|| FenError::new("invalid piece character"))?;
                if file >= 8 {
                    return Err(FenError::new("rank contains too many squares"));
                }
                let color = if ch.is_ascii_uppercase() {
                    Color::White
                } else {
                    Color::Black
                };
                let square = Square::from_file_rank(file, rank).expect("validated file and rank");
                board.add_piece_raw(Piece::new(color, kind), square);
                file += 1;
            }
            if file != 8 {
                return Err(FenError::new("every rank must contain eight squares"));
            }
        }

        board.state.side_to_move = match fields[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(FenError::new("active color must be 'w' or 'b'")),
        };

        let mut rights = 0;
        if fields[2] != "-" {
            for ch in fields[2].chars() {
                let bit = match ch {
                    'K' => CastlingRights::WHITE_KINGSIDE,
                    'Q' => CastlingRights::WHITE_QUEENSIDE,
                    'k' => CastlingRights::BLACK_KINGSIDE,
                    'q' => CastlingRights::BLACK_QUEENSIDE,
                    _ => return Err(FenError::new("invalid castling rights")),
                };
                if rights & bit != 0 {
                    return Err(FenError::new("duplicate castling right"));
                }
                rights |= bit;
            }
        }
        board.state.castling = CastlingRights::new(rights).expect("parsed castling mask is valid");

        board.state.en_passant = if fields[3] == "-" {
            None
        } else {
            let square = fields[3]
                .parse::<Square>()
                .map_err(|_| FenError::new("invalid en-passant square"))?;
            let required_rank = if board.side_to_move() == Color::White { 5 } else { 2 };
            if square.rank() != required_rank || board.piece_at(square).is_some() {
                return Err(FenError::new("en-passant target is inconsistent with the active color"));
            }
            Some(square)
        };

        board.state.halfmove_clock = fields[4].parse().map_err(|_| FenError::new("invalid halfmove clock"))?;
        board.state.fullmove_number = fields[5]
            .parse()
            .map_err(|_| FenError::new("invalid fullmove number"))?;
        if board.state.fullmove_number == 0 {
            return Err(FenError::new("fullmove number must be positive"));
        }

        if board.colored_pieces(Color::White, PieceType::King).count_ones() != 1
            || board.colored_pieces(Color::Black, PieceType::King).count_ones() != 1
        {
            return Err(FenError::new("position must contain exactly one king per side"));
        }
        if board.pieces(PieceType::Pawn) & (0xff | (0xff_u64 << 56)) != 0 {
            return Err(FenError::new("pawns cannot be placed on the first or eighth rank"));
        }

        board.reset_history_and_hash();
        Ok(board)
    }

    #[must_use]
    pub fn to_fen(&self) -> String {
        let mut placement = String::new();
        for rank in (0..8).rev() {
            if rank != 7 {
                placement.push('/');
            }
            let mut empty = 0;
            for file in 0..8 {
                let square = Square::from_file_rank(file, rank).expect("loop values form a square");
                if let Some(piece) = self.piece_at(square) {
                    if empty != 0 {
                        placement.push(char::from(b'0' + empty));
                        empty = 0;
                    }
                    placement.push(piece.kind.fen(piece.color));
                } else {
                    empty += 1;
                }
            }
            if empty != 0 {
                placement.push(char::from(b'0' + empty));
            }
        }

        format!(
            "{} {} {} {} {} {}",
            placement,
            if self.side_to_move() == Color::White { "w" } else { "b" },
            self.castling_rights(),
            self.en_passant()
                .map_or_else(|| "-".to_owned(), |square| square.to_string()),
            self.halfmove_clock(),
            self.fullmove_number()
        )
    }
}
