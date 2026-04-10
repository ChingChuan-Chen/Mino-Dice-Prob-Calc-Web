# Probability Model

This document introduces the probability model from game math first, then maps that model to code variants.
All computation is local (Rust/WASM in browser, Rust native for tooling).

## 1. End-to-End Objective

Given:

1. Player count $n \in [3,6]$.
2. A player hand of size $h$.
3. Player order (seat) in the current trick.

Estimate:

1. Trick-count distribution $P(K=k)$ for $k=0..h$.
2. Expected score by bid.
3. Best bid $\arg\max_b E[\text{score}\mid b]$.

## 2. Primitive Random Variable: Single Die Face

Each die has 6 faces. Duplicate outcomes are merged into a PMF.

Example (Red):

| Face | Count | Probability |
|---|---|---|
| 5 | 2 | 1/3 |
| 6 | 2 | 1/3 |
| 7 | 2 | 1/3 |

## 3. Single-Trick Winner Probability

For fixed dice in one trick, winner probability is exact enumeration:

$$
P(\mathrm{seat}\ t\ \mathrm{wins})
=
\sum_{\mathbf{f}:\,\mathrm{winner}(\mathbf{f})=t}\prod_i P(f_i)
$$

Winner rules:

1. Flags lose to any non-flag. If all flags, later roller wins.
2. Specials beat number faces.
3. Special cycle: Minotaur > Griffin > Mermaid > Minotaur.
4. Number vs number: higher wins, ties to later roller.

At most 6 players and at most 3 merged outcomes per die gives at most $3^6=729$ outcome tuples per trick.

## 4. Full-Hand Distribution Model

This project keeps both approaches:

1. Seat-aware inter-trick DP (baseline).
2. Bag-aware + seat-aware DP (current improved estimator).

### 4.1 Seat-Aware Inter-Trick DP (Baseline)

For hand size $h$, define state by wins and current seat:

$$
\mathrm{dp}_j[k,s]=P(\text{after } j \text{ tricks: wins}=k,\text{ seat}=s)
$$

Given seat $s$, let winner probability be:

$$
q_j(w\mid s)
$$

Transition:

$$
\mathrm{dp}_{j+1}[k',s'] += \mathrm{dp}_j[k,s]\cdot q_j(w\mid s)
$$

where

$$
s'=(s-w)\bmod n,\qquad k'=k+\mathbf{1}[w=s]
$$

### 4.2 Bag-Aware + Seat-Aware DP (Current)

Current DP keeps the same seat transition, and additionally tracks expected remaining opponent bag composition.

Per-state expected remaining opponent bag (die-type index $t$):

$$
\mathrm{rem}_j[k,s,t]=E[\text{remaining count of die type }t\mid j,k,s]
$$

Winner probabilities are then conditioned on both seat and expected remaining bag:

$$
q_j(w\mid s,\mathrm{rem}_j[k,s,\cdot])
$$

Transition keeps the same seat-update formula:

$$
\mathrm{dp}_{j+1}[k',s'] += \mathrm{dp}_j[k,s]\cdot q_j(w\mid s,\mathrm{rem}_j[k,s,\cdot])
$$

For each winner outcome, expected opponent draw composition is removed from state bag expectation before accumulating into $\mathrm{rem}_{j+1}$.

Final trick-count distribution:

$$
P(K=k)=\sum_s \mathrm{dp}_h[k,s]
$$

For $h=1$, code uses the existing exact single-trick path.

## 5. Score Model

Let $K$ be tricks won and $B$ bonus points.

$$
\text{base}(b,k)=
\begin{cases}
+10h & b=0,\ k=0 \\
-10h & b=0,\ k>0 \\
20b & b>0,\ k=b \\
-10|k-b| & b>0,\ k\ne b
\end{cases}
$$

$$
E[\text{score}\mid b]=\sum_k P(K=k)\cdot \text{base}(b,k)+E[B]
$$

Here, $E[B]$ is an additive expected bonus term from special captures.
In code this is modeled explicitly by total-score utilities and round simulation.

Optimal bid is $\arg\max_b E[\text{score}\mid b]$.

## 6. Code Variants

### 6.1 Exact Single-Trick Engine

Implemented in `src/trick.rs`.
This is exact and is the foundation for both lookup and DP transitions.

### 6.2 Static Lookup Variant

For 3-6 players, single-trick results are precomputed and loaded from SQLite.

Storage format:

1. Key: `player_count` + `seq_idx`.
2. `seq_idx` = base-7 encoding of ordered seat dice.
3. Value: per-seat integer win counts with denominator.

This compact integer key replaces long string keys and improves storage/index efficiency.

### 6.3 Seat-Aware Inter-Trick DP Variant (Baseline)

A seat-aware inter-trick DP baseline is retained for comparison/reference in tests and analysis.

### 6.4 Bag-Aware + Seat-Aware DP Variant (Current)

Implemented in `src/round.rs` using the bag-aware model above.
It is fast and stable for interactive UI use.

### 6.5 Monte Carlo Variant

Also in `src/round.rs`, used as behavioral reference/simulation:

1. Remove player hand from bag.
2. Sample opponent hands from remaining dice.
3. Simulate trick order, lead changes, and dice usage.
4. Track trick counts and bonus captures.

Die-choice policy in simulation:

1. If led color exists and follower has matching color, legal choices are matching-color dice plus any special dice.
2. If no matching color exists, all remaining dice are legal.
3. If multiple legal choices exist, choose uniformly among legal choices.

## 7. Why Bag-Aware + Seat-Aware DP and Monte Carlo Still Differ

The current bag-aware + seat-aware DP reduces the baseline seat-only approximation, but it still compresses trajectory detail. Main reasons for residual gaps include:

1. DP tracks expected remaining opponent bag composition per state, not the full joint distribution of every opponent hand.
2. DP transitions are aggregated by winner outcomes, while Monte Carlo simulates concrete legal-play choices and exact per-player depletion paths.
3. Nonlinear bonus events are path dependent. Special-capture bonuses depend on exact face/trick histories not fully represented in compressed DP state.
4. Opponent-opponent interactions are explicit in Monte Carlo every trick, but only represented through aggregated transitions in DP.
5. Finite-sample noise exists in Monte Carlo outputs. Even with 100000 replications, small probability cells can fluctuate.
6. Numerical/rounding presentation can amplify visible differences.

So DP is the fast estimator; Monte Carlo is the higher-fidelity behavioral simulator.

## 8. Current Calibration Examples

### 8.1 4 Players

Scenario:

1. Player count: 4
2. Hand: Mermaid, Red, Gray
3. Play order: 1st

Measured results (from `cargo run --example compare_trick_distribution`, 100000 Monte Carlo replications, seed = 42, measured on 2026-04-10):

| Method | P(K=0) | P(K=1) | P(K=2) | P(K=3) | Expected tricks | Optimal bid |
|---|---|---|---|---|---|---|
| DP | 21.86% | 53.27% | 23.29% | 1.59% | 1.0460 | 1 |
| Monte Carlo | 22.42% | 56.53% | 19.69% | 1.37% | 1.0001 | 1 |

Expected-tricks gap:

$$
|1.0460-1.0001|=0.0459
$$

### 8.2 6 Players

Scenario:

1. Player count: 6
2. Hand: Mermaid, Red, Gray
3. Play order: 1st

Measured results (from `cargo run --example compare_trick_distribution -- --players 6`, 100000 Monte Carlo replications, seed = 42, measured on 2026-04-10):

| Method | P(K=0) | P(K=1) | P(K=2) | P(K=3) | Expected tricks | Optimal bid |
|---|---|---|---|---|---|---|
| DP | 36.69% | 51.55% | 11.37% | 0.38% | 0.7545 | 1 |
| Monte Carlo | 36.94% | 53.66% | 9.09% | 0.31% | 0.7277 | 1 |

Expected-tricks gap:

$$
|0.7545-0.7277|=0.0267
$$

## 9. Repro Commands

```bash
cargo run --example compare_trick_distribution
cargo run --example compare_trick_distribution -- --players 6
cargo run --example benchmark_6p_static_vs_dynamic
cargo run --example build_probability_sqlite_db
```
