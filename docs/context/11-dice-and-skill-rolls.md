# Dice System & Skill Roll Mechanics (Call of Cthulhu 7e)

> Context doc for the CoC Phoenix tracker backend. Mechanics are paraphrased in our own words
> from public/free sources, with page pointers into the commercial books for verification.
> No rule text appears verbatim.

## Overview

Nearly everything in 7th Edition resolves through one mechanic: a roll-under d100 check
against a percentile value (skill rating or characteristic). The outcome is not binary — the
roll lands on a **success level** somewhere on an ordered ladder (Fumble < Failure < Regular
< Hard < Extreme < Critical), determined by comparing the die against the target value, half
of it, and one fifth of it. Independently, the Keeper declares a **required difficulty**
(Regular / Hard / Extreme); the check passes when the achieved level reaches or exceeds that
bar. On top sit three modifier systems a rules engine must keep strictly apart: *difficulty*
(moves the bar), *bonus/penalty dice* (changes the dice pool, never the bar), and *Luck
spending* (edits the rolled number after the fact). Two mechanics leave state behind after
the roll resolves — **pushed rolls** (a retry that turns plain failure into Keeper-authored
disaster) and **skill improvement flags**. Everything else in this doc — opposed, combined,
and group rolls — composes out of those primitives.

---

## 1. The d100 and how a result is read

Physically, the percentile roll is **two dice**: a tens die showing 10–90 plus 00, and a
units die showing 1–9 plus 0. That physical detail is load-bearing, because bonus/penalty
dice add *extra tens dice only* while the single units die is shared. Model the roll as an
opaque `randint(1, 100)` and the distribution goes wrong the instant a bonus die appears.

Reading rules:

| Tens die | Units die | Result |
|---|---|---|
| 00 | 0 | 100 |
| 00 | 7 | 7 |
| 30 | 0 | 30 |
| 90 | 9 | 99 |

One special case only: double-zero reads as **100**, never 0. A roll of 0 does not exist;
legal results span 1–100 inclusive.

```
def read(tens_index, units):        # tens_index in 0..9, units in 0..9
    return 100 if (tens_index == 0 and units == 0) else tens_index * 10 + units
```

---

## 2. Success levels and thresholds

From an effective target value `T` (the rating after any flat modifiers), derive three
thresholds by integer division, always rounding **down**:

```
regular_target = T
hard_target    = floor(T / 2)
extreme_target = floor(T / 5)
```

Shape of the reference chart shipped in the rulebook (2 rows, reworded):

| Rating | Hard | Extreme |
|---|---|---|
| 45 | 22 | 9 |
| 60 | 30 | 12 |

Classification runs **in this exact order**:

1. Roll of exactly **01** → Critical success.
2. Roll at or past the fumble threshold (§4) → Fumble.
3. Roll ≤ `extreme_target` → Extreme success.
4. Roll ≤ `hard_target` → Hard success.
5. Roll ≤ `regular_target` → Regular success.
6. Anything else → Failure.

Because the ladder is totally ordered, opposed rolls become trivially cheap:

```
FUMBLE < FAILURE < REGULAR < HARD < EXTREME < CRITICAL
```

The Foundry VTT CoC7 system stores this as integers (`fumble: -99, failure: 0, regular: 1,
hard: 2, extreme: 3, critical: 4`) — a model worth copying, since the wide negative gap under
fumble stops any accidental "fumble = failure minus one" arithmetic.

**Degenerate thresholds.** Low ratings collapse the ladder: rating 4 yields
`extreme_target = 0`, making Extreme unreachable; rating 1 collapses `hard_target` to 0 as
well. Treat a threshold of 0 as "unreachable" rather than something a roll could match (no
roll of 0 exists, so correctness comes free, but the *display* of an unreachable tier should
be hidden). Foundry suppresses any tier whose threshold isn't strictly above the critical
threshold of 1.

**Ratings above 100.** Skills and characteristics may legitimately exceed 100. That grants no
infallibility: fumbles still happen and opposed contests can still be lost. Hard and Extreme
derive from the raw value (rating 110 → hard 55, extreme 22).

---

## 3. Difficulty levels

Difficulty belongs to the *task* and is fixed by the Keeper before any dice move. It never
touches the dice — it decides which rung of the ladder counts as passing.

- **Regular** — within a competent person's reach. Requires Regular or better.
- **Hard** — would stretch a professional. Requires Hard or better.
- **Extreme** — brushing the limits of human ability. Requires Extreme.

The success test is `achieved_level >= required_level`, and a Critical satisfies any bar. A
Regular success against an Extreme-difficulty task **fails the task** despite being a
"success" on the ladder — store both facts independently, because improvement eligibility and
narrative framing hang off different ones.

A published shortcut derives difficulty from a *passive* opponent's rating (Keeper Rulebook
~p.83): below 50 → Regular; 50–89 → Hard; 90+ → Extreme. It stands in for a full opposed roll
and deserves an API helper. Social-skill guidance follows the same contour: a friendly NPC
needs no roll; neutral is Regular; strong opposition pushes the bar up a rung or two; a
convincing threat or argument pulls it down one.

---

## 4. Criticals and fumbles

**Critical success: a roll of exactly 01.** Top of the ladder, independent of rating — a 5%
skill crits on 01 like anyone else. In combat a critical usually means maximum damage or an
exceptional outcome; the tracker records the flag and lets the combat subsystem decide what
it means.

**Fumble: the threshold depends on the chance of success.**

```
fumble_min = 96 if effective_chance < 50 else 100
is_fumble  = roll >= fumble_min
```

A 40% skill fumbles on 96–100 (5% of all rolls); a 70% skill fumbles only on 100 (1%). For
the 70% character, 96–99 is a plain failure.

**The ambiguity a backend must settle.** The text under-specifies "effective chance." Two
defensible readings exist:

1. **Base reading** — the unmodified skill/characteristic rating. A 60% skill rolled at
   Extreme difficulty still fumbles only on 100.
2. **Difficulty-adjusted reading** — the threshold actually needed. A 60% skill rolled at
   Hard difficulty has an effective chance of 30 and fumbles on 96–100.

Foundry implements reading (2): `#minimumFumbleFromThreshold()` divides by difficulty first,
then compares to 50. Whichever way the project rules, it must be one configurable policy,
applied identically everywhere, and stamped on the roll for audit.

**Check 01 before checking fumble** — 01 is never a fumble, even at ratings so low every
threshold has collapsed.

**Fumble vs. Critical Fumble.** Core rules define a single fumble tier. A separate,
worse "critical fumble" is a house rule (often attached to failed pushes). Keep it an
optional flag, not a sixth rung.

---

## 5. Bonus and penalty dice

The mechanic: each bonus or penalty die adds **one extra tens die** to the pool. The units
die rolls once and is shared by every candidate reading. Then:

- **Bonus die(s)** → keep the **lowest** candidate total.
- **Penalty die(s)** → keep the **highest** candidate total.

**Cancellation is 1-for-1 and precedes rolling.** A bonus and a penalty die wipe each other
out. Reduce to a signed net modifier first:

```
net = bonus_count - penalty_count      # positive = bonus, negative = penalty, 0 = plain roll
tens_dice_to_roll = abs(net) + 1
```

**Practical cap.** The rules intend bonus/penalty dice as coarse, meaningful swings, not fine
tuning — the guidance is to stop around two either way and shift difficulty instead. Foundry
hard-caps at `maxDiceBonus = 2` / `maxDicePenalty = 2`. Clamp the net rather than rejecting
the request.

**The 00-with-units-0 edge case.** Since double-zero reads 100, a tens die of 00 is sometimes
the *best* available tens result and sometimes the *worst*, depending on the units die. Units
= 0 with tens dice 00 and 40 gives candidates 100 and 40 — a bonus die keeps 40, a penalty
die keeps 100. Units = 6 with the same tens dice gives 6 and 46 — a bonus die keeps 6. Any
implementation that picks the "lower tens die" instead of the "lower total" fails here.
Always materialize every candidate total, then take min/max.

```
def roll_d100(net: int) -> tuple[int, list[int]]:
    units = randint(0, 9)
    tens_indices = [randint(0, 9) for _ in range(abs(net) + 1)]
    candidates = [read(t, units) for t in tens_indices]
    if net > 0:   result = min(candidates)
    elif net < 0: result = max(candidates)
    else:         result = candidates[0]
    return result, candidates
```

**Never approximate with `2d100 keep-lowest`.** Some VTT macros shorthand this as `2D100L1`
/ `2D100H1`, but that varies the units digit too and produces a different distribution. If
the tracker is the authority on rolls, implement the true construction and persist each tens
result so clients can render the dice.

**Ordering versus difficulty.** Difficulty comes first (a fact about the task); bonus/penalty
dice come after (a fact about circumstances). The order only matters under the
difficulty-adjusted fumble reading, where the fumble threshold derives from the adjusted
target and ignores the dice pool.

**A re-roll re-rolls the entire pool** — fresh units die, fresh tens dice, never a reused
units value.

---

## 6. Pushed rolls

A pushed roll is a second try at a *failed* check, unlocked by the player narrating extra
effort, time, or risk. It is a state transition on the existing check, not a fresh one.

**Preconditions**

- The original roll failed (didn't meet the required difficulty).
- The check is *pushable*. Skill and characteristic rolls are; Luck rolls, Sanity rolls,
  combat rolls, damage rolls, rolls fixing an amount of Sanity lost, and opposed rolls are
  not.
- Certain skills are situationally un-pushable (the lock you're out of ideas on). Foundry
  models a per-skill `push` property — a good pattern: expose `pushable: bool` on the skill
  definition with a Keeper override.
- In-fiction time must exist to spend. The rules note explicitly that time passes between
  the first roll and the push, which can matter to scene clocks and pursuers.
- Luck cannot be spent on a roll being pushed (§9). A failed roll therefore forks into
  *push* or *spend Luck* — never both.

**Resolution**

- The Keeper may foreshadow the price of failure; the player then commits.
- Push succeeds: goal achieved, and the earlier failure carries no penalty.
- Push fails: the Keeper is licensed to impose something severe — injury, a Sanity check,
  lost gear, capture, unwanted attention, a scene-level complication. Community summaries
  put the severity at fumble-grade.

**One push per check, period.** A pushed roll can't be pushed again. (Pushing after a
*fumble* is a known house rule; keep it behind a flag.)

**Implicit pushes.** Retries the fiction would allow anyway — re-attempting a social skill
after a rebuff, a second investigator trying the same lock — count as pushes, precisely to
kill skill-spam. Key push state to a *(scene, task)* pair rather than a roll id, and let the
Keeper tag a new roll as "counts as a push of task X."

---

## 7. Opposed rolls

Two parties with mutually exclusive goals each roll against their own rating, then compare
levels.

**Resolution order**

1. Both sides roll and classify normally. Each side's difficulty is Regular unless
   circumstances dictate otherwise; in a true opposed roll the *opponent's* rating does not
   set a threshold (that's the passive shortcut of §3 — use one mechanism or the other,
   never both).
2. Higher success level wins: Critical > Extreme > Hard > Regular > Failure > Fumble.
3. **Tie on level:** the higher skill/characteristic rating wins.
4. **Still tied (equal ratings):** declare an impasse, or both roll 1d100 with the lower
   result winning. Both options are sanctioned; pick a default, expose the other.
5. **Both failed:** nobody gets their goal — an impasse. The Keeper may re-run the contest
   if the fiction demands it.

Step 3 means the resolution record must capture the participants' *ratings*, not merely
their levels. Foundry compares levels first with a configurable tie-breaker, adding a "who
holds the advantage" notion for combat ties.

**Opposed rolls cannot be pushed** — the most frequently missed interaction. Reject any push
request whose parent roll belongs to an opposed contest.

Weighting an opposed contest is done with bonus/penalty dice, since shifting one side's
difficulty inside a symmetric contest is awkward.

---

## 8. Combined rolls

A combined roll compares **one d100 result against two or more ratings** at once — a device
that is both electrical and mechanical, or a skill paired with a characteristic. The Keeper
states up front whether success needs **all** listed ratings or **any** single one.

- *All* mode: the one roll must reach the required level against every listed rating.
- *Any* mode: reaching the required level against any one rating is enough.

Rolling once is the point — it avoids multiplying failure chances across separate rolls.
Each listed rating may carry its own difficulty, so store a list of
`(rating_ref, required_difficulty)` targets plus a combine mode.

Foundry's names — `combinedall` and `combinedany` — are clean vocabulary to adopt.
(Discussed in the 7e skill chapter; about p.55 in the combined printing and in the Keeper
Rulebook's skill-roll section.)

---

## 9. Luck rolls, group Luck, and spending Luck

**Luck as a rating.** Luck is a percentile value (rolled at creation, typically 3d6×5, with
a re-roll allowance for teenage investigators). A **Luck roll** is a normal roll-under check
against *current* Luck, called when the outcome hinges on circumstances beyond anyone's
control — is the needed item in the drawer, does the car start. Luck rolls ride the same
ladder and can be demanded at Hard or Extreme.

**Group Luck roll.** When one Luck roll covers the party, the member with the **lowest
current Luck** among those present rolls, and the result binds everyone. It's a deliberate
anti-gaming rule: compute "lowest Luck among present" from live scene membership and don't
let the group conveniently exclude their jinx.

**Spending Luck (optional rule, Keeper Rulebook ~p.99).** Points come off the current Luck
score to reduce a rolled result 1-for-1, after the roll is seen. Constraints:

- Only the spender's own roll.
- Only skill and characteristic rolls.
- **Never** Luck rolls, Sanity rolls, damage rolls, or rolls determining Sanity lost.
- Incompatible with pushing the same roll.
- Cannot manufacture a Critical, undo a Fumble, or dodge a firearm malfunction.
- Converting a failure into a success by Luck forfeits the improvement mark.
- Spent Luck lowers the current value, which weakens future Luck rolls — genuinely
  consumable, so it needs a mutable per-character resource with an audit trail.

**Recovering Luck (optional).** A common end-of-session rule: roll 1d100 against current
Luck; exceeding it grants 1d10, capped at 99. Treat as a per-campaign toggle.

---

## 10. Group rolls, cooperation, and assistance

- **Parallel group rolls** — everyone rolls independently against their own rating (Spot
  Hidden, Listen, Psychology). No aggregation; the Keeper narrates each result.
- **Sequential attempts at one task** — attempt two and onward are pushes (§6).
- **Cooperation / assisted rolls** (Keeper Rulebook ~p.88) — one primary actor rolls;
  helpers lower the effective difficulty (or add a bonus die, per Keeper preference) instead
  of rolling themselves. If the primary fails, everyone fails.
- **First Aid is the carve-out** — a limited number of characters may each attempt it, any
  one success sufficing; attempts past the allowance become pushes.

Tables genuinely differ on cooperation (difficulty cut vs. bonus die), so ship it as a
policy setting, not a hard-coded rule.

---

## 11. Canonical resolution pipeline

```
resolve_check(actor, rating_ref, difficulty, bonus, penalty, flat_mod, policy):
    base       = lookup_rating(actor, rating_ref)         # may exceed 100
    effective  = max(0, base + flat_mod)
    targets    = { regular: effective,
                   hard:    effective // 2,
                   extreme: effective // 5 }

    chance_for_fumble = effective if policy.fumble_uses_base
                        else targets[difficulty]
    fumble_min = 96 if chance_for_fumble < 50 else 100

    net    = clamp(bonus - penalty, -policy.max_penalty, policy.max_bonus)
    roll, candidates = roll_d100(net)

    if   roll == 1:                                 level = CRITICAL
    elif roll >= fumble_min:                        level = FUMBLE
    elif targets.extreme >= 1 and roll <= targets.extreme: level = EXTREME
    elif targets.hard    >= 1 and roll <= targets.hard:    level = HARD
    elif roll <= targets.regular:                   level = REGULAR
    else:                                           level = FAILURE

    succeeded = level >= difficulty                 # CRITICAL satisfies everything
    return RollResult(roll, candidates, net, targets, fumble_min,
                      level, succeeded, pushable = ...)
```

## 12. Edge-case checklist for tests

- Double-zero reads 100, never 0; the result domain is `[1, 100]`.
- 01 is a Critical at rating 1, and even when the required difficulty is Extreme.
- 100 is a Fumble at every rating, including ratings past 100.
- 96–99: plain Failure at rating 50+, Fumble at rating 49 or lower.
- Ratings 49 and 50 straddle the fumble-threshold boundary; test both sides.
- Equal counts of bonus and penalty dice → plain single-tens-die roll.
- Bonus die, units = 0, one tens die showing 00: candidates include 100; min must not pick it.
- Penalty die, units = 0, one tens die showing 00: 100 must be selected.
- Rating 4 at Extreme difficulty: `extreme_target` is 0, so only 01 (Critical) passes.
- Regular success against Extreme difficulty: level REGULAR, `succeeded` false.
- Opposed roll where both sides fumble: level tie at FUMBLE, broken by rating comparison.
- Push attempted on an opposed, Luck, Sanity, or combat roll → rejected.
- Luck spend plus push on one roll → rejected.
- Luck spend that would land the result exactly on 01 → rejected (no purchased criticals).
- Group Luck roll with two characters tied for lowest Luck → needs a deterministic tiebreak.

---

## Backend implications

**Entities**

- `Rating` (skill or characteristic): `key`, `label`, `base_value`, `is_characteristic`,
  `pushable` (default true), `flags` (e.g. "Cthulhu Mythos", combat skill), plus improvement
  bookkeeping (`marked_for_improvement: bool`).
- `RollRequest`: `actor_id`, `targets: [(rating_ref, difficulty)]`, `combine_mode`
  (`none | all | any`), `bonus_dice`, `penalty_dice`, `flat_modifier`, `reason`, `scene_id`,
  `task_id`, `requested_by` (Keeper vs player), `blind` flag.
- `RollResult`: `id`, `request_id`, `units_die`, `tens_dice: [int]`, `net_dice_modifier`,
  `result` (1–100), `thresholds` snapshot `{regular, hard, extreme}`, `fumble_min`,
  `success_level` (enum), `succeeded` (vs required difficulty), `is_critical`, `is_fumble`,
  `timestamp`, `rng_seed` or audit token.
- `PushRecord`: `original_roll_id`, `pushed_roll_id`, `justification`,
  `foreshadowed_consequence`, `consequence_applied`. Enforce at most one push per
  `(scene_id, task_id)`.
- `LuckSpend`: `roll_id`, `points_spent`, `result_before`, `result_after`, `level_before`,
  `level_after`. Reject on ineligible roll kinds, on pushed rolls, and wherever it would
  fabricate a Critical.
- `OpposedContest`: `attacker_roll_id`, `defender_roll_id`, `winner`, `tie_broken_by`
  (`level | rating | rolloff | impasse`), plus the ratings snapshot the tie-break needs.
- `GroupLuckRoll`: `scene_id`, `participant_ids`, `chosen_actor_id` (lowest Luck), `roll_id`.

**Enums**

- `SuccessLevel` as an *ordered* type with a comparison operator. Explicit integers with a
  wide gap below failure (the fumble = -99 style) so no arithmetic mistakes fumble for
  "failure minus one."
- `Difficulty` as `REGULAR | HARD | EXTREME`, comparable against `SuccessLevel`.
- `RollKind` as `SKILL | CHARACTERISTIC | LUCK | SANITY | COMBAT | DAMAGE | OPPOSED |
  COMBINED` — pushability and Luck-spend eligibility are both functions of it.

**Policy / configuration surface** (the genuine variation points — make them a per-campaign
`RulesPolicy` document, not constants):

- `fumble_uses_base_rating` vs difficulty-adjusted (§4).
- `max_bonus_dice` / `max_penalty_dice` (default 2/2).
- `opposed_tie_default` (`rating` then `impasse` vs `rating` then `rolloff`).
- `allow_luck_spending`, `allow_luck_recovery`, `allow_push_after_fumble`.
- `cooperation_mode` (`difficulty_reduction` vs `bonus_die`).
- `ratings_can_exceed_100`.

**Architectural notes**

- RNG lives server-side, and every roll persists its component dice — Keepers can audit,
  and blind and open rolls share a single code path.
- Roll classification must be a **pure function** of `(result, thresholds, fumble_min,
  policy)`, decoupled from dice generation, so it can replay against stored results. That is
  precisely what lets Keeper overrides and Luck spends work as re-classification instead of
  re-rolling.
- Keeper override of a success level is a first-class operation in the wild (Foundry ships
  increase/decrease buttons, disabled at Critical and Fumble respectively). Model it as an
  audited mutation carrying `overridden_from`.
- Skill-improvement marking is a side effect of the roll pipeline with carve-outs: no mark
  when Luck bought the success; a successful push does earn one. Emit it as an event.
- Opposed and combined rolls are *compositions* of the same primitive. Don't fork the
  resolution code — build orchestrators that issue N `RollRequest`s (opposed) or one request
  with N targets (combined).
- Pushes form a state machine on a task, not on a roll: `PENDING → ROLLED → (SUCCEEDED |
  FAILED) → PUSHED → (PUSH_SUCCEEDED | PUSH_FAILED_CONSEQUENCE_PENDING → RESOLVED)`. The
  consequence step wants Keeper acknowledgement, so it should block rather than
  auto-resolve.

---

## Open questions

1. **Fumble threshold basis.** Base rating or difficulty-adjusted target? Foundry adjusts;
   many tables don't. The choice materially changes fumble frequency on Hard/Extreme checks
   and needs an explicit product decision (recommend: configurable, defaulting to
   difficulty-adjusted to match the most widely deployed implementation).
2. **Is 01 an automatic success?** The rules crown 01 a Critical but never say a Critical
   clears an Extreme bar when `extreme_target` is 0. Assume yes (Critical tops the ladder),
   but verify against the book before freezing it.
3. **Push scope key.** How does the backend recognize "the same task" so a retry is forced
   into push semantics? Keeper-declared task ids, or inference from `(scene, skill, target
   object)`? Fully automatic inference will generate false positives.
4. **Cooperation implementation.** Difficulty reduction, bonus die, or "subtract helper
   skills from difficulty" (as some summaries have it)? Mechanically distinct, and the
   sources conflict; needs a table-facing decision.
5. **Opposed rolls with 3+ parties.** The rules describe two-sided contests; Foundry's
   opposed card supports exactly one attacker and one defender. Does the tracker need N-way
   contests (chases, group grapples), and if so, what breaks ties?
6. **Combined rolls with mixed difficulties.** May each target of a combined roll carry its
   own difficulty, or must they share one? The rules don't clearly forbid mixing.
7. **Luck spend granularity and ordering.** After spending to reach Hard, can a player spend
   more to reach Extreme within the same transaction — and is a spend reversible before the
   Keeper confirms? What caps a single spend?
8. **Bonus dice from several sources.** Equipment, cooperation, and circumstance each grant
   one — do they sum before the clamp at 2, or does the clamp apply per source? Recommend
   sum-then-clamp, but it shapes effect-stacking design.
9. **Pulp Cthulhu deltas.** Pulp alters Luck spending and adds roll modifiers. If Pulp is in
   scope for v1, `RulesPolicy` must be designed for it now.
10. **Are combat rolls "opposed" or "difficulty-based"?** 7e melee is opposed (Fighting vs
    Fighting/Dodge) while firearms are usually a straight roll at a range/cover-set
    difficulty. The dice engine must serve both without special-casing combat.

---

## Sources

**Official / free**

- Chaosium, *The Call of Cthulhu RPG Wiki* — The Game System:
  https://cthulhuwiki.chaosium.com/rules/game-system.html
- Chaosium, *The Call of Cthulhu RPG Wiki* — Skill Rolls and Difficulty Levels:
  https://cthulhuwiki.chaosium.com/rules/skill-rolls-and-difficulty-levels.html
- Chaosium, *The Call of Cthulhu RPG Wiki* — Opposed Skill Rolls:
  https://cthulhuwiki.chaosium.com/rules/opposed-skill-rolls.html
- Chaosium, *Call of Cthulhu 7th Edition Quick-Start Rules* (free PDF):
  https://www.chaosium.com/cthulhu-quickstart/ and
  https://www.chaosium.com/content/FreePDFs/CoC/CHA23131%20Call%20of%20Cthulhu%207th%20Edition%20Quick-Start%20Rules.pdf
- Chaosium, *Basic Roleplaying ORC Content Document* (BRP-family comparison; note BRP's
  critical/special/fumble bands differ from CoC 7e's):
  https://www.chaosium.com/content/orclicense/BasicRoleplaying-ORC-Content-Document.pdf

**Reference implementation (source read directly)**

- Miskatonic Investigative Society, *CoC7-FoundryVTT*:
  https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT
  - `coc7/apps/dice-pool.js` — success-level enum, `#minimumFumbleFromThreshold()`,
    `#populateThresholdRanges()`, `maxDiceBonus`/`maxDicePenalty` (both 2), and the
    tens/units candidate-selection logic including the 00-with-units-0 case.
  - `coc7/apps/chat-opposed-message.js` — opposed resolution, tie detection, tie-breaker
    setting.
  - `coc7/apps/chat-combined-message.js` — `combinedall` / `combinedany` modes.
  - `coc7/apps/check.js` — Keeper increase/decrease success-level overrides.

**Secondary / community (cross-checking only; non-authoritative)**

- Skill Checks Q&A (carries page citations): https://morganhua.blogspot.com/2020/08/call-of-cthulhu-7th-ed-skill-checks-q.html
- Luck rules discussion: https://morganhua.blogspot.com/2022/08/call-of-cthulhu-7th-edition-luck.html
- TRPGLine CoC7 system rules summary: https://trpgline.com/rules/coc7/gameSystem
- mythos.wiki CoC 7e Cheat Sheet: https://mythos.wiki/index.php/Call_of_Cthulhu_7th_Edition_Cheat_Sheet
- Let's Study CoC 7e, Part 2a: https://philgamer.wordpress.com/2018/02/13/lets-study-call-of-cthulhu-7th-edition-part-2a-skill-rolls/
- Bonus, Penalty and Pushed Rolls: https://www.nonhumanuser.com/articles/bonus-penalty-and-pushed-rolls/
- Combined Skill Rolls in 7th ed.: https://shoggoth.net/gaming-news-2/combined-skill-roles-in-7th-ed-call-of-cthulhu/

**Commercial rulebook page pointers** (community-reported; check your printing — the
*Keeper Rulebook* and the older combined *7th Edition Rulebook* paginate differently)

- Difficulty levels — Keeper Rulebook ~p.83
- Pushing rolls — Keeper Rulebook ~p.84
- Cooperation / assisted rolls — Keeper Rulebook ~p.88
- Opposed rolls — Keeper Rulebook ~p.90
- Combined rolls — Keeper Rulebook ~p.92; combined 7e Rulebook ~p.55
- Spending Luck / Recovering Luck (optional rules) — Keeper Rulebook ~p.99
