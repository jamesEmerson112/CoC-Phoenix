# Weapons, Equipment & NPC/Creature Stat Blocks (CoC 7e)

## Overview

7th Edition expresses weapons as rows in a handful of nearly identical tables (hand-to-hand, handguns, rifles/shotguns, submachine guns, machine guns, explosives) whose columns are stable enough to fold into a single `Weapon` entity with mostly-nullable fields. Nearly every column is a dice expression, a distance, a small integer, or a threshold compared against the d100 attack roll. NPCs and Mythos creatures share one stat-block shape: eight-ish characteristics, a few derived attributes (HP, DB, Build, MP, MOV), an attacks-per-round count, a list of attack lines each shaped `skill% (hard/extreme), damage <dice>[+DB|+½DB]`, a Dodge line, an armor entry that is frequently a *rule* rather than a number, an optional spell allowance, and a Sanity-loss pair `X/Y`. The load-bearing details for a tracker backend: which weapons add damage bonus and at what fraction, how range bands map to difficulty levels rather than flat modifiers, that malfunction is a failure mode distinct from a fumble, and that creature armor is often a damage-transformation rule no numeric field can hold.

---

## 1. Weapon table column model

The printed tables use these columns. Types below are what a backend wants, not what is printed.

| Printed column | Meaning | Suggested type | Notes |
|---|---|---|---|
| Name | Weapon identity | string | Caliber/action ride along inconsistently ("2B" = two barrels, "Lg" = large) |
| Skill | Which skill resolves the attack | skill reference | `Fighting (Brawl/Sword/Whip/…)`, `Firearms (Handgun/Rifle-Shotgun/SMG/MG/Bow/Flamethrower/Heavy Weapons)`, `Throw` |
| Damage | Base damage dice | dice expression + DB flag | e.g. `1D10+2`, `1D3+DB`, `1D6+½DB` |
| Base range | Distance at which the shot is Regular difficulty | number (yards) or formula | Melee rows print "touch"/melee; thrown rows often derive from STR |
| Uses per round | Attacks/shots initiable per round | `{normal, max, burst}` | Printed as `1`, `1(3)`, `2`, `1 or burst` |
| Bullets in gun | Magazine / cylinder / drum capacity | integer, nullable | "Capacity" in earlier editions; null for melee |
| Cost | Era price | map era → string | The rulebook prints 1920s (often also Modern) prices |
| Malfunction (Mal) | Threshold at/above which the weapon fails | integer 1–100, nullable | Roughly 88–100 in practice; null = never malfunctions |
| Era / availability | Which periods the item exists in | set of era enums | 1890s / 1920s / Modern, plus rarity flags |

Per-weapon booleans the tables bury in footnotes, symbols, or prose — these matter more to a rules engine than the numeric columns:

- `ranged` — uses range bands and the firearms resolution path
- `impale` — impales on Extreme success (blades, bullets, arrows; explicitly *not* shotgun pellets)
- `add_db` / `add_half_db` — full DB (melee), half DB (thrown, bow, sling), none (firearms, crossbow)
- `thrown`, `shotgun`, `burst`, `auto`, `dual_barrel`, `mounted`, `blast` (+ blast radius), `stun`, `burn`, `silent`, `special`, `rare`
- `maneuver` — usable for a fighting maneuver instead of plain damage

(The Foundry VTT CoC7 weapon data model carries essentially this exact set: `skill.main`/`skill.alternativ`, `range.{normal,long,extreme}.{value,damage}`, `usesPerRound.{normal,max,burst}`, `bullets`, `ammo`, `malfunction`, `blastRadius`, a `properties` boolean bag, and a per-era `price` map. Useful as a sanity check on any schema you draft.)

### 1.1 Representative rows (reworded, illustrative only)

| Weapon | Skill | Damage | Base range | Uses/rd | Mag | Mal |
|---|---|---|---|---|---|---|
| Unarmed strike | Fighting (Brawl) | 1D3 + DB | melee | 1 | — | — |
| Heavy revolver (.45) | Firearms (Handgun) | 1D10+2 | ~15 yd | 1 (up to 3) | 6 | 100 |
| Bolt-action service rifle (.303) | Firearms (Rifle/Shotgun) | 2D6+4 | ~110 yd | 1 | 10 | 100 |
| Submachine gun (Thompson) | Firearms (SMG) | 1D10+2 | ~20 yd | 1 or burst | 20 | 96 |
| Bow | Firearms (Bow) | 1D6 + ½DB | ~30 yd | 1 | 1 | 97 |

Just enough rows to show the shape; the full tables live in the Keeper Rulebook (weapons chapter, roughly pp. 397–405 in the standard hardback, and the Investigator Handbook equipment chapter).

### 1.2 Shotguns: range-banded damage

Shotguns are the one family whose damage is a function of range band rather than a constant — the row carries up to three (range, damage) pairs. Shape example for a 20-gauge double-barrel: `2D6` to ~10 yd, `1D6` to ~20 yd, `1D3` to ~50 yd. A 12-gauge repeats the shape with bigger dice (`4D6 / 2D6 / 1D6`). Model consequences:

- `range` must be a list/struct of up to three bands, each with its own damage expression — not a scalar.
- The band boundaries are **not** the §3.1 difficulty bands. For shotguns the band picks the *damage dice*; difficulty escalation on the attack roll still keys off base range.
- Shotguns never impale.
- A double-barrel can fire both barrels; treat as two attacks (or a special "both barrels" mode), never one big dice pool.

### 1.3 Melee and thrown quirks

- Melee rows leave range, magazine, and malfunction blank. Uses per round is normally 1.
- Thrown weapons commonly print a STR-derived range (a small fraction of STR, in yards), so `base_range` must support a formula, not just an integer.
- Some entries have an alternate skill (a rifle butt swung as a club uses Fighting, not Firearms). Model `skill.primary` and `skill.alternate`.

---

## 2. Damage bonus (DB) and Build

### 2.1 Derivation

Both read off one table keyed on `STR + SIZ`:

| STR+SIZ | Damage bonus | Build |
|---|---|---|
| ≤ 64 | −2 | −2 |
| 65–84 | −1 | −1 |
| 85–124 | none | 0 |
| 125–164 | +1D4 | +1 |
| 165–204 | +1D6 | +2 |

Above that, the table extends by another `+1D6` DB and `+1` Build per further 80 points (205–284 → `+2D6`/3, 285–364 → `+3D6`/4, and onward). This matters because Mythos creature stat blocks routinely sit in the `+4D6` to `+6D6` region, far past any printed row — implement the extension as a formula, never a hard-coded lookup.

Implementation note: DB is heterogeneous — sometimes an integer (`−2`, `−1`, `0`), sometimes a dice expression (`+1D4`, `+6D6`). Store it as a string/expression the dice evaluator can consume, with a separate `max_db` evaluation path for impale (§4.1).

### 2.2 Application rules

- **Full DB**: melee/hand-to-hand attacks, unarmed included, and most Fighting-skill weapons.
- **Half DB**: thrown weapons and muscle-powered missiles whose force comes from the user (bows, slings). Halving a dice expression means either halving the dice or the rolled total — the practical convention halves the rolled total, rounding per table preference; flag as a config decision.
- **No DB**: firearms and crossbows. Their damage is fixed by the weapon.
- Negative DB still applies and can shrink damage; enforce a floor of 0 (or 1, per ruling) — never negative damage.

Build is a *separate* scalar for combat maneuvers, grappling, and chase/physical contests — not a damage modifier. Maneuver rule of thumb: compare initiator Build to target Build; a shortfall of 1–2 puts a penalty die on the initiator, a shortfall of 3+ makes the maneuver impossible. The comparison is asymmetric — being bigger removes the penalty but never grants a bonus die.

---

## 3. Firearm resolution pipeline

Model an attack as a pipeline producing (a) a difficulty level, (b) a bonus/penalty die count, (c) a roll, (d) an outcome branch.

### 3.1 Range → difficulty (not a flat modifier)

7e converts distance into a *difficulty level* on the skill roll:

| Distance | Difficulty |
|---|---|
| Point-blank (within DEX ÷ 5, in feet) | Regular, plus one bonus die |
| Up to base range | Regular (roll ≤ skill) |
| Up to 2× base range | Hard (roll ≤ skill ÷ 2) |
| Up to 4× base range | Extreme (roll ≤ skill ÷ 5) |
| Beyond 4× base range | Not possible |

This matters to a rules engine: difficulty raises the *success threshold* — it never adds a modifier. It also interacts with impale (§4.1): at Extreme difficulty the Extreme-success threshold coincides with the difficulty itself, so the rule falls back to requiring a critical (01) to impale.

### 3.2 Bonus/penalty dice sources (stackable, then netted)

- Aiming a full round: +1 bonus die.
- Point-blank: +1 bonus die.
- Firing more than one shot in a round: −1 penalty die on *every* shot that round (not merely the extras).
- Target diving for / behind cover: −1 penalty die.
- Firing into a melee containing friendlies: −1 penalty die (a fumble there can strike a bystander).
- Net bonus against penalty first; the remainder is how many extra tens dice roll.

### 3.3 Uses per round

`uses_per_round` caps shots. Semi-auto/revolver rows typically print normal 1, maximum 3. Loading a single round and firing in the same round is possible for a penalty die. Track a "readied weapon" flag separately: a weapon already drawn and levelled grants a large initiative (DEX) bump, and reloading or drawing clears it.

### 3.4 Automatic fire

Full-auto resolves as *volleys*, never one roll per bullet:

1. Volley size = `floor(skill ÷ 10)`, floored at 3 bullets regardless of skill.
2. The player declares total bullets, split into volleys of that size (the last may run short).
3. Volley 1 rolls normally. Volley 2 takes one penalty die. Volley 3 takes two. From volley 4 the penalty dice hold at two and the required difficulty escalates instead (one level, then two, …).
4. On a Regular/Hard success, half the volley's bullets hit (round down, minimum 1); roll damage per hit.
5. On an Extreme success, the whole volley hits — the first half as impales, the remainder rolled normally.
6. Targets must sit within roughly a 60° arc; traversing between targets burns bullets by distance.
7. Ammunition decrements by bullets *fired*, not bullets that hit.

### 3.5 Malfunction vs fumble (two distinct failure modes)

- **Malfunction**: the roll is ≥ the weapon's malfunction number. The weapon does not fire at all — not merely a miss. Clearing a jam usually costs several rounds (a 1D6-ish window), except revolvers, which just skip the bad chamber. Model as a `jammed` state on the weapon instance with `rounds_to_clear`, plus a Mechanical Repair path for hard jams.
- **Fumble**: a roll of 100, or 96–100 when skill is below 50. A fumble is a *narrative* failure (dropped weapon, hit a friend, etc.), resolved separately.
- The two can coincide; a 100 on a weapon with malfunction 96 is both. Decide precedence explicitly.
- Automatic fire needs per-volley malfunction checks — each volley is its own roll.

---

## 4. Turning a hit into damage

### 4.1 Success level and impale

- Regular/Hard success: roll the weapon's damage dice, adding DB per §2.2.
- Extreme success with an `impale`-capable weapon: maximum weapon damage (plus maximum DB where DB applies) **plus** one additional roll of the weapon's damage dice. A 1D10 pistol impale is `10 + 1D10`.
- Non-impaling weapons (blunt instruments, shotgun pellets, unarmed) on an Extreme: maximum damage with no extra roll (verify against your table's preferred ruling; some sources read it as maximum only).
- Impale is unavailable when the hit is a "fight back" defensive success in melee.
- At Extreme difficulty (very long range) only a critical 01 delivers the impale bonus.

### 4.2 Armor

- Ordinary/investigator armor is a flat point value subtracted from each incoming damage roll (about 1 for a heavy leather jacket up to the teens/twenties for improvised cover like sandbags). Floor at 0.
- Creature armor entries are frequently **not numbers**. Real forms include: a flat value ("2-point hide"), a damage transformation ("firearms do half damage"), a per-hit cap ("bullets and shot do 1 point"), an immunity ("physical weapons do no harm; only enchanted weapons or spells"), and conditionals ("no armor, but regenerates N HP per round"). The honest model is a `string` field plus a set of typed modifier rules; a plain integer cannot represent this.
- Regeneration, where present, is a per-round state transition running at end of round — capable of reviving a creature from 0 HP.

### 4.3 Wound thresholds (cross-reference)

Damage feeds the health state machine: a single attack dealing ≥ half maximum HP is a major wound; 0 HP is unconsciousness; 0 HP while already carrying a major wound is the dying state; one blow exceeding maximum HP is instant death. Those rules live in the health/combat doc, but the damage pipeline must emit enough (single-hit total, post-armor) for that state machine to run.

---

## 5. Non-weapon equipment

7e models non-weapon gear very lightly — no encumbrance system, no formal inventory weight. What actually matters mechanically:

- **Cost / era**: gear lists give period prices; purchases gate on the Credit Rating skill and its spending-level tiers, not a running cash balance. A `price` map keyed by era plus a `credit_rating_required` hint covers most needs.
- **Availability / rarity**: some items are era-gated or flagged rare.
- **Skill-enabling gear**: certain rolls presuppose tools (first-aid supplies, lockpicks, a camera). Model as an optional `enables_skills` / `penalty_if_missing` relation, not a hard requirement.
- **Light, cover, and vehicles** are situational modifiers, not inventory attributes.
- **Ammunition** is the one consumable genuinely worth counting: track loose rounds separately from rounds sitting in a specific weapon instance.

---

## 6. NPC and creature stat blocks

### 6.1 Canonical shape (human NPC)

A reworded example of the standard layout — stable enough to parse:

```
Name, age NN, short descriptor
STR 75  CON 60  SIZ 80  DEX 70  APP 60  INT 80
POW 50  EDU 85  SAN 55  HP 14   DB +1D4
Build 1   Move 7   MP 10   Luck 40   Armor 1
Attacks per round: 3     Sanity loss: 1D4/1D8
Combat
  Bite       50% (25/10), damage 1D6
  Brawl      30% (15/6),  damage 1D3
  Derringer  40% (20/8),  damage 1D8+1
  Dodge      50% (25/10)
Skills: <name> N%, <name> N%, …
Languages: <name> N%, …
Spells: <spell>, <spell>
```

Field-by-field observations for a parser/data model:

- Characteristic ordering varies across publications (STR CON SIZ DEX APP INT POW EDU vs. STR CON DEX INT SIZ POW APP EDU, and others). Parse by label, never by position.
- `HP`, `MP`, `Build`, `DB`, `Move` are derived for investigators but authored (sometimes deliberately off-formula) for NPCs. Every derived field wants an `auto: bool` override flag so a Keeper can pin a printed value that defies the formula.
- `DB` is a string (`+1D4`, `none`, `+6D6`). `Build` is a signed integer. `Armor` may be an integer or prose. `Move` may be one number or several modes (§6.2).
- Percentile entries carry their derived halves in parentheses: `45% (22/9)` is regular/hard/extreme — `v`, `floor(v/2)`, `floor(v/5)`. Store only the base; compute the rest.
- Attack lines are `name`, `skill%`, `damage expression`. Damage expressions embed `+ damage bonus` or `+ half damage bonus` in prose — normalize to the §1 `add_db` / `add_half_db` flags.
- `Dodge` is shaped like an attack line but is a defensive skill; treat it as a skill, never a weapon.
- `Attacks per round` is a small integer (or "varies"/"none") capping attack actions. Multi-attack creatures often carry heterogeneous attacks (two claws plus a bite) with per-attack constraints in prose.
- `Sanity loss` is the pair `X/Y`: X on a passed SAN roll, Y on a failed one. Either side may be flat or dice (`0/1D6`, `1/1D10`, `1D10/1D100`). It is normally charged on *first* sight per investigator — the tracker must remember who has already paid.

### 6.2 Mythos creature differences

Creature entries add or change the following versus human NPCs:

- **Characteristics appear twice**: as a roll formula and as an average. A typical line pairs something like `(3D6+6) × 5` with an average of 80. Keep both — the formula generates a fresh individual, the average supports quick "use as printed" instantiation. Note the `× 5` convention: creature characteristics roll on the 3D6/2D6+6 scale then multiply by 5 into percentile space.
- **APP and EDU are frequently absent** (many creatures have neither meaningful appearance nor education). Characteristic fields must be nullable, and every dependent formula must tolerate null.
- **Derived values are labelled "average"**: Average Hit Points, Average Damage Bonus, Average Build, Average Magic Points — computed from the average characteristics.
- **Move can be multi-modal**: `8/10 swimming`, `7/15 flying`, `6 burrowing`. Model as a list of `{value, mode}`, never a scalar.
- **Armor is often a rule** (§4.2).
- **Spells** appear as a probability plus a count ("40% chance of knowing 1D4 spells") rather than a fixed list, sometimes with named guaranteed spells. Model as `{chance, count_formula, guaranteed[]}`.
- **Skills** are a short list — usually only those differing from a plain roll.
- **Special abilities** are free prose (regeneration, mind control, paralysis on touch, damage-type immunity, HP loss inflicting characteristic drain). Full schematization is impossible — reserve a structured `traits[]` for the recurring ones and free text for the rest.
- Great Old Ones and outer gods break the schema outright: characteristics in the hundreds or thousands, "attacks per round" in dozens, damage measured in Build points, SAN loss up to `1D100`, and rules like "damage is only meaningful in aggregate." Treat them as a distinct actor subtype with looser validation.

---

## 7. Backend implications

**Entities**

- `WeaponDefinition` (catalog/template): `id`, `name`, `category` (hand-to-hand | handgun | rifle-shotgun | smg | mg | explosive | other), `skill_primary`, `skill_alternate?`, `damage` (dice expression), `db_mode` (none | full | half), `range_bands[]` (`{max_distance | formula, damage?}`, 1–3 entries), `uses_per_round {normal, max, burst?}`, `magazine?`, `malfunction?`, `blast_radius?`, `properties` (bitset/boolean bag: ranged, impale, thrown, shotgun, burst, auto, dual_barrel, mounted, stun, burn, silent, rare, special, maneuver), `price_by_era{}`, `eras[]`, `source_ref`.
- `WeaponInstance` (owned): `definition_id`, `owner_id`, `ammo_loaded`, `ammo_reserve`, `readied` (bool), `jammed` (bool + `rounds_to_clear`), condition/quality modifiers, `custom_overrides` (any definition field a Keeper pinned).
- `EquipmentItem`: `name`, `price_by_era{}`, `eras[]`, `credit_rating_hint?`, `quantity`, `notes`, `enables_skills[]`.
- `StatBlock` (shared by NPC and creature, discriminated by `kind`): nullable `characteristics{str,con,siz,dex,app,int,pow,edu}` each holding `{value, formula?, is_average}`; `derived{hp, mp, san?, luck?, db, build, mov[]}` each with an `auto` override flag; `attacks_per_round`; `attacks[]`; `dodge`; `armor{value?, rule?, text}`; `spells{chance?, count_formula?, list[]}`; `skills[]`; `traits[]`; `san_loss{on_success, on_failure}`; `notes`.
- `AttackLine`: `label`, `skill_value`, `damage_expression`, `db_mode`, `weapon_ref?`, `constraints` (e.g. "only if grappling", "one per round").

**Derived-value service**

- `db_and_build(str, siz)` as a formula with the +80-per-step extension — never a fixed lookup.
- `success_bands(v) -> {regular: v, hard: floor(v/2), extreme: floor(v/5)}` — compute, never store.
- `hp = floor((con + siz) / 10)`, `mp = floor(pow / 5)`; MOV derived from the DEX/STR-vs-SIZ comparison with a per-decade penalty past age 40 for humans, authored outright for creatures.

**Rules-engine surface**

- An attack resolution function taking `(attacker, weapon_instance, target, distance, declared_shots, situational_flags)` and returning `{difficulty_level, net_bonus_dice, roll, outcome, damage_events[], weapon_state_changes[], ammo_delta}`.
- Difficulty is a first-class enum (regular | hard | extreme | critical-only), *never* a numeric modifier — the single most common modelling mistake when porting from d20-style systems.
- The outcome branch must distinguish: malfunction, fumble, miss, regular hit, hard hit, extreme/impale, critical.
- Damage application takes armor rules as pluggable transformations `(raw_damage, source_type) -> final_damage`, so "firearms do half" and "bullets do 1" are expressible.
- Automatic fire needs a volley loop with per-volley escalation state.
- SAN-loss-on-sight needs an idempotency record keyed on `(investigator_id, creature_type_id)` so repeat encounters don't re-charge.

**Data-import surface**

- Stat blocks circulate as plain text inside adventures, so a text importer earns its keep early; the Foundry CoC7 importer's label-driven regexes are a good reference for the label vocabulary (`Attacks per round` / `# Attacks`, `Sanity loss` / `SAN loss`, `HP` / `Hit points`, `DB` / `Damage Bonus`, `Move`, `Build`, `Armor`).
- Never ship copyrighted stat data in the product; ship the *schema* and let Keepers import.

---

## 8. Open questions

1. **Half damage bonus rounding.** Halve the dice (`1D6` → `1D3`) or halve the rolled total (rounding which way)? Tables differ; needs a configurable rule with a documented default.
2. **Extreme success with non-impaling weapons.** Some readings grant maximum damage; others plain damage with no bonus at all. Pick one, expose as config.
3. **Malfunction vs fumble precedence** when one roll satisfies both — and whether a fumbled firearm roll should also jam.
4. **Jam clearing.** Rounds/skill rolls required vary by weapon type (revolvers vs. autoloaders vs. belt-fed). Is clearing an action, a skill roll, or both?
5. **Negative DB floor.** Can a `−2` DB drop a damage roll below 1, or below 0?
6. **Multi-shot penalty scope.** Does the penalty die from firing 2–3 shots hit the first shot too (sources say yes), and how does it interact with a point-blank bonus die?
7. **Shotgun band boundaries vs difficulty bands.** Confirm whether a shotgun fired past its longest damage band is impossible, or merely Extreme difficulty at minimum damage.
8. **Creature armor taxonomy.** How many distinct armor-rule types before free text is acceptable? Candidate set: flat, halve-by-source, cap-by-source, immune-by-source, regenerate.
9. **"Attacks per round" semantics.** A hard cap on attack actions, a budget shared with movement/maneuvers, or advisory? Multi-limb creatures with per-attack prose constraints force a decision.
10. **Pulp Cthulhu variants.** Pulp changes hit points, damage handling, and adds talents. Should the weapon/stat-block schema carry a ruleset discriminator from day one?
11. **Era model.** Is era a campaign property (single global) or a per-item availability filter — and do modern/1890s price columns need to be first-class?

---

## Sources

- Chaosium, *Call of Cthulhu 7th Edition Quick-Start Rules* (free PDF) — https://www.chaosium.com/cthulhu-quickstart/ and https://www.chaosium.com/7th-edition-pdfs/
- The Call of Cthulhu RPG Wiki (Chaosium official) — Ammunition & Weapons: https://cthulhuwiki.chaosium.com/equipment/weapons.html
- The Call of Cthulhu RPG Wiki — Combat: https://cthulhuwiki.chaosium.com/rules/combat.html
- The Call of Cthulhu RPG Wiki — Secondary Attributes (HP, DB/Build, MP, MOV): https://cthulhuwiki.chaosium.com/investigators/step-two-secondary-attributes.html
- The Call of Cthulhu RPG Wiki — Mythos Monsters (stat block examples: deep one, ghoul): https://cthulhuwiki.chaosium.com/bestiary/mythos-monsters.html
- Foundry VTT CoC7 system (unofficial, actively maintained) — weapon data model `coc7/models/item/weapon-system.js`, NPC/creature models `coc7/models/actor/npc-system.js` and `global-system.js`, weapon compendium `compendiums/en-wiki-weapons.yaml`, stat-block importer `coc7/apps/actor-importer-regexp.js`: https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT
- "Weapons in Call of Cthulhu, Seventh Edition", Shooting Dice blog (column-by-column commentary): https://shootingdiceblog.wordpress.com/2016/12/15/weapons-in-call-of-cthulhu-seventh-edition/
- "Let's Study: Call of Cthulhu 7th Edition, Part 2c — Firearms & Damage Systems": https://philgamer.wordpress.com/2018/02/15/lets-study-call-of-cthulhu-7th-edition-part-2c-firearms-damage-systems/
- "Automatic Fire in Call of Cthulhu 7th Edition", RogerBW's Blog (cites Keeper Rulebook rev. pp. 114–116): https://blog.firedrake.org/archive/2021/05/Automatic_Fire_in_Call_of_Cthulhu_7th_Edition.html
- CoC 7e combat Q&A (malfunction vs fumble, jam clearing, maneuvers/Build): https://morganhua.blogspot.com/2017/06/call-of-cthulhu-7th-ed-combat-q.html
- TRPGLine CoC7 rules summary (range difficulty bands, armor values, MOV/MPH): https://trpgline.com/rules/coc7/summary
- Book references for the authoritative text: *Call of Cthulhu Keeper Rulebook* (7e) — combat and firearms chapter (~pp. 100–120), weapons tables (~pp. 397–405), monsters/bestiary chapter; *Investigator Handbook* (7e) — equipment and possessions chapter.
