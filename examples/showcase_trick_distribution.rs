use mino_dice_prob_calc::dice::DieType;
use mino_dice_prob_calc::round::{
    analytical_trick_count_distribution, expected_score_for_bid, expected_tricks, optimal_bid,
    round_count, top_opponent_hand_patterns,
};
use std::env;

#[derive(Debug, Clone)]
struct ShowcaseConfig {
    players: usize,
    dice: usize,
    position: usize,
    hand: Vec<DieType>,
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
    let hand: Result<Vec<DieType>, String> = csv.split(',').map(parse_die_type).collect();
    let hand = hand?;
    if hand.is_empty() {
        return Err("hand must not be empty".to_string());
    }
    Ok(hand)
}

fn default_hand(dice_count: usize) -> Vec<DieType> {
    let pattern = [
        DieType::Mermaid,
        DieType::Red,
        DieType::Gray,
        DieType::Yellow,
        DieType::Purple,
        DieType::Griffin,
        DieType::Minotaur,
        DieType::Gray,
    ];
    pattern.iter().copied().take(dice_count).collect()
}

fn parse_args() -> Result<ShowcaseConfig, String> {
    let mut players = 3usize;
    let mut dice = 3usize;
    let mut position = 0usize;
    let mut hand_override: Option<Vec<DieType>> = None;

    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--players" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --players".to_string())?;
                players = value
                    .parse::<usize>()
                    .map_err(|_| "--players must be an integer".to_string())?;
            }
            "--dice" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --dice".to_string())?;
                dice = value
                    .parse::<usize>()
                    .map_err(|_| "--dice must be an integer".to_string())?;
            }
            "--position" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --position".to_string())?;
                position = value
                    .parse::<usize>()
                    .map_err(|_| "--position must be an integer".to_string())?;
            }
            "--hand" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "missing value for --hand".to_string())?;
                hand_override = Some(parse_hand(value)?);
            }
            "--help" | "-h" => {
                return Err(
                    "usage: cargo run --example showcase_trick_distribution -- [--players N] [--dice N] [--position N] [--hand csv]"
                        .to_string(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    if !(3..=6).contains(&players) {
        return Err("--players must be in 3..=6".to_string());
    }
    if dice == 0 {
        return Err("--dice must be > 0".to_string());
    }
    let max_dice = round_count(players);
    if dice > max_dice {
        return Err(format!(
            "--dice must be <= {max_dice} for {players} players"
        ));
    }
    if position >= players {
        return Err("--position must be in 0..players-1".to_string());
    }

    let hand = hand_override.unwrap_or_else(|| default_hand(dice));
    if hand.len() != dice {
        return Err("--hand length must equal --dice".to_string());
    }

    Ok(ShowcaseConfig {
        players,
        dice,
        position,
        hand,
    })
}

fn main() {
    let cfg = match parse_args() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };

    let dist = analytical_trick_count_distribution(&cfg.hand, cfg.players, cfg.position);
    let bid = optimal_bid(&dist);
    let exp_tricks = expected_tricks(&dist);
    let patterns = top_opponent_hand_patterns(&cfg.hand, cfg.hand.len(), 3);

    println!("Mino Dice showcase session");
    println!("Players: {}", cfg.players);
    println!("Dice in hand: {}", cfg.dice);
    println!("You act: {}", ordinal_label(cfg.position));
    println!("Hand: {}", format_hand(&cfg.hand));
    println!("Model: Analytical single-trick probabilities + DP");
    println!();

    println!("Chart distribution (P[tricks = k]):");
    for (k, prob) in dist.iter().enumerate() {
        println!("  {k}: {:>5.2}%", prob * 100.0);
    }
    println!();

    println!("Expected tricks: {exp_tricks:.4}");
    println!("Optimal bid: {bid}");
    println!("Expected scores by bid:");
    for bid_value in 0..=cfg.hand.len() {
        let score = expected_score_for_bid(bid_value, &dist, cfg.hand.len());
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
