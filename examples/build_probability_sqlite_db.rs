use std::fs;
use std::path::PathBuf;

use mino_dice_prob_calc::dice::DieType;
use mino_dice_prob_calc::trick::win_probabilities_for_all_seats_dynamic;

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Connection, params};

const DIE_BAG_MAX: [u8; 7] = [1, 3, 2, 7, 7, 8, 8];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_arch = "wasm32")]
    {
        return Err("This example must run on a native target (not wasm32).".into());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let output_path = std::env::args()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data/win_probability_mapping.db"));

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if output_path.exists() {
            fs::remove_file(&output_path)?;
        }

        let mut conn = Connection::open(&output_path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE single_trick_probabilities (
                player_count INTEGER NOT NULL,
                seq_idx INTEGER NOT NULL,
                p0_count INTEGER NOT NULL,
                p1_count INTEGER NOT NULL,
                p2_count INTEGER NOT NULL,
                p3_count INTEGER NOT NULL,
                p4_count INTEGER NOT NULL,
                p5_count INTEGER NOT NULL,
                denominator INTEGER NOT NULL,
                PRIMARY KEY(player_count, seq_idx)
            );

            CREATE INDEX idx_probs_player_count
                ON single_trick_probabilities(player_count);
            ",
        )?;

        let tx = conn.transaction()?;

        tx.execute(
            "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
            params!["model", "single_trick_win_probabilities"],
        )?;
        tx.execute(
            "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
            params!["source", "generated_directly"],
        )?;
        tx.execute(
            "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
            params!["player_counts", "3,4,5,6"],
        )?;
        tx.execute(
            "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
            params!["key_format", "base7_seq_idx"],
        )?;
        tx.execute(
            "INSERT INTO metadata(key, value) VALUES (?1, ?2)",
            params!["value_format", "per_seat_win_counts_with_denominator"],
        )?;

        let mut inserted = 0usize;

        for player_count in 3..=6 {
            let denominator = 3_i64.pow(player_count as u32);
            let mut digits = vec![0usize; player_count];
            let mut done = false;

            while !done {
                let mut counts = [0u8; 7];
                for &d in &digits {
                    counts[d] += 1;
                }

                if counts_within_bag_limits(&counts) {
                    let dice: Vec<DieType> = digits.iter().map(|&d| DieType::ALL[d]).collect();
                    let probs = win_probabilities_for_all_seats_dynamic(&dice);
                    let mut win_counts: Vec<i64> = probs
                        .iter()
                        .map(|p| (p * denominator as f64).round() as i64)
                        .collect();

                    // Preserve exact row sum by adjusting the largest-probability seat.
                    let sum_counts: i64 = win_counts.iter().sum();
                    let diff = denominator - sum_counts;
                    if diff != 0 {
                        let mut max_idx = 0usize;
                        for i in 1..win_counts.len() {
                            if win_counts[i] > win_counts[max_idx] {
                                max_idx = i;
                            }
                        }
                        win_counts[max_idx] += diff;
                    }

                    let mut padded = [0i64; 6];
                    for (i, &count) in win_counts.iter().enumerate() {
                        padded[i] = count;
                    }

                    tx.execute(
                        "
                        INSERT INTO single_trick_probabilities(
                            player_count, seq_idx,
                            p0_count, p1_count, p2_count, p3_count, p4_count, p5_count,
                            denominator
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                        ",
                        params![
                            player_count as i64,
                            encode_base7_digits(&digits) as i64,
                            padded[0],
                            padded[1],
                            padded[2],
                            padded[3],
                            padded[4],
                            padded[5],
                            denominator,
                        ],
                    )?;
                    inserted += 1;
                }

                done = !increment_base7_digits(&mut digits);
            }
        }

        tx.commit()?;

        println!(
            "Built SQLite probability DB with {} rows at {}",
            inserted,
            output_path.display()
        );

        Ok(())
    }
}

fn encode_base7_digits(digits: &[usize]) -> usize {
    digits.iter().fold(0usize, |acc, &d| acc * 7 + d)
}

fn counts_within_bag_limits(counts: &[u8; 7]) -> bool {
    counts
        .iter()
        .zip(DIE_BAG_MAX.iter())
        .all(|(&count, &max_count)| count <= max_count)
}

fn increment_base7_digits(digits: &mut [usize]) -> bool {
    let mut pos = digits.len();
    while pos > 0 {
        pos -= 1;
        digits[pos] += 1;
        if digits[pos] < 7 {
            return true;
        }
        digits[pos] = 0;
    }
    false
}
