use crate::bitboard::Bitboard;
use crate::tables::{ATTACK_TABLE, BETWEEN_TABLE};
use crate::{Color, Move, PieceType, Position, Square};

pub struct MoveList(Vec<Move>);

impl MoveList {
    const MAX_LEGAL_MOVES: usize = 593;

    pub fn generate_legals(&mut self, pos: &Position) {
        if pos.in_check() {
            self.generate_evasions(pos);
        } else {
            self.generate_all(pos);
        }
        // 王手放置になってしまう指し手を除外
        let c = pos.side_to_move();
        self.0.retain(|m| {
            if let Some(from) = m.from() {
                // 玉が相手の攻撃範囲内に動いてしまう指し手
                if pos.piece_on(from).piece_type() == Some(PieceType::OU) {
                    return pos.attackers_to(!c, m.to()).is_empty();
                }
                // 飛び駒から守っている駒が直線上から外れてしまう指し手
                if !(pos.pinned() & Bitboard::from_square(from)).is_empty() {
                    let sq = pos.pieces_cp(c, PieceType::OU).pop();
                    return !((BETWEEN_TABLE[sq.index()][from.index()] & m.to()).is_empty()
                        && (BETWEEN_TABLE[sq.index()][m.to().index()] & from).is_empty());
                }
            }
            true
        });
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    fn generate_all(&mut self, pos: &Position) {
        let target = !pos.pieces_c(pos.side_to_move());
        self.generate_for_fu(pos, &target);
        self.generate_for_ky(pos, &target);
        self.generate_for_ke(pos, &target);
        self.generate_for_gi(pos, &target);
        self.generate_for_ka(pos, &target);
        self.generate_for_hi(pos, &target);
        self.generate_for_ki(pos, &target);
        self.generate_for_ou(pos, &target);
        // TODO: drop
    }
    fn generate_evasions(&mut self, pos: &Position) {
        let c = pos.side_to_move();
        let mut checkers_attacks = Bitboard::ZERO;
        let mut checkers_count = 0;
        for sq in pos.checkers() {
            if let Some(pt) = pos.piece_on(sq).piece_type() {
                checkers_attacks |= Self::pseudo_attack(pt, sq);
            }
            checkers_count += 1;
        }
        let sq = pos.pieces_cp(c, PieceType::OU).pop();
        for to in ATTACK_TABLE.ou.attack(sq, !c) & !pos.pieces_c(c) & !checkers_attacks {
            self.push(Move::new_normal(sq, to, false, pos.piece_on(sq)));
        }
        // 両王手の場合は玉が逃げるしかない
        if checkers_count > 1 {
            return;
        }
        let checker = pos.checkers().pop();
        let target = pos.checkers() | BETWEEN_TABLE[checker.index()][sq.index()];
        self.generate_for_fu(pos, &target);
        self.generate_for_ky(pos, &target);
        self.generate_for_ke(pos, &target);
        self.generate_for_gi(pos, &target);
        self.generate_for_ka(pos, &target);
        self.generate_for_hi(pos, &target);
        self.generate_for_ki(pos, &target);
        // TODO: drop
    }
    fn push(&mut self, m: Move) {
        self.0.push(m);
    }
    fn generate_for_fu(&mut self, pos: &Position, target: &Bitboard) {
        let c = pos.side_to_move();
        for from in pos.pieces_cp(c, PieceType::FU) {
            let p_from = pos.piece_on(from);
            for to in ATTACK_TABLE.fu.attack(from, c) & *target {
                // TODO: (force) promote?
                self.push(Move::new_normal(from, to, false, p_from));
            }
        }
    }
    fn generate_for_ky(&mut self, pos: &Position, target: &Bitboard) {
        let c = pos.side_to_move();
        for from in pos.pieces_cp(c, PieceType::KY) {
            let p_from = pos.piece_on(from);
            let occupied = pos.pieces_p(PieceType::OCCUPIED);
            for to in ATTACK_TABLE.ky.attack(from, c, &occupied) & *target {
                // TODO: (force) promote?
                self.push(Move::new_normal(from, to, false, p_from));
            }
        }
    }
    fn generate_for_ke(&mut self, pos: &Position, target: &Bitboard) {
        let c = pos.side_to_move();
        for from in pos.pieces_cp(c, PieceType::KE) {
            let p_from = pos.piece_on(from);
            for to in ATTACK_TABLE.ke.attack(from, c) & *target {
                // TODO: (force) promote?
                self.push(Move::new_normal(from, to, false, p_from));
            }
        }
    }
    fn generate_for_gi(&mut self, pos: &Position, target: &Bitboard) {
        let c = pos.side_to_move();
        for from in pos.pieces_cp(c, PieceType::GI) {
            let p_from = pos.piece_on(from);
            let from_is_opponent_field = from.rank().is_opponent_field(c);
            for to in ATTACK_TABLE.gi.attack(from, c) & *target {
                self.push(Move::new_normal(from, to, false, p_from));
                if from_is_opponent_field || to.rank().is_opponent_field(c) {
                    self.push(Move::new_normal(from, to, true, p_from));
                }
            }
        }
    }
    fn generate_for_ka(&mut self, pos: &Position, target: &Bitboard) {
        let c = pos.side_to_move();
        for from in pos.pieces_cp(c, PieceType::KA) {
            let p_from = pos.piece_on(from);
            let occupied = pos.pieces_p(PieceType::OCCUPIED);
            let from_is_opponent_field = from.rank().is_opponent_field(c);
            for to in ATTACK_TABLE.ka.attack(from, &occupied) & *target {
                self.push(Move::new_normal(from, to, false, p_from));
                if from_is_opponent_field || to.rank().is_opponent_field(c) {
                    self.push(Move::new_normal(from, to, true, p_from));
                }
            }
        }
    }
    fn generate_for_hi(&mut self, pos: &Position, target: &Bitboard) {
        let c = pos.side_to_move();
        for from in pos.pieces_cp(c, PieceType::HI) {
            let p_from = pos.piece_on(from);
            let occupied = pos.pieces_p(PieceType::OCCUPIED);
            let from_is_opponent_field = from.rank().is_opponent_field(c);
            for to in ATTACK_TABLE.hi.attack(from, &occupied) & *target {
                self.push(Move::new_normal(from, to, false, p_from));
                if from_is_opponent_field || to.rank().is_opponent_field(c) {
                    self.push(Move::new_normal(from, to, true, p_from));
                }
            }
        }
    }
    fn generate_for_ki(&mut self, pos: &Position, target: &Bitboard) {
        let color = pos.side_to_move();
        // TODO: promoted pieces
        for from in pos.pieces_cp(color, PieceType::KI) {
            let p_from = pos.piece_on(from);
            for to in ATTACK_TABLE.ki.attack(from, color) & *target {
                self.push(Move::new_normal(from, to, false, p_from));
            }
        }
    }
    fn generate_for_ou(&mut self, pos: &Position, target: &Bitboard) {
        let color = pos.side_to_move();
        // TODO: use king_square?
        if let Some(from) = pos.pieces_cp(color, PieceType::OU).next() {
            let p_from = pos.piece_on(from);
            for to in ATTACK_TABLE.ou.attack(from, color) & *target {
                self.push(Move::new_normal(from, to, false, p_from));
            }
        }
    }
    #[rustfmt::skip]
    fn pseudo_attack(pt: PieceType, sq: Square) -> Bitboard {
        match pt {
            // TODO
            PieceType::KA => ATTACK_TABLE.ka.pseudo_attack(sq),
            PieceType::UM => ATTACK_TABLE.ka.pseudo_attack(sq) | ATTACK_TABLE.ou.attack(sq, Color::Black),
            PieceType::HI => ATTACK_TABLE.hi.pseudo_attack(sq),
            PieceType::RY => ATTACK_TABLE.hi.pseudo_attack(sq) | ATTACK_TABLE.ou.attack(sq, Color::Black),
            _ => Bitboard::ZERO,
        }
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self(Vec::with_capacity(MoveList::MAX_LEGAL_MOVES))
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IntoIterator for MoveList {
    type Item = Move;
    type IntoIter = MoveListIter;
    fn into_iter(self) -> Self::IntoIter {
        MoveListIter {
            moves: self,
            index: 0,
        }
    }
}

pub struct MoveListIter {
    moves: MoveList,
    index: usize,
}

impl Iterator for MoveListIter {
    type Item = Move;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.moves.0.len() {
            let m = self.moves[self.index];
            self.index += 1;
            Some(m)
        } else {
            None
        }
    }
}
