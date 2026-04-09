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
| Minotaur die | ![minotaur](assets/dice/minotaur_die.svg) | Minotaur ×4, Flag ×2 | 1 | Special character die (dark red) |
| Griffin die | ![griffin](assets/dice/griffin_die.svg) | Griffin ×4, Flag ×2 | 3 | Special character die (green) |
| Mermaid die | ![mermaid](assets/dice/mermaid_die.svg) | Mermaid ×4, Flag ×2 | 2 | Special character die (blue) |
| Red die | ![red](assets/dice/red_die.svg) | 7 ×2, 6 ×2, 5 ×2 | 7 | High-value number die |
| Yellow die | ![yellow](assets/dice/yellow_die.svg) | 5 ×2, 4 ×2, 3 ×2 | 7 | Mid-value number die |
| Purple die | ![purple](assets/dice/purple_die.svg) | 3 ×2, 2 ×2, 1 ×2 | 8 | Low-value number die |
| Gray die | ![gray](assets/dice/gray_die.svg) | Flag ×3, 1 ×2, 7 ×1 | 8 | Mostly flags; risky suit die |

**Total: 36 dice in bag** (1 + 3 + 2 + 7 + 7 + 8 + 8)

### Round Structure

The game plays **6–8 hands** depending on player count. In 3p-4p game, there are 8 rounds. In 5p game, there are 7 rounds. In 6p game, there are 6 rounds.
In hand *n*, each player draws **n dice** from the bag and hides them behind their screen.

**Step 1 — Bid:** All players simultaneously reveal their bid (number of tricks they expect to win) by holding up fingers. Bids are recorded on the scoresheet.

**Step 2 — Play tricks:** The leading player picks one of their hidden dice, rolls it publicly, then:
- If a **number die** is rolled, every other player must follow suit by rolling a number die of the **same color** if they have one; otherwise they may roll any die.
- A player may **always** choose to roll a **special character die** (Minotaur, Griffin, or Mermaid) instead of following suit.

**Step 3 — Determine trick winner:**
- Special character dice beat all number dice.
- Among special characters: **Minotaur > Griffin > Mermaid > Minotaur** (rock-paper-scissors cycle).
- Among number dice only: the **highest number** wins; ties go to the **later roller**.
- A rolled **Flag face** counts as 0 and loses to any non-Flag face.
- If **all rolled faces are Flags**, the **last roller wins** the trick.
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

## Probability Model

This section explains the mathematics behind the calculator.
All computation runs in Rust compiled to WebAssembly; no data ever leaves the browser.

### Step 1 — Single-die face distribution (`src/dice.rs`)

Each die type has 6 faces.  Equal-probability faces are merged, giving a
probability mass function (PMF) for each die.  For example, a Red die yields:

| Face | Count | Probability |
|------|-------|-------------|
| 5 | 2 | 1/3 |
| 6 | 2 | 1/3 |
| 7 | 2 | 1/3 |

### Step 2 — Single-trick win probability (`src/trick.rs`)

Given a set of *n* dice (one per player), the win probability for a target player
is computed by **full enumeration** of all face combinations:

$$P(\text{player } t \text{ wins}) = \sum_{\mathbf{f} \in F_1 \times \cdots \times F_n} \left[ \text{winner}(\mathbf{f}) = t \right] \prod_{i=1}^{n} P(f_i)$$

where $F_i$ is the set of distinct faces of player $i$'s die, and $P(f_i)$ is the
probability of that face.

The trick-winner function `trick_winner` applies the full rule hierarchy:

1. **Flags** (value 0) lose to any non-Flag face. If all rolled faces are Flags, the later roller wins the tie.
2. **Special characters** (Minotaur, Griffin, Mermaid) beat all number faces.
3. Among specials the **rock-paper-scissors cycle** applies:
   Minotaur > Griffin > Mermaid > Minotaur.
   If two identical specials tie, the **later roller** wins.
4. Among number faces only: **highest number** wins; ties go to the **later roller**.

Because each die has at most 3 distinct face outcomes and the game supports at
most 6 players, the product $|F_1| \times \cdots \times |F_n|$ is bounded by
$3^6 = 729$ per trick, making exhaustive enumeration fast (sub-millisecond in WASM).

### Step 3 — Trick-count distribution for a full hand (`src/round.rs`)

For a hand of *h* dice against *n*−1 opponents the calculator needs
$P(\text{win exactly } k \text{ tricks})$ for $k = 0, \ldots, h$.

It uses an **independence approximation** + **dynamic programming (DP)**:

> **Approximation:** the *h* tricks are treated as independent Bernoulli trials — die
> depletion across tricks within a round is ignored.  In practice this is accurate
> enough because each player's hand is drawn uniformly from the full bag and the
> per-trick win probability barely changes as dice are removed.

For each trick slot $j = 0, \ldots, h-1$:

1. Assign the player's $j$-th die and each opponent's $(j \bmod h_{\text{opp}})$-th die.
2. Compute $p_j = P(\text{player wins trick } j)$ via Step 2.
3. Update the DP table:

$$\text{dp}[k] \leftarrow \text{dp}[k-1] \cdot p_j + \text{dp}[k] \cdot (1 - p_j)$$

This produces the full PMF $\{P(k)\}_{k=0}^{h}$ in $O(h^2)$ time, typically under 1 ms.

### Step 4 — Optimal bid and expected score (`src/round.rs`)

Given the PMF, the expected score for any bid $b$ is:

$$E[\text{score} \mid b] = \begin{cases}
+10h \cdot P(k=0) - 10h \cdot P(k>0) & b = 0 \\
\sum_{k=0}^{h} P(k) \cdot \begin{cases} +20b & k = b \\ -10|k - b| & k \neq b \end{cases} & b > 0
\end{cases}$$

The **optimal bid** is $\arg\max_b\, E[\text{score} \mid b]$.

### Step 5 — Monte Carlo round simulation (`src/round.rs`)

The current simulation UI runs *N* samples of a selected round number, where the number of dice drawn equals the round number:

1. **Choose a round** — the user selects a legal round from `1..=round_count(player_count)`.
2. **Draw hands** — each simulated player draws exactly that many dice using a Fisher-Yates partial shuffle on the 36-die bag.
3. **Bid** — each simulated player computes its optimal bid using Steps 2–4.
4. **Play tricks** — each trick is resolved by rolling a uniformly random face for each die using a lightweight **Xorshift64** RNG (no OS entropy; safe in WASM).
5. **Score** — scoring rules are applied for that round only, producing a single-round score sample.

This produces per-player score distributions for the chosen round, which is more useful when you want to study how hand size changes risk and expected outcome.

## Worked Session: 3 Players, 3 Dice In Hand

This is a concrete calculator session you can replay.
It uses the same Monte Carlo path as the current UI for hands larger than 1 die.

### Inputs

- Player count: 3
- Hand size: 3
- Player hand: Mermaid, Red, Gray
- Monte Carlo replications: 100,000
- Seed: 20260410

Because the current UI stores the hand in die-type order, this example is evaluated as:

1. Trick 1 uses Mermaid.
2. Trick 2 uses Red.
3. Trick 3 uses Gray.

### What math the app applies

1. Remove the player's three dice from the 36-die bag.
2. For each Monte Carlo replication, draw two opponent hands of 3 dice each from the remaining 33 dice.
3. Play a 3-trick round using the simulator in `src/round.rs`.
4. Record how many tricks the player wins: 0, 1, 2, or 3.
5. Normalize those counts into the chart probabilities.

So the chart bars are estimates of:

$$P(K = 0),\; P(K = 1),\; P(K = 2),\; P(K = 3)$$

where $K$ is the number of tricks won by the player in that 3-trick hand.

The expected tricks shown above the chart are:

$$E[K] = \sum_{k=0}^{3} k \cdot P(K = k)$$

The expected score for each bid $b$ in the table is:

$$E[\text{score} \mid b] = \sum_{k=0}^{3} P(K = k) \cdot \text{score}(b, k)$$

and the highlighted recommendation is the bid with the largest expected score.

### What the chart gives for this session

Run this command to reproduce the current numbers exactly:

```bash
cargo run --example showcase_3p_3dice
```

The example prints the chart distribution, expected tricks, expected score per bid, and the top 3 exact opponent-hand patterns from the remaining bag.

With the current code, that command yields:

| Tricks won `k` | Chart value `P(K = k)` |
|---|---|
| 0 | 13.32% |
| 1 | 53.90% |
| 2 | 29.82% |
| 3 | 2.96% |

So the chart is telling us that this hand most often wins exactly **1 trick**, sometimes **2 tricks**, and only rarely gets swept or takes all 3.

From those bars, the calculator reports:

- Expected tricks: **1.2242**
- Optimal bid: **1**
- Expected score at bid 0: **−22.0074**
- Expected score at bid 1: **+5.8729**
- Expected score at bid 2: **+3.5799**
- Expected score at bid 3: **−15.9825**

That is why the UI highlights **bid 1** for this session.

This is the intended reading of the chart for the example hand:

- Bar `0`: probability of getting swept and winning no tricks.
- Bar `1`: probability of winning exactly one trick.
- Bar `2`: probability of winning exactly two tricks.
- Bar `3`: probability of taking all three tricks.

The top-opponent list is separate from the chart: it is an exact combinatorial summary of the most likely 3-die opponent hands from the remaining bag, not a Monte Carlo estimate.

For this hand, the current top 3 opponent patterns are:

| Rank | Opponent hand pattern | Exact probability |
|---|---|---|
| 1 | Yellow, Purple, Gray | 7.1848% |
| 2 | Red, Purple, Gray | 6.1584% |
| 3 | Red, Yellow, Purple | 6.1584% |

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

- [ ] Add a game rules summary / quick-reference card in the UI.
- [ ] Optimize WASM binary size (`wasm-opt`, `lto`, `opt-level = "z"`).
- [ ] Deploy as a static site (GitHub Pages or Cloudflare Pages) — no server required.
- [ ] Write a user-facing README / how-to-use guide.

### Phase 6 — In-Browser Game Simulation

- [ ] Create a new tab to have a full game simulation.
- [ ] Implement a full game simulation engine in Rust: set the number of players, draw dice from bag, play rounds, apply scoring rules.
- [ ] Expose simulation API via WASM: run N simulated games given a hand, return aggregated statistics.
- [ ] Build a simulation UI page: let users set up a game state (hand, player count, round), run simulation, view outcome distribution.
- [ ] Visualize simulated score distributions per player using `plotters`.
- [ ] Allow step-through replay of a single simulated game (trick by trick).

