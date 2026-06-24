use crate::{Board, Color, GameStatus, Move, MoveKind, Piece, PieceType, Square, tools::perft};

#[test]
fn fen_round_trip() {
    let fens = [
        Board::STARTING_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "4k3/8/8/3pP3/8/8/8/4K3 w - d6 17 42",
    ];
    for fen in fens {
        let board = Board::from_fen(fen).unwrap();
        assert_eq!(board.to_fen(), fen);
        assert!(board.validate());
    }
}

#[test]
fn rejects_invalid_fens() {
    for fen in [
        "8/8/8/8/8/8/8/8 w - - 0 1",
        "4k3/8/8/8/8/8/8/4K3 x - - 0 1",
        "4k3/8/8/8/8/8/8/4K3 w - e4 0 1",
        "4k3/8/8/8/8/8/8/P3K3 w - - 0 1",
    ] {
        assert!(Board::from_fen(fen).is_err(), "accepted invalid FEN: {fen}");
    }
}

#[test]
fn starting_position_perft() {
    let mut board = Board::default();
    let expected = [1, 20, 400, 8_902, 197_281];
    for (depth, nodes) in expected.into_iter().enumerate() {
        assert_eq!(perft(&mut board, depth as u8), nodes, "depth {depth}");
    }
}

#[test]
fn kiwipete_perft() {
    let mut board = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    let expected = [1, 48, 2_039, 97_862];
    for (depth, nodes) in expected.into_iter().enumerate() {
        assert_eq!(perft(&mut board, depth as u8), nodes, "depth {depth}");
    }
}

#[test]
fn position_three_perft() {
    let mut board = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
    let expected = [1, 14, 191, 2_812, 43_238];
    for (depth, nodes) in expected.into_iter().enumerate() {
        assert_eq!(perft(&mut board, depth as u8), nodes, "depth {depth}");
    }
}

#[test]
fn promotion_and_check_evasion_perft_positions() {
    let positions = [
        (
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            [1, 44, 1_486, 62_379],
        ),
        (
            "r4rk1/1pp1qppp/p1np1n2/4p3/2B1P1b1/1PN2N2/P1PP1PPP/R1BQR1K1 w - - 0 10",
            [1, 34, 1_334, 45_514],
        ),
    ];

    for (fen, expected) in positions {
        let mut board = Board::from_fen(fen).unwrap();
        for (depth, nodes) in expected.into_iter().enumerate() {
            assert_eq!(perft(&mut board, depth as u8), nodes, "{fen} at depth {depth}");
        }
    }
}

#[test]
fn every_starting_move_round_trips_state() {
    let original = Board::default();
    let mut board = original.clone();
    for mv in original.legal_moves() {
        let undo = board.make_move(mv).unwrap();
        assert!(board.validate());
        board.unmake_move(undo);
        assert_eq!(board, original, "failed to restore after {mv}");
    }
}

#[test]
fn null_move_round_trips_state_and_clears_en_passant() {
    let original = Board::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 17 42").unwrap();
    let mut board = original.clone();

    let undo = board.make_null_move();

    assert!(board.validate());
    assert_eq!(board.side_to_move(), Color::Black);
    assert_eq!(board.en_passant(), None);
    assert_eq!(undo.delta().removed().count(), 0);
    assert_eq!(undo.delta().added().count(), 0);

    board.unmake_move(undo);
    assert_eq!(board, original);
}

#[test]
fn castling_uses_king_takes_rook_encoding_and_delta() {
    let original = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
    let kingside = Move::new_castling(Square::E1, Square::H1);
    let queenside = Move::new_castling(Square::E1, Square::A1);
    assert!(original.legal_moves().contains(&kingside));
    assert!(original.legal_moves().contains(&queenside));

    let mut board = original.clone();
    let undo = board.make_move(kingside).unwrap();
    assert_eq!(
        board.piece_at(Square::G1),
        Some(Piece::new(Color::White, PieceType::King))
    );
    assert_eq!(
        board.piece_at(Square::F1),
        Some(Piece::new(Color::White, PieceType::Rook))
    );
    assert_eq!(undo.delta().removed().count(), 2);
    assert_eq!(undo.delta().added().count(), 2);
    assert!(undo.delta().king_moved(Color::White));
    board.unmake_move(undo);
    assert_eq!(board, original);
}

#[test]
fn en_passant_and_promotion_deltas_are_complete() {
    let mut ep = Board::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
    let ep_move = ep
        .legal_moves()
        .into_iter()
        .find(|mv| mv.kind() == MoveKind::EnPassant)
        .unwrap();
    let undo = ep.make_move(ep_move).unwrap();
    assert_eq!(undo.delta().removed().count(), 2);
    assert_eq!(undo.delta().added().count(), 1);
    assert_eq!(
        ep.piece_at("d6".parse().unwrap()),
        Some(Piece::new(Color::White, PieceType::Pawn))
    );
    assert!(ep.piece_at("d5".parse().unwrap()).is_none());
    ep.unmake_move(undo);

    let mut promotion = Board::from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    let promotions = promotion
        .legal_moves()
        .into_iter()
        .filter(|mv| mv.kind() == MoveKind::Promotion)
        .collect::<Vec<_>>();
    assert_eq!(promotions.len(), 4);
    let queen = promotions
        .into_iter()
        .find(|mv| mv.promotion() == Some(PieceType::Queen))
        .unwrap();
    let undo = promotion.make_move(queen).unwrap();
    assert_eq!(undo.delta().removed().count(), 1);
    assert_eq!(undo.delta().added().count(), 1);
    assert_eq!(
        promotion.piece_at(Square::A8),
        Some(Piece::new(Color::White, PieceType::Queen))
    );
    promotion.unmake_move(undo);
}

#[test]
fn deltas_match_full_piece_square_refresh() {
    let mut board = Board::default();
    verify_deltas_recursively(&mut board, 3);
}

#[test]
fn detects_threefold_repetition() {
    let mut board = Board::default();
    let cycle = [
        Move::new("g1".parse().unwrap(), "f3".parse().unwrap()),
        Move::new("g8".parse().unwrap(), "f6".parse().unwrap()),
        Move::new("f3".parse().unwrap(), "g1".parse().unwrap()),
        Move::new("f6".parse().unwrap(), "g8".parse().unwrap()),
    ];
    for _ in 0..2 {
        for mv in cycle {
            board.make_move(mv).unwrap();
        }
    }
    assert!(board.is_threefold_repetition());
    assert_eq!(board.game_status(), GameStatus::DrawThreefold);
}

#[test]
fn recognizes_terminal_positions() {
    let checkmate = Board::from_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(checkmate.game_status(), GameStatus::Checkmate { winner: Color::White });

    let stalemate = Board::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(stalemate.game_status(), GameStatus::Stalemate);

    let bare_kings = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    assert_eq!(bare_kings.game_status(), GameStatus::DrawInsufficientMaterial);
}

fn feature_counts(board: &Board) -> [[u8; 64]; 12] {
    let mut result = [[0; 64]; 12];
    for square in board.occupied_squares() {
        let piece = board.piece_at(square).unwrap();
        result[piece.zobrist_index()][square.index()] += 1;
    }
    result
}

fn verify_deltas_recursively(board: &mut Board, depth: u8) {
    if depth == 0 {
        return;
    }
    let original = board.clone();
    let baseline = feature_counts(board);
    for mv in board.legal_moves() {
        let undo = board.make_move(mv).unwrap();
        let mut incremental = baseline;
        for entry in undo.delta().removed() {
            incremental[entry.piece.zobrist_index()][entry.square.index()] -= 1;
        }
        for entry in undo.delta().added() {
            incremental[entry.piece.zobrist_index()][entry.square.index()] += 1;
        }
        assert_eq!(incremental, feature_counts(board), "bad delta after {mv}");
        verify_deltas_recursively(board, depth - 1);
        board.unmake_move(undo);
        assert_eq!(*board, original, "bad restoration after {mv}");
    }
}
