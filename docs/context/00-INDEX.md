# Context Docs — Annotated Index

> Output of the research phase for the CoC Phoenix tracker backend. Fourteen docs, each
> written in its researcher's own words with page pointers into the commercial books rather
> than reproduced text. Docs `10`–`19` form the **rules** layer, `20`–`21` the
> **environment** layer (prior art and legal constraints), and the two `30`s the **runtime
> container** layer every rules doc assumes but none defines. `90-backend-implications.md`
> is the synthesis built on top of them all.
>
> Ordered by filename.

---

## `10-characteristics-derived-stats.md`

The eight percentile characteristics, their generation methods (including the fixed-array
and point-buy alternatives), the age-modifier table with closed-form expressions per band,
and everything derived from them — HP, MP, Sanity, Luck, Damage Bonus, Build, MOV, and the
half/fifth columns. Also holds the dependency graph separating values that must be
recomputed on a characteristic change from values seeded once at creation that then diverge.

**Backend takeaway:** the derived-vs-state distinction is the single most common modelling
bug in CoC tools — `hpMax`/`mpMax`/`sanMax`/`DB`/`Build`/`mov.base` are live-derived and
must flow from one pure `deriveAttributes()` function, while `san.current`, starting Dodge,
starting Language (Own), and both skill-point pools are **seed-only** and must never be
recomputed.

## `11-dice-and-skill-rolls.md`

The d100 resolution core: the physical tens-die/units-die construction, the six-rung success
ladder, difficulty as a threshold change (never a modifier), bonus/penalty dice as extra
*tens* dice cancelling 1-for-1, fumble thresholds, and the compositional rules stacked on
top — pushed rolls, opposed rolls, combined rolls, group and cooperative rolls, Luck rolls
and Luck spending.

**Backend takeaway:** roll *classification* must be a pure function of `(result, thresholds,
fumbleMin, policy)` held strictly apart from dice generation, because Luck spends, Keeper
overrides, and post-hoc dice additions are all re-classifications of stored dice rather than
new rolls — and the bonus-die algorithm must materialize every candidate total before taking
min/max, never selecting on the raw tens die.

## `12-skills-and-improvement.md`

Skill anatomy (base + occupation + personal + experience as separable components), the
base-value distribution, formula-derived bases (Dodge, Language (Own)), the seven
specialization groups, and the two-phase improvement loop: the idempotent in-play "tick"
followed by the between-scenario Investigator Development Phase. Covers the two anti-skills,
Cthulhu Mythos and Credit Rating, plus training, study, and experience packages.

**Backend takeaway:** the improvement flag is a **boolean, never a counter** (ten successes
in one scenario buy exactly one improvement roll), it must record the roll id that set it so
undo can withdraw it, and the development phase must be one transactional,
idempotent-per-downtime-instance operation writing only to the experience component.

## `13-combat.md`

Round structure and DEX-descending initiative with the readied-firearm ordering bump, the
melee opposed exchange with its two asymmetric tie rules, fighting maneuvers gated on the
Build differential, the full firearms pipeline (range-as-difficulty, point-blank, aiming,
rate of fire, automatic-fire volleys, cover, malfunction, reloading), damage and impale,
surprise, and outnumbering.

**Backend takeaway:** the opposed comparator takes tie-direction as an explicit parameter —
fight-back ties favour the **attacker**, dodge ties the **defender** — and combat modifiers
travel as a list of named `{source, delta}` contributions collapsed to a single signed
integer only at roll time, so the UI can explain the net and house rules can toggle per
source.

## `14-wounds-healing-death.md`

Damage application ordering (impale → armour → threshold test → HP subtraction →
transitions), the major-wound and instant-death thresholds measured against *maximum* HP and
a *single* blow, the healthy → major wound → unconscious → dying → dead state machine, First
Aid and Medicine with their per-injury treatment budget, natural and major-wound healing
cadences, and non-standard damage (asphyxiation, poison, falls) that skips parts of the
pipeline.

**Backend takeaway:** this subsystem runs three clocks simultaneously (per-round dying
countdown, hourly stabilization checks, daily/weekly recovery), so damage must be modelled
as per-instance `Injury` rows plus a `PendingCheck` timer queue — never as a net HP delta,
because two 4-point hits and one 8-point hit leave identical HP and different wound state.

## `15-sanity.md`

The Sanity pool as both resource and roll target, the `X/Y` loss notation, the ordered
resolution pipeline (bout-immunity gate → hardening gate → roll → loss → thresholds), the
three insanity thresholds and their precedence, the two-phase bout/underlying-insanity shape
with its real-time and summary tables, the Cthulhu Mythos feedback into the Sanity ceiling,
per-source hardening, phobias and manias, and every recovery route.

**Backend takeaway:** a Sanity check is a **persisted, resumable aggregate**, not a function
call — it blocks on Keeper input at several steps — and the two counter-intuitive rules
(passing the INT roll is what makes you temporarily insane; a Mythos-driven ceiling clamp is
*not* a Sanity loss and must never feed `dailyLoss`) each deserve a loud comment and a test.

## `16-luck-magic-mp.md`

Luck as characteristic, spendable meta-currency, and improvement-check subject; the hard
blocklist of what Luck may never touch; magic points as a POW-derived regenerating pool with
HP spillover on overspend and a separate non-regenerating over-cap bucket; POW improvement
triggers; and the spellcasting pipeline including the first-cast Hard POW roll, pushed
castings, and pooled ritual contributions.

**Backend takeaway:** the natural die is sticky — criticals, fumbles, and firearm
malfunctions read off `naturalRoll` while success level reads off `effectiveRoll`, so both
must be persisted; and casting-cost payment must be atomic across the MP-then-HP spillover,
with pooled contributors summed before validation.

## `17-chases.md`

The chase as a linear track of abstract locations: the one-per-participant speed roll
adjusting MOV by ±1, the escape/exclusion gate, the relative movement-action formula,
staggered initial placement, DEX-ordered rounds, hazards (always crossed, costing 1D3
actions) versus barriers (hard stop, carrying hit points), cautious approach and ally
assistance, conflict during a chase, vehicles, Pedal to the Metal, collisions, and the
terminating predicates.

**Backend takeaway:** the movement-action allowance is **relative to the slowest active
participant**, so any roster change — a latecomer, an escape, a knockout, a vehicle swap —
recomputes `minMOV` and with it everyone's action budget; and obstacles live on the *edges*
between locations, so modelling them as node properties misbehaves.

## `18-character-creation-occupations.md`

The strictly ordered creation pipeline (roll → age → derive → occupation → occupation points
→ personal interest points → backstory → finishing touches), the three generation methods,
the age-bracket table with its player-directed deductions, occupation skill-point formulas
as characteristic × multiplier terms with mutually exclusive optional groups, occupation
skill slots as descriptors rather than a flat list, Credit Rating's wealth tiers, and the
ten backstory categories.

**Backend takeaway:** persist the *inputs* — the dice log, age, allocation splits, which
optional characteristic was picked, which skill filled each choice slot — and treat the
sheet as a computed projection, because age must be applied before the occupation formula
reads EDU, and the whole pipeline has to re-run cleanly when a characteristic changes in
play.

## `19-weapons-equipment-npc-stats.md`

The weapon table column model (skill, damage plus a DB mode flag, base range, uses per
round, magazine, malfunction, era price) with the per-range-band damage shape shotguns
force, the open-ended Damage Bonus / Build banding formula, the firearm resolution pipeline,
impale rules, and the shared NPC/creature stat-block shape including multi-modal MOV,
creature armour, spell allowances, and the `X/Y` Sanity-loss pair.

**Backend takeaway:** several fields that look numeric are not — Damage Bonus is a flat
integer at the bottom of the ladder and a dice expression at the top, creature armour is
frequently a damage *transformation rule* ("firearms do half", "bullets do 1", "immune to
physical") rather than a number, and every derived field on an authored stat block needs an
`auto` override flag.

## `20-digital-tools-landscape.md`

A survey of the four families of existing CoC 7e software, with a deep read of the
open-source Foundry VTT CoC7 system's actor/item schemas, its d100 dice engine, its
content-slug namespace, and its roll mutation model; plus the Roll20 sheets as a cautionary
tale, the Dhole's House JSON as the de-facto interchange format, and a borrow/avoid list.

**Backend takeaway:** a roll is a **persisted aggregate that keeps mutating after the dice
land** — bonus dice added, Luck spent, the roll pushed, the result forced — which is why
`isRolledSuccess` (earned with no Luck and no dice modifiers) must be tracked apart from
`isSuccess`, and why the naive keep-highest/lowest-on-raw-tens-dice macro breaks whenever
the units die shows 0.

## `21-licensing-and-ip.md`

The four legal tracks (Fan Material Policy, Miskatonic Repository, BRP-UGE under ORC,
commercial / Small Publisher licence), what the fan policy permits and forbids — notably
blessing web calculators while excluding downloadable software — what the ORC-licensed BRP
engine actually delivers, including a *different* Sanity system, the idea/expression line,
and a four-class content classification for the schema.

**Backend takeaway:** the licence position forces entities into existence — `ContentPack`
with a `license` discriminator as the audit boundary and kill switch, `Ruleset` as a
first-class campaign discriminator (ORC and CoC run different success ladders and different
Sanity maths), and a schema shipping **empty** for creatures, spells, tomes, weapons, and
occupations.

## `30-authority-visibility-and-event-log.md`

The permission layer none of the rules docs specify: role + campaign policy + a thin
per-actor ownership grant, with field authority as static rules metadata; the full
CheckRequest/Roll state machine with its post-resolution mutation window and explicit freeze
point; visibility as a server-side projection closed over the derived-value dependency
graph; the Keeper adjudication queue; and the event log with undo, concurrency, and
idempotency semantics.

**Backend takeaway:** a roll must be **explicitly frozen**, because the experience-tick
predicate cannot be evaluated while Luck may still be spent and dice may still be added —
and the projection rule `visibility(derived) ≤ min(visibility(inputs))` must be enforced at
build time, since leaking `san.max` leaks Cthulhu Mythos and leaking `hp.max` leaks the
major-wound threshold.

## `30-session-scene-and-game-time.md`

The runtime container: the campaign/scenario/session/scene/encounter ladder with presence as
live queryable state, one two-coordinate clock (in-game seconds plus a non-convertible
encounter-local round ordinal), an enumeration of all thirty timed rules across docs 10–19
with their clock and cancellation semantics, `advanceTime` as a discrete-event loop, one
polymorphic Encounter hosting both combat and chase turn loops, a unified condition model
keyed by lifetime class, and downtime as an idempotent committable entity.

**Backend takeaway:** rounds are **not convertible to seconds** in CoC 7e, so the round
clock and the world clock are separate coordinates compared only within themselves; and
`advanceTime` must iterate every due interval instead of multiplying, because a three-week
skip is three weekly recovery rolls with the major-wound clear condition re-checked after
each.
