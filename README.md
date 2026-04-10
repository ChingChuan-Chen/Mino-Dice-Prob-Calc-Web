# Mino-Dice-Prob-Calc-Web

Mino Dice Probability Calculator is a frontend-only Rust + WASM tool for estimating trick outcomes, comparing bids, and inspecting likely opponent hands for the board game Mino Dice.

## What This App Does

- estimate the probability of winning 0 through *n* tricks for your current hand
- recommend the bid with the highest expected score
- show likely opponent-hand patterns from the remaining bag
- estimate special-capture bonus chances for Mermaid and Minotaur hands
- keep all calculations in the browser with no server round-trip

## How To Use

1. Select the number of players.
2. Set your play order for the next trick.
3. Build your hand with the die counters.
4. Choose the number of Monte Carlo replications.
5. Optionally set a seed if you want reproducible results.
6. Click **Calculate Distribution**.

## How To Read The Results

- **Optimal Bid**: the recommended bid based on expected score.
- **Probability chart**: estimated chance of winning exactly 0, 1, 2, ... tricks with the selected hand.
- **Expected score at bid**: the expected score if you deliberately choose that bid.
- **Special Capture Odds**: estimated chance to earn Mermaid/Minotaur bonus captures.
- **Top 3 Likely Opponent Hands**: the most likely opponent hand patterns from the remaining bag.

## Tech Stack

| Layer | Choice | Rationale |
|---|---|---|
| Language | Rust (stable) | Core probability engine; compiled to WASM |
| WASM toolchain | [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) + [`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/) | Standard Rust→WASM pipeline |
| Bundler | [Trunk](https://trunkrs.dev/) | Rust-native; no Node.js required; handles WASM loading and static asset copying |
| CSS | [Tailwind CSS](https://tailwindcss.com/) via CDN | Utility-first; no build step; sufficient for a small single-page tool |
| Charting | [`plotters`](https://github.com/plotters-rs/plotters) compiled to WASM/Canvas | All-Rust; no JS charting dependency; renders directly onto an HTML `<canvas>` element |

## Local Development

```bash
trunk serve
```

Useful checks:

```bash
cargo test
cargo clippy
cargo fmt --all -- --check
```

## Test Quality Review and Coverage

Current tests include both happy-path checks and edge-case checks:

- **Happy-path examples**: output shape checks for simulation APIs, normal score calculations, and standard winner resolution cases.
- **Edge-case examples**: invalid inputs (`target_idx`, invalid die names, invalid round bounds), all-flag trick tie behavior, special-cycle interactions, deterministic table-vs-enumeration parity, and limit clamping for opponent-pattern queries.

Coverage snapshot (from `cargo llvm-cov --lib --summary-only`, 2026-04-10):

| Scope | Region Coverage | Function Coverage | Line Coverage |
|---|---:|---:|---:|
| `src/dice.rs` | 94.15% | 90.48% | 94.56% |
| `src/trick.rs` | 95.33% | 94.34% | 97.40% |
| `src/round.rs` | 98.43% | 99.01% | 97.86% |
| `src/api.rs` | 78.57% | 75.51% | 83.93% |
| **Total (lib target)** | **64.28%** | **66.03%** | **68.25%** |

Notes:
- Total coverage is reduced by currently untested UI/chart entry points (`src/ui.rs`, `src/chart.rs`, and wasm startup in `src/lib.rs`).
- Core probability/round/trick logic is highly covered by unit tests.

## SQLite Probability Lookup (Native)

For native runs and tests, you can preload the single-trick probability table into SQLite
and have the trick engine read it directly instead of rebuilding the in-memory reference map.

Build the database directly from the probability engine:

```bash
cargo run --example build_probability_sqlite_db
```

By default, runtime lookup checks `data/win_probability_mapping.db` first. You can override
the path with:

```bash
MINO_DICE_PROB_DB=/path/to/win_probability_mapping.db
```

If the database is missing or incomplete, the code falls back to the existing in-memory table build.

The SQLite table stores a compact integer key (`seq_idx`, base-7 encoded ordered dice)
instead of comma-separated die-name strings.

## Documentation

- [Mino Dice Rules](mino_dice_rule.md)
- [Probability Model](probability_model.md)
- [Roadmap](roadmap.md)
