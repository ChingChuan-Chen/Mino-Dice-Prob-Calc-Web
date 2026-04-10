# Runtime Benchmarks

This document covers the two native-target benchmark examples shipped with this project.
Both examples measure hot-path computation in a release build; they are **not** run in WASM.

## Quick Start

```bash
# DP vs Monte Carlo — trick-count distribution runtime across all player counts
cargo run --release --example benchmark_dp_vs_monte_carlo

# Static table vs dynamic enumeration — single-trick win probability
cargo run --release --example benchmark_6p_static_vs_dynamic
```

> **Always use `--release`.**  Debug builds include overflow checks and produce timings that are 5–50× slower and unrepresentative of production performance.

---

## 1. `benchmark_dp_vs_monte_carlo`

### What it measures

For every player count (3, 4, 5, 6) the benchmark runs two algorithms that both compute the full trick-count probability distribution — $P(K=k)$ for $k = 0 \ldots h$ — for a fixed set of randomly generated hands:

| Algorithm | Function | Description |
|---|---|---|
| **DP** | `analytical_trick_count_distribution` | Bag-aware, seat-aware dynamic programming.  Exact within model assumptions.  Cost grows with player count. |
| **MC** | `monte_carlo_trick_count_distribution` | Repeated random simulation of full rounds.  Cost is proportional to `n_samples`, nearly flat across player counts. |

Each algorithm sees the **same set of hands** (same random seed, same `REPLICATIONS` hands per player count), so timings are directly comparable.

### How to run

```bash
cargo run --release --example benchmark_dp_vs_monte_carlo
```

#### Source constants (edit `examples/benchmark_dp_vs_monte_carlo.rs` to tune)

| Constant | Default | Effect |
|---|---|---|
| `REPLICATIONS` | `20` | Number of random hands per player count.  Increase (e.g., to 100) for more stable averages; this multiplies total runtime. |
| `MC_SAMPLE_COUNTS` | `[1_000, 5_000, 10_000]` | MC sample sizes tried per hand.  Add or remove entries; each entry multiplies MC time linearly. |
| `SEED` | `20_260_410` | Base RNG seed.  Change to get a different random hand set.  Each player count uses `SEED + player_count` so they never share hands. |

### Output format

```
=== 4-player  (hand_size=8, replications=20) ===
  DP (analytical):  total=123.936 ms   avg=6196.8 µs/hand
  MC (n=   1000):  total=87.085 ms   avg=4354.2 µs/hand   ratio=0.7x   max|E_gap|=0.3460
  MC (n=   5000):  total=434.970 ms   avg=21748.5 µs/hand   ratio=3.5x   max|E_gap|=0.2768
  MC (n=  10000):  total=868.246 ms   avg=43412.3 µs/hand   ratio=7.0x   max|E_gap|=0.2583
```

#### Column definitions

| Column | Unit | Meaning |
|---|---|---|
| `total` | ms | Wall-clock time to process all `REPLICATIONS` hands with this algorithm/sample-count. |
| `avg` | µs/hand | `total / REPLICATIONS`.  Average time to compute the full distribution for one hand. |
| `ratio` | — | `MC total / DP total` for this player count.  Values > 1 mean MC is slower; values < 1 mean MC is faster. |
| `max\|E_gap\|` | tricks | Maximum absolute difference in **expected tricks** between the DP result and MC result across all hands: $\max_i \lvert E_\text{DP}[K]_i - E_\text{MC}[K]_i \rvert$.  Measures MC accuracy relative to DP. |

Note: `max|E_gap|` measures accuracy on expected tricks (a single scalar per hand), not on the full distribution shape.  It is worst-case over all hands in the batch, so it decreases slowly as `n_samples` grows and fluctuates run to run.

### Reference results

Measured on 2026-04-10, `cargo run --release --example benchmark_dp_vs_monte_carlo`,
REPLICATIONS=20, MC_SAMPLE_COUNTS=[1000, 5000, 10000], SEED=20260410.

```
Benchmark: DP vs Monte Carlo trick-count distribution
Replications per player count: 20
MC sample counts: [1000, 5000, 10000]
Seed: 20260410

=== 3-player  (hand_size=8, replications=20) ===
  DP (analytical):  total=13.749 ms   avg=687.5 µs/hand
  MC (n=   1000):  total=62.733 ms   avg=3136.6 µs/hand   ratio=4.6x   max|E_gap|=0.5126
  MC (n=   5000):  total=314.693 ms   avg=15734.7 µs/hand   ratio=22.9x   max|E_gap|=0.4400
  MC (n=  10000):  total=628.475 ms   avg=31423.7 µs/hand   ratio=45.7x   max|E_gap|=0.4504

=== 4-player  (hand_size=8, replications=20) ===
  DP (analytical):  total=123.936 ms   avg=6196.8 µs/hand
  MC (n=   1000):  total=87.085 ms   avg=4354.2 µs/hand   ratio=0.7x   max|E_gap|=0.3460
  MC (n=   5000):  total=434.970 ms   avg=21748.5 µs/hand   ratio=3.5x   max|E_gap|=0.2768
  MC (n=  10000):  total=868.246 ms   avg=43412.3 µs/hand   ratio=7.0x   max|E_gap|=0.2583

=== 5-player  (hand_size=7, replications=20) ===
  DP (analytical):  total=746.127 ms   avg=37306.3 µs/hand
  MC (n=   1000):  total=95.132 ms   avg=4756.6 µs/hand   ratio=0.1x   max|E_gap|=0.1847
  MC (n=   5000):  total=498.666 ms   avg=24933.3 µs/hand   ratio=0.7x   max|E_gap|=0.1604
  MC (n=  10000):  total=966.504 ms   avg=48325.2 µs/hand   ratio=1.3x   max|E_gap|=0.1626

=== 6-player  (hand_size=6, replications=20) ===
  DP (analytical):  total=4110.562 ms   avg=205528.1 µs/hand
  MC (n=   1000):  total=95.324 ms   avg=4766.2 µs/hand   ratio=0.0x   max|E_gap|=0.1649
  MC (n=   5000):  total=475.285 ms   avg=23764.2 µs/hand   ratio=0.1x   max|E_gap|=0.1570
  MC (n=  10000):  total=950.096 ms   avg=47504.8 µs/hand   ratio=0.2x   max|E_gap|=0.1421
```

### Interpreting the numbers

#### DP cost grows super-linearly with player count; MC cost is nearly flat

The DP state space is `(wins, seat, bag composition)` and expands multiplicatively with player count.  Each additional player adds more possible winner outcomes per trick, more seat transitions, and more bag-composition paths.

| Players | DP avg (µs/hand) | MC avg at n=10k (µs/hand) | crossover |
|---------|-----------------|--------------------------|-----------|
| 3 | 688 | 31,424 | DP wins by **46×** |
| 4 | 6,197 | 43,412 | DP wins by **7×** |
| 5 | 37,306 | 48,325 | roughly **equal** |
| 6 | 205,528 | 47,505 | MC wins by **4×** |

At 3 players the DP completes in under 1 ms per hand — fast enough for a real-time UI response with no perceived lag.  At 6 players the DP takes ~200 ms per hand, which is noticeable but still single-frame.  The crossover point lies between 5 and 6 players for this sample size.

#### MC cost scales linearly with `n_samples`; DP cost does not

Doubling `n_samples` doubles MC time.  DP cost is independent of any sample count — it is always deterministic and exact within model assumptions.  This is why MC at n=1000 (cheap) sometimes beats DP at high player counts, but MC at n=10000 (10× pricier) becomes slower again at 4–5 players.

#### MC accuracy (max|E_gap|) improves slowly with `n_samples` and with player count

The `max|E_gap|` column shows how far the worst-case hand's expected-tricks estimate drifts from the DP reference:

| Players | MC 1k gap | MC 5k gap | MC 10k gap |
|---------|----------|----------|-----------|
| 3 | 0.51 tricks | 0.44 tricks | 0.45 tricks |
| 4 | 0.35 tricks | 0.28 tricks | 0.26 tricks |
| 5 | 0.18 tricks | 0.16 tricks | 0.16 tricks |
| 6 | 0.16 tricks | 0.16 tricks | 0.14 tricks |

Two effects explain the pattern:

1. **Variance shrinks with player count.**  With more opponents the player wins fewer tricks on average and the distribution is tighter, so random sampling noise has less absolute room to deviate.
2. **Sample size has diminishing returns.**  Going from 1k to 10k (10× more samples) reduces the 3-player gap only from 0.51 to 0.45 tricks.  To cut the gap in half you need roughly 4× more samples (standard-error ∝ $1/\sqrt{n}$).

For a game assistant that recommends a bid the expected-tricks gap matters less than the distribution shape near the optimal bid, so even 1k–5k MC samples can be sufficient for quick feedback.  Use DP for players ≤ 5 where it is faster and exact.

---

## 2. `benchmark_6p_static_vs_dynamic`

### What it measures

This benchmark compares two implementations of **single-trick win probability** for 6 players:

| Path | Function | Description |
|---|---|---|
| **Static** | `win_probabilities_for_all_seats` | Looks up pre-computed per-seat win probabilities from an in-memory reference table built at startup.  O(1) per hand. |
| **Dynamic** | `win_probabilities_for_all_seats_dynamic` | Re-enumerates all outcome tuples on every call (up to $3^6 = 729$ tuples for 6 players).  Correct by construction. |

### How to run

```bash
cargo run --release --example benchmark_6p_static_vs_dynamic
```

Source constants are at the top of `examples/benchmark_6p_static_vs_dynamic.rs`:

| Constant | Default | Meaning |
|---|---|---|
| `PLAYER_COUNT` | `6` | Fixed at 6 (the benchmark targets the worst-case player count for enumeration). |
| `REPLICATIONS` | `10_000` | Number of random 6-die hands timed. |
| `SEED` | `20_260_410` | RNG seed for hand generation. |

### Output format

```
Benchmark: static table vs dynamic enumeration
Players: 6
Replications: 10000
Seed: 20260410

Static table path:
  total time: 1.131 ms
  avg time:   0.113 us/hand

Dynamic path:
  total time: 1968.056 ms
  avg time:   196.806 us/hand

Relative ratio (dynamic/static): 1740.11x
Max abs diff between paths: 0.000e0
```

#### Column definitions

| Field | Unit | Meaning |
|---|---|---|
| `total time` | ms | Wall-clock time to compute win probabilities for all `REPLICATIONS` hands. |
| `avg time` | µs/hand | `total / REPLICATIONS`. |
| `Relative ratio` | — | `dynamic total / static total`.  How many times slower the dynamic path is. |
| `Max abs diff` | probability | Maximum element-wise absolute difference between per-seat probabilities from the two paths.  `0.000e0` means the outputs are numerically identical. |

### Reference results

Measured on 2026-04-10, `cargo run --release --example benchmark_6p_static_vs_dynamic`,
REPLICATIONS=10000, SEED=20260410.

```
Benchmark: static table vs dynamic enumeration
Players: 6
Replications: 10000
Seed: 20260410

Static table path:
  total time: 1.131 ms
  avg time:   0.113 us/hand
  checksum:   10000.000000

Dynamic path:
  total time: 1968.056 ms
  avg time:   196.806 us/hand
  checksum:   10000.000000

Relative ratio (dynamic/static): 1740.11x
Max abs diff between paths: 0.000e0
```

### Interpreting the numbers

**The static lookup is ~1,740× faster than dynamic enumeration.**  Both paths produce **identical outputs** (`Max abs diff = 0.000e0`), confirming that the table was built correctly from the same enumeration logic.

The dynamic path spends ~197 µs per hand enumerating up to 729 face-outcome tuples and computing win probabilities from scratch every time.  The static path replaces that with a single hash-map lookup built once at program start, reducing per-hand cost to ~0.11 µs.

This speedup is the reason the DP (`analytical_trick_count_distribution`) can afford to call win-probability computation in an inner loop: each call hits the pre-built table, not the full enumerator.  Without the static table the DP would be thousands of times slower.

---

## 3. Customising and Extending the Benchmarks

### Changing constants without recompiling

The constants `REPLICATIONS`, `MC_SAMPLE_COUNTS`, and `SEED` are defined at the top of each example source file.  Edit and recompile:

```bash
# Edit examples/benchmark_dp_vs_monte_carlo.rs, then:
cargo run --release --example benchmark_dp_vs_monte_carlo
```

### Running a specific player count only

The `bench_player_count` function in `benchmark_dp_vs_monte_carlo.rs` is a standalone helper.  You can call it from a modified `main` with only the player counts you care about.

### Redirecting output to a file

```bash
cargo run --release --example benchmark_dp_vs_monte_carlo > results.txt 2>&1
```

### Comparing before and after a code change

1. Record baseline: `cargo run --release --example benchmark_dp_vs_monte_carlo > before.txt`.
2. Make your change.
3. Record after: `cargo run --release --example benchmark_dp_vs_monte_carlo > after.txt`.
4. Diff: `diff before.txt after.txt`.

---

## 4. Summary: When to Prefer DP vs Monte Carlo

| Scenario | Recommendation |
|---|---|
| **≤ 5 players, interactive UI** | Use **DP**.  It is faster and produces a deterministic, noise-free distribution every time. |
| **6 players** | Use **MC** with ≥ 5,000 samples, or accept the ~200 ms DP cost if exact results are required. |
| **Accuracy over speed** | Use **DP** (exact within model) for any player count, then refine with MC only if model residuals matter. |
| **Speed over accuracy** | Use **MC at n=1,000**.  It delivers ~0.16–0.51 trick expected-value error but takes only ~4–5 ms/hand regardless of player count. |
| **Behavioral simulation** (special captures, full-game scoring) | MC only — the DP state does not capture path-dependent bonus events. |

See [Probability Model § 7](probability_model.md#7-why-bag-aware--seat-aware-dp-and-monte-carlo-still-differ) for a detailed explanation of the structural differences between DP and MC outputs.
