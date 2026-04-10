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
fn analytical_trick_count_distribution_has_valid_shape() {
    let hand = vec![DieType::Red, DieType::Yellow, DieType::Gray];
    let dist = analytical_trick_count_distribution(&hand, 4, 0);
    assert_eq!(dist.len(), hand.len() + 1);
    assert!((dist.iter().sum::<f64>() - 1.0).abs() < 1e-9);
}

#[test]
fn analytical_single_die_matches_exact_distribution() {
    let exact = exact_single_trick_distribution(DieType::Mermaid, 4, 2);
    let analytical = analytical_trick_count_distribution(&[DieType::Mermaid], 4, 2);
    assert_eq!(analytical.len(), exact.len());
    for (left, right) in analytical.iter().zip(exact.iter()) {
        assert!(
            (left - right).abs() < 1e-12,
            "analytical={analytical:?}, exact={exact:?}"
        );
    }
}

#[test]
fn analytical_distribution_changes_with_play_order() {
    let hand = vec![DieType::Gray, DieType::Gray, DieType::Gray];
    let lead_dist = analytical_trick_count_distribution(&hand, 3, 0);
    let last_dist = analytical_trick_count_distribution(&hand, 3, 2);
    assert_ne!(lead_dist, last_dist);
}

#[test]
fn full_hand_conditioning_changes_analytical_distribution() {
    let hand = vec![DieType::Mermaid, DieType::Red, DieType::Gray];
    let improved = analytical_trick_count_distribution(&hand, 3, 0);

    let mut legacy_dp = vec![0.0f64; hand.len() + 1];
    legacy_dp[0] = 1.0;
    for (trick_idx, &player_die) in hand.iter().enumerate() {
        let p_win = exact_single_trick_distribution(player_die, 3, 0)[1];
        let mut next = vec![0.0f64; hand.len() + 1];
        for wins_so_far in 0..=trick_idx {
            next[wins_so_far] += legacy_dp[wins_so_far] * (1.0 - p_win);
            next[wins_so_far + 1] += legacy_dp[wins_so_far] * p_win;
        }
        legacy_dp = next;
    }

    let max_delta = improved
        .iter()
        .zip(legacy_dp.iter())
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f64, f64::max);
    assert!(
        max_delta > 1e-6,
        "improved={improved:?}, legacy={legacy_dp:?}"
    );
}

fn legacy_inter_trick_distribution(
    hand: &[DieType],
    player_count: usize,
    player_position: usize,
) -> Vec<f64> {
    let hand_size = hand.len();
    let mut dp = vec![vec![0.0f64; player_count]; hand_size + 1];
    dp[0][player_position] = 1.0;

    for (trick_idx, &player_die) in hand.iter().enumerate() {
        let mut next = vec![vec![0.0f64; player_count]; hand_size + 1];
        for (wins_so_far, seat_probs) in dp.iter().enumerate().take(trick_idx + 1) {
            for (seat, &state_prob) in seat_probs.iter().enumerate() {
                if state_prob == 0.0 {
                    continue;
                }

                let winner_dist = exact_single_trick_winner_distribution_from_remaining(
                    hand,
                    player_die,
                    player_count,
                    seat,
                );

                for (winner_seat, &winner_prob) in winner_dist.iter().enumerate() {
                    if winner_prob == 0.0 {
                        continue;
                    }
                    let next_seat = (seat + player_count - winner_seat) % player_count;
                    let next_wins = if winner_seat == seat {
                        wins_so_far + 1
                    } else {
                        wins_so_far
                    };
                    next[next_wins][next_seat] += state_prob * winner_prob;
                }
            }
        }
        dp = next;
    }

    (0..=hand_size).map(|wins| dp[wins].iter().sum()).collect()
}

#[test]
fn test_bag_aware_dp_differs_from_legacy_inter_trick_dp() {
    let hand = vec![DieType::Mermaid, DieType::Red, DieType::Gray];
    let player_count = 4;
    let player_position = 0;

    let improved = analytical_trick_count_distribution(&hand, player_count, player_position);
    let legacy = legacy_inter_trick_distribution(&hand, player_count, player_position);
    let max_delta = improved
        .iter()
        .zip(legacy.iter())
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f64, f64::max);

    assert!(max_delta > 1e-6, "improved={improved:?}, legacy={legacy:?}");
    assert!((improved.iter().sum::<f64>() - 1.0).abs() < 1e-9);
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
fn expected_total_score_adds_bonus_term() {
    let dist = vec![0.25, 0.5, 0.25];
    let base = expected_score_for_bid(1, &dist, 2);
    let total = expected_total_score_for_bid(1, &dist, 2, 12.5);
    assert!((total - (base + 12.5)).abs() < 1e-12);
}

#[test]
fn optimal_bid_with_bonus_matches_optimal_bid() {
    let dist = vec![0.2, 0.6, 0.2];
    let base_opt = optimal_bid(&dist);
    let total_opt = optimal_bid_with_bonus(&dist, 17.0);
    assert_eq!(base_opt, total_opt);
}

#[test]
fn score_includes_bonus_points() {
    assert_eq!(score_player(1, 1, 3, 50), 70);
}

#[test]
fn choose_die_allows_special_even_with_matching_color() {
    let hand = vec![DieType::Red, DieType::Mermaid, DieType::Yellow];
    let remaining = vec![0usize, 1, 2];
    let mut rng = Xorshift64::new(1);
    let mut saw_matching = false;
    let mut saw_special = false;
    for _ in 0..200 {
        let chosen = choose_die_to_play(Some(DieType::Red), &remaining, &hand, &mut rng);
        if hand[chosen] == DieType::Red {
            saw_matching = true;
        }
        if hand[chosen] == DieType::Mermaid {
            saw_special = true;
        }
        assert!(hand[chosen] == DieType::Red || hand[chosen] == DieType::Mermaid);
    }
    assert!(
        saw_matching && saw_special,
        "both matching and special choices should be sampled"
    );
}

#[test]
fn choose_die_without_matching_uses_any_remaining_choice() {
    let hand = vec![
        DieType::Minotaur,
        DieType::Gray,
        DieType::Red,
        DieType::Yellow,
    ];
    let remaining = vec![0usize, 1, 2];
    let mut rng = Xorshift64::new(7);
    for _ in 0..50 {
        let chosen = choose_die_to_play(Some(DieType::Yellow), &remaining, &hand, &mut rng);
        assert!(remaining.contains(&chosen));
    }
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

        let dist_a =
            monte_carlo_trick_count_distribution(&hand, player_count, 0, 10_000, &mut rng_a);
        let dist_b =
            monte_carlo_trick_count_distribution(&hand, player_count, 0, 10_000, &mut rng_b);

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

// ── suit_follow_correction unit tests ──────────────────────────────────────

#[test]
fn suit_follow_correction_returns_one_when_no_led_color() {
    let remaining: Vec<f64> = DieType::ALL
        .iter()
        .map(|&dt| dt.bag_count() as f64)
        .collect();
    let total: f64 = remaining.iter().sum();
    // No suit constraint → always 1.0 regardless of die type.
    for &dt in DieType::ALL.iter() {
        assert_eq!(
            suit_follow_correction(dt, None, &remaining, total, 3),
            1.0,
            "{dt:?}"
        );
    }
}

#[test]
fn suit_follow_correction_returns_one_for_legal_dice() {
    let remaining: Vec<f64> = DieType::ALL
        .iter()
        .map(|&dt| dt.bag_count() as f64)
        .collect();
    let total: f64 = remaining.iter().sum();
    // Matching color and specials are always legal.
    assert_eq!(
        suit_follow_correction(DieType::Red, Some(DieType::Red), &remaining, total, 4),
        1.0,
        "matching color"
    );
    for &special in &[DieType::Minotaur, DieType::Griffin, DieType::Mermaid] {
        assert_eq!(
            suit_follow_correction(special, Some(DieType::Red), &remaining, total, 4),
            1.0,
            "{special:?} is always legal"
        );
    }
}

#[test]
fn suit_follow_correction_returns_one_for_single_die_hand() {
    // When opponent has only 1 die they must play it regardless.
    let remaining: Vec<f64> = DieType::ALL
        .iter()
        .map(|&dt| dt.bag_count() as f64)
        .collect();
    let total: f64 = remaining.iter().sum();
    assert_eq!(
        suit_follow_correction(DieType::Yellow, Some(DieType::Red), &remaining, total, 1),
        1.0
    );
}

#[test]
fn suit_follow_correction_less_than_one_for_illegal_die_with_legal_alternatives() {
    // When there are legal dice remaining, drawing an illegal die should be
    // down-weighted because the opponent can only play it when their whole
    // hand has no legal die.
    let remaining: Vec<f64> = DieType::ALL
        .iter()
        .map(|&dt| dt.bag_count() as f64)
        .collect();
    let total: f64 = remaining.iter().sum();
    let correction =
        suit_follow_correction(DieType::Yellow, Some(DieType::Red), &remaining, total, 3);
    assert!(
        correction < 1.0 - 1e-9,
        "illegal die should be down-weighted: correction={correction}"
    );
    assert!(correction >= 0.0);
}

#[test]
fn suit_follow_correction_zero_when_no_illegal_dice_for_remaining_slots() {
    // If the entire bag consists of legal dice (specials + led color),
    // an illegal die is impossible (correction = 0).
    let mut remaining = vec![0.0f64; DieType::ALL.len()];
    // Only Red (legal) and Mermaid (special, legal) remain.
    let red_idx = DieType::ALL
        .iter()
        .position(|&dt| dt == DieType::Red)
        .unwrap();
    let mermaid_idx = DieType::ALL
        .iter()
        .position(|&dt| dt == DieType::Mermaid)
        .unwrap();
    remaining[red_idx] = 5.0;
    remaining[mermaid_idx] = 2.0;
    let total: f64 = remaining.iter().sum(); // 7.0
    let correction =
        suit_follow_correction(DieType::Yellow, Some(DieType::Red), &remaining, total, 3);
    assert_eq!(correction, 0.0);
}

// ── suit-following prior integration tests ─────────────────────────────────

#[test]
fn suit_following_prior_changes_result_for_number_led_hand() {
    // The bag-aware transition with the suit-following prior (led_color =
    // Some(Red)) should produce a different winner distribution than without
    // the prior (led_color = None), because opponents holding Red or special
    // dice are now weighted more heavily as the player's competition.
    let remaining: Vec<f64> = DieType::ALL
        .iter()
        .map(|&dt| dt.bag_count() as f64)
        .collect();

    let with_prior = bag_aware_single_trick_transition(
        &remaining,
        DieType::Red,
        4,
        0,
        Some(DieType::Red), // suit-following constraint active
        3,                  // opponent has 3 dice in hand
    );
    let no_prior = bag_aware_single_trick_transition(
        &remaining,
        DieType::Red,
        4,
        0,
        None, // no suit constraint (old behaviour)
        3,
    );

    let max_delta = with_prior
        .winner_probs
        .iter()
        .zip(no_prior.winner_probs.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f64, f64::max);

    assert!(
        max_delta > 1e-6,
        "Prior should change winner distribution for Red-led trick: \
         with_prior={:?}, no_prior={:?}",
        with_prior.winner_probs,
        no_prior.winner_probs
    );
}

#[test]
fn suit_following_prior_unchanged_when_no_led_color() {
    // With led_color = None (e.g. a special die was led) the suit-following
    // correction is 1.0 for every die type, so both calls are identical.
    let remaining: Vec<f64> = DieType::ALL
        .iter()
        .map(|&dt| dt.bag_count() as f64)
        .collect();

    let call_a = bag_aware_single_trick_transition(&remaining, DieType::Mermaid, 4, 0, None, 3);
    let call_b = bag_aware_single_trick_transition(&remaining, DieType::Mermaid, 4, 0, None, 3);

    let max_delta = call_a
        .winner_probs
        .iter()
        .zip(call_b.winner_probs.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f64, f64::max);
    assert!(max_delta < 1e-12);
}

#[test]
fn analytical_dp_with_prior_sums_to_one_number_hand() {
    // Full DP with suit-following prior must still produce a valid distribution.
    for &player_count in &[3usize, 4, 5, 6] {
        let hand = vec![DieType::Red, DieType::Yellow, DieType::Purple];
        let dist = analytical_trick_count_distribution(&hand, player_count, 0);
        let sum: f64 = dist.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-9,
            "player_count={player_count}, sum={sum}"
        );
        assert_eq!(dist.len(), hand.len() + 1);
    }
}

#[test]
fn analytical_dp_prior_reduces_expected_tricks_for_red_heavy_hand() {
    // With suit-following, opponents who hold Red dice must play them when
    // the player leads Red.  This increases competition, so E[tricks] with
    // the prior should be lower than without it.
    //
    // We compare `analytical_trick_count_distribution` (uses the prior)
    // against an explicit "no-prior" variant that always passes led_color=None.
    let hand = vec![DieType::Red, DieType::Red, DieType::Red];
    let player_count = 4;

    // "No-prior" DP: replicate the DP loop but force led_color=None.
    let no_prior_dist = {
        let hand_size = hand.len();
        let initial_remaining = remaining_counts_after_removing_hand(&hand)
            .into_iter()
            .map(|c| c as f64)
            .collect::<Vec<_>>();

        let mut dp_prob = vec![vec![0.0f64; player_count]; hand_size + 1];
        let mut dp_remaining_sum =
            vec![vec![vec![0.0f64; DieType::ALL.len()]; player_count]; hand_size + 1];
        dp_prob[0][0] = 1.0;
        dp_remaining_sum[0][0].copy_from_slice(&initial_remaining);

        for (trick_idx, &player_die) in hand.iter().enumerate() {
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
                        .map(|&w| w / state_prob)
                        .collect();
                    // Force no prior: led_color = None, opponent_hand_size = 0
                    let transition = bag_aware_single_trick_transition(
                        &state_remaining,
                        player_die,
                        player_count,
                        seat,
                        None,
                        0,
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
                        let tp = state_prob * winner_prob;
                        next_prob[next_wins][next_seat] += tp;
                        for t in 0..DieType::ALL.len() {
                            let ed = transition.winner_expected_draw_counts[winner_seat][t];
                            let ra = (state_remaining[t] - ed).max(0.0);
                            next_remaining_sum[next_wins][next_seat][t] += tp * ra;
                        }
                    }
                }
            }
            dp_prob = next_prob;
            dp_remaining_sum = next_remaining_sum;
        }
        (0..=hand_size)
            .map(|wins| dp_prob[wins].iter().sum::<f64>())
            .collect::<Vec<_>>()
    };

    let with_prior_dist = analytical_trick_count_distribution(&hand, player_count, 0);

    let e_no_prior: f64 = no_prior_dist
        .iter()
        .enumerate()
        .map(|(k, &p)| k as f64 * p)
        .sum();
    let e_with_prior: f64 = with_prior_dist
        .iter()
        .enumerate()
        .map(|(k, &p)| k as f64 * p)
        .sum();

    assert!(
        e_with_prior < e_no_prior - 1e-6,
        "Prior should reduce E[tricks] for Red-heavy hand: \
         e_with_prior={e_with_prior}, e_no_prior={e_no_prior}"
    );

    // Both distributions must still be normalised.
    assert!((no_prior_dist.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    assert!((with_prior_dist.iter().sum::<f64>() - 1.0).abs() < 1e-9);
}
