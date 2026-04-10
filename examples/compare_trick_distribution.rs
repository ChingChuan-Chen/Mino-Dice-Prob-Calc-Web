/// Compare DP vs Hand-Aware DP vs Monte Carlo for a configurable scenario.
///
/// Usage examples:
///   cargo run --example compare_trick_distribution
///   cargo run --example compare_trick_distribution -- --players 6
///   cargo run --example compare_trick_distribution -- --players 5 --hand mermaid,red,gray --position 0 --samples 100000 --seed 42
use mino_dice_prob_calc::{
    dice::DieType,
    round::{
        analytical_trick_count_distribution, hand_aware_trick_count_distribution,
        monte_carlo_trick_count_distribution, Xorshift64,
    },
};
use std::env;

#[derive(Debug, Clone)]
struct CompareConfig {
    players: usize,
    hand: Vec<DieType>,
    position: usize,
    samples: usize,
    seed: u64,
}

fn parse_die_type(token: &str) -> Result<DieType, String> {
    match token.trim().to_ascii_lowercase().as_str() {
        "minotaur" => Ok(DieType::Minotaur),
        "griffin" => Ok(DieType::Griffin),
        "mermaid" => Ok(DieType::Mermaid),
        "red" => Ok(DieType::Red),
        "yellow" => Ok(DieType::Yellow),
        "purple" => Ok(DieType::Purple),
        "gray" => Ok(DieType::Gray),
        other => Err(format!("unknown die type: {other}")),
    }
}

fn parse_hand(csv: &str) -> Result<Vec<DieType>, String> {
    let dice: Result<Vec<DieType>, String> = csv.split(',').map(parse_die_type).collect();
    let dice = dice?;
    if dice.is_empty() {
        return Err("hand must not be empty".to_string());
    }
    Ok(dice)
}

fn parse_args() -> Result<CompareConfig, String> {
    let mut cfg = CompareConfig {
        players: 4,
        hand: vec![DieType::Mermaid, DieType::Red, DieType::Gray],
        position: 0,
        samples: 100_000,
        seed: 42,
    };

    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--players" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --players".to_string())?;
                cfg.players = value
                    .parse::<usize>()
                    .map_err(|_| "--players must be an integer".to_string())?;
            }
            "--hand" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --hand".to_string())?;
                cfg.hand = parse_hand(value)?;
            }
            "--position" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --position".to_string())?;
                cfg.position = value
                    .parse::<usize>()
                    .map_err(|_| "--position must be an integer".to_string())?;
            }
            "--samples" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --samples".to_string())?;
                cfg.samples = value
                    .parse::<usize>()
                    .map_err(|_| "--samples must be an integer".to_string())?;
            }
            "--seed" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --seed".to_string())?;
                cfg.seed = value
                    .parse::<u64>()
                    .map_err(|_| "--seed must be an integer".to_string())?;
            }
            "--help" | "-h" => {
                return Err(
                    "usage: cargo run --example compare_trick_distribution -- [--players N] [--hand csv] [--position N] [--samples N] [--seed N]"
                        .to_string(),
                );
            }
            other => {
                return Err(format!("unknown argument: {other}"));
            }
        }
        i += 1;
    }

    if !(3..=6).contains(&cfg.players) {
        return Err("--players must be in 3..=6".to_string());
    }
    if cfg.position >= cfg.players {
        return Err("--position must be in 0..players-1".to_string());
    }
    if cfg.samples == 0 {
        return Err("--samples must be > 0".to_string());
    }

    Ok(cfg)
}

fn format_hand(hand: &[DieType]) -> String {
    hand.iter()
        .map(|die| format!("{die:?}"))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn main() {
    let cfg = match parse_args() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };

    let player_count = cfg.players;
    let player_position = cfg.position;
    let hand = cfg.hand;
    let replications = cfg.samples;
    let seed = cfg.seed;

    println!(
        "=== {player_count}-player, hand: {} (position {}) ===\n",
        format_hand(&hand),
        player_position + 1
    );

    // Analytical (DP)
    let dp_dist = analytical_trick_count_distribution(&hand, player_count, player_position);
    println!("Analytical (DP):");
    let dp_expected: f64 = dp_dist
        .iter()
        .enumerate()
        .map(|(k, &p)| k as f64 * p)
        .sum();
    for (k, &p) in dp_dist.iter().enumerate() {
        println!("  P(tricks={k}) = {:.4}  ({:.2}%)", p, p * 100.0);
    }
    println!("  Expected tricks = {dp_expected:.4}\n");

    // Hand-Aware DP
    let ha_dist = hand_aware_trick_count_distribution(&hand, player_count, player_position);
    println!("Hand-Aware DP:");
    let ha_expected: f64 = ha_dist
        .iter()
        .enumerate()
        .map(|(k, &p)| k as f64 * p)
        .sum();
    for (k, &p) in ha_dist.iter().enumerate() {
        println!("  P(tricks={k}) = {:.4}  ({:.2}%)", p, p * 100.0);
    }
    println!("  Expected tricks = {ha_expected:.4}\n");

    // Monte Carlo
    let mut rng = Xorshift64::new(seed);
    let mc_dist = monte_carlo_trick_count_distribution(
        &hand, player_count, player_position, replications, &mut rng,
    );
    println!("Monte Carlo ({replications} replications, seed={seed}):");
    let mc_expected: f64 = mc_dist
        .iter()
        .enumerate()
        .map(|(k, &p)| k as f64 * p)
        .sum();
    for (k, &p) in mc_dist.iter().enumerate() {
        println!("  P(tricks={k}) = {:.4}  ({:.2}%)", p, p * 100.0);
    }
    println!("  Expected tricks = {mc_expected:.4}\n");

    println!("Gap (|DP expected - MC expected|)          = {:.4}", (dp_expected - mc_expected).abs());
    println!("Gap (|Hand-Aware DP expected - MC expected|) = {:.4}", (ha_expected - mc_expected).abs());
}
