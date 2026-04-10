use std::hint::black_box;
use std::time::Instant;

use mino_dice_prob_calc::dice::DieType;
use mino_dice_prob_calc::round::{Xorshift64, sample_hand};
use mino_dice_prob_calc::trick::{
    win_probabilities_for_all_seats, win_probabilities_for_all_seats_dynamic,
};

const PLAYER_COUNT: usize = 6;
const REPLICATIONS: usize = 10_000;
const SEED: u64 = 20_260_410;

fn main() {
    let mut rng = Xorshift64::new(SEED);

    let mut hands: Vec<Vec<DieType>> = Vec::with_capacity(REPLICATIONS);
    for _ in 0..REPLICATIONS {
        hands.push(sample_hand(PLAYER_COUNT, &mut rng));
    }

    // Warm up both paths once to avoid cold-start effects in steady-state timing.
    let warm_hand = &hands[0];
    let _ = black_box(win_probabilities_for_all_seats(warm_hand));
    let _ = black_box(win_probabilities_for_all_seats_dynamic(warm_hand));

    let static_start = Instant::now();
    let mut static_checksum = 0.0f64;
    for hand in &hands {
        let probs = win_probabilities_for_all_seats(hand);
        static_checksum += probs.iter().sum::<f64>();
        black_box(&probs);
    }
    let static_elapsed = static_start.elapsed();

    let dynamic_start = Instant::now();
    let mut dynamic_checksum = 0.0f64;
    for hand in &hands {
        let probs = win_probabilities_for_all_seats_dynamic(hand);
        dynamic_checksum += probs.iter().sum::<f64>();
        black_box(&probs);
    }
    let dynamic_elapsed = dynamic_start.elapsed();

    let mut max_abs_diff = 0.0f64;
    for hand in &hands {
        let static_probs = win_probabilities_for_all_seats(hand);
        let dynamic_probs = win_probabilities_for_all_seats_dynamic(hand);
        for (a, b) in static_probs.iter().zip(dynamic_probs.iter()) {
            let d = (a - b).abs();
            if d > max_abs_diff {
                max_abs_diff = d;
            }
        }
    }

    let static_ms = static_elapsed.as_secs_f64() * 1_000.0;
    let dynamic_ms = dynamic_elapsed.as_secs_f64() * 1_000.0;
    let static_us_per_hand = static_elapsed.as_secs_f64() * 1_000_000.0 / REPLICATIONS as f64;
    let dynamic_us_per_hand = dynamic_elapsed.as_secs_f64() * 1_000_000.0 / REPLICATIONS as f64;
    let speedup = if static_ms > 0.0 {
        dynamic_ms / static_ms
    } else {
        f64::INFINITY
    };

    println!("Benchmark: static table vs dynamic enumeration");
    println!("Players: {PLAYER_COUNT}");
    println!("Replications: {REPLICATIONS}");
    println!("Seed: {SEED}");
    println!();

    println!("Static table path:");
    println!("  total time: {:.3} ms", static_ms);
    println!("  avg time:   {:.3} us/hand", static_us_per_hand);
    println!("  checksum:   {:.6}", static_checksum);

    println!("Dynamic path:");
    println!("  total time: {:.3} ms", dynamic_ms);
    println!("  avg time:   {:.3} us/hand", dynamic_us_per_hand);
    println!("  checksum:   {:.6}", dynamic_checksum);

    println!();
    println!("Relative ratio (dynamic/static): {:.2}x", speedup);
    println!("Max abs diff between paths: {:.3e}", max_abs_diff);
}
