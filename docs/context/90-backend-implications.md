# Backend Implications — Synthesis for the Design Phase

> This document consolidates docs `10`–`21` plus the two `30`s in this folder. It contains no new
> research — it is the combined conclusions of those docs, reorganized for whoever sits down to
> draw the schema. Every mechanic is restated in our own words. When two docs disagree, the
> disagreement is kept visible rather than quietly picked apart — see §D and §E.
>
> Section map: **A** candidate entities · **B** derived-value reference · **C** state machines ·
> **D** design tensions · **E** consolidated open questions.

---

## A. Candidate core entities

Organized by layer. Only fields the *rules* force into existence are listed; routine plumbing
(ids, timestamps, soft-delete) is left out. `←` flags a derived value whose formula lives in §B.

### A.1 Reference / catalogue layer (versioned, era-tagged, licence-scoped)

| Entity | Fields the rules demand |
|---|---|
| `Ruleset` | `family` (COC7 / BRP_UGE / PULP), `version`; holds the formula overrides: `hpDivisor`, `luckDice`, `applyAgePenalties`, `sanMaxBase`, the shape of the success ladder, the rounding strategy, and the Sanity accumulation window |
| `ContentPack` | `license` (ORC / FAN / USER / THIRD_PARTY), `noticeText`, `sourceUrl`, `version`, `isBundled`, `ownerId`, `visibility` (private / campaign / public). Every content row carries an FK to a pack — the pack is both the audit boundary and the takedown kill switch |
| `SkillDefinition` | `group`, `name`, `baseValue` (an integer **or a formula string** — Dodge starts at DEX/2, Language (Own) at EDU), `pushable`, `noExperienceGain`, `noPersonalAdjustment`, `specialization`, `requiresName`, `uncommon`, `fighting`, `firearm`, `keepBaseValue`, `eraTags[]`, `packId` |
| `OccupationTemplate` | `formulaTerms[] {characteristic, multiplier, optional, groupId}`, `creditRating {min, max}`, `skillSlots[] {kind: fixed \| chooseFromGroup \| chooseAny, count, candidates[]}`, `eras[]`, `source` |
| `WeaponDefinition` | `skillPrimary`, `skillAlternate?`, `damage` (dice expression), `dbMode` (none / full / half), `rangeBands[] {maxDistance \| formula, damage?}` (1–3 bands), `usesPerRound {normal, max, burst}`, `magazine?`, `malfunction?`, `blastRadius?`, `properties{}` (ranged, impale, thrown, shotgun, burst, auto, dualBarrel, mounted, stun, burn, silent, rare, maneuver), `priceByEra{}`, `eras[]` |
| `Spell` | `magicPointCost` (expression), `powCost`, `hitPointCost`, `sanityCost` (a fixed automatic value **vs** `X/Y` roll notation — the two mean different things), `castingTime`, `components`, `requiresOpposedPow`, `costList[] {type, condition, config}` |
| `StatBlock` (NPC / creature / GOO subtype) | nullable `characteristics{}`, each `{value, formula?, isAverage}`; `derived{hp, mp, san?, luck?, db, build, mov[]}`, each carrying an `auto` override flag; `attacksPerRound`; `attacks[] {label, skillValue, damageExpression, dbMode, constraints}`; `dodge`; `armor {value?, ruleType, text}`; `spells {chance, countFormula, guaranteed[]}`; `skills[]`; `traits[]`; `sanLoss {onSuccess, onFailure}` |
| `RollTable` | typed results — a single entry must be able to produce narration **or** a structured payload (a phobia/mania trait), and must be able to chain into another table |

### A.2 Actor layer

**`Actor`** — discriminated by `kind ∈ {investigator, npc, creature, vehicle, container}`. A vehicle
is not just a stripped-down character: it carries no characteristics, tracks Build as
current-versus-max because Build erodes, and can have hit locations. Model this as a shared
"combatant" core with a per-type extension.

**`Investigator`**
- `characteristics{str,con,siz,dex,app,int,pow,edu}` — each stored as `{rolled, ageDelta,
  inPlayDelta, current}` so aging can be re-applied and drains reversed independently; `←`
  half/fifth columns per characteristic
- `age`, `ageBand ←`, `ageAdjustments` (the allocation the player actually chose — required to
  replay or undo the aging step)
- `hp {current, max ←, temporaryHp, auto}`
- `mp {current, max ←, temporaryMagicPoints[] {amount, expiresAt}, auto}`
- `san {current, starting, max ←, dailyLoss, dailyLimit, auto}`
- `luck {current}` (capped at 99, no max column)
- `derived {damageBonus, build, movBase, movEffective}` — never treated as authoritative
- `flags {mythosInsanityExperienced, mythosHardened}` — both are sticky and never cleared
- `creditProfile {creditRating, tierId, cash, assets, spendingLevel, manual}`
- `rulesVariant`, `usesFullWoundRules`

**`InvestigatorSkill`** — `definitionId`, `specializationName?`, plus the **adjustments ledger**:
`occupationPoints`, `personalPoints`, `archetypePoints`, `experiencePackagePoints`,
`experiencePoints`. Also `improvementFlag` (a boolean, never a counter),
`improvementFlagSourceRollId`, and `masteryAwarded`. Derived: `value ←`, `hardValue ←`,
`extremeValue ←`.

**`BackstoryEntry`** — `type` (one of the ten categories), `text`, `subject?`, `reason?`, `order`,
`isKeyConnection`, `status ∈ {active, struck, revised}`, `createdBy`, `sourceEventId`. Striking an
entry is a consequence of play, not an edit — the transition belongs to the Keeper.

**`HardeningEntry`** — `{investigatorId, sourceKey, cap, totalLoss}`; `cap` equals the largest
value the source's failure-side loss formula can produce. Decays by 1 at each development phase.

**`Trait`** — phobias and manias: `{name, kind ∈ {phobia, mania}, description, source,
suppressedUntil}`. Persistent; they outlive the episode of insanity that created them.

**`WeaponInstance`** — `definitionId`, `ownerId`, `ammoLoaded`, `ammoReserve`, `readied`,
`jammed {roundsToClear}`, `customOverrides`.

**`KnownSpell`** — `{spellId, casterId, firstCastSucceeded, pushedAndFailed, learnedAt}`.

**`TomeStudyProgress`** — `{tomeSlug, initialReading, progress, necessary, units, fullStudies,
spellsLearned[]}`. A true multi-session state machine that emits a Mythos gain and a Sanity loss
when it completes.

### A.3 Resolution layer

**`CheckRequest`** — `subject {type ∈ characteristic|attribute|skill|item|value, name, id?}`,
`targets[] {ratingRef, difficulty}` (more than one target = a combined roll), `combineMode ∈ {none,
all, any}` **fixed before the dice are thrown**, `difficulty` (where `unknown` is a genuine
sentinel value, not null), `flatDiceModifier` (−2..+2), `flatThresholdModifier`, `bonusCount`,
`penaltyCount`, `allowPush`, `blind`, `concealRequest`, `rollMode`, `requestedBy`, `assignedTo[]`,
`opposedTo?`, `sceneId`, `taskId`, `requestGroupId`, `idempotencyKey`.

**`Roll`** — the persisted aggregate. `naturalRoll`, `effectiveRoll`, `unitDie`, `tensDice[]`,
`candidateTotals[]`, `activeIndex`, `netDiceModifier`, a threshold snapshot `{regular, hard,
extreme}`, `fumbleMin`, `successLevel`, `succeeded`, `isCritical`, `isFumble` (the last two read
off `naturalRoll`), `luckSpent`, `pushCount`, `isForced`/`forcedFrom`, `isRolledSuccess`,
`wasBlindAtFreeze`, `rulesPolicyVersion`, `diceSource ∈ {server, declared}`,
`rngAlgo`/`seedRef`/`drawIndex`, `state`.

**`OpposedContest`** — `attackerRollId`, `defenderRollId`, a snapshot of both ratings (the
tie-break needs it), `tieDirection`, `winner`, `tieBrokenBy ∈ {level, rating, rolloff, impasse}`.
The aggregate that freezes is the *contest*, not either individual roll.

**`PushRecord`** — `originalRollId`, `pushedRollId`, `justification`, `foreshadowedConsequence`,
`consequenceApplied`. Never more than one per `(sceneId, taskId)`.

**`LuckSpend` / `LuckLedgerEntry`** — `rollId?`, `delta`, `reason ∈ {spend, recovery, keeperAward,
pulpSpend:<type>}`, `resultBefore/After`, `levelBefore/After`.

**`PendingDecision`** — `type` (drawn from the ~20-entry catalogue: push consequence, fumble
effect, maneuver legality/effect, bout table result, phobia/mania assignment, SAN source identity,
Mythos award, chase cash-out, lasting injury, …), `blocking`, `requiredRole`, `subjectRefs[]`,
`payload`, `defaultResolution`, `status`, `resolvedBy`, `resultingEventIds[]`.

### A.4 Consequence layer

**`Injury`** — `{actorId, rawDamage, armorApplied, appliedDamage, source, wasMajorWound,
occurredAtGameTime}`. The injury row is what the treatment budget attaches to, and it carries the
one-hour First Aid window. **Never sum damage together before testing thresholds.**

**`Treatment`** — `{injuryId, skill ∈ {firstAid, medicine}, practitionerId, rollResult, wasPushed,
succeeded, hpRestored, appliedAtGameTime}`. Each injury allows at most one *successful* First Aid
and one *successful* Medicine.

**`SanityCheck`** — a persisted, **resumable** aggregate: `sourceKey`, `lossMin`, `lossMax`,
`skipRoll`, `sanRollResult`, `involuntaryActionAcknowledged`, `rolledLoss`, `preHardenedLoss`,
`finalLoss`, `lossApplied`, `intRollResult`, `memoryRepressed`, `boutMode`, `boutDuration`,
`boutResult`, plus the derived flags `{immuneAlreadyInBout, immuneToSource, limitedBySource,
temporaryInsane, indefinitelyInsane, permanentlyInsane, mythosAwarded}`.

**`SanLossEvent`** — `{investigatorId, amount, sourceRef, occurredAtGameTime, rollResult,
windowId}`. Both CoC 7e and ORC Sanity are implemented as windowed aggregation over these rows;
stamp them with game-time, never wall clock. `SanityClamped` (the Mythos ceiling) must be a
**separate event type** from `SanityLossApplied`.

**`CastingAttempt`** — `casterId`, `contributors[] {actorId, magicPoints, hitPoints}`, `spellId`,
`costsPaid`, `castingRollId?`, `pushed`, `outcome ∈ {success, nothingHappens,
successWithDireConsequence}`, `consequenceNote`.

**`MythosGain`**, `CreditRatingAdjustment`, `TrainingAttempt` — plain event rows carrying source,
amount, and reason.

### A.5 Runtime container layer

| Entity | Fields the rules demand |
|---|---|
| `Campaign` | `rulesetId`, `rules: RulesConfig`, `clock: GameClock` |
| `GameClock` | `worldSeconds` (bigint — the only wall-clock-like coordinate), `epochLabel`, `activeEncounterId`, `roundOrdinal` (meaningful only inside an encounter; **not** derivable from seconds) |
| `GameInstant` | `{worldSeconds, encounterId?, roundOrdinal?, anchored: bool}` — stamped onto every timed fact |
| `Scenario` | The unit that the development phase, hardening decay, and the duration of underlying indefinite insanity all key off |
| `Session` | Attendance plus the world-time anchor the Keeper sets when play starts. **Not** a rules unit |
| `Scene` | A location label, a world-time span, and the presence set |
| `SceneParticipant` | `role`, `presence ∈ {present, offstage, hidden, departed}`, `joinedAt`, `leftAt`, `controllerUserId`, `usesFullRules: RuleOptOut {wounds, sanity, scheduler, chase}` |
| `Party` | A named persistent roster; the default target of a group roll is `party.members ∩ present(scene)` |
| `Encounter` | `mode ∈ {combat, chase}` — **switchable in place**, `status`, `roundOrdinal`, `phase ∈ {surprise, normal}`, `turnOrder[]`, `activeIndex`, `startWorldSeconds?`, `elapsedCommittedSeconds?`, `resolutionReason` |
| `EncounterModeSegment` | `{ordinal, mode, fromRound, toRound?, stateRef}` — the audit trail of mode switches |
| `EncounterParticipant` | `initiativeKeyBase`, `readiedFirearm`, `hasActed`, `defensesUsedThisRound`, `defensesAllowed`, `holdState ∈ {none, held, resuming}`, `resumeBefore`, `status`, `forfeitAttackUntilRound`, `aiming {targetId, sinceRound}`, `chase: ChaseParticipantRef?` |
| `ChaseState` | `startingGap` (default 2), `startingOffset`, `includeEscaped`, `includeLatecomers`, `isVehicleChase`, an ordered `ChaseLocation[]`, `ChaseObstacle[]` |
| `ChaseLocation` | `order` (a sparse/fractional index — locations get inserted mid-chase), `name`, `isSetupLocation`, occupants |
| `ChaseObstacle` | `afterLocationId` (an **edge**, not a node), `kind ∈ {hazard, barrier}`, `checkSkill`/`checkCharacteristic`, `difficulty`, `failureActionCost` (default 1D3), `failureDamage?`, `hitPoints`/`hitPointsRemaining`, `destroyed` |
| `ChaseParticipant` | `side`, `mode ∈ {foot, vehicle, mount}`, `baseMOV`, `speedRoll {skill, roll, level}`, `movAdjustment` (−1/0/+1), `adjustedMOV ←`, `movementActionAllowance ←`, `movementActionsRemaining` (can go negative), `bonusDice` (0–2), `locationId`, `status ∈ {active, escaped, excludedTooSlow, caught, out}` |
| `Vehicle` | `build`, `buildRemaining ←`, `damageAccumulated`, `mov[]` (multi-modal), `driverParticipantId`, `passengerParticipantIds[]`, `impaired ←`, `wrecked ←` |
| `Condition` | `key`, `lifetimeClass ∈ {A…F}` (§C.6), `startedAt`, `remainingRounds?`, `expiresAtWorldSeconds?`, `scopeEncounterId?`, `sourceEventId`, `effects: ConditionEffect[]` |
| `PendingCheck` | `scopeType`, `type`, `clock ∈ {world, round}`, `dueAt` (`{worldSeconds}` **or** `{encounterId, roundOrdinal}`), `recurrence`, `priority`, `sourceConditionId` (the cancellation link), `createdByEventId` (the undo link), `status ∈ {pending, resolved, cancelled, superseded}` |
| `DowntimeInstance` + `AppliedStep` | `{startWorldSeconds, endWorldSeconds, status, participantActorIds[]}`; `AppliedStep` unique on `(downtimeId, actorId, stepKey)` — that uniqueness is exactly what makes the development phase idempotent |

### A.6 Authority / audit layer

| Entity | Fields |
|---|---|
| `CampaignMembership` | `(campaignId, userId, role ∈ {KEEPER, CO_KEEPER, PLAYER, SPECTATOR})` — always campaign-scoped, never global |
| `ActorGrant` | `(actorId, userId \| 'default', level ∈ {NONE, LIMITED, OBSERVER, OWNER})` |
| `FieldAuthority` | A static code table `(entityType, fieldPath) → {writer ∈ {player_owner, keeper, engine}, gate ∈ {none, decision, policy_flag}}` |
| `FieldClassification` + `RevealFlag` | `public \| party \| owner \| keeper \| derived(inputs…)`; `RevealFlag {entityId, fieldPath, audience, revealedByEventId}` is the only per-field state that exists at runtime |
| `Event` | `streamId`, `streamSeq` (UNIQUE together with streamId — this doubles as the optimistic-concurrency check), `globalSeq`, `type`, `schemaVersion`, `gameTime` (data, **never** an ordering key), `occurredAt`, `actorUserId`, `actorRole`, `causedByEventId`, `correlationId`, `idempotencyKey`, `rulesPolicyVersion`, `visibility`, `reversible`, `affectedStreams[]`, `payload` |
| `Snapshot` | Pure cache, taken roughly every 200 events per stream; must always be re-derivable |
| `IdempotencyRecord` | `(campaignId, key)` UNIQUE, plus a request fingerprint and the stored response |
| `RulesPolicy` | Versioned per campaign; every roll, damage, and Sanity event records its `rulesPolicyVersion` so a later rule change can never re-grade a frozen roll |
| `ImportJob` | `{userId, packId, format, checksum, acceptedRows, rejectedRows}` |

---

## B. Derived values — one reference table

`//` means integer division (floor) throughout unless a row says otherwise. Every entry here is
**computed, never stored as authority** — except where the "Kind" column reads *seed-only*
(computed once at creation, independent thereafter) or *snapshot* (frozen at a specific event).

| Value | Formula | Inputs / recompute trigger | Kind | Doc |
|---|---|---|---|---|
| Half value | `v // 2` | any change to `v` | live | 10 |
| Fifth value | `v // 5` | any change to `v` | live | 10 |
| `hardTarget` | `T // 2` | effective rating | live | 11 |
| `extremeTarget` | `T // 5`; a threshold of 0 means that tier cannot be reached | effective rating | live | 11 |
| `fumbleMin` | `96` when effective chance `< 50`, otherwise `100` — where "effective chance" is **either** the base rating **or** the difficulty-adjusted target, depending on policy | rating, difficulty, policy | live | 11 |
| d100 read | `100` when `tens==00 && units==0`, otherwise `tens*10 + units`; domain `[1,100]` | — | live | 11 |
| Bonus/penalty pool | `net = bonus − penalty` (1-for-1 cancellation, clamp to ±2); roll `abs(net)+1` tens dice against one shared units die; **materialize every candidate total, then take min (bonus) / max (penalty)** | — | live | 11, 20 |
| Success level | the ordered ladder `FUMBLE < FAILURE < REGULAR < HARD < EXTREME < CRITICAL`; test `01` → critical **before** testing for fumble | roll, thresholds | live | 11 |
| Task succeeded | `achievedLevel >= requiredDifficulty` (a critical clears every bar) | — | live | 11 |
| `hpMax` | `(CON + SIZ) // 10` — divisor becomes **5** under Pulp | CON, SIZ, ruleset | live | 10 |
| `mpMax` | `POW // 5` (floored; authored stat blocks may override) | POW | live | 10, 16 |
| MP regen rate | `1 + (POW // 100)` per hour, capped at `mpMax`; temporary over-cap MP never regenerates | POW | live | 16 |
| `sanMax` | `max(0, 99 − CthulhuMythos)` | Mythos skill | live | 10, 15 |
| `san.current` initial | `= POW` | — | **seed-only** | 10 |
| `san.dailyLimit` | `san.current // 5` | day reset | **snapshot** (or live, per policy) | 15 |
| Damage Bonus / Build | on `sum = STR + SIZ`: `<65 → −2/−2`; `<85 → −1/−1`; `<125 → 0/0`; `<165 → 1D4/+1`; otherwise `n = (sum − 45) // 80` → `nD6` with Build `n+1`. The ladder is open-ended — implement it as a formula, never a lookup table | STR, SIZ | live | 10, 13, 19 |
| `mov.base` | `9` if `DEX > SIZ && STR > SIZ`; `8` if `DEX >= SIZ \|\| STR >= SIZ`; otherwise `7` | STR, DEX, SIZ | live | 10, 17 |
| MOV age penalty | `0` under age 40; from 40 up: `movBase − (age // 10) + 3`, floored at 0 | age | live | 10 |
| Age: EDU checks | `min(4, (age // 10) − 2)` for age ≥ 40; `1` for ages 20–39; `0` for teens | age band | **seed-only** | 10 |
| Age: physical deduction | `5 * 2^((age // 10) − 4)` for age ≥ 40 → 5, 10, 20, 40, 80. **Allocated by the player** across STR/CON/DEX; teens instead split 5 points between STR and SIZ | age band | **seed-only** | 10, 18 |
| Age: APP reduction | `((age // 10) − 3) * 5` for age ≥ 40 → 5, 10, 15, 20, 25 | age band | **seed-only** | 10, 18 |
| Improvement check | `roll = d100`; the skill improves when `roll > currentValue` (or `roll >= ALWAYS_IMPROVE`); gain `+1D10`; clamp at 99. **Strictly sequential, never batched** — every success raises the target for the next check | — | live | 10, 12, 16 |
| Skill `rawValue` | `baseValue(actor) + occupation + personal + archetype + experiencePackage + experience` | any component | live | 12 |
| Dodge base | `DEX // 2` | DEX (live or frozen — policy) | seed-only *or* live | 12 |
| Language (Own) base | `= EDU` | EDU (live or frozen — policy) | seed-only *or* live | 12 |
| Occupation skill points | `Σ (characteristic × multiplier)` over the mandatory terms plus exactly one chosen optional term (in practice `EDU×4` or `EDU×2 + X×2`) | — | **seed-only** | 18 |
| Personal interest points | `INT × 2` | — | **seed-only** | 18 |
| Mastery award | fires on the strict crossing `before < 90 <= after`, and **only inside a development phase**: `+2D6` Sanity, clamped to `sanMax` | — | event | 12 |
| Cash / assets | by CR tier: 1–9 → `CR×1` / `CR×10`; 10–49 → `CR×2` / `CR×50`; 50–89 → `CR×5` / `CR×500`; 90–98 → `CR×20` / `CR×2000`; 0 and 99 are flat amounts. All multipliers scale by era | Credit Rating, era | live | 18 |
| Major wound threshold | one blow, after armour: `2 * appliedDamage >= maxHp` (the integer form of "half of max") | maxHp | live | 13, 14 |
| Instant death | one blow, after armour: `appliedDamage >= maxHp` | maxHp | live | 14 |
| Applied damage | `max(0, rolled − armour)`; armour never reduces poison, asphyxiation, or magical damage | — | live | 14 |
| Impale damage | maximum weapon damage + maximum DB **+ one additional roll of the weapon's damage dice**; a non-impaling weapon gets maximum damage with no extra roll | weapon flags | live | 13, 19 |
| Range → difficulty | `≤ base` → Regular; `≤ 2×` → Hard; `≤ 4×` → Extreme; beyond that → not possible. Difficulty moves the **threshold**; it is never a modifier | distance, baseRange | live | 13, 19 |
| Point-blank | inside `DEX // 5` **feet** → one bonus die | DEX | live | 13, 19 |
| Max volley | `max(3, skill // 10)` | skill | live | 13, 19 |
| Volley hits | on success → `max(1, volleySize // 2)`; on Extreme success → every bullet, the first half as impales | — | live | 13 |
| Volley escalation | volley 2 → 1 penalty die; volley 3 → 2; from volley 4 on, penalty dice stay capped at 2 and the required difficulty steps up once per additional volley | — | live | 13, 19 |
| Maneuver legality | `delta = targetBuild − attackerBuild`; `<=0` → no modifier; `1` → 1 penalty die; `2` → 2; `>=3` → **impossible — reject before any dice are thrown**. Asymmetric: being the bigger party earns no bonus | Build | live | 13, 19 |
| Outnumbering | derived, never stored: once `defensesUsedThisRound >= defensesAllowed`, later **melee** attacks this round gain one bonus die. Never cumulative, never for firearms | per round | live | 13 |
| `adjustedMOV` | `baseMOV + movAdjustment`, where the speed roll maps Extreme+ → `+1`, Regular/Hard → `0`, failure or fumble → `−1`; fixed for the whole chase | speed roll | live | 17 |
| `minMOV` | `min(adjustedMOV)` over the **active** participants | roster change | live | 17 |
| Movement actions | `adjustedMOV − minMOV + 1`; the refill is `min(allowance, remaining + allowance)` so deficits carry forward | roster, MOV | live | 17 |
| Chase escape gate | the quarry escapes iff `adjustedMOV > max(pursuer adjustedMOV)`; a pursuer is excluded iff `adjustedMOV < min(quarry adjustedMOV)`. **Strictly greater** — a tie keeps the chase going | setup | live | 17 |
| Chase placement | `index(chaser) = adjustedMOV − min(chaser MOV)`; `index(prey) = adjustedMOV − min(prey MOV) + startingGap` (default 2) | setup | live | 17 |
| Pedal to the metal | 1 location → 0 penalty dice; 2–3 → 1; 4–5 → 2 | — | live | 17 |
| Vehicle vs barrier | `1D10` damage per point of vehicle Build; the vehicle absorbs half of what it dealt, rounded down | Build | live | 17 |
| Vehicle degradation | `buildRemaining = build − (damageAccumulated // 10)`; impaired once `buildRemaining <= build/2`; wrecked once `damageAccumulated >= build × 10` | damage | live | 17 |
| `awardTick` | `isSuccess ∧ luckSpent==0 ∧ bonusCount==0 ∧ (penaltyCount==0 ∨ policy.penaltyDiceStillTick) ∧ ¬isForced ∧ ¬wasBlind ∧ difficultyWasStated ∧ skill.experienceEligible ∧ subject.kind==investigator` — evaluated **once and only once, at freeze** | frozen roll | **snapshot** | 12, 30-auth |
| `luckRequired{tier}` | the points needed to buy up to each band; offer a band only when the actor can strictly afford it. **Belongs to the projection** — it leaks how badly the roll missed | roll, luck | live | 20, 30-auth |
| Presence set | `{p \| p.leftAt IS NULL ∧ p.presence == 'present'}`; a group Luck roll uses the **lowest current Luck among those present** and snapshots the id set it evaluated | scene membership | live + snapshot | 30-session |
| "Same day" test | `floor(a / 86400) == floor(b / 86400)` against a configurable day boundary — but the **Sanity** day is a Keeper-declared fiction boundary, not this arithmetic | worldSeconds | live | 30-session |

---

## C. State machines the rules imply

### C.1 Wound / health

```
                       single blow: appliedDamage >= maxHp
   HEALTHY ────────────────────────────────────────────────────► DEAD
      │                                                            ▲
      │ single blow: 2*appliedDamage >= maxHp                      │ failed dying CON roll
      ▼                                                            │
   MAJOR WOUND (persistent flag) ──► prone + CON roll              │
      │              └─ fail ──► UNCONSCIOUS                       │
      │                                                            │
      │ currentHp == 0 while majorWound set                        │
      ▼                                                            │
   DYING (unconscious) ──► CON roll at end of the *following*      │
      │                    round, then every round ────────────────┘
      │ successful First Aid
      ▼
   STABILIZED (0 HP + 1 temporary HP)
      │  └─ CON roll each hour; failure → lose temp HP → DYING
      │ successful Medicine
      ▼
   CONVALESCING (majorWound still set) ──► weekly CON recovery rolls
```

Invariants: `majorWound` is set at damage time from the size of one **single** damage instance and
is never re-derived from HP; `hp` never drops below zero; instant death is tested **before** the
major-wound test; only First Aid can interrupt the dying countdown — Medicine only applies after
stabilization; the major-wound flag clears either on an Extreme success on a weekly recovery roll
**or** once HP climbs back to at least half of max, after which the actor returns to healing
1 HP/day. Asphyxiation skips the major-wound gate altogether — it kills at 0 HP no matter what.

### C.2 Insanity

```
 SANE ──► [Sanity check pipeline] ──► loss applied
            │
            ├─ san.current <= 0 ─────────────────► PERMANENTLY INSANE (absorbing; leaves play)
            ├─ dailyLoss >= dailyLimit ──────────► INDEFINITE INSANITY (automatic, no INT roll)
            └─ single loss >= 5 ──► INT roll ──┬─ success ──► TEMPORARY INSANITY
                                               └─ failure ──► memory repressed, nothing happens

 TEMPORARY / INDEFINITE each run the same two phases:
   BOUT OF MADNESS  (1D10 rounds real-time  |  1D10 hours summary)
        · immune to all further Sanity loss while inside
        · d10 table; slots 9 and 10 grant a phobia / mania
        ▼
   UNDERLYING INSANITY  (temporary: 1D10 hours | indefinite: for the scenario / until treated)
        · ANY further loss of 1+ re-triggers a bout (the spiral)
        · reality checks: failure costs 1 SAN → which triggers a bout → loop
```

The pipeline's ordering carries real weight: bout-immunity gate → hardening gate → Sanity roll →
involuntary action → formula selection (a fumble maximizes the failure side) → roll the loss →
clamp against the remaining hardening allowance → Mythos-Hardened halving → apply → threshold
tests **most severe first** → Mythos award. Precedence: permanent short-circuits everything else;
indefinite overrides the INT-gated temporary outcome; an actor already in underlying insanity
skips the 5-point rule entirely. The Mythos ceiling clamp is **not** a loss event.

### C.3 Combat round flow

```
ENCOUNTER: setup ──► running ──► resolved
  per round:
    (1) tick multi-round counters (jam, bout-in-rounds, stun)
    (2) fire round-clock PendingChecks (dying CON, asphyxiation, jam clear)
    (3) resolve aiming (declared last round → consumed this round)
    (4) clear per-round stored flags (hasActed, defensesUsedThisRound, excess-shot penalties)
    (5) recompute derived flags (outnumbering, readied firearm)
    (6) re-derive turn order: DEX desc (+50 while readiedFirearm), tie-break chain
  activation: each participant acts once; surprised participants are skipped and cannot defend
```

Per melee exchange: the attacker rolls Fighting; the defender picks fight back / dodge /
maneuver / nothing. Levels are compared with **tie-direction passed as an explicit parameter** —
a fight-back tie goes to the attacker, a dodge tie to the defender. A defender who wins by
fighting back inflicts *regular damage only* and can never impale. Defending is a free reaction
and does **not** set `hasActed` — that invariant is exactly what lets outnumbering stay derived
instead of stored.

Per shot (firearms): range determines the required difficulty; named modifier contributions are
assembled; the dice are rolled; `result >= malfunction` is tested and **short-circuits** before
any hit resolution; then level versus required difficulty; then damage → armour → the wound
machine.

### C.4 Chase flow

```
setup ──► all speed rolls recorded ──► escape / exclusion gate ──► running ──► resolved
  running, per round:
    refill pools: remaining = min(allowance, remaining + allowance)   # deficits carry
    order by initiativeKey desc (DEX, +50 readied firearm)
    activation policy = "highest-key participant with actions remaining"  # repeats within the round
    spend actions: move 1 location | attempt obstacle | attack | cautious (+1 die per action, cap 2)
                   | assist ally (+1 die, same cap) | create hazard | cast
    hazard  failure → you still cross, lose 1D3 actions (+ optional damage)
    barrier failure → you do NOT advance, movement over for the round; or destroy it (it has HP)
  resolved by: pre-chase escape | caught (same location) | broken contact | attrition
             | wreck / incapacitation | goal reached
```

Roster churn is an **invalidation, never an append**: every join, escape, knockout, or vehicle
swap recomputes `minMOV`, and with it every participant's allowance and the active-participant
derivation. Obstacles live on edges. A move is "traverse the edge, then enter the node".

### C.5 Roll lifecycle (the freeze point)

```
DRAFT ──► ISSUED ──► ROLLED ──► FROZEN
  │          │          │  ▲        │
  │          │          │  └─ PENDING_DECISION (blocking Keeper adjudication)
  │          │          └─ mutations: add/remove dice, spend Luck, push, force result, reveal
  └──────────┴──► VOID ◄────────────────────────┘  (compensating event; dice preserved)
```

Ordering constraints: a dice change is rejected once `luckSpent > 0`; spending Luck locks out
further dice changes and locks out pushing; a push requires `allowPush ∧ ¬isSuccess ∧
pushCount==0 ∧ luckSpent==0` and is never allowed for Luck, Sanity, combat, damage,
Sanity-loss-amount, or opposed rolls. Auto-freeze triggers when a downstream consequence event
references the roll, when the round or scene advances, when the mutation window runs out, or when
the parent contest resolves. **The tick gate runs exactly once, at freeze** — it can never be
evaluated earlier.

### C.6 Condition lifetime classes

| Class | Storage | Members |
|---|---|---|
| **A** derived per round | never stored | outnumbered, firing-into-melee penalty, point-blank, target size/speed dice, melee eligibility (same chase location) |
| **B** round-scoped, stored, reset | wiped by the round hook | `hasActed`, `defensesUsedThisRound`, `readiedFirearm`, `aiming`, excess-shot penalties, `forfeitAttackUntilRound`, chase movement pool (refill-with-carry, not a wipe) |
| **C** multi-round counter | `remainingRounds`, decremented by the round hook | weapon jam (~1D6 rounds), real-time bout of madness (1D10 rounds), dying (open-ended), stunned |
| **D** hour duration | `expiresAtWorldSeconds` | summary bout (1D10 h), temporary-insanity underlying phase (1D10 h), stabilized (hour-ticked), timed spell effects |
| **E** open-ended flag | cleared only by a named rule | `majorWound`, `unconscious`, `prone`, `held/seized`, `indefiniteInsanity`, chase movement deficit, `escaped`/`excluded` |
| **F** permanent | forever; outlives death | phobias, manias, permanent insanity, lasting injuries, `mythosHardened`, `mythosInsanityExperienced`, Pulp talents |

The roll engine exposes a single entry point: `getModifiers(actorId, ctx) → Contribution[]`.
Contributions marked `requiresKeeperConfirm` (phobia and mania triggers — no engine can know the
spider is in the room) are **returned to the caller but never auto-applied**.

### C.7 Other machines worth naming

- **Push:** `PENDING → ROLLED → (SUCCEEDED | FAILED) → PUSHED → (PUSH_SUCCEEDED |
  PUSH_FAILED_CONSEQUENCE_PENDING → RESOLVED)`. The consequence step blocks until the Keeper
  acknowledges it.
- **Aiming:** a two-round machine, `AIMING → committed shot`, cancelled by taking damage or moving.
- **Spell mastery:** `learned → first cast (Hard POW roll) → (mastered | failed)`; a failed first
  cast can be pushed (a failed push still fires the spell but earns a Keeper consequence) or reset
  by relearning (~2D6 weeks plus a Hard INT roll).
- **Creation pipeline:** `generate → apply age (sequential EDU checks) → derive → choose
  occupation → allocate occupation points (incl. mandatory Credit Rating) → allocate personal
  points → backstory → finalize`. The ordering is a hard constraint, not a convention — aging has
  to land before the occupation formula reads EDU.
- **Downtime:** `draft → running → committed → (reopened)`, running `advanceTime` first, then
  Mythos awards (so later Sanity awards fall under the new ceiling), then skill improvement,
  Keeper award, Self-Help, Luck recovery, hardening decay, and Credit Rating.

---

## D. Cross-cutting design tensions

**D.1 Optional rules are not optional to model.** Around forty distinct rules across these docs are
explicitly optional, house-variable, or contradicted between sources: the fumble-threshold basis,
the cooperation implementation, the opposed tie-break, Luck spending and recovery, push-after-fumble,
armour-per-die, outnumbering reset timing, point-blank scope, generalizing the penalty-die cap,
`dailyLimit` snapshot-versus-live, bonus-die tick suppression, and half-value rounding. A
per-campaign `RulesPolicy` document, **versioned**, with every roll and damage event stamped with
its `rulesPolicyVersion`, is not a luxury — without it, changing a rule silently re-grades frozen
history.

**D.2 Pulp is either a seam or a fork, and the docs disagree.** Pulp changes the HP divisor, the
Luck die and recovery cadence, the set of legal Luck spends (including several that mutate HP and
Sanity loss directly instead of touching a die), the age-penalty block, and the healing dice, and
it adds a third skill-point pool plus talents and archetypes. Docs 10 and 16 model it as flags
plus a strategy object; doc 14 argues the divergence may be wide enough to warrant a separate
strategy per subsystem. This must be settled before `RulesPolicy` is designed, because the two
answers give the policy different *shapes*.

**D.3 Enforce versus advise is an API-shape decision, not a setting.** An engine that auto-applies
lethal transitions needs an override path on every transition; an engine that only prompts needs a
prompt object for every rule and a reachable Keeper. The synthesized answer is *both, chosen per
decision type* (`auto` for arithmetic and unambiguous outcomes — major wound, unconscious at 0 HP,
the temporary-insanity threshold; `ask` for narrative ones — push consequence, fumble effect, bout
result, chase cash-out), with one non-negotiable invariant: **posture changes who authors a
decision, never what gets recorded** — an auto-resolved decision still emits the identical
raised/resolved event pair.

**D.4 Licence limits reach into the schema.** CoC 7e is closed IP with no SRD. The fan policy
blesses web calculators and excludes downloadable software; the ORC-licensed BRP engine ships
genuinely reusable content — but with a *different* success ladder and a *different* Sanity
system. The consequences are already reflected in §A: `ContentPack` with a `license` discriminator
on every content row, `Ruleset` as a first-class campaign discriminator (rounding itself is
per-ruleset — BRP rounds half-up where CoC floors), tables for creatures/spells/tomes/weapons/
occupations that ship empty on first boot, per-tenant search indexes so the corpus never gets
publicly reassembled, and a `DELETE /packs/{id}` takedown affordance. Test fixtures are a genuine
leak vector; every fixture uses invented entities.

**D.5 Two live paradigms in one dice engine.** Melee is a true opposed contest (Fighting versus
Fighting/Dodge, with asymmetric tie rules), while firearms are a plain roll against a
range-derived difficulty. Chases layer per-participant action budgets on top. All three must be
served by the single `resolve_check` primitive with orchestrators above it — forking the
resolution code is the single most likely way to ship a divergence.

**D.6 Derived-value visibility is a correctness constraint, not a UI preference.**
`visibility(derived) ≤ min(visibility(inputs))` has to be enforced at build time over the doc-10
dependency graph: `san.max` leaks Cthulhu Mythos, `hp.max` leaks the major-wound threshold,
precomputed success bands leak the exact rating, `fumbleMin` leaks whether the rating sits under
50, and the Luck-spend affordance list leaks *how badly* a hidden roll missed. Build the redacted
view first, then derive affordances from that redacted view.

**D.7 Time is three clocks pretending to be one.** Rounds in CoC 7e are narratively defined and
genuinely not convertible to seconds; hours drive First Aid windows, stabilization, and MP regen;
days and weeks drive healing and recovery; and the Sanity "day" is a *fiction* boundary that must
be a Keeper command and never a timer. The synthesis: two coordinates (`worldSeconds` plus an
encounter-local `roundOrdinal`), each compared only within itself, with `secondsPerRound`
defaulting to 0 and a `commitElapsed` prompt when an encounter closes.

**D.8 Batch versus iterate.** Skipping three weeks has to run three weekly recovery rolls with the
major-wound clear condition re-tested after each one; skipping six months of hourly MP regen for
six investigators is roughly 26,000 events. Closed-form collapse is safe only for branchless
resource regeneration — and it becomes wrong the instant any rule makes regeneration conditional.

**D.9 Undo is a first-class feature, and it cascades.** Keepers reverse mistakes constantly. Dice
are preserved and never re-rolled (re-rolling on undo is an exploit surface). Withdrawing a tick
depends on `tickSourceRollId`; if a development phase has already consumed that tick, the cascade
reaches into the phase itself — which is precisely why the phase must be one transactional,
idempotent operation. A strict causal closure will routinely propose voiding half a session, so
the closure has to be shown for confirmation, never applied silently.

**D.10 Some facts are unrecoverable from later state.** `majorWound` (set from the size of one
single blow), the per-instance damage split (two 4-point hits ≠ one 8-point hit at the same HP
total), the Mythos clamp versus a real Sanity loss, the `dailyLimit` snapshot,
`tickSourceRollId`, the frozen classification inputs, the dice faces and the candidate set, the
Keeper-typed hardening source string, and whether a roll was blind at freeze. Each must be written
into an event at the moment it is true. Everything else should be a pure reduction over the log.

**D.11 The catalogue is user-extensible by construction.** Specialization names are open-ended
(players invent "Art/Craft (Taxidermy)"), era supplements add and remove skills, occupations get
homebrewed, and NPC stat blocks are routinely off-formula. Skills, weapons, spells, tomes, and
manias have to be rows with a per-derived-field `auto` override flag — never columns, never
numbered slots.

**D.12 Interop pressure versus schema purity.** The Dhole's House JSON is the de-facto interchange
format, and its quirks are effectively part of the contract (numbers serialized as strings, one
misspelled key, single-element collections arriving as bare objects, `+DB` as a literal damage
suffix, inconsistent skill naming). Adopting Foundry's content-slug namespace verbatim buys
compendium compatibility at the price of inheriting their subtype taxonomy.

---

## E. Consolidated open questions

Deduplicated across all fourteen docs; a question raised by several docs appears once, merged.
Tags: **[rules]** needs a rulebook check or a table ruling · **[product]** needs a scope or UX
decision · **[legal]** needs a licence or IP decision.

### Dice and resolution

1. **Fumble threshold basis** — the base rating, or the difficulty-adjusted target? Foundry uses
   the adjusted value; a plain reading of the text points at the raw one. The choice materially
   shifts fumble frequency on Hard/Extreme checks. Recommendation: configurable, defaulting to
   difficulty-adjusted. *[rules]*
2. Is a roll of **01 automatically a success** when `extremeTarget` collapses to 0? Critical sits
   at the top of the ladder, so presumably yes — but verify before locking it in. *[rules]*
3. **Push scope key** — how does the backend recognize "the same task" so a repeat attempt is
   forced through push semantics? Keeper-declared task ids versus inferring from `(scene, skill,
   target)`; pure inference will generate false positives. *[product]*
4. **Cooperation / assisted rolls** — a difficulty reduction, a granted bonus die, or subtracting
   the helpers' skills from the difficulty? The sources genuinely conflict. *[rules]*
5. **Opposed rolls with more than two parties** (chases, group grapples) — is N-way support
   needed, and what would the tie rule even be? Both the text and the reference implementation
   assume exactly two sides. *[product]*
6. **Opposed roll resolution model** — (A) both sides roll at Regular and compare tiers, or
   (B) the opponent's rating sets your difficulty (<50 Regular, 50–89 Hard, 90+ Extreme)? Plus the
   tie-break: higher rating wins versus defender wins, and then impasse versus a 1d100 roll-off.
   This materially changes POW-vs-POW hit rates and must be settled before any combat or
   opposed-casting code exists. *[rules]*
7. **Combined rolls with per-target mixed difficulty** — allowed, or must every target share one
   difficulty? Not clearly addressed anywhere. *[rules]*
8. **Luck spend policy** — granularity and ordering (may a player buy up to Hard and then buy
   again to Extreme within one transaction?), reversibility before Keeper confirmation, a per-roll
   cap, and whether the exclusion list (Luck rolls, Sanity rolls, criticals, fumbles,
   malfunctions) is configurable for tables that let players buy away a fumble. *[rules]*
9. **Bonus dice arriving from several sources at once** (equipment + cooperation + circumstance) —
   sum then clamp at 2, or clamp per source? This shapes the whole effect-stacking design. *[rules]*
10. A **deterministic tiebreak when two characters share the lowest Luck** on a group Luck
    roll. *[rules]*
11. **Does the backend roll its own dice, accept externally rolled results, or both?**
    Physical-dice tables need result entry; automated tables need a seeded, auditable RNG. *[product]*

### Skills and improvement

12. **95 or 96** in the always-improve clause — `roll >= 95` or `roll >= 96`? Needs a direct check
    against the rulebook and current errata. *[rules]*
13. **Does rolling with a bonus die forfeit the tick?** Multiple secondary sources present this as
    RAW, yet it is widely played the other way. Penalty dice almost certainly should still tick —
    the reference implementation suppresses both, which reads like an implementation divergence
    rather than a rule. *[rules]*
14. Does the **improvement roll target the raw value** or the value with active temporary
    modifiers included? Raw is the safer default, but it needs an explicit decision. *[rules]*
15. **Live versus frozen formula bases** — should Language (Own) climb when EDU climbs after
    creation, and should Dodge drop when DEX drops through aging or injury? *[rules]*
16. **Is there a hard skill ceiling?** RAW appears silent, and the always-improve clause lets a
    99% skill drift past 100. Clamp at 99, leave unbounded, or make it configurable — and what do
    Hard/Extreme thresholds even mean above 100? *[rules]*
17. Does the rulebook provide any **post-creation path to raising EDU** beyond the aging table and
    the study/training rules, or is it purely the Keeper's call? Sources flatly contradict each
    other. *[rules]*
18. The **Credit Rating downtime dice** for promotion and job loss — secondary sources quote
    specific dice that could not be confirmed against the rulebook. *[rules]*
19. Whether **per-source hardening decay** during the development phase is RAW or a
    reference-implementation design choice. *[rules]*
20. **Tick withdrawal semantics** — when a Keeper retroactively edits a success level, or a player
    spends Luck after the fact, does the granted tick get pulled, and how far back does that
    window reach? *[product]*
21. **Party-wide or per-investigator development phase?** Party-wide matches how tables play;
    per-investigator is needed when players miss sessions. The answer determines the idempotency
    key. *[product]*

### Characteristics, creation and derived values

22. **Characteristic floors under aging** — is the minimum 1 or 0, does 0 mean incapacitated or
    dead, and is a 0-STR investigator a legal sheet at all? The rules lean on common sense; a
    validator needs an actual number. *[rules]*
23. **Aging during play** (multi-year time skips) — re-apply the delta between age bands, and do
    the EDU improvement checks fire a second time? *[rules]*
24. **Do temporary characteristic drains flow into every derived value?** A POW drain should
    clearly lower `mpMax`; by the rules it should *not* lower Sanity — but house rules vary. *[rules]*
25. **Clamping when a max falls below current** (SIZ drops and `hpMax` lands under current HP) —
    clamp down, or leave it? Clamping is the recommendation, but it needs a Keeper-facing UX
    decision. *[product]*
26. **Ages above 89** are absent from the published table — extrapolate the closed forms, or
    require Keeper-entered manual modifiers? *[rules]*
27. **The MOV discrepancy** — some quick-reference sources flatten human MOV to 8. Confirm that
    the full STR/DEX-versus-SIZ comparison is the investigator rule and the flat 8 is NPC
    stat-block shorthand. *[rules]*
28. **Rounding for half and fifth values of odd numbers**, everywhere they occur — one shared
    helper, decided up front (and remember rounding is per-ruleset — see D.4). *[rules]*
29. **Point-buy floors and ceilings** — the 460-point pool and the fixed array are confirmed; the
    per-characteristic caps are not. *[rules]*
30. May **personal interest points push Credit Rating above the occupation's maximum**? Real
    tables read it both ways; this needs a campaign setting with a documented default. *[rules]*
31. **Key connection scope** — must it be a Significant People entry, or can any backstory entry
    (a location, a possession) qualify? Determines whether `isKeyConnection` is type-constrained.
    *[rules]*
32. **Struck backstory entries** — soft-delete via status, or hard removal? And do struck entries
    remain visible to the player? *[product]*
33. Whether **backstory entries are first-class structured records** — Self-Help, key connections,
    and the bout results that corrupt backstory all assume they are. *[product]*
34. Whether the **free quick-start allocation ladder** is supported; if it is, the allocation
    subsystem needs a strategy interface rather than one hardcoded point-pool
    implementation. *[product]*
35. Whether Keepers get a **homebrew occupation authoring UI**, and what validation stops
    degenerate formulas. *[product]*
36. How tightly the **"arcane tomes & spells" backstory entries bind to the tome/spell entities**
    versus staying free text. Full linkage is cleaner but requires the tome subsystem to exist
    first. *[product]*
37. **The scope of era/setting support** (classic 1920s + Modern + Pulp only, or also Gaslight /
    Dark Ages / Down Darker Trails) — this decides whether era is a tag or a full ruleset
    dimension. *[product]*

### Combat, weapons and equipment

38. **Rounding `maxHp/2`** for the major-wound threshold when max HP is odd, and the same question
    for the "healed to at least half maximum" clear condition. Integer arithmetic
    (`2 * damage >= maxHp`) is the recommendation, but it needs to be a documented project
    decision. *[rules]*
39. **Damage exactly equal to max HP** — the official Quick-Start says "greater than or equal to"
    (instant death); several fan summaries say strictly greater. *[rules]*
40. **Half damage bonus and negative DB** — halve the dice (1D4 → 1D2) or halve the rolled total,
    and round which direction? Is a negative DB halved, doubled, or untouched — and does damage
    floor at 0 or at 1? *[rules]*
41. **Extreme success with a non-impaling weapon** — maximum damage, or ordinary damage with no
    bonus at all? Sources disagree. *[rules]*
42. Does the **"2 penalty dice max, then escalate difficulty" cap** (written for automatic fire)
    generalize to every roll, or stay auto-fire-only? *[rules]*
43. **Outnumbering reset timing** — RAW resets per combat round; many tables reset at the start of
    the defender's own turn. Which is our default? *[rules]*
44. **Malfunction plus fumble** — one roll can be both at once. Do both consequences fire, does
    the malfunction short-circuit the fumble, and should a fumbled firearm roll also jam the
    weapon? *[rules]*
45. **Jam clearing** differs by weapon action type (revolver versus autoloader versus belt-fed).
    Which per-weapon fields are needed (`jamClearSkill`, `jamClearRounds`), is a skill roll
    required, and what defaults ship? *[rules]*
46. **Aiming interruption** — "takes damage or moves" is clear enough, but does being targeted by
    an attack that misses, or diving for cover, also break aim? *[rules]*
47. **Point-blank scope** — as written, RAW covers any firearm; an optional rule narrows it to
    handguns and short-barrelled shotguns. Which default? *[rules]*
48. Whether the **multi-shot penalty die hits the first shot too** (sources say yes) and how it
    nets against a point-blank bonus die. *[rules]*
49. **Shotgun ranges beyond the longest damage band** — impossible, or merely Extreme difficulty
    at minimum damage? The damage bands and the difficulty bands are independent axes. *[rules]*
50. **Impale at extreme range**, where only an Extreme success hits at all — does an impale there
    demand a genuine critical? *[rules]*
51. The **"firing into melee" friendly-fire target rule** ("lowest Luck ally") appears only in
    community summaries — confirm before encoding it. *[rules]*
52. **"Attacks per round" semantics** — a hard cap on attack actions, a budget shared with
    movement and maneuvers, or purely advisory? Plus the schema for multi-attack creatures:
    per-attack weapons, per-attack impale/Build flags, and whether the N attacks can split across
    targets. *[rules]*
53. **Initiative-key ties** are unspecified — we need a deterministic default plus a Keeper
    override, and a decision on whether to generalize the chase's "higher relevant combat skill"
    tie-break into combat (and what "relevant" means when the tied participants use different
    skills). *[product]*
54. **How many creature-armour rule types** deserve first-class modelling (flat / halve-by-source
    / cap-by-source / immune-by-source / regenerate) before falling back to free text. *[product]*
55. Whether **weapons and stat blocks carry a ruleset discriminator (Classic vs Pulp)** from day
    one. *[product]*
56. Whether **era is a campaign-level setting or a per-item availability filter**, and whether the
    1890s / 1920s / Modern price columns need to be first-class fields. *[product]*

### Wounds, healing and death

57. **Enforce or advise** — does the backend auto-apply lethal transitions (kill on a failed dying
    roll) or only prompt the Keeper? Enforcement demands an override path on every
    transition. *[product]*
58. What counts as **one "instance of damage"** for the once-per-injury First Aid / Medicine
    budget when several attacks land in the same round — per-attack `Injury` rows, or an
    aggregating wound concept? *[rules]*
59. Does **fresh damage break stabilization**, and if so does the actor revert to dying
    immediately or at the next hourly check? *[rules]*
60. Is **spending Luck on a dying CON roll** legal? CON rolls are absent from the printed
    exclusion list, but this is effectively a table ruling the tracker should surface rather than
    assume. *[rules]*
61. What shape should a **weekly recovery-roll fumble ("lasting injury")** take — a free-text
    Keeper note, a structured characteristic penalty, or an optional expanded-injury
    table? *[product]*
62. Are **NPCs and monsters first-class actors** with full wound state, or a lightweight HP +
    alive/dead record with an opt-in upgrade path? *[product]*
63. Do we need to track **where** a character was wounded? Core 7e has no hit locations, but
    Keepers improvise them, and vehicles have optional locations. *[product]*
64. **The ownership boundary** — how much transient combat state (prone, unconscious, outnumbered,
    aiming, dived-for-cover, held/pinned) belongs to the wound subsystem versus the combat
    subsystem? Prone and unconscious clearly straddle the line. *[product]*

### Sanity

65. **Indefinite-insanity threshold basis and snapshot-versus-live** — is "one fifth" measured
    against Sanity at the start of the game day, against starting Sanity (POW), or against current
    Sanity at the moment of loss? A day-reset snapshot is stable; a live recompute makes the bar
    sink as the investigator bleeds. Pick one, document it, and probably make it
    configurable. *[rules]*
66. **`dailyLimit == 0` for very low-SAN investigators** — at 4 SAN or less, any loss at all
    triggers indefinite insanity. Floor the limit at 1, or accept that (arguably thematically
    correct) behaviour? *[rules]*
67. **What defines a game "day"** for the reset, and what clears `dailyLoss`? Purely
    Keeper-invoked, or a scene/session heuristic like a "party rests safely" event? The same
    question applies to temporary-insanity durations given in real minutes versus combat rounds —
    the duration needs a unit discriminator either way. *[product]*
68. **Hardening source-key normalization** — free text is flexible but typos fragment the
    counters; a controlled creature vocabulary needs a fallback for homebrew monsters. *[product]*
69. Is the **involuntary action** on a failed Sanity roll a blocking checkpoint (two API calls) or
    purely cosmetic (one call)? No mechanical weight either way, but it changes the endpoint
    shape. *[product]*
70. **Who controls an investigator during a bout of madness** — the Keeper, the player with
    prompts, or a per-table setting? This is a *permission* change, so: a time-boxed `ActorGrant`
    override, or nothing more than a UI convention? *[product]*
71. **Multi-actor Sanity checks** — when an entire room witnesses the same horror, does one Keeper
    action fan out one `SanityCheck` per actor, and how is the aggregate presented? *[product]*

### Luck, magic points and spellcasting

72. **MP rounding** for `POW / 5` once POW is not a multiple of 5 — flooring matches how 7e treats
    halves and fifths but is unverified against a printed sheet. *[rules]*
73. **MP regeneration scaling** — 2/hour above POW 100 is solid; the next tier is quoted as both
    POW 200 and POW 300. *[rules]*
74. **Spend order for temporary (over-cap) magic points** — spent first or last? They can be
    spent, never regenerate, and evaporate after a night's sleep. *[rules]*
75. Does a **Luck spend that merely upgrades an already-successful roll** also suppress the
    experience check? The rules only address failure → success. *[rules]*
76. Are **improvement-type checks** (skill improvement, POW improvement, the Luck recovery check
    itself) Luck-spendable? Almost certainly not — but that is inference, not a printed
    rule. *[rules]*
77. Is there a **printed, dice-driven consequence table for a failed pushed casting roll**, or is
    it pure Keeper fiat? This decides whether the backend rolls it or merely prompts. *[rules]*
78. What happens when **POW hits 0** through spell costs or Mythos drain? No clear core rule was
    found. *[rules]*
79. Whether the **"96+ always improves" convention** from skill improvement also governs Luck
    recovery and POW improvement checks. *[rules]*
80. **Luck recovery cadence** — core ties it to the development phase, but tables vary (per
    scenario, per session, never). This wants a scheduling hook, not a hardwired
    trigger. *[product]*
81. The **Pulp "It's Only a Scratch"** healing die is quoted as both 1D3+3 and 1D6. *[rules]*

### Chases

82. **No RAW numeric threshold exists for escaping a chase already in progress** (only at setup) —
    add a configurable "N locations clear = escaped" rule, or require an explicit Keeper
    action? *[rules]*
83. **Do unspent bonus dice carry over to a later obstacle?** The reference implementation exposes
    a "consume bonus dice" toggle, which implies it is a judgment call. *[rules]*
84. **Chases with three or more sides** break the quarry/pursuer boolean — generalize to
    per-participant pursuit targets, or make the Keeper model it manually? *[product]*
85. The exact **vehicular collision severity ladder** (die size per Build point per severity tier)
    and the occupant damage need transcribing from the rulebook — the version in doc 17 is
    approximate and explicitly flagged for verification. *[rules]*
86. Should **excluded (too-slow) participants count toward `minMOV`**? Taking the minimum before
    the exclusion flags are applied inflates everyone's action counts whenever a too-slow pursuer
    is force-included. *[rules]*
87. When a **pursuer reaches the quarry's location**, does the tracker auto-open a combat
    encounter, or run both subsystems side by side? *[product]*

### Time, session and orchestration

88. **The `secondsPerRound` default** — 0 (rounds live outside the wall clock; the Keeper commits
    an elapsed duration when the encounter closes) or 12 (the BRP-derived value)? 0 is faithful to
    CoC 7e; 12 means less Keeper work. Per-campaign or per-encounter? This subsumes the older
    "canonical in-game time representation" question, which the two-coordinate model already
    answers. *[product]*
89. **Split party / parallel scenes** — a per-scene `clockOffsetSeconds` with a Keeper-declared
    sync point, or strictly sequential play with clock rewinds between scenes? The second option
    re-fires every `PendingCheck`. *[product]*
90. **`activeIndex` across a combat↔chase mode switch** — resume with the same participant, or
    restart the turn order? RAW is silent and both are defensible. *[product]*
91. Does a **chase movement-action deficit survive into combat**? No rule exists; the
    recommendation is to drop it. *[rules]*
92. What happens to **round-clock `PendingCheck`s when an encounter ends** with someone still
    dying — auto-convert them to a narrative timer, keep an encounter open, or force explicit
    Keeper resolution? Each answer redefines what "the fight is over" means. *[product]*
93. **Batch versus iterate** for MP regen and daily HP — enforce iterate-only, or permit the
    closed-form collapse behind a registry of branchless check types? *[product]*
94. **The priority ladder for simultaneous checks** — the proposal runs lethal checks last so that
    same-instant healing counts. Is that the table ruling we want, or should the dying CON roll
    fire first? *[product]*
95. **The scenario boundary as a duration** — underlying indefinite insanity lasts "for the
    scenario". Is scenario-close the automatic cancellation trigger, or must the Keeper clear it
    by hand? *[rules]*
96. **Where does `usesFullRules` live** — on the Actor (a property of the monster), on the
    `SceneParticipant` (a property of this particular appearance), or on both with the participant
    winning? *[product]*
97. **Does Session matter mechanically at all?** Nothing in core 7e is per-session (only optional
    Luck-recovery house rules are). If not, is Session a first-class entity or just an attendance
    log? *[product]*

### Authority, visibility and the event log

98. **Is `blind` a property of the request or of the roll?** The tick gate hinges on "was blind at
    freeze", yet a Keeper may reveal mid-window. Recommendation: store both `requestedBlind` and
    `wasBlindAtFreeze` — but does revealing restore the tick? *[product]*
99. Does **`concealRequest` break the tick gate beyond repair**? A secretly rolled Spot Hidden
    success is genuinely earned yet never seen by the player. RAW would tick it; precedent would
    not. *[product]*
100. Will **auto-freeze on scene/round advance surprise Keepers** by silently slamming the
     Luck-spend window shut? The alternative is a warning plus an explicit "close all open rolls"
     action. *[product]*
101. **Sub-streams per actor resource** (`actor:{id}:hp` versus `actor:{id}`)? Field-scoped
     streams cut down on 409s but complicate rewind — every sub-stream has to rewind atomically.
     One actor stream is the v1 recommendation, unvalidated. *[product]*
102. **How far should cascading undo actually reach?** A strict causal closure will routinely
     propose voiding half a session. Is there a "local undo" mode that tolerates incoherence, and
     who is allowed to authorize it? *[product]*
103. **The rewind default policy** — discard (compensate) or retain events past the rewind point?
     Retaining preserves data but leaves timestamps that no longer match the fiction. *[product]*
104. **Policy versioning under undo** — if the Keeper changed a rule between the original event
     and the undo, does the compensating event carry the old policy version (correct) or the
     current one (intuitive)? *[product]*
105. **Co-Keeper limits** — may a Co-Keeper resolve a Mythos award or run a development phase?
     Both are long-lived and effectively irreversible. Keeper-only is the recommendation, but
     two-GM tables will push back. *[product]*
106. **Spectator granularity** — is there a streaming-audience mode that sees even less than
     players (no character sheets at all), and does it need its own classification tier beyond
     `public`? More broadly: how far does field-level visibility go in v1? *[product]*
107. **Retention of voided events** — does a campaign export include voided history, and does
     account deletion tombstone a user's authored events? *[product]* *[legal]*
108. **Cross-campaign investigator portability** — stream ids are campaign-scoped, which turns
     moving a character into an export/import rather than a move. Acceptable? *[product]*
109. **Is commit-reveal RNG worth its UX cost** (an extra round trip plus one more concept to
     teach players), or does a post-hoc HMAC audit token suffice? *[product]*
110. Whether **`ContentPack.visibility` and `ActorGrant` should ever merge** — currently specified
     as an intersection, with the licence check at the write boundary rather than inside the ACL
     resolver. *[product]*

### Interop and prior art

111. **Interchange direction** — importing Dhole's House JSON is clearly worthwhile, but is
     *export* a goal? Exporting Foundry actor JSON is feasible; Roll20 has no import path at
     all. *[product]*
112. Do we **adopt the Foundry CoC ID namespace verbatim** (`i.skill.spot-hidden`) for
     compendium-content compatibility, or mint our own slug scheme? Verbatim adoption eases
     content reuse but drags in their subtype taxonomy. *[product]*
113. Do we need a **general modifier / active-effects layer**? Foundry lets effects override any
     derived value down to individual skill values — powerful, but it complicates every "what is
     this skill actually worth" query. *[product]*
114. **Vehicles and creatures do not fit the character shape** — a shared "combatant" core with
     per-type extensions, or separate tables? *[product]*
115. **Which representation do we serve for success bands** — precomputed inclusive ranges per
     check, or only the Hard/Extreme thresholds? Serving full ranges spares clients from
     re-implementing the rounding, but couples the API to the band model (and interacts with the
     leak in D.6). *[product]*
116. **Is Chase a first-class entity in v1?** The rules are heavy, and a standalone open-source
     chase tracker exists precisely because VTTs handle chases badly. *[product]*
117. **The scope of char-gen extras** — are archetypes, experience packages, and Pulp talents in
     scope? Foundry models them as separate item types, and they add an entire character-creation
     dimension. *[product]*

### Ruleset scope

118. **Is Pulp Cthulhu in scope for v1**, or do we only build the extension seams (`hpDivisor`,
     `luckDice`, `applyAgePenalties`, `sanMaxBase`)? And if it is in scope: a runtime variant of
     one engine, or a separate strategy object? Pulp touches Luck spending and recovery, HP,
     healing dice, Death's Door, Sanity-loss halving, and age penalties, and adds a third point
     pool — divergences possibly broad enough to justify a split. *[product]*
119. Do we support **BRP-UGE as a fully populated, first-class ruleset** (legal, shippable
     content) alongside CoC 7e as an empty shell? Attractive — and it roughly doubles the
     rules-engine surface. *[product]*

### Licensing and IP

120. **The deployment model** — a free hosted service (squarely inside the fan policy) versus a
     self-hostable server (outside the policy's literal wording, though tolerated for Foundry)?
     Must be settled before first release, since it drives packaging, docs, and the whole risk
     posture. *[legal]*
121. Does Chaosium regard a **Keeper-side session/combat/initiative tracker as a "character
     sheet"** under the fan policy's web-calculator allowance? The policy names sheets and
     generators but never trackers. Worth an email to licensing@chaosium.com — Dhole's House got
     its endorsement by asking. *[legal]*
122. **Is a Small Publisher Limited License usable for software at all**, or only for zines,
     props, and PDFs? Ambiguous — and it only matters if revenue is ever intended. *[legal]*
123. **ORC share-alike scope** — if we ship ORC-derived seed data, does rules content we author
     ourselves become ORC-licensed "game mechanics"? Where exactly is the line between source code
     and published game mechanics? *[legal]*
124. **How much seed content ships, and from which corpus** — the CoC skill table (names plus base
     percentages, the Foundry precedent), the weapon tables, the spell list, the vehicle/animal
     MOV table, hazard-damage presets, creature stat blocks. The ORC BRP corpus is reusable
     royalty-free; CoC-specific and Mythos content is not. The options: ship an amber-class
     names-and-numbers pack, seed only from the open corpus, or ship schema plus a text importer
     and make Keepers import everything themselves. *[legal]*
125. **UGC moderation** — once Keepers can share bestiaries campaign-wide or publicly, what stops
     a user uploading an entire Chaosium monster book? Upload heuristics, volume caps, or a review
     gate on public sharing? *[legal]*
126. A **trademark-safe public product name**, and whether a "for use with Call of Cthulhu 7th
     Edition" compatibility statement is worth the exposure compared to a generic "1920s
     investigative horror d100" framing. *[legal]*
127. **Which CoC 7e printing to target** — the 2014 hardcover and the 2023 revised edition are
     paginated differently, so book-pointer strings and any page references surfaced in the UI
     need a printing/edition field, and our own page citations need to commit to one of
     them. *[legal]*
