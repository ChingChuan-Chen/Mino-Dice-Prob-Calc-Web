# Mino-Dice-Prob-Calc-Web

This is a web application based on Rust to make a frontend-only probability calculator for the board game, Mino Dice.

---

## Game Rules: Mino Dice (a.k.a. Mythical Dice)

**BGG:** [Mythical Dice (2016)](https://boardgamegeek.com/boardgame/191071/mythical-dice) — reimplements [Skull King](https://boardgamegeek.com/boardgame/150145/skull-king)
**Players:** 3–6 | **Play Time:** ~30 min | **Age:** 8+

Mino Dice is a trick-taking game with simultaneous bidding. Players draw dice secretly from a bag, bid how many tricks they will win, then compete round by round. The player who predicts their performance most accurately earns the most points.

### Components

| Die Type | Icon | Faces | Count in bag | Notes |
|---|---|---|---|---|
| Minotaur die | ![minotaur](assets/dice/minotaur.png) | Minotaur ×4, Flag ×2 | 1 | Special character die (dark red) |
| Griffin die | ![griffin](assets/dice/griffin.png) | Griffin ×4, Flag ×2 | 3 | Special character die (green) |
| Mermaid die | ![mermaid](assets/dice/mermaid.png) | Mermaid ×4, Flag ×2 | 2 | Special character die (blue) |
| Red die | ![red](assets/dice/red_dice.png) | 7 ×2, 6 ×2, 5 ×2 | 7 | High-value number die |
| Yellow die | ![yellow](assets/dice/yellow_dice.png) | 5 ×2, 4 ×2, 3 ×2 | 7 | Mid-value number die |
| Purple die | ![purple](assets/dice/purple_dice.png) | 3 ×2, 2 ×2, 1 ×2 | 8 | Low-value number die |
| Gray die | ![gray](assets/dice/gray_dice.png) | Flag ×3, 1 ×2, 7 ×1 | 8 | Mostly flags; risky suit die |

**Total: 36 dice in bag** (1 + 3 + 2 + 7 + 7 + 8 + 8)

### Round Structure

The game plays **6–8 hands** depending on player count. In 3p-4p game, there are 8 rounds. In 5p game, there are 7 rounds. In 6p game, there are 6 roungs.
In hand *n*, each player draws **n dice** from the bag and hides them behind their screen.

**Step 1 — Bid:** All players simultaneously reveal their bid (number of tricks they expect to win) by holding up fingers. Bids are recorded on the scoresheet.

**Step 2 — Play tricks:** The leading player picks one of their hidden dice, rolls it publicly, then:
- If a **number die** is rolled, every other player must follow suit by rolling a number die of the **same color** if they have one; otherwise they may roll any die.
- A player may **always** choose to roll a **special character die** (Minotaur, Griffin, or Mermaid) instead of following suit.

**Step 3 — Determine trick winner:**
- Special character dice beat all number dice.
- Among special characters: **Minotaur > Griffin > Mermaid > Minotaur** (rock-paper-scissors cycle).
- Among number dice only: the **highest number** wins; ties go to the **later roller**.
- A rolled **Flag face** counts as 0 and can never win a trick.
- The trick winner collects the rolled dice and leads the next trick.

Repeat until all dice in hand are used.

### Scoring (per hand)

| Outcome | Points |
|---|---|
| Made bid exactly (bid > 0) | +20 × bid |
| Missed bid (bid > 0) | −10 × \|bid − tricks taken\| |
| Bid 0 and succeeded | +10 × tricks in the hand |
| Bid 0 and failed | −10 × tricks in the hand |
| **Bonus:** Captured a Minotaur with a Mermaid (no Flag captured) | +50 |
| **Bonus:** Captured a Griffin with a Minotaur (no Flag captured) | +30 |

The player with the **highest total score** after the final hand wins.

---

## Tech Stack

| Layer | Choice | Rationale |
|---|---|---|
| Language | Rust (stable) | Core probability engine; compiled to WASM |
| WASM toolchain | [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) + [`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/) | Standard Rust→WASM pipeline |
| Bundler | [Trunk](https://trunkrs.dev/) | Rust-native; no Node.js required; handles WASM loading and static asset copying |
| CSS | [Tailwind CSS](https://tailwindcss.com/) via CDN | Utility-first; no build step; sufficient for a small single-page tool |
| Charting | [`plotters`](https://github.com/plotters-rs/plotters) compiled to WASM/Canvas | All-Rust; no JS charting dependency; renders directly onto an HTML `<canvas>` element |

---

## Development Plan

### Assets

Dice face images are stored in [`assets/dice/`](assets/dice/). File inventory:

| File | Represents |
|---|---|
| `minotaur.png` | Minotaur face (used on Minotaur die ×4) |
| `griffin.png` | Griffin face (used on Griffin die ×4) |
| `mermaid.png` | Mermaid face (used on Mermaid die ×4) |
| `red_dice.png` | Red number die icon |
| `yellow_dice.png` | Yellow number die icon |
| `purple_dice.png` | Purple number die icon |
| `gray_dice.png` | Gray die/Flag face icon |

> Note: a Flag face image (shared across all dice) is not yet present — consider adding `flag.png` to this folder.

### Phase 1 — Project Setup

- [x] Initialize a Rust + WebAssembly project with `wasm-pack` and `wasm-bindgen`.
- [x] Set up [Trunk](https://trunkrs.dev/) as the bundler; add `Trunk.toml` and `index.html`.
- [x] Add Tailwind CSS CDN link to `index.html`; add a `<canvas>` element to host `plotters` output.
- [x] Set up `cargo fmt`, `cargo clippy`, and a lint check in CI (`.github/workflows/ci.yml`).
- [x] Create a minimal "Hello World" page to verify WASM loads correctly in the browser.

### Phase 2 — Core Probability Engine (Rust)

- [ ] Model the dice: define enums/structs for die types (Minotaur, Griffin, Mermaid, Number×4 colors), their faces, and bag counts (1 Minotaur, 3 Griffin, 2 Mermaid, 7 Red, 7 Yellow, 8 Purple, 8 Gray = 36 total).
- [ ] Implement single-die probability distribution (face probabilities for each die type).
- [ ] Implement trick-winning probability: given a set of dice in play, compute the probability that each die wins.
  - Handle special character vs. number die hierarchy.
  - Handle suit-following constraint (color matching).
  - Handle Flag (0) — always loses.
  - Handle the circular Minotaur/Griffin/Mermaid cycle.
- [ ] Implement round-level simulation: given a player's hand (set of dice) and player count, estimate the probability distribution over number of tricks won.
- [ ] Write unit tests for all probability calculations.
- [ ] Benchmark hot paths to confirm real-time performance in WASM.

### Phase 3 — WASM Bindings

- [ ] Expose probability engine functions to JavaScript via `wasm-bindgen`.
- [ ] Define clean public API types (serializable input/output structs).
- [ ] Add integration tests that call bindings end-to-end.

### Phase 4 — Frontend UI

- [ ] Design the input interface: select dice in hand (type, count), number of players, current hand number.
- [ ] Display output: probability table and bar chart (`plotters` on `<canvas>`) showing expected trick distribution.
- [ ] Show bid recommendation (the bid value with highest expected score).
- [ ] Add scoring helper: input bids and results per hand, show running totals.
- [ ] Mobile-friendly design: ensure all controls and charts are fully usable on small screens (touch targets, responsive layout, no horizontal scroll).

### Phase 5 — Polish & Deployment

- [ ] Add a game rules summary / quick-reference card in the UI.
- [ ] Optimize WASM binary size (`wasm-opt`, `lto`, `opt-level = "z"`).
- [ ] Deploy as a static site (GitHub Pages or Cloudflare Pages) — no server required.
- [ ] Write a user-facing README / how-to-use guide.

### Phase 6 — In-Browser Game Simulation

- [ ] Implement a full game simulation engine in Rust: set the number of players and the number of rounds, draw dice from bag, play rounds, apply scoring rules.
- [ ] Expose simulation API via WASM: run N simulated games given a hand, return aggregated statistics.
- [ ] Build a simulation UI page: let users set up a game state (hand, player count, round), run simulation, view outcome distribution.
- [ ] Visualize simulated score distributions per player using `plotters`.
- [ ] Allow step-through replay of a single simulated game (trick by trick).

