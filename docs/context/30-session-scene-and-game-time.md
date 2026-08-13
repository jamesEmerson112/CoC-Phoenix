# Session, Scene & Encounter Orchestration + the Unified Game-Time Model

> Context doc for the CoC Phoenix tracker backend. This doc owns the **runtime container**
> that docs 10–19 all presuppose but none defines: how play is structured into campaigns,
> scenarios, sessions, scenes, and encounters, and how in-game time is represented so every
> timed rule in the system fires from one scheduler. All mechanics are restated in our own
> words; page pointers reference the Chaosium *Call of Cthulhu 7th Edition Keeper Rulebook*
> (KRB) for lookup. Rules already specified elsewhere are **cited, not restated** — see docs
> 10–19 in this folder.

## Overview

Every other rules doc in this folder eventually says some flavour of "…at the end of the
round," "…within one hour," "…each week," "…during the development phase" — and leaves the
reader to imagine where a round number or an hour actually lives. This doc pins it down. The
proposal is one **discrete-event timeline** per campaign with two coordinates rather than
three clocks: an integer world-time in in-game seconds (driving the hour / day / week / month
rules) and an **encounter-local round ordinal** (driving the per-round rules), deliberately
*not* convertible to seconds because CoC 7e defines a round narratively rather than
temporally. Around that timeline sit four things: a **presence model** (a scene is a live,
queryable membership set, not a UI list — doc 11's group Luck rule demands it), one
**polymorphic Encounter** hosting both the combat and chase turn loops through pluggable
per-round phases, a **PendingCheck scheduler** every domain enqueues into and that
`advanceTime` drains by iterating each due interval, and a **unified Condition model** with
explicit lifetime classes so the roll engine can ask "what modifies this roll?" once instead
of each subsystem hardcoding its own flags. Finally, **downtime** is a first-class,
idempotent, committable entity, because docs 12, 14, 15, and 16 all write into it.

---

## 1. Entity hierarchy

### 1.1 The ladder

| Level | Persistence | Owns | Notes |
|---|---|---|---|
| **Campaign** | Entity | The authoritative `GameClock`, rules config, content-pack bindings, the actor roster | One clock per campaign. The `Ruleset` discriminator lives here (doc 21). |
| **Scenario** | Entity | A named arc; the unit the development phase and hardening decay key off (docs 12, 15) | Also the duration unit for *underlying indefinite insanity* (doc 15 §6.2). Not necessarily 1:1 with sessions. |
| **Session** | Entity | A real-world sitting: attendance, start/end wall-clock, and the **world-time anchor** the Keeper set at the start of play | A session may span scenarios or stop mid-scenario. Never a rules unit — nothing in RAW is "per session" except optional Luck-recovery house rules. |
| **Scene** | Entity | **Presence**. Location label, world-time span, participant membership with join/leave times | The scope for group rolls, ambient Sanity checks, and "who is here." |
| **Encounter** | Entity | A turn order. `kind ∈ {combat, chase}`, round ordinal, active index, per-round flag scope | An optional child of a scene. A scene may hold zero, one, or several sequential encounters. |
| **Round** | *Value* (an integer on the Encounter) + an event | — | Not a row. Round advancement emits `RoundAdvanced{encounterId, from, to}`. |
| **Turn** | *Value* (`activeIndex` into `turnOrder`) + an event | — | Not a row. §2.1 covers the two conflicting meanings of "turn." |

**Persistence vs. UI grouping.** Campaign, Scenario, Session, Scene, Encounter, and Downtime
are entities. Round and Turn are **derived positions** on the Encounter plus append-only
events — which matters because undo (doc 13, doc 14 §10.12) is dramatically easier when the
round pointer is a projection. "Party" is an entity but not a rung on the ladder (§1.3).

### 1.2 Presence: what "in the scene" means

Doc 11 §9 and Chaosium's own rules wiki both phrase the group Luck rule as: the investigator
with the **lowest current Luck among those present** rolls for everyone. That one sentence
forces scene membership to be live, queryable, server-side state — with a client-side list,
the group can quietly leave the unlucky member out, which is precisely what the rule exists
to prevent.

```
SceneParticipant {
  sceneId, actorId,
  role:      'investigator' | 'npc' | 'monster' | 'vehicle' | 'prop',
  presence:  'present' | 'offstage' | 'hidden' | 'departed',
  joinedAt:  GameInstant,          -- (worldSeconds, encounterRound?) — see §2.2
  leftAt:    GameInstant | null,   -- null == still in the scene
  controllerUserId: string | null, -- who acts for it (matters during a bout of madness, doc 15)
  usesFullRules: RuleOptOut        -- see §1.5
}
```

- **A live query, never a snapshot:** `present(sceneId) = {p | p.leftAt IS NULL AND
  p.presence = 'present'}`.
- **Bitemporal enough to rewind:** with `joinedAt`/`leftAt` as game instants,
  `presentAt(sceneId, t)` is reconstructible — which is what lets a Keeper undo a group roll
  made before someone walked in.
- **`hidden` means present-but-unknown.** A lurking monster is mechanically present (it can
  be outnumbered, an area effect can catch it) but must never appear in player-facing reads.
  `offstage` is the inverse: nominally in the scene but explicitly out of the rolls (asleep
  upstairs, waiting in the car).
- **Every presence-consulting roll snapshots what it saw.** Doc 11 already wants
  `GroupLuckRoll { scene_id, participant_ids[], chosen_actor_id, roll_id }`. Generalize: any
  presence-derived selection persists the id set it observed, so the audit explains *why*
  Harvey rolled.

**Presence scope resolution.** With an encounter running inside a scene, "those present"
turns ambiguous — the scene may hold non-combatants. Rule: default to **scene** presence;
allow the caller to request `scope: 'encounter'`; always let the Keeper edit the member set
before the roll commits. Record the scope used on the roll.

### 1.3 Party

A **Party** is a named, persistent roster of actors inside a campaign — *not* a scene concept
and not a synonym for "the investigators." It exists because (a) split-party play needs two
named groups, and (b) some group rules want "the party" even when one member is elsewhere.

```
Party { id, campaignId, name, memberActorIds[], isDefault }
```

The default target set for a group roll is `party.members ∩ present(scene)`. Splitting the
party creates a second Party and a second active Scene; the clock-divergence problem lives in
§2.6 and Open Questions.

### 1.4 Joining and leaving mid-scene

Both the combat and chase subsystems explicitly anticipate roster churn.

- **Chases** (doc 17 §11, KRB p. 142) have first-class *latecomers* and *escapees*, and the
  Foundry CoC7 chase item mirrors them with `includeLatecomers` / `includeEscaped` booleans.
  A roster change moves `minMOV`, which moves **everyone's** movement-action allowance — so
  joining is an invalidation, not an append.
- **Combat** has no formal join rule; RAW simply slots the newcomer into DEX order.

Model both as one operation on the Encounter, emitting `RosterChanged` with
`{ added[], removed[], reason }`, plus a per-kind `onRosterChanged` hook (chase recomputes
allowances and re-derives the active participant; combat re-sorts `turnOrder` and repairs
`activeIndex` so the current actor keeps their slot).

Leaving is a **status transition, never a delete** — an escaped quarry, an unconscious
combatant, and a dead one are all still rows with history. Deleting them breaks undo and
breaks the outnumbering recomputation for the round they left in.

### 1.5 NPCs, monsters and per-actor rule opt-outs

Doc 14 §10.10 says it plainly: most Keepers drop NPCs at 0 HP rather than run the dying
machinery. Doc 19 adds that Great Old Ones shatter the stat schema entirely. So "does this
actor run the full rules?" must be **per participant, per subsystem**, defaulted from actor
kind and overridable per scene:

```
RuleOptOut {
  wounds:   'full' | 'simple',   -- simple: 0 HP == out of the fight, no major wound / dying / stabilize
  sanity:   'full' | 'none',     -- monsters and most NPCs: none
  scheduler:'full' | 'none',     -- whether PendingChecks are enqueued for this actor at all
  chase:    'full' | 'narrative' -- narrative: Keeper positions it manually, no speed roll / MOV math
}
```

`scheduler: 'none'` is the important one — it is what stops a 30-cultist mob from stuffing
the timer queue with 30 hourly stabilization checks.

---

## 2. The one clock

### 2.1 What a "round" and a "turn" actually are

Nail this down first, because three systems in this project's source material use "turn" for
three different things.

| Term | System | Meaning | Real-time length |
|---|---|---|---|
| **Round** | CoC 7e | Everyone acts once, in DEX order | **Undefined in RAW.** The rules frame it as long enough for each participant to do one meaningful thing; Chaosium's rules wiki uses the same framing. |
| **Turn** | CoC 7e (colloquial) | *One participant's slot* inside a round — "DEX turn order" | n/a — an index, not a duration |
| **Round** | BRP / BRP-UGE | Same idea | **12 seconds**, four fixed phases (statement of intent → movement → actions → resolution) |
| **Turn** | BRP / BRP-UGE | A non-combat time block for movement and slow skill use | **5 minutes = 25 rounds** |
| **Turn** | Foundry VTT | One combatant's activation inside a round | n/a |

Two consequences the engine must respect:

1. **CoC 7e rounds do not convert to seconds.** Any `secondsPerRound` constant is a house
   rule. Silently set it to 12 and let a fight run 40 rounds, and you've shifted the hour
   clock by 8 minutes — enough to matter for the First Aid one-hour window (doc 14 §5) and
   nothing else, which is exactly the near-invisible wrongness that erodes trust in a
   tracker.
2. **The BRP 5-minute turn must never be conflated with a CoC 7e round or day.** Doc 21
   flags this precisely: BRP-UGE's ORC Sanity rules trigger temporary insanity on cumulative
   loss within a **5-minute turn**, while CoC 7e triggers on **≥5 SAN from one roll** and
   indefinite insanity on **≥1/5 of current SAN within a game day**. Different windows,
   different rulesets. If the backend ever ships the ORC Sanity pack alongside CoC 7e
   (doc 21 §3.2), the accumulation window must be a `Ruleset` property, never a constant.

### 2.2 The timeline model

```
GameClock {
  campaignId,
  worldSeconds:  bigint,   -- in-game seconds since a campaign epoch; the ONLY wall-clock coordinate
  epochLabel:    string,   -- e.g. "1925-03-14 21:00" rendered from worldSeconds by a calendar adapter
  activeEncounterId: id | null,
  roundOrdinal:  int | null -- valid only while an encounter is running; NOT derived from worldSeconds
}

GameInstant { worldSeconds: bigint, encounterId: id | null, roundOrdinal: int | null }
```

- **One authoritative clock per campaign.** Sessions and scenes carry `startWorldSeconds` /
  `endWorldSeconds` spans; they never carry clocks of their own.
- **Calendar is a rendering concern.** Store seconds; render dates through a pluggable
  calendar (Gregorian for the 1920s default). "Same day" tests become
  `floor(a/86400) == floor(b/86400)` against a configurable day boundary — never string
  comparison.
- **The round ordinal is orthogonal.** A per-round due check is `dueAt = {encounterId,
  roundOrdinal}`; a per-hour check is `dueAt = {worldSeconds}`. Each compares within its own
  clock; they never cross-convert.
- **Rounds may optionally nudge the world clock.** `campaign.rules.secondsPerRound` defaults
  to `0` ("combat is instantaneous on the hour clock"). Setting 12 gives BRP-style
  behaviour. Either way, closing an encounter offers the Keeper a
  `commitElapsed(seconds)` prompt seeded with `rounds × secondsPerRound` — "that fight took
  about ten minutes" is what actually happened at the table.

### 2.3 Every timed rule in docs 10–19

The full enumeration the rest of the system is specified against. **Clock** is which
coordinate `dueAt` uses. **Cancel** is what removes a queued check without firing it.

| # | Rule | Doc | Clock | Trigger | Interval / due semantics | Cancel |
|---|---|---|---|---|---|---|
| 1 | Dying CON roll | 14 §4 | round | `hp == 0 && majorWound` | First check at the **end of the *following* round** (a one-round grace), then end of every round | Successful First Aid (→ stabilized), death |
| 2 | Asphyxiation CON roll | 14 §7 | round | Cannot breathe | Each round until one fails | Able to breathe |
| 3 | Asphyxiation damage | 14 §7 | round | After the first failed roll (#2) | Each round; bypasses the major-wound gate entirely, death at 0 HP | Able to breathe |
| 4 | Outnumbered bonus die | 13 §8, 14 §8 | round | `defensesUsedThisRound >= defensesAllowed` | **Derived, never stored**; window is the current round | Round advance |
| 5 | Aiming | 13 §5.4 | round | Declared on the shooter's turn | Consumed on the shooter's turn in round *N+1*; no benefit for aiming longer | Shooter takes damage or moves |
| 6 | Readied firearm (`DEX+50`) | 13 §2 | round | Weapon drawn, pointed, shooter has not moved | Re-evaluated every round | Moving, drawing this round, reloading |
| 7 | Weapon jam / malfunction | 13 §5.10, 19 | round | Final result ≥ malfunction number | Clearing attempt once per round; heavier jams ~1D6 rounds | Cleared |
| 8 | Dived for cover | 13 §5.7 | round | Successful dodge into cover | Forfeits next attack — this round if unacted, else next | One-shot |
| 9 | Chase movement pool | 17 §4 | round | Chase round start | `remaining = min(allowance, remaining + allowance)`; deficits carry | Chase ends |
| 10 | Bout of madness (real time) | 15 §6.1 | round | Insanity triggered, Keeper picks real-time mode | **1D10 rounds** | Expiry → underlying insanity |
| 11 | Bout of madness (summary) | 15 §6.1 | hour | Insanity triggered, Keeper picks summary mode | **1D10 hours** | Expiry → underlying insanity |
| 12 | First Aid HP window | 14 §5 | hour | Injury created | Heals 1 HP only if applied within **1 hour** of `injury.occurredAt`; outside the window it still rouses / stabilizes | Injury treated |
| 13 | Medicine minimum work | 14 §5 | hour | Treatment started | Requires ≥1 hour elapsed + supplies — a *precondition*, not a due check | Abandoned |
| 14 | Medicine difficulty by age | 14 §5 | day | Injury age at time of treatment | Same game day → Regular; older → **Hard**. A clock-driven difficulty selector, not a timer | — |
| 15 | Stabilized CON roll | 14 §5 | hour | Successful First Aid on a dying patient | End of that hour and **each hour after**; failure loses the 1 temporary HP and reverts to dying (#1) | Successful Medicine |
| 16 | Magic point regeneration | 16 §7.2 | hour | Always, while `mp < mpMax` | **+1/hour**; 2/hour above POW 100, 3/hour above ~200 (doc 16 flags 200 vs 300 as unresolved). Hard-capped at `POW/5`. Excess/temporary MP never regenerates | `mp == mpMax` |
| 17 | Temporary (over-cap) MP expiry | 16 §7.4 | narrative | Gained above cap | Dissipates after a night's sleep or a Keeper-declared juncture — **a Keeper event, not a timer** | Spent |
| 18 | Temporary insanity duration | 15 §6.2, §10.1 | hour | Temporary insanity begins | **1D10 hours**, or a full night's safe rest; the Keeper may suspend the countdown under sustained tension | Expiry / rest |
| 19 | Underlying indefinite insanity | 15 §6.2 | scenario | Indefinite insanity begins | Runs **for the scenario** or until treated — its length is a scenario boundary, not a duration | Cure (#24, §6) |
| 20 | Natural healing (no major wound) | 14 §6 | day | `hp < maxHp && !majorWound` | **+1 HP/day**, automatic, no roll | `hp == maxHp` |
| 21 | Sanity `dailyLoss` reset | 15 §5.2 | **none** | Keeper declares a new game day | `dailyLoss = 0`, re-snapshot `dailyLimit = floor(san/5)`. **Never a timer** — a "day" here means "until they reach safety and sleep" | — |
| 22 | Weekly recovery CON roll | 14 §6 | week | `majorWound` set | End of each week; fail → 0 HP, regular/hard → 1D3, extreme → 2D3, fumble → lasting injury. Clears `majorWound` on an extreme success or when `hp ≥ maxHp/2` | `majorWound` cleared |
| 23 | Weekly caregiver Medicine roll | 14 §6 | week | A caregiver is attending | Grants a bonus die on #22; a fumble grants a penalty die | Care ends |
| 24 | Monthly indefinite-insanity recovery | 15 §10.2–10.3 | month | Indefinite insanity + a care regime | Each full month; separate d100 bands for private care vs. institutionalization | Cured |
| 25 | Psychotherapy | 15 §10.4 | month | A therapist is engaged | Monthly Psychoanalysis roll; a fumble ends the therapeutic relationship | Relationship ends |
| 26 | Spell relearn | 16 §8.5 | week | Caster elects to relearn instead of pushing | ~**2D6 weeks** of study, then a Hard INT roll; clears the failed-first-cast state | Abandoned |
| 27 | Development phase | 12 §6 | downtime | Keeper declares downtime / scenario end | Per-instance, idempotent (§6) | — |
| 28 | Hardening decay | 15 §8 | downtime | Each development phase | `−1` from every accumulated per-source total | — |
| 29 | Luck recovery (optional) | 16, 11 §9 | downtime | If enabled | One check per downtime instance; cap 99 | Rule disabled |
| 30 | Credit Rating adjustment | 12 §8 | downtime | Employment change | Keeper-narrated, no roll | — |

Rows 4–9 are why §5 exists: all "conditions" in the loose sense, spanning five distinct
lifetimes.

### 2.4 `advanceTime` — a discrete-event loop, not a multiply

Doc 14 §10.9 states the requirement flatly: skipping three weeks runs three weekly recovery
rolls, re-checking the major-wound clear condition after each — never one roll scaled by
three. Daily healing and hourly MP regen behave identically. So `advanceTime` is a bounded
discrete-event simulation:

```
advanceTime(campaignId, targetWorldSeconds):
  assert target >= clock.worldSeconds            # rewind is a separate op (§4.4)
  iterations = 0
  loop:
    next = queue.peekEarliestDue(clock.worldSeconds, target)   # world-clock checks only
    if next is null: break
    if ++iterations > MAX_STEPS: raise NeedsKeeperConfirmation(remaining)
    clock.worldSeconds = next.dueAt.worldSeconds               # move to the event, not past it
    resolve(next)            # may emit rolls, mutate state, enqueue the next occurrence, cancel others
  clock.worldSeconds = target
  emit TimeAdvanced { from, to, resolvedCheckIds[] }
```

Notes:

- **Recurring checks re-enqueue themselves on resolution** (`+1 hour`, `+1 day`,
  `+1 week`), so the loop naturally yields N iterations for an N-interval jump.
- **State re-reads every iteration.** A weekly roll that lifts HP to half of max clears
  `majorWound`, which cancels the remaining weekly checks and enqueues daily healing — that
  only works when the loop re-derives after each step.
- **`MAX_STEPS`** (a few thousand, say) with a resumable continuation, because "advance six
  months" for a party of six with hourly MP regen is ~26,000 events. Batch resource regen is
  the standard optimization: MP and daily HP are closed-form
  (`min(cap, cur + rate × elapsed)`) and collapsible, but **anything that rolls dice or
  branches must iterate**.
- **Rolls inside a jump.** Weekly recovery, monthly insanity, and psychotherapy all roll.
  Either the engine rolls with a seeded auditable RNG, or `advanceTime` returns a batch of
  `RollPrompt` objects and suspends. Doc 14 already asks for prompt objects over inline dice
  — keep that.

`advanceRounds(encounterId, n)` is the sibling on the round coordinate: it drains the
round-clock queue (#1, #2, #3, #7, #10), runs the per-round reset hooks (§3.4), and — when
`secondsPerRound > 0` — advances world time by `n × secondsPerRound`, which may itself
trigger `advanceTime` fan-out.

### 2.5 Rounds without a wall-clock anchor

An encounter opened from a scene without a committed `startWorldSeconds` (very common — the
Keeper just says "roll initiative") has a valid round ordinal and an unknown world time.
Rules:

- Round-clock checks operate normally; they never touch `worldSeconds`.
- `Injury.occurredAt` is a full `GameInstant`. With `worldSeconds` unknown, record
  `worldSeconds = clock.worldSeconds` (last known) and set `anchored: false`.
- Hour-window tests against an unanchored instant are **optimistic**: the First Aid window
  counts as open, and the Medicine "same day" test resolves same-day. This fails safe toward
  the player and matches how tables actually run.
- On encounter close, `commitElapsed` retro-anchors: the encounter's instants get real
  `worldSeconds`, `anchored` flips true, and any hour-window checks that should have expired
  in the elapsed span are re-evaluated. Emit `EncounterTimeCommitted` for the audit.

### 2.6 Parallel scenes

A split party produces two active scenes that are simultaneous in fiction. With one campaign
clock, you cannot advance one without advancing the other. Two workable answers: (a) scenes
carry a `clockOffsetSeconds` and the Keeper declares a sync point on reunion; (b) the Keeper
plays them strictly sequentially, rewinding the clock between (§4.4). Recommendation: (a),
with the offset visible in the UI, because (b) fires every PendingCheck twice. Flagged in
Open Questions.

---

## 3. The shared turn-order engine

### 3.1 One Encounter, two modes

Doc 13 §2 (combat: DEX descending, readied firearm at DEX+50, ties unspecified) and doc 17
§6 (chase: DEX descending, readied firearm at DEX+50, ties broken by the higher relevant
combat skill, movement-action pools, voluntary delay) describe **the same loop with a
different activation policy**. And they don't merely resemble each other — they interleave:
doc 17 §12 hands a pursuer who reaches the quarry's location off to normal combat, and combat
routinely spills back into a chase.

**Decision: one `Encounter` entity whose `mode` changes in place**, plus mode-specific child
state. Not two entities with a hand-off. Rationale: the participant list, the conditions on
those participants, the round counter, and the audit log are all continuous in the fiction;
making them continuous in the data model turns a hard carry-over problem (§3.5) into a
no-op.

```
Encounter {
  id, sceneId, mode: 'combat' | 'chase',
  status: 'setup' | 'running' | 'resolved',
  roundOrdinal: int, phase: 'surprise' | 'normal',
  turnOrder: EncounterParticipantId[],   -- materialized, re-derived on roster/key change
  activeIndex: int,
  startWorldSeconds: bigint | null, elapsedCommittedSeconds: int | null,
  resolutionReason: string | null
}
EncounterModeSegment { encounterId, ordinal, mode, fromRound, toRound|null, stateRef }
```

`stateRef` points at `CombatState` (thin — combat's state mostly lives on participants) or
`ChaseState` (track, locations, obstacles, `startingGap`, `includeEscaped`,
`includeLatecomers` — per doc 17).

### 3.2 Shared parts

Parameterize the loop; never fork it:

```
TurnOrderPolicy {
  initiativeKey(p)   -> number        # both modes: DEX, +50 while readiedFirearm
  tieBreakers[]      -> comparator[]  # ordered; see below
  activation:  'oncePerRound'         # combat: each participant activates exactly once
             | 'whileBudgetRemains'   # chase: highest-initiative participant with actions left
  roundResetHooks[]  -> (participant) => void
  phasePipeline[]    -> PhaseFn[]
}
```

- **`initiativeKey`** is identical across modes: DEX, plus 50 while the readied-firearm flag
  holds. Strictly an *ordering key* — it never mutates DEX (doc 13 §2). Foundry's CoC7
  implements it as a per-combatant `hasGun` flag that recomputes initiative on toggle —
  useful confirmation that "flag, not stat" is the right shape.
- **Ties.** Doc 13 notes RAW is silent; doc 17 gives chases "higher relevant combat skill."
  Generalize the chase rule to both modes and make the chain explicit and configurable:
  `initiativeKey desc → relevant combat skill desc → Luck desc → stable join order`.
  Determinism outranks fidelity here, since RAW offers nothing to be faithful to.
- **The round counter** lives on the Encounter and survives a mode change (§3.5).
- **Surprise round.** Doc 13 §7: a surprised target can neither fight back nor dodge, and
  the attack is automatic or takes a bonus die at Keeper discretion. Model as
  `phase: 'surprise'` with `surprisedParticipantIds[]`; surprised participants are skipped
  in that round's activation loop with `canDefend` false. Normal DEX ordering resumes when
  surprise ends.
- **Delayed / held actions.** Doc 17 explicitly permits voluntary delay; combat RAW is
  silent but every table does it. `participant.holdState ∈ {none, held, resuming}` plus
  `resumeBefore: participantId | null`. Invariant: resuming consumes the held activation —
  delay must never yield two actions in one round.
- **Defending is a reaction.** Doc 13 §2 is explicit, and it is the invariant that makes
  outnumbering derivable: `hasActed` is set **only** by initiating an action; defending
  increments `defensesUsedThisRound` and nothing else. Diving for cover is the documented
  exception (it costs an attack — doc 13 §5.7), modelled as `forfeitAttackUntilRound`.
- **Per-round reset hooks** (§5, class B): `hasActed`, `defensesUsedThisRound`, excess-shot
  penalties, aiming resolution, readied-firearm re-evaluation, chase pool refill. Registered
  by domain, invoked by the engine in a documented order at `RoundAdvanced`.

### 3.3 Mode-specific phase pipelines

| Phase | Combat | Chase |
|---|---|---|
| Round start | Reset per-round flags; re-derive turn order | Refill movement pools `min(allowance, rem + allowance)`; recompute `minMOV` if the roster changed |
| Activation | Next unacted participant in key order | Highest-key participant with actions remaining (repeats within the round) |
| Action | One significant action; defences are free reactions | Spend movement actions; dodging and fighting back are free (doc 17 §6) |
| Round end | Fire round-clock PendingChecks (dying, asphyxiation, jam) | Same, plus resolve deficits |

### 3.4 Reset-hook ordering

Non-obvious and worth fixing in code: run **expiries and decrements before resets**. A jam
counter that hits zero should clear before the round's flags are wiped, so the "weapon is
usable again" event carries the right round number. Suggested order at `RoundAdvanced`:
`(1) tick multi-round counters → (2) fire round-clock due checks → (3) resolve aiming →
(4) clear per-round stored flags → (5) recompute derived flags → (6) re-derive turn order`.

### 3.5 Mode transitions

**Chase → Combat** (doc 17 §12: a pursuer lands in the quarry's location). **Combat →
Chase** (someone breaks and runs). Under the in-place model:

- **Participants:** unchanged rows. Anyone not continuing gets a status transition, never a
  delete.
- **The round ordinal:** continues. Never restart at 1 — the dying CON countdown (#1) is
  mid-flight, and restarting silently resets it.
- **Conditions:** carried by lifetime class (§5) — persistent and multi-round-counter
  conditions travel with their remaining counts intact (a jam doesn't clear just because the
  fiction changed gear); per-round stored flags drop and recompute; derived flags recompute
  by definition.
- **Initiative:** the ordering key survives, but `tieBreakers` and `activation` follow the
  mode, so re-sort. `activeIndex` is the open question: resuming at the same participant is
  kinder; restarting the order is simpler. Recommend resuming, with a Keeper override.
- **Chase state on entry:** every participant needs a speed roll (doc 17 §2), and `minMOV`
  must be computed *after* the roster is final. Entering chase mode from combat is therefore
  a `setup` sub-phase, never an instant switch.
- **Chase state on exit:** the track, locations, obstacles, and movement pools retire with
  the segment (kept for audit, not live). Movement-action deficits do **not** convert into
  any combat penalty — no rule for that exists.

Emit `EncounterModeChanged { encounterId, from, to, atRound, carriedConditionIds[],
droppedFlagKeys[] }`. That event is what a Keeper will point to when something feels wrong.

---

## 4. The PendingCheck / timer queue

Doc 14 sketched `PendingCheck { actorId, type, dueAt, resolved }`. Generalized:

```
PendingCheck {
  id, campaignId,
  actorId,                       -- nullable: some checks are scene- or encounter-scoped
  scopeType: 'actor' | 'encounter' | 'scene' | 'campaign', scopeId,
  type: 'dying' | 'asphyxiation' | 'asphyxiationDamage' | 'stabilization' |
        'naturalHealingDay' | 'weeklyRecovery' | 'mpRegen' | 'boutExpiry' |
        'temporaryInsanityExpiry' | 'monthlyInsanityRecovery' | 'psychotherapy' |
        'spellRelearn' | 'jamClear' | 'aimResolution' | 'custom',
  clock: 'world' | 'round',
  dueAt: { worldSeconds } | { encounterId, roundOrdinal },
  recurrence: null | { every: Duration, clock },   -- self re-enqueues on resolve
  priority: int,                 -- see below
  sourceConditionId: id | null,  -- cancellation link (§4.3)
  createdByEventId,              -- undo link (§4.4)
  status: 'pending' | 'resolved' | 'cancelled' | 'superseded',
  resolvedAt: GameInstant | null, resolutionRollId | null
}
```

### 4.1 Expressing `dueAt` per clock

- World-clock: absolute `worldSeconds`. Hour/day/week/month intervals come from the calendar
  adapter, never a blind `3600 × n` — "end of the week" and "each full month" are calendar
  boundaries in the fiction, even when seconds are the storage.
- Round-clock: `{encounterId, roundOrdinal}`. The dying check's one-round grace (#1) is
  literally `roundOrdinal = enteredAtRound + 1`, then `+1` per recurrence. Bake the grace
  into the *first* enqueue, never the resolver.
- A round-clock check is **inert while its encounter isn't running.** If a fight ends with
  someone still dying, the Keeper either keeps an encounter open or converts to narrative
  resolution. Provide `convertToWorldClock(checkId, seconds)` for exactly this.

### 4.2 Ordering and priority at the same instant

Ties are routine (end of a round with two dying investigators; the hour boundary where a
bout expires and a stabilization check comes due). A fixed ladder, lowest number first:

| Priority | Class | Rationale |
|---|---|---|
| 10 | Condition expiry / counter decrement | Expiries land first so their cancellations propagate before anything reads state |
| 20 | Resource regeneration (MP, daily HP) | Rolls made this instant should see the healed value |
| 30 | Recovery / improvement rolls (weekly CON, monthly insanity, psychotherapy) | — |
| 40 | Survival checks (hourly stabilization) | — |
| 50 | Lethal checks (dying CON, asphyxiation damage) | Last, so healing landing on the same instant is honoured — charitable, and matching table practice |

Within a priority, order by `(actorId, checkId)` for determinism. Persist the resolution
order in the event log; never rely on database iteration order.

### 4.3 Cancellation

Every check that exists *because of* a condition carries `sourceConditionId`. Ending the
condition cancels its checks in one statement — successful First Aid ends `dying`, which
cancels the per-round CON checks and enqueues the hourly stabilization ones. Cancellation is
a status transition (`cancelled`), never a delete, so the log shows the check that *didn't*
fire.

Further cancellation triggers worth a named rule: actor death (cancel everything except
post-mortem bookkeeping), an actor leaving the campaign, encounter resolution (cancel or
convert round-clock checks), and `majorWound` clearing (cancels weekly recovery, enqueues
daily healing).

### 4.4 Rewind and undo

Keepers reverse mistakes constantly (doc 14 §10.12). Two distinct operations:

- **`undoEvent(eventId)`** — compensates one event. Every check with
  `createdByEventId = eventId` is cancelled; every check *resolved* by a descendant of that
  event returns to `pending` with its resolution effects compensated. This is why the queue
  should be a projection over the append-only event log rather than free-standing mutable
  rows: reverting is "re-fold the log," never "guess what to unwind."
- **`rewindClock(to)`** — moves `worldSeconds` backwards. Explicit and loud. Policy: every
  event with `occurredAt > to` is presented to the Keeper with a choice of *discard*
  (compensate) or *retain* (keep the effects, move the timestamps). Checks resolved after
  `to` return to `pending` with their original `dueAt`; checks created after `to` are
  cancelled. Emit `TimeRewound { from, to, policy }`.

Round-clock rewind is the same operation scoped to an encounter (`rewindRound`) — and far
more common in practice.

---

## 5. The unified condition / effect model

Conditions are currently scattered piecemeal across docs 13, 14, 15, 17, and 19. One
taxonomy, keyed by **lifetime class**, because lifetime dictates storage:

| Class | Storage rule | Members |
|---|---|---|
| **A. Derived per round** | **Never stored.** Recompute from other state on demand | Outnumbered (`defensesUsedThisRound >= defensesAllowed`), firing-into-melee penalty, point-blank, target size/speed dice, melee eligibility (same chase location) |
| **B. Round-scoped, stored, reset** | Stored; cleared by a round-reset hook | `hasActed`, `defensesUsedThisRound`, `readiedFirearm`, `aiming{targetId, sinceRound}`, excess-shot penalty count, `forfeitAttackUntilRound` (dived for cover), chase `movementActionsRemaining` (reset is a *refill with carry*, never a wipe) |
| **C. Multi-round counter** | Stored with `remainingRounds`; decremented by the round hook | Weapon jam (~1D6 rounds), bout of madness in real-time mode (1D10 rounds), dying (round-ticked, open-ended), stunned |
| **D. Duration in hours** | Stored with `expiresAtWorldSeconds` | Bout of madness in summary mode (1D10 h), temporary insanity underlying phase (1D10 h), stabilized (hour-ticked, open-ended until Medicine), timed spell effects |
| **E. Open-ended flag** | Stored; cleared only by a named rule | `majorWound`, `unconscious`, `prone`, `held/seized`, `indefiniteInsanity`, chase movement deficit, `escaped` / `excluded` chase statuses |
| **F. Permanent** | Stored forever; survives death for the record | Phobias, manias, permanent insanity, lasting injuries from a recovery-roll fumble, `mythosHardened`, `mythosInsanityExperienced`, Pulp talents |

Two structural notes. First, **the Foundry CoC7 `status` item historically carried
`duration { permanent, hours, minutes, rounds }`, and its actor code writes a bout of madness
as either `duration.rounds = n` (real-time) or `duration.seconds = n × 3600` (summary)** —
independent confirmation that a duration needs a *unit discriminator*, and that rounds and
seconds are separate axes rather than one convertible scale. Second, that system's condition
set (`tempoInsane`, `indefInsane`, `unconscious`, `criticalWounds`, `dying`, `prone`,
`dead`) is far smaller than the list above, because it shoves the rest into ad-hoc flags.
Don't repeat that.

### 5.1 Schema and mechanical payload

```
Condition {
  id, actorId, key,                       -- e.g. 'prone', 'aiming', 'boutOfMadness'
  lifetimeClass: 'A'|'B'|'C'|'D'|'E'|'F',
  startedAt: GameInstant,
  remainingRounds: int | null,            -- class C
  expiresAtWorldSeconds: bigint | null,   -- class D
  scopeEncounterId: id | null,            -- class B/C are usually encounter-scoped
  sourceEventId, sourceActorId | null,
  effects: ConditionEffect[],
  keeperNote: string | null
}

ConditionEffect =
  | { kind: 'diceMod',       sign: 'bonus'|'penalty', count: int, selector: RollSelector }
  | { kind: 'difficulty',    steps: int,                          selector: RollSelector }
  | { kind: 'prohibit',      actionKinds: string[] }              -- unconscious/dying: everything
  | { kind: 'defenseDenied', defenses: ('fightBack'|'dodge')[] }  -- surprised
  | { kind: 'gate',          blocks: 'sanityLoss'|'healing'|'improvement' }  -- bout of madness
  | { kind: 'resource',      field: 'temporaryHp'|'movementActions', delta: int }
  | { kind: 'note',          text: string }

RollSelector {
  rollKinds?: ('skill'|'characteristic'|'combat'|'sanity'|'luck'|'damage'|'opposed')[],
  skillKeys?: string[],
  role?: 'attacker' | 'defender' | 'any',
  weaponClass?: 'melee' | 'ranged' | 'any',
  requiresKeeperConfirm?: boolean         -- phobia/mania: the engine can't know the trigger is present
}
```

The selector shape mirrors the Foundry CoC7 `DICE_POOL_REASONS` registry, which tags each
modifier with `ifAttacker / ifDefender / forMelee / forRanged / forBonus / forPenalty` — a
good sign that a declarative registry beats per-subsystem conditionals.

### 5.2 The one query the roll engine makes

```
getModifiers(actorId, ctx: RollContext) -> Contribution[]
  # ctx: { rollKind, skillKey, role, weaponClass, targetActorId?, encounterId? }
  # returns [{ source, sign, count, difficultyShift, requiresConfirm, conditionId }]
```

Everything — conditions (classes B–F), derived flags (class A), phobia/mania penalties
(doc 15 §9), Pulp talents, held/seized penalties (doc 13 §4), chase cautious-approach bonus
dice (doc 17 §8) — answers through this single function. Doc 11 §4 already requires
modifiers collapse to one signed integer before rolling, and doc 13 §10 already wants named
contributions for explainability. Both requirements are satisfied here, once.

Two rules the query enforces: `requiresKeeperConfirm` contributions are **returned but not
applied** until confirmed (the engine cannot know whether the spider is in the room), and
the bonus-die cap belongs to the *caller's* policy, not here (chases cap at 2 — doc 17 §8;
combat's general cap is an open question in doc 13).

---

## 6. Downtime as a first-class phase

Docs 12, 14, 15, and 16 all write into "between scenarios," so it must be an entity, not a
UI mode.

```
DowntimeInstance {
  id, campaignId, scenarioId | null, label,
  startWorldSeconds, endWorldSeconds,       -- the Δ that advanceTime will traverse
  status: 'draft' | 'running' | 'committed' | 'reopened',
  participantActorIds[]
}
AppliedStep {
  downtimeId, actorId, stepKey,             -- UNIQUE (downtimeId, actorId, stepKey)
  payload, rollIds[], appliedAt
}
```

### 6.1 Ordering

Doc 12 §6 already demands a transactional, auditable phase. The dependency graph fixes most
of the order; where it doesn't, pick and document:

1. **`advanceTime(startWorldSeconds → endWorldSeconds)`** — this is what actually fires the
   weekly healing batches (#22/#23), hourly MP regen (#16), daily HP (#20), monthly insanity
   recovery (#24), psychotherapy (#25), and spell-relearn completion (#26). Downtime is
   *the clock jump plus the between-scenario steps*, in that order.
2. **Deferred Cthulhu Mythos awards** (doc 12 §7) → recompute `san.max = 99 − mythos` →
   clamp `san.value` silently. **Before** any SAN award, so awards land under the new
   ceiling rather than being eaten by it.
3. **Skill improvement rolls** (doc 12 §6) — per flagged skill: roll, `+1D10` on success,
   clear the flag either way. Detect 90% threshold *crossings* and award `+2D6` SAN each
   (doc 12 §6, doc 15 §10.4), clamped to `san.max`.
4. **The Keeper's SAN award** for surviving the scenario (doc 15 §10.4).
5. **Self-Help / key connection** (doc 15 §10.4) — Keeper-gated, needs the roleplay beat; a
   key-connection success also cures indefinite insanity, which cancels #24's queued checks.
6. **The Luck recovery check** if enabled (docs 11 §9, 16) — cap 99.
7. **Hardening decay**: `−1` on every accumulated per-source total (doc 15 §8), floored at
   0.
8. **Credit Rating** narrative adjustment (doc 12 §8) — no roll.
9. **Underlying indefinite insanity** whose duration was "for the scenario" (#19) expires
   here if the scenario is closing.

### 6.2 Idempotency

Doc 12 §6 is explicit: running the phase twice must not double-improve. Mechanism:

- Every mutation is an `AppliedStep` under a **unique `(downtimeId, actorId, stepKey)`**.
  `stepKey` is deterministic — `improve:skill:spot_hidden`, `hardening:decay`,
  `luck:recovery`, `mastery:spot_hidden`. A re-run re-attempts every step; already-applied
  steps no-op.
- The dice results live on the `AppliedStep`, so a re-run never re-rolls.
- `commit` freezes the instance. `reopen` is allowed but emits compensating events and bumps
  a revision, so the audit trail shows a corrected phase, never a silently different one.
- Status is per-actor, not per-instance, because absent players get resolved later.
- **`advanceTime` (step 1) is idempotent by construction** — on a re-run the clock is
  already at `endWorldSeconds`, so the loop is empty. A real advantage of an absolute clock
  over counters.

---

## 7. Schema sketch

```
Campaign      { id, name, rulesetId, rules: RulesConfig, clock: GameClock }
RulesConfig   { variant: 'core'|'pulp', secondsPerRound: int, luckSpendEnabled, luckRecoveryEnabled,
                dailyLimitSnapshot: bool, outnumberedResetTiming, tieBreakChain[], enforceMode: 'enforce'|'advise' }
Scenario      { id, campaignId, name, status, startedAt, closedAt }
Session       { id, campaignId, scenarioIds[], realStart, realEnd, worldSecondsAtStart, attendeeUserIds[] }
Scene         { id, sessionId, scenarioId, name, locationLabel,
                startWorldSeconds, endWorldSeconds|null, status }
SceneParticipant  { sceneId, actorId, role, presence, joinedAt, leftAt, controllerUserId, usesFullRules }
Party         { id, campaignId, name, memberActorIds[], isDefault }

Encounter     { id, sceneId, mode, status, roundOrdinal, phase, turnOrder[], activeIndex,
                startWorldSeconds|null, elapsedCommittedSeconds|null, resolutionReason }
EncounterModeSegment  { encounterId, ordinal, mode, fromRound, toRound|null, stateRef }
EncounterParticipant  { id, encounterId, actorId|null, sceneParticipantId,
                        initiativeKeyBase, readiedFirearm, hasActed, defensesUsedThisRound,
                        defensesAllowed, holdState, resumeBefore, status,
                        chase: ChaseParticipantRef|null }        -- doc 17 owns the chase fields

GameClock     { campaignId, worldSeconds, epochLabel, activeEncounterId, roundOrdinal }
GameInstant   { worldSeconds, encounterId|null, roundOrdinal|null, anchored: bool }

PendingCheck  { id, campaignId, actorId|null, scopeType, scopeId, type, clock, dueAt,
                recurrence, priority, sourceConditionId, createdByEventId,
                status, resolvedAt, resolutionRollId }
Condition     { id, actorId, key, lifetimeClass, startedAt, remainingRounds, expiresAtWorldSeconds,
                scopeEncounterId, sourceEventId, sourceActorId, effects[], keeperNote }

DowntimeInstance { id, campaignId, scenarioId|null, label, startWorldSeconds, endWorldSeconds,
                   status, participantActorIds[] }
AppliedStep      { downtimeId, actorId, stepKey, payload, rollIds[], appliedAt }

GameEvent     { id, campaignId, occurredAt: GameInstant, recordedAt, actorUserId, type, payload,
                causedByEventId|null, compensatesEventId|null }   -- append-only; the source of truth
```

---

## Backend implications

- **One clock per campaign, two coordinates.** `worldSeconds` (bigint) + encounter-local
  `roundOrdinal`. Never convert rounds to seconds implicitly; `secondsPerRound` defaults to
  0 and is a documented house rule whenever non-zero.
- **`advanceTime` is a discrete-event loop with a step bound and a resumable
  continuation.** Closed-form batching is legal only for branchless resource regen; anything
  that rolls or branches iterates.
- **Presence is a temporal join table**, queried live and snapshotted onto every roll that
  consults it. `present()` is the single source for group Luck, area SAN checks, and
  encounter roster seeding.
- **The Encounter is polymorphic and changes mode in place.** Turn order, round counter, and
  conditions are continuous across the combat↔chase boundary; only mode-specific state gets
  created and retired.
- **Turn order is a parameterized service**, never two implementations. `initiativeKey`,
  `tieBreakers`, `activation`, and `roundResetHooks` are the extension points.
- **`hasActed` is set only by initiating an action** — defending is free. This invariant is
  what makes outnumbering (class A) derivable instead of stored.
- **PendingCheck is the only scheduling mechanism.** No domain sets private timers. Every
  check carries a clock discriminator, a priority, a cancellation link
  (`sourceConditionId`), and an undo link (`createdByEventId`).
- **Conditions carry declarative effects**, and the roll engine's sole entry point is
  `getModifiers(actorId, ctx)`. Keeper-confirmation is a property of the contribution, not
  of the UI.
- **The event log is the source of truth**; the clock, the queue, and the condition set are
  all projections. That is what makes undo and rewind implementable at all.
- **Downtime is an entity with per-step idempotency keys** and an explicit ordering,
  committed transactionally.
- **Per-participant rule opt-outs** (`wounds`, `sanity`, `scheduler`, `chase`) keep NPC mobs
  out of the scheduler and Great Old Ones out of the wound state machine.
- **Sanity's `dailyLoss` reset is a command, never a job.** Never attach it to the day
  boundary in `advanceTime`, however tempting; doc 15 §5.2 is clear that a "day" here is a
  fiction boundary.
- **A reference-implementation warning:** the Foundry CoC7 system has no game-time
  integration at all — no `worldTime` hooks, no calendar interop. It stores durations on
  conditions and trusts the Keeper to remember. That gap is exactly the value this backend
  adds, so there is no prior art to copy for §2 and §4; there *is* good prior art for §3 and
  §5.

---

## Open questions

1. **The `secondsPerRound` default.** 0 (rounds sit outside the wall clock; closing an
   encounter commits an elapsed duration) or 12 (BRP-derived)? 0 is truer to CoC 7e; 12 is
   less Keeper work. Which is default, and is it per-campaign or per-encounter?
2. **Parallel scenes / split party.** Per-scene `clockOffsetSeconds` with a Keeper sync
   point, or strictly sequential play with clock rewinds between? The latter re-fires every
   PendingCheck.
3. **`activeIndex` across a mode change.** Resume at the same participant, or restart the
   order? RAW is silent; both are defensible.
4. **Does a chase movement-action deficit survive into combat?** No rule exists. Recommend
   dropping it; confirm.
5. **Round-clock checks when the encounter ends.** Auto-convert a still-dying investigator's
   per-round CON checks to a narrative timer, keep an encounter open, or force the Keeper to
   resolve? Each option redefines what "the fight is over" means.
6. **Batch vs. iterate for MP regen and daily HP.** Closed-form is safe today, but any
   future rule making regeneration conditional turns the collapse wrong. Enforce
   iterate-only, or allow the optimization behind a registry of "branchless" check types?
7. **The rewind default policy** — discard or retain events after the rewind point? Retain
   is safer for data but leaves timestamps that no longer match the fiction.
8. **The priority ladder for simultaneous checks.** The proposed ladder runs lethal checks
   last so same-instant healing counts. Is that the table ruling we want, or should the
   dying roll fire first?
9. **Scenario boundary as a duration.** Underlying indefinite insanity lasts "for the
   scenario" (doc 15 §6.2) — is the scenario-close event the cancellation trigger, or must
   the Keeper clear it explicitly?
10. **Where does `usesFullRules` live** — on the Actor (a property of the monster), on the
    SceneParticipant (a property of this appearance), or both with the participant winning?
11. **The turn-order tie-break chain.** Is generalizing the chase's "higher relevant combat
    skill" rule into combat acceptable, given combat RAW is silent? And what does "relevant"
    mean when the tied participants use different skills?
12. **Do sessions matter mechanically at all?** Nothing in core 7e is per-session; only
    optional Luck-recovery house rules are. If not, is Session a first-class entity or just
    an attendance log?

---

## Sources

- Chaosium, *Call of Cthulhu 7th Edition Keeper Rulebook* — combat and initiative
  ~pp. 100–130 (round structure and DEX order ~p. 101, readied firearm DEX+50 ~p. 101,
  outnumbering ~p. 108, aiming ~p. 111, malfunction ~p. 113); wounds/healing/death
  ~pp. 119–125 with the damage flow chart at p. 122; chases pp. 132–151 (chase round
  pp. 133–134, joining a chase p. 142, monsters in chases p. 144) with summary tables
  pp. 414–416; Sanity ~pp. 154–169; optional Luck spending and recovery ~p. 99. Page numbers
  differ between the 2014 and 2023 revised printings.
- Chaosium, *Investigator Handbook* 7th Edition — the Investigator Development Phase and
  backstory / key connections.
- Chaosium official *Call of Cthulhu* RPG Wiki —
  [Combat](https://cthulhuwiki.chaosium.com/rules/combat.html) (round defined as "long
  enough for everyone to take one significant action"; DEX order; readied firearms at
  DEX+50; defending does not consume your action) and
  [Game System](https://cthulhuwiki.chaosium.com/rules/game-system.html) (group Luck roll
  made by the lowest-Luck investigator **among those present in the scene** — the
  load-bearing citation for §1.2).
- Chaosium, *Call of Cthulhu 7th Edition Quick-Start Rules* (free):
  https://www.chaosium.com/content/FreePDFs/CoC/CHA23131%20Call%20of%20Cthulhu%207th%20Edition%20Quick-Start%20Rules.pdf
- Chaosium *Basic Roleplaying* SRD —
  [4.2 The Turn](https://brp.chaosium.com/basic-roleplaying/4-0-time/4-2-the-turn/) ("each
  turn equals five minutes (25 combat rounds)") and
  [5.1 The Combat Round](https://brp.chaosium.com/basic-roleplaying/5-0-combat/5-1-the-combat-round/)
  ("a combat round is 12 seconds long", four fixed phases). Used only to establish that
  CoC 7e's round/turn are *not* these, per §2.1.
- Chaosium, *Basic Roleplaying ORC Content Document*:
  https://www.chaosium.com/content/orclicense/BasicRoleplaying-ORC-Content-Document.pdf —
  the ORC Sanity system's 5-minute-turn accumulation window (see doc 21 §3.2 for why it
  must never be conflated with CoC 7e's game-day window).
- Foundry VTT CoC7 system (Miskatonic Investigative Society), `develop` branch —
  https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT :
  `coc7/apps/combat.js` and `coc7/hooks/render-combat-tracker.js` (combat-tracker
  integration; the per-combatant `hasGun` flag that recomputes initiative, confirming
  readied-firearm is a flag not a stat; a setting toggling regular DEX-value initiative vs.
  the optional DEX-roll rule); `coc7/models/item/status-system.js` (the `status` item type,
  its legacy `duration { permanent, hours, minutes, rounds }` shape and its
  `type { mania, phobia }` discriminator); `coc7/models/actor/document-class.js`
  (`enterBoutOfMadness`, `conditionsSet` — writes `duration.rounds = n` for real-time bouts
  and `duration.seconds = n × 3600` for summary bouts; `resetDailySanity`);
  `coc7/constants.js` (`STATUS_EFFECTS` condition set and the `DICE_POOL_REASONS` modifier
  registry with its `ifAttacker / ifDefender / forMelee / forRanged / forBonus / forPenalty`
  selectors, the model for §5.1); `coc7/models/item/chase-system.js` and
  `coc7/models/chase/participant.js` (chase item/actor model, `includeEscaped` /
  `includeLatecomers`). **Verified absence:** no references to `game.time`, `worldTime`,
  `updateWorldTime`, or Simple Calendar anywhere in the repository — the system has no
  game-clock integration.
- Foundry VTT, [Combat Encounters](https://foundryvtt.com/article/combat/) — encounters
  scoped to a Scene; rounds and turns as tracker positions; initiative stored as a mutable
  number.
- Alchemy RPG help centre,
  [Call of Cthulhu ruleset](https://help.alchemyrpg.com/en/articles/10157823-call-of-cthulhu)
  — a commercial tracker that explicitly enumerates rounds / hours / days / weeks / months /
  a development phase as distinct time increments, and lists conditions (regular damage,
  major wound, instant death, unconscious, dying, prone, temporary / indefinite / permanent
  insanity). Alchemy's product framing is scene-based rather than grid-based — the closest
  commercial analogue to the Scene entity in §1.
- Fantasy Grounds,
  [Call of Cthulhu 7E Ruleset](https://fantasygroundsunity.atlassian.net/wiki/spaces/FGCP/pages/996642343)
  and its forum thread https://www.fantasygrounds.com/forums/showthread.php?t=35672 — a
  combat tracker whose per-combatant state persists until explicitly removed, plus a "reset
  turn order" operation; evidence that combatant rows must be status-transitioned, never
  deleted.
- RPGGeek threads "Duration of a 'round'" (https://rpggeek.com/thread/3535610/duration-of-a-round)
  and "Timing in CoC (Hours? Rounds?)" (https://rpggeek.com/thread/1545747/timing-in-coc-hours-rounds)
  — community confirmation that 7e leaves round length deliberately undefined.
- Sibling context docs in this folder, cited throughout rather than restated: 11 (dice and
  rolls, group Luck), 12 (skills and the development phase), 13 (combat), 14 (wounds,
  healing, death), 15 (Sanity), 16 (Luck, magic points, spells), 17 (chases), 19 (weapons
  and NPC stat blocks), 20 (digital tools landscape), 21 (licensing and IP).
