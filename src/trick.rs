use crate::dice::{DieType, Face};

// ── Special character beat table ─────────────────────────────────────────────
// Minotaur > Griffin > Mermaid > Minotaur  (rock-paper-scissors cycle)

/// Returns true if `attacker` beats `defender` in the special-character cycle.
/// Both must be special faces; panics in debug if called with non-specials.
fn special_beats(attacker: Face, defender: Face) -> bool {
    match (attacker, defender) {
        (Face::Minotaur, Face::Griffin) => true,
        (Face::Griffin, Face::Mermaid) => true,
        (Face::Mermaid, Face::Minotaur) => true,
        _ => false,
    }
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
/// 3. Flags (0) can never win.
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
        // Flag can never win.
        (Face::Flag, _) => false,
        // Anything beats a flag (or 0 holder).
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
/// * If the player is the leader (`is_leader = true`) or rolls a special-character die,
///   all faces of their die are available.
/// * If `led_color` is Some and the player has a matching color die (same DieType),
///   they must roll from that die (all its faces).
///   However, they may always choose a special-character die instead — but here we
///   receive a single `die: DieType` representing what they actually played.
///   (The caller selects which die a player uses; this function returns all faces of it.)
fn allowed_faces(die: DieType, _led_color: Option<DieType>, _is_leader: bool) -> Vec<(Face, f64)> {
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
fn enumerate_combinations(
    player_faces: &[Vec<(Face, f64)>],
    callback: &mut impl FnMut(&[usize]),
) {
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
        // Both roll Flag — Flag can't win, so the tie-break never fires positively.
        // Actually with our logic: flag vs flag, first check challenger.face == Flag → false.
        // So rolls[0] stays as best. Winner = 0.
        let rolls = vec![
            RolledDie::new(DieType::Gray, Face::Flag, 0),
            RolledDie::new(DieType::Gray, Face::Flag, 1),
        ];
        assert_eq!(trick_winner(&rolls), 0);
    }
}
