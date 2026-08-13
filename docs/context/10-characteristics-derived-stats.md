# Investigator Characteristics & Derived Attributes (Call of Cthulhu 7e)

> Context doc for the CoC Phoenix tracker backend, aimed at engineers building the
> investigator data model and rules engine. Every mechanic here is paraphrased in our own
> words; page references exist so claims can be checked against the official books.

## Overview

A 7th Edition investigator is built on eight percentile **characteristics** (STR, CON, SIZ,
DEX, APP, INT, POW, EDU), plus **Luck** — rolled the same way but deliberately kept outside
the eight. Every other number on the sheet with mechanical weight — Hit Points, Magic Points,
Sanity, Damage Bonus, Build, Move Rate, and the "half"/"fifth" columns that drive Hard and
Extreme difficulty — falls out of those values through small deterministic formulas. Three
consequences matter for a backend: (a) derived values need recomputation any time an input
moves, and inputs really do move mid-campaign (POW drain, aging, lasting injury, Keeper
ruling); (b) a handful of "derived" numbers are merely *seeded* from a characteristic and then
live their own life as mutable state — Sanity being the canonical case; and (c) several
values are pools carrying a max and a current, with threshold-triggered state changes hanging
off them (major wound, dying, temporary/indefinite insanity). Confusing seeded state with
live-derived values is the classic modelling mistake in CoC software.

---

## 1. The eight characteristics

Every characteristic is a percentile score (a fresh human investigator usually lands between
15 and 90; NPCs and monsters may go past 100).

| Code | Name | Rolled as | Range | Mean | Primarily drives |
|------|------|-----------|-------|------|------------------|
| STR | Strength | 3D6 × 5 | 15–90 | 52.5 | Damage Bonus, Build, MOV, feats of strength |
| CON | Constitution | 3D6 × 5 | 15–90 | 52.5 | Hit Points, stamina/poison/dying rolls |
| SIZ | Size | (2D6+6) × 5 | 40–90 | 65 | Hit Points, Damage Bonus, Build, MOV |
| DEX | Dexterity | 3D6 × 5 | 15–90 | 52.5 | MOV, initiative order, starting Dodge (DEX/2) |
| APP | Appearance | 3D6 × 5 | 15–90 | 52.5 | Social rolls; nothing else is derived from it |
| INT | Intelligence | (2D6+6) × 5 | 40–90 | 65 | "Idea" rolls, personal interest points (INT × 2), the temporary-insanity check |
| POW | Power | 3D6 × 5 | 15–90 | 52.5 | Starting Sanity, Magic Points, magical resistance |
| EDU | Education | (2D6+6) × 5 | 40–90 | 65 | "Know" rolls, occupation skill points, starting Language (Own) |

Watch the asymmetry that regularly bites implementers: **SIZ, INT and EDU roll (2D6+6) × 5**
rather than 3D6 × 5, so their minimum is 40, not 15.

**Luck** rolls 3D6 × 5 in core 7e. Treat it as a *resource* rather than a characteristic: it
sits outside the eight, gets spent, and nothing else derives from it.

Reference: Investigator Handbook ch. 2 (Creating Investigators), Keeper Rulebook ch. 3.

### Alternative generation methods

Character creation in the tracker should cover at least these officially sanctioned options:

1. **Roll** — dice as above, either strictly in order or eight values assigned freely.
2. **Fixed array** — the player distributes 40, 50, 50, 50, 60, 60, 70, 80 across the eight
   characteristics however they like.
3. **Point buy** — a fixed pool (460 points is the commonly cited figure) split across the
   eight, subject to per-characteristic bounds (usually 15–90, with SIZ and INT advised to
   stay at 40+ to remain in their natural range). Investigator Handbook, optional rules
   (~p. 58).

Since all three converge on the same downstream steps, implement generation as a pluggable
strategy sitting in front of one shared "finalize characteristics" stage.

---

## 2. Half and fifth values

Every characteristic (and every skill) drags two derived numbers along with it:

- **Half** = `floor(value / 2)`
- **Fifth** = `floor(value / 5)`

Rounding always goes **down**: 75 → 37 / 15; 55 → 27 / 11; 40 → 20 / 8.

They exist because the core D100 roll-under mechanic recognises three success tiers measured
against the value:

| Tier | Succeeds on |
|------|-------------|
| Regular | roll ≤ value |
| Hard | roll ≤ half value |
| Extreme | roll ≤ fifth value |
| Critical | roll of 01 |
| Fumble | roll of 100; also 96–99 when the value is below 50 |

Implementation note: never persist half/fifth as ordinary columns that can fall out of sync.
Either compute them at read time or declare them as generated/computed columns, so
disagreement with the base value is impossible. Skills have the identical shape and deserve
the identical treatment.

---

## 3. Age and age modifiers

Age is picked at creation (the published table spans 15–89) and imposes a bundle of
adjustments *before* any derived value gets computed.

### The adjustment table (summarized)

| Age band | EDU improvement checks | Deduct from STR/CON/DEX (player-allocated) | APP reduction | Other |
|----------|------------------------|--------------------------------------------|---------------|-------|
| 15–19 | — | — | — | Deduct 5 total across **STR and SIZ**; EDU −5; roll Luck twice, keep the better |
| 20–39 | 1 | — | — | — |
| 40–49 | 2 | 5 | 5 | MOV −1 |
| 50–59 | 3 | 10 | 10 | MOV −2 |
| 60–69 | 4 | 20 | 15 | MOV −3 |
| 70–79 | 4 | 40 | 20 | MOV −4 |
| 80–89 | 4 | 80 | 25 | MOV −5 |

Two closed-form expressions reproduce the table (and agree with the reference implementation
in the Foundry VTT CoC7 system, `document-class.js` / `investigator-wizard.js`), taking
`d = floor(age / 10)` for age ≥ 40:

```
eduChecks     = min(4, d - 2)
physDeduction = 5 * 2^(d - 4)      # 5, 10, 20, 40, 80 for d = 4..8
appReduction  = (d - 3) * 5        # 5, 10, 15, 20, 25
movPenalty    = d - 3              # 1..5
```

Behavioural rules the backend has to enforce:

- The **STR/CON/DEX deduction is allocated by the player**, who decides the split across the
  three. Validation must require the allocation to sum to *exactly* the mandated total. The
  teen 5-point STR/SIZ split works the same way.
- The **APP reduction and the teen EDU reduction happen automatically** — no allocation.
- Bands are **cumulative by design, not stacked**: the table already folds earlier decades
  in, so a 55-year-old applies only the 50–59 row, never 40–49 plus 50–59.
- Aging **mid-campaign** (a multi-decade time skip, say) means applying the difference
  between the old and new bands. The books don't spell this out; treat it as a
  Keeper-adjudicated event.
- The MOV penalty lands *after* base MOV is computed from the already-age-adjusted
  STR/DEX/SIZ — not before.

### EDU improvement checks

Each check is its own roll:

```
roll = 1D100
if roll > currentEDU:
    currentEDU = min(99, currentEDU + 1D10)
```

Checks resolve **one after another**, and every success raises the bar for the next one, so
sequence and intermediate state are load-bearing — never batch them. EDU caps at 99. (The
identical "improvement check" mechanic reappears for between-scenario skill growth and the
optional Luck recovery rule, so extract it as a shared primitive.)

EDU feeds occupation skill points (typically EDU × 4, or blends like EDU × 2 + DEX × 2) and
starting Language (Own) = EDU, which means **age adjustments must be completely settled
before any skill points are handed out**. That ordering constraint in the creation pipeline
is hard, not advisory.

---

## 4. Derived attributes

### 4.1 Hit Points

```
hpMax = floor((CON + SIZ) / 10)
```

- Typical investigator spread: roughly 6–18.
- `hp.current` is free-standing mutable state, clamped to `[<negatives untracked>, hpMax]`.
- Recompute `hpMax` on any CON or SIZ change. Pick a policy for when `hpMax` sinks below
  `hp.current` (recommendation: clamp down; don't rescale proportionally).
- **Pulp Cthulhu** doubles HP by using divisor 5 in place of 10.

Thresholds (details in §6): **major wound** when one blow deals ≥ `floor(hpMax / 2)`;
**instant death** when one blow deals ≥ `hpMax`; **0 HP** means unconscious, and dying only
if a major wound is present too.

### 4.2 Magic Points

```
mpMax = floor(POW / 5)
```

- `mp.current` is free-standing mutable state.
- Regeneration runs at about 1 MP per hour of ordinary rest (POW above 100/200 regenerates
  faster — relevant to entity stat blocks, not investigators).
- Some castings permit overspending: each MP short of the cost is paid with 1 HP. An MP
  spend can therefore cascade into HP loss and even a major wound — model the spend as a
  transaction that may return an HP-damage side effect.
- Recompute `mpMax` when POW changes. POW drain from spells and entities genuinely happens
  in play.

### 4.3 Sanity

```
sanCurrent_initial = POW        # at creation only
sanMax             = 99 - CthulhuMythosSkill
sanStarting        = POW        # retain this; several rules refer back to it
```

This is the value that absolutely must **not** be a computed field. Sanity is seeded from POW
and diverges from it immediately. An ORM that re-derives SAN from POW on every save will
quietly restore the whole party's minds. Persist `san.current` and `san.starting`; derive
only `san.max`.

- `san.max` shrinks as Cthulhu Mythos grows — learning Mythos lore permanently lowers the
  ceiling. Whenever Mythos rises, clamp `san.current` down to the new `san.max`.
- Sanity loss uses `X/Y` notation: X applies on a passed Sanity roll, Y on a failed one
  (e.g. `1/1D6`).
- Insanity with a Mythos cause grants Cthulhu Mythos skill: 5 points the first time, 1 point
  each time after — and that feeds straight back into `san.max`. A genuine feedback loop the
  engine must run in order: award Mythos → recompute sanMax → clamp.
- Loss from any one category of horror is capped: investigators become inured, so the
  tracker needs a per-investigator ledger of `{sourceType, totalLost, immune}`. Foundry's
  CoC7 system models precisely this (`sanityLossEvents`) — good evidence the need is real.
- Recovery: successful psychotherapy or self-help yields small amounts (order of 1D3), and
  scenario resolutions award Sanity. Every gain clamps to `san.max`.

### 4.4 Luck

- Rolled 3D6 × 5 (core). Teenagers roll twice and keep the higher result.
- Spent point-for-point to lift a failed skill or characteristic roll into a success. Not
  spendable on Sanity rolls, Luck rolls, or damage, and it can never buy a critical.
- No automatic refresh. The optional recovery rule (Keeper Rulebook ~p. 99) runs an
  improvement check at session end: roll 1D100; above current Luck grants 1D10; capped at 99.
- Model as `luck.current` (spendable) with no ceiling beyond the 99 cap, plus a per-session
  spend log so the Keeper can see where it all went.
- **Pulp Cthulhu** rewrites this materially: Luck rolls (2D6+6) × 5, recovery is far more
  generous, and new spends appear (buying back HP, cheating death). Keep the Luck rules
  behind a swappable policy object.

### 4.5 Damage Bonus and Build

Both derive from a single input, `sum = STR + SIZ`:

```
if sum <  65:  DB = -2,     Build = -2
if sum <  85:  DB = -1,     Build = -1
if sum < 125:  DB =  0,     Build =  0
if sum < 165:  DB = "1D4",  Build =  1
else:
    n = floor((sum - 45) / 80)        # n >= 1
    DB = f"{n}D6"                     # 1D6, 2D6, 3D6, ...
    Build = n + 1                     # 2, 3, 4, ...
```

First few bands for illustration (reworded; not the full published table):

| STR+SIZ | Damage Bonus | Build |
|---------|--------------|-------|
| up to 64 | −2 | −2 |
| 65–84 | −1 | −1 |
| 85–124 | none | 0 |
| 125–164 | +1D4 | 1 |
| 165–204 | +1D6 | 2 |

Edge cases code must respect:

- **Damage Bonus is not always a dice expression.** The bottom two bands are flat integer
  *penalties* (−2, −1), the middle band contributes nothing, and only the top bands roll
  dice. Represent DB as a small union type (`{kind: "flat", value: -2}` versus
  `{kind: "dice", expr: "2D6"}`) rather than a string parsed when the dice hit the table.
- DB applies to melee and thrown attacks — **never firearms**.
- Floor total damage at 0; a −2 DB on a feeble unarmed strike must not heal anyone.
- Build drives grapples/maneuvers plus chase and shoving contests; a big enough Build gap
  makes maneuvers penalized or outright impossible. The combat context doc owns the exact
  differential rules.
- Recompute both whenever STR or SIZ moves.

### 4.6 Move Rate (MOV)

Base MOV compares DEX and STR against SIZ:

```
if DEX > SIZ and STR > SIZ:   base = 9
elif DEX >= SIZ or STR >= SIZ: base = 8
else:                          base = 7
```

Then the age penalty applies:

```
mov = base - movPenalty(age)      # 0 below 40; 1..5 across the 40s..80s bands
mov = max(0, mov)
```

Equivalently, for age ≥ 40: `mov = base - floor(age / 10) + 3`.

Notes:

- SIZ bottoms out at 40 while STR and DEX bottom out at 15, so MOV 7 is common and MOV 9
  demands that both STR and DEX strictly exceed SIZ.
- Some quick references collapse this to "humans move 8" — that's a stat-block shorthand for
  NPCs, not the investigator rule. Implement the full comparison.
- MOV feeds the chase mechanics (movement actions per round) and can take situational
  modifiers from encumbrance, injury, or terrain. Keep `mov.base` (derived) distinct from
  `mov.effective` (base plus live modifiers).
- Recompute when STR, DEX, SIZ, or age changes.

---

## 5. Dependency graph and recomputation

| Derived value | Inputs | Recompute trigger |
|---------------|--------|-------------------|
| half / fifth of X | X | any change to X |
| hpMax | CON, SIZ | CON or SIZ change; ruleset toggle (Pulp) |
| mpMax | POW | POW change |
| san.max | Cthulhu Mythos skill | Mythos skill change |
| san.current | — (seeded once from POW) | never recomputed; mutated by events, clamped to san.max |
| Damage Bonus, Build | STR, SIZ | STR or SIZ change |
| mov.base | STR, DEX, SIZ, age | any of those change |
| Dodge (starting) | DEX | creation only — an ordinary skill thereafter |
| Language (Own) (starting) | EDU | creation only |
| Occupation skill points | EDU (+ one other, per occupation) | creation only |
| Personal skill points | INT | creation only |

The final four rows are **seed-only**: computed once at creation, independent forever after.
Raising DEX later never retroactively moves Dodge. Make the "seed-only" versus
"live-derived" distinction explicit in the model — slapping `@computed` on everything is
wrong.

Recommended engine shape: one pure function `deriveAttributes(characteristics, age, ruleset)
-> DerivedAttributes` with zero I/O, invoked on every characteristic mutation, followed by a
separate reconciliation pass that clamps the pools (`hp.current`, `mp.current`,
`san.current`) into their fresh bounds and emits events for any threshold crossed.

---

## 6. Thresholds and state transitions

### Hit Points

| Condition | State |
|-----------|-------|
| single blow damage ≥ `floor(hpMax / 2)` | **Major wound**; CON roll or drop unconscious |
| single blow damage ≥ `hpMax` | **Dead**, on the spot |
| hp.current ≤ 0, no major wound | Unconscious, but not dying |
| hp.current ≤ 0, with major wound | **Dying**: CON roll at each round's end; failure is death |
| stabilized via First Aid | 1 temporary HP; afterwards an hourly CON roll against deterioration |
| recovery, no major wound | ~1 HP per game day |
| recovery, major wound | ~1D3 HP per week (more on an Extreme Medicine result); the flag clears once HP returns to at least half of max |

`majorWound` has to be a persisted boolean, never inferred from the HP total — its trigger is
the size of a *single* damage instance, information the resulting HP value doesn't retain.

### Sanity

| Condition | State |
|-----------|-------|
| loss ≥ 5 from one Sanity roll | INT roll; **passing** means the investigator fully comprehends the horror and goes **temporarily insane** (counter-intuitively, success is the bad result) |
| temporary insanity | bout of madness lasting 1D10 rounds (real time) or 1D10 hours (summary); the temporary insanity itself persists ~1D10 hours; any phobia/mania acquired is permanent |
| loss ≥ one fifth of Sanity inside a single game day | **Indefinite insanity**; typically 1D6 months of in-game care before recovery attempts |
| san.current = 0 | **Permanently insane**; the investigator exits play |
| any prior insanity | `underlyingInsanity` flag — episodes can resurface |

This demands a **daily/session Sanity-loss accumulator** that resets on the right boundary,
plus the per-source loss ledger from §4.3.

### Magic Points

Reaching `mp.current = 0` is not itself a state transition, but a spend beyond the pool
converts the shortfall into HP damage — which can cascade into a major wound or death.

---

## 7. Ruleset variants worth designing for

- **Pulp Cthulhu**: HP divisor 5; Luck at (2D6+6) × 5 with richer recovery and additional
  spends; age penalties commonly waived; archetypes may roll a favoured characteristic on a
  boosted die.
- **Common house rules**: reroll/swap the lowest characteristic, skip age penalties, refresh
  Luck between scenarios, cap or uncap EDU.

Encode all of this as a named ruleset object carrying flags and formula overrides
(`hpDivisor`, `luckDice`, `applyAgePenalties`, `sanMaxBase`) threaded through
`deriveAttributes`. Foundry's CoC7 system does exactly this with per-world settings like
`pulpRuleDoubleMaxHealth` and `pulpRuleIgnoreAgePenalties` — a useful precedent.

---

## 8. Worked example (sanity check for tests)

Rolled: STR 60, CON 55, SIZ 65, DEX 70, APP 50, INT 75, POW 60, EDU 70, Luck 45. Age 45.

1. Age band 40–49: 2 EDU improvement checks; 5 points deducted across STR/CON/DEX; APP −5;
   MOV −1.
2. Player drops the whole 5-point deduction on STR → STR 55. APP → 45.
3. EDU check 1: roll 82 > 70 → +1D10 = 6 → EDU 76. Check 2: roll 40 ≤ 76 → no change. EDU 76.
4. Derived: hpMax = floor((55 + 65) / 10) = 12. mpMax = floor(60 / 5) = 12.
   SAN starts at 60, sanMax = 99 − 0 = 99.
5. STR + SIZ = 55 + 65 = 120 → band 85–124 → no DB, Build 0.
6. MOV: DEX 70 > SIZ 65 but STR 55 < SIZ 65 → base 8; age penalty 1 → MOV 7.
7. Half/fifth of DEX 70 → 35 / 14. Major wound threshold = floor(12 / 2) = 6.

---

## Backend implications

**Entities / fields**

- `Investigator`
  - `characteristics`: `{str, con, siz, dex, app, int, pow, edu}` — integers, the source of
    truth. Consider `{base, modifiers[], effective}` per characteristic so temporary drains
    (spell effects, injuries) stay reversible and auditable.
  - `age`, `ageBand` (derived), `ageAdjustments` (the recorded allocation — needed to replay
    or undo creation).
  - `luck`: `{current}`, cap 99.
  - `hp`: `{current, max (derived), majorWound: bool, dying: bool, stabilized: bool}`.
  - `mp`: `{current, max (derived)}`.
  - `san`: `{current, starting, max (derived from Mythos), dailyLossAccumulator,
    temporaryInsanity: {active, until}, indefiniteInsanity: {active, until},
    underlyingInsanity: bool, phobias[], manias[]}`.
  - `sanityLossLedger`: `[{sourceType, totalLost, immune}]`.
  - `derived` (never authoritative in storage): `damageBonus`, `build`, `movBase`,
    `movEffective`, plus half/fifth per characteristic.
- `Ruleset` — flags and formula overrides (core vs Pulp vs house rules), referenced by
  campaign.
- `AuditEvent` — every characteristic/HP/MP/SAN/Luck delta with cause, actor, timestamp. A
  Keeper-facing tracker stands or falls on answering "why is my Sanity 12?".

**Behaviour**

- A single pure `deriveAttributes(characteristics, age, ruleset)`; run it on every write.
- A `reconcilePools()` pass after derivation that clamps currents into new maxes and emits
  threshold-crossing events.
- Creation is a **pipeline with hard ordering**: generate → apply age adjustments (including
  the sequential EDU checks) → derive attributes → allocate skill points → finalize. Enforce
  the order in code, not by convention.
- Damage Bonus is a tagged union, never a string.
- Improvement checks (EDU, skills, optional Luck) share one primitive: roll D100, compare
  against current, add a die on success, clamp at 99.
- Offer a "recompute preview" so the Keeper can see what a proposed POW drain does to MP
  before committing it.

---

## Open questions

1. **Indefinite insanity threshold basis.** Is the one-fifth measured against Sanity at the
   start of the game day, against starting Sanity (POW), or against current Sanity at the
   moment of loss? Foundry precomputes a daily limit of `floor(POW / 5)` at creation,
   implying "one fifth of starting Sanity". Needs a ruling plus a config flag.
2. **Session vs game-day boundary.** The Sanity accumulator resets per *game day*, which
   doesn't map cleanly to a session. Does the Keeper advance an in-fiction clock explicitly?
3. **Characteristic floors.** An investigator in their 80s deducts 80 points across
   STR/CON/DEX. What's the legal minimum for a characteristic (1? 0? is 0 incapacitated or
   dead)? The books lean on common sense; a validator needs an actual number.
4. **Aging in play.** When a scenario jumps 15 years, do we apply the band delta, and do EDU
   improvement checks fire again? Currently a Keeper call.
5. **Should temporary characteristic drains propagate to derived values?** A POW-draining
   spell clearly should lower mpMax — should it touch Sanity? (The rules say no; house rules
   vary.)
6. **Clamping policy when maxes fall.** If SIZ drops and hpMax lands below current HP: clamp
   or leave? We recommend clamping, but confirm against the Keeper-facing UX.
7. **Build differential rules for maneuvers/chases.** Exact penalty and impossibility
   thresholds belong in the combat context doc, pinned down rather than guessed here.
8. **Age above 89.** The published table ends there. Extrapolate the closed forms, or demand
   Keeper-entered manual modifiers?
9. **Pulp Cthulhu scope.** Is Pulp a v1 target, or do we only need the extension seams?

---

## Sources

- Chaosium official Call of Cthulhu RPG Wiki — Investigator Characteristics:
  https://cthulhuwiki.chaosium.com/investigators/step-one-investigator-characteristics.html
- Chaosium official wiki — Secondary Attributes (HP, SAN, MP, Luck, DB/Build, MOV):
  https://cthulhuwiki.chaosium.com/investigators/step-two-secondary-attributes.html
- Chaosium official wiki — Sanity: https://cthulhuwiki.chaosium.com/rules/sanity.html
- Chaosium official wiki — Hit Points, Wounds and Healing:
  https://cthulhuwiki.chaosium.com/rules/hit-points-wounds-and-healing.html
- Chaosium official wiki — Magic / Magic Points: https://cthulhuwiki.chaosium.com/rules/magic.html
- Call of Cthulhu 7th Edition Quick-Start Rules (free, Chaosium):
  https://www.chaosium.com/cthulhu-quickstart/ (archive copy:
  https://archive.org/details/cha-23131-call-of-cthulhu-7th-edition-quick-start-rules)
- Foundry VTT CoC7 system (open-source reference implementation) —
  `coc7/models/actor/document-class.js` (`dbFromCharacteristics`, `buildFromCharacteristics`,
  `hpFromCharacteristics`, `mpFromCharacteristics`, `movFromCharacteristics`) and
  `coc7/apps/investigator-wizard.js` (`getAgeAdjustments`):
  https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT
- Roll20 CoC compendium — Wounds and Healing:
  https://roll20.net/compendium/coc/Wounds%20and%20Healing
- BRP Central (Chaosium forums) — age bonuses and penalties discussion:
  https://basicroleplaying.org/topic/9511-age-bonus-penalties/
- Book references for verification: *Call of Cthulhu Keeper Rulebook (7e)* ch. 3 (Creating
  Investigators), ch. 4 (Skills), ch. 6 (Combat), ch. 8 (Sanity), Luck recovery ~p. 99;
  *Call of Cthulhu Investigator Handbook (7e)* ch. 2, Damage Bonus/Build table ~p. 48,
  optional generation methods ~p. 58, age table ~p. 31.
- Variant: *Pulp Cthulhu* (Chaosium) — doubled HP, (2D6+6) × 5 Luck, expanded Luck spends.
