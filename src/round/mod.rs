use crate::dice::DieType;
use crate::dice::Face;
use crate::trick::{RolledDie, trick_winner, win_probabilities_for_all_seats, win_probability};

const FLOAT_EPSILON: f64 = 1e-12;

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

    exact_single_trick_distribution_from_remaining(
        &[player_die],
        player_die,
        player_count,
        player_position,
    )
}

fn exact_single_trick_distribution_from_remaining(
    removed_player_dice: &[DieType],
    player_die: DieType,
    player_count: usize,
    player_position: usize,
) -> Vec<f64> {
    let winner_dist = exact_single_trick_winner_distribution_from_remaining(
        removed_player_dice,
        player_die,
        player_count,
        player_position,
    );
    let p_win = winner_dist[player_position];
    vec![1.0 - p_win, p_win]
}

fn exact_single_trick_winner_distribution_from_remaining(
    removed_player_dice: &[DieType],
    player_die: DieType,
    player_count: usize,
    player_position: usize,
) -> Vec<f64> {
    let die_types = DieType::ALL;
    let mut remaining_counts: Vec<usize> = die_types
        .iter()
        .map(|&dt| dt.bag_count() as usize)
        .collect();
    for &removed_die in removed_player_dice {
        let player_idx = die_types
            .iter()
            .position(|&dt| dt == removed_die)
            .expect("player die must be in DieType::ALL");
        remaining_counts[player_idx] -= 1;
    }

    let mut opponent_dice = Vec::with_capacity(player_count - 1);
    let mut winner_probs = vec![0.0f64; player_count];
    let mut state = OpponentDrawState {
        slots_left: player_count - 1,
        die_types: &die_types,
        remaining_counts: &mut remaining_counts,
        remaining_total: 36 - removed_player_dice.len(),
        draw_prob: 1.0,
    };
    enumerate_ordered_opponent_draws_winner_distribution(
        player_die,
        player_position,
        &mut state,
        &mut opponent_dice,
        &mut winner_probs,
    );

    winner_probs
}

fn enumerate_ordered_opponent_draws_winner_distribution(
    player_die: DieType,
    player_position: usize,
    state: &mut OpponentDrawState<'_>,
    opponent_dice: &mut Vec<DieType>,
    winner_probs: &mut [f64],
) {
    if state.slots_left == 0 {
        let mut all_dice = Vec::with_capacity(opponent_dice.len() + 1);
        let mut opp_iter = opponent_dice.iter().copied();
        for seat in 0..(opponent_dice.len() + 1) {
            if seat == player_position {
                all_dice.push(player_die);
            } else {
                all_dice.push(opp_iter.next().expect("missing opponent die"));
            }
        }

        let seat_win_probs = win_probabilities_for_all_seats(&all_dice);
        for seat in 0..winner_probs.len() {
            winner_probs[seat] += state.draw_prob * seat_win_probs[seat];
        }
        return;
    }

    for (idx, &die_type) in state.die_types.iter().enumerate() {
        let count = state.remaining_counts[idx];
        if count == 0 {
            continue;
        }
        state.remaining_counts[idx] -= 1;
        opponent_dice.push(die_type);
        let saved_slots_left = state.slots_left;
        let saved_remaining_total = state.remaining_total;
        let saved_draw_prob = state.draw_prob;
        state.slots_left -= 1;
        state.remaining_total -= 1;
        state.draw_prob *= count as f64 / saved_remaining_total as f64;
        enumerate_ordered_opponent_draws_winner_distribution(
            player_die,
            player_position,
            state,
            opponent_dice,
            winner_probs,
        );
        state.slots_left = saved_slots_left;
        state.remaining_total = saved_remaining_total;
        state.draw_prob = saved_draw_prob;
        opponent_dice.pop();
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

/// Estimates P(tricks = k) for a specific player hand using a bag-aware seat DP.
pub fn analytical_trick_count_distribution(
    player_hand: &[DieType],
    player_count: usize,
    player_position: usize,
) -> Vec<f64> {
    assert!((3..=6).contains(&player_count), "player_count must be 3–6");
    assert!(
        player_position < player_count,
        "player_position out of range"
    );

    let hand_size = player_hand.len();
    if hand_size == 0 {
        return vec![1.0];
    }
    if hand_size == 1 {
        return exact_single_trick_distribution(player_hand[0], player_count, player_position);
    }

    let initial_remaining = remaining_counts_after_removing_hand(player_hand)
        .into_iter()
        .map(|count| count as f64)
        .collect::<Vec<_>>();

    // Bag-aware seat DP:
    // - dp_prob[k][s]: probability mass of (wins=k, seat=s)
    // - dp_remaining_sum[k][s][t]: probability-weighted expected remaining
    //   count of die type `t` for that state.
    let mut dp_prob = vec![vec![0.0f64; player_count]; hand_size + 1];
    let mut dp_remaining_sum =
        vec![vec![vec![0.0f64; DieType::ALL.len()]; player_count]; hand_size + 1];
    dp_prob[0][player_position] = 1.0;
    dp_remaining_sum[0][player_position].copy_from_slice(&initial_remaining);

    for (trick_idx, &player_die) in player_hand.iter().enumerate() {
        let mut next_prob = vec![vec![0.0f64; player_count]; hand_size + 1];
        let mut next_remaining_sum =
            vec![vec![vec![0.0f64; DieType::ALL.len()]; player_count]; hand_size + 1];

        for wins_so_far in 0..=trick_idx {
            for seat in 0..player_count {
                let state_prob = dp_prob[wins_so_far][seat];
                if state_prob == 0.0 {
                    continue;
                }
                let state_remaining: Vec<f64> = dp_remaining_sum[wins_so_far][seat]
                    .iter()
                    .map(|&weighted| weighted / state_prob)
                    .collect();

                let led_color = if player_die.is_special_die() {
                    None
                } else {
                    Some(player_die)
                };
                let opponent_hand_size = hand_size - trick_idx;
                let transition = bag_aware_single_trick_transition(
                    &state_remaining,
                    player_die,
                    player_count,
                    seat,
                    led_color,
                    opponent_hand_size,
                );

                for (winner_seat, &winner_prob) in transition.winner_probs.iter().enumerate() {
                    if winner_prob == 0.0 {
                        continue;
                    }
                    let next_seat = (seat + player_count - winner_seat) % player_count;
                    let next_wins = if winner_seat == seat {
                        wins_so_far + 1
                    } else {
                        wins_so_far
                    };

                    let transition_prob = state_prob * winner_prob;
                    next_prob[next_wins][next_seat] += transition_prob;

                    for t in 0..DieType::ALL.len() {
                        let expected_draw = transition.winner_expected_draw_counts[winner_seat][t];
                        debug_assert!(
                            expected_draw <= state_remaining[t] + 1e-6,
                            "expected draw exceeds remaining count: expected_draw={expected_draw}, remaining={}",
                            state_remaining[t]
                        );
                        let remaining_after = (state_remaining[t] - expected_draw).max(0.0);
                        next_remaining_sum[next_wins][next_seat][t] +=
                            transition_prob * remaining_after;
                    }
                }
            }
        }

        dp_prob = next_prob;
        dp_remaining_sum = next_remaining_sum;
    }

    (0..=hand_size)
        .map(|wins| dp_prob[wins].iter().sum())
        .collect()
}

fn remaining_counts_after_removing_hand(player_hand: &[DieType]) -> Vec<usize> {
    let die_types = DieType::ALL;
    let mut remaining_counts: Vec<usize> = die_types
        .iter()
        .map(|&dt| dt.bag_count() as usize)
        .collect();
    for &removed_die in player_hand {
        let player_idx = die_types
            .iter()
            .position(|&dt| dt == removed_die)
            .expect("player die must be in DieType::ALL");
        remaining_counts[player_idx] -= 1;
    }
    remaining_counts
}

struct BagAwareTrickTransition {
    winner_probs: Vec<f64>,
    winner_expected_draw_counts: Vec<Vec<f64>>,
}

/// Returns the probability that an opponent who drew the illegal die `die_type`
/// (one that does not match `led_color` and is not a special) would actually
/// play it under suit-following rules.
///
/// An opponent may only play an illegal die when their entire hand contains no
/// legal die (no matching-color die and no special die).  The probability of
/// that event, given the current bag composition and hand size `h`, is the
/// hypergeometric void-in-suit probability:
///
/// ```text
/// P(no legal die in h-1 remaining slots | drew one illegal die)
///   = ∏_{j=0}^{h-2}  (rem_illegal_after - j) / (rem_after - j)
/// ```
///
/// where `rem_after = remaining_total - 1` (bag after the illegal draw) and
/// `rem_illegal_after = rem_after - rem_legal`.
///
/// Returns 1.0 when `led_color` is `None` (no suit constraint) or when the
/// die is legal (matching color or special).
fn suit_follow_correction(
    die_type: DieType,
    led_color: Option<DieType>,
    remaining_counts: &[f64],
    remaining_total: f64,
    opponent_hand_size: usize,
) -> f64 {
    let Some(color) = led_color else {
        return 1.0;
    };

    // Legal dice: same color as led or any special.
    if die_type == color || die_type.is_special_die() {
        return 1.0;
    }

    // Illegal die.  With only one die left the opponent must play it anyway.
    let h = opponent_hand_size;
    if h <= 1 {
        return 1.0;
    }

    // Count legal dice remaining in the bag before this draw.
    let rem_legal: f64 = DieType::ALL
        .iter()
        .enumerate()
        .filter(|&(_, &dt)| dt == color || dt.is_special_die())
        .map(|(i, _)| remaining_counts[i])
        .sum();

    // After drawing this illegal die the bag has `rem_after` dice left,
    // of which `rem_illegal_after` are non-legal.
    let rem_after = remaining_total - 1.0;
    let rem_illegal_after = rem_after - rem_legal;

    if rem_illegal_after < (h - 1) as f64 {
        // Not enough non-legal dice to fill the opponent's remaining slots.
        return 0.0;
    }

    // Falling-factorial form of the hypergeometric probability.
    let mut result = 1.0f64;
    for j in 0..(h - 1) {
        let num = rem_illegal_after - j as f64;
        let den = rem_after - j as f64;
        if num <= 0.0 || den <= 0.0 {
            return 0.0;
        }
        result *= num / den;
    }
    result
}

fn bag_aware_single_trick_transition(
    remaining_counts: &[f64],
    player_die: DieType,
    player_count: usize,
    player_position: usize,
    led_color: Option<DieType>,
    opponent_hand_size: usize,
) -> BagAwareTrickTransition {
    let mut winner_probs = vec![0.0f64; player_count];
    let mut winner_draw_weighted_sum = vec![vec![0.0f64; DieType::ALL.len()]; player_count];
    let mut opp_dice = Vec::with_capacity(player_count - 1);
    let mut draw_counts = vec![0usize; DieType::ALL.len()];
    enumerate_bag_aware_opponent_draws(
        remaining_counts.to_vec(),
        player_count - 1,
        1.0,
        &mut opp_dice,
        &mut draw_counts,
        led_color,
        opponent_hand_size,
        player_die,
        player_position,
        &mut winner_probs,
        &mut winner_draw_weighted_sum,
    );

    let mut winner_expected_draw_counts = vec![vec![0.0f64; DieType::ALL.len()]; player_count];
    for winner_seat in 0..player_count {
        let winner_prob = winner_probs[winner_seat];
        if winner_prob == 0.0 {
            continue;
        }
        for die_idx in 0..DieType::ALL.len() {
            winner_expected_draw_counts[winner_seat][die_idx] =
                winner_draw_weighted_sum[winner_seat][die_idx] / winner_prob;
        }
    }

    BagAwareTrickTransition {
        winner_probs,
        winner_expected_draw_counts,
    }
}

#[allow(clippy::too_many_arguments)]
fn enumerate_bag_aware_opponent_draws(
    remaining_counts: Vec<f64>,
    slots_left: usize,
    draw_prob: f64,
    opponent_dice: &mut Vec<DieType>,
    draw_counts: &mut [usize],
    led_color: Option<DieType>,
    opponent_hand_size: usize,
    player_die: DieType,
    player_position: usize,
    winner_probs: &mut [f64],
    winner_draw_weighted_sum: &mut [Vec<f64>],
) {
    if slots_left == 0 {
        let mut all_dice = Vec::with_capacity(opponent_dice.len() + 1);
        let mut opp_iter = opponent_dice.iter().copied();
        for seat in 0..(opponent_dice.len() + 1) {
            if seat == player_position {
                all_dice.push(player_die);
            } else {
                all_dice.push(opp_iter.next().expect("missing opponent die"));
            }
        }

        let seat_win_probs = win_probabilities_for_all_seats(&all_dice);
        for winner_seat in 0..winner_probs.len() {
            let weighted = draw_prob * seat_win_probs[winner_seat];
            winner_probs[winner_seat] += weighted;
            for die_idx in 0..DieType::ALL.len() {
                winner_draw_weighted_sum[winner_seat][die_idx] +=
                    weighted * draw_counts[die_idx] as f64;
            }
        }
        return;
    }

    let remaining_total: f64 = remaining_counts.iter().sum();
    if remaining_total <= 0.0 {
        return;
    }

    // Compute suit-following correction weights for each die type.
    // Legal dice (matching led color or specials) keep weight = count.
    // Illegal dice are down-weighted by the probability that the opponent
    // holds no legal die in their remaining hand slots, so their draw
    // probability reflects how often they would actually play that die.
    let slot_weights: Vec<f64> = DieType::ALL
        .iter()
        .enumerate()
        .map(|(die_idx, &die_type)| {
            let count = remaining_counts[die_idx];
            if count <= FLOAT_EPSILON {
                return 0.0;
            }
            let correction = suit_follow_correction(
                die_type,
                led_color,
                &remaining_counts,
                remaining_total,
                opponent_hand_size,
            );
            count * correction
        })
        .collect();

    let total_weight: f64 = slot_weights.iter().sum();
    if total_weight <= FLOAT_EPSILON {
        return;
    }

    for (die_idx, &die_type) in DieType::ALL.iter().enumerate() {
        let weight = slot_weights[die_idx];
        if weight <= FLOAT_EPSILON {
            continue;
        }

        let mut next_counts = remaining_counts.clone();
        next_counts[die_idx] = (next_counts[die_idx] - 1.0).max(0.0);
        let next_draw_prob = draw_prob * (weight / total_weight);

        opponent_dice.push(die_type);
        draw_counts[die_idx] += 1;
        enumerate_bag_aware_opponent_draws(
            next_counts,
            slots_left - 1,
            next_draw_prob,
            opponent_dice,
            draw_counts,
            led_color,
            opponent_hand_size,
            player_die,
            player_position,
            winner_probs,
            winner_draw_weighted_sum,
        );
        draw_counts[die_idx] -= 1;
        opponent_dice.pop();
    }
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

/// Computes expected total score by adding expected bonus points to the base
/// expected score.
///
/// This matches the full round scoring rule:
///   E[score | b] = E[base_score | b] + E[bonus_points]
pub fn expected_total_score_for_bid(
    bid: usize,
    dist: &[f64],
    hand_size: usize,
    expected_bonus_points: f64,
) -> f64 {
    expected_score_for_bid(bid, dist, hand_size) + expected_bonus_points
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

/// Returns the bid (0..=hand_size) that maximises expected total score.
///
/// Because bonus is an additive term independent of bid in this model, this is
/// equivalent to `optimal_bid` and provided for scoring-rule completeness.
pub fn optimal_bid_with_bonus(dist: &[f64], expected_bonus_points: f64) -> usize {
    let hand_size = dist.len() - 1;
    (0..=hand_size)
        .max_by(|&a, &b| {
            expected_total_score_for_bid(a, dist, hand_size, expected_bonus_points)
                .partial_cmp(&expected_total_score_for_bid(
                    b,
                    dist,
                    hand_size,
                    expected_bonus_points,
                ))
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
            let chosen_die_idx = choose_die_to_play(led_color, &remaining[p], &hands[p], rng);
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
/// If multiple legal choices exist, sample uniformly at random.
fn choose_die_to_play(
    led_color: Option<DieType>,
    remaining_indices: &[usize],
    hand: &[DieType],
    rng: &mut impl Rng,
) -> usize {
    let legal_indices = legal_die_choices(led_color, remaining_indices, hand);
    let idx = rng.next_usize(legal_indices.len());
    legal_indices[idx]
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
mod tests;
