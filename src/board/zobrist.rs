use crate::types::{Piece, Square};

const fn mix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

const fn make_piece_keys() -> [[u64; 64]; 12] {
    let mut keys = [[0; 64]; 12];
    let mut piece = 0;
    while piece < 12 {
        let mut square = 0;
        while square < 64 {
            keys[piece][square] = mix64(0x0ba1_2e00_0000_0000 ^ ((piece * 64 + square) as u64));
            square += 1;
        }
        piece += 1;
    }
    keys
}

const fn make_castling_keys() -> [u64; 16] {
    let mut keys = [0; 16];
    let mut index = 0;
    while index < 16 {
        keys[index] = mix64(0x0ba1_2eca_5700_0000 ^ index as u64);
        index += 1;
    }
    keys
}

const fn make_en_passant_keys() -> [u64; 8] {
    let mut keys = [0; 8];
    let mut index = 0;
    while index < 8 {
        keys[index] = mix64(0x0ba1_2eee_0000_0000 ^ index as u64);
        index += 1;
    }
    keys
}

const PIECES: [[u64; 64]; 12] = make_piece_keys();
const CASTLING: [u64; 16] = make_castling_keys();
const EN_PASSANT: [u64; 8] = make_en_passant_keys();
const SIDE: u64 = mix64(0x0ba1_2e51_de00_0000);

pub const fn piece(piece: Piece, square: Square) -> u64 {
    PIECES[piece.zobrist_index()][square.index()]
}

pub const fn castling(rights: u8) -> u64 {
    CASTLING[rights as usize]
}

pub const fn en_passant_file(file: u8) -> u64 {
    EN_PASSANT[file as usize]
}

pub const fn side() -> u64 {
    SIDE
}
