# Chases (Call of Cthulhu 7th Edition)

## Overview

A 7e chase is a self-contained, turn-based subsystem running on a **linear track of abstract
"locations"**, not a map or grid. Setup is deterministic: every participant makes one **speed
roll** (CON on foot; Drive Auto / Pilot / Ride in or atop a vehicle or animal), nudging their
**MOV** rating by +1/0/−1; the adjusted MOV values then decide (a) whether anyone simply gets
away before the chase begins and (b) how many **movement actions** each participant receives
per round (1 plus the gap between their adjusted MOV and the slowest participant's).
Participants are placed on the track staggered by that same gap, with the pursuit block
starting a fixed number of locations (default 2) behind the quarry block. Each round runs in
DEX order, participants spending movement actions to advance one location at a time;
**hazards** between locations cost time on a failed roll (typically 1D3 movement actions,
plus possible damage) but are always crossed, while **barriers** stop a participant dead
until passed or destroyed. Attacks, spells, and other actions cost movement actions; dodging
and fighting back are free. A chase resolves when the quarry breaks contact, when a pursuer
shares a location with the quarry (dropping into normal combat), or when a
crash/wound/narrative goal ends it. All of this is highly deterministic and maps cleanly to
a server-side state machine — the fuzzy parts (what counts as a location, when line of sight
is lost, how bad a collision is) are explicitly Keeper judgment calls and must be modelled
as Keeper-supplied inputs, never computed.

Primary reference: **CoC 7e Keeper Rulebook, Chases chapter, pp. 132-151**, with the summary
tables on **pp. 414-416**. Page anchors below come from Chaosium's official free index PDF.

---

## 1. The MOV statistic

MOV is derived, never rolled. Derivation (Investigator Handbook / Keeper Rulebook character
creation):

| Condition | MOV |
| --- | --- |
| STR and DEX both below SIZ | 7 |
| Either STR or DEX at least equal to SIZ (all three equal included) | 8 |
| STR and DEX both above SIZ | 9 |

Age erodes MOV: a cumulative −1 per decade from the 40s onward (40s −1, 50s −2, through the
80s). Pulp Cthulhu drops age modifiers altogether.

Engine notes:

- MOV also serves outside chases (distance per combat round, sprint = MOV × 5 units), so it
  lives on the actor, not on the chase.
- Non-human creatures, animals, and vehicles bring their own MOV from stat blocks. Vehicles
  sit on the same scale but far above it — an economy car around MOV 13, a sports car around
  MOV 16 in the rulebook's chase MOV table (Keeper Rulebook p. 146, optional MOV rates). One
  MOV point equals one extra movement action per round, so vehicle-versus-foot chases are
  lopsided by design.
- Inside a chase, MOV always means **adjusted** MOV (below). Store base and adjustment
  separately.

---

## 2. Establishing a chase — the speed roll (p. 132)

Every participant makes exactly one speed roll as the chase begins. The skill depends on the
mode of movement:

| Mode | Speed roll |
| --- | --- |
| On foot / self-propelled | CON |
| Driving a motor vehicle | Drive Auto |
| Boat, aircraft, horse, etc. | The matching Pilot / Ride / Boat skill |

Result mapping — this is the entire rule:

```
extreme success or better (includes a critical 01) -> MOV +1
regular or hard success                            -> MOV +0
failure (including fumble)                         -> MOV -1
```

`adjustedMOV = baseMOV + speedRollAdjustment`, and the adjustment holds **for the whole
chase**. There is no separate fumble tier: a fumbled speed roll is simply a failure for MOV
purposes (the Keeper may layer narrative fallout on top). The speed roll is normally
Regular difficulty, raisable by the Keeper for terrain, injury, load, or vehicle condition.

Edge cases the engine must handle:

- A participant with no relevant skill (a monster without Drive Auto, an NPC without listed
  CON) — the book's monster guidance substitutes the creature's DEX, or a fraction of it
  (half or one fifth) depending on how suited it is to the task (p. 144).
- Switching mode of movement mid-chase (leaping into a car) demands a **fresh speed roll**
  with the new skill, replacing the old adjustment (p. 143).
- Pushing a speed roll makes no sense — there's no time for a second try — so mark chase
  rolls non-pushable by default, with a Keeper override.

---

## 3. Does a chase happen at all? (the escape / exclusion gate)

With all speed rolls in, compare adjusted MOV across the sides:

```
fastestChaser = max(adjustedMOV of all pursuers)
slowestPrey   = min(adjustedMOV of all quarry)

prey.adjustedMOV  > fastestChaser  -> that quarry escapes immediately; no chase for them
chaser.adjustedMOV < slowestPrey   -> that pursuer is outpaced and drops out before the chase
otherwise                          -> the participant is in the chase
```

If *every* quarry escapes, there is no chase and the scene resolves narratively. If only
some escape, only the caught-up ones enter the track. Both gates should be **Keeper
overridable** — the Foundry CoC7 implementation exposes exactly two booleans,
`includeEscaped` and `includeLatecomers`, to force otherwise-excluded participants onto the
track; the backend should mirror that.

Ties matter: escape demands strictly greater MOV. Equal MOV means the chase is on.

---

## 4. Movement actions per round (p. 134)

```
minMOV = min(adjustedMOV over all participants still in the chase)
movementActions(p) = adjustedMOV(p) - minMOV + 1
```

The slowest participant always gets exactly 1, and each MOV point above the slowest buys one
more. One movement action normally advances one location.

Consequences code must respect:

- The value is **relative**, so it must be **recomputed whenever the roster changes** —
  someone joining, dropping out, getting knocked out, or switching vehicles can move
  `minMOV`, and with it everyone's action budget (p. 142, joining a chase).
- A participant may finish a round with a **negative** balance (a deficit) when a hazard or
  barrier charges more actions than remain. The deficit carries: at the next round's start
  the pool refills by the per-round allowance but caps at that allowance —
  `current = min(allowance, current + allowance)`. That single line is the entire carry-over
  rule.
- Movement actions are the universal chase currency: moving, attacking, being cautious,
  assisting an ally, and creating hazards all spend them.

---

## 5. The chase track (p. 133)

- A **location** is a narrative chunk, not a fixed distance — a landing, a doorway, an alley
  mouth, a stretch of road. Two chases can differ wildly in real-world scale.
- Locations sit ordered along a single line. Model as an ordered list, not coordinates.
- **Obstacles occupy the gaps between locations**, not the locations themselves. Each gap
  holds at most one hazard or barrier.
- Any number of participants may share a location.

### "Cut to the chase" and initial placement

The Keeper skips the dull opening stretch: play begins with the lead pursuer a small fixed
gap behind the quarry — **two locations** by default, whatever the fiction said (p. 133).

Within each side, placement staggers by the same MOV gap that drives movement actions:

```
startingGap = 2   (configurable)

index(chaser) = adjustedMOV - min(adjustedMOV of chasers)
index(prey)   = adjustedMOV - min(adjustedMOV of prey) + startingGap [+ keeper offset]
```

Higher index = further along the track. The slowest pursuer anchors the back of the pursuit
block, the slowest quarry sits `startingGap` locations ahead of it, and faster participants
on each side push forward within their own block. This matches the Foundry CoC7
implementation exactly, and it makes the opening layout fully derivable from the speed-roll
results.

### Setup checklist (p. 136, "five steps")

1. Place the pursuers.
2. Place the fleeing characters.
3. Position hazards and barriers between locations.
4. Compute movement actions.
5. Determine DEX order.

---

## 6. The chase round (pp. 133-134)

Turn order is DEX, highest first; ties break toward the higher relevant combat skill. A
participant with a **readied firearm acts at DEX + 50** (the standard CoC initiative rule,
applied here too). Participants may voluntarily delay and slot in later.

On their turn, a participant spends movement actions. Typical costs:

| Action | Cost |
| --- | --- |
| Move one location | 1 movement action |
| Attempt a hazard or barrier | folded into the move; failure costs extra |
| Initiate an attack (Fighting, Firearms, ramming with Drive Auto) | 1 movement action |
| Stop and shoot properly | 1 movement action, no movement that action |
| Cast a spell / pick a lock / other timed act | Keeper-set, usually 1+ movement actions |
| Take a cautious approach (below) | 1 movement action per bonus die |
| Assist an ally facing an obstacle | 1 movement action, grants them a bonus die |
| Dodge, Fighting Back | free |

The round ends once every participant has spent (or forfeited) their movement actions; pools
refill and the next round starts. A practical scheduling rule from the Foundry
implementation: the "active" participant is the highest-initiative one with actions
remaining — which naturally gives faster participants multiple passes.

---

## 7. Hazards (pp. 134-136)

A hazard is something you can take at speed but might pay for: a fence, a crowded market, a
hairpin bend, loose scree.

- The Keeper names the skill and difficulty (Jump, Climb, Drive Auto, DEX, STR…).
- **Success:** move into the next location for the normal 1 movement action, no penalty.
- **Failure:** you still cross — a hazard never stops you — but you lose **1D3 movement
  actions** and may take whatever damage the Keeper attached (the rulebook points at its
  general "other forms of damage" table for falls, scrapes, fire, and the rest).
- Lost actions come out of the current round's pool and can drive it negative (the deficit
  carries).
- Optional rule (pp. 143-144): a participant may **create a hazard** for those behind —
  toppling a fruit stand, slamming a gate. Costs a movement action, a skill roll, or both,
  at Keeper discretion.
- Optional rule (p. 139): **random barriers/hazards** can be rolled instead of pre-placed,
  meaning the track can mutate mid-chase; the data model must permit inserting an obstacle
  during play.

## 8. Barriers (pp. 136-138)

A barrier is a hard stop: a locked door, a wall, a stalled truck across the road.

- **Success on the named roll:** pass into the next location.
- **Failure:** no advance, and movement for the round is over; damage may apply (running or
  driving into the thing). Retry next round.
- Alternatively a barrier can be **destroyed**: barriers carry **hit points**, and attacks
  accumulate damage until the total meets or exceeds them. Illustrative shape of the book's
  table (reworded, not exhaustive): a flimsy interior door in the low single digits, a
  solid exterior door around fifteen, a brick wall in the mid twenties, a mature tree around
  fifty, reinforced concrete around a hundred.
- **Vehicles smashing barriers** deal **1D10 damage per point of the vehicle's Build**, and
  the vehicle takes half of what it dealt, rounded down. That is the link between the Build
  stat and barrier hit points (Build referenced at p. 138 in the chase chapter).

### Cautious approach and assistance

Both obstacle kinds can be approached carefully:

- Spend **1 extra movement action** for **+1 bonus die** on the obstacle roll, capped at
  **2 bonus dice** (2 extra actions). Two tiers in practice: "cautious" and "very cautious."
- An **ally in the same group** may spend 1 of their own movement actions to hand the actor
  a bonus die, under the same cap of 2.
- Bonus dice are consumed by the roll they were bought for; the engine must decide (and
  expose) whether unspent dice persist to the next obstacle.

---

## 9. Conflict during a chase (p. 138)

- **Melee** requires attacker and target in the **same location**. Initiating costs 1
  movement action; the defender's Dodge or Fighting Back response is free.
- **Fighting maneuvers** landed during a chase mirror a failed hazard roll: the victim loses
  **1D3 movement actions**.
- **Firearms** can reach across locations (normal range bands and modifiers apply):
  - Firing while still moving (on foot or in a vehicle) adds **one penalty die** and costs
    nothing beyond the attack.
  - Stopping to shoot properly clears that penalty die but costs 1 movement action and
    forfeits movement for that action.
  - Optional rule (p. 142) details ranged attacks, including shooting out tires: an extra
    penalty die, tires treated as armor 3.
- Optional rule (p. 142): **driver damage** — a wounded driver, and what the vehicle does
  when the driver goes down.
- Optional rule (p. 142): **passengers** — a passenger may act independently, or spend their
  action assisting the driver (a successful Spot Hidden or Navigate lets the next
  acceleration attempt drop one penalty die).

---

## 10. Vehicles

### Driving as the speed roll

Drivers swap Drive Auto (or Pilot/Boat/Ride) in for CON on the speed roll; everything
downstream — adjusted MOV, movement actions, placement — is unchanged.

### Pedal to the Metal (pp. 139-141, optional)

A driver can buy extra distance from a single movement action at the price of penalty dice
on the accompanying Drive Auto roll:

| Locations covered by one movement action | Penalty dice |
| --- | --- |
| 1 (normal) | 0 |
| 2 or 3 | 1 |
| 4 or 5 | 2 |

A vehicle with several movement actions can therefore eat enormous stretches of track in one
round (4 actions × 5 locations = 20 locations) — at real risk. A passenger's successful
assist trims one penalty die from the next acceleration. Failure here is where crashes come
from.

### Build, damage, and collisions (pp. 146-147; summary tables pp. 414-416)

- Vehicles carry a **Build** rating standing in for mass/solidity; a typical passenger car
  is Build 5.
- Collision damage dealt by a vehicle is **one die per point of Build**, die size set by
  impact severity. The book's ladder runs roughly: a glancing knock uses a very small die
  per Build point, a moderate impact about 1D6 per Build point, a severe impact 1D10 per
  Build point, catastrophic impacts multiple D10 per Build point. Occupants each take **one
  die of the same size** (a moderate collision dealing 5D6 to the target deals 1D6 to each
  person inside).
- **Every full 10 points of accumulated damage strips 1 Build.** At half its starting Build
  the vehicle is impaired — a penalty die on rolls made with it. When accumulated damage
  reaches Build × 10 (50 for a standard car), the vehicle is wrecked; occupants are thrown
  clear or worse, taking a large flat roll (on the order of 2D10) or dying outright at the
  Keeper's call.
- Ramming another vehicle is a Drive Auto attack; the target may evade with their own Drive
  Auto.

---

## 11. Optional rules that change the state machine

All at pp. 139-147; each adds a field or a transition:

| Rule | Page | Effect on the model |
| --- | --- | --- |
| Choosing a route | 139 | The track can fork; the leader picks and later participants inherit or diverge. |
| Random barriers | 139 | Obstacles generated during play, not at setup. |
| Pedal to the metal | 139-141 | One movement action buys 1-5 locations at 0-2 penalty dice. |
| Hiding | 141 | Quarry attempts Stealth (Stealth + Drive Auto combined for a vehicle) to break contact and end the chase. |
| Passengers | 142 | Non-driver occupants with their own action economy and an assist option. |
| Driver damage | 142 | Wounds to a driver degrade vehicle control. |
| Ranged attacks | 142 | Detailed firing rules, tire shots (armor 3, extra penalty die). |
| Joining a chase | 142 | Mid-chase joiners; a new pursuer needs MOV at least equal to the slowest quarry; a slower newcomer forces recomputation of everyone's movement actions. |
| Location (optional) | 142 | Finer-grained handling of what a location means. |
| Switching modes of movement | 143 | New speed roll with the new skill; MOV base changes. |
| Creating hazards | 143-144 | Participants can drop obstacles behind them. |
| Monsters in chases | 144 | Skill substitution from DEX (full, half, or one fifth). |
| Fleeing the scene | 144 | Chase machinery reused to answer "did they get clear in N rounds?" — count locations covered. |
| MOV rates | 146 | Table of MOV values for animals and vehicles. |
| Vehicular collision | 146-147 | Severity ladder, Build-scaled damage. |
| Multiple characters | 147 | Group chases; every participant makes their own speed roll. |

---

## 12. How a chase resolves ("cash-out")

There is no single victory condition; the engine wants a set of terminating predicates plus
a Keeper override:

1. **Pre-chase escape** — a quarry's adjusted MOV strictly beats every pursuer's at setup.
2. **Caught** — a pursuer ends in the **same location** as the quarry. This doesn't end the
   chase by itself; it enables melee, grapples, tackles, and surrender, typically handing
   off to the combat subsystem.
3. **Broken contact** — the quarry gets far enough ahead, out of sight, or hides
   successfully (Stealth, or Stealth + Drive Auto for a vehicle). The rules deliberately
   leave "far enough" to the Keeper; no official numeric threshold exists.
4. **Attrition** — a pursuer falls so far behind (or so deep into an action deficit) that
   the Keeper rules them out.
5. **Wreck / incapacitation** — a crash, a major wound, or a knocked-out driver removes a
   participant.
6. **Goal reached** — the quarry hits a defined end location (a safe house, the docks), or
   the "fleeing the scene" clock expires.

Popular house rules like "the chase ends when the MOV gap hits 3" are **not** official —
they come from forum threads reacting to the bookkeeping load. Don't bake them into the core
engine; offer them as configurable variants if at all.

---

## 13. Edge cases the rules engine must decide

- **Negative movement-action pools.** Hazard costs can exceed remaining actions. Carry the
  deficit; refill next round to at most the per-round allowance.
- **Roster changes mid-chase.** Any add/remove moves `minMOV`, which moves everyone's
  allowance, which can change who acts and how often. Recompute lazily; version the result.
- **MOV ties.** Escape demands strictly greater; equal MOV keeps the chase alive.
- **DEX ties.** Break by relevant combat skill, then Keeper decision. Readied firearm =
  DEX + 50.
- **Same-location stacking.** Unlimited co-occupancy; melee eligibility is a
  location-equality test.
- **Obstacles live between locations**, so a "move" is really "traverse edge, then enter
  node." Modelling obstacles as node properties misbehaves for participants who start on
  the far side.
- **Direction.** The track is directional; a participant can be pushed backward (knocked
  back, turned around). Allow negative movement.
- **Bonus die cap.** 2 maximum, regardless of how the dice were bought (cautious approach,
  assists, or both together).
- **Several obstacles in one round.** A fast participant with 4 actions can meet multiple
  obstacles in one turn; resolve each independently.
- **Pushed rolls.** Chase rolls are non-pushable by default.
- **Sides aren't always two.** Three-way chases (A chases B who chases C) break the simple
  chaser/prey boolean; the escape/exclusion gate needs per-pair evaluation or a Keeper
  override.

---

## Backend implications

**Entities**

- `Chase` — id, sessionId, keeperId, status (`setup` | `running` | `resolved`),
  `startingGap` (default 2), `startingOffset`, `includeEscaped`, `includeLatecomers`,
  `isVehicleChase`, `roundNumber`, `activeParticipantId`, `activeLocationId`,
  createdAt/resolvedAt, resolution reason.
- `ChaseLocation` — id, chaseId, `order` (integer position stable under insertion — use a
  sparse or fractional index), `name`, `isSetupLocation` (the pre-track staging squares),
  optional map coordinates, list of occupying participant ids.
- `ChaseObstacle` — id, chaseId, `afterLocationId` (the edge it guards), `kind`
  (`hazard` | `barrier`), `name`, `checkSkill` / `checkCharacteristic`, `difficulty`,
  `failureActionCost` (dice expression, default `1D3` for hazards), `failureDamage` (dice
  expression, nullable), `hitPoints` + `hitPointsRemaining` (barriers only), `destroyed`.
- `ChaseParticipant` — id, chaseId, actorId (nullable — the Keeper must be able to add an
  unstatted NPC on the fly), `side` (`quarry` | `pursuer`), `mode`
  (`foot` | `vehicle` | `mount`), `baseMOV`, `speedRoll` {skill, target, roll,
  successLevel}, `movAdjustment` (−1/0/+1), `adjustedMOV` (derived), `initiativeStat`
  (usually DEX), `hasReadiedFirearm`, `movementActionAllowance` (derived),
  `movementActionsRemaining` (may be negative), `bonusDice` (0-2), `locationId`, `status`
  (`active` | `escaped` | `excluded-too-slow` | `caught` | `out`), `syntheticHp` for
  unstatted NPCs.
- `Vehicle` (referenced by a participant) — `build`, `buildRemaining`, `damageAccumulated`,
  `MOV`, `driverParticipantId`, `passengerParticipantIds`, `impaired` (derived:
  buildRemaining <= build/2), `wrecked` (derived: damageAccumulated >= build * 10).

**Derived values (compute; never store as source of truth)**

```
adjustedMOV            = baseMOV + movAdjustment
minMOV                 = min(adjustedMOV) over participants with status 'active'
movementActionAllowance= adjustedMOV - minMOV + 1
canEscapeAtSetup       = side == quarry && adjustedMOV > max(adjustedMOV of pursuers)
isExcludedAtSetup      = side == pursuer && adjustedMOV < min(adjustedMOV of quarry)
vehicleBuildRemaining  = build - floor(damageAccumulated / 10)
```

**State machine**

`setup` -> (all speed rolls recorded) -> escape/exclusion gate -> `running`
-> per round: refill pools (`min(allowance, remaining + allowance)`), order by
`initiativeStat + (hasReadiedFirearm ? 50 : 0)` desc, activate highest with actions
remaining, resolve actions, advance to next participant, next round -> `resolved` on any
cash-out predicate.

**Events worth emitting** (for replay, undo, and a Keeper log): `SpeedRollMade`,
`ChaseEstablished`, `ParticipantEscaped`, `ParticipantExcluded`, `RoundStarted`,
`ParticipantActivated`, `Moved`, `ObstacleAttempted` (with success level, actions lost,
damage), `ObstacleDestroyed`, `BonusDiceBought`, `AllyAssisted`, `AttackMade`,
`CollisionResolved`, `ParticipantCaught`, `ChaseResolved`.

**API surface implied**

Keeper-only mutations for track editing (insert/remove location, place obstacle,
force-include an escaped or excluded participant, drag a participant anywhere, adjust action
counts). Player-visible reads should probably conceal obstacle difficulties and barrier hit
points until encountered.

**Invariants to enforce**

- `movementActionsRemaining <= movementActionAllowance` at all times.
- `bonusDice` clamped to [0, 2].
- Every roster or MOV change recomputes allowances and re-derives the active participant.
- Obstacles belong to edges; a location holds at most one obstacle per side.
- The speed-roll adjustment is written once per mode of movement; switching modes replaces
  it.

---

## Open questions

1. **Escape threshold during a running chase.** The rules give a hard numeric gate only at
   setup. Invent a configurable "quarry N locations clear → escaped" rule, or require an
   explicit Keeper action? (Recommendation: Keeper action, with an optional auto-suggest.)
2. **Bonus die persistence.** If a participant buys bonus dice and then never faces the
   obstacle that round, do they carry? Foundry exposes a "consume bonus dice" toggle,
   implying it's a judgment call.
3. **Three-or-more-sided chases.** The quarry/pursuer boolean falls short; generalize to
   per-participant "pursuing" targets, or keep two sides and let the Keeper fake the rest?
4. **Exact collision severity table.** The die-size-per-Build ladder must be entered from
   Keeper Rulebook pp. 146-147 / 414-416, not guessed; the summary above is approximate and
   needs book verification before it lands in a table.
5. **Do excluded participants count toward `minMOV`?** The Foundry code takes the minimum
   before applying exclusion flags, quietly inflating everyone's action counts whenever a
   too-slow pursuer is force-included. Decide deliberately; document it.
6. **Vehicle MOV table.** Ship a bundled table of vehicle/animal MOV values (IP-sensitive,
   Keeper Rulebook p. 146), or require Keepers to enter MOV per vehicle?
7. **Hazard damage tables.** Failure damage references the rulebook's general environmental
   damage table; bundle a small preset set, or leave the dice expression free-form?
8. **Integration with the combat subsystem.** When a pursuer shares the quarry's location,
   does the tracker auto-open a combat encounter or run both subsystems side by side?
9. **Pulp Cthulhu variants.** Pulp talents and the absence of age modifiers change MOV;
   confirm whether both rule sets must ship on day one.

---

## Sources

- Chaosium, *Call of Cthulhu 7th Edition Keeper Rulebook* — Chases chapter pp. 132-151,
  summary tables pp. 414-416. Anchors used throughout: Establishing/Speed Roll 132; Location
  and Cut To 133; Round 133-134; Actions and Movement Actions 134; Hazards 134-136; Five
  Steps 136; Barriers 136-138; Barrier HP / Build 138; Conflict 138; optional rules 139-147
  (see the §11 table); worked example 151.
- Chaosium official free index PDF (used to verify page anchors):
  https://www.chaosium.com/content/FreePDFs/CoC%207/Call%20of%20Cthulhu%207th%20Edition%20Index%20for%20Keepers%20and%20Investigators.pdf
- Chaosium, *Call of Cthulhu 7th Edition Quick-Start Rules* (free):
  https://www.chaosium.com/content/FreePDFs/CoC/CHA23131%20Call%20of%20Cthulhu%207th%20Edition%20Quick-Start%20Rules.pdf
- TRPGLine system rules summary, CoC 7e chase chapter: https://trpgline.com/rules/coc7/chase
- Foundry VTT CoC7 system (unofficial reference implementation, Miskatonic Investigative
  Society), https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT — chase
  manual `docs/en/chases.md`; schema `coc7/models/item/chase-system.js`; speed-roll
  adjustment, movement-action formula and too-fast/too-slow gates in
  `coc7/models/chase/participant.js`; obstacle flow (cautious approach, assists, failure
  cost/damage) in `coc7/apps/chat-chase-obstacle.js`; bonus-dice cap of 2 in
  `coc7/apps/dice-pool.js`.
- Morgan Hua, "Call of Cthulhu 7th Ed - Combat Q&A" (vehicle Build, collision damage,
  occupant damage): https://morganhua.blogspot.com/2017/06/call-of-cthulhu-7th-ed-combat-q.html
- Colossal Stories, "Applied Mechanics: Dead Light and the CoC 7th ed. Chase Rules" (worked
  vehicle chase): https://colossalstories.blogspot.com/2019/02/applied-mechanics-dead-light-and-coc.html
- Phil Gamer, "Let's Study CoC 7e Part 3: Chase Rules" (overview):
  https://philgamer.wordpress.com/2018/02/19/lets-study-call-of-cthulhu-7th-edition-part-3-chase-rules/
- Roll20 CoC compendium, Step Two (Generate Characteristics) — MOV derivation table:
  https://roll20.net/compendium/coc/Step%20Two%20(Generate%20Characteristics)
- BRP Central / RPG.net threads on chase-rule friction and house rules (used only to spot
  contested edge cases, never as rules authority):
  https://basicroleplaying.org/topic/11840-simple-change-to-chase-rules/ and
  https://forum.rpg.net/index.php?threads/coc-7e-cutting-the-chase.773025/
