//! Baize chess engine core.
//!
//! The core intentionally owns its board representation and move semantics.
//! Search, UCI, datagen, and NNUE layers build on this crate without changing
//! the rules implementation.

pub mod board;
pub mod evaluation;
pub mod search;
pub mod time;
pub mod tools;
pub mod types;
pub mod uci;

pub use board::{Board, FenError, GameStatus, MoveError, PieceDelta, Undo};
pub use types::{CastlingRights, Color, Move, MoveKind, Piece, PieceOnSquare, PieceType, Square};
