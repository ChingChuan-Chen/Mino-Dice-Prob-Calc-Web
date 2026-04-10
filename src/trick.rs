use crate::dice::{DieType, Face};
use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::Connection;

// ── Special character beat table ─────────────────────────────────────────────
// Minotaur > Griffin > Mermaid > Minotaur  (rock-paper-scissors cycle)

/// Returns true if `attacker` beats `defender` in the special-character cycle.
/// Both must be special faces; panics in debug if called with non-specials.
fn special_beats(attacker: Face, defender: Face) -> bool {
    matches!(
        (attacker, defender),
        (Face::Minotaur, Face::Griffin)
            | (Face::Griffin, Face::Mermaid)
            | (Face::Mermaid, Face::Minotaur)
    )
}

// ── Trick context ─────────────────────────────────────────────────────────────

/// The die rolled by one player in a trick.
#[derive(Debug, Clone, Copy)]
pub struct RolledDie {
    /// Which die type was used.
    pub die_type: DieType,
    /// The rolled face.
    pub face: Face,
    /// Roll order (0 = led, 1 = second, … used for tie-breaking among numbers).
    pub roll_order: usize,
}

impl RolledDie {
    pub fn new(die_type: DieType, face: Face, roll_order: usize) -> Self {
        Self {
            die_type,
            face,
            roll_order,
        }
    }
}

// ── Trick winner resolution ───────────────────────────────────────────────────

/// Given all dice rolled in a trick (in roll order), returns the index of the winner.
///
/// Rules:
/// 1. Special characters beat all numbers.
/// 2. Among specials: Minotaur > Griffin, Griffin > Mermaid, Mermaid > Minotaur.
///    If two identical specials clash, the later roller wins.
/// 3. Flags (0) lose to any non-flag face. If every roll is a flag, the later roller wins.
/// 4. Among numbers only: highest wins; ties go to the later roller.
pub fn trick_winner(rolls: &[RolledDie]) -> usize {
    assert!(!rolls.is_empty(), "trick must have at least one roll");

    let mut best_idx = 0;
    for (i, roll) in rolls.iter().enumerate().skip(1) {
        if beats_current(roll, &rolls[best_idx]) {
            best_idx = i;
        }
    }
    best_idx
}

/// Returns true if `challenger` beats `current` holder.
fn beats_current(challenger: &RolledDie, current: &RolledDie) -> bool {
    match (challenger.face, current.face) {
        // If everyone rolled Flag, the later roller wins the tie.
        (Face::Flag, Face::Flag) => challenger.roll_order > current.roll_order,
        // A flag loses to any non-flag face.
        (Face::Flag, _) => false,
        // Any non-flag face beats a flag.
        (_, Face::Flag) => true,
        // Both specials: check cycle, tie → later roller wins.
        (c, h) if c.is_special() && h.is_special() => {
            if c == h {
                challenger.roll_order > current.roll_order
            } else {
                special_beats(c, h)
            }
        }
        // Special beats number.
        (c, _) if c.is_special() => true,
        // Number loses to special.
        (_, h) if h.is_special() => false,
        // Both numbers: higher wins; tie → later roller.
        (Face::Number(cn), Face::Number(hn)) => {
            if cn == hn {
                challenger.roll_order > current.roll_order
            } else {
                cn > hn
            }
        }
        _ => false,
    }
}

// ── Per-die win probability for a single trick ────────────────────────────────

/// Computes the probability that player `target_idx` wins the trick, given each
/// player's die type and the suit-following constraints.
///
/// * `hand` — die type each player will roll from (in play order).
/// * `led_color` — the color (DieType) of the leading number die, if the lead
///   was a number die. `None` if the lead was a special-character die.
/// * `target_idx` — the player whose win probability we want.
///
/// The function enumerates all possible combinations of face outcomes for every
/// player (6^n combinations) and counts the fraction where `target_idx` wins.
pub fn win_probability(hand: &[DieType], led_color: Option<DieType>, target_idx: usize) -> f64 {
    let n = hand.len();
    assert!(target_idx < n);

    // Fast path: with no suit constraint, use the precomputed single-trick table
    // for legal player counts. This is exact and avoids repeated face enumeration.
    if led_color.is_none() && (3..=6).contains(&n) {
        return lookup_single_trick_reference(hand, target_idx)
            .unwrap_or_else(|| win_probability_enumerated(hand, led_color, target_idx));
    }

    win_probability_enumerated(hand, led_color, target_idx)
}

fn win_probability_enumerated(
    hand: &[DieType],
    led_color: Option<DieType>,
    target_idx: usize,
) -> f64 {
    let n = hand.len();
    assert!(target_idx < n);

    // Build per-player allowed faces and their probabilities.
    let player_faces: Vec<Vec<(Face, f64)>> = hand
        .iter()
        .enumerate()
        .map(|(i, &die)| allowed_faces(die, led_color, i == 0))
        .collect();

    // Enumerate all combinations.
    let mut win_prob = 0.0;
    enumerate_combinations(&player_faces, &mut |combo: &[usize]| {
        // combo[i] = index into player_faces[i]
        let rolls: Vec<RolledDie> = combo
            .iter()
            .enumerate()
            .map(|(i, &fi)| {
                let (face, _) = player_faces[i][fi];
                RolledDie::new(hand[i], face, i)
            })
            .collect();

        let winner = trick_winner(&rolls);
        if winner == target_idx {
            let prob: f64 = combo
                .iter()
                .enumerate()
                .map(|(i, &fi)| player_faces[i][fi].1)
                .product();
            win_prob += prob;
        }
    });

    win_prob
}

/// Returns the (face, probability) pairs available to a player given suit constraints.
///
/// * The leader (`is_leader = true`) may play any die, with all its faces available.
/// * A follower may always play a special-character die, ignoring the led suit.
/// * A follower who plays a number die of the led color must use that die.
/// * A follower with no matching-color number die may play any die.
///
/// This function receives the actual die the player will play (`die`) and the
/// led color from the leader's roll. It returns the correctly weighted face distribution.
pub fn allowed_faces(
    die: DieType,
    _led_color: Option<DieType>,
    _is_leader: bool,
) -> Vec<(Face, f64)> {
    // All faces of the chosen die with uniform 1/6 probability.
    let faces = die.faces();
    let prob = 1.0 / faces.len() as f64;
    // Merge duplicates.
    let mut map: std::collections::HashMap<Face, f64> = std::collections::HashMap::new();
    for f in &faces {
        *map.entry(*f).or_insert(0.0) += prob;
    }
    let mut v: Vec<(Face, f64)> = map.into_iter().collect();
    v.sort_by_key(|(f, _)| face_sort_key(*f));
    v
}

pub fn win_probability_with_suit_context(all_dice: &[DieType], target_idx: usize) -> f64 {
    assert!(!all_dice.is_empty());
    assert!(target_idx < all_dice.len());

    // This path intentionally uses the same reference table as `win_probability`
    // when no additional per-seat suit-choice information is present.
    lookup_single_trick_reference(all_dice, target_idx)
        .unwrap_or_else(|| win_probability_enumerated(all_dice, None, target_idx))
}

/// Returns the one-trick win probability for every seat in `all_dice` order.
///
/// This is backed by the same static reference table used by
/// `win_probability_with_suit_context` for 3-6 players.
pub fn win_probabilities_for_all_seats(all_dice: &[DieType]) -> Vec<f64> {
    let n = all_dice.len();
    (0..n)
        .map(|target_idx| win_probability_with_suit_context(all_dice, target_idx))
        .collect()
}

/// Returns one-trick win probabilities for every seat using direct face enumeration.
///
/// This bypasses the static reference table and is intended for diagnostics and
/// performance comparisons.
pub fn win_probabilities_for_all_seats_dynamic(all_dice: &[DieType]) -> Vec<f64> {
    let n = all_dice.len();
    (0..n)
        .map(|target_idx| win_probability_enumerated(all_dice, None, target_idx))
        .collect()
}

/// For 3-6 players, stores exact single-trick win probabilities for every
/// ordered die assignment and every target seat.
///
/// Layout per player count `n`:
/// - `table[n]` has length `7^n * n`
/// - block `seq_idx * n .. seq_idx * n + n` stores target-seat probabilities.
static SINGLE_TRICK_REFERENCE_TABLE: OnceLock<Vec<Vec<f64>>> = OnceLock::new();
static SQLITE_SINGLE_TRICK_REFERENCE_TABLE: OnceLock<Option<Vec<Vec<f64>>>> = OnceLock::new();

fn lookup_single_trick_reference(all_dice: &[DieType], target_idx: usize) -> Option<f64> {
    let n = all_dice.len();
    if !(3..=6).contains(&n) || target_idx >= n {
        return None;
    }

    let seq_idx = encode_die_sequence(all_dice)?;
    let sqlite_table = SQLITE_SINGLE_TRICK_REFERENCE_TABLE
        .get_or_init(load_single_trick_reference_table_from_sqlite);
    if let Some(table) = sqlite_table {
        return Some(table[n][seq_idx * n + target_idx]);
    }

    let fallback = SINGLE_TRICK_REFERENCE_TABLE.get_or_init(build_single_trick_reference_table);
    Some(fallback[n][seq_idx * n + target_idx])
}

#[cfg(not(target_arch = "wasm32"))]
fn load_single_trick_reference_table_from_sqlite() -> Option<Vec<Vec<f64>>> {
    let db_path = std::env::var("MINO_DICE_PROB_DB")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "data/win_probability_mapping.db".to_string());

    let conn = Connection::open(db_path).ok()?;

    let mut per_n: Vec<Vec<f64>> = vec![Vec::new(); 7];
    for n in 3..=6 {
        let seq_count = 7usize.pow(n as u32);
        per_n[n] = vec![f64::NAN; seq_count * n];
    }

    let mut stmt = conn
        .prepare(
            "
            SELECT
                player_count,
                seq_idx,
                p0_count,
                p1_count,
                p2_count,
                p3_count,
                p4_count,
                p5_count,
                denominator
            FROM single_trick_probabilities
            ",
        )
        .ok()?;

    let mut rows = stmt.query([]).ok()?;
    while let Some(row) = rows.next().ok()? {
        let player_count: usize = row.get::<_, i64>(0).ok()? as usize;
        if !(3..=6).contains(&player_count) {
            return None;
        }

        let seq_idx: usize = row.get::<_, i64>(1).ok()? as usize;
        let p_counts = [
            row.get::<_, i64>(2).ok()?,
            row.get::<_, i64>(3).ok()?,
            row.get::<_, i64>(4).ok()?,
            row.get::<_, i64>(5).ok()?,
            row.get::<_, i64>(6).ok()?,
            row.get::<_, i64>(7).ok()?,
        ];
        let denominator: i64 = row.get(8).ok()?;
        if denominator <= 0 {
            return None;
        }

        let seq_count = 7usize.pow(player_count as u32);
        if seq_idx >= seq_count {
            return None;
        }
        let base = seq_idx * player_count;
        for target in 0..player_count {
            per_n[player_count][base + target] = p_counts[target] as f64 / denominator as f64;
        }
    }

    // Require full coverage; otherwise fallback to computed table.
    for n in 3..=6 {
        if per_n[n].iter().any(|v| v.is_nan()) {
            return None;
        }
    }

    Some(per_n)
}

#[cfg(target_arch = "wasm32")]
fn load_single_trick_reference_table_from_sqlite() -> Option<Vec<Vec<f64>>> {
    None
}

fn build_single_trick_reference_table() -> Vec<Vec<f64>> {
    let mut per_n: Vec<Vec<f64>> = vec![Vec::new(); 7];

    for (n, table_slot) in per_n.iter_mut().enumerate().skip(3) {
        let seq_count = 7usize.pow(n as u32);
        let mut table = vec![0.0f64; seq_count * n];
        let mut die_digits = vec![0usize; n];
        let mut seq_dice = vec![DieType::Minotaur; n];

        for seq_idx in 0..seq_count {
            let probs = win_probability_all_targets(&seq_dice);
            let base = seq_idx * n;
            for (target, p) in probs.into_iter().enumerate() {
                table[base + target] = p;
            }

            if seq_idx + 1 < seq_count {
                increment_base7_digits(&mut die_digits, &mut seq_dice);
            }
        }

        *table_slot = table;
    }

    per_n
}

fn win_probability_all_targets(hand: &[DieType]) -> Vec<f64> {
    let player_faces: Vec<Vec<(Face, f64)>> = hand
        .iter()
        .enumerate()
        .map(|(i, &die)| allowed_faces(die, None, i == 0))
        .collect();

    let mut probs = vec![0.0f64; hand.len()];
    enumerate_combinations(&player_faces, &mut |combo: &[usize]| {
        let rolls: Vec<RolledDie> = combo
            .iter()
            .enumerate()
            .map(|(i, &fi)| {
                let (face, _) = player_faces[i][fi];
                RolledDie::new(hand[i], face, i)
            })
            .collect();
        let winner = trick_winner(&rolls);
        let outcome_prob: f64 = combo
            .iter()
            .enumerate()
            .map(|(i, &fi)| player_faces[i][fi].1)
            .product();
        probs[winner] += outcome_prob;
    });

    probs
}

fn increment_base7_digits(digits: &mut [usize], dice: &mut [DieType]) {
    let mut pos = digits.len();
    while pos > 0 {
        pos -= 1;
        digits[pos] += 1;
        if digits[pos] < 7 {
            dice[pos] = DieType::ALL[digits[pos]];
            return;
        }
        digits[pos] = 0;
        dice[pos] = DieType::ALL[0];
    }
}

fn encode_die_sequence(all_dice: &[DieType]) -> Option<usize> {
    all_dice.iter().try_fold(0usize, |acc, die| {
        let idx = die_to_index(*die)?;
        Some(acc * 7 + idx)
    })
}

fn die_to_index(die: DieType) -> Option<usize> {
    match die {
        DieType::Minotaur => Some(0),
        DieType::Griffin => Some(1),
        DieType::Mermaid => Some(2),
        DieType::Red => Some(3),
        DieType::Yellow => Some(4),
        DieType::Purple => Some(5),
        DieType::Gray => Some(6),
    }
}

fn face_sort_key(f: Face) -> u16 {
    match f {
        Face::Flag => 0,
        Face::Number(n) => n as u16,
        Face::Mermaid => 100,
        Face::Griffin => 101,
        Face::Minotaur => 102,
    }
}

/// Iterates over all combinations of indices, calling `callback` with each combo.
fn enumerate_combinations(player_faces: &[Vec<(Face, f64)>], callback: &mut impl FnMut(&[usize])) {
    let n = player_faces.len();
    let counts: Vec<usize> = player_faces.iter().map(|v| v.len()).collect();
    let mut indices = vec![0usize; n];

    loop {
        callback(&indices);

        // Increment indices (last player varies fastest).
        let mut pos = n;
        loop {
            if pos == 0 {
                return;
            }
            pos -= 1;
            indices[pos] += 1;
            if indices[pos] < counts[pos] {
                break;
            }
            indices[pos] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_always_loses() {
        let rolls = vec![
            RolledDie::new(DieType::Red, Face::Flag, 0),
            RolledDie::new(DieType::Purple, Face::Number(1), 1),
        ];
        assert_eq!(trick_winner(&rolls), 1);
    }

    #[test]
    fn higher_number_wins() {
        let rolls = vec![
            RolledDie::new(DieType::Red, Face::Number(7), 0),
            RolledDie::new(DieType::Yellow, Face::Number(5), 1),
        ];
        assert_eq!(trick_winner(&rolls), 0);
    }

    #[test]
    fn number_tie_later_roller_wins() {
        let rolls = vec![
            RolledDie::new(DieType::Red, Face::Number(5), 0),
            RolledDie::new(DieType::Yellow, Face::Number(5), 1),
        ];
        assert_eq!(trick_winner(&rolls), 1);
    }

    #[test]
    fn minotaur_beats_griffin() {
        let rolls = vec![
            RolledDie::new(DieType::Minotaur, Face::Minotaur, 0),
            RolledDie::new(DieType::Griffin, Face::Griffin, 1),
        ];
        assert_eq!(trick_winner(&rolls), 0);
    }

    #[test]
    fn griffin_beats_mermaid() {
        let rolls = vec![
            RolledDie::new(DieType::Griffin, Face::Griffin, 0),
            RolledDie::new(DieType::Mermaid, Face::Mermaid, 1),
        ];
        assert_eq!(trick_winner(&rolls), 0);
    }

    #[test]
    fn mermaid_beats_minotaur() {
        let rolls = vec![
            RolledDie::new(DieType::Minotaur, Face::Minotaur, 0),
            RolledDie::new(DieType::Mermaid, Face::Mermaid, 1),
        ];
        assert_eq!(trick_winner(&rolls), 1);
    }

    #[test]
    fn special_beats_number() {
        let rolls = vec![
            RolledDie::new(DieType::Red, Face::Number(7), 0),
            RolledDie::new(DieType::Griffin, Face::Griffin, 1),
        ];
        assert_eq!(trick_winner(&rolls), 1);
    }

    #[test]
    fn same_special_later_wins() {
        let rolls = vec![
            RolledDie::new(DieType::Griffin, Face::Griffin, 0),
            RolledDie::new(DieType::Griffin, Face::Griffin, 1),
        ];
        assert_eq!(trick_winner(&rolls), 1);
    }

    #[test]
    fn win_prob_two_red_sums_to_one() {
        // Two players each rolling a Red die, no suit led → probs must sum to 1.
        let hand = [DieType::Red, DieType::Red];
        let p0 = win_probability(&hand, None, 0);
        let p1 = win_probability(&hand, None, 1);
        assert!((p0 + p1 - 1.0).abs() < 1e-9, "p0={p0}, p1={p1}");
    }

    #[test]
    fn win_prob_minotaur_vs_red() {
        // Minotaur die rolls Minotaur (4/6) or Flag (2/6).
        // Red die rolls numbers only, so Minotaur wins exactly 4/6.
        let hand = [DieType::Minotaur, DieType::Red];
        let p = win_probability(&hand, None, 0);
        assert!((p - 4.0 / 6.0).abs() < 1e-9, "p={p}");
    }

    #[test]
    fn all_flags_later_roller_wins() {
        let rolls = vec![
            RolledDie::new(DieType::Gray, Face::Flag, 0),
            RolledDie::new(DieType::Gray, Face::Flag, 1),
        ];
        assert_eq!(trick_winner(&rolls), 1);
    }

    #[test]
    fn all_flags_last_of_three_wins() {
        let rolls = vec![
            RolledDie::new(DieType::Gray, Face::Flag, 0),
            RolledDie::new(DieType::Gray, Face::Flag, 1),
            RolledDie::new(DieType::Gray, Face::Flag, 2),
        ];
        assert_eq!(trick_winner(&rolls), 2);
    }

    #[test]
    #[should_panic(expected = "trick must have at least one roll")]
    fn trick_winner_panics_on_empty_rolls() {
        let _ = trick_winner(&[]);
    }

    #[test]
    fn reference_table_matches_direct_enumeration_samples() {
        let cases: Vec<(Vec<DieType>, usize)> = vec![
            (vec![DieType::Red, DieType::Yellow, DieType::Purple], 0),
            (vec![DieType::Mermaid, DieType::Red, DieType::Gray], 0),
            (
                vec![
                    DieType::Minotaur,
                    DieType::Griffin,
                    DieType::Mermaid,
                    DieType::Gray,
                ],
                2,
            ),
            (
                vec![
                    DieType::Red,
                    DieType::Yellow,
                    DieType::Purple,
                    DieType::Gray,
                    DieType::Mermaid,
                ],
                4,
            ),
        ];

        for (hand, target_idx) in cases {
            let direct = win_probability_enumerated(&hand, None, target_idx);
            let table = win_probability_with_suit_context(&hand, target_idx);
            assert!(
                (direct - table).abs() < 1e-12,
                "hand={hand:?} target={target_idx} direct={direct} table={table}"
            );
        }
    }

    #[test]
    fn all_seat_probabilities_sum_to_one() {
        let hand = vec![
            DieType::Mermaid,
            DieType::Red,
            DieType::Gray,
            DieType::Purple,
        ];
        let probs = win_probabilities_for_all_seats(&hand);
        let total: f64 = probs.iter().sum();
        assert!((total - 1.0).abs() < 1e-12, "probs={probs:?} total={total}");
    }

    #[test]
    fn all_seat_probabilities_match_dynamic_enumeration() {
        let hand = vec![DieType::Mermaid, DieType::Red, DieType::Gray];
        let from_table = win_probabilities_for_all_seats(&hand);
        let dynamic = win_probabilities_for_all_seats_dynamic(&hand);
        for (a, b) in from_table.iter().zip(dynamic.iter()) {
            assert!(
                (a - b).abs() < 1e-12,
                "from_table={from_table:?} dynamic={dynamic:?}"
            );
        }
    }
}
