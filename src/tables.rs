use std::cmp::Ordering;

use crate::bitboard::Bitboard;
use crate::square::{File, Rank};
use crate::{Color, Square};
use bitintr::Pext;
use once_cell::sync::Lazy;

#[derive(Clone, Copy)]
struct Delta(pub i8);

impl Delta {
    const N: Delta = Delta(-1);
    const E: Delta = Delta(-9);
    const S: Delta = Delta(1);
    const W: Delta = Delta(9);
    const NE: Delta = Delta(Delta::N.0 + Delta::E.0);
    const SE: Delta = Delta(Delta::S.0 + Delta::E.0);
    const SW: Delta = Delta(Delta::S.0 + Delta::W.0);
    const NW: Delta = Delta(Delta::N.0 + Delta::W.0);
    const NNE: Delta = Delta(Delta::N.0 + Delta::NE.0);
    const NNW: Delta = Delta(Delta::N.0 + Delta::NW.0);
    const SSE: Delta = Delta(Delta::S.0 + Delta::SE.0);
    const SSW: Delta = Delta(Delta::S.0 + Delta::SW.0);
}

impl std::ops::Add<Delta> for Square {
    type Output = Option<Square>;

    fn add(self, rhs: Delta) -> Self::Output {
        let i = self.0 + rhs.0;
        if 0 <= i && i < Square::NUM as i8 {
            Some(Square(i))
        } else {
            None
        }
    }
}

pub struct PieceAttackTable([[Bitboard; Color::NUM]; Square::NUM]);

impl PieceAttackTable {
    #[rustfmt::skip]    const BFU_DELTAS: &'static [Delta] = &[Delta::N];
    #[rustfmt::skip]    const WFU_DELTAS: &'static [Delta] = &[Delta::S];
    #[rustfmt::skip]    const BKE_DELTAS: &'static [Delta] = &[Delta::NNE, Delta::NNW];
    #[rustfmt::skip]    const WKE_DELTAS: &'static [Delta] = &[Delta::SSE, Delta::SSW];
    #[rustfmt::skip]    const BGI_DELTAS: &'static [Delta] = &[Delta::N, Delta::NE, Delta::SE, Delta::SW, Delta::NW];
    #[rustfmt::skip]    const WGI_DELTAS: &'static [Delta] = &[Delta::S, Delta::NE, Delta::SE, Delta::SW, Delta::NW];
    #[rustfmt::skip]    const BKI_DELTAS: &'static [Delta] = &[Delta::N, Delta::E, Delta::S, Delta::W, Delta::NE, Delta::NW];
    #[rustfmt::skip]    const WKI_DELTAS: &'static [Delta] = &[Delta::N, Delta::E, Delta::S, Delta::W, Delta::SE, Delta::SW];
    #[rustfmt::skip]    const BOU_DELTAS: &'static [Delta] = &[Delta::N, Delta::E, Delta::S, Delta::W, Delta::NE, Delta::SE, Delta::SW, Delta::NW];
    #[rustfmt::skip]    const WOU_DELTAS: &'static [Delta] = &[Delta::N, Delta::E, Delta::S, Delta::W, Delta::NE, Delta::SE, Delta::SW, Delta::NW];

    fn new(deltas: &[&[Delta]; Color::NUM]) -> Self {
        let mut table = [[Bitboard::ZERO; Color::NUM]; Square::NUM];
        for sq in Square::ALL {
            for color in Color::ALL {
                for &delta in deltas[color.index()] {
                    if let Some(to) = sq + delta {
                        if (to.file() - sq.file()).abs() <= 1 && (to.rank() - sq.rank()).abs() <= 2
                        {
                            table[sq.0 as usize][color.index()] |= to;
                        }
                    }
                }
            }
        }
        Self(table)
    }
    pub fn attack(&self, sq: Square, c: Color) -> Bitboard {
        self.0[sq.0 as usize][c.index()]
    }
}

fn sliding_attacks(sq: Square, occupied: Bitboard, deltas: &[Delta]) -> Bitboard {
    let mut bb = Bitboard::ZERO;
    for &delta in deltas {
        let (mut prev, mut curr) = (sq, sq + delta);
        while let Some(to) = curr {
            // stop if rank diff is +8 or -8
            if (to.rank() - prev.rank()) & 0x0f == 0x08 {
                break;
            }
            bb |= to;
            if !(occupied & to).is_empty() {
                break;
            }
            prev = to;
            curr = to + delta;
        }
    }
    bb
}

pub struct LanceAttackTable(
    [[[Bitboard; LanceAttackTable::MASK_TABLE_NUM as usize]; Color::NUM]; Square::NUM],
);

impl LanceAttackTable {
    const MASK_BITS: u32 = 7;
    const MASK_TABLE_NUM: usize = 1 << LanceAttackTable::MASK_BITS;
    #[rustfmt::skip]
    const OFFSETS: [u32; Square::NUM] = [
         1,  1,  1,  1,  1,  1,  1,  1,  1,
        10, 10, 10, 10, 10, 10, 10, 10, 10,
        19, 19, 19, 19, 19, 19, 19, 19, 19,
        28, 28, 28, 28, 28, 28, 28, 28, 28,
        37, 37, 37, 37, 37, 37, 37, 37, 37,
        46, 46, 46, 46, 46, 46, 46, 46, 46,
        55, 55, 55, 55, 55, 55, 55, 55, 55,
         1,  1,  1,  1,  1,  1,  1,  1,  1,
        10, 10, 10, 10, 10, 10, 10, 10, 10,
    ];

    fn attack_mask(sq: Square) -> Bitboard {
        Bitboard::from_file(sq.file())
            & !(Bitboard::from_rank(Rank::RANK1) | Bitboard::from_rank(Rank::RANK9))
    }
    fn index_to_occupied(index: usize, mask: Bitboard) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        for (i, sq) in mask.enumerate() {
            if (index & (1 << i)) != 0 {
                bb |= sq;
            }
        }
        bb
    }
    fn new() -> Self {
        let mut table = [[[Bitboard::ZERO; LanceAttackTable::MASK_TABLE_NUM as usize]; Color::NUM];
            Square::NUM];
        for sq in Square::ALL {
            let mask = Self::attack_mask(sq);
            for c in Color::ALL {
                let deltas = match c {
                    Color::Black => vec![Delta::N],
                    Color::White => vec![Delta::S],
                };
                for index in 0..LanceAttackTable::MASK_TABLE_NUM {
                    let occupied = Self::index_to_occupied(index, mask);
                    table[sq.0 as usize][c.index()][index] = sliding_attacks(sq, occupied, &deltas);
                }
            }
        }
        Self(table)
    }
    pub fn attack(&self, sq: Square, c: Color, occupied: &Bitboard) -> Bitboard {
        let index = ((occupied.value(if sq.0 > Square::SQ79.0 { 1 } else { 0 })
            >> LanceAttackTable::OFFSETS[sq.0 as usize]) as usize)
            & (Self::MASK_TABLE_NUM - 1);
        self.0[sq.0 as usize][c.index()][index]
    }
}

pub struct SlidingAttackTable {
    table: Vec<Bitboard>,
    masks: [Bitboard; Square::NUM],
    offsets: [usize; Square::NUM],
}

impl SlidingAttackTable {
    fn attack_mask(sq: Square, deltas: &[Delta]) -> Bitboard {
        let mut bb = sliding_attacks(sq, Bitboard::ZERO, deltas);
        if sq.file() != File::FILE1 {
            bb &= !Bitboard::FILES[File::FILE1.0 as usize];
        }
        if sq.file() != File::FILE9 {
            bb &= !Bitboard::FILES[File::FILE9.0 as usize];
        }
        if sq.rank() != Rank::RANK1 {
            bb &= !Bitboard::RANKS[Rank::RANK1.0 as usize];
        }
        if sq.rank() != Rank::RANK9 {
            bb &= !Bitboard::RANKS[Rank::RANK9.0 as usize];
        }
        bb
    }
    fn index_to_occupied(index: usize, mask: Bitboard) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        for (i, sq) in mask.enumerate() {
            if (index & (1 << i)) != 0 {
                bb |= sq;
            }
        }
        bb
    }
    fn occupied_to_index(occupied: Bitboard, mask: Bitboard) -> usize {
        (occupied & mask).merge().pext(mask.merge()) as usize
    }
    fn new(table_size: usize, deltas: &[Delta]) -> Self {
        let mut table = vec![Bitboard::ZERO; table_size];
        let mut masks = [Bitboard::ZERO; Square::NUM];
        let mut offsets = [0; Square::NUM];
        let mut offset = 0;
        for sq in Square::ALL {
            let mask = SlidingAttackTable::attack_mask(sq, deltas);
            let ones = mask.count_ones();
            masks[sq.0 as usize] = mask;
            offsets[sq.0 as usize] = offset;
            for index in 0..1 << ones {
                let occupied = Self::index_to_occupied(index, mask);
                table[offset + Self::occupied_to_index(occupied, mask)] =
                    sliding_attacks(sq, occupied, deltas);
            }
            offset += 1 << ones;
        }
        Self {
            table,
            masks,
            offsets,
        }
    }
    pub fn attack(&self, sq: Square, occupied: &Bitboard) -> Bitboard {
        self.table[self.offsets[sq.index()]
            + Self::occupied_to_index(*occupied, self.masks[sq.index()])]
    }
    pub fn pseudo_attack(&self, sq: Square) -> Bitboard {
        self.table[self.offsets[sq.index()]]
    }
}

pub struct AttackTable {
    pub fu: PieceAttackTable,
    pub ky: LanceAttackTable,
    pub ke: PieceAttackTable,
    pub gi: PieceAttackTable,
    pub ki: PieceAttackTable,
    pub ka: SlidingAttackTable,
    pub hi: SlidingAttackTable,
    pub ou: PieceAttackTable,
}

pub static ATTACK_TABLE: Lazy<AttackTable> = Lazy::new(|| AttackTable {
    fu: PieceAttackTable::new(&[PieceAttackTable::BFU_DELTAS, PieceAttackTable::WFU_DELTAS]),
    ky: LanceAttackTable::new(),
    ke: PieceAttackTable::new(&[PieceAttackTable::BKE_DELTAS, PieceAttackTable::WKE_DELTAS]),
    gi: PieceAttackTable::new(&[PieceAttackTable::BGI_DELTAS, PieceAttackTable::WGI_DELTAS]),
    ki: PieceAttackTable::new(&[PieceAttackTable::BKI_DELTAS, PieceAttackTable::WKI_DELTAS]),
    ka: SlidingAttackTable::new(
        20224, // 1 * (1 << 12) + 8 * (1 << 10) + 16 * (1 << 8) + 52 * (1 << 6) + 4 * (1 << 7)
        &[Delta::NE, Delta::SE, Delta::SW, Delta::NW],
    ),
    hi: SlidingAttackTable::new(
        495616, // 4 * (1 << 14) + 28 * (1 << 13) + 49 * (1 << 12)
        &[Delta::N, Delta::E, Delta::S, Delta::W],
    ),
    ou: PieceAttackTable::new(&[PieceAttackTable::BOU_DELTAS, PieceAttackTable::WOU_DELTAS]),
});

pub static BETWEEN_TABLE: Lazy<[[Bitboard; Square::NUM]; Square::NUM]> = Lazy::new(|| {
    let mut bbs = [[Bitboard::ZERO; Square::NUM]; Square::NUM];
    for sq0 in Square::ALL {
        for sq1 in Square::ALL {
            let (df, dr) = (sq1.file() - sq0.file(), sq1.rank() - sq0.rank());
            if (df | dr == 0) || (df != 0 && dr != 0 && df.abs() != dr.abs()) {
                continue;
            }
            #[rustfmt::skip]
            let delta = match (df.cmp(&0), dr.cmp(&0)) {
                (Ordering::Equal,   Ordering::Less)    => Delta::N,
                (Ordering::Less,    Ordering::Equal)   => Delta::E,
                (Ordering::Equal,   Ordering::Greater) => Delta::S,
                (Ordering::Greater, Ordering::Equal)   => Delta::W,
                (Ordering::Less,    Ordering::Less)    => Delta::NE,
                (Ordering::Less,    Ordering::Greater) => Delta::SE,
                (Ordering::Greater, Ordering::Greater) => Delta::SW,
                (Ordering::Greater, Ordering::Less)    => Delta::NW,
                _ => unreachable!(),
            };
            bbs[sq0.index()][sq1.index()] =
                sliding_attacks(sq0, Bitboard::from_square(sq1), &[delta])
                    & !Bitboard::from_square(sq1);
        }
    }
    bbs
});
