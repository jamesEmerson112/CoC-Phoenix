# Character Creation & Occupations (Call of Cthulhu 7e)

Building a 7e investigator is a short, strictly ordered pipeline of derivations: roll eight characteristics, choose an age and apply its modifier block, derive secondary attributes from the (post-age) characteristics, pick an occupation whose formula turns characteristics into a skill-point pool, spend that pool plus a second pool from INT, then write a structured backstory. Nearly every value on the finished sheet is a pure function of earlier values — so a tracker backend should persist the *inputs* (dice results, age, occupation id, allocation deltas, player choices) and treat the sheet as a computed projection; anything else silently corrupts derived fields the moment an in-play EDU gain or SIZ change forces a recalculation. Ordering is load-bearing: age modifies characteristics, and characteristics feed both the derived attributes and the occupation point formula, so applying age *after* computing occupation points yields a different (wrong) character. This doc covers the mechanics a rules engine must implement, the validation invariants, and the state that outlives creation.

---

## 1. Creation pipeline (ordered)

The canonical sequence per the Investigator Handbook / Keeper Rulebook:

1. **Roll characteristics** (8 values).
2. **Choose age** (15–89 typical) and apply the age modifier block, including EDU improvement checks.
3. **Compute derived attributes** (HP, MP, SAN, Luck, MOV, Build, Damage Bonus) from post-age characteristics.
4. **Choose occupation** → yields the occupation skill-point pool, a Credit Rating range, and a set of occupation skill slots.
5. **Allocate occupation points**, including the mandatory Credit Rating allocation inside its range.
6. **Allocate personal interest points** (INT × 2) to any skill except Cthulhu Mythos.
7. **Write backstory** (structured entries; nominate a key connection).
8. **Finishing touches**: name, residence, birthplace, gender, portrait, starting gear derived from Credit Rating, "fellow investigators" links.

For a backend this is best expressed as an explicit **creation state machine**, one step per stage: steps 4–6 are the ones players revisit, and steps 1–3 must be re-runnable (re-derivation) without wiping the step 5–6 allocations.

---

## 2. Step 1 — Characteristics

Characteristics are percentile values (nominally 0–99 for humans; rolled values top out lower).

| Characteristic | Roll | Rolled range |
|---|---|---|
| STR, CON, DEX, APP, POW | 3D6 × 5 | 15–90 |
| SIZ, INT, EDU | (2D6 + 6) × 5 | 40–90 |

Luck rolls like the 3D6 group (3D6 × 5) but is an attribute, not a characteristic — notably it can never feed an occupation formula.

**Alternative generation methods** (optional rules, Investigator Handbook ~p. 58), which deserve first-class modes rather than hacks:

- **Point buy** — distribute 460 points across the eight characteristics.
- **Fixed array ("quick fire")** — assign 40, 50, 50, 50, 60, 60, 70, 80 to the eight slots in any order.
- **Roll and assign** — roll the eight dice expressions, then let the player map results onto slots (a variant of straight rolling).

Each mode implies its own validation: point buy needs a running total plus per-characteristic floors/ceilings; the fixed array needs a bijection check between the value multiset and the eight slots; straight rolling needs the individual die results retained for audit.

**Derived percentile tiers.** Every characteristic and skill carries a *hard* value, a *half* value (floor(v/2)), and a *fifth* value (floor(v/5)). The roll engine references these constantly, and several skill bases are themselves characteristic derivations (Dodge starts at half DEX, Language (Own) starts at EDU) — recompute them whenever the source characteristic moves.

---

## 3. Step 2 — Age and its modifiers

Age is chosen (typically 15–89) and drives a modifier block applied to the rolled characteristics. Paraphrased mechanically, the brackets behave like this:

| Bracket | EDU improvement checks | Physical point deduction | APP penalty | MOV penalty | Extra |
|---|---|---|---|---|---|
| 15–19 | 0 | 5 points split between STR and SIZ | — | — | EDU −5; roll Luck twice, keep the better |
| 20–39 | 1 | — | — | — | — |
| 40–49 | 2 | 5 points split among STR/CON/DEX | −5 | −1 | — |
| 50–59 | 3 | 10 points | −10 | −2 | — |
| 60–69 | 4 | 20 points | −15 | −3 | — |
| 70–79 | 4 | 40 points | −20 | −4 | — |
| 80–89 | 4 | 80 points | −25 | −5 | — |

Two things make this awkward in code:

- **The physical deduction is a player-directed split**, not an automatic subtraction. The engine must accept an allocation object (e.g. `{str: 10, con: 5, dex: 5}`) and verify it sums to the bracket total without dragging any characteristic below its floor (treat 1 as the practical floor; 0 STR/CON/DEX is a debilitated/dead state, not a legal investigator).
- **The teen bracket splits across STR and SIZ specifically**, not the usual STR/CON/DEX trio, and additionally imposes a flat EDU penalty and a Luck advantage roll. It is genuinely a different shape, so a "deduct N points from a list" abstraction must carry the list as bracket data.

**EDU improvement check** (the same procedure the post-scenario development phase reuses): roll D100; if the result is *strictly greater* than current EDU, raise EDU by 1D10; otherwise nothing. EDU caps at 99. A bracket's check count means running the procedure N times sequentially — each check reads the EDU the previous one produced, so they can be neither batched nor reordered. Log every check's die results; players notice when a "roll twice" quietly became "roll once."

Edge cases: age can push a characteristic below its normally-rolled minimum; EDU can pass its rolled maximum of 90 via improvement checks but never 99; MOV penalties stack on top of the base MOV computation (they are not a separate MOV bracket).

---

## 4. Step 3 — Derived / secondary attributes

Computed from the post-age characteristics:

- **Hit Points** = floor((CON + SIZ) / 10). This also fixes the major-wound threshold (half HP, rounded down) used by the combat/damage system.
- **Magic Points** = floor(POW / 5).
- **Sanity** starts at POW. Maximum Sanity = 99 − Cthulhu Mythos skill — which is exactly why Mythos is locked at 0 during creation (§6).
- **Luck** = 3D6 × 5, with the young-investigator advantage roll noted above.
- **Movement Rate (MOV)**: compare STR and DEX to SIZ. Both below SIZ → 7. Both above → 9. Anything else (ties included) → 8. Then subtract the age MOV penalty. The quick-start rules flatten this to 8 for all humans, so a backend serving both rule sets needs a toggle.
- **Build and Damage Bonus**: one lookup on STR + SIZ.

| STR + SIZ | Damage Bonus | Build |
|---|---|---|
| up to 64 | −2 | −2 |
| 65–84 | −1 | −1 |
| 85–124 | none | 0 |
| 125–164 | +1D4 | 1 |
| 165–204 | +1D6 | 2 |

The table climbs in bands (205–284 → +2D6/3, 285–364 → +3D6/4, 365–444 → +4D6/5), then extends roughly +80 per step, adding a D6 and a Build point each time. Monsters and vehicles use Build too, so implement this as a shared lookup over an arbitrary STR+SIZ total, never an investigator-only table — and store Damage Bonus as a structured dice expression (`{count, sides, sign}` or `null`) rather than a display string, because the combat resolver has to roll it.

---

## 5. Step 4 — Occupations

An occupation contributes three things: a **skill-point formula**, a **Credit Rating range**, and a **set of skill slots**.

### 5.1 Skill-point formula

Formulas are sums of `characteristic × multiplier` terms, some **mandatory** and some forming a **mutually exclusive optional group** the player picks from at creation. In practice 7e only uses two shapes:

- `EDU × 4` — the "pure academic/professional" shape.
- `EDU × 2 + X × 2`, X being one choice from a small set (commonly STR, DEX, APP, or POW).

Some occupations give no choice (`EDU × 2 + APP × 2`), some two options, some three (`EDU × 2 + (APP or DEX or STR) × 2`).

Don't hard-code the two shapes — encode the general form. The Foundry VTT CoC7 occupation item does precisely this: each characteristic stores `{multiplier, selected, optional}`, so a formula is the sum of all non-optional selected terms plus exactly one selected-optional term. That representation absorbs homebrew occupations and odd supplements for free.

Illustrative shape of the occupation catalogue (reworded; ~6 of the 40+ published entries):

| Occupation | Point formula | Credit Rating range |
|---|---|---|
| Antiquarian | EDU × 4 | 30–70 |
| Dilettante | EDU × 2 + APP × 2 | 50–99 |
| Drifter | EDU × 2 + (APP or DEX or STR) × 2 | 0–5 |
| Private investigator | EDU × 2 + (DEX or STR) × 2 | 9–30 |
| Doctor of medicine | EDU × 4 | 30–80 |
| Artist | EDU × 2 + (POW or DEX) × 2 | 9–50 |

Validation-relevant facts: Credit Rating ranges cover the whole 0–99 space across the catalogue (drifters and missionaries bottom at 0; dilettantes and criminal bosses touch 95–99), so no global floor above 0 may be assumed. Occupations also carry **era/setting** tags (classic 1920s, modern, Lovecraftian/Gaslight, pulp), and the catalogue must filter on them — no "hacker" occupation in a 1923 campaign.

### 5.2 Occupation skill slots

A typical occupation names **eight skills plus Credit Rating**. Slots come in three kinds:

1. **Fixed** — a named skill, always included.
2. **Choose N from a listed group** — e.g. "one of Occult or Natural World."
3. **Choose N from anything** — the "personal specialty" slot, sometimes constrained to a family (one interpersonal skill; one language; one Art/Craft; one Science).

The occupation skill list is therefore not a flat array but a list of slot descriptors resolved into concrete skills at creation — and the resolution choices must persist, since they define which skills count as "occupational" for the rest of the character's life (which matters to the development phase).

### 5.3 Specialized skills

Several skills are families needing a specialization label: Art/Craft, Science, Fighting, Firearms, Language (Own) and Language (Other), Survival, Pilot. A skill instance is `(skillDefinitionId, specialization?)`, and two Science instances with different specializations are independent skills with independent values. Language (Own) derives its base from EDU; the rest use fixed printed bases. Uniqueness constraints must key on the pair, never on the definition id alone.

### 5.4 Creating a custom occupation

The rules explicitly bless building one: pick about eight thematically fitting skills, pick a formula (EDU × 4, or EDU × 2 plus one other characteristic × 2), set a Credit Rating range. Treat published and Keeper-authored occupations as one entity type with a `source` field — never separate hard-coded enums.

---

## 6. Step 5 — Skill allocation

Two pools, spent in order:

- **Occupation points** = the occupation formula's result. Spendable only on the occupation's resolved skill list (plus Credit Rating).
- **Personal interest points** = **INT × 2**. Spendable on any skill, including topping up occupation skills.

Rules the engine enforces:

- **Points add to the printed base.** 40 points into Spot Hidden (base 25) makes 65, not 40. Store the allocation delta separately from the computed total, per pool — it shows the player where a value came from and lets a base-value correction propagate.
- **Credit Rating must come from the occupation pool**, and its final value must land inside the occupation's declared range. This is the single mandatory allocation; everything else is discretionary. (Whether personal points may push CR past the occupation maximum is a known table-level ambiguity — see Open questions.)
- **Cthulhu Mythos is locked at 0** during creation, from both pools, absent explicit Keeper permission. Since max Sanity = 99 − Mythos, an accidental Mythos allocation silently caps the Sanity ceiling — make this a hard validation, not a warning.
- **No skill may end below its base value**, and points never move between pools once creation closes.
- **Unspent occupation points evaporate** — the pool doesn't carry into play. Surface the remainder prominently, but don't block completion on a nonzero one.
- **Optional 75% cap on starting skills** (Keeper Rulebook, "A Cap on Starting Skill Values," ~p. 48). Off by default; a campaign-level setting, since it changes what a legal sheet is.
- The **quick-start / starter-set variant** replaces the pools with a fixed value ladder assigned across the occupation skills — one skill at 70, two at 60, three at 50, three at 40 (Credit Rating consumes one slot), plus four non-occupation skills at +20 over base. Supporting the free quick-start rules means this is an alternate allocation strategy, not a special case of the point pools.

---

## 7. Credit Rating, cash and assets

Credit Rating is simultaneously a skill (rollable in social/financial situations) and the driver of the investigator's economic state. It maps to a wealth tier, and the tier plus the CR value yields three sheet numbers: **spending level**, **cash on hand**, and **assets**.

| CR | Tier | Cash | Assets | Spending level |
|---|---|---|---|---|
| 0 | Penniless | trivial fixed amount | none | trivial |
| 1–9 | Poor | CR × 1 | CR × 10 | small fixed |
| 10–49 | Average | CR × 2 | CR × 50 | moderate fixed |
| 50–89 | Wealthy | CR × 5 | CR × 500 | large fixed |
| 90–98 | Rich | CR × 20 | CR × 2000 | very large fixed |
| 99 | Super rich | large fixed | very large fixed | very large fixed |

(The multipliers above use the 1920s currency scale; modern and other settings keep the tier structure with different absolute figures. Foundry's CoC7 stores this as an editable per-actor array of `{name, min, max, cashType, cashValue, assetsType, assetsValue, spendingType, spendingValue}`, where each `*Type` selects "multiply CR" versus "flat value" — a model worth copying, since it turns era swaps into data instead of code.)

Credit Rating moves in play (sprees, rewards, ruin), so the wealth tier and derived money fields are recomputed state: recompute cash/assets on CR change while letting the Keeper override manually (Foundry exposes a `manualCredit` flag for exactly this).

---

## 8. Step 6 — Backstory

The 7e sheet carries a structured backstory block: six sections filled at creation, four accumulating in play. The books treat all ten as free text, but a tracker earns real value making them typed, ordered, individually addressable records.

**Filled at creation:**

1. **Personal description** — appearance and manner.
2. **Ideology / beliefs** — one line; what the investigator holds true.
3. **Significant people** — who, and *why* they matter (the "why" is a separate prompt in the rules; keep it a separate field).
4. **Meaningful locations** — places of emotional weight.
5. **Treasured possessions** — objects of emotional weight (distinct from inventory; a treasured possession may or may not be a carried item).
6. **Traits** — habits, mannerisms, temperament.

**Accumulated in play:**

7. **Injuries & scars** — lasting physical consequences.
8. **Phobias & manias** — acquired from bouts of madness; the rulebook offers sample phobia and mania tables (Keeper Rulebook ~pp. 159–162) a tracker can roll on.
9. **Arcane tomes, spells & artifacts** — what the investigator has read, learned, or acquired. Overlaps the Mythos-tome subsystem (reading a tome grants Mythos skill, costs Sanity, unlocks spells), so link to tome and spell entities rather than keeping pure prose.
10. **Encounters with strange entities** — which monsters have been seen. Mechanically relevant: repeat exposure to a creature already survived is handled differently by the Sanity rules.

**Key connection.** One backstory entry — usually a significant person — is designated the investigator's *key connection* (Investigator Handbook ~p. 56): a single-select flag across the backstory collection. Mechanically it participates in recovery from indefinite insanity: time with / reunion with the key connection permits a Sanity roll — success restores a little Sanity, failure severs the connection (the entry is struck from the sheet and no key connection exists until a new one is established). Backstory entries therefore need a **status** (`active` / `struck` / `revised`) and an audit trail, not just text.

Backstory is also a Keeper lever: bouts of madness append phobia or mania entries, and Keepers are encouraged to threaten or damage entries as plot. Entries are thus **mutable in play, from the Keeper's side**, and should carry `createdAt`, `createdBy`, and `sourceEvent` (e.g. the Sanity-loss event that produced the phobia).

---

## 9. Pulp vs classic (brief)

Pulp Cthulhu keeps the characteristic rolls and the occupation/skill machinery, layering on:

- **Archetype**, chosen *before* characteristics and stacking with occupation. It names a **core characteristic** (often a choice of two) rolled as 1D6 + 13 then × 5 — i.e. 70–95 instead of the usual ceiling; grants **100 bonus skill points** across the archetype's listed skills (a *third* pool beside occupation and personal interest); grants **talents** (two by default, tunable via the "pulp-o-meter"); and suggests occupations and traits.
- **Double hit points.**
- **No age modifiers** — age becomes purely descriptive; the whole §3 block is skipped.
- Softer Luck and Sanity handling (Luck is spendable and recovers; several optional rules let Luck buy off death or heal).

Backend impact is small but structural: the point-pool set becomes a map (`occupation`, `personalInterest`, `archetype`, `experiencePackage`) rather than two fixed fields; talents are an item collection on the character; and the age-modifier stage turns conditional on game mode. Foundry's CoC7 models exactly this with a `development: {personal, occupation, archetype, experiencePackage}` record and a separate `archetype` item type.

---

## 10. Validation invariants

A creation-time validator should assert:

- Every characteristic is inside its legal range for the chosen generation mode; point-buy totals exactly 460; fixed-array assignment uses each value exactly once.
- Age lies in 15–89 and matches exactly one bracket; the physical-deduction allocation sums to the bracket total and touches only that bracket's permitted characteristics; no characteristic ends below 1.
- The number of EDU improvement checks equals the bracket's count; EDU ≤ 99.
- The occupation exists, is legal for the campaign era, and exactly one option is selected from each optional characteristic group in its formula.
- Occupation points spent ≤ pool; personal interest spent ≤ INT × 2.
- Occupation points land only on resolved occupation skills; Credit Rating's final value sits within `[crMin, crMax]`.
- Cthulhu Mythos allocation is 0 (unless the Keeper override flag is set); Sanity ≤ 99 − Mythos.
- Every skill total ≥ its base; every specialized skill instance carries a non-empty specialization; no duplicate `(skillId, specialization)` pairs.
- At most one backstory entry carries the key-connection flag.
- Derived attributes match a recomputation from stored characteristics (nothing hand-edited without an override flag).

---

## Backend implications

**Entities**

- `Investigator` — identity fields (name, player, occupation label, age, sex, residence, birthplace, archetype, organization) plus the computed sheet.
- `CharacteristicSet` — the eight values, each with `rolled`, `ageDelta`, `inPlayDelta`, `current`, and cached half/fifth. Keeping the deltas apart is what makes age re-application and in-play changes safe.
- `DerivedAttributes` — HP (current/max), MP (current/max), SAN (current/starting/max), Luck, MOV, Build, DamageBonus. All recomputed; each field carries a `manualOverride` flag.
- `OccupationTemplate` — `{id, name, era[], source, formulaTerms: [{characteristic, multiplier, optional, groupId}], creditRating: {min, max}, skillSlots: [{kind: fixed|chooseFromGroup|chooseAny, count, candidates[]}]}`.
- `SkillDefinition` — `{id, name, baseValue | baseFormula, specializable, family, isMythos, isCreditRating, eras[]}`.
- `SkillInstance` (per investigator) — `{definitionId, specialization?, base, occupationPoints, personalPoints, archetypePoints, developmentGains, total, isOccupational, hasSucceededThisSession}`.
- `BackstoryEntry` — `{type: enum(10), text, subject?, reason?, order, isKeyConnection, status, createdAt, sourceEventId}`.
- `CreditProfile` — `{creditRating, tierId, cash, assets, spendingLevel, manual}` plus an `EraEconomy` lookup keyed by campaign era.
- `CreationSession` — the state machine: `{step, mode(classic|pulp|quickstart), generationMethod, diceLog[], pendingChoices[], validationErrors[], completedAt}`.

**Behaviour**

- Derivation is a **pure recompute function** over `(characteristics, age, occupation, allocations, mode)`. Every source write triggers a recompute; every derived field can be shadowed by an explicit override that survives recomputes.
- Persist the **dice log** (each characteristic roll, each EDU improvement check, the Luck advantage roll). These get disputed, and post-hoc reconstruction is impossible without it.
- Persist **player choices** as first-class records: which optional characteristic fed the occupation formula, how the age deduction was split, which skills filled each choice slot. These are inputs, not results.
- Keep the occupation catalogue, skill catalogue, age-bracket table, Build/DB table, and Credit Rating economy as **seed data**, era-tagged and versionable — Keepers homebrew all five.
- Creation is not a one-shot: support **re-opening** a finished investigator (Keepers fix mistakes mid-campaign) with an audit entry, and support **partial completion** so a session can be saved half-built.
- The last four backstory categories and the `status` field bridge into the in-play systems (Sanity, insanity, tomes, bestiary). Wire `BackstoryEntry.sourceEventId` to the Sanity/madness event log from day one; retrofitting provenance later hurts.

---

## Open questions

1. **Personal interest points and Credit Rating** — real tables read this both ways: some restrict CR funding to occupation points inside the occupation range; others let personal points push it higher. Needs to be a campaign setting with a documented default, not a silent engine choice.
2. **Exact floors/ceilings in point-buy mode** — the 460-point method's per-characteristic minimum and maximum need confirmation against the Investigator Handbook before the validator hard-fails on them.
3. **Rounding conventions** — round down for HP/MP is stated; confirm the intended behaviour for half/fifth of odd numbers everywhere they occur, and pick one shared helper.
4. **Age deduction floor** — the books never clearly state the minimum a characteristic may be reduced to by age. Is 1 the floor, or 0 — and is a 0-STR investigator a legal (if pitiable) sheet?
5. **Key connection scope** — must it be a Significant People entry, or may it be any backstory entry (a location, a possession)? Determines whether `isKeyConnection` is constrained by entry type.
6. **Struck backstory entries** — soft-delete a severed key connection (retain with `status: struck`) or remove it? Soft delete is the recommendation for narrative continuity, but the UI needs a rule on whether struck entries remain visible to the player.
7. **Era/setting scoping of the catalogue** — how far to go: Gaslight, Dark Ages, Down Darker Trails, or just classic 1920s + modern + pulp? Decides whether era is a tag or a full ruleset dimension.
8. **Quick-start vs full rules** — does the tracker support the free quick-start allocation ladder at all? If yes, the allocation subsystem needs a strategy interface, not a single point-pool implementation.
9. **Homebrew occupation authoring** — do Keepers get an editor (formula builder, slot builder, CR range)? If so, what validation blocks degenerate occupations (EDU × 10)?
10. **Mythos-tome ↔ backstory linkage** — how tightly to couple "arcane tomes & spells" backstory entries to tome/spell entities. Full linkage is cleaner but requires the tome subsystem to exist before creation ships.

---

## Sources

- Chaosium, *Call of Cthulhu RPG Wiki* — investigator creation steps: <https://cthulhuwiki.chaosium.com/investigators/> (step one characteristics, step two secondary attributes, step three occupation and skills, step four backstory, step five final touches).
- Chaosium, *Call of Cthulhu 7th Edition Quick-Start Rules* (free PDF): <https://www.chaosium.com/content/FreePDFs/CoC/CHA23131%20Call%20of%20Cthulhu%207th%20Edition%20Quick-Start%20Rules.pdf>; landing page <https://www.chaosium.com/cthulhu-quickstart/>.
- Chaosium, *Call of Cthulhu 7th Edition Index for Keepers and Investigators* (free PDF) — used for the page references below: <https://www.chaosium.com/content/FreePDFs/CoC%207/Call%20of%20Cthulhu%207th%20Edition%20Index%20for%20Keepers%20and%20Investigators.pdf>.
- *Call of Cthulhu Keeper Rulebook* (7e) page references: characteristics pp. 30–32; age pp. 32–33; Table I Damage Bonus and Build p. 33; movement rate p. 33; occupation skills p. 32, 36; personal interest skills p. 31, 36; sample occupations pp. 40–41; significant people p. 43; meaningful locations and treasured possessions p. 44; traits p. 45; cash pp. 45–46; cap on starting skills (optional rule) p. 48; Credit Rating skill p. 61; Cthulhu Mythos skill pp. 61–62; maximum Sanity p. 155; phobia/mania pp. 159–162 (sample tables pp. 160–161); indefinite insanity pp. 156, 164–166; backstory pp. 39, 95, 193–194.
- *Call of Cthulhu Investigator Handbook* (7e) page references: age pp. 45, 48–49; EDU improvement check p. 48; derived attributes (HP, Build, Damage Bonus, MOV) p. 49; occupation pp. 49–50, 68ff; occupation skills pp. 45, 49, 68; personal interest skills pp. 45, 50; Credit Rating pp. 49–51, 68; base skill values pp. 50, 97; skill specialization p. 96; backstory pp. 51ff; ideology/beliefs p. 53; significant people pp. 53ff; meaningful location pp. 55–56; traits p. 56; key connection p. 56; cash and assets pp. 57–58; optional creation rules p. 58; example of investigator creation pp. 47–48.
- Roll20 Call of Cthulhu Compendium — occupation formulas and Credit Rating ranges: <https://roll20.net/compendium/coc/Step%20Four%20(Determine%20Occupation)>; skill allocation: <https://roll20.net/compendium/coc/Step%20Five%20(Decide%20Skills%20and%20Allocate%20Skill%20Points)>; Pulp characteristics: <https://roll20.net/compendium/coc/Step%20Two%20(Generate%20Characteristics)>; Pulp archetypes: <https://roll20.net/compendium/coc/Step%20One%20(Choose%20a%20Pulp%20Archetype)>.
- Foundry VTT CoC7 system (unofficial, actively maintained) — data model reference for occupation formula terms, Credit Rating min/max, wealth tiers, point pools, and backstory/biography fields: <https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT> (see `coc7/models/item/occupation-system.js`, `coc7/models/actor/character-system.js`, `docs/en/item_occupation.md`, `docs/en/item_archetype.md`).
- `ajwalkiewicz/cochar` character generator — cross-check for the age modifier table, Build/Damage Bonus bands, and the occupation → formula-group / Credit Rating range mapping: <https://github.com/ajwalkiewicz/cochar> (see `cochar/__init__.py` `MODIFIERS`/`VALUE_MATRIX` and `cochar/data/occupations.json`).
- The Role Players Guild, CoC 7e occupations guide (occupation point spending, unspent points, personal interest scope): <https://theroleplayersguild.com/articles/coc-7e/character-creation/occupations>.
