# Damage, Wounds, Healing & Death (CoC 7e)

> Context document for the CoC Phoenix tracker backend. All mechanics are paraphrased in our
> own words; page numbers point into the *Call of Cthulhu 7th Edition Keeper Rulebook* (KRB)
> for lookup. Sources at the bottom.

## Overview

7e models physical harm as one pool (hit points) plus a handful of binary condition flags —
and nearly all the interesting behaviour lives in the *thresholds*, not the pool. A single
blow is measured against two derived numbers — half of maximum HP (major wound) and full
maximum HP (instant death) — and that comparison, combined with whether current HP has
reached zero, selects which state the character lands in: fine, unconscious, dying, or dead.
From there the system runs three clocks simultaneously: a per-round death countdown while
dying, a per-hour stabilization check after First Aid, and a per-day or per-week recovery
cadence during convalescence. For a tracker backend, the right mental model is therefore not
"subtract HP" but a small state machine with timers bound to distinct time granularities,
plus a per-injury ledger recording which treatments have already been used up.

---

## 1. Core quantities

| Quantity | Derivation | Notes |
|---|---|---|
| Maximum HP | `floor((SIZ + CON) / 10)` | KRB p. 33. Typical adult range ~10–14. |
| Current HP | `0 <= currentHp <= maxHp` | Never stored negative — clamp at zero (KRB p. 119). |
| Major wound threshold | `maxHp / 2` | Measured against damage from **one** blow. |
| Instant death threshold | `maxHp` | One blow meeting or exceeding this kills outright. |
| Armour | Flat subtraction from incoming physical damage | KRB p. 108, 112. |
| Damage bonus (DB) | From the STR+SIZ band | Added to melee/unarmed; halved for thrown; never added to firearms (KRB p. 33, 103). |

Both thresholds derive from **maximum** HP, never current HP. A character sitting at 2/12 HP
still needs a 6-point blow to take a major wound.

### Damage application pipeline

Order of operations matters, because thresholds are checked against damage *actually taken*:

1. Roll weapon damage (+ DB where it applies).
2. Apply extreme-success/impale rules when the attack was an Extreme success (KRB
   p. 103–104): blunt/non-impaling → maximum weapon damage + maximum DB; impaling → maximum
   weapon damage + DB + one extra roll of the weapon's damage dice.
3. Subtract armour → `appliedDamage = max(0, rolled - armour)`. Armour does **not** reduce
   poison, drowning/asphyxiation, or magical damage.
4. Compare `appliedDamage` against the thresholds (major wound / instant death).
5. Subtract `appliedDamage` from current HP; clamp at 0.
6. Evaluate state transitions.

---

## 2. Thresholds and immediate outcomes

**Regular damage** — `appliedDamage < maxHp / 2`. No special effect by itself. Regular
damage can never kill on its own; the worst it does is knock a character out at 0 HP (KRB
p. 119–120).

**Major wound** — `appliedDamage >= maxHp / 2` from a single blow (KRB p. 119). Three
immediate effects:
- The *major wound* flag is set (persistent flag, not a per-hit event).
- The character drops **prone**, immediately.
- The character makes a Regular CON roll; failing it means falling **unconscious**.

**Instant death** — `appliedDamage >= maxHp` from a single blow. Death on the spot — no CON
roll, no dying state (KRB p. 119, 123; the official Quick-Start says "greater than or equal
to," so a hit exactly at max is lethal).

**Zero hit points** — when current HP reaches 0:
- *Without* the major-wound flag: unconscious but stable. Natural recovery proceeds, and
  First Aid can rouse them.
- *With* the flag: **dying** (below), and unconscious.

Further damage to a character already at 0 HP is normally ignored in play unless someone is
deliberately finishing them off; the tracker should decide between "ignored" and a
Keeper-confirmed coup de grâce.

---

## 3. The condition state machine

The conditions that matter for damage/death (the Foundry VTT CoC7 system carries essentially
the same set: *dying, dead, major wound, prone, unconscious*):

```
                       damage >= maxHp (one blow)
   HEALTHY ───────────────────────────────────────────────► DEAD
      │                                                       ▲
      │ damage >= maxHp/2 (one blow)                          │ failed dying CON roll
      ▼                                                       │
   MAJOR WOUND (flag) ──► prone + CON roll                     │
      │        └─ fail ──► UNCONSCIOUS                        │
      │                                                       │
      │ currentHp == 0 while major wound flag set             │
      ▼                                                       │
   DYING (unconscious) ──► CON roll at end of next round      │
      │                    and every round thereafter ────────┘
      │
      │ successful First Aid
      ▼
   STABILIZED (unconscious, 0 HP + 1 temporary HP)
      │  └─ CON roll at end of each hour; on failure → back to DYING (lose temp HP)
      │
      │ successful Medicine while stabilized
      ▼
   UNCONSCIOUS / CONVALESCING (major wound flag still set) ──► weekly recovery rolls
```

The point implementers must internalize: **major wound is an independent, persistent flag**,
orthogonal to current HP. It decides whether 0 HP means "unconscious" or "dying," and it
selects the healing rate. It clears only under the specific conditions in §6.

---

## 4. Dying: the per-round countdown

- Entry condition: `currentHp == 0 && majorWound == true`.
- The character is unconscious and cannot act.
- One round of grace: the first CON roll happens at the **end of the following round**, then
  at the end of every round after (KRB p. 120–121).
- Each roll is a Regular CON roll. Failure means death, immediately.
- Only **First Aid** interrupts the countdown. Medicine cannot stabilize someone still
  actively dying — it applies *after* stabilization.

This is the tightest timer in the whole system and where a tracker most obviously earns its
keep: the backend must know the current combat round, whose turn it is, and when "end of
round" fires — and it must prompt the right CON roll without relying on the Keeper's memory.

---

## 5. First Aid and Medicine

### First Aid (base 30%; KRB p. 65, 88, 120)

- Restores **1 HP** if delivered within **one hour** of the injury; outside the window it
  grants no HP.
- Rouses an unconscious (but not dying) character.
- One attempt per injury. Further attempts are **pushed rolls**, with the usual severe
  consequence on a failed push — on a dying patient, a failed push can reasonably be ruled
  fatal.
- Two characters may work the same patient together: either one succeeding counts.
- On a **dying** patient, success **stabilizes**: the per-round countdown halts and the
  patient gains 1 *temporary* HP. The Quick-Start describes it as "no immediate HP gain,
  but extends life so Medicine can be used"; the full KRB adds the temporary-HP
  bookkeeping. Same transition either way.
- A stabilized patient rolls a Regular CON roll at the end of that hour and hourly
  thereafter. On a failure the temporary HP is lost and they revert to **dying** (per-round
  rolls again). Successful Medicine ends the hourly checks.

### Medicine (base 1%; KRB p. 69, 120)

- Needs at least **one hour** of work plus proper equipment and supplies.
- Restores **1D3 HP** (minimum 1) to an injured patient.
- Same-day treatment is a Regular roll; an older injury is treated at **Hard**.
- On a **stabilized dying** patient: grants no HP itself, but permanently removes the dying
  condition (no more hourly checks) and moves the patient onto the weekly recovery track
  (KRB p. 120).
- One successful Medicine per injury, mirroring First Aid.

### Treatment budgeting

A patient benefits from one successful First Aid **and** one successful Medicine per
instance of damage; both may target the same wound. Fresh damage resets the budget. This is
the fiddliest bookkeeping rule in the chapter and the strongest argument for a per-injury
treatment ledger in the data model instead of per-character booleans.

---

## 6. Natural healing

### Without a major wound (KRB p. 121)

- **1 HP per day**, automatic, no roll.
- Applies even to someone unconscious at 0 HP from regular damage — they tick upward and
  wake once above zero (or sooner via First Aid).

### With a major wound (KRB p. 121–122)

- No daily healing. Instead, a **CON roll at the end of each week**:
  - Failure → 0 HP recovered that week.
  - Regular or Hard success → **1D3 HP**.
  - Extreme success → **2D3 HP**.
  - Fumble → a lasting injury, adjudicated by the Keeper (permanent disability, scar,
    characteristic loss, etc.).
- Modifiers on the weekly roll:
  - Bonus die: full rest in a comfortable environment (hospital, good home care).
  - Bonus die: effective medical care — a caregiver passing a weekly Medicine roll.
  - Penalty die: poor conditions, insufficient rest, or a caregiver's fumble.
- The **major-wound flag clears** when *either*:
  1. a weekly recovery roll comes up an Extreme success, **or**
  2. current HP has climbed back to **at least half of maximum**.
- Once cleared, the character resumes 1 HP/day until whole.

*Illustrative trace (reworded from the Quick-Start's worked example).* A 12-HP investigator
is worn down to 2 HP by regular damage in one evening; no major wound, so he heals 1/day.
Three days later, at 5 HP, a fall deals 7 damage — ≥ 6, so a major wound. He's now on the
weekly track. Week one: CON roll passes, 1D3 → 2 HP. Week two: Extreme success, 2D3 → 3 HP,
reaching 5 HP; the Extreme success clears the flag and he returns to 1 HP/day.

---

## 7. Other forms of damage (KRB p. 123–124, Table III p. 124)

The rulebook supplies a severity ladder mapping real-world hazards onto dice, roughly:

| Band | Dice | Flavour of source |
|---|---|---|
| Minor | 1D3 | Punch, kick, a thrown rock, a 10-ft fall onto something soft |
| Moderate | 1D6 | Club, small-calibre bullet, arrow, torch fire, 10-ft fall onto grass |
| Severe | 1D10 | Heavier bullets, axe, flamethrower, 10-ft fall onto concrete |
| Deadly | 2D10 | Car at 30 mph, close blast, strong poison |
| Terminal / Splat | 4D10 / 8D10 | Speeding car, point-blank explosive, train, head-on collision |

(Abridged — the published table has more rows and far more examples.)

Special cases that must bypass the normal pipeline:

- **Asphyxiation / drowning** (KRB p. 124): a CON roll every round; after the first failure,
  damage lands every round until the victim can breathe. Death occurs at 0 HP — the
  major-wound requirement is **skipped** entirely. Armour is useless.
- **Poison** (KRB p. 124, 128; sample poisons p. 129): an Extreme CON roll halves the
  damage. Armour does not apply.
- **Falling**: damage scales per 10 feet; the landing surface picks the band.
- **Fire, acid, vacuum, explosions**: read from the same severity table.

---

## 8. Transient states a tracker must carry

Beyond the wound conditions proper, these short-lived states affect resolution and are easy
for a Keeper to drop:

- **Prone** — set automatically by a major wound; also the result of a "knock down"
  manoeuvre. Standing costs action. (An optional prone-modifier rule at KRB p. 128 covers
  melee and firearms.)
- **Unconscious** — blocks all action; cleared by successful First Aid, HP rising above 0,
  or Keeper fiat.
- **Dying** — carries a round counter and a "next check at end of round N" pointer.
- **Stabilized** — carries an hour counter and a "next check at hour N" pointer, plus 1
  temporary HP that must be stripped on relapse.
- **Held / pinned / grappled** — from a fighting manoeuvre; one penalty die on everything
  the target does until they break free with their own manoeuvre (KRB p. 105–108).
- **Outnumbered** — once a character has fought back or dodged this round, later *melee*
  attacks against them that round take a bonus die. Not for firearms, and not when one
  creature makes multiple attacks. A per-round, per-target derived flag — not stored state.
- **Aiming** — built up over a round; lost if the aimer moves or takes damage, so damage
  application must invalidate it.
- **Dived for cover** — sacrifices the next attack; restricts defence options that round.
- **Weapon malfunction / jam** — 1D6 rounds to clear.

These live on different lifetimes: some are per-round derived, some span rounds, some span
days. Do not store them uniformly.

---

## 9. Variants and optional rules

**Pulp Cthulhu** rewrites much of this chapter and must be a per-campaign toggle, never a
hardcode. Notable deltas: heroes carry a larger HP pool; natural healing is **2 HP/day** (3
with the Quick Healer talent); First Aid and Medicine each restore **1D4** instead of 1 and
1D3; stabilizing a dying hero with First Aid grants **1D4 HP** (a full 4 on an Extreme) and
a follow-up Medicine roll adds another 1D4; a failed First Aid may be pushed but a failed
push is fatal; and a "Death's Door" Luck spend (30+ points) lets a dying hero survive and
immediately recover 1D6+1 HP. Pulp also lets a hero pay Luck to stay conscious after a
failed CON roll, at doubling cost per round (1, 2, 4, 8, …).

Several third-party cheat sheets and the Roll20 compendium's "Wounds and Healing" page
present the **Pulp** numbers unlabelled. Any source claiming First Aid heals 1D4 is Pulp (or
6th edition) — not core 7e.

**Optional rules in the KRB wounds chapter** worth exposing as feature flags: Luck spending
(p. 99 / 125 / 154), concealed damage — the Keeper hides HP totals from players (p. 126),
armour revisited (p. 126), and prone modifiers in melee and firearms (p. 128).

The KRB prints a **combat damage flow chart** at p. 122 — treat it as the reference
specification for this state machine.

---

## 10. Edge cases the engine must decide explicitly

1. **Odd maximum HP rounding.** At max HP 11, is the major-wound threshold 5.5 (6+ triggers)
   or floored to 5? Exact integer arithmetic (`damage * 2 >= maxHp`) dodges floating point
   and matches "greater than or equal to half."
2. **Damage exactly equal to max HP.** The official Quick-Start reads "greater than or equal
   to" — instant death, not dying. Some fan summaries say strictly greater. Pick one,
   document it.
3. **Half-max recovery check at odd max HP.** The same rounding question for clearing the
   major wound at "half maximum or greater."
4. **A second major wound while one is active.** The flag is binary and doesn't stack — but
   the new blow still triggers prone + the CON roll.
5. **Simultaneous or split damage.** Two 4-point hits on a 12-HP character are no major
   wound; a single 6-point hit is. Never sum hits before evaluating thresholds.
6. **Multiple hits in one round versus the treatment budget.** One "instance of damage" or
   several, for First Aid/Medicine purposes? The rules are informal here.
7. **Damage while stabilized.** No rule explicitly says fresh damage breaks stabilization,
   but any sane ruling does. Decide.
8. **Healing at 0 HP, unconscious, no major wound.** The daily +1 applies and wakes them.
9. **Batch time advancement.** If the Keeper skips three weeks, the engine runs three weekly
   recovery rolls — re-evaluating the clear condition after each — not one roll times three.
10. **NPCs and monsters.** Most Keepers just drop NPCs at 0 HP rather than run the dying
    machinery; only a few published monsters interact with major wounds at all. Let an actor
    opt out of the full rules.
11. **Luck on survival rolls.** Core 7e bans Luck on Luck, damage, SAN, and SAN-loss rolls;
    CON rolls are characteristic rolls and are not on the list. Whether a table lets Luck
    buy a dying CON roll is a house decision the tracker should surface, not assume.
12. **Retroactive correction.** Keepers reverse mistakes constantly. Damage and condition
    changes must be undoable.

---

## Backend implications

**Entities / fields**

- `Actor`: `id`, `kind` (investigator | npc | monster), `con`, `siz`, `maxHp` (derived,
  cached, recomputed when CON/SIZ shift through ageing or effects), `currentHp`,
  `temporaryHp`, `armorValue`, `damageBonus`, `build`, `rulesVariant` (core | pulp),
  `usesFullWoundRules` (bool).
- `Condition` rows (or a bitfield plus side table): `majorWound`, `dying`, `stabilized`,
  `unconscious`, `prone`, `dead`, plus the transient combat states (`held`, `aiming`,
  `divedForCover`). Each carries `setAtRound` / `setAtGameTime` and, where relevant,
  `expiresAt`.
- `Injury` (one damage instance): `id`, `actorId`, `appliedDamage`, `rawDamage`,
  `armorApplied`, `source`, `wasMajorWound`, `occurredAtGameTime`. Injuries are what the
  treatment budget attaches to, and they carry the one-hour First Aid window.
- `Treatment`: `injuryId`, `skill` (first_aid | medicine), `practitionerId`, `rollResult`,
  `wasPushed`, `succeeded`, `hpRestored`, `appliedAtGameTime`. Enforce at most one
  *successful* First Aid and one *successful* Medicine per injury.
- `PendingCheck` / timer queue: `actorId`, `type` (dying_round | stabilization_hour |
  weekly_recovery | asphyxiation_round), `dueAt` in the appropriate clock, `resolved`.
- `Event` log (append-only): every damage application, heal, condition change, and roll,
  detailed enough to replay and reverse. The Keeper needs undo.

**Time model**

The single most important architectural decision here. Three clocks coexist:
- *Round clock* — dying countdown, asphyxiation, outnumbered flags. Advances only in combat.
- *Hour clock* — First Aid's one-hour window, stabilization checks, Medicine's one-hour
  minimum, same-day-vs-later Medicine difficulty.
- *Calendar clock* — daily +1 HP, weekly recovery rolls.

Keep one explicit in-game timestamp on the session and derive all three, rather than running
independent counters. Advancing the clock should be a single operation that fans out into
due checks — and it must handle large jumps by iterating every due interval.

**Rules engine shape**

- A pure `applyDamage(actor, rolledDamage, opts) -> { appliedDamage, transitions[],
  pendingChecks[] }` with no I/O, so the threshold logic unit-tests cleanly against the KRB
  flow chart (p. 122).
- Threshold comparisons in integer arithmetic (`2 * damage >= maxHp`) to sidestep rounding
  disputes.
- Compute conditions with a deterministic reducer over the event log where possible, making
  undo and replay free.
- Every roll the engine *requests* (CON to stay conscious, dying CON, weekly recovery) is a
  first-class prompt object, never an inline dice call — the Keeper may override, the player
  may spend Luck, and the tracker may be set to advise rather than enforce.
- Bonus/penalty dice on the weekly recovery roll must be composable inputs (environment,
  care, fumble), since they cancel under the standard modifier-die rules.

**Configuration surface**

`rulesVariant` (core/pulp), enforce-vs-advise mode, NPC simplification, Luck-on-CON
permission, concealed damage, and the optional rules — all campaign-level settings resolved
at rule-evaluation time.

---

## Open questions

1. Does the tracker **enforce** the rules (auto-kill on a failed dying roll) or **advise**
   the Keeper? This fundamentally shapes the API — enforcement needs an override path on
   every transition.
2. What is the canonical in-game time representation: one integer of in-game seconds, a
   structured `{day, hour, round}`, or a hybrid with a separate combat-round counter?
3. How is "one instance of damage" defined for the treatment budget when several attacks
   land in one round? Per-attack `Injury` rows, or an aggregating "wound" concept?
4. Does new damage break stabilization — and if so, does the patient return to dying
   immediately or at the next check?
5. Should Pulp Cthulhu be a runtime variant of the same engine or a separate rule set? The
   divergences (HP formula, healing dice, Luck mechanics, Death's Door) are wide enough that
   a strategy-object split may beat conditionals.
6. Are NPCs first-class actors with full wound state, or a lightweight "HP plus alive/dead"
   record with an opt-in upgrade?
7. Does the backend roll its own dice, accept externally rolled results, or both?
   Physical-dice tables need result entry; automated tables need a seeded, auditable RNG.
8. How much of §8's transient combat state belongs here versus the combat/initiative
   subsystem? Prone and unconscious clearly straddle both.
9. What happens on a recovery-roll fumble ("lasting injury")? Free-text Keeper note, a
   structured characteristic penalty, or an optional injury table (several third-party
   expanded-injury tables exist)?
10. Do we track *where* a character was wounded (limb, torso) for narrative colour? Core 7e
    has no hit locations, but Keepers improvise them all the time.

---

## Sources

- *Call of Cthulhu 7th Edition Quick-Start Rules* (Chaosium, free/official) — "Hit Points,
  Wounds, and Healing" and "Other Forms of Damage" sections, pp. 16–18. Copy consulted:
  https://www.fullasagoog.com/uploads/short-url/oTeMcZZrw4u3OOYgrpP0y3lZi2W.pdf
- Chaosium official *Call of Cthulhu* RPG Wiki — Hit Points, Wounds and Healing:
  https://cthulhuwiki.chaosium.com/rules/hit-points-wounds-and-healing.html
- Chaosium official *Call of Cthulhu* RPG Wiki — Combat:
  https://cthulhuwiki.chaosium.com/rules/combat.html
- Chaosium free PDF — *Call of Cthulhu 7th Edition Index for Keepers and Investigators*
  (used for the page references below):
  https://www.chaosium.com/content/FreePDFs/CoC%207/Call%20of%20Cthulhu%207th%20Edition%20Index%20for%20Keepers%20and%20Investigators.pdf
- *CoC 7e Keeper Rulebook* page references (via the official index): Hit Points 33, 119,
  120; Damage 103, 119–121; Regular Damage 119–121; Major Wound 119, 120, 121–123; Dying
  120–121; Death 119, 123; Unconsciousness 123; Combat Damage Flow Chart 122; Recovery
  (Regular Damage) 121; Recovery (Major Wound) 121–122; First Aid Skill 65, 88, 120;
  Medicine Skill 69, 120; Other Forms of Damage 123 with Table III at 124; Asphyxiation 124;
  Drowning 124; Poison 124, 128; Sample Poisons 129; Armor 108, 112; Extreme Damage / Impale
  103–104; Fighting Manoeuvre 105–108; optional rules — Prone 128, Concealed Damage 126,
  Armor Revisited 126, Luck Spending 99/125/154.
- TRPGLine CoC7 rules digest (detailed First Aid/Medicine/recovery modifiers):
  https://trpgline.com/rules/coc7/combat
- *CoC 7th Ed. Combat Cheatsheet v1.4* (community reference, used to cross-check
  prone-on-major-wound and the once-per-instance treatment limits):
  https://www.antagonistsrpg.com/files/CoC%20CheatSheet.pdf
- Roll20 *Call of Cthulhu* Compendium — "Hit Points, Wounds, and Healing" (core) and
  "Wounds and Healing" (**Pulp Cthulhu** numbers):
  https://roll20.net/compendium/coc/Hit%20Points,%20Wounds,%20and%20Healing ,
  https://roll20.net/compendium/coc/Wounds%20and%20Healing
- Alchemy RPG help centre, CoC ruleset summary (independent confirmation of the HP formula
  and condition progression): https://help.alchemyrpg.com/en/articles/10157823-call-of-cthulhu
- Foundry VTT CoC7 system (reference implementation; condition set and damage-triggered
  status): https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT and its
  changelog (`.github/CHANGELOG.md`, entries for 0.6.0, 0.6.3, 0.7.4).
- "Let's Study: Call of Cthulhu 7th Edition, Part 2c — Firearms & Damage Systems":
  https://philgamer.wordpress.com/2018/02/15/lets-study-call-of-cthulhu-7th-edition-part-2c-firearms-damage-systems/
