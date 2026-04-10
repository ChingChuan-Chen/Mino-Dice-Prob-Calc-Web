# Mino Dice Rules

## Game Rules: Mino Dice (a.k.a. Mythical Dice)

**BGG:** [Mythical Dice (2016)](https://boardgamegeek.com/boardgame/191071/mythical-dice) — reimplements [Skull King](https://boardgamegeek.com/boardgame/150145/skull-king)
**Players:** 3–6 | **Play Time:** ~30 min | **Age:** 8+

Mino Dice is a trick-taking game with simultaneous bidding. Players draw dice secretly from a bag, bid how many tricks they will win, then compete round by round. The player who predicts their performance most accurately earns the most points.

## Components

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

## Round Structure

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

## Scoring

| Outcome | Points |
|---|---|
| Made bid exactly (bid > 0) | +20 × bid |
| Missed bid (bid > 0) | −10 × \|bid − tricks taken\| |
| Bid 0 and succeeded | +10 × tricks in the hand |
| Bid 0 and failed | −10 × tricks in the hand |
| **Bonus:** Captured a Minotaur with a Mermaid (no Flag captured) | +50 |
| **Bonus:** Captured a Griffin with a Minotaur (no Flag captured) | +30 |

The player with the **highest total score** after the final hand wins.