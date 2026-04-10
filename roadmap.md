# Roadmap

## Development Plan

### Assets

Dice face images are stored in [`assets/dice/`](assets/dice/). File inventory:

| File | Represents |
|---|---|
| `minotaur_die.svg` | Minotaur face (used on Minotaur die ×4) |
| `griffin_die.svg` | Griffin face (used on Griffin die ×4) |
| `mermaid_die.svg` | Mermaid face (used on Mermaid die ×4) |
| `red_die.svg` | Red number die icon |
| `yellow_die.svg` | Yellow number die icon |
| `purple_die.svg` | Purple number die icon |
| `gray_die.svg` | Gray die/Flag face icon |

### Phase 1 — Project Setup ✅ *(completed 2026-04-09)*

- [x] Initialize a Rust + WebAssembly project with `wasm-pack` and `wasm-bindgen`.
- [x] Set up [Trunk](https://trunkrs.dev/) as the bundler; add `Trunk.toml` and `index.html`.
- [x] Add Tailwind CSS CDN link to `index.html`; add a `<canvas>` element to host `plotters` output.
- [x] Set up `cargo fmt`, `cargo clippy`, and a lint check in CI (`.github/workflows/ci.yml`).
- [x] Create a minimal "Hello World" page to verify WASM loads correctly in the browser.

**Artifacts:** `src/lib.rs`, `Cargo.toml`, `Trunk.toml`, `index.html`, `.github/workflows/ci.yml`
**Toolchain:** Rust 1.94.1 · wasm-pack 0.14.0 · Trunk 0.21.14 · wasm32-unknown-unknown target

**Run locally:** `trunk serve`

### Phase 2 — Core Probability Engine (Rust) ✅ *(completed 2026-04-09)*

- [x] Model the dice: define enums/structs for die types (Minotaur, Griffin, Mermaid, Number×4 colors), their faces, and bag counts (1 Minotaur, 3 Griffin, 2 Mermaid, 7 Red, 7 Yellow, 8 Purple, 8 Gray = 36 total).
- [x] Implement single-die probability distribution (face probabilities for each die type).
- [x] Implement trick-winning probability: given a set of dice in play, compute the probability that each die wins.
  - Handle special character vs. number die hierarchy.
  - Handle suit-following constraint (color matching).
  - Handle Flag (0) — loses to any non-Flag; if all rolled faces are Flags, the later roller wins.
  - Handle the circular Minotaur/Griffin/Mermaid cycle.
- [x] Implement round-level probability logic: exact single-trick probability and round-distribution estimation from a player's hand and player count.
- [x] Write unit tests for all probability calculations.
- [x] Benchmark hot paths to confirm real-time performance in WASM.

**Artifacts:** `src/dice.rs`, `src/trick.rs`, `src/round.rs`
**Test results:** 28 tests, 0 failed

### Phase 3 — WASM Bindings ✅ *(completed 2026-04-09)*

- [x] Expose probability engine functions to JavaScript via `wasm-bindgen`.
- [x] Define clean public API types (serializable input/output structs).
- [x] Add integration tests that call bindings end-to-end.

**Artifacts:** `src/api.rs`
**Exported functions:**

| Function | Input | Output |
|---|---|---|
| `get_all_dice_info()` | — | Array of die metadata (type, bag count, face probabilities) |
| `get_trick_distribution(input)` | `TrickDistInput` | `TrickDistOutput` (distribution, expected tricks, optimal bid, expected scores) |
| `get_win_probability(input)` | `WinProbInput` | `WinProbOutput` (win probability for one player) |
| `run_simulation(input)` | `SimInput` | `SimOutput` (per-player score arrays + means) |
| `get_round_count(player_count)` | `number` | `number` |

**Test results:** 40 tests, 0 failed

### Phase 4 — Frontend UI ✅ *(completed 2026-04-09)*

- [x] Design the input interface: select dice in hand (type, count), number of players, current hand number.
- [x] Display output: probability table and bar chart (`plotters` on `<canvas>`) showing expected trick distribution.
- [x] Show bid recommendation (the bid value with highest expected score).
- [x] Mobile-friendly design: all controls and chart usable on small screens (responsive Tailwind grid, no horizontal scroll).

**Artifacts:** `src/chart.rs`, `src/ui.rs`, rebuilt `index.html`
**Test results:** 40 tests, 0 failed

### Phase 4-1 — Responsive Chart & Opponent Pattern Analysis ✅ *(completed 2026-04-10)*

- [x] Make the `plotters` bar chart responsive and interactive: read the canvas's `clientWidth` before each render, resize the canvas pixel dimensions accordingly, and re-render on `window.resize`.
- [x] Make the ticks on the probability histogram be at the center of bars.
- [x] For Monte Carlo game simulation, list the top 3 most likely opponent-hand combinations for the current setup so users can inspect the strongest common opponent patterns behind the aggregate result.
- [x] Verify the bid-0 success scoring formula against the current rules section; current implementation remains `+10 × hand_size`.

**Artifacts:** `index.html`, responsive `src/chart.rs`, rewritten `src/ui.rs`, extended `src/round.rs`, extended `src/api.rs`
**Test results:** 43 tests, 0 failed

### Phase 5 — Polish & Deployment

- [x] Optimize WASM binary size (`wasm-opt`, `lto`, `opt-level = "z"`).
  - Baseline (before optimization tuning): `trunk build --release` produced `dist/*_bg.wasm` = **256,095 bytes** (gzip **117,950 bytes**) and JS glue = **33,485 bytes** (gzip **7,080 bytes**).
  - Current (after Phase 5 tuning): `dist/*_bg.wasm` = **250,979 bytes** (gzip **116,101 bytes**) and JS glue = **32,492 bytes** (gzip **6,951 bytes**).
  - CI budget gate added: wasm raw ≤ **308,000 bytes**, wasm gzip ≤ **143,000 bytes**.
- [ ] Deploy as a static site (GitHub Pages or Cloudflare Pages) — no server required.
- [x] Write a user-facing README / how-to-use guide.

### Phase 6 — In-Browser Game Simulation

- [ ] Create a new tab to have a full game simulation.
- [ ] Implement a full game simulation engine in Rust: set the number of players, draw dice from bag, play rounds, apply scoring rules.
- [ ] Expose simulation API via WASM: run N simulated games given a hand, return aggregated statistics.
- [ ] Build a simulation UI page: let users set up a game state (hand, player count, round), run simulation, view outcome distribution.
- [ ] Visualize simulated score distributions per player using `plotters`.
- [ ] Allow step-through replay of a single simulated game (trick by trick).
