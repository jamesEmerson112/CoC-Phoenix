# Luck Spending, Magic Points & Spellcasting (CoC 7e)

> Context doc for the CoC Phoenix tracker backend. All mechanics are paraphrased in our own
> words from the sources listed at the bottom. Page references point into the Chaosium books
> for human verification; nothing here reproduces rulebook text.

## Overview

Three separate resource economies ride on top of the core d100 engine, and a tracker backend
must own all three authoritatively — players can't be trusted to track them and Keepers
don't want to. **Luck** is a characteristic (rolled at creation, capped at 99) that doubles
as spendable meta-currency: under the optional spend rule, a player burns Luck one-for-one
to lower a die result after seeing it, turning failure into success or upgrading a success
tier — but only on skill/characteristic rolls, never on Luck rolls, Sanity rolls,
Sanity-loss amounts, or damage, and never to fabricate a critical or erase a fumble. **Magic
points** are a small regenerating pool derived as POW/5, refilling at roughly one point per
hour and spilling into hit points when overspent. **Spellcasting** consumes magic points
(plus, variously, permanent POW, Sanity, hit points, time, and components), demands a Hard
POW roll only on a spell's first-ever casting, and can be pushed with consequences worse
than failure. The engine-critical pieces: keep the *natural* die roll separate from the
*effective* (Luck-adjusted) roll, gate Luck spends behind a roll-eligibility predicate, make
cost payment atomic, and treat every improvement check (Luck recovery, POW growth) as an
explicit "roll above your own score" event.

---

## 1. Rule-source map: what is core, optional, or Pulp

Model these as three independently toggleable rule modules — groups mix them freely.

| Module | Status | Where |
|---|---|---|
| Luck as a characteristic, Luck rolls | Core, always on | Keeper Rulebook ch. 3 & 5 |
| **Spending Luck** (1:1 buy-down) | **Optional rule** in core 7e | KR ~p.99 |
| **Recovering Luck** (between-scenario check) | **Optional rule** in core 7e | KR ~p.99 |
| Pulp Luck spends (heal, cheat death, negate malfunction, halve SAN loss…) | Pulp Cthulhu only | Pulp Cthulhu ch. 3 |
| Pulp Luck recovery (per-session, larger dice) | Pulp Cthulhu only | Pulp Cthulhu ch. 3 |
| Magic points, spellcasting | Core | KR ch. 12 (Magic), summary ~p.419 |

Practical upshot: a `campaign.rules` config object with at least `luckSpendEnabled`,
`luckRecoveryEnabled`, `pulpMode` booleans — the *set of legal Luck spends* changes
completely between core and Pulp.

---

## 2. Luck the characteristic

- **Starting value:** 3D6 × 5 → range 15–90. Rolled like a characteristic; not point-bought
  under the standard method.
- **Age modifier:** teenaged investigators roll Luck twice and keep the higher — the only
  creation-time Luck re-roll.
- **Ceiling:** 99. Every path adding Luck must clamp there. Floor is 0 (you can spend
  yourself dry; nothing goes negative).
- **Not a skill:** Luck never earns experience/improvement checks the way skills do. It
  moves only via the Recovering Luck rule, Keeper awards, or spending.
- **Independent of POW.** In 6e and earlier, Luck was POW × 5; in 7e it stands alone. Never
  derive one from the other — a classic porting bug.

### Luck rolls

A Luck roll is a plain d100 against the Luck value on the normal difficulty ladder:

| Difficulty | Succeed on |
|---|---|
| Regular | roll ≤ Luck |
| Hard | roll ≤ ⌊Luck / 2⌋ |
| Extreme | roll ≤ ⌊Luck / 5⌋ |

- Critical = natural 01. Fumble = natural 100, or natural 96–100 when the *base* value
  rolled against is under 50.
- The Keeper calls one for things outside investigator control ("is there a payphone on
  this corner?", "does the tenement have a fire escape?"). Rule of thumb: if a real skill
  or characteristic applies, use it instead.
- **Group Luck rolls:** when the whole party's fortune rides on it, convention puts the
  roll in the hands of the member with the *lowest* Luck. The same lowest-Luck tiebreak
  reappears elsewhere — e.g. picking which bystander catches a stray bullet when firing
  into melee.
- Luck rolls **cannot be pushed** and **cannot be bought down with Luck points** (§3).

---

## 3. Spending Luck (optional rule)

### 3.1 Core mechanic

Once the die is rolled and its result known, the player may spend Luck to *reduce the number
rolled*, one point per point. The printed rule imposes no per-roll cap — spend anything you
have. The reduced value re-evaluates against the difficulty ladder, so a spend can:

- convert a failure into a Regular success,
- upgrade Regular → Hard, or Hard → Extreme (relevant to damage and opposed rolls).

Shape example: skill 60, difficulty Regular, natural 68 → spend 8 Luck → effective 60 →
Regular success. Spending 38 instead (effective 30 = ⌊60/2⌋) buys a Hard success.

### 3.2 What Luck may NOT be spent on

A hard blocklist, and it belongs in one predicate function:

1. **Luck rolls** themselves.
2. **Sanity rolls.**
3. **Rolls determining the amount of Sanity lost** (the 1D6 of a `0/1D6`, etc.).
4. **Damage rolls** of every kind (weapon damage, falls, poison…).
5. **Pushed rolls.** Push and Luck-spend are mutually exclusive branches for a task: pick
   one. A pushed result stands exactly as rolled.
6. **Another character's roll.** Luck only ever bends the spender's own die.
7. **Rolls made to determine improvement/experience** (see also §3.4).

Grey areas the rulebook never enumerates but a tracker must rule on:
characteristic-improvement checks, POW improvement checks, and the Luck-recovery check
itself are all "roll against your own score" checks — best treated as ineligible by default
(they are structurally Luck-roll-like).

### 3.3 Critical, fumble, malfunction — the natural roll is sticky

The single most important engine rule here: **success-tier extremes come from the natural
die, never the Luck-adjusted one.**

- Buying down to an effective 01 does **not** make a critical. The best a spend yields is
  an Extreme success. (It still beats any opposing success level in a contest, but for
  damage purposes it is Extreme, not critical.)
- A natural fumble (100, or 96–100 at values under 50) **cannot** be bought off with Luck
  in core 7e. Same for a firearm malfunction and a weapon fumble.
- The roll record therefore needs both `naturalRoll` and `effectiveRoll`, with the tier
  evaluator reading criticals/fumbles/malfunctions off `naturalRoll` and success level off
  `effectiveRoll`.

### 3.4 Experience-check suppression

When a Luck spend is what flipped a failure into a success, the investigator earns **no**
improvement check on that skill — only unaided successes tick the box. So the roll result
object wants a `grantsExperienceCheck` flag set false whenever
`luckSpent > 0 && naturalTier === Failure`. (Whether a spend that merely *upgrades* an
already-successful roll also suppresses the tick is unstated; most tables still award it.
Make it configurable.)

### 3.5 Combat and firearms interaction

- Luck **can** be spent on attack rolls, firearms included — an attack roll is an ordinary
  skill roll.
- Luck **cannot** touch the damage roll that follows.
- 7e impaling weapons (firearms, bows, spears and kin) deal extra damage on an Extreme
  success. Since Luck can legitimately buy an Extreme on the *attack* roll, a spend **can**
  indirectly produce an impale. That's allowed — the restriction covers the damage dice,
  not reaching the tier.
- Edge case worth encoding: at extreme range or under penalties where only an Extreme
  success hits at all, the "impale on Extreme" behaviour can shift so that only a genuine
  critical impales. Keep impale determination in one place keyed off `(tier,
  weaponIsImpaling, rangeBand)` rather than scattered.
- Luck may be spent on your side of an **opposed roll** (§9); it bends your die only.

### 3.6 Timing / state machine for a Luck spend

```
ROLL_MADE (natural roll revealed, tier computed)
  ├─ eligible(rollType) == false ──> RESOLVED
  └─ eligible == true
       ├─ player declares PUSH        ──> re-roll; Luck locked out for this task ──> RESOLVED
       ├─ player declares LUCK_SPEND(n)
       │     ├─ n > currentLuck  ──> reject
       │     ├─ effectiveRoll = naturalRoll - n
       │     ├─ deduct n from Luck (immediate, irreversible)
       │     └─ recompute tier ──> RESOLVED (naturalTier retained)
       └─ player declines           ──> RESOLVED
```

Spends are declared after the result is known and before consequences are narrated. Make the
window explicit in the API (a pending-decision state on the roll) rather than implicit —
otherwise you inherit the table's retroactive-spend arguments.

---

## 4. Recovering Luck

### 4.1 Core 7e (optional rule)

Between scenarios / during the Investigator Development Phase, each investigator makes a
Luck-recovery check:

- Roll 1D100.
- Result **above** current Luck → gain **1D10** Luck.
- Result **at or below** current Luck → nothing.
- Clamp at 99.

Note the inverted logic: it's the *low-Luck* investigators who tend to recover, pulling the
pool toward the middle — the same "roll above your value to improve" shape as skill
improvement. Some tables also apply the "96+ always improves" convention borrowed from skill
improvement; core text doesn't clearly demand it, so flag it.

Core 7e has no automatic per-session Luck refresh. Spent Luck is gone until a development
phase arrives — exactly why the spend rule is optional, and why one-shots often just pre-set
Luck to a fixed value.

### 4.2 Pulp Cthulhu

Pulp swaps in a far more generous per-session recovery: at **session start**, roll 1D100;
above current Luck grants roughly 2D10+10, at or below grants roughly 1D10+5. Still capped
at 99. In Pulp, spending Luck is standard, not optional.

Because the dice, the trigger (session start vs. development phase), and the
always-gain-something behaviour all differ, model recovery as a strategy object keyed by
rules module — not an if-branch inside one function.

---

## 5. Pulp Cthulhu Luck spends (optional module)

Enable only under `pulpMode`. They matter to the backend because several are *not* die-roll
modifications — they mutate HP, Sanity loss, and combat state directly.

| Spend | Approx. cost | Effect |
|---|---|---|
| Negate a weapon fumble / firearm malfunction | 10 Luck | The roll still fails; the mechanical consequence is cancelled |
| Halve Sanity lost after a failed Sanity roll | 2× the SAN points lost | Loss halved (define rounding) |
| Stay conscious | 1 Luck, doubling each subsequent round (1, 2, 4, 8, 16…) | Avoid falling unconscious |
| "Only a scratch" | 20 Luck | Recover a small HP amount at once (sources differ: 1D3+3 or 1D6), never above max HP |
| Cheat certain death | All remaining Luck, minimum 30 | Survive a lethal event, return with a small HP amount |

Two engine notes: the stay-conscious spend needs a per-scene escalating counter (it doubles
each round used), and the cheat-death spend zeroes Luck and needs an idempotency guard so it
can't double-apply to the same lethal event.

---

## 6. POW and POW rolls

- **POW** rolls 3D6 × 5 at creation (range 15–90); it seeds starting Sanity (SAN = POW at
  creation) and magic points (POW / 5).
- **POW rolls** ride the standard ladder: Regular ≤ POW, Hard ≤ ⌊POW/2⌋, Extreme ≤ ⌊POW/5⌋;
  critical 01, fumble 100 (or 96–100 when POW < 50).
- **POW does not improve through use** the way skills do — no default development-phase POW
  check. The two documented paths are both event-triggered improvement checks:
  1. **Winning an opposed POW contest** as part of successfully casting a spell that needs
     one.
  2. **Rolling 01 on a Luck roll** (the "how sorcerers get that way" path).
  Either way: roll 1D100; above current POW (some readings also allow an automatic gain on
  96+) adds 1D10 to POW permanently.
- **Downstream effects of a POW change** must recompute: max magic points (POW/5) moves
  with it. Current Sanity does **not** retroactively track POW — SAN is seeded from POW at
  creation and then lives its own life. Max Sanity is `99 − Cthulhu Mythos`, not any POW
  function.
- Some spells and Mythos entities drain POW permanently. A drain therefore lowers max MP
  and can strand current MP above the new cap (§7.4). What POW 0 means is not clearly
  specified in the core book — treat as campaign-defined (commonly "soulless husk /
  character is lost").

---

## 7. Magic points

### 7.1 Derivation and cap

- `maxMagicPoints = POW / 5`.
- At creation POW is always a multiple of 5, so the division is exact. It stops being exact
  once POW moves by 1D10 (POW 57 → 11.4). Pick a rounding rule and hold it — **floor**
  matches how 7e handles halves and fifths everywhere else. Document it; this is precisely
  the sort of thing that makes character-sheet apps disagree.
- Non-player entities (cultists, sorcerers, Mythos beings) often carry explicitly set MP
  pools rather than derived ones. The data model must allow an override instead of always
  computing from POW.

### 7.2 Regeneration

- Baseline: **1 MP per elapsed hour**.
- Superhuman POW scales it: 2/hour above POW 100, 3/hour above 200, and so on per extra 100
  POW. (One secondary source puts the third tier at 300; the licensed compendium says 200.
  Prefer 200 and expose it as a constant.)
- Regeneration **hard-caps at the POW/5 maximum** — no overfill.
- Practical consequence: a typical investigator (POW 50–65, MP 10–13) needs 10–13 in-world
  hours to refill from empty. 7e has no "full night's sleep restores all" shortcut — the
  clock is what counts. The backend therefore needs a **game clock**, not merely a session
  counter, to drive MP recovery. An `advanceGameTime(hours)` operation fanning out to MP
  regen (and HP healing, which follows different rules) is the clean design.

### 7.3 Spending

- Spending MP takes only the owner's will — no roll, no action cost.
- MP fund spellcasting, powering artifacts and gates, and assorted Mythos effects.
- **Overspend spills into hit points 1:1.** At 0 MP, any further required expenditure comes
  out of HP, point for point. That is real damage — narrated as nosebleeds, blistering,
  sores, sudden aging — and it can kill the caster. The engine must treat it as ordinary HP
  loss capable of crossing the major-wound / dying thresholds.
- Payment is **atomic**: if the caster cannot cover the full cost (MP + available HP), the
  spell does not activate and nothing is spent. No partial casting.
- **Pooling:** several ritual participants may each contribute MP (and HP) toward one cost.
  Common practice has contributions declared secretly and summed by the Keeper. The data
  model therefore wants a cast referencing a *set* of contributors with per-contributor
  MP/HP amounts — not a single caster field.

### 7.4 Excess magic points

A character can temporarily hold MP above the POW/5 cap (from rituals, artifacts, drained
victims). Those excess points:

- spend normally,
- do **not** regenerate once spent,
- dissipate after a night's sleep or at a fitting narrative juncture.

So current MP is not simply `min(current, max)` — you need a distinct `temporaryMagicPoints`
bucket with an expiry, plus a spend order (temporary-first is the usual reading; confirm).
Likewise a POW drain that lowers the cap must not silently delete MP already held; the
excess just stops regenerating.

---

## 8. Spellcasting

### 8.1 Prerequisites

- The caster must know the spell (learned from tomes or taught; learning takes study time
  and typically grants Cthulhu Mythos skill points).
- Learning a spell costs no Sanity by itself. Sanity is paid on **casting** (the earlier
  SAN hit came from reading the tome that held it).
- Casting normally demands speech (chants, rites) and freedom of movement/gesture. Gagged,
  bound, or silenced casters generally cannot cast.
- Material components may be required; some are consumed, some reusable.

### 8.2 Costs

Every spell definition should carry an explicit cost vector:

| Cost | Notes |
|---|---|
| Magic points | Fixed, variable ("X MP"), or scaling (per target / per unit of effect) |
| Hit points | Via MP overspend, or explicitly demanded by the spell |
| POW | Permanent expenditure for some spells; recompute max MP afterward |
| Sanity | Fixed automatic loss, or `X/Y` notation implying a Sanity roll |
| Time | Instantaneous → rounds → hours → weeks |
| Components | Free text / item references |

The Sanity notation matters: a bare value ("1D6 SAN") is an **automatic** loss with no
roll — casting is voluntary, so there's nothing to resist. A `0/1D6`-style notation invokes
the usual Sanity roll. Accumulated casting SAN loss can trigger temporary insanity (5+
points from one source in one go) and indefinite insanity (a fifth or more of current SAN in
one game session) like any other loss. Folkloric edge case: a fully insane 0-SAN cultist has
nothing left to lose and effectively casts SAN-free.

### 8.3 The casting roll

- A casting roll is required **only the first time a given caster casts a given newly
  learned spell**: a **Hard POW roll** (roll ≤ ⌊POW/2⌋).
- Success → the spell resolves per its description and is permanently "mastered" for that
  caster: no casting roll ever again.
- Failure → nothing happens, and the costs are still gone.
- NPCs and Mythos entities never make casting rolls.

The per-caster spell record therefore carries a `firstCastSucceeded: boolean`; the casting
roll is gated on it.

### 8.4 Pushing a casting roll

After a failed first casting, the caster may retry — immediately or much later — paying the
full cost again, and that retry is a **pushed roll**.

- Pushed roll succeeds → spell works normally, no extra penalty, spell becomes mastered.
- Pushed roll **fails** → the spell **still works**, but the Keeper inflicts a dire
  consequence. In this framing the casting roll measures harm to the caster, not whether
  the magic fires. Consequences come off a Keeper menu: extra MP and Sanity drain, physical
  deformity or aging, a summoned entity arriving uncontrolled, drawing Mythos attention.
  One secondary summary cites an additional 1D6-scale MP and SAN hit; keep the specifics
  Keeper-configurable rather than hardcoding a formula.
- The Keeper may foreshadow the consequence before the player commits.
- Per §3.2: **Luck cannot be spent on the pushed casting roll.**

### 8.5 Alternative to pushing

Instead of pushing, the caster can relearn the spell from scratch — roughly 2D6 weeks of
study plus a Hard INT roll — resetting to a fresh (unpushed) first-casting attempt. Model as
a long-duration downtime activity that clears the failed-first-cast state.

---

## 9. Opposed POW rolls

Spells targeting a mind or soul (domination, mind blast, soul transfer) resolve as caster
POW vs. target POW. Two readings of 7e opposed rolls circulate, and they give different
numbers — make the resolution strategy configurable:

**Reading A — both sides roll, compare tiers (the wiki/quick-start reading).**
Each side rolls against their own POW at Regular difficulty. Compare levels:
`Critical > Extreme > Hard > Regular > Failure/Fumble`. Higher tier wins. On a tier tie, the
higher POW value wins; if POW is also equal, both roll 1D100 and lower wins (some summaries
instead give ties to the defender — pick one and document it).

**Reading B — the opponent's value sets your difficulty (the common table reading).**
You roll against your own POW, at a difficulty derived from the opponent's POW:

| Opposing POW | You need |
|---|---|
| below 50 | Regular success |
| 50–89 | Hard success |
| 90+ | Extreme success |

Reading B shines when only one side is a player and the Keeper doesn't want to roll.

Either way:

- Luck may be spent on your own die in an opposed roll. An effective 01 bought with Luck
  beats any opposing success level but still counts as Extreme (not critical) for
  downstream damage.
- **Winning an opposed POW roll as part of casting triggers a POW improvement check**
  (§6) — the main way sorcerer NPCs and magic-using investigators grow POW, and easy to
  forget. A tracker that fires it automatically is genuinely valuable.
- A tie/impasse may simply mean nothing happens this round and the contest repeats next
  round.

---

## 10. Worked micro-examples

1. **Luck buy-down.** Firearms (Handgun) 55, natural 61, failure. Player spends 6 Luck →
   effective 55 → Regular success. No experience check for Handgun this session. Luck
   62 → 56.
2. **Luck to Extreme for an impale.** Same skill 55; Extreme threshold 11. Natural 30 —
   already a Hard success. Spending 19 more Luck reaches effective 11 = Extreme → impaling
   damage applies. The damage dice themselves stay untouchable.
3. **MP overspend.** POW 60 → max MP 12. Caster at 4 MP casts a 9-MP spell: 4 MP → 0, and
   5 HP burn. With only 3 HP available, the cast fails outright and nothing is spent.
4. **Recovery clock.** Same caster at 0 MP rests 7 hours → 7 MP. A full refill takes 12
   hours.
5. **POW growth.** Sorcerer POW 65 wins an opposed POW roll while casting. Improvement
   check: rolls 78 (> 65) → +1D10 POW → say +6 → POW 71; max MP recomputes from 13 to 14
   (floor rounding).

---

## 11. Backend implications

**Entities / fields**

- `Investigator`: `pow`, `luck` (0–99), `magicPointsCurrent`, `magicPointsMaxOverride?`,
  `temporaryMagicPoints[] {amount, expiresAt}`, `sanityCurrent`, `sanityMax (= 99 −
  cthulhuMythos)`, `hitPointsCurrent`.
  Derived: `magicPointsMax = override ?? floor(pow / 5)`;
  `magicPointRegenPerHour = 1 + floor(pow / 100)`.
- `Roll`: `naturalRoll`, `effectiveRoll`, `targetValue`, `difficulty`, `naturalTier`,
  `effectiveTier`, `isCritical` / `isFumble` (both derived from `naturalRoll` alone),
  `luckSpent`, `wasPushed`, `grantsExperienceCheck`, `rollKind` enum (`skill |
  characteristic | luck | sanity | sanityLoss | damage | improvement | casting | opposed`).
  The enum is what the Luck-eligibility predicate keys off.
- `LuckLedgerEntry`: `investigatorId`, `delta`, `reason` (`spend | recovery | keeperAward |
  pulpSpend:<type>`), `rollId?`, `timestamp`. An append-only ledger settles table arguments
  a bare integer never will.
- `Spell` (definition): `magicPointCost` (may be an expression), `powCost`, `sanityCost`
  (structured: automatic value vs. `X/Y` roll notation), `hitPointCost`, `castingTime`,
  `components`, `requiresOpposedPow`, `opposedCharacteristic`.
- `KnownSpell` (per caster): `spellId`, `firstCastSucceeded`, `pushedAndFailed`,
  `learnedAt`.
- `CastingAttempt`: `casterId`, `contributors[] {actorId, magicPoints, hitPoints}`,
  `spellId`, `costsPaid`, `castingRollId?`, `pushed`, `outcome` enum (`success |
  nothingHappens | successWithDireConsequence`), `consequenceNote`.
- `Campaign.rules`: `luckSpendEnabled`, `luckRecoveryEnabled`, `pulpMode`,
  `opposedRollStrategy` (`compareTiers | opponentSetsDifficulty`), `tieBreak`
  (`higherValue | defenderWins`), `mpRoundingMode`, `experienceCheckOnLuckUpgrade`.

**Behaviours the engine owns**

- `canSpendLuck(roll)` — one predicate, one place. False for
  luck/sanity/sanityLoss/damage/improvement rolls, pushed rolls, other players' rolls, and
  (configurably) casting rolls.
- Tier evaluation takes `(naturalRoll, effectiveRoll, targetValue, difficulty)` and never
  collapses them into one number.
- `advanceGameTime(hours)` → MP regeneration capped at max, temporary-MP expiry, plus
  whatever HP healing rules live elsewhere.
- `payCastingCost(attempt)` — atomic, all-or-nothing, MP first then HP spillover, pooled
  contributors summed before validation.
- `runDevelopmentPhase(investigator)` — includes the Luck-recovery check when enabled;
  clamp at 99.
- `powImprovementCheck(investigator, trigger)` — fired on a winning opposed POW cast and on
  a natural 01 Luck roll; recompute `magicPointsMax` after and emit a change event.
- Recompute-on-change: any POW mutation cascades to `magicPointsMax`; any Cthulhu Mythos
  mutation cascades to `sanityMax` (clamping current SAN down if it now overshoots).

**Auditability**

Every Luck spend, MP spend, HP spillover, and SAN loss should be an event carrying a cause
pointer. The entire value of a tracker in this game is answering "why do I only have 4
Luck?" without a five-minute argument.

---

## 12. Open questions

1. **MP rounding.** Floor vs. round-half-up for `POW / 5` once POW leaves multiples of 5.
   Floor matches 7e's Hard/Extreme handling, but confirm against a printed sheet.
2. **Regen scaling threshold.** 2 MP/hour above POW 100 is solid; the next tier is quoted
   as both 200 and 300 by different secondary sources. Verify in KR ch. 12 before
   hardcoding.
3. **Spend order for temporary (over-cap) MP.** Temporary first or last? The rules are
   silent.
4. **Does a Luck spend that merely upgrades an already-successful roll suppress the
   experience check?** Text only addresses failure→success. Table decision needed.
5. **Are improvement checks (skill, POW, Luck recovery) Luck-spendable?** Almost certainly
   not — but that's inference, not printed rule.
6. **Opposed roll resolution.** Which of the two readings (§9) is canonical for 7e, and
   which tie-break (higher value vs. defender wins)? Materially changes hit rates; settle
   before combat/opposed-cast code exists.
7. **Impale determination at long range.** Pin down the interaction between range band,
   required success level, and whether an impale needs a critical rather than an Extreme.
8. **Failed pushed casting roll.** Is there a printed consequence table (with dice), or
   pure Keeper fiat? Decides whether the backend rolls it or just prompts the Keeper.
9. **POW reaching 0.** No clear core rule found. Campaign-configurable, or a defined
   terminal state?
10. **Luck recovery cadence.** Core ties it to the development phase; groups vary (per
    scenario, per session, never). Needs a scheduling hook, not a fixed trigger.
11. **The "96+ always improves" convention.** Certain for skill improvement checks; unclear
    whether Luck recovery and POW improvement inherit it.
12. **Pulp "It's Only a Scratch" healing die** — sources split (1D3+3 vs 1D6). Verify in
    Pulp Cthulhu before enabling Pulp mode.

---

## Sources

**Official / licensed**

- Chaosium, *Call of Cthulhu Keeper Rulebook*, 7th ed. — Spending Luck & Recovering Luck
  (optional rules, ~p.99); Magic chapter incl. failing a pushed casting roll (~p.178) and
  how sorcerers gain POW (~p.179); Summary of Magic Rules and Increasing POW (~p.419); core
  mechanics summary (~p.407). Page numbers approximate; verify against the print/PDF
  edition.
- Chaosium, *Call of Cthulhu Investigator Handbook*, 7th ed. — characteristic generation,
  Luck, age modifiers.
- Chaosium, *Pulp Cthulhu* — Luck spends and Pulp Luck recovery (ch. 3).
- The Call of Cthulhu RPG Wiki (Chaosium): https://cthulhuwiki.chaosium.com/rules/magic.html
- The Call of Cthulhu RPG Wiki (Chaosium): https://cthulhuwiki.chaosium.com/rules/opposed-skill-rolls.html
- CoC 7e Quick-Start Rules (free official PDF):
  https://www.chaosium.com/content/FreePDFs/CoC/CHA23131%20Call%20of%20Cthulhu%207th%20Edition%20Quick-Start%20Rules.pdf
- Basic Roleplaying ORC Content Document (Chaosium):
  https://www.chaosium.com/content/orclicense/BasicRoleplaying-ORC-Content-Document.pdf —
  power/magic point concepts shared with the BRP engine.
- Roll20 CoC Compendium (licensed text): https://roll20.net/compendium/coc/Using%20Magic ,
  https://roll20.net/compendium/coc/Casting%20Spells ,
  https://roll20.net/compendium/coc/Using%20Luck
  (note: the Using Luck page mixes core and Pulp spends).

**Community / secondary (cross-checking only, not authority)**

- mythos.wiki CoC 7e cheat sheet: https://mythos.wiki/index.php/C1_Game_Mechanics
- TRPGLine CoC 7e system rules: https://trpgline.com/rules/coc7/summary
- 21st Century Philosopher — CoC 7e Luck: https://morganhua.blogspot.com/2022/08/call-of-cthulhu-7th-edition-luck.html
- 21st Century Philosopher — Magic Q&A: https://morganhua.blogspot.com/2020/06/call-of-cthulhu-7th-ed-magic-q.html
- 21st Century Philosopher — Skill Checks Q&A: https://morganhua.blogspot.com/2020/08/call-of-cthulhu-7th-ed-skill-checks-q.html
- Let's Study CoC 7e (philgamer): parts 2b (opposed rolls), 2c (firearms/damage), 5 (magic)
- Miskatonic Investigative Society, CoC7 system for Foundry VTT (unofficial reference
  implementation): https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT —
  notably its changelog records fixes preventing Luck spends on Luck and Sanity rolls, and
  MP recovery tied to a rest/hours dialog.
