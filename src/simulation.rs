use rand::Rng;
use std::collections::HashMap;

/// The default bag of dice available in the game.
fn default_bag() -> HashMap<String, usize> {
    let mut bag = HashMap::new();
    bag.insert("Minotaur".into(), 1);
    bag.insert("Griffin".into(), 3);
    bag.insert("Mermaid".into(), 2);
    bag.insert("Red".into(), 7);
    bag.insert("Yellow".into(), 7);
    bag.insert("Purple".into(), 8);
    bag.insert("Gray".into(), 8);
    bag
}

/// Dice face values used for rolling.
/// Special creature dice use sentinel values:
///   Minotaur = 99, Griffin = 98, Mermaid = 97, Flag = 0
fn dice_sides(name: &str) -> [u8; 6] {
    match name {
        "Minotaur" => [99, 99, 99, 99, 0, 0],
        "Griffin" => [98, 98, 98, 98, 0, 0],
        "Mermaid" => [97, 97, 97, 97, 0, 0],
        "Red" => [7, 7, 6, 6, 5, 5],
        "Yellow" => [5, 5, 4, 4, 3, 3],
        "Purple" => [3, 3, 2, 2, 1, 1],
        _ => [7, 1, 1, 0, 0, 0], // Gray
    }
}

fn roll_dice(rng: &mut impl Rng, name: &str) -> u8 {
    let sides = dice_sides(name);
    sides[rng.random_range(0..6usize)]
}

/// Returns the remaining bag after removing the player's own dice.
fn initialize_bag(own_dices: &[String]) -> HashMap<String, usize> {
    let mut bag = default_bag();
    for dice in own_dices {
        if let Some(count) = bag.get_mut(dice.as_str())
            && *count > 0
        {
            *count -= 1;
        }
    }
    bag
}

/// Draw dice from the bag for each player.
/// The player at `order` gets `own_dices`; others draw randomly.
fn initialize_players(
    rng: &mut impl Rng,
    number_players: usize,
    round_number: usize,
    order: usize,
    own_dices: &[String],
    bag: &mut HashMap<String, usize>,
) -> Vec<Vec<String>> {
    let dice_names: Vec<String> = bag.keys().cloned().collect();
    let mut players: Vec<Vec<String>> = Vec::with_capacity(number_players);
    for i in 0..number_players {
        if i == order {
            players.push(own_dices.to_vec());
        } else {
            let mut hand = Vec::with_capacity(round_number);
            for _ in 0..round_number {
                loop {
                    let idx = rng.random_range(0..dice_names.len());
                    let name = &dice_names[idx];
                    if let Some(count) = bag.get_mut(name.as_str())
                        && *count > 0
                    {
                        *count -= 1;
                        hand.push(name.clone());
                        break;
                    }
                }
            }
            players.push(hand);
        }
    }
    players
}

/// Pick which die to play: follow suit if possible, otherwise pick random.
fn choose_dice(rng: &mut impl Rng, hand: &mut Vec<String>, leading: &str) -> String {
    let idx = hand.iter().position(|d| d == leading);
    let selected = match idx {
        Some(i) => i,
        None => rng.random_range(0..hand.len()),
    };
    hand.remove(selected)
}

/// Determine the winner of a single trick.
/// Returns the index into `round_dices` that corresponds to the winning player.
/// The original `first_roll_player` offset maps `round_dices[0]` to `first_roll_player`.
fn get_round_winner(rng: &mut impl Rng, round_dices: &[String], first_roll_player: usize) -> usize {
    let n = round_dices.len();
    let results: Vec<u8> = round_dices.iter().map(|d| roll_dice(rng, d)).collect();

    // Reorder so index 0 = first_roll_player's result
    // We work in the shifted order where index 0 is first_roll_player
    let shifted_results: Vec<u8> = (0..n).map(|i| results[(i + first_roll_player) % n]).collect();

    let mut contains98 = shifted_results[0] == 98;
    let mut contains99 = shifted_results[0] == 99;
    let mut cur_max = shifted_results[0];
    let mut winner_shifted = 0usize;

    for (idx, &val) in shifted_results.iter().enumerate().skip(1) {
        contains98 = contains98 || (val == 98);
        contains99 = contains99 || (val == 99);

        if val == 97 && cur_max == 99 {
            // Mermaid beats Minotaur
            cur_max = 97;
            winner_shifted = idx;
        } else if val == 98 && cur_max == 97 && !contains99 {
            // Griffin beats Mermaid (when no Minotaur)
            cur_max = 98;
            winner_shifted = idx;
        } else if val > cur_max {
            cur_max = val;
            winner_shifted = idx;
        }
    }

    (winner_shifted + first_roll_player) % n
}

/// Simulate one full round and record how many tricks the player at `order` wins.
fn play_round(
    rng: &mut impl Rng,
    number_players: usize,
    round_number: usize,
    order: usize,
    players: &mut [Vec<String>],
) -> usize {
    let mut win_count = 0usize;
    let mut first_player = 0usize;

    // First player picks a leading die
    let first_die_idx = rng.random_range(0..players[first_player].len());
    let mut leading_die = players[first_player].remove(first_die_idx);

    for _ in 0..round_number {
        let mut round_dices: Vec<String> = Vec::with_capacity(number_players);
        #[allow(clippy::needless_range_loop)]
        for i in 0..number_players {
            if i == first_player {
                round_dices.push(leading_die.clone());
            } else {
                let chosen = choose_dice(rng, &mut players[i], &leading_die);
                round_dices.push(chosen);
            }
        }

        first_player = get_round_winner(rng, &round_dices, first_player);
        if first_player == order {
            win_count += 1;
        }

        if players[first_player].is_empty() {
            break;
        }

        let next_die_idx = rng.random_range(0..players[first_player].len());
        leading_die = players[first_player].remove(next_die_idx);
    }

    win_count
}

pub struct SimulationParams {
    pub number_players: usize,
    pub round_number: usize,
    pub order: usize,
    pub own_dices: Vec<String>,
    pub number_experiments: usize,
}

/// Run Monte Carlo simulation and return the win-count distribution.
pub fn run_simulation(params: &SimulationParams) -> HashMap<usize, usize> {
    let mut rng = rand::rng();
    let mut win_count_dict: HashMap<usize, usize> = HashMap::new();
    for i in 0..=params.round_number {
        win_count_dict.insert(i, 0);
    }

    for _ in 0..params.number_experiments {
        let mut bag = initialize_bag(&params.own_dices);
        let mut players = initialize_players(
            &mut rng,
            params.number_players,
            params.round_number,
            params.order,
            &params.own_dices,
            &mut bag,
        );
        let wins = play_round(
            &mut rng,
            params.number_players,
            params.round_number,
            params.order,
            &mut players,
        );
        *win_count_dict.entry(wins).or_insert(0) += 1;
    }

    win_count_dict
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_bag_total() {
        let bag = default_bag();
        let total: usize = bag.values().sum();
        assert_eq!(total, 36, "Default bag should have 36 dice total");
    }

    #[test]
    fn test_dice_sides_length() {
        for name in &["Minotaur", "Griffin", "Mermaid", "Red", "Yellow", "Purple", "Gray"] {
            let sides = dice_sides(name);
            assert_eq!(sides.len(), 6);
        }
    }

    #[test]
    fn test_initialize_bag_removes_own_dice() {
        let own = vec!["Red".to_string(), "Red".to_string()];
        let bag = initialize_bag(&own);
        assert_eq!(bag["Red"], 5, "Should have 5 Red dice remaining after drawing 2");
    }

    #[test]
    fn test_simulation_output_keys() {
        let params = SimulationParams {
            number_players: 3,
            round_number: 2,
            order: 0,
            own_dices: vec!["Red".to_string(), "Yellow".to_string()],
            number_experiments: 1000,
        };
        let result = run_simulation(&params);
        for i in 0..=2 {
            assert!(result.contains_key(&i), "Result should contain key {}", i);
        }
        let total: usize = result.values().sum();
        assert_eq!(total, 1000);
    }

    #[test]
    fn test_simulation_probabilities_sum_to_one() {
        let params = SimulationParams {
            number_players: 4,
            round_number: 3,
            order: 1,
            own_dices: vec!["Minotaur".to_string(), "Griffin".to_string(), "Red".to_string()],
            number_experiments: 2000,
        };
        let result = run_simulation(&params);
        let total: usize = result.values().sum();
        assert_eq!(total, 2000);
    }
}
