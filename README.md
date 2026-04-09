# Mino-Dice-Prob-Calc-Web

A web-based Monte Carlo probability calculator for the **Mino Dice** board game.
Configure the number of players, the round, your turn order, and which dice you hold — then run a simulation to see the probability distribution for the number of tricks you can win.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024, i.e. **Rust 1.85 or later**)

Install Rust via `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Running the project

1. **Clone the repository**

   ```bash
   git clone https://github.com/ChingChuan-Chen/Mino-Dice-Prob-Calc-Web.git
   cd Mino-Dice-Prob-Calc-Web
   ```

2. **Build and start the server**

   ```bash
   cargo run --release
   ```

   The server compiles the project and then starts listening on port **3000**.
   You will see:

   ```
   Server running at http://localhost:3000
   ```

3. **Open the application**

   Navigate to <http://localhost:3000> in your browser.

## Usage

1. Select the **number of players** (3 – 6).
2. Choose the **round** (1st – 8th; some rounds are disabled for larger player counts because there are not enough dice in the bag).
3. Pick **your turn order** (1st, 2nd, … relative to the starting player).
4. Set the **number of experiments** for the Monte Carlo simulation (default: 50 000).
5. Select **your dice** — one drop-down per trick in the chosen round.
6. Click **Run Simulation**.

The bar chart and table below the form show the probability of winning 0, 1, 2, … tricks.

## Running the tests

```bash
cargo test
```

## Project structure

```
.
├── Cargo.toml          # Rust package manifest and dependencies
├── src/
│   ├── main.rs         # Axum web server and API handler
│   └── simulation.rs   # Monte Carlo simulation logic
└── static/
    └── index.html      # Single-page frontend (Chart.js)
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| [axum](https://crates.io/crates/axum) | HTTP web framework |
| [tokio](https://crates.io/crates/tokio) | Async runtime |
| [serde / serde_json](https://crates.io/crates/serde) | JSON serialisation |
| [rand](https://crates.io/crates/rand) | Random number generation |
| [tower-http](https://crates.io/crates/tower-http) | Static file serving |