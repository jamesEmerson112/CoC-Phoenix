# Sanity System (Call of Cthulhu 7th Edition)

> Context doc for the CoC Phoenix tracker backend. All mechanics are paraphrased in our own
> words from the sources listed at the bottom. Page references point into the Chaosium
> *Keeper Rulebook* (7e) for verification; they are approximate ranges, never quotations.

## Overview

Sanity (SAN) is a percentile pool doing double duty as resource and skill rating:
investigators roll against their **current** SAN to withstand the psychological impact of
witnessing horror, and the very same number shrinks each time they fail. Around that pool
sits a small state machine. Every horrific stimulus carries a two-sided loss notation
(`success/failure`, e.g. `1/1D6`); losing enough in one hit triggers **temporary insanity**,
losing enough across one in-game day triggers **indefinite insanity**, and hitting zero
triggers **permanent insanity** (the investigator exits play). Insanity itself runs in two
phases — a short, Keeper-driven **bout of madness**, then a longer **underlying insanity**
during which even one further point of loss re-triggers a bout. Layered over all of it are
two long-lived accumulators the backend must persist: the **Cthulhu Mythos** skill, which
permanently lowers the SAN ceiling (`max SAN = 99 − Mythos`), and per-source **hardening**
counters capping how much any one kind of horror can ever cost an investigator. Recovery is
slow, happens mostly between scenarios, and comes with its own dice tables.

---

## 1. Core quantities and invariants

| Field | Derivation | Notes |
|---|---|---|
| `san.value` | Starts equal to POW at creation | The live pool; also the roll target |
| `san.max` | `max(0, 99 − cthulhuMythos)` | Ceiling on `san.value`; recomputed on any Mythos change |
| `san.startingValue` | POW at creation | Useful only for "how far have they fallen" reporting |
| `cthulhuMythos` | An ordinary skill's `value` | In practice only ever rises |
| `san.dailyLimit` | `floor(san.value / 5)`, snapshotted at day reset | The indefinite-insanity threshold |
| `san.dailyLoss` | Running total of SAN lost this game day | Zeroed at day reset |

Invariants to enforce on every write:

1. `0 <= san.value <= san.max`.
2. Raising `cthulhuMythos` lowers `san.max`; if the new `san.max < san.value`, clamp
   `san.value` down at once. That clamp is a *silent* reduction — not a "SAN loss event" —
   and must **never** feed `dailyLoss` or fire insanity checks.
3. `san.value == 0` is absorbing (permanent insanity). Nothing raises it afterward.
4. No recovery source may lift `san.value` past `san.max`; the excess evaporates — it is
   not banked.

Mind the asymmetry: `san.max` is *not* derived from POW. A fresh investigator with POW 40
has `san.value = 40` but `san.max = 99` — genuine headroom to recover into.

---

## 2. The Sanity roll

A Sanity roll is a plain d100 roll-under against `san.value`, deliberately stripped of the
usual apparatus:

- **No pushing.** Sanity rolls cannot be pushed like ordinary skill rolls.
- **No Luck spending** (core rules). Luck cannot buy the success. §11 covers the Pulp
  variant.
- **No difficulty levels.** No Hard or Extreme Sanity rolls — pass/fail against the full
  value. (Bonus dice show up in exactly one place: the Self-Help key-connection roll,
  §10.4.)
- **Fumbles matter.** A fumbled Sanity roll costs the **maximum** the failure formula could
  produce, no dice rolled. Implementation: evaluate the failure formula in `maximize` mode.
- **Critical success (01)** carries no special SAN effect in core rules, though the same
  `01` is the trigger for forming a new key connection during Self-Help (§10.4).

A **failed** Sanity roll also forces an *involuntary action* — a moment of lost self-control
the Keeper narrates (dropping what you hold, crying out, flinching, freezing, an involuntary
combat action). Mechanically negligible, but it is a distinct beat in the resolution
sequence, worth modelling as a checkpoint the Keeper acknowledges before the loss lands.

---

## 3. SAN loss notation

Written `MIN/MAX`, each side an independent dice expression:

```
sanLoss := <expr> "/" <expr>
expr    := integer | NdM | NdM+K | NdM-K | "0"
```

- **Left side** applies when the Sanity roll **succeeds**. Usually a small flat number
  (`0`, `1`).
- **Right side** applies when it **fails**. Usually dice (`1D6`, `1D10`, `2D10`).
- Shape examples: `0/1D3` (a corpse), `1/1D6` (a mangled body), `1D10/1D100` (the top of
  the pantheon). Both sides may be dice; either side may be `0`.

Parser requirements:

- Accept a bare `0` on either side and short-circuit to "no loss, no insanity check."
- Evaluate any `expr` in three modes: **roll**, **minimize**, **maximize**. Minimize serves
  the hardening short-circuit (§8); maximize serves fumbles and hardening caps.
- Some sources (Mythos tomes, certain spells) inflict loss **automatically, no Sanity
  roll**. Model as a `skipRoll` flag forcing the failure-side formula.

---

## 4. Resolution pipeline

The order of operations is load-bearing. A single "Sanity check" should be a multi-step,
resumable card rather than one atomic function, because several steps sit waiting on Keeper
or player input.

```
1. Bout immunity gate  ─ if actor is currently in a bout of madness → no loss, no roll. STOP.
2. Hardening gate      ─ if remaining allowance for this source is 0 → no loss. STOP. (§8)
3. Sanity roll         ─ d100 vs san.value → success | failure | fumble
4. Involuntary action  ─ on failure only; Keeper narrates
5. Pick formula        ─ success → MIN side; failure → MAX side; fumble → MAX side maximized
6. Roll the loss       ─ value = eval(formula)
7. Clamp to hardening  ─ value = min(value, remainingAllowanceForSource)
8. Mythos Hardened     ─ if optional rule active for this actor: value = floor(value / 2)  (§7.3)
9. Apply              ─ san.value -= value ; san.dailyLoss += value ; hardening[source] += value
10. Insanity checks    ─ evaluate in the order given in §5
11. Mythos award       ─ if the source was Mythos and insanity resulted (§7.2)
```

Step 1 is the rule most often missed: **an investigator already inside a bout of madness
cannot lose further Sanity.** Fresh horrors seen mid-bout are repressed or blamed on the
madness. Immunity ends the instant the bout does — if the horror still stands there when
they come to, the check fires then.

---

## 5. Insanity thresholds and precedence

Three thresholds, evaluated after the loss lands. Check **most severe first**:

### 5.1 Permanent insanity — `san.value <= 0`

Permanently insane; removed from play as a player character. No INT roll, no bout, no
recovery. This outcome dominates everything else and should short-circuit the rest of the
pipeline.

### 5.2 Indefinite insanity — `san.dailyLoss >= san.dailyLimit`

Losing a fifth or more of Sanity within one game "day" causes indefinite insanity. Details
code must get right:

- The divisor is **one fifth of current Sanity** — not POW, not `san.max`. A common,
  practical implementation (Foundry's CoC7 among them) snapshots
  `dailyLimit = floor(san.value / 5)` when the day resets, so the bar doesn't sink as the
  investigator bleeds points through the day. Continuous recomputation makes the threshold
  a moving target that gets easier to hit the more is already lost. **Pick one and document
  it** — see Open Questions.
- The trigger is **automatic**. No INT roll, no save.
- It accumulates across the day: five separate 2-point losses can trip it where no single
  loss would.
- A "day" is a Keeper-defined unit — in practice "until the investigators reach safety and
  sleep," not 24 wall-clock hours. The reset must therefore be a **Keeper-invoked
  command**, never a timer.

### 5.3 Temporary insanity — single loss `>= 5`

When one Sanity roll costs 5 or more points, roll INT (d100 roll-under INT).

- **INT success → temporarily insane.** The investigator understood exactly what they saw.
- **INT failure → the memory is repressed**, and nothing further happens.

This inversion is the most commonly mis-implemented rule in the whole system. Succeeding is
the bad outcome.

Duration: 1D10 hours of insanity, or until a full night's sleep somewhere safe.

### 5.4 Precedence and simultaneity

When one loss trips both 5.2 and 5.3, indefinite wins: it is automatic and longer-lived, so
it supersedes the INT-gated temporary result. Practically:

- Check permanent first → if true, stop.
- Check indefinite → if true, set indefinite, skip the INT roll, go to the bout.
- Otherwise check the 5-point rule → INT roll → maybe temporary.

Extra wrinkle: an actor **already** in underlying insanity (§6.2) enters a fresh bout on
*any* loss of 1 or more, regardless of size — the 5-point rule is bypassed entirely.

---

## 6. The two phases of insanity

Temporary and indefinite insanity share the same two-phase shape; only durations differ.

### 6.1 Phase 1 — bout of madness

A short, intense episode in which the Keeper (or the player, prompted) drives behaviour. Two
resolution modes, chosen by whether the scene is being played beat-by-beat:

| Mode | Duration | Typical use |
|---|---|---|
| **Real time** | 1D10 combat rounds | The investigator is with the party and the scene is live |
| **Summary** | 1D10 hours (mostly) | The investigator is alone, or the whole party broke at once — narrated as a lost interval |

Each mode has its own d10 result table, and both share slots 9 and 10: **9 = gain a
phobia**, **10 = gain a mania**. The remaining eight slots differ in kind:

- The *real-time* table yields immediate, playable behaviours: scene amnesia, a
  psychosomatic disability, a violent outburst, paranoia, mistaking a bystander for someone
  from the backstory, fainting, blind flight, emotional collapse.
- The *summary* table yields "you come to later and…" outcomes: robbed, beaten,
  institutionalized, displaced far from where you were, or having acted out a backstory
  belief to an extreme.

Backend takeaway: these are **two configurable roll tables** whose results can be plain
narration or a structured effect (a phobia/mania item attached to the character). Make the
tables pluggable, let entries carry a typed payload rather than prose alone, and support
recursive tables (an entry that chains into the phobia table).

During a bout the investigator is **immune to further SAN loss** (§4 step 1). The Keeper may
also amend or add one backstory entry per bout to mark the character's decay, with player
agreement.

### 6.2 Phase 2 — underlying insanity

The player regains control, but the character stays brittle.

- **Duration:** 1D10 hours if the insanity was temporary; until scenario end or successful
  treatment if indefinite.
- **Fragility rule:** any further Sanity loss — a single point suffices — instantly starts a
  new bout (back to 6.1). This is what turns insanity into a spiral.
- The character can otherwise act rationally. The insanity shows through the bouts, the
  acquired phobias/manias, and Keeper-planted delusions.

### 6.3 Delusions and reality checks

While underlying insane, the Keeper may feed the player false sensory information. The
player may attempt a **reality check** — an ordinary Sanity roll:

- **Success:** the investigator sees the true environment and resists further delusions
  until the next Sanity loss.
- **Failure:** costs 1 SAN and the delusion holds. Because the character is underlying
  insane, that 1 point instantly triggers another bout (§6.2). A genuine trap — implement
  as a loop, not a one-shot.

A successful Psychoanalysis roll by an ally can also break a delusion.

---

## 7. Cthulhu Mythos interaction

### 7.1 The ceiling

`max SAN = 99 − Cthulhu Mythos%`. Each point of Mythos knowledge permanently costs a point
of potential Sanity. Mythos being effectively monotonic, `san.max` only ever falls.

### 7.2 Mythos gained from going mad

When insanity (temporary or indefinite) traces to a **Mythos** source:

- **First ever Mythos-caused episode: +5 Cthulhu Mythos.**
- **Each subsequent Mythos-caused episode: +1 Cthulhu Mythos.**

This needs a persistent per-character boolean (`mythosInsanityExperienced`) that flips on
the first award and never resets. Non-Mythos trauma (a car crash, a murder scene) grants
nothing. Apply the award *after* the loss; it lowers `san.max` and may clamp `san.value`
further (invariant 2).

Mythos also accrues from tomes and spells — outside this doc's scope but hitting the same
ceiling. Note that reading a tome can raise Mythos (and sink `san.max`) even when the reader
shrugs the content off as fiction and takes no immediate loss.

### 7.3 "Mythos Hardened" (optional rule)

Once an investigator's Cthulhu Mythos skill exceeds their current Sanity, their worldview
flips for good: all subsequent Sanity loss is **halved** (round down). Irreversible even if
Sanity later climbs back above Mythos. Model as an opt-in campaign setting plus a sticky
per-character flag, checked at pipeline step 8.

---

## 8. Hardening ("getting used to awfulness")

Investigators grow numb to specific horrors. For each distinct source type (a creature
species, a recurring sight):

- Track cumulative SAN lost to it: `hardening[sourceKey].totalLoss`.
- The cap equals the **maximum** the failure-side formula for that source can roll. For a
  `1/1D6` source, the cap is 6.
- `remainingAllowance = max(0, cap − totalLoss)`. At 0, the source costs nothing — skip the
  roll entirely.
- Clamp each individual loss to `remainingAllowance` (pipeline step 7). Useful
  optimization: if the loss formula's *minimum* already meets or exceeds the remaining
  allowance, skip the roll and charge exactly the remainder.
- **Decay:** each development phase reduces every accumulated total by 1 — familiarity
  fades.

Per-character and per-source, so it's a child collection keyed by a normalized source
identifier. Key normalization (is "deep one" the same source as "deep one hybrid"?) is a
design decision the Keeper should be able to override.

---

## 9. Phobias and manias

- Acquired on results 9 and 10 of either bout table, or handed out by the Keeper. The
  rulebook offers sample d100 lists of each (arachnophobia, hemophobia, kleptomania, and so
  on).
- They are **persistent** — outliving the insanity that spawned them, removed only through
  therapy (§10.5).
- The mechanical effect applies **while the investigator is insane**:
  - **Phobia:** direct exposure to the trigger panics them. Non-combat actions and escape
    attempts take **one penalty die**. Sanity rolls and reality checks are explicitly
    exempt.
  - **Mania:** exposure creates overwhelming compulsion — **one penalty die on all rolls**
    until the character indulges it or is removed from the stimulus. Full surrender may
    prompt further Keeper difficulty adjustments.
- A successful **Psychoanalysis** roll lets an insane investigator set a phobia or mania
  aside temporarily.

Backend: phobias and manias are typed, named, persistent character-attached items with a
`type ∈ {phobia, mania}` discriminator, a trigger description, and an active/suppressed
state. Surface their penalty die to the roller as a conditional modifier the Keeper
confirms — never auto-applied, since the engine can't know whether the trigger is in the
scene.

---

## 10. Recovery

### 10.1 Temporary insanity

Ends on its own after 1D10 hours, or after a good night's sleep somewhere safe. The Keeper
may pause the countdown while the group stays under sustained tension (standing watch with
something circling outside). No dice, no SAN restored — the *condition* clears; the lost
points stay lost.

### 10.2 Indefinite insanity — private / home care

After each full month of safe treatment, roll d100:

- **01–95:** gain 1D3 SAN, then make a Sanity roll. Success = cured (condition cleared).
  Failure = keep the points, retry next month.
- **96–00:** the patient rebels; lose 1D6 SAN, no progress; retry in a month.

### 10.3 Indefinite insanity — institutionalization

After each full month, roll d100:

- **01–50:** gain 1D3 SAN, then a Sanity roll; success = cured, failure = retry next month.
- **51–95:** no change.
- **96–00:** lose 1D6 SAN, no progress.

The Keeper may shift the 50 threshold roughly ±5 for a better or worse institution. Note the
deliberate irony: home care from someone who cares beats a 1920s asylum by a wide margin.

### 10.4 Development-phase gains

Four distinct routes for *raising* current SAN (all clamped at `san.max`):

1. **Keeper award** at the end of a scenario or campaign chapter — a die roll scaled to the
   danger faced, docked for cowardly or brutal play.
2. **Skill mastery:** the first time any skill reaches 90%+, gain 2D6 SAN.
3. **Psychotherapy:** a monthly roll against the therapist's Psychoanalysis skill. Success
   = +1D3 SAN; failure = nothing; **fumble = lose 1D6 SAN and the therapeutic relationship
   ends**. The same roll can instead target removing one phobia or mania — that requires
   both the Psychoanalysis roll *and* a Sanity roll, and grants no SAN points when used
   this way.
4. **Self-Help:** the investigator leans on a supportive backstory entry (phobias, manias,
   and Mythos entries excluded). After thematic roleplay, roll Sanity: success = +1D6 SAN;
   failure = lose 1 SAN and the backstory entry must be rewritten.
   - If the entry is the character's designated **key connection**, the Sanity roll takes a
     **bonus die**, and success additionally **cures indefinite insanity**. Failure strips
     the entry of key-connection status.
   - A new key connection can be established in a later development phase, or on a
     critical (01).

### 10.5 Summary of what recovery touches

Recovery is the only place `san.value` moves upward, and every path clamps at `san.max`.
Insanity *conditions* clear on their own schedule, independent of point restoration — a
cured investigator can still be sitting at 8 SAN.

---

## 11. Pulp Cthulhu and other variants

Worth designing for even if v1 skips them, because they alter core arithmetic:

- **Luck halves Sanity loss (Pulp):** spend 2 Luck per 1 point of SAN averted. Core CoC
  forbids Luck touching Sanity at all.
- **Insane Insight (optional):** on going insane, the Keeper hands the investigator a
  genuine, plot-relevant realization. Purely narrative, but a hook worth surfacing.
- **Multiple simultaneous horrors (optional):** facing several creatures at once, roll the
  loss formula once per creature and apply only the highest — never the sum.

---

## 12. Worked edge cases

- **Loss of exactly 5** triggers the INT roll. The threshold is `>= 5`, not `> 5`.
- **Loss of 5+ that also drops SAN to 0 in the same hit** — permanent insanity wins; no INT
  roll.
- **`dailyLimit` of 0.** At 4 or fewer SAN, `floor(4/5) == 0`, so *any* daily loss
  satisfies `dailyLoss >= dailyLimit`. Decide: floor `dailyLimit` at 1, or accept that
  very-low-SAN characters go indefinitely insane on any loss (thematically defensible).
- **Zero-loss sources.** A `0/0`, or a successful roll against `0/1D4`, produces no loss:
  skip the insanity checks entirely; touch neither `dailyLoss` nor hardening.
- **The Mythos clamp is not a loss.** Gaining Mythos may pull `san.value` down via the
  ceiling; that must not feed `dailyLoss` or trigger insanity, or reading alone would drive
  investigators mad in a way the rules never intended.
- **A second bout while already insane.** An indefinitely insane investigator losing 1 SAN
  re-enters a bout, regaining immunity for its duration. The indefinite condition itself is
  unchanged; only the bout sub-state cycles.
- **Fumbled Sanity roll under hardening.** Maximize the failure formula, *then* clamp to
  the remaining allowance — the clamp is the outer bound.
- **Bout ends with the horror still present.** Re-run the Sanity check at that moment; the
  immunity was temporal, not per-stimulus.

---

## Backend implications

**Entities / fields**

- `Investigator.sanity`: `{ value, max, startingValue, dailyLoss, dailyLimit }`. `max` is
  derived (`99 − mythos`) but store it denormalized so a Keeper can override.
- `Investigator.flags`: `{ mythosInsanityExperienced: bool, mythosHardened: bool }` — both
  sticky, never reset.
- `Condition` records with `type ∈ {temporaryInsanity, indefiniteInsanity,
  permanentInsanity, boutOfMadness}`, each carrying `{ active, startedAt, duration,
  durationUnit ∈ {rounds, hours}, realTime: bool }`. Temporary and indefinite are *not*
  mutually exclusive with a bout — the bout is a sub-state cycling inside them.
- `Trait` records for phobias/manias: `{ name, kind ∈ {phobia, mania}, description,
  suppressedUntil }`.
- `HardeningEntry`: `{ investigatorId, sourceKey, cap, totalLoss }` with a
  development-phase decay job.
- `SanityCheck` as a **persisted, resumable aggregate**, never a function call — it has to
  survive waiting on a Keeper decision between steps. Suggested fields: `sourceKey`,
  `lossMin`, `lossMax`, `skipRoll`, `sanRollResult`, `involuntaryActionAcknowledged`,
  `rolledLoss`, `preHardenedLoss`, `finalLoss`, `lossApplied`, `intRollResult`,
  `memoryRepressed`, `boutMode`, `boutDuration`, `boutResult`, and the derived flags
  `temporaryInsane / indefinitelyInsane / permanentlyInsane / immuneAlreadyInBout /
  immuneToSource / limitedBySource / mythosAwarded`. This list closely matches what the
  Foundry CoC7 sanity card actually carries — decent validation that the shape is right.
- `RollTable` support with typed results, so bout tables can emit narration or a
  phobia/mania trait, and can chain into the phobia/mania d100 tables.

**Operations / commands**

- `resetDay(investigatorId)` — Keeper-invoked, never scheduled. Zeroes `dailyLoss` and
  re-snapshots `dailyLimit`. The single most important manual control in the system.
- `applySanityLoss` must be idempotent per check ID; double-application is a realistic
  failure mode when a Keeper double-clicks.
- Recovery operations are per-month batch actions (`privateCare`, `institutionalize`,
  `psychotherapy`, `selfHelp`, `masteryAward`, `keeperAward`) belonging to a separate
  development-phase/downtime module, not the live-session path.
- Everything emits an **event-log** entry: SAN swings are the emotional heart of the game,
  and Keepers will demand the audit trail ("when did Harvey lose those 14 points?").

**Rules-engine cautions**

- The INT-roll inversion (success → insane) deserves a loud comment and a test.
- The §5.4 precedence needs explicit tests for the simultaneous case.
- The bout-immunity gate sits at the *top* of the pipeline, ahead of any dice.
- Mythos-driven ceiling clamps bypass the loss pipeline entirely.
- Phobia/mania penalty dice are situational — offer as a suggestion for Keeper
  confirmation, never auto-applied.

---

## Open questions

1. **`dailyLimit` snapshot vs. live.** The text says "one fifth of *current* Sanity."
   Snapshotting at day reset (Foundry's approach) is stable and predictable; live
   recomputation drops the bar as the investigator bleeds. Which does CoC Phoenix
   implement, and is it configurable?
2. **`dailyLimit == 0` at very low SAN.** Floor it at 1, or let any loss trigger indefinite
   insanity?
3. **What is a "day"?** Purely Keeper-invoked reset, or should the tracker offer a
   scene/session heuristic (e.g. reset on a "the party rests safely" event)?
4. **Source-key normalization for hardening.** Free text (Keeper types "deep one") versus a
   controlled vocabulary of creature IDs. Free text flexes but fragments counters through
   typos; a vocabulary needs a homebrew fallback.
5. **Is the involuntary action a blocking step?** Mechanically weightless, but making it a
   checkpoint turns one API call into two.
6. **Who controls the character during a bout?** Keeper-controlled, player-controlled with
   prompts, or per-table. This touches actor permissions, not just presentation.
7. **Multi-actor Sanity checks.** A room of investigators sees the same horror. Fan out one
   `SanityCheck` per actor from a single Keeper action? How is the aggregate presented?
8. **Pulp Cthulhu scope for v1** — is Luck-halves-loss in or out? It alters the loss
   pipeline and requires a mid-check Luck-spend prompt.
9. **Backstory entries as first-class data.** Self-Help, key connections, and
   backstory-corrupting bout results all assume structured backstory records, not a prose
   blob. Is that in scope — and does it belong in the character-sheet doc instead?

---

## Sources

- Chaosium, *Call of Cthulhu Keeper Rulebook*, 7th Edition — Sanity chapter. Approximate
  references: Sanity rolls and loss notation ~pp. 154–156; temporary and indefinite
  insanity ~pp. 155–159; bouts of madness tables (real time and summary) ~pp. 157–160;
  phobia and mania sample tables ~pp. 164–166; Sanity recovery, psychotherapy, and
  institutionalization ~pp. 162–164; getting used to awfulness, Insane Insight, and Mythos
  Hardened optional rules ~pp. 168–169.
- Chaosium, *Investigator Handbook*, 7th Edition — character creation (SAN = POW),
  backstory entries, key connections, and the development phase.
- The Call of Cthulhu RPG Wiki (official Chaosium wiki), Sanity page —
  https://cthulhuwiki.chaosium.com/rules/sanity.html
- Chaosium 7th Edition free PDFs / Quick-Start Rules — https://www.chaosium.com/7th-edition-pdfs/
  (Quick-Start also mirrored at https://archive.org/details/cha-23131-call-of-cthulhu-7th-edition-quick-start-rules)
- TRPGLine CoC 7th system rules reference, Sanity — https://trpgline.com/rules/coc7/sanity
- Miskatonic Investigative Society, CoC7 system for Foundry VTT (unofficial but
  well-maintained reference implementation) —
  https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT
  Specifically `coc7/apps/san-check-card.js` (the sanity-check state machine),
  `coc7/models/actor/document-class.js` (`setSan`, `loseSan`, `resetDailySanity`,
  `maxLossToSanReason`, `enterBoutOfMadness`, `useMythosHardened`).
- "Call of Cthulhu 7th Ed — Insanity Q&A", 21st Century Philosopher —
  https://morganhua.blogspot.com/2019/11/call-of-cthulhu-7th-ed-insanity-q.html
- RPGnet forum thread on 7e Sanity rules interpretation —
  https://forum.rpg.net/index.php?threads/call-of-cthulhu-i-dont-fully-understand-the-sanity-rules-7th-edition.909418/
- The Role Players Guild, maximum Sanity and Cthulhu Mythos —
  https://theroleplayersguild.com/articles/coc-7e/sanity-system/max-sanity-mythos-skill
