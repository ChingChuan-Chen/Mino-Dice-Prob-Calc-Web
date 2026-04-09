use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::dice::{face_distribution, DieType};
use crate::round::{
    expected_score_for_bid, expected_tricks, optimal_bid, round_count, simulate_games,
    trick_count_distribution, Xorshift64,
};

// ── Input / output types ──────────────────────────────────────────────────────
// All types derive Serialize + Deserialize so they can cross the WASM boundary
// with serde-wasm-bindgen (JsValue → Rust and back).

/// A die type string understood by the API.
/// Accepted values: "minotaur", "griffin", "mermaid", "red", "yellow", "purple", "gray"
fn parse_die_type(s: &str) -> Result<DieType, JsValue> {
    match s.to_ascii_lowercase().as_str() {
        "minotaur" => Ok(DieType::Minotaur),
        "griffin" => Ok(DieType::Griffin),
        "mermaid" => Ok(DieType::Mermaid),
        "red" => Ok(DieType::Red),
        "yellow" => Ok(DieType::Yellow),
        "purple" => Ok(DieType::Purple),
        "gray" => Ok(DieType::Gray),
        other => Err(JsValue::from_str(&format!("Unknown die type: {other}"))),
    }
}

fn die_type_to_str(dt: DieType) -> &'static str {
    match dt {
        DieType::Minotaur => "minotaur",
        DieType::Griffin => "griffin",
        DieType::Mermaid => "mermaid",
        DieType::Red => "red",
        DieType::Yellow => "yellow",
        DieType::Purple => "purple",
        DieType::Gray => "gray",
    }
}

// ── Serialisable API structs ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct FaceProbDto {
    pub face: String,
    pub prob: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DieInfoDto {
    pub die_type: String,
    pub bag_count: u8,
    pub faces: Vec<FaceProbDto>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickDistInput {
    /// Die types in the player's hand, e.g. ["red", "yellow", "minotaur"]
    pub hand: Vec<String>,
    /// Each opponent's hand (same length as `hand`).
    pub opponent_hands: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickDistOutput {
    /// P(tricks = k) for k = 0..=hand_size
    pub distribution: Vec<f64>,
    pub expected_tricks: f64,
    pub optimal_bid: usize,
    /// Expected score for each possible bid (index = bid value)
    pub expected_scores: Vec<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WinProbInput {
    /// Die types for each player in roll order (index 0 = leader).
    pub player_dice: Vec<String>,
    /// Optional led color (the color of the number die the leader rolled).
    pub led_color: Option<String>,
    /// Index of the player whose win probability to compute.
    pub target_idx: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WinProbOutput {
    pub win_probability: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimInput {
    pub player_count: usize,
    pub n_games: usize,
    pub seed: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimOutput {
    /// Per-player score arrays, length = n_games each.
    pub scores: Vec<Vec<i64>>,
    /// Per-player mean score across all simulated games.
    pub mean_scores: Vec<f64>,
}

// ── WASM-exported functions ───────────────────────────────────────────────────

/// Returns metadata about all 7 die types: bag count and face probability distribution.
///
/// Returns a JS array of `DieInfoDto`.
#[wasm_bindgen]
pub fn get_all_dice_info() -> Result<JsValue, JsValue> {
    let info: Vec<DieInfoDto> = DieType::ALL
        .iter()
        .map(|&dt| {
            let faces = face_distribution(dt)
                .into_iter()
                .map(|fp| FaceProbDto {
                    face: format!("{:?}", fp.face),
                    prob: fp.prob,
                })
                .collect();
            DieInfoDto {
                die_type: die_type_to_str(dt).to_string(),
                bag_count: dt.bag_count(),
                faces,
            }
        })
        .collect();
    Ok(serde_wasm_bindgen::to_value(&info)?)
}

/// Computes the trick-count probability distribution for a player's hand.
///
/// `input` — a `TrickDistInput` JS object.
/// Returns a `TrickDistOutput` JS object.
#[wasm_bindgen]
pub fn get_trick_distribution(input: JsValue) -> Result<JsValue, JsValue> {
    let input: TrickDistInput = serde_wasm_bindgen::from_value(input)?;
    let output = inner_trick_distribution(input).map_err(|e| JsValue::from_str(&e))?;
    Ok(serde_wasm_bindgen::to_value(&output)?)
}

/// Computes the probability that `target_idx` wins a single trick.
///
/// `input` — a `WinProbInput` JS object.
/// Returns a `WinProbOutput` JS object.
#[wasm_bindgen]
pub fn get_win_probability(input: JsValue) -> Result<JsValue, JsValue> {
    let input: WinProbInput = serde_wasm_bindgen::from_value(input)?;
    let output = inner_win_probability(input).map_err(|e| JsValue::from_str(&e))?;
    Ok(serde_wasm_bindgen::to_value(&output)?)
}

/// Runs `n_games` full game simulations and returns per-player score distributions.
///
/// `input` — a `SimInput` JS object.
/// Returns a `SimOutput` JS object.
#[wasm_bindgen]
pub fn run_simulation(input: JsValue) -> Result<JsValue, JsValue> {
    let input: SimInput = serde_wasm_bindgen::from_value(input)?;
    let output = inner_simulation(input).map_err(|e| JsValue::from_str(&e))?;
    Ok(serde_wasm_bindgen::to_value(&output)?)
}

/// Returns the number of rounds played for a given player count (3–6).
#[wasm_bindgen]
pub fn get_round_count(player_count: usize) -> Result<usize, JsValue> {
    if !(3..=6).contains(&player_count) {
        return Err(JsValue::from_str("player_count must be 3–6"));
    }
    Ok(round_count(player_count))
}

// ── Pure (non-wasm) inner helpers used by tests ───────────────────────────────

fn parse_die_type_str(s: &str) -> Result<DieType, String> {
    match s.to_ascii_lowercase().as_str() {
        "minotaur" => Ok(DieType::Minotaur),
        "griffin" => Ok(DieType::Griffin),
        "mermaid" => Ok(DieType::Mermaid),
        "red" => Ok(DieType::Red),
        "yellow" => Ok(DieType::Yellow),
        "purple" => Ok(DieType::Purple),
        "gray" => Ok(DieType::Gray),
        other => Err(format!("Unknown die type: {other}")),
    }
}

// ── Pure (non-wasm) inner helpers used by tests ───────────────────────────────

fn inner_trick_distribution(input: TrickDistInput) -> Result<TrickDistOutput, String> {
    let hand: Result<Vec<DieType>, String> = input
        .hand
        .iter()
        .map(|s| parse_die_type_str(s))
        .collect();
    let hand = hand?;

    let opp_hands: Result<Vec<Vec<DieType>>, String> = input
        .opponent_hands
        .iter()
        .map(|opp| opp.iter().map(|s| parse_die_type_str(s)).collect())
        .collect();
    let opp_hands = opp_hands?;

    let hand_size = hand.len();
    let dist = trick_count_distribution(&hand, &opp_hands);
    let exp = expected_tricks(&dist);
    let bid = optimal_bid(&dist);
    let scores: Vec<f64> = (0..=hand_size)
        .map(|b| expected_score_for_bid(b, &dist, hand_size))
        .collect();

    Ok(TrickDistOutput {
        distribution: dist,
        expected_tricks: exp,
        optimal_bid: bid,
        expected_scores: scores,
    })
}

fn inner_win_probability(input: WinProbInput) -> Result<WinProbOutput, String> {
    let dice: Result<Vec<DieType>, String> = input
        .player_dice
        .iter()
        .map(|s| parse_die_type_str(s))
        .collect();
    let dice = dice?;

    if input.target_idx >= dice.len() {
        return Err("target_idx out of range".to_string());
    }

    let led_color = input
        .led_color
        .as_deref()
        .map(parse_die_type_str)
        .transpose()?;

    let prob = crate::trick::win_probability(&dice, led_color, input.target_idx);
    Ok(WinProbOutput {
        win_probability: prob,
    })
}

fn inner_simulation(input: SimInput) -> Result<SimOutput, String> {
    if !(3..=6).contains(&input.player_count) {
        return Err("player_count must be 3–6".to_string());
    }
    if input.n_games == 0 {
        return Err("n_games must be > 0".to_string());
    }
    let mut rng = Xorshift64::new(input.seed);
    let scores = simulate_games(input.player_count, input.n_games, &mut rng);
    let mean_scores: Vec<f64> = scores
        .iter()
        .map(|s| s.iter().sum::<i64>() as f64 / s.len() as f64)
        .collect();
    Ok(SimOutput {
        scores,
        mean_scores,
    })
}

// ── Integration tests ─────────────────────────────────────────────────────────
// Use the pure inner helpers so tests run on the host target without hitting
// JsValue internals that are WASM-only.

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trick_input(hand: &[&str], opp_hand: &[&str]) -> TrickDistInput {
        TrickDistInput {
            hand: hand.iter().map(|s| s.to_string()).collect(),
            opponent_hands: vec![opp_hand.iter().map(|s| s.to_string()).collect()],
        }
    }

    // --- dice info (pure Rust, no JsValue) ---

    #[test]
    fn all_dice_info_has_seven_entries() {
        for &dt in &DieType::ALL {
            let dist = face_distribution(dt);
            assert!(!dist.is_empty());
        }
    }

    #[test]
    fn bag_counts_sum_to_36() {
        let total: u8 = DieType::ALL.iter().map(|d| d.bag_count()).sum();
        assert_eq!(total, 36);
    }

    // --- trick distribution ---

    #[test]
    fn trick_distribution_sums_to_one() {
        let input = make_trick_input(&["red", "yellow"], &["purple", "gray"]);
        let output = inner_trick_distribution(input).unwrap();
        let sum: f64 = output.distribution.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "sum={sum}");
    }

    #[test]
    fn trick_distribution_length_correct() {
        let input = make_trick_input(&["red", "yellow"], &["purple", "gray"]);
        let output = inner_trick_distribution(input).unwrap();
        assert_eq!(output.distribution.len(), 3); // hand size 2 → 3 outcomes
        assert!(output.optimal_bid <= 2);
        assert_eq!(output.expected_scores.len(), 3);
    }

    #[test]
    fn unknown_die_type_errors() {
        let input = make_trick_input(&["banana"], &["red"]);
        assert!(inner_trick_distribution(input).is_err());
    }

    // --- win probability ---

    #[test]
    fn win_probability_in_range() {
        let input = WinProbInput {
            player_dice: vec!["minotaur".to_string(), "red".to_string()],
            led_color: None,
            target_idx: 0,
        };
        let output = inner_win_probability(input).unwrap();
        assert!(output.win_probability >= 0.0 && output.win_probability <= 1.0);
    }

    #[test]
    fn win_probability_complements_to_one() {
        let make = |target: usize| {
            inner_win_probability(WinProbInput {
                player_dice: vec!["red".to_string(), "yellow".to_string()],
                led_color: None,
                target_idx: target,
            })
            .unwrap()
            .win_probability
        };
        let p0 = make(0);
        let p1 = make(1);
        assert!((p0 + p1 - 1.0).abs() < 1e-9, "p0={p0}, p1={p1}");
    }

    #[test]
    fn target_idx_out_of_range_errors() {
        let input = WinProbInput {
            player_dice: vec!["red".to_string()],
            led_color: None,
            target_idx: 5,
        };
        assert!(inner_win_probability(input).is_err());
    }

    // --- simulation ---

    #[test]
    fn simulation_correct_shape() {
        let output = inner_simulation(SimInput {
            player_count: 4,
            n_games: 20,
            seed: 42,
        })
        .unwrap();
        assert_eq!(output.scores.len(), 4);
        for p in &output.scores {
            assert_eq!(p.len(), 20);
        }
        assert_eq!(output.mean_scores.len(), 4);
    }

    #[test]
    fn simulation_invalid_player_count_errors() {
        assert!(inner_simulation(SimInput { player_count: 2, n_games: 1, seed: 0 }).is_err());
        assert!(inner_simulation(SimInput { player_count: 7, n_games: 1, seed: 0 }).is_err());
    }

    #[test]
    fn simulation_zero_games_errors() {
        assert!(inner_simulation(SimInput { player_count: 4, n_games: 0, seed: 0 }).is_err());
    }

    // --- round count ---

    #[test]
    fn round_count_values_correct() {
        assert_eq!(round_count(3), 8);
        assert_eq!(round_count(4), 8);
        assert_eq!(round_count(5), 7);
        assert_eq!(round_count(6), 6);
    }
}
