/// Benchmark DP (analytical_trick_count_distribution) vs
/// Monte Carlo (monte_carlo_trick_count_distribution) across all player counts.
///
/// Usage:
///   cargo run --release --example benchmark_dp_vs_monte_carlo
use std::hint::black_box;
use std::time::Instant;

use mino_dice_prob_calc::dice::DieType;
use mino_dice_prob_calc::round::{
    Xorshift64, analytical_trick_count_distribution, monte_carlo_trick_count_distribution,
    round_count, sample_hand,
};

const REPLICATIONS: usize = 20;
const MC_SAMPLE_COUNTS: &[usize] = &[1_000, 5_000, 10_000];
const SEED: u64 = 20_260_410;

fn expected_tricks(dist: &[f64]) -> f64 {
    dist.iter().enumerate().map(|(k, &p)| k as f64 * p).sum()
}

fn bench_player_count(player_count: usize, seed: u64) {
    let hand_size = round_count(player_count);
    let player_position = 0;

    // Pre-generate all hands using a dedicated RNG so MC and DP see identical inputs.
    let mut rng = Xorshift64::new(seed);
    let hands: Vec<Vec<DieType>> = (0..REPLICATIONS)
        .map(|_| sample_hand(hand_size, &mut rng))
        .collect();

    // ── Warm-up (one call each to avoid cold-start effects) ───────────────────
    {
        let _ = black_box(analytical_trick_count_distribution(
            &hands[0],
            player_count,
            player_position,
        ));
        let mut warm_rng = Xorshift64::new(seed ^ 0xffff);
        let _ = black_box(monte_carlo_trick_count_distribution(
            &hands[0],
            player_count,
            player_position,
            MC_SAMPLE_COUNTS[0],
            &mut warm_rng,
        ));
    }

    println!("=== {player_count}-player  (hand_size={hand_size}, replications={REPLICATIONS}) ===");

    // ── DP timing ─────────────────────────────────────────────────────────────
    let dp_start = Instant::now();
    let dp_expected: Vec<f64> = hands
        .iter()
        .map(|hand| {
            let dist = analytical_trick_count_distribution(hand, player_count, player_position);
            let e = expected_tricks(&dist);
            black_box(dist);
            e
        })
        .collect();
    let dp_elapsed = dp_start.elapsed();

    let dp_ms = dp_elapsed.as_secs_f64() * 1_000.0;
    let dp_us_per = dp_elapsed.as_secs_f64() * 1_000_000.0 / REPLICATIONS as f64;
    println!("  DP (analytical):  total={dp_ms:.3} ms   avg={dp_us_per:.1} µs/hand");

    // ── Monte Carlo timing at each sample count ───────────────────────────────
    for &n_samples in MC_SAMPLE_COUNTS {
        // Use a fresh, deterministic RNG per sample count so timings are
        // comparable across runs.
        let mut mc_rng = Xorshift64::new(seed.wrapping_add(n_samples as u64));

        let mc_start = Instant::now();
        let mut max_expected_tricks_gap = 0.0f64;
        for (i, hand) in hands.iter().enumerate() {
            let dist = monte_carlo_trick_count_distribution(
                hand,
                player_count,
                player_position,
                n_samples,
                &mut mc_rng,
            );
            let gap = (expected_tricks(&dist) - dp_expected[i]).abs();
            if gap > max_expected_tricks_gap {
                max_expected_tricks_gap = gap;
            }
            black_box(dist);
        }
        let mc_elapsed = mc_start.elapsed();

        let mc_ms = mc_elapsed.as_secs_f64() * 1_000.0;
        let mc_us_per = mc_elapsed.as_secs_f64() * 1_000_000.0 / REPLICATIONS as f64;
        let ratio = if dp_ms > 0.0 {
            mc_ms / dp_ms
        } else {
            f64::INFINITY
        };
        println!(
            "  MC (n={n_samples:>7}):  total={mc_ms:.3} ms   avg={mc_us_per:.1} µs/hand   \
             ratio={ratio:.1}x   max|E_gap|={max_expected_tricks_gap:.4}"
        );
    }

    println!();
}

fn main() {
    println!("Benchmark: DP vs Monte Carlo trick-count distribution");
    println!("Replications per player count: {REPLICATIONS}");
    println!("MC sample counts: {MC_SAMPLE_COUNTS:?}");
    println!("Seed: {SEED}");
    println!();

    for &player_count in &[3usize, 4, 5, 6] {
        bench_player_count(player_count, SEED.wrapping_add(player_count as u64));
    }
}
