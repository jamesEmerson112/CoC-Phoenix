# Skills, Specializations & Improvement (CoC 7e)

> Context doc for the CoC Phoenix tracker backend. Every mechanic is paraphrased in our own
> words; page references point into the Chaosium books so a Keeper can verify against RAW.
> Nothing here reproduces rulebook text or its tables.

## Overview

In 7th Edition, every skill is a percentile rating the investigator rolls `1d100` under to
succeed. Each skill carries a *base value* — an untrained person's chance — and rises above
it at character creation (occupation points, personal-interest points) and later through
play. In-play improvement is a two-phase state machine: (1) during a scenario, a skill used
successfully gets **flagged** (the "tick" / experience-check box on the sheet); (2) between
scenarios, in the **Investigator Development Phase**, each flagged skill is rolled — and it
is *failing* that roll (rolling **above** the current value, or rolling very high no matter
what) that grants the `+1d10` percentile growth. The design intent: the better you already
are, the harder further improvement gets. A few skills sit outside the loop entirely:
**Cthulhu Mythos** never grows through improvement checks (only through exposure to the
Mythos — and it caps maximum Sanity), and **Credit Rating** moves by Keeper adjudication
rather than ticks. For a backend this means the tick flag is durable per-skill state that
outlives sessions, a skill's value must keep its *components* separable (base vs. occupation
vs. personal vs. experience), and specialization skills are not single fields but open-ended
sets of named instances under a group.

---

## 1. Anatomy of a skill

A skill instance on an investigator needs at least:

| Field | Meaning |
| --- | --- |
| `group` / `specialization` | e.g. `Science`, `Firearms`, or empty for a plain skill |
| `name` | e.g. `Chemistry`, `Handgun`, `Spot Hidden` |
| `baseValue` | integer, or a formula (see §3) |
| `occupationPoints` | points spent from the occupation pool at creation |
| `personalPoints` | points spent from the personal-interest pool at creation |
| `experiencePoints` | cumulative growth from development phases / study |
| `improvementFlag` | boolean — the tick box |
| `properties` | capability flags (below) |

**Value composition.** Persist the components, never just the total. The pre-modifier total
is simply their sum. Why it matters:

- Formula bases (Dodge, Language (Own)) must be re-derivable when the source characteristic
  moves.
- The development phase writes to `experiencePoints` alone, keeping growth auditable and
  reversible.
- Creation-time validation ("did the occupation pool get overspent?") requires the split.

```
rawValue   = baseValue(actor) + occupationPoints + personalPoints + experiencePoints
value      = applyTemporaryModifiers(rawValue)     # conditions, spells, wounds, etc.
hardValue  = floor(value / 2)                      # Hard success threshold
extremeVal = floor(value / 5)                      # Extreme success threshold
```

**Capability flags** worth carrying (mirroring the Foundry CoC7 skill model — see Sources):

- `noExperienceGain` — can never be ticked (Cthulhu Mythos, Credit Rating).
- `noPersonalAdjustment` — takes no personal-interest points (Credit Rating: occupation
  points only).
- `pushable` — whether a failure may be pushed (most can; Cthulhu Mythos notably cannot).
- `specialization` / `requiresName` — belongs to a group and needs a concrete name before
  it is usable.
- `uncommon` — hidden by default (Demolitions, Diving, Hypnosis, Read Lips, Artillery,
  Animal Handling).
- `fighting` / `firearm` — drive combat resolution and weapon linkage.
- `keepBaseValue` — the base is a live formula that re-derives instead of freezing at
  creation.

---

## 2. Base values — shape of the data

7e draws on a small set of distinct base values. Instead of copying the rulebook table, here
is the *distribution* — which is what a schema designer actually needs:

| Base | Character of the skills in this band |
| --- | --- |
| 01% | Specialist/academic skills nobody fakes — Anthropology, Archaeology, Electronics, Locksmith, Medicine, Operate Heavy Machinery, Psychoanalysis, and (one exception aside) every `Science` and `Pilot` specialization, plus non-native `Language` |
| 05% | Broad but trained skills — Accounting, Appraise, Disguise, Fast Talk, History, Law, Occult, Ride, Rope Use, Art/Craft specializations, Animal Handling |
| 10% | Everyday-competence skills — Electrical Repair, Mechanical Repair, Gambling, Natural World, Navigate, Persuade, Psychology, Sleight of Hand, Survival specializations, Track, Science (Mathematics) |
| 15% | Charm, Intimidate, plus a few weapon specializations |
| 20%–25% | Physical/perceptual defaults — Climb, Drive Auto, Jump, Library Use, Listen, Stealth, Swim, Throw (20); Spot Hidden (25) |
| 30% | First Aid — the highest fixed base in the core list |
| 00% | Credit Rating and Cthulhu Mythos (§4, §5) |
| formula | Dodge, Language (Own) (§3) |

Illustrative rows in seed-file shape:

```yaml
- { group: null,       name: "Spot Hidden",  base: 25, pushable: true }
- { group: "Science",  name: "Chemistry",    base: 1,  pushable: true, specialization: true }
- { group: "Firearms", name: "Handgun",      base: 20, pushable: true, firearm: true }
- { group: null,       name: "Cthulhu Mythos", base: 0, noExperienceGain: true, pushable: false }
```

**Never hand-type the full table.** Seed from a maintained open data set (Foundry CoC7 ships
an MIT-licensed `compendiums/en-skills.yaml` — roughly 100 entries with base values and era
tags) or have the deploying Keeper supply one. The catalogue lives as *reference data in a
table*, never an enum baked into code: supplements and eras (Gaslight, Dark Ages, Down
Darker Trails, Regency, Pulp) add and remove skills, so tag rows by era from the start.
Keeper Rulebook ch. 4 and Investigator Handbook ch. 2 hold the authoritative table.

---

## 3. Formula-derived bases

Two core skills compute their base from a characteristic: **Dodge** is half DEX rounded down
(`floor(DEX / 2)`), and **Language (Own)** equals EDU. This is exactly why `baseValue` wants
to be a formula rather than a frozen integer. Edge cases code must rule on:

- When EDU rises (aging checks, Keeper-granted improvement), does Language (Own)'s *base*
  rise too, lifting the total? A live formula says yes; a creation snapshot says no. Foundry
  treats it as live (`keepBaseValue`). The same question applies to Dodge when DEX falls
  through aging or injury. Pick a side, state it, make it configurable — Keepers disagree.
- Either way, recomputation must never eat `experiencePoints`; earned growth survives any
  characteristic swing.
- Rounding is floor, applied to the base alone before components are summed — never to the
  total.

---

## 4. Specialization skills

Seven core-rules groups take specializations: **Art/Craft**, **Fighting**, **Firearms**,
**Language (Other)**, **Pilot**, **Science**, **Survival**. What follows from that:

- Each named specialization is an **independent skill**: its own value, its own tick flag,
  its own improvement roll. Fighting (Brawl) 60% says nothing about Fighting (Sword).
- Base values attach per specialization and are **not uniform within a group**: Fighting
  (Brawl) is 25 while Fighting (Whip) is 5; Firearms (Rifle/Shotgun) is 25 while Firearms
  (Machine Gun) is 10; every Science specialization is 1 except Mathematics at 10.
- The specialization list is **open**. Art/Craft, Language, Pilot, Science, Survival, and
  Lore all accept player-invented names ("Art/Craft (Taxidermy)"). The catalogue must accept
  free-text specialization names, not merely a fixed enum — the rulebook's named entries are
  suggestions.
- The display/canonical name is `Group (Name)`. Store the parts separately and render;
  never parse the string at read time.
- Occupation templates use **placeholder slots** like "Art/Craft (Any)" or "one Firearms
  skill." Model these as unresolved template slots that must bind to a concrete
  specialization before the investigator is playable. Foundry does this via
  `pickNameOnly` / `requiresName` marker items.
- **Fighting (Brawl)** is the default for unarmed and improvised melee — the fallback the
  combat engine reaches for.
- **Throw** is a standalone skill in 7e (base 20), *not* a Firearms or Fighting
  specialization. An easy bug to write.

---

## 5. Earning an improvement check (the tick) during play

The trigger is narrow; encode it precisely.

**Grants a tick:**
- A **successful** skill roll (any level: Regular, Hard, Extreme, Critical) made against a
  difficulty the Keeper actually set. Rolling well isn't enough — the roll must have been
  called for.
- A successful **pushed** roll. The player risked a dire consequence, so the reward stands
  (Keeper Rulebook ~p.84).
- A winning roll in an **opposed** contest.

**Grants no tick:**
- Any failure.
- A success bought by **spending Luck** — fortune, not competence, closed the gap (Keeper
  Rulebook ~p.99).
- A roll made with a **bonus die** (Keeper Rulebook ~p.94). Widely misremembered; ship it
  as a configurable toggle since tables vary and Pulp Cthulhu muddies it further.
- Skills flagged `noExperienceGain` — Cthulhu Mythos and Credit Rating.
- Rolls never made against a stated difficulty (idle/flavour rolls; Keeper-hidden rolls in
  some implementations).

**Flag semantics:**
- **Idempotent** — one tick per skill, not a counter. Ten successful Spot Hidden rolls in a
  scenario still buy exactly one improvement roll. A naive `experienceChecks += 1` is wrong,
  and this is the single most important detail to get right.
- **Durable across sessions** — the flag persists until a development phase consumes it,
  and it is per skill *instance*, hence per specialization.
- If a roll's outcome is revised so it no longer counts as a rolled success (retroactive
  Luck spend, Keeper override), the tick it granted should be withdrawn. Record *which
  roll* set the flag so it can be un-set, and give the Keeper a manual toggle regardless.
- Penalty dice: a success achieved *despite* penalty dice still ticks under RAW. (Foundry
  is stricter and suppresses it — an implementation divergence, not a rule.)

---

## 6. The Investigator Development Phase

Runs at scenario end, or whenever the Keeper calls downtime. For each flagged skill:

```
for skill in investigator.skills where skill.improvementFlag:
    roll = d100                       # 01..100, "00" reads as 100
    if roll > skill.value OR roll >= ALWAYS_IMPROVE:      # ALWAYS_IMPROVE = 95 (see Open Questions)
        gain = d10
        crossedMastery = (skill.value < 90) and (skill.value + gain >= 90)
        skill.experiencePoints += gain
        if crossedMastery:
            investigator.sanity += 2d6                    # clamped to max SAN
    skill.improvementFlag = false     # cleared either way
```

Engine notes:

- The improvement roll targets the **current total value** — with or without temporary
  modifiers is a choice. Recommendation: use the *unmodified* raw value, since transient
  combat/condition modifiers shouldn't warp long-term growth.
- The flag clears on **both** outcomes. A failed improvement roll doesn't bank the tick.
- The `ALWAYS_IMPROVE` clause is the only reason very high skills grow at all; without it a
  99% skill would be frozen.
- `+1d10` yields 1–10, never zero — a successful improvement roll always moves the number.
- **Mastery reward:** crossing 90% *during a development phase* grants `+2d6` current
  Sanity — the self-esteem of mastery. It fires only on the crossing and only via
  development; an investigator *created* at 90%+ gets nothing. Encode as a strict crossing
  (`before < 90 <= after`), never `after >= 90`, or it re-fires on every later improvement.
  The Sanity gain still clamps against maximum Sanity (§7).

Other business the development phase settles (adjacent topics; listed so the backend leaves
room):

- **Sanity recovery / therapy** between scenarios; winding down accumulated insanity
  conditions.
- **Credit Rating** adjustments for employment changes (§8).
- **Cthulhu Mythos** awards the Keeper deferred to downtime (§7).
- **Training and study** (§9).
- **Luck** improvement roll where the table plays with that rule (§10).

Build the phase as one transactional operation that emits an **event log** — a row per skill
with the roll, the gain, and any Sanity award — so a Keeper can audit or undo it. It must be
idempotent per "downtime instance": running twice must not double-improve.

---

## 7. Cthulhu Mythos — the anti-skill

Cthulhu Mythos is mechanically inverted and needs bespoke treatment:

- **Base 0.** Fresh investigators have none absent Keeper dispensation; occupation and
  personal-interest points cannot legally buy it at creation.
- **Never ticked.** No improvement checks, no part in the development-phase loop.
- **Grows by award only** — surviving Mythos encounters, reading Mythos tomes (each tome
  defines a gain for the initial skim and for full study), learning spells, other Keeper
  grants. These are *events*, not rolls against the skill.
- **It caps Sanity.** Maximum Sanity is `99 − CthulhuMythos`, so every Mythos gain lowers
  the ceiling and may push current Sanity down to it. Recompute and clamp Sanity on every
  Mythos change — a cross-entity invariant, not a display nicety.
- Typically **not pushable**.
- Tome study is time-boxed downtime work (skim vs. full study), making tome-derived gains a
  natural development-phase concern even though no improvement roll is involved.

---

## 8. Credit Rating

- Base 0; at creation it takes points from the **occupation pool only**, and each occupation
  defines a legal Credit Rating range creation-time validation should enforce.
- **No improvement checks.** It is never ticked.
- It moves in downtime by Keeper adjudication anchored in fiction: holding a job, a
  promotion, an inheritance raise it; unemployment, ruinous spending, scandal lower it. The
  dice for such adjustments are Keeper-facing guidance in the Keeper Rulebook, not a hard
  formula — surface it as a Keeper-driven adjustment with a reason string and audit trail,
  not an automated roll.
- Range 0–99: 0 destitute, 99 effectively limitless means. Clamp both ends.
- Credit Rating feeds derived values (cash on hand, assets, spending level), so changes
  should emit an event other subsystems can react to.

---

## 9. EDU improvement checks and characteristic growth

The **improvement check** pattern extends past skills: roll `1d100`; strictly above the
current value adds `1d10`. Characteristics use it in two well-defined places:

**Character creation / aging** (Keeper Rulebook ~p.32, Investigator Handbook ~p.44). Older
investigators trade body for education: under 20 cuts STR/SIZ and EDU and rolls Luck twice
keeping the better; 20–39 gets one EDU improvement check; 40–49 gets two plus a
player-allocated STR/CON/DEX reduction and an APP cut; each further decade stacks more
checks and steeper physical/APP reductions, topping out at four checks in the oldest bands.

These checks cap EDU at 99; anything that would overshoot clamps. Because the check is
strictly "roll greater than," EDU 99 improves only on a natural 100 — which clamps anyway.
EDU 99 is terminal.

**During play.** RAW includes no routine EDU improvement in the development phase; the
standard phase grows skills, not characteristics. In practice Keepers grant one after long
stretches of formal study, and the training rules (§9) explicitly cover learning. Treat
post-creation characteristic improvement as a **Keeper-authorized action**: expose an
"improvement check" primitive aimable at any characteristic, fired at the Keeper's
discretion. POW growth (from successful magic use in some campaign styles) is the other
common use of the same primitive.

Consequences for code: EDU changes may cascade into Language (Own)'s base, the Know roll
(`= EDU`), and derived EDU×n values; POW changes cascade into Sanity and Magic Points. Make
characteristic writes events that trigger dependent recomputation.

---

## 10. Training, study, and other downtime growth

Beyond the tick loop, investigators can learn deliberately. Per the Keeper Rulebook's
development-phase guidance: sustained study or formal instruction over a block of downtime
(roughly four months of game time per attempt) permits an improvement check on the studied
skill, with the usual `+1d10` on success. Stacking multiple gains into one block demands
exceptional justification — the intent is to keep a montage from lifting a 5% skill to 60%
inside a year. Self-directed academic study works identically.

Backend shape: a `TrainingAttempt` — skill, downtime block consumed, Keeper approval,
resulting roll — reusing the development phase's improvement-check primitive but gated on
elapsed in-game time instead of a tick flag. Track the in-game date if the cadence is to be
enforced.

**Experience packages** (bulk skill grants for campaigns starting with veteran
investigators, and in Pulp Cthulhu) are a separate mechanism: a lump of points spread across
a defined skill list. Keep them as their own value component so they can be applied and
rolled back independently of earned experience.

---

## 11. Luck

Core 7e makes Luck spendable rather than improvable, but a widely used rule (standard in
Pulp Cthulhu, optional elsewhere) adds a Luck improvement roll to the development phase:
roll `1d100`; above current Luck gains `+1d10`. Pulp variants are more generous (a bigger
gain, plus a consolation gain on failure). Ship as a rule toggle, never hard-coded.

Note the §5 interaction: spending Luck to convert a failure forfeits that skill's tick. The
roll-resolution engine must therefore record *how much Luck was spent* per roll, not merely
the final success level.

---

## 12. Caps and boundary conditions

- **d100 reading:** tens `00` with units `0` reads 100. Results run 1–100; the RNG must
  never emit 0.
- **Skill ceiling:** core rules state no hard cap on ordinary skills, and the always-improve
  clause lets a 99% skill drift past 100. Practical guidance: values above 100 add nothing
  for Regular success but do raise Hard/Extreme thresholds, and a natural 100 still fumbles.
  Recommend a configurable `maxSkillValue` (default 99, optionally unbounded) over an
  unstated assumption.
- **Credit Rating:** hard clamp 0–99.
- **Cthulhu Mythos:** effectively capped at 99 by the Sanity relationship; at 99 the
  investigator's maximum Sanity is 0.
- **Sanity:** `maxSanity = 99 − CthulhuMythos`; clamp on every Sanity write, the 2d6 mastery
  reward included. A mastery reward fully absorbed by the cap is correct behaviour, not a
  bug.
- **EDU:** clamp improvement checks at 99.
- **Creation caps:** an optional rule limits starting skills to 75% (higher in Pulp). If
  offered, it binds at creation only and must not constrain later growth.
- **Mastery crossing:** strict before/after comparison so the 2d6 fires exactly once per
  skill, ever. A persisted per-skill `masteryAwarded` boolean is cheap belt-and-braces
  against replays and manual edits.

---

## Backend implications

**Entities**

- `SkillDefinition` (catalogue / reference data): `id`, `group`, `name`, `baseValue` (int or
  formula string), `pushable`, `noExperienceGain`, `noPersonalAdjustment`, `specialization`,
  `requiresName`, `uncommon`, `fighting`, `firearm`, `eraTags[]`. Versioned, seedable,
  Keeper-extensible.
- `InvestigatorSkill` (instance): `investigatorId`, `definitionId`, `specializationName`
  (nullable), `occupationPoints`, `personalPoints`, `experiencePoints`,
  `experiencePackagePoints`, `improvementFlag`, `improvementFlagSourceRollId`,
  `masteryAwarded`. Derived: `value`, `hardValue`, `extremeValue`.
- `SkillRoll`: the resolution record — target value, dice, bonus/penalty die counts, Luck
  spent, pushed flag, success level, and whether it ticked. This record is what makes tick
  withdrawal possible.
- `DevelopmentPhase`: `investigatorId`, `sessionOrDowntimeId`, timestamp, and
  `DevelopmentResult` rows (`skillId`, `roll`, `improved`, `gain`, `sanityAwarded`).
- `MythosGain` and `CreditRatingAdjustment`: event rows with source, amount, reason.
- `TrainingAttempt`: skill, downtime consumed, approval, resulting improvement check.

**Invariants the rules engine owns**

1. `improvementFlag` is a boolean, never a counter; setting it twice is a no-op.
2. `noExperienceGain` skills can never carry `improvementFlag = true`.
3. A development phase clears every flag it touches, success and failure alike.
4. The development phase writes only `experiencePoints` (plus Sanity on mastery) — never
   base or creation-time pools.
5. `sanity <= 99 - cthulhuMythos` holds after every write to either field.
6. Formula bases recompute on characteristic change; earned experience is never recomputed
   away.
7. Re-running a development phase for the same downtime instance is a no-op.

**Configuration surface (rule toggles for the Keeper)**

`alwaysImproveThreshold` (95 vs. 96), `bonusDiceSuppressTick`, `penaltyDiceSuppressTick`,
`luckSpendSuppressTick`, `maxSkillValue`, `masteryRewardEnabled`, `luckImprovementRule`
(off / core / pulp), `languageOwnBaseIsLive`, `dodgeBaseIsLive`, `startingSkillCap`.

**API shape**

The development phase belongs behind one explicit Keeper-triggered endpoint (`POST
/investigators/{id}/development-phase`, or a party-wide variant) returning the full
per-skill result log — dice included, so players see the rolls. Ticking is a side effect of
roll resolution, with a Keeper-facing manual override endpoint.

---

## Open questions

1. **95 or 96?** Sources split on whether always-improve means "95 or above" (`roll >= 95`,
   Foundry's implementation) or "over 95" (`roll >= 96`). Check Keeper Rulebook p.94 and
   current errata. Ship as a constant, default 95.
2. **Bonus die and the tick.** Several secondary sources cite a RAW rule that a bonus-die
   roll earns no tick, yet many tables play otherwise. Confirm against the rulebook and ship
   as a toggle either way. Penalty dice almost certainly *should* still tick — confirm.
3. **Which value does the improvement roll target** — raw, or including active temporary
   modifiers? Needs a decision; raw is the safer default.
4. **Live vs. frozen formula bases.** Does Language (Own) climb with post-creation EDU, and
   Dodge sink with DEX? Determines whether `baseValue` is stored or computed.
5. **Skill ceiling above 100.** RAW appears silent. Decide: clamp at 99, unbounded, or
   configurable — and what Hard/Extreme mean above 100.
6. **EDU improvement during play.** Verify whether the Keeper Rulebook offers any
   post-creation EDU path beyond the aging table and study rules, or whether it is purely a
   Keeper call.
7. **Credit Rating downtime dice.** Exact promotion/job-loss adjustment dice need rulebook
   verification; secondary sources cite figures we could not confirm.
8. **Tick withdrawal semantics.** When a Keeper retroactively edits a roll's level, or Luck
   is spent after the fact, does the tick come off? Foundry removes it; decide whether the
   tracker follows, and how wide the window is.
9. **Party-wide vs. per-investigator development.** One phase for the whole table (simpler,
   matches play) or per investigator (needed when players miss sessions)? Affects the
   idempotency key.
10. **Skill catalogue licensing.** Ship a skill table, seed from an open-licensed data set,
    or require Keeper import? A product decision with legal weight, not merely engineering.

---

## Sources

**Official / primary**

- *CoC 7e Quick-Start Rules*, Chaosium (free). Skill rolls, difficulty levels, pushing, and
  the improvement-check loop (tick on success; roll over the skill at scenario end for
  `+1d10`) — pp. 11–12.
  https://archive.org/details/cha-23131-call-of-cthulhu-7th-edition-quick-start-rules
- *CoC Keeper Rulebook*, 7th ed. Skill list and descriptions (ch. 4); aging / EDU
  improvement checks (~p. 32); optional starting skill cap (~p. 48); pushing rolls (~p. 84);
  Rewards of Experience and the Investigator Development Phase (~p. 94); Luck spending vs.
  ticks (~p. 99).
- *CoC Investigator Handbook*, 7th ed. Full skill table with base values (ch. 2);
  occupations and skill point formulas; aging table (~p. 44).
- Chaosium's official CoC wiki — "Rewards of Success":
  https://cthulhuwiki.chaosium.com/rules/rewards-of-success.html
- Chaosium's official CoC wiki — "Occupation and Skills" (point allocation, Mythos
  restriction at creation, specialization groups):
  https://cthulhuwiki.chaosium.com/investigators/step-three-occupation-and-skills.html
- Roll20 licensed CoC compendium — Part Three, Occupation and Skills (Dodge = half DEX,
  Language (Own) = EDU, specialization examples):
  https://roll20.net/compendium/coc/Rules:Part%20Three%20Occupation%20and%20Skills

**Reference implementation (verified by reading the code) — Foundry VTT CoC7 system,
Miskatonic Investigative Society, MIT licensed:**
https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT

- `compendiums/en-skills.yaml` — ~100 skills with base values, specializations, era tags,
  property flags.
- `coc7/models/item/skill-system.js` — skill data model; `adjustments` split into
  base/occupation/personal/archetype/experiencePackage/experience.
- `coc7/models/actor/document-class.js` — `developSkills()`: `skillMasteringThreshold = 90`,
  `alwaysSuccessThreshold = 95`, condition `roll > value || roll >= 95`, gain `1d10`, `2d6`
  Sanity on crossing 90, flag cleared either way. Also `developLuck()`.
- `coc7/apps/check.js` — `flagForDevelopment()`, gated on
  `isRolledSuccess = isSuccess && luckSpent === 0 && bonusCount === 0 && penaltyCount === 0`,
  skipped for `noxpgain` skills, and un-flags when a previously ticking roll is revised.

**Secondary / community (cross-check only; unverified against RAW)**

- Morgan Hua, "CoC 7th Ed — Skill Checks Q&A" (cites Keeper Rulebook pp. 84, 94, 99 for tick
  edge cases): https://morganhua.blogspot.com/2020/08/call-of-cthulhu-7th-ed-skill-checks-q.html
- TRPGLine CoC7 rules summary (development phase, study cadence, Credit Rating adjustments):
  https://trpgline.com/rules/coc7/gameSystem
- BRP Central, Chaosium's official forums: https://basicroleplaying.org/
