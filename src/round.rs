use crate::dice::DieType;
use crate::dice::Face;
use crate::trick::{RolledDie, trick_winner, win_probability};

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
        .flat_map(|&dt| std::iter::repeat_n(dt, dt.bag_count() as usize))
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
pub fn trick_count_distribution(hand: &[DieType], opponent_dice: &[Vec<DieType>]) -> Vec<f64> {
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

pub fn exact_single_trick_distribution(
    player_die: DieType,
    player_count: usize,
    player_position: usize,
) -> Vec<f64> {
    assert!((3..=6).contains(&player_count), "player_count must be 3–6");
    assert!(
        player_position < player_count,
        "player_position out of range"
    );

    let die_types = DieType::ALL;
    let mut remaining_counts: Vec<usize> = die_types
        .iter()
        .map(|&dt| dt.bag_count() as usize)
        .collect();
    let player_idx = die_types
        .iter()
        .position(|&dt| dt == player_die)
        .expect("player die must be in DieType::ALL");
    remaining_counts[player_idx] -= 1;

    let mut opp_dice = Vec::with_capacity(player_count - 1);
    let mut win_prob = 0.0;
    let mut state = OpponentDrawState {
        slots_left: player_count - 1,
        die_types: &die_types,
        remaining_counts: &mut remaining_counts,
        remaining_total: 35,
        draw_prob: 1.0,
    };
    enumerate_ordered_opponent_draws(
        player_die,
        player_position,
        &mut state,
        &mut opp_dice,
        &mut win_prob,
    );

    vec![1.0 - win_prob, win_prob]
}

fn enumerate_ordered_opponent_draws(
    player_die: DieType,
    player_position: usize,
    state: &mut OpponentDrawState<'_>,
    opp_dice: &mut Vec<DieType>,
    win_prob: &mut f64,
) {
    if state.slots_left == 0 {
        let mut all_dice = Vec::with_capacity(opp_dice.len() + 1);
        let mut opp_iter = opp_dice.iter().copied();
        for seat in 0..(opp_dice.len() + 1) {
            if seat == player_position {
                all_dice.push(player_die);
            } else {
                all_dice.push(opp_iter.next().expect("missing opponent die"));
            }
        }
        *win_prob += state.draw_prob * win_probability(&all_dice, None, player_position);
        return;
    }

    for (idx, &die_type) in state.die_types.iter().enumerate() {
        let count = state.remaining_counts[idx];
        if count == 0 {
            continue;
        }
        state.remaining_counts[idx] -= 1;
        opp_dice.push(die_type);
        let saved_slots_left = state.slots_left;
        let saved_remaining_total = state.remaining_total;
        let saved_draw_prob = state.draw_prob;
        state.slots_left -= 1;
        state.remaining_total -= 1;
        state.draw_prob *= count as f64 / saved_remaining_total as f64;
        enumerate_ordered_opponent_draws(player_die, player_position, state, opp_dice, win_prob);
        state.slots_left = saved_slots_left;
        state.remaining_total = saved_remaining_total;
        state.draw_prob = saved_draw_prob;
        opp_dice.pop();
        state.remaining_counts[idx] += 1;
    }
}

struct OpponentDrawState<'a> {
    slots_left: usize,
    die_types: &'a [DieType],
    remaining_counts: &'a mut [usize],
    remaining_total: usize,
    draw_prob: f64,
}

/// Estimates P(tricks = k) for a specific player hand by Monte Carlo simulation.
pub fn monte_carlo_trick_count_distribution(
    player_hand: &[DieType],
    player_count: usize,
    player_position: usize,
    n_samples: usize,
    rng: &mut impl Rng,
) -> Vec<f64> {
    assert!((3..=6).contains(&player_count), "player_count must be 3–6");
    assert!(
        player_position < player_count,
        "player_position out of range"
    );
    assert!(n_samples > 0, "n_samples must be > 0");

    let hand_size = player_hand.len();
    if hand_size == 0 {
        return vec![1.0];
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let worker_count = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1)
            .min(n_samples.max(1));
        if worker_count > 1 {
            let chunk_size = n_samples.div_ceil(worker_count);
            let player_hand = player_hand.to_vec();
            let mut jobs = Vec::new();
            let mut remaining_samples = n_samples;

            while remaining_samples > 0 {
                let current_chunk = remaining_samples.min(chunk_size);
                remaining_samples -= current_chunk;
                let seed_hi = rng.next_usize(u32::MAX as usize) as u64;
                let seed_lo = rng.next_usize(u32::MAX as usize) as u64;
                let seed = (seed_hi << 32) ^ seed_lo ^ current_chunk as u64;
                jobs.push((current_chunk, seed));
            }

            let mut handles = Vec::with_capacity(jobs.len());
            for (chunk_samples, seed) in jobs {
                let hand = player_hand.clone();
                handles.push(std::thread::spawn(move || {
                    let mut local_rng = Xorshift64::new(seed);
                    sample_trick_count_distribution_counts(
                        &hand,
                        player_count,
                        player_position,
                        chunk_samples,
                        &mut local_rng,
                    )
                }));
            }

            let mut counts = vec![0usize; hand_size + 1];
            for handle in handles {
                let chunk_counts = handle.join().expect("monte carlo worker thread panicked");
                for (index, count) in chunk_counts.into_iter().enumerate() {
                    counts[index] += count;
                }
            }

            return counts
                .into_iter()
                .map(|count| count as f64 / n_samples as f64)
                .collect();
        }
    }

    let counts = sample_trick_count_distribution_counts(
        player_hand,
        player_count,
        player_position,
        n_samples,
        rng,
    );

    counts
        .into_iter()
        .map(|count| count as f64 / n_samples as f64)
        .collect()
}

fn sample_trick_count_distribution_counts(
    player_hand: &[DieType],
    player_count: usize,
    player_position: usize,
    n_samples: usize,
    rng: &mut impl Rng,
) -> Vec<usize> {
    let mut counts = vec![0usize; player_hand.len() + 1];
    for _ in 0..n_samples {
        let opponent_hands =
            sample_opponent_hands_from_remaining(player_hand, player_count - 1, rng);
        let hands = embed_player_hand(player_hand, &opponent_hands, player_count, player_position);

        let outcome = simulate_round(&hands, player_position, rng);
        counts[outcome.tricks_won[player_position]] += 1;
    }
    counts
}

pub fn monte_carlo_special_capture_stats(
    player_hand: &[DieType],
    player_count: usize,
    player_position: usize,
    n_samples: usize,
    rng: &mut impl Rng,
) -> SpecialCaptureStats {
    assert!((3..=6).contains(&player_count), "player_count must be 3–6");
    assert!(
        player_position < player_count,
        "player_position out of range"
    );
    assert!(n_samples > 0, "n_samples must be > 0");

    if player_hand.is_empty() {
        return SpecialCaptureStats::default();
    }

    let mut mermaid_capture_wins = 0usize;
    let mut minotaur_capture_wins = 0usize;
    let mut total_bonus_points = 0i64;

    for _ in 0..n_samples {
        let opponent_hands =
            sample_opponent_hands_from_remaining(player_hand, player_count - 1, rng);
        let hands = embed_player_hand(player_hand, &opponent_hands, player_count, player_position);
        let outcome = simulate_round(&hands, player_position, rng);
        if outcome.mermaid_capture_won[player_position] {
            mermaid_capture_wins += 1;
        }
        if outcome.minotaur_capture_won[player_position] {
            minotaur_capture_wins += 1;
        }
        total_bonus_points += outcome.bonus_points[player_position];
    }

    SpecialCaptureStats {
        mermaid_captures_minotaur_prob: mermaid_capture_wins as f64 / n_samples as f64,
        minotaur_captures_griffin_prob: minotaur_capture_wins as f64 / n_samples as f64,
        expected_bonus_points: total_bonus_points as f64 / n_samples as f64,
    }
}

/// Returns the expected number of tricks won from a given distribution.
pub fn expected_tricks(dist: &[f64]) -> f64 {
    dist.iter().enumerate().map(|(k, &p)| k as f64 * p).sum()
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

/// Computes the actual score for one round given the bid placed and tricks won.
///
/// Scoring rules:
/// - Bid 0: win 0 tricks → +10 × hand_size; win any → −10 × hand_size
/// - Bid N > 0: exact → +20 × N; miss by d → −10 × d
pub fn score_for_outcome(bid: usize, tricks: usize, hand_size: usize) -> i64 {
    if bid == 0 {
        if tricks == 0 {
            (10 * hand_size) as i64
        } else {
            -((10 * hand_size) as i64)
        }
    } else if tricks == bid {
        (20 * bid) as i64
    } else {
        let diff = (tricks as i64 - bid as i64).unsigned_abs();
        -((10 * diff) as i64)
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoundOutcome {
    tricks_won: Vec<usize>,
    bonus_points: Vec<i64>,
    mermaid_capture_won: Vec<bool>,
    minotaur_capture_won: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpecialCaptureStats {
    pub mermaid_captures_minotaur_prob: f64,
    pub minotaur_captures_griffin_prob: f64,
    pub expected_bonus_points: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct TrickBonusOutcome {
    points: i64,
    mermaid_captures_minotaur: bool,
    minotaur_captures_griffin: bool,
}

/// A likely opponent hand pattern drawn from the remaining bag.
#[derive(Debug, Clone, PartialEq)]
pub struct OpponentHandPattern {
    pub hand: Vec<DieType>,
    pub probability: f64,
}

/// Simulate `n_games` complete games for `player_count` players, returning per-player
/// total-score distributions (vector of length `n_games` per player).
pub fn simulate_games(player_count: usize, n_games: usize, rng: &mut impl Rng) -> Vec<Vec<i64>> {
    let rounds = round_count(player_count);
    let mut all_scores: Vec<Vec<i64>> = vec![vec![]; player_count];

    for _ in 0..n_games {
        let mut totals = vec![0i64; player_count];

        for round in 1..=rounds {
            // Each player draws `round` dice.
            let hands: Vec<Vec<DieType>> =
                (0..player_count).map(|_| sample_hand(round, rng)).collect();

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
            let outcome = simulate_round(&hands, 0, rng);

            // Score each player.
            for p in 0..player_count {
                totals[p] += score_player(
                    bids[p],
                    outcome.tricks_won[p],
                    round,
                    outcome.bonus_points[p],
                );
            }
        }

        for p in 0..player_count {
            all_scores[p].push(totals[p]);
        }
    }
    all_scores
}

/// Simulate a specific round number `round_number` across `n_games` samples.
///
/// In Mino Dice, the number of dice drawn in a round equals the round number,
/// so this function draws `round_number` dice per player and returns the score
/// distribution for that single round only.
pub fn simulate_round_number(
    player_count: usize,
    round_number: usize,
    n_games: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<i64>> {
    assert!((3..=6).contains(&player_count), "player_count must be 3–6");
    assert!(round_number >= 1, "round_number must be >= 1");
    assert!(
        round_number <= round_count(player_count),
        "round_number exceeds game length"
    );

    let mut all_scores: Vec<Vec<i64>> = vec![vec![]; player_count];

    for _ in 0..n_games {
        let hands: Vec<Vec<DieType>> = (0..player_count)
            .map(|_| sample_hand(round_number, rng))
            .collect();

        let bids: Vec<usize> = hands
            .iter()
            .enumerate()
            .map(|(p, hand)| {
                let opp_dice: Vec<Vec<DieType>> = (0..player_count)
                    .filter(|&o| o != p)
                    .map(|o| hands[o].clone())
                    .collect();
                let dist = trick_count_distribution(hand, &opp_dice);
                optimal_bid(&dist)
            })
            .collect();

        let outcome = simulate_round(&hands, 0, rng);
        for p in 0..player_count {
            all_scores[p].push(score_player(
                bids[p],
                outcome.tricks_won[p],
                round_number,
                outcome.bonus_points[p],
            ));
        }
    }

    all_scores
}

pub fn simulate_round_for_player_hand(
    player_hand: &[DieType],
    player_count: usize,
    n_games: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<i64>> {
    assert!((3..=6).contains(&player_count), "player_count must be 3–6");
    assert!(n_games > 0, "n_games must be > 0");

    let round_number = player_hand.len();
    assert!(round_number > 0, "player_hand must not be empty");
    assert!(
        round_number <= round_count(player_count),
        "player hand size exceeds legal round count"
    );

    let mut all_scores: Vec<Vec<i64>> = vec![vec![]; player_count];

    for _ in 0..n_games {
        let opponent_hands =
            sample_opponent_hands_from_remaining(player_hand, player_count - 1, rng);
        let hands = embed_player_hand(player_hand, &opponent_hands, player_count, 0);

        let bids: Vec<usize> = hands
            .iter()
            .enumerate()
            .map(|(p, hand)| {
                let opp_dice: Vec<Vec<DieType>> = (0..player_count)
                    .filter(|&o| o != p)
                    .map(|o| hands[o].clone())
                    .collect();
                let dist = trick_count_distribution(hand, &opp_dice);
                optimal_bid(&dist)
            })
            .collect();

        let outcome = simulate_round(&hands, 0, rng);
        for p in 0..player_count {
            all_scores[p].push(score_player(
                bids[p],
                outcome.tricks_won[p],
                round_number,
                outcome.bonus_points[p],
            ));
        }
    }

    all_scores
}

fn sample_opponent_hands_from_remaining(
    player_hand: &[DieType],
    n_opponents: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<DieType>> {
    if n_opponents == 0 {
        return Vec::new();
    }

    let hand_size = player_hand.len();
    let total_needed = hand_size * n_opponents;
    let mut remaining: Vec<DieType> = Vec::new();
    for &dt in DieType::ALL.iter() {
        let used = player_hand.iter().filter(|&&die| die == dt).count();
        let available = dt.bag_count() as usize;
        assert!(
            used <= available,
            "player hand exceeds bag count for {dt:?}"
        );
        remaining.extend(std::iter::repeat_n(dt, available - used));
    }
    assert!(
        total_needed <= remaining.len(),
        "not enough dice remaining for opponents"
    );

    for i in 0..total_needed {
        let j = i + rng.next_usize(remaining.len() - i);
        remaining.swap(i, j);
    }

    remaining[..total_needed]
        .chunks(hand_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

/// Enumerates the most likely unordered opponent hands given the player's current hand.
///
/// The probability is computed from the exact multivariate-hypergeometric count:
/// choose `hand_size` dice from the remaining bag after removing the player's hand.
pub fn top_opponent_hand_patterns(
    player_hand: &[DieType],
    hand_size: usize,
    limit: usize,
) -> Vec<OpponentHandPattern> {
    if hand_size == 0 || limit == 0 {
        return Vec::new();
    }

    let die_types = DieType::ALL;
    let remaining_counts: Vec<usize> = die_types
        .iter()
        .map(|&dt| {
            let used = player_hand.iter().filter(|&&d| d == dt).count();
            dt.bag_count() as usize - used
        })
        .collect();
    let remaining_total: usize = remaining_counts.iter().sum();
    if hand_size > remaining_total {
        return Vec::new();
    }

    let denom = combinations(remaining_total, hand_size) as f64;
    let mut counts = vec![0usize; die_types.len()];
    let mut patterns = Vec::new();
    enumerate_hand_patterns(
        &die_types,
        &remaining_counts,
        0,
        hand_size,
        &mut counts,
        denom,
        &mut patterns,
    );

    patterns.sort_by(|a, b| {
        b.probability
            .partial_cmp(&a.probability)
            .unwrap()
            .then_with(|| a.hand.len().cmp(&b.hand.len()))
            .then_with(|| format!("{:?}", a.hand).cmp(&format!("{:?}", b.hand)))
    });
    patterns.truncate(limit);
    patterns
}

fn enumerate_hand_patterns(
    die_types: &[DieType],
    remaining_counts: &[usize],
    index: usize,
    remaining_slots: usize,
    counts: &mut [usize],
    denom: f64,
    out: &mut Vec<OpponentHandPattern>,
) {
    if index == die_types.len() - 1 {
        if remaining_slots <= remaining_counts[index] {
            counts[index] = remaining_slots;
            let mut numer = 1u128;
            let mut hand = Vec::new();
            for (i, &count) in counts.iter().enumerate() {
                numer *= combinations(remaining_counts[i], count);
                hand.extend(std::iter::repeat_n(die_types[i], count));
            }
            out.push(OpponentHandPattern {
                hand,
                probability: numer as f64 / denom,
            });
        }
        return;
    }

    let max_take = remaining_slots.min(remaining_counts[index]);
    for take in 0..=max_take {
        counts[index] = take;
        enumerate_hand_patterns(
            die_types,
            remaining_counts,
            index + 1,
            remaining_slots - take,
            counts,
            denom,
            out,
        );
    }
}

fn combinations(n: usize, k: usize) -> u128 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result = 1u128;
    for i in 0..k {
        result = result * (n - i) as u128 / (i + 1) as u128;
    }
    result
}

/// Simulate one round trick-by-trick, returning tricks won and bonus points per player.
fn simulate_round(
    hands: &[Vec<DieType>],
    starting_leader: usize,
    rng: &mut impl Rng,
) -> RoundOutcome {
    let player_count = hands.len();
    let num_tricks = hands[0].len();
    let mut tricks_won = vec![0usize; player_count];
    let mut bonus_points = vec![0i64; player_count];
    let mut mermaid_capture_won = vec![false; player_count];
    let mut minotaur_capture_won = vec![false; player_count];

    // Keep a pool of remaining dice per player (indices into their hand).
    let mut remaining: Vec<Vec<usize>> = (0..player_count)
        .map(|p| (0..hands[p].len()).collect())
        .collect();

    let mut leader = starting_leader;

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
            let chosen_die_idx = choose_die_to_play(led_color, &rolls, &remaining[p], &hands[p]);
            let chosen_die = hands[p][chosen_die_idx];
            // Remove from remaining.
            if let Some(pos) = remaining[p].iter().position(|&i| i == chosen_die_idx) {
                remaining[p].remove(pos);
            }
            let face = random_face(chosen_die, rng);
            rolls.push(RolledDie::new(chosen_die, face, order));
        }

        let winner_roll_idx = trick_winner(&rolls);
        let winner_player = (leader + winner_roll_idx) % player_count;
        tricks_won[winner_player] += 1;
        let bonus = trick_bonus_points(&rolls, winner_roll_idx);
        bonus_points[winner_player] += bonus.points;
        mermaid_capture_won[winner_player] |= bonus.mermaid_captures_minotaur;
        minotaur_capture_won[winner_player] |= bonus.minotaur_captures_griffin;
        leader = winner_player;
    }

    RoundOutcome {
        tricks_won,
        bonus_points,
        mermaid_capture_won,
        minotaur_capture_won,
    }
}

/// Pick a die to play given suit-following constraints.
/// Legal plays are:
/// - any special die,
/// - any die matching the led color,
/// - or any die if the player has neither.
///
/// The simulator remains deterministic and chooses the first legal die by hand order.
fn choose_die_to_play(
    led_color: Option<DieType>,
    current_rolls: &[RolledDie],
    remaining_indices: &[usize],
    hand: &[DieType],
) -> usize {
    let legal_indices = legal_die_choices(led_color, remaining_indices, hand);
    let mut losing_choices = Vec::new();
    for idx in legal_indices.iter().copied() {
        let win_prob = die_win_probability_against_current_rolls(hand[idx], current_rolls);
        if win_prob < 1.0 - 1e-12 {
            losing_choices.push(idx);
        }
    }

    let candidates = if losing_choices.is_empty() {
        legal_indices
    } else {
        losing_choices
    };

    candidates
        .into_iter()
        .min_by(|&left, &right| {
            let left_prob = die_win_probability_against_current_rolls(hand[left], current_rolls);
            let right_prob = die_win_probability_against_current_rolls(hand[right], current_rolls);
            left_prob
                .partial_cmp(&right_prob)
                .unwrap()
                .then_with(|| die_strength_key(hand[left]).cmp(&die_strength_key(hand[right])))
                .then_with(|| left.cmp(&right))
        })
        .expect("at least one legal die choice is required")
}

fn legal_die_choices(
    led_color: Option<DieType>,
    remaining_indices: &[usize],
    hand: &[DieType],
) -> Vec<usize> {
    let specials: Vec<usize> = remaining_indices
        .iter()
        .copied()
        .filter(|&idx| hand[idx].is_special_die())
        .collect();
    if let Some(color) = led_color {
        let matching: Vec<usize> = remaining_indices
            .iter()
            .copied()
            .filter(|&idx| hand[idx] == color)
            .collect();
        if !matching.is_empty() {
            let mut legal = specials;
            legal.extend(matching);
            legal.sort_unstable();
            legal.dedup();
            return legal;
        }
    }
    remaining_indices.to_vec()
}

fn die_win_probability_against_current_rolls(die: DieType, current_rolls: &[RolledDie]) -> f64 {
    let faces = merged_die_faces(die);
    faces
        .into_iter()
        .filter_map(|(face, prob)| {
            let mut rolls = current_rolls.to_vec();
            rolls.push(RolledDie::new(die, face, current_rolls.len()));
            let winner = trick_winner(&rolls);
            if winner == current_rolls.len() {
                Some(prob)
            } else {
                None
            }
        })
        .sum()
}

fn die_strength_key(die: DieType) -> (u8, u8) {
    match die {
        DieType::Gray => (0, 0),
        DieType::Purple => (1, 0),
        DieType::Yellow => (2, 0),
        DieType::Red => (3, 0),
        DieType::Mermaid => (4, 0),
        DieType::Griffin => (5, 0),
        DieType::Minotaur => (6, 0),
    }
}

fn embed_player_hand(
    player_hand: &[DieType],
    opponent_hands: &[Vec<DieType>],
    player_count: usize,
    player_position: usize,
) -> Vec<Vec<DieType>> {
    assert_eq!(opponent_hands.len(), player_count - 1);
    let mut hands = Vec::with_capacity(player_count);
    let mut opp_iter = opponent_hands.iter().cloned();
    for seat in 0..player_count {
        if seat == player_position {
            hands.push(player_hand.to_vec());
        } else {
            hands.push(opp_iter.next().expect("missing opponent hand"));
        }
    }
    hands
}

fn trick_bonus_points(rolls: &[RolledDie], winner_roll_idx: usize) -> TrickBonusOutcome {
    let winner_face = rolls[winner_roll_idx].face;
    let any_flag = rolls.iter().any(|roll| roll.face == Face::Flag);
    if any_flag {
        return TrickBonusOutcome::default();
    }

    if winner_face == Face::Mermaid && rolls.iter().any(|roll| roll.face == Face::Minotaur) {
        return TrickBonusOutcome {
            points: 50,
            mermaid_captures_minotaur: true,
            minotaur_captures_griffin: false,
        };
    }

    if winner_face == Face::Minotaur && rolls.iter().any(|roll| roll.face == Face::Griffin) {
        return TrickBonusOutcome {
            points: 30,
            mermaid_captures_minotaur: false,
            minotaur_captures_griffin: true,
        };
    }

    TrickBonusOutcome::default()
}

/// Roll a random face from a die.
fn random_face(die: DieType, rng: &mut impl Rng) -> Face {
    let faces = die.faces();
    faces[rng.next_usize(faces.len())]
}

fn merged_die_faces(die: DieType) -> Vec<(Face, f64)> {
    let faces = die.faces();
    let prob = 1.0 / faces.len() as f64;
    let mut map: std::collections::HashMap<Face, f64> = std::collections::HashMap::new();
    for face in faces {
        *map.entry(face).or_insert(0.0) += prob;
    }
    let mut merged: Vec<(Face, f64)> = map.into_iter().collect();
    merged.sort_by_key(|(face, _)| match face {
        Face::Flag => 0,
        Face::Number(value) => *value as u16,
        Face::Mermaid => 100,
        Face::Griffin => 101,
        Face::Minotaur => 102,
    });
    merged
}

/// Compute score for one player in one round, including special-capture bonuses.
fn score_player(bid: usize, tricks: usize, hand_size: usize, bonus_points: i64) -> i64 {
    let base_score = if bid == 0 {
        if tricks == 0 {
            (10 * hand_size) as i64
        } else {
            -((10 * hand_size) as i64)
        }
    } else if tricks == bid {
        (20 * bid) as i64
    } else {
        -(10 * (tricks as i64 - bid as i64).unsigned_abs() as i64)
    };

    base_score + bonus_points
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
    fn simulate_round_number_returns_correct_shape() {
        let mut rng = Xorshift64::new(42);
        let scores = simulate_round_number(4, 3, 10, &mut rng);
        assert_eq!(scores.len(), 4);
        for p in &scores {
            assert_eq!(p.len(), 10);
        }
    }

    #[test]
    fn monte_carlo_trick_count_distribution_has_valid_shape() {
        let mut rng = Xorshift64::new(42);
        let hand = vec![DieType::Red, DieType::Yellow, DieType::Gray];
        let dist = monte_carlo_trick_count_distribution(&hand, 4, 0, 200, &mut rng);
        assert_eq!(dist.len(), hand.len() + 1);
        assert!((dist.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn exact_single_trick_distribution_mermaid_three_players() {
        let dist = exact_single_trick_distribution(DieType::Mermaid, 3, 0);
        assert_eq!(dist.len(), 2);
        assert!(
            (dist[1] - 0.5680672268907563).abs() < 1e-12,
            "dist={dist:?}"
        );
        assert!((dist.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn exact_single_trick_distribution_changes_with_play_order() {
        let lead_dist = exact_single_trick_distribution(DieType::Gray, 3, 0);
        let last_dist = exact_single_trick_distribution(DieType::Gray, 3, 2);
        assert!((lead_dist[1] - last_dist[1]).abs() > 1e-9);
    }

    #[test]
    fn simulate_round_for_player_hand_returns_correct_shape() {
        let mut rng = Xorshift64::new(42);
        let hand = vec![DieType::Red, DieType::Yellow, DieType::Gray];
        let scores = simulate_round_for_player_hand(&hand, 4, 10, &mut rng);
        assert_eq!(scores.len(), 4);
        for p in &scores {
            assert_eq!(p.len(), 10);
        }
    }

    #[test]
    fn score_exact_bid() {
        assert_eq!(score_player(3, 3, 5, 0), 60);
    }

    #[test]
    fn score_missed_bid() {
        assert_eq!(score_player(3, 1, 5, 0), -20);
    }

    #[test]
    fn score_zero_bid_success() {
        assert_eq!(score_player(0, 0, 5, 0), 50);
    }

    #[test]
    fn score_zero_bid_fail() {
        assert_eq!(score_player(0, 2, 5, 0), -50);
    }

    #[test]
    fn score_includes_bonus_points() {
        assert_eq!(score_player(1, 1, 3, 50), 70);
    }

    #[test]
    fn choose_die_allows_special_even_with_matching_color() {
        let hand = vec![DieType::Red, DieType::Mermaid, DieType::Yellow];
        let remaining = vec![0usize, 1, 2];
        let current_rolls = vec![RolledDie::new(DieType::Red, Face::Number(5), 0)];
        let chosen = choose_die_to_play(Some(DieType::Red), &current_rolls, &remaining, &hand);
        assert_eq!(chosen, 1);
        assert_eq!(hand[chosen], DieType::Mermaid);
    }

    #[test]
    fn choose_die_prefers_weaker_losing_option() {
        let hand = vec![DieType::Minotaur, DieType::Gray, DieType::Red];
        let remaining = vec![0usize, 1, 2];
        let current_rolls = vec![RolledDie::new(DieType::Yellow, Face::Number(5), 0)];
        let chosen = choose_die_to_play(Some(DieType::Yellow), &current_rolls, &remaining, &hand);
        assert_eq!(chosen, 1);
        assert_eq!(hand[chosen], DieType::Gray);
    }

    #[test]
    fn trick_bonus_mermaid_captures_minotaur_without_flag() {
        let rolls = vec![
            RolledDie::new(DieType::Minotaur, Face::Minotaur, 0),
            RolledDie::new(DieType::Mermaid, Face::Mermaid, 1),
            RolledDie::new(DieType::Red, Face::Number(7), 2),
        ];
        assert_eq!(
            trick_bonus_points(&rolls, 1),
            TrickBonusOutcome {
                points: 50,
                mermaid_captures_minotaur: true,
                minotaur_captures_griffin: false,
            }
        );
    }

    #[test]
    fn trick_bonus_blocked_when_any_flag_captured() {
        let rolls = vec![
            RolledDie::new(DieType::Griffin, Face::Griffin, 0),
            RolledDie::new(DieType::Minotaur, Face::Minotaur, 1),
            RolledDie::new(DieType::Gray, Face::Flag, 2),
        ];
        assert_eq!(trick_bonus_points(&rolls, 1), TrickBonusOutcome::default());
    }

    #[test]
    fn monte_carlo_special_capture_stats_are_bounded() {
        let mut rng = Xorshift64::new(42);
        let hand = vec![DieType::Mermaid, DieType::Minotaur, DieType::Gray];
        let stats = monte_carlo_special_capture_stats(&hand, 3, 0, 200, &mut rng);
        assert!((0.0..=1.0).contains(&stats.mermaid_captures_minotaur_prob));
        assert!((0.0..=1.0).contains(&stats.minotaur_captures_griffin_prob));
    }

    #[test]
    fn monte_carlo_distribution_is_stable_for_four_dice_across_player_counts() {
        let hand = vec![
            DieType::Mermaid,
            DieType::Red,
            DieType::Yellow,
            DieType::Gray,
        ];

        for player_count in 3..=6 {
            let mut rng_a = Xorshift64::new(20260410 + player_count as u64);
            let mut rng_b = Xorshift64::new(20260420 + player_count as u64);

            let dist_a = monte_carlo_trick_count_distribution(&hand, player_count, 0, 10_000, &mut rng_a);
            let dist_b = monte_carlo_trick_count_distribution(&hand, player_count, 0, 10_000, &mut rng_b);

            let max_delta = dist_a
                .iter()
                .zip(&dist_b)
                .map(|(left, right)| (left - right).abs())
                .fold(0.0_f64, f64::max);

            assert!(
                max_delta < 0.03,
                "player_count={player_count}, dist_a={dist_a:?}, dist_b={dist_b:?}, max_delta={max_delta}"
            );
        }
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

    #[test]
    fn top_opponent_patterns_respects_limit() {
        let hand = vec![DieType::Minotaur, DieType::Red];
        let patterns = top_opponent_hand_patterns(&hand, 2, 3);
        assert_eq!(patterns.len(), 3);
        assert!(patterns[0].probability >= patterns[1].probability);
        assert!(patterns[1].probability >= patterns[2].probability);
    }

    #[test]
    fn top_opponent_patterns_sum_is_bounded() {
        let hand = vec![DieType::Red, DieType::Yellow];
        let patterns = top_opponent_hand_patterns(&hand, 2, 3);
        let total: f64 = patterns.iter().map(|p| p.probability).sum();
        assert!(total > 0.0);
        assert!(total < 1.0);
    }
}
