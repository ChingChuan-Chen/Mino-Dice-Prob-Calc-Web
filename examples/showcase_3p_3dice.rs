use mino_dice_prob_calc::dice::DieType;
use mino_dice_prob_calc::round::{
    Xorshift64, expected_score_for_bid, expected_tricks, monte_carlo_trick_count_distribution,
    optimal_bid, top_opponent_hand_patterns,
};

const PLAYER_COUNT: usize = 3;
const PLAYER_POSITION: usize = 0;
const REPLICATIONS: usize = 100_000;
const SEED: u64 = 20_260_410;

fn main() {
    let hand = vec![DieType::Mermaid, DieType::Red, DieType::Gray];
    let mut rng = Xorshift64::new(SEED);
    let dist = monte_carlo_trick_count_distribution(
        &hand,
        PLAYER_COUNT,
        PLAYER_POSITION,
        REPLICATIONS,
        &mut rng,
    );
    let bid = optimal_bid(&dist);
    let exp_tricks = expected_tricks(&dist);
    let patterns = top_opponent_hand_patterns(&hand, hand.len(), 3);

    println!("Mino Dice showcase session");
    println!("Players: {PLAYER_COUNT}");
    println!("You act: {}", ordinal_label(PLAYER_POSITION));
    println!("Hand: {}", format_hand(&hand));
    println!("Replications: {REPLICATIONS}");
    println!("Seed: {SEED}");
    println!();

    println!("Chart distribution (P[tricks = k]):");
    for (k, prob) in dist.iter().enumerate() {
        println!("  {k}: {:>5.2}%", prob * 100.0);
    }
    println!();

    println!("Expected tricks: {exp_tricks:.4}");
    println!("Optimal bid: {bid}");
    println!("Expected scores by bid:");
    for bid_value in 0..=hand.len() {
        let score = expected_score_for_bid(bid_value, &dist, hand.len());
        println!("  bid {bid_value}: {score:+.4}");
    }
    println!();

    println!("Top 3 exact opponent-hand patterns from the remaining bag:");
    for (idx, pattern) in patterns.iter().enumerate() {
        println!(
            "  {}. {} ({:.4}%)",
            idx + 1,
            format_hand(&pattern.hand),
            pattern.probability * 100.0
        );
    }
}

fn format_hand(hand: &[DieType]) -> String {
    hand.iter()
        .map(|&die| format_die(die))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_die(die: DieType) -> &'static str {
    match die {
        DieType::Minotaur => "Minotaur",
        DieType::Griffin => "Griffin",
        DieType::Mermaid => "Mermaid",
        DieType::Red => "Red",
        DieType::Yellow => "Yellow",
        DieType::Purple => "Purple",
        DieType::Gray => "Gray",
    }
}

fn ordinal_label(position: usize) -> &'static str {
    match position {
        0 => "1st",
        1 => "2nd",
        2 => "3rd",
        3 => "4th",
        4 => "5th",
        _ => "6th",
    }
}
