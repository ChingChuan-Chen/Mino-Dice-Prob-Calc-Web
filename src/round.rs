use crate::dice::DieType;
use crate::trick::{win_probability, RolledDie, trick_winner};
use crate::dice::Face;

// ── Round-level simulation ────────────────────────────────────────────────────

/// The number of rounds played for a given player count.
pub fn round_count(player_count: usize) -> usize {
    match player_count {
        3 | 4 => 8,
        5 => 7,
        6 => 6,
        _ => panic!("player_count must be 3–6"),
    }
}

/// Draws `n` dice from the bag uniformly at random (without replacement).
/// Returns a Vec of `n` DieType values.
///
/// The bag contains:
///   Minotaur ×1, Griffin ×3, Mermaid ×2,
///   Red ×7, Yellow ×7, Purple ×8, Gray ×8  (total 36)
pub fn sample_hand(n: usize, rng: &mut impl Rng) -> Vec<DieType> {
    let mut bag: Vec<DieType> = DieType::ALL
        .iter()
        .flat_map(|&dt| std::iter::repeat(dt).take(dt.bag_count() as usize))
        .collect();
    assert_eq!(bag.len(), 36);

    // Partial Fisher-Yates: only shuffle the first `n` elements.
    for i in 0..n {
        let j = i + rng.next_usize(bag.len() - i);
        bag.swap(i, j);
    }
    bag[..n].to_vec()
}

// ── Analytical trick-count distribution ──────────────────────────────────────

/// Returns the probability distribution P(tricks = k) for a player with the
/// given hand, assuming each trick is played independently (approximation:
/// ignores die depletion across tricks within a round).
///
/// * `hand` — the player's dice (drawn from the bag for this round).
/// * `opponent_dice` — a representative set of die types for all opponents.
///   Pass the same die for every opponent for a worst-case / best-case analysis,
///   or use `expected_opponent_die` for an average draw.
/// * Returns a Vec of length `hand.len() + 1` where index k = P(win exactly k tricks).
pub fn trick_count_distribution(
    hand: &[DieType],
    opponent_dice: &[Vec<DieType>],
) -> Vec<f64> {
    let num_tricks = hand.len();
    let _num_opponents = opponent_dice.len();
    // dp[k] = probability of winning exactly k tricks so far.
    let mut dp = vec![0.0f64; num_tricks + 1];
    dp[0] = 1.0;

    for trick_idx in 0..num_tricks {
        let player_die = hand[trick_idx];
        // Pick one representative die per opponent for this trick slot.
        let mut all_dice = vec![player_die];
        for opp in opponent_dice {
            all_dice.push(opp[trick_idx % opp.len()]);
        }
        let p_win = win_probability(&all_dice, None, 0);

        // Update DP: iterate backwards to avoid using updated values.
        let mut new_dp = vec![0.0f64; num_tricks + 1];
        for k in 0..=trick_idx {
            if dp[k] > 0.0 {
                new_dp[k + 1] += dp[k] * p_win;
                new_dp[k] += dp[k] * (1.0 - p_win);
            }
        }
        dp = new_dp;
    }
    dp
}

/// Returns the expected number of tricks won from a given distribution.
pub fn expected_tricks(dist: &[f64]) -> f64 {
    dist.iter()
        .enumerate()
        .map(|(k, &p)| k as f64 * p)
        .sum()
}

// ── Expected score from a bid ─────────────────────────────────────────────────

/// Computes the expected score for placing bid `b` in a round with `hand_size` tricks,
/// given the trick-count distribution `dist`.
///
/// Scoring rules:
/// - Bid > 0, exact match: +20 × b
/// - Bid > 0, miss by d: −10 × d
/// - Bid = 0, succeed (0 tricks): +10 × hand_size
/// - Bid = 0, fail: −10 × hand_size
pub fn expected_score_for_bid(bid: usize, dist: &[f64], hand_size: usize) -> f64 {
    if bid == 0 {
        let p_zero = dist[0];
        let p_fail = 1.0 - p_zero;
        let bonus = 10.0 * hand_size as f64;
        p_zero * bonus + p_fail * (-bonus)
    } else {
        dist.iter()
            .enumerate()
            .map(|(tricks, &p)| {
                if p == 0.0 {
                    return 0.0;
                }
                let score = if tricks == bid {
                    20.0 * bid as f64
                } else {
                    -10.0 * (tricks as i64 - bid as i64).unsigned_abs() as f64
                };
                p * score
            })
            .sum()
    }
}

/// Returns the bid (0..=hand_size) that maximises expected score.
pub fn optimal_bid(dist: &[f64]) -> usize {
    let hand_size = dist.len() - 1;
    (0..=hand_size)
        .max_by(|&a, &b| {
            expected_score_for_bid(a, dist, hand_size)
                .partial_cmp(&expected_score_for_bid(b, dist, hand_size))
                .unwrap()
        })
        .unwrap()
}

// ── Minimal RNG abstraction (no std::collections dependency in WASM) ──────────

pub trait Rng {
    fn next_usize(&mut self, bound: usize) -> usize;
}

/// A simple xorshift64 RNG suitable for WASM (no OS entropy needed at init).
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xdeadbeef_cafebabe } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

impl Rng for Xorshift64 {
    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

// ── Monte Carlo simulation ─────────────────────────────────────────────────────

/// Result of one simulated game for a single player.
#[derive(Debug, Clone)]
pub struct SimResult {
    /// Tricks won per round (index = round number starting from 0).
    pub tricks_per_round: Vec<usize>,
    /// Total score.
    pub total_score: i64,
}

/// Simulate `n_games` complete games for `player_count` players, returning per-player
/// total-score distributions (vector of length `n_games` per player).
pub fn simulate_games(
    player_count: usize,
    n_games: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<i64>> {
    let rounds = round_count(player_count);
    let mut all_scores: Vec<Vec<i64>> = vec![vec![]; player_count];

    for _ in 0..n_games {
        let mut totals = vec![0i64; player_count];

        for round in 1..=rounds {
            // Each player draws `round` dice.
            let hands: Vec<Vec<DieType>> = (0..player_count)
                .map(|_| sample_hand(round, rng))
                .collect();

            // Naive AI: bid the expected number of tricks (rounded).
            let bids: Vec<usize> = hands
                .iter()
                .enumerate()
                .map(|(p, hand)| {
                    // Build opponent representative hands.
                    let opp_dice: Vec<Vec<DieType>> = (0..player_count)
                        .filter(|&o| o != p)
                        .map(|o| hands[o].clone())
                        .collect();
                    let dist = trick_count_distribution(hand, &opp_dice);
                    optimal_bid(&dist)
                })
                .collect();

            // Simulate round tricks.
            let tricks_won = simulate_round(&hands, rng);

            // Score each player.
            for p in 0..player_count {
                totals[p] += score_player(bids[p], tricks_won[p], round);
            }
        }

        for p in 0..player_count {
            all_scores[p].push(totals[p]);
        }
    }
    all_scores
}

/// Simulate one round trick-by-trick, returning tricks won per player.
fn simulate_round(hands: &[Vec<DieType>], rng: &mut impl Rng) -> Vec<usize> {
    let player_count = hands.len();
    let num_tricks = hands[0].len();
    let mut tricks_won = vec![0usize; player_count];

    // Keep a pool of remaining dice per player (indices into their hand).
    let mut remaining: Vec<Vec<usize>> = (0..player_count)
        .map(|p| (0..hands[p].len()).collect())
        .collect();

    let mut leader = 0usize;

    for _ in 0..num_tricks {
        // Leader picks a die (first available for determinism).
        let leader_die_idx = remaining[leader].remove(0);
        let leader_die = hands[leader][leader_die_idx];

        // Determine led color if it's a number die.
        let led_color = if leader_die.is_special_die() {
            None
        } else {
            Some(leader_die)
        };

        // Each other player rolls a die.
        let mut rolls: Vec<RolledDie> = Vec::with_capacity(player_count);

        // Leader's roll.
        let leader_face = random_face(leader_die, rng);
        rolls.push(RolledDie::new(leader_die, leader_face, 0));

        for order in 1..player_count {
            let p = (leader + order) % player_count;
            let chosen_die = choose_die_to_play(hands[p][0], led_color, &remaining[p], &hands[p]);
            // Remove from remaining.
            if let Some(pos) = remaining[p].iter().position(|&i| hands[p][i] == chosen_die) {
                remaining[p].remove(pos);
            }
            let face = random_face(chosen_die, rng);
            rolls.push(RolledDie::new(chosen_die, face, order));
        }

        let winner_roll_idx = trick_winner(&rolls);
        let winner_player = (leader + winner_roll_idx) % player_count;
        tricks_won[winner_player] += 1;
        leader = winner_player;
    }

    tricks_won
}

/// Pick a die to play given suit-following constraints.
/// Simplified: always follow suit with the first matching die; otherwise play first available.
fn choose_die_to_play(
    _first_die: DieType,
    led_color: Option<DieType>,
    remaining_indices: &[usize],
    hand: &[DieType],
) -> DieType {
    if let Some(color) = led_color {
        // Try to follow suit.
        for &idx in remaining_indices {
            if hand[idx] == color {
                return hand[idx];
            }
        }
    }
    // No match or no suit led: play first remaining.
    hand[remaining_indices[0]]
}

/// Roll a random face from a die.
fn random_face(die: DieType, rng: &mut impl Rng) -> Face {
    let faces = die.faces();
    faces[rng.next_usize(faces.len())]
}

/// Compute score for one player in one round.
fn score_player(bid: usize, tricks: usize, hand_size: usize) -> i64 {
    if bid == 0 {
        if tricks == 0 {
            (10 * hand_size) as i64
        } else {
            -((10 * hand_size) as i64)
        }
    } else if tricks == bid {
        (20 * bid) as i64
    } else {
        -(10 * (tricks as i64 - bid as i64).unsigned_abs() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_counts_correct() {
        assert_eq!(round_count(3), 8);
        assert_eq!(round_count(4), 8);
        assert_eq!(round_count(5), 7);
        assert_eq!(round_count(6), 6);
    }

    #[test]
    fn trick_count_dist_sums_to_one() {
        let hand = vec![DieType::Red, DieType::Yellow, DieType::Purple];
        let opp = vec![vec![DieType::Red, DieType::Yellow, DieType::Purple]];
        let dist = trick_count_distribution(&hand, &opp);
        let total: f64 = dist.iter().sum();
        assert!((total - 1.0).abs() < 1e-9, "sum={total}");
    }

    #[test]
    fn dist_length_equals_hand_size_plus_one() {
        let hand = vec![DieType::Red, DieType::Red];
        let opp = vec![vec![DieType::Yellow, DieType::Yellow]];
        let dist = trick_count_distribution(&hand, &opp);
        assert_eq!(dist.len(), hand.len() + 1);
    }

    #[test]
    fn optimal_bid_is_bounded() {
        let hand = vec![DieType::Red, DieType::Yellow, DieType::Purple];
        let opp = vec![vec![DieType::Gray, DieType::Gray, DieType::Gray]];
        let dist = trick_count_distribution(&hand, &opp);
        let bid = optimal_bid(&dist);
        assert!(bid <= hand.len());
    }

    #[test]
    fn simulate_games_returns_correct_shape() {
        let mut rng = Xorshift64::new(42);
        let scores = simulate_games(4, 10, &mut rng);
        assert_eq!(scores.len(), 4);
        for p in &scores {
            assert_eq!(p.len(), 10);
        }
    }

    #[test]
    fn score_exact_bid() {
        assert_eq!(score_player(3, 3, 5), 60);
    }

    #[test]
    fn score_missed_bid() {
        assert_eq!(score_player(3, 1, 5), -20);
    }

    #[test]
    fn score_zero_bid_success() {
        assert_eq!(score_player(0, 0, 5), 50);
    }

    #[test]
    fn score_zero_bid_fail() {
        assert_eq!(score_player(0, 2, 5), -50);
    }

    #[test]
    fn sample_hand_correct_size() {
        let mut rng = Xorshift64::new(99);
        let hand = sample_hand(5, &mut rng);
        assert_eq!(hand.len(), 5);
    }

    #[test]
    fn xorshift_not_stuck() {
        let mut rng = Xorshift64::new(1);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            seen.insert(rng.next_usize(1000));
        }
        assert!(seen.len() > 50, "RNG appears to be stuck");
    }
}
