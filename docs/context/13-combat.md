# Combat Rules (Call of Cthulhu 7th Edition) — Backend Reference

> Context doc for the CoC Phoenix tracker backend. All mechanics are restated in our own
> words from the free official Quick-Start rules, Chaosium's Call of Cthulhu wiki, and
> community summaries; book page numbers are pointers only, never reproductions.

## Overview

7e combat proceeds in *rounds*; within a round every participant acts once, ordered by
descending DEX, with readied firearms acting as though DEX were 50 higher. There is no
to-hit number and no AC: an attack is a d100 skill roll whose **level of success** (fumble <
fail < regular < hard < extreme < critical) is set against the defender's own d100 roll —
Fighting if they fight back, Dodge if they dodge or dive for cover. The higher level wins;
ties go *to the attacker when the defender fights back* and *to the defender when the
defender dodges*. Winning means avoiding damage and (if attacking or fighting back) dealing
it. Everything else — point-blank shots, aiming, outnumbering, cover, target size — enters
as either **bonus/penalty dice** (extra tens dice, take best/worst) or a **difficulty bump**
(roll under half or one-fifth of skill). The load-bearing pieces for a rules engine: a
correct roll/success-level evaluator, a modifier stack fed by many independent sources, a
strict opposed comparison honouring the two asymmetric tie rules, and a wound state machine
(`healthy → major wound → unconscious → dying → dead`).

---

## 1. Roll primitives (prerequisites to any combat code)

### 1.1 The d100 roll

One *units* die (0–9) plus one *tens* die (00–90). `tens=00, units=0` reads as **100**, not
0; `tens=00, units=4` reads as **04** — the classic trap when a bonus die asks you to pick
the "lowest tens."

### 1.2 Success levels

Given `skill` (0–99+) and `roll` (1–100):

| Level | Condition |
|---|---|
| Critical | `roll == 1` |
| Extreme | `roll <= floor(skill / 5)` |
| Hard | `roll <= floor(skill / 2)` |
| Regular | `roll <= skill` |
| Failure | `roll > skill` |
| Fumble | `roll == 100`, or `roll >= 96` when `skill < 50` |

Comparison order: `Fumble < Failure < Regular < Hard < Extreme < Critical`. Store the level
as an ordinal enum (0..5, say) so comparisons reduce to integer comparisons.

### 1.3 Difficulty levels

A roll may be *demanded* at Regular / Hard / Extreme difficulty: it still yields a level,
but fails whenever that level sits below the requirement. Long-range shooting works exactly
this way. The consequence for damage code: **a shot at Extreme difficulty can only land on
an Extreme or Critical, so every hit at very long range is an impale.**

### 1.4 Bonus and penalty dice

Each bonus or penalty die contributes one extra *tens* die (the units die is shared). With
`n` net bonus dice, roll `1 + n` tens dice and keep the lowest combined result; with `n` net
penalty dice, keep the highest. One bonus and one penalty cancel exactly, so the modifier
stack collapses to a single signed integer before any dice move. Remember `00/0 = 100` when
choosing "lowest." The automatic-fire rules cap the stack at 2 penalty dice, converting
overflow into difficulty increases; generalizing that cap is a common, safe engine choice
(see Open questions).

### 1.5 Opposed rolls (generic)

Both sides roll and compare levels; higher wins. The generic tie-break: higher skill value
wins; equal skills → both roll d100, lower wins. **Combat overrides this tie-break** — §3.2.
Combat rolls can never be *pushed*; a failed attack simply means attacking again next round.

---

## 2. Round structure and initiative

* Order is **DEX descending**; investigators, NPCs, and monsters share a single list.
* A **readied firearm** — drawn, levelled, and fired before moving — acts at `DEX + 50`.
  That is purely an ordering key, not a DEX change. It is volatile: moving first, drawing
  mid-round, or having just reloaded forfeits it, so model it as a per-round boolean flag
  rather than a stat.
* Ties on the ordering key are unspecified by the rules; choose something deterministic
  (join order, or DEX then Luck) and let the Keeper override.
* A round is narrative — "long enough for everyone to do one significant thing." No
  action-point economy; the invariant is one action on your turn.
* **Defending is a reaction, not your action.** Fighting back or dodging on someone else's
  turn leaves your own attack intact. Diving for cover *does* cost an attack (§5.7).
* Combat movement is roughly MOV in yards (human default MOV 8); a sprint is about MOV × 5.
  There are no attacks of opportunity — leaving melee is free unless you're being held.

---

## 3. Melee attack resolution

### 3.1 The exchange

On their turn the attacker names a target and rolls their Fighting specialization (Brawl,
Sword, Whip, …). A defender aware of the attack picks one of:

* **Fight back** — roll Fighting. Winning both avoids damage *and* deals it.
* **Dodge** — roll Dodge. Winning avoids damage only; no counter-damage.
* **Maneuver** (as a defense) — roll Fighting (Brawl) for a non-damaging effect (§4).
* **Nothing** — an undefended attack succeeds automatically or takes a bonus die, failing
  only on a fumble; treat like a surprise attack (§7).

### 3.2 Outcome matrix and the two tie rules

Let `A` = attacker's level, `D` = defender's level.

Apply the success gate first — if `A` is below a regular success the attack cannot land, and
if `D` is below a regular success the defense achieves nothing.

| Defense | Condition | Result |
|---|---|---|
| Fight back | `A` succeeded and `A >= D` | Attacker hits, rolls damage |
| Fight back | `D` succeeded and `D > A` | Defender hits, damage **capped at regular** |
| Fight back | neither succeeded | Nothing happens |
| Dodge | `A` succeeded and `A > D` | Attacker hits, rolls damage |
| Dodge | `D` succeeded and `D >= A` | Attack avoided, no damage |
| Dodge | neither succeeded | Nothing happens |

The asymmetry is the thing code must nail: **fight-back ties favour the attacker, dodge ties
favour the defender.** Never fall back to the generic "higher skill wins" tiebreak here.

### 3.3 Fight-back damage cap

A defender who wins by fighting back deals **regular damage only** — no maximum damage, no
impale, however well they rolled. Only the initiator of the attack can impale. Carry this as
a separate flag on the damage call, not a property of the roll.

### 3.4 Fumbles in melee

A fumble is worse than a plain failure, but by itself it does not hand victory to an
opponent who also failed. Effects are Keeper-adjudicated (dropped weapon, a slip, a blade
stuck fast). Model as: emit a `fumble` event with context; the Keeper applies any condition
manually.

---

## 4. Fighting maneuvers

A maneuver is an attack aimed at something other than damage: disarm, knock prone, seize and
hold, shove someone through a window. Mechanics:

* Rolled as a normal Fighting (Brawl) attack; the target may fight back or dodge as usual.
* **Build comparison** (attacker's Build vs target's Build):
  * `delta = targetBuild - attackerBuild`
  * `delta <= 0` → no modifier. A *bigger* attacker earns **no bonus** dice from Build.
  * `delta == 1` → 1 penalty die; `delta == 2` → 2 penalty dice (the cap).
  * `delta >= 3` → the maneuver is **impossible**; reject before any dice roll.
* A successful maneuver deals **no damage** — it imposes a condition instead.
* "Seize and hold" puts one penalty die on everything the held character attempts until they
  break free; breaking free is itself a maneuver taken on the held character's turn, with
  Build differences applying to that escape too.
* Build derives from `STR + SIZ` alongside Damage Bonus (§6.2), so for human-scale
  characters it is a computed field, never stored independently. Monsters carry explicit
  Build.

---

## 5. Firearms

### 5.1 What a firearm attack is not

There is **no fighting back against a firearm**. A defender's only mechanical options are
diving for cover (a Dodge roll, §5.7) or closing to melee / attempting a maneuver such as
rushing the shooter. Also: **outnumbering never applies to firearm attacks** (§8).

### 5.2 Range bands

Every ranged weapon has a *base range*. Distance escalates the required difficulty rather
than stacking penalty dice:

| Distance | Required difficulty |
|---|---|
| ≤ base range | Regular |
| > base to 2× base | Hard |
| > 2× base to 4× base | Extreme |
| beyond 4× base | Out of range (Keeper call) |

### 5.3 Point-blank

Within `DEX / 5` **feet** of the target: one bonus die. Note the unit is feet and the
divisor is the shooter's DEX fifth-value — the same "fifth value" already cached on the
sheet. Some tables limit point-blank to handguns and short-barrelled shotguns (an optional
rule); make it configurable.

### 5.4 Aiming

Declared on the shooter's turn; the shot fires on the shooter's turn in the **following**
round with one bonus die. Aiming longer buys nothing extra. Aim breaks if the shooter takes
damage or moves. This is a two-round state machine (`AIMING → committed shot`) with
cancellation triggers.

### 5.5 Rate of fire and multiple shots

* Weapons list "uses per round" as `normal (max)` — e.g. a handgun that fires once per
  round normally but can be triggered up to three times.
* Exceeding the normal shot count applies **one penalty die to every shot that round,
  including the first**.
* Double-barrelled shotguns: the second barrel at the *same* target carries no extra
  penalty; at a *different* target it does.
* Loading and firing within the same round costs a penalty die.

### 5.6 Automatic / burst fire

* The shooter chooses volley size, up to `floor(skill / 10)`, with a floor of 3 bullets:
  `maxVolley = max(3, floor(skill / 10))`.
* One attack roll per volley, plus a fresh roll whenever the shooter switches targets.
* Second volley in a round: 1 penalty die. Third: 2 penalty dice. Beyond: hold at 2 penalty
  dice and raise difficulty one step per further volley.
* Hits: a success lands `max(1, floor(volleySize / 2))` bullets. An extreme success lands
  **all** of them, the first half as impales and the rest as normal damage.
* Sweeping fire across targets burns roughly one bullet per yard/metre traversed.
* Ammo decrements by the full volley size no matter how many bullets connected.

### 5.7 Cover and diving for cover

* **Diving for cover**: the target rolls Dodge. On success the shooter suffers one penalty
  die. The diver forfeits their next attack — this round if they haven't acted yet,
  otherwise next round. Cover must actually exist to dive behind.
* **Static cover** works like armor: subtract the cover's armor points from any damage that
  gets through (illustrative values in §6.4).
* Concealment of roughly half the target or more: one penalty die.

### 5.8 Modifier registry

| Bonus die | Penalty die |
|---|---|
| Aimed for one round | Target successfully dived for cover |
| Point-blank range | Target ≥50% concealed |
| Large target (Build 4+) | Fast-moving target (MOV 8+) |
| Target unaware / not defending | Small target (Build −2 or lower) |
| | More shots than the weapon's normal rate |
| | Loading and firing in the same round |
| | Firing into a melee |
| | Shooter moved this round instead of holding still |

### 5.9 Firing into melee

One penalty die. On a **fumble** (not a mere miss) the shot hits a friendly combatant in the
line of fire — a common table convention picks the ally with the lowest Luck. Once a shooter
is personally engaged in melee, the rules steer them toward clubbing with the gun (roughly
1D8 + DB) instead of firing; shooting while grappled stacks further penalty dice.

### 5.10 Malfunction

* Every firearm carries a **malfunction number** (typically in the 96–100 band).
* A final attack result `>= malfunction` means the weapon **does not fire at all** —
  distinct from a miss and from a fumble, though one roll can be both.
* Test malfunction *after* bonus/penalty dice resolve to a final result, and *before*
  computing hit or damage. Short-circuit the rest of the attack pipeline.
* Clearing a jam depends on the weapon: usually a Mechanical Repair (or relevant Firearms)
  roll, retried once per round, with serious jams taking on the order of 1D6 rounds. Some
  weapons merely suffer a dud cleared by cycling the action. Make the clearing cost
  per-weapon data, never hard-coded logic.

### 5.11 Reloading

Order-of-magnitude figures — store as per-weapon data, not constants:

* Loose rounds/shells into a revolver, rifle, or shotgun: about two per round.
* Swapping a box magazine or clip: one round.
* Feeding a belt into a machine gun: about two rounds.

Reloading normally forfeits the readied-firearm DEX+50 ordering bonus for that round.

---

## 6. Damage

### 6.1 Basic damage

Melee: `damage = weaponDamageDice + damageBonusDice`. Firearms take **no** damage bonus.
Thrown weapons add **half** the damage bonus. Illustrative values (Quick-Start): unarmed
1D3+DB, small knife 1D4+DB, baseball bat 1D8+DB, handgun 1D10, rifle 2D6+4, shotgun 4D6 up
close, tapering with distance.

### 6.2 Damage Bonus and Build

Both derive from `STR + SIZ`. Shape of the table (illustrative rows, not the full listing):

| STR+SIZ | Damage Bonus | Build |
|---|---|---|
| 2–64 | −2 | −2 |
| 65–84 | −1 | −1 |
| 85–124 | none | 0 |
| 125–164 | +1D4 | +1 |
| 165–204 | +1D6 | +2 |

The progression continues upward in +80 bands, each adding another D6 to DB and +1 to Build
— implement as a computed banding function rather than a lookup table, since monsters
routinely land far past the human end.

### 6.3 Extreme success / impale

* **Non-impaling weapons** (fists, clubs, bats): maximum weapon damage plus maximum damage
  bonus.
* **Impaling weapons** (blades, bullets, arrows — a per-weapon flag): maximum weapon damage
  plus maximum damage bonus, **plus one additional roll of the weapon's damage dice**. A
  handgun on an extreme success is therefore `10 + 1D10`.
* Shotgun pellet loads never impale; slug loads may.
* A **critical (01)** outranks extreme and deals at least extreme damage; the Keeper can
  stack narrative effects on top (ignore armor, stun, dismember).
* Defenders winning via fight-back never impale (§3.3).

### 6.4 Armor and cover

Armor points come off the damage total after the roll; damage floors at 0. Illustrative
scale: heavy leather jacket ~1 point, WWI helmet ~2, heavy Kevlar vest ~8, modern military
body armor ~12, sandbag ~20. Treat armor as a stackable list of sources (worn armor, cover,
a monster's natural hide) reducing a single incoming damage packet.

Optional variants worth exposing as flags: armor subtracted from *each rolled die* instead
of the total, and armor bypassed on criticals.

---

## 7. Surprise

* When one side is unaware, the Keeper may grant the target a Listen, Spot Hidden, or
  Psychology roll to notice in time; the attacker's Stealth usually sets that roll's
  difficulty.
* Against a surprised target the attack is either an **automatic success** or rolled with a
  **bonus die**, at Keeper discretion. Firearm and thrown attacks always require a roll,
  surprise or not.
* A surprised target can neither fight back nor dodge. Once surprise passes, normal DEX
  ordering resumes.
* A target who simply declines to defend gets the same treatment.

---

## 8. Outnumbering

Once a character has fought back **or** dodged once in the current round, every later
**melee** attack against them that round gains one bonus die. Implementation notes:

* It is **one** bonus die, never cumulative per attacker.
* It does **not** apply to firearms.
* The counter resets when a new round begins.
* Multi-attack creatures get a matching allowance: a monster with N attacks per round may
  defend N times before attackers earn the outnumbering die. The trigger is
  `defensesUsedThisRound >= defensesAllowed`, where `defensesAllowed` defaults to 1 and
  equals attacks-per-round for monsters.
* Symmetrically, a monster striking the same target several times in one round gains the
  outnumbering die on its second and later attacks against that target.

---

## 9. Hit points, wounds, and the health state machine

Derived: `maxHP = floor((SIZ + CON) / 10)`.

Transitions fired by an incoming damage packet `dmg` (post-armor):

1. `hp = max(0, hp - dmg)` — HP never goes negative.
2. **Instant death** when `dmg >= maxHP` from a single attack.
3. **Major wound** when `dmg >= maxHP / 2` from a single attack. The character falls prone
   and must pass a CON roll or drop unconscious. Major wound is a persistent boolean, not a
   one-off event.
4. **Unconscious** at `hp == 0` without a major wound.
5. **Dying** at `hp == 0` *with* the major-wound flag set. A dying character is unconscious
   and rolls CON at the end of each round; failure is death. Only First Aid lifts the dying
   state.
6. **First Aid** on a dying character stabilizes rather than heals (commonly modelled as 1
   temporary HP); the stabilized character then rolls CON hourly, and a follow-up Medicine
   roll is needed to move them out of danger. On a non-dying patient, First Aid restores 1
   HP and rouses the unconscious.
7. **Healing**: without a major wound, 1 HP per day. With one, a CON roll at each week's end
   for 1D3 HP (2D3 on an extreme success). The major-wound flag clears when that weekly roll
   comes up extreme **or** current HP reaches at least half of max.
8. **Medicine** heals 1D3 HP and needs at least an hour plus supplies.

Edge cases code must rule on: rounding `maxHP / 2` when max HP is odd; whether a second
qualifying hit on an already-major-wounded character re-triggers the prone/CON check; the
ordering of instant-death versus major-wound checks (instant death should win); and whether
armor applies before the major-wound threshold test (it should — the threshold measures
damage actually taken).

---

## 10. Resolution pipeline (suggested shape)

```
resolveMeleeAttack(attacker, defender, weapon, defense):
  if defender is unaware or declines: return autoHit(...)
  aRoll = roll(attacker.skill(weapon), mods.attacker)
  dRoll = roll(defender.skill(defense), mods.defender)
  winner = compare(aRoll.level, dRoll.level,
                   tieGoesTo = (defense == DODGE ? DEFENDER : ATTACKER))
  defender.defensesUsedThisRound += 1                     # outnumbering bookkeeping
  if winner == ATTACKER and aRoll.level >= REGULAR:
      dmg = rollDamage(weapon, attacker.db, impale = aRoll.level >= EXTREME)
  elif winner == DEFENDER and defense == FIGHT_BACK and dRoll.level >= REGULAR:
      dmg = rollDamage(defender.weapon, defender.db, impale = false)   # regular only
  else: dmg = 0
  loser = (winner == ATTACKER ? defender : attacker)
  applyDamage(loser, max(0, dmg - armor(loser)))

resolveShot(shooter, target, weapon):
  required = rangeBand(distance, weapon.baseRange)        # REGULAR | HARD | EXTREME
  mods = pointBlank + aiming + targetSize - cover - concealment - motion
         - excessShots - inMelee
  r = roll(shooter.skill(weapon), mods)
  if r.result >= weapon.malfunction: return MALFUNCTION   # before hit resolution
  if r.level < required: return MISS (plus fumble effects if fumbled)
  ...
```

Keep modifiers as a list of named contributions (`{source, delta}`) summed only at roll
time, so the UI can explain "3 bonus, 1 penalty → net +2" and house rules can be toggled
per source.

---

## Backend implications

**Entities / fields**

* `Combat` — `round`, `activeIndex`, ordered `turnOrder`, `state` (setup/running/ended),
  `surprise` phase flag.
* `Combatant` — actor ref; `dexKey` (DEX, +50 while readied-firearm holds), `readiedFirearm`,
  `hasActed`, `defensesUsedThisRound`, `defensesAllowed` (1 for humans, attacks-per-round
  for monsters), `aiming {targetId, sinceRound}`, `forfeitAttackUntilRound`
  (dive-for-cover), `conditions[]` (prone, held, surprised, unconscious, dying, major
  wound).
* `Weapon` — skill ref, damage expression, `addsDB`/`addsHalfDB`, `impaling`, `baseRange`
  plus optional per-band damage (shotguns), `usesPerRound {normal, max, burst}`,
  `magazineSize`, `ammoLoaded`, `malfunction`, `reloadCost`, property flags (ranged,
  shotgun, double-barrel, burst, auto, thrown, stun, blast, silent).
* `Health` — `maxHP` (from SIZ+CON), `hp`, `majorWound`, `unconscious`, `dying`,
  `stabilized`, plus due-times for the periodic CON rolls (per round while dying, hourly
  once stabilized, weekly while a major wound heals).
* `Roll` — persisted for audit: skill at roll time, requested difficulty, modifier
  contributions, every die face (units + all tens), chosen tens die, final result, level.
* `AttackResolution` — attacker, defender, defense chosen, both roll ids, winner, damage
  packet, armor applied, condition changes. This is the unit a Keeper will want to undo.

**Rules-engine shape**

* Success level is a pure function of `(skill, roll)` — isolate it and unit-test it hard,
  especially `00/0 = 100` and both fumble thresholds.
* Hold the modifier stack as data; resolve to a signed int at roll time; cap at ±2 with a
  configurable overflow-to-difficulty policy.
* The opposed comparator takes tie-direction as an explicit parameter — the single most
  likely place to ship a bug.
* Damage application is one funnel: `armor → hp clamp → instant death → major wound →
  unconscious → dying`, emitting an event per transition so the Keeper UI can prompt for
  CON rolls rather than silently auto-rolling them.
* Round advancement resets `hasActed`, `defensesUsedThisRound`, outnumbering flags, and
  excess-shot penalties, then re-evaluates readied-firearm and aiming state.
* Nearly every rule above has a table variant. Ship a `houseRules` config (point-blank by
  weapon type, armor-per-die, outnumbering reset timing, fumble-on-100-always, half-value
  rounding) instead of hard-coding one reading.

## Open questions

1. **Rounding `maxHP / 2`** for the major-wound threshold at odd max HP — RAW says "half";
   engines split between `>= maxHP/2` (real division) and `>= ceil(maxHP/2)`. Project
   decision needed.
2. **Half damage bonus on thrown weapons**: how do you halve a die expression (1D4 → 1D2?
   halve the rolled total?), and is a *negative* DB halved, doubled, or untouched? RAW is
   thin.
3. **Penalty-die cap**: "2 penalty dice max, then raise difficulty" is stated for automatic
   fire. Generalize to all rolls?
4. **Outnumbering reset timing**: RAW resets per combat round; several tables reset at the
   start of the defender's own turn. Configurable — but which default?
5. **Malfunction vs fumble interaction**: one roll can trigger both. Apply both
   consequences, or does malfunction short-circuit the fumble?
6. **Jam clearing**: varies by weapon action type. What per-weapon fields are needed
   (`jamClearSkill`, `jamClearRounds`), and what defaults?
7. **Aiming interruption**: "takes damage or moves" is clear — but does being targeted by a
   failed attack, or diving for cover, also break aim?
8. **Auto-roll Keeper-facing checks?** (CON to stay conscious, CON while dying, jam
   clearing.) Automation is convenient but strips Keeper discretion — likely per-table.
9. **Monster attack profiles**: creatures with N attacks per round need a schema for
   per-attack weapons, per-attack build/impale flags, and whether the N attacks may split
   across different foes.
10. **Friendly-fire target selection when firing into melee**: the "lowest-Luck ally"
    convention comes from community summaries; verify against the rulebook before encoding.
    Likewise, chase/movement rules (MOV 8+ penalty die, sprinting, disengaging) touch combat
    but live in their own doc.

## Sources

* *CoC 7e Quick-Start Rules*, Chaosium (free official PDF) — Combat, close-quarters
  fighting, maneuvers, firearms, outnumbered, hit points/wounds/healing, Damage Bonus and
  Build table. https://www.chaosium.com/call-of-cthulhu-quick-start-rules/
* Chaosium's official Call of Cthulhu RPG wiki:
  [Combat](https://cthulhuwiki.chaosium.com/rules/combat.html),
  [Game System](https://cthulhuwiki.chaosium.com/rules/game-system.html),
  [Hit Points, Wounds, Healing](https://cthulhuwiki.chaosium.com/rules/hit-points-wounds-and-healing.html),
  [Secondary Attributes](https://cthulhuwiki.chaosium.com/investigators/step-two-secondary-attributes.html),
  [Ammunition & Weapons](https://cthulhuwiki.chaosium.com/equipment/weapons.html)
* Basic Roleplaying ORC Content Document, Chaosium (open-licensed BRP engine; opposed rolls,
  bonus/penalty dice): https://www.chaosium.com/content/orclicense/BasicRoleplaying-ORC-Content-Document.pdf
* TRPGLine CoC 7e summary (firearm modifier list, armor values, reloading, automatic fire):
  https://trpgline.com/rules/coc7/summary
* CoC7 system for Foundry VTT — weapon data model and combat flow:
  https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT
  (`coc7/models/item/weapon-system.js`, `coc7/apps/chat-combat-melee.js`, `docs/en/combat.md`)
* "CoC 7th Ed — Combat Q&A", 21st Century Philosopher (clarifications with rulebook page
  pointers): https://morganhua.blogspot.com/2017/06/call-of-cthulhu-7th-ed-combat-q.html
* "Let's Study: CoC 7th Edition" parts
  [2b](https://philgamer.wordpress.com/2018/02/14/lets-study-call-of-cthulhu-7th-edition-part-2b-opposed-rolls-and-hand-to-hand-combat/)
  and [2c](https://philgamer.wordpress.com/2018/02/15/lets-study-call-of-cthulhu-7th-edition-part-2c-firearms-damage-systems/)
* "Automatic Fire in CoC 7th Edition", RogerBW's Blog (volley mechanics, cites Keeper
  Rulebook pp. 114–116):
  https://blog.firedrake.org/archive/2021/05/Automatic_Fire_in_Call_of_Cthulhu_7th_Edition.html
* *CoC 7e Keeper Rulebook*, Chaosium — combat chapter roughly pp. 100–130 (maneuvers
  ~p. 106, outnumbering ~p. 108, firearms ~pp. 110–118, automatic fire ~pp. 114–116, wounds
  ~pp. 119–125). *CoC 7e Investigator Handbook* — Damage Bonus and Build table ~p. 48. Page
  numbers differ between the 2014 and 2023 revised printings.
