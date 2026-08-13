# Existing CoC 7e Digital Tools & Data Models

> Companion to `10-…` through `19-…` in this folder. Those files hold authority on **rules**;
> this one covers how *other software* has already modelled those rules — entity shapes, JSON
> keys, dice notation, and API patterns worth stealing or avoiding. Where a rule appears here
> it exists only to explain a modelling choice; the rules docs win on detail.

## Overview

CoC 7e software falls into roughly four families: (1) full VTT game systems, of which the
open-source **Foundry VTT CoC7 system** by the Miskatonic Investigative Society is by far the
most complete and the single best data-model reference; (2) **Roll20 character sheets** (one
community HTML/attribute sheet plus an official Roll20-authored one) — essentially flat
key/value attribute bags driven by roll-template macros; (3) **The Dhole's House**, a
Chaosium-endorsed free web toolkit whose JSON blob has become the de-facto interchange format
for investigator data because other tools import it; and (4) a long tail of small open-source
apps — chase trackers, PDF-filling character creators, Discord dice bots — mostly
single-purpose and short-lived. The same handful of hard problems recurs across all of them:
representing a d100 roll built from a *tens* die and a *units* die (including the `00`+`0` =
100 edge case naive `kh/kl` dice notation gets wrong), representing user-extensible skills
with specializations, keeping derived stats recomputable without clobbering manual overrides,
and making a roll a *stateful, resumable object* rather than a fire-and-forget number —
because pushing, after-the-fact bonus/penalty dice, and Luck spending all mutate a roll after
it lands.

---

## 1. Tool survey

| Tool | Type | Source | Maturity (checked 2026-08) | Why it matters here |
|---|---|---|---|---|
| **Foundry VTT CoC7** (`Miskatonic-Investigative-Society/CoC7-FoundryVTT`) | Full game system | Open (GitHub), ~163★, v8.15, active | Actively pushed 2026-07 | The reference data model; ~15 document types, full dice engine, importers |
| **Roll20 CoC 7th Ed** (community sheet) | HTML character sheet | Open (`Roll20/roll20-character-sheets`) | Maintained via the shared sheet repo | Cautionary tale: flat, hardcoded attribute namespace |
| **Roll20 "Call of Cthulhu 7E by Roll20"** | Official licensed sheet | Closed | Sold with Roll20 modules | Shows the commercially-blessed feature set (compendium-linked sheets) |
| **The Dhole's House** (`dholeshouse.org`) | Web investigator toolkit | Closed, free, Chaosium-promoted | Live | De-facto interchange JSON; import target for Foundry |
| **Chase Tracker** (`coulgreer/call-of-cthulhu_chase-tracker`) | Single-purpose tracker | Open, TypeScript | Low activity (2024) | Evidence that chases deserve a separate stateful entity |
| **CoC Character Creator** (`apoapostolov/…`) | PDF-filling generator | Open, TypeScript | Active (2026) | Char-gen as a pipeline terminating in a filled PDF |
| **`coc-dice-bot`, `cthulhubot`, Dorian, etc.** | Discord dice bots | Mixed | Varies | Command-string DSLs for checks; useful for a text/chat API surface |
| **`cochar`** (PyPI) | Character generator library | Open, Python | Varies | Random investigator generation as a library, not an app |
| **Basic Roleplaying ORC Content Document** | Rules text | ORC-licensed, free PDF/RTF | Current | The only *legally reusable* d100 rules text; useful for seeding content |

**Note on "abandoned":** the CoC7 Foundry repo contains a `.github/ABANDONED.md` that is
*not* a project-status notice — it's an auto-generated list of untranslated i18n keys per
locale. The system is actively maintained (v8.15, Foundry v12–v14 compatibility). Don't
mistake it for a dead project.

---

## 2. Foundry CoC7: the actor/item data model

Foundry v12+ systems declare schemas as `DataModel` classes
(`coc7/models/actor/*-system.js`, `coc7/models/item/*-system.js`) rather than the older
single `template.json`. Two structural decisions dominate the design and are the primary
candidates for borrowing.

### 2.1 Actor types

Five actor types over a shared base (`global-system.js`):

- `character` — player investigator; the full model (money, development ledger, backstory,
  books)
- `npc` — humans/cultists; adds `special.sanLoss.{checkPassed,checkFailled}` (sic),
  `attacksPerRound`, macro hooks
- `creature` — mythos entities; thinnest, reusing the shared characteristic/attrib blocks
- `vehicle` — no characteristics at all; `stats.{hp, mov, build.{value,current},
  armor.{value, localized, locations[]}}`
- `container` — pure inventory holder

Lesson: **a vehicle is not a degenerate character.** It has Build (current vs max — Build
degrades in a chase or crash), optional hit locations, and no SAN/POW. A backend forcing one
`Actor` table with 80% nullable columns will suffer; a shared "combatant" core plus per-type
extension sits closer to reality.

### 2.2 Shared attribute block (`defineSchemaAttribs`)

```jsonc
attribs: {
  hp:    { value: null, max: null, auto: true },
  mp:    { value: null, max: null, auto: true },
  lck:   { value: null },                                  // no max — Luck is spent, not damaged
  san:   { value: null, max: 99, dailyLoss: 0, dailyLimit: 0, auto: true },
  mov:   { value: null, auto: true, type: "walk" },
  db:    { value: null, auto: true },                      // STRING: "-2" | "0" | "1D4" | "2D6"
  build: { value: null, auto: true },                      // integer, can be negative
  armor: { value: null, notes: false }                     // string: may be a formula, not a number
}
```

Three things to copy: (1) **the per-derived-stat `auto` flag** — every derived value declares
"recompute me from characteristics" versus "a human typed this"; without it, importing an NPC
stat block whose HP defies SIZ+CON gets silently overwritten. (2) **`db` is a string** —
damage bonus runs `-2`, `-1`, `0`, `1D4`, `1D6`, `2D6`, …: a flat integer at the bottom of
the ladder, a dice expression at the top; any numeric column is wrong, and the same holds for
`armor.value`, which may be a formula. (3) **SAN carries session-scoped counters**
(`dailyLoss`, `dailyLimit`) alongside `value`/`max`, because the "one-fifth of starting SAN
in a day → indefinite insanity" rule needs a running total that resets on a game-time
boundary the app has to define.

### 2.3 Conditions

`conditions` is a flat boolean set plus one duration-bearing entry:

```jsonc
conditions: {
  criticalWounds: { value: false },
  unconscious:    { value: false },
  dying:          { value: false },
  dead:           { value: false },
  prone:          { value: false },
  tempoInsane:    { value: false, realTime: false, duration: 0 },  // duration in rounds or real minutes
  indefInsane:    { value: false }
}
```

Temporary insanity needs a *unit discriminator* (`realTime`) because the rules express its
duration in combat rounds or real-world minutes depending on the situation. Duration-bearing
and instantaneous conditions want the same table but different lifecycles.

### 2.4 Items are documents, not columns

Everything list-like is its own embedded document type: `skill`, `weapon`, `armor`, `spell`,
`book`, `talent`, `status` (manias/phobias), `occupation`, `archetype`,
`experience-package`, `setup`, `chase`, plus a generic `item`. The opposite of the Roll20
approach, and clearly correct: investigators pick up arbitrary skills, and no fixed skill
column set survives era supplements.

**Skill** (`skill-system.js`) — the most instructive schema:

```jsonc
{
  skillName: "Spot Hidden",        // the base name
  specialization: "",              // e.g. "Fighting" in "Fighting (Brawl)"; name = "Spec (Skill)"
  base: "25",                      // string: may be a formula like "DEX/2" or "EDU"
  adjustments: {                   // ← a LEDGER, not a single total
    base: 0, personal: 0, occupation: 0,
    archetype: 0, experiencePackage: 0, experience: 0
  },
  properties: {
    noxpgain, special, rarity, push, fighting, firearm, ranged,
    requiresname, picknameonly, own, keepbasevalue
  },
  flags: { archetype, developement, experiencePackage, occupation }
}
```

The **adjustments ledger** is the pattern to steal: a skill's effective value is literally
`sum(Object.values(adjustments))`. Every point's origin is preserved forever — char-gen
spending can be audited or undone, occupation points can be re-allocated, and post-scenario
experience gains stay distinguishable from starting values. A lone `value: 55` column throws
all of that away and makes "how many occupation points have I spent?" unanswerable.

Also note `properties.push` is *per-skill*: Fighting/Firearms/Dodge ship with pushing
disabled by default, since those rolls generally can't be pushed. Pushability is data, not
code.

**Weapon** (`weapon-system.js`):

```jsonc
{
  skill: { main: {name,id}, alternativ: {name,id} },     // weapons reference skills by id AND name
  range: { normal:{value,damage}, long:{value,damage}, extreme:{value,damage} },
  usesPerRound: { normal: "1", max: null, burst: null }, // strings — "1(3)" style values exist
  bullets: null,      // magazine capacity
  ammo: 0,            // currently loaded
  malfunction: null,  // roll >= this number = malfunction
  blastRadius: null,
  properties: { rngd, mnvr, thrown, shotgun, dbrl, impl, brst, auto,
                ahdb /* add half DB */, addb /* add DB */, slnt, spcl,
                mont, blst, stun, rare, burn },
  price: { standard: "...", modern: "...", pulp: "..." }  // keyed by ERA
}
```

Key modelling facts: damage is **per range band**, not per weapon; `addb`/`ahdb` are
booleans rather than a `+DB` baked into the damage string (the importer strips a literal
`+DB` suffix and sets the flag); `malfunction` is a threshold compared to the same d100 as
the attack; price is era-keyed.

**Spell**: `costs.{hitPoints, magicPoints, sanity, power, others}` are all *strings* (they
can be `1D6`, `POW/2`, or conditional), plus a `costList[]` of `{type, if, config}` for
conditional costs and a boolean map
`type.{bind,call,combat,contact,dismiss,enchantment,gate,summon}`.

**Book / tome study** — tracked on the *actor*, not the item, as `books[]`:

```jsonc
{ id, cocid, name, initialReading: false, fullStudies: 0,
  necessary: 4, progress: 0, units: "CoC7.weeks", spellsLearned: [] }
```

Progress accumulates until `progress >= necessary`, whereupon `fullStudies++` and the Mythos
gain / SAN loss fires (with a different gain when current Mythos sits below the book's
rating). A genuine long-running state machine spanning sessions — the backend needs it
first-class.

**Status**: manias/phobias as items with `type.{mania, phobia}` and a `source` string.

### 2.5 Content addressing: "CoC IDs"

The system layers a stable, human-readable ID over Foundry's random 16-char document IDs:

```
<docType>.<subtype>.<kebab-slug>      e.g.  i.skill.spot-hidden
                                            i.weapon.colt-1911a1
                                            a.npc.cultist
```

`docType` ∈ `a` (Actor), `i` (Item), `je` (JournalEntry), `m` (Macro), `rt` (RollTable),
`s` (Scene). The same CoC ID can resolve to different documents per **era** (`standard`/
1920s, `modern`, `pulp`, `downDarkerTrails`, `darkAges`, `regency`, plus `…Pulp` variants)
and per **language**, with fallback.

A strong pattern for a tracker backend: a stable slug namespace decoupled from database
primary keys, so a scenario/compendium can say "give them Spot Hidden" without knowing row
IDs — and so the 1920s and Dark Ages versions of a skill remain the *same* logical skill.

### 2.6 Derived-stat formulas as implemented

Transcribed from `models/actor/document-class.js` (see `10-characteristics-derived-stats.md`
for the rules-side treatment; this is how one shipping implementation resolves the edges):

| Value | Implementation |
|---|---|
| Build / DB | on `STR+SIZ`: `<65 → -2`; `<85 → -1`; `<125 → 0`; `<165 → 1 / 1D4`; else `floor((sum-45)/80)+1` and `floor((sum-45)/80)+"D6"` |
| HP max | `floor((SIZ+CON)/10)` (divisor 5 under the Pulp double-HP option, characters only) |
| MP max | `floor(POW/5)` |
| MOV | `9` if DEX>SIZ **and** STR>SIZ; `8` if DEX≥SIZ **or** STR≥SIZ; else `7`; then if age ≥ 40, `mov - floor(age/10) + 3` (skipped for creatures and under the ignore-age-penalties option); floored at 0 |
| SAN max | `max(99 - CthulhuMythos, 0)` |

Two details worth copying: the Build/DB ladder is *open-ended* (one formula continues past
2D6 instead of a lookup table capping out), and `prepareDerivedData` re-clamps `value ≤ max`
for HP/MP/SAN after every recompute — preserving "was at full" so raising max also raises
current.

---

## 3. The d100 dice engine (and where naive implementations break)

`coc7/apps/dice-pool.js` is the single most valuable file in the ecosystem for a rules
engine.

### 3.1 Roll construction

A CoC 7e d100 is a **tens die** (custom d10 with faces reading 00–90) plus a **units die**
(d10 reading 0–9). Foundry defines a custom die term (`CoC7DecaderDie`, notation `dt`) and
rolls `1dt+1d10`. Faces run 1–10 internally with 10 mapped to 0. Each candidate total is:

```
tens = (raw === 10 ? 0 : raw * 10)
unit = (rawUnit === 10 ? 0 : rawUnit)
total = (tens === 0 && unit === 0) ? 100 : tens + unit
```

### 3.2 The edge case every naive implementation gets wrong

Bonus/penalty dice add extra **tens** dice, keeping lowest/highest. The common Roll20-style
macro reads:

```
/r ((3d10kh1)-1)*10 + (1d10-1)      // "2 penalty dice"
```

That applies keep-highest/lowest to the **raw tens dice**, then combines — wrong whenever
the units die shows 0, because `00 + 0` reads as **100**, not 0, so the *lowest* tens die
can produce the *highest* total. Example: bonus die, tens showing `00` and `40`, units `0`.
The real candidates are 100 and 40; a bonus die takes 40. Keep-lowest on the raw dice picks
the `00` die and yields 100 (or 10, depending on the `-1` offset variant) — the exact
opposite of the intent.

The correct algorithm (Foundry's): **materialize every candidate total first, then take
`min` (bonus) or `max` (penalty) over the totals.** For a plain roll with no modifier dice
the naive formula is statistically identical, so the bug stays hidden until modifiers
appear.

Backend implication: expose bonus/penalty dice as a **structured integer modifier**
(`poolModifier ∈ [-2, +2]`), never as client-composed dice-notation strings.

### 3.3 Success-level ladder and threshold ranges

Levels are an ordered enum (fumble deliberately far below failure so comparisons sort
right):

```
fumble: -99, failure: 0, regular: 1, hard: 2, extreme: 3, critical: 4
```

The Keeper's requested difficulty is a parallel enum: `unknown: -1, regular: 1, hard: 2,
extreme: 3, critical: 4, impossible: 9`. Success is `successLevel >= difficulty`, and the
*margin* (`successLevel - difficulty + 1`) renders as N stars — a nice touch for
opposed-roll adjudication.

The engine precomputes **inclusive integer ranges** per level instead of if/else per roll:

- `critical` → `[1, 1]`
- `extreme` → up to `floor(threshold / 5)`
- `hard` → up to `floor(threshold / 2)`
- `regular` → up to `threshold`
- `failure` → the gap up to `fumbleMin - 1`
- `fumble` → `[fumbleMin, 100]`

with `fumbleMin = 96` when the **effective threshold at the requested difficulty** is below
50, else `100`. Note the subtlety: the 50-point test uses the *difficulty-adjusted*
threshold — a Hard check tests `floor(skill/2) < 50`, Extreme tests `floor(skill/5) < 50`. A
70 skill fumbles only on 100 at Regular but on 96+ when the Keeper called Hard. Verify
against the rulebook before adopting; it's a defensible reading but an *interpretation*, and
the backend should make it a configurable rule flag.

Ranges collapse gracefully at the extremes (skill 1 leaves no room for hard/extreme bands;
the builder clamps each band to remaining space and stops). Test cases to write: skill 1, 2,
5, 9, 10, 49, 50, 99, 100, and >100.

### 3.4 Post-roll mutations — why a roll must be an entity

A CoC roll is not immutable once evaluated. The engine supports:

- **Adding bonus/penalty dice after the roll** (net ±2 max, only while no Luck has been
  spent). Extra tens dice are rolled up front and revealed as needed, keeping results
  honest. Bonus and penalty cancel 1-for-1; the store keeps separate
  `bonusCount`/`penaltyCount` plus a net `currentPoolModifier`.
- **Luck spending**: `total = clamp(total + flatDiceModifier - luckSpent, 1, 100)`. The
  engine precomputes `luckRequired{Regular,Hard,Extreme,Critical}` (points needed per band)
  and only offers a band when the actor holds *strictly more* Luck than the cost. Luck
  cannot buy out of a fumble, nor apply to Luck rolls, Sanity rolls, or weapon
  malfunctions — six exclusions in one method.
- **Pushing**: re-rolls the pool as a second dice group appended to the same roll object;
  the original stays visible. Blocked for Sanity rolls and non-pushable skills, and for
  fumbles by default (a settable house rule).
- **Forcing a result** (Keeper fiat): picks a uniform random total *inside* the target
  band, then back-solves plausible die faces so the displayed dice stay consistent.
- **`setSuccess`**: overrides success/failure without touching the level — automatic melee
  hits and weapon malfunctions bypass the ladder.

`isRolledSuccess` is tracked apart from `isSuccess`: a success counts toward a skill
improvement check only if *earned* — no Luck, no dice modifiers. Most implementations miss
this.

The pool serialises via `toObject()`/`fromObject()` behind an `isValidPool()` guard, and the
check serialises alongside it (`allowPush`, `appliedDevelopment`, `isForced`, `isStandby`,
`rollMode`, …). **A roll is a persisted, resumable aggregate** — the right model for a
multiplayer backend where the Keeper acts on a player's roll seconds later.

### 3.5 A roll-request DSL worth borrowing

Foundry's inline links are effectively a compact roll-request API:

```
@coc7.check[type:skill, name:Spot Hidden, difficulty:+, modifier:-1, blind, pushing]{Spot Hidden}
@coc7.check[type:characteristic, name:str, difficulty:++]{Strength}
@coc7.check[type:attribute, name:lck]{Luck}
@coc7.sanloss[sanMin:1, sanMax:1D6, sanReason:The thing in the well]
@coc7.item[...]        // weapon check
@coc7.effect[...]
```

- `difficulty`: `?` (unknown/blind to the player), `0` regular, `+` hard, `++` extreme,
  `+++` critical
- `modifier`: a signed dice count — `-2 … +2` — **not** a percentage modifier
- the check subject is `(type, name)`, type ∈ `characteristic | attribute | skill | item |
  value`

That triple — subject, difficulty, dice modifier — plus optional `blind`/`pushing` flags is
a complete minimal request payload for a `POST /rolls` endpoint. There is also a separate
**flat threshold modifier** (`flatThresholdModifier`) and **flat dice modifier**
(`flatDiceModifier`) — how percentage-style situational modifiers coexist with dice-based
ones.

---

## 4. Dice notation conventions in d100 tools

| Convention | Where | Meaning / caveat |
|---|---|---|
| `1D100` | Rulebooks | The standard check roll; written as tens + units |
| `1D100/5`, `1D3`, `1D6+DB` | Rulebooks | Division and `+DB` suffixes appear inline in damage/SAN entries; parsers must handle both |
| `1d100cs1cf100` | Roll20 community sheet | Roll20's "crit success on 1 / crit fail on 100" annotation; ~1,650 uses in that one sheet |
| `((3d10kh1)-1)*10 + (1d10-1)` | Roll20 forum idiom for penalty dice | Statistically plausible, **incorrect** for the `00`/`0` case (§3.2) |
| `1dt+1d10` | Foundry CoC7 | Custom "decader" die term; the system owns the 00-reading and min/max-over-totals logic |
| `+DB` / half-DB | Weapon damage | Modelled as booleans `addb` / `ahdb`, not string concatenation |
| Malfunction number | Firearms | A threshold on the *same* d100 as the attack roll, not a separate roll |
| `!roll` / `#probe` / `/check <skill>` | Discord bots | Chat-command DSLs; typically `skill value` + optional bonus/malus dice count |

Practical rule for a backend: accept dice **expressions** only where the rules genuinely
contain them (damage, SAN loss, spell costs, DB, some skill bases like `DEX/2`), and use
structured integers everywhere else. Wherever a rules value can be a formula, the column
must be a string plus an evaluator — and the evaluator needs the actor's characteristics in
scope.

---

## 5. Roll20 sheets — mostly a list of things not to do

The community **Call of Cthulhu 7th Ed** sheet is one ~1.2 MB HTML file with a flat
attribute namespace. Every skill is a hardcoded attribute, and every era variant clones it
with a suffix:

```
attr_accounting, attr_accounting_checkbox, attr_accounting_txt,
attr_accounting_da, attr_accounting_ic, attr_accounting_inv,
attr_accounting_mdr, attr_accounting_ow, …          (× ~90 skills)
attr_cthulhu_mythos, attr_cthulhu_mythos_da, attr_cthulhu_mythos_ic, …
attr_automatic_weapon1_ammo … attr_automatic_weapon5_ammo
```

(`_da` Dark Ages, `_ic` Cthulhu Invictus, `_inv` Invictus, `_mdr` modern, `_ow` Old West,
`_et` Gaslight/other era.) Weapons are numbered slots (1–5) rather than a collection.
Success levels are evaluated in the roll template with Mustache-ish helpers
(`{{#rollLess() roll1 96}}`, `{{#rollBetween() roll1 2 hard}}`,
`{{#rollGreater() roll1 extreme}}`) where `hard`/`extreme` are sheet-worker-computed
attributes, and fumble is chosen by `{{#rollLess() success 50}}` → 96 vs 100.

Takeaways: never model skills as columns (era support multiplies columns instead of being a
tag); never model collections as numbered slots (`weapon1…weapon5` caps the model
arbitrarily). Roll20 *does* get two things right worth copying — it precomputes and stores
`hard = floor(v/2)` / `extreme = floor(v/5)` so clients never reimplement rounding, and it
computes `sanity_max = 99 - CthulhuMythos` as a sheet attribute. It does **not** mechanically
implement bonus/penalty dice in its main roll — players bolt them on with ad-hoc macros — so
automating that correctly is a genuine differentiator.

The **official Roll20 CoC7E sheet** is closed-source and its help pages block automated
fetching, so treat any detail about it as unverified. Its observable value proposition is
compendium-linked drag-and-drop content — the same idea as Foundry's CoC IDs, and further
evidence that a stable content-slug namespace is table stakes.

---

## 6. The Dhole's House export — the de-facto interchange format

Chaosium promotes The Dhole's House as the free investigator toolkit; its JSON export is
what Foundry's `dholehouse-importer.js` consumes. Reconstructed shape:

```jsonc
{ "Investigator": {
    "PersonalDetails": { "Name", "Occupation", "Age", "Gender", "Residence", "Birthplace" },
    "Characteristics": {
      "STR","CON","SIZ","DEX","APP","INT","POW","EDU",
      "Sanity","SanityMax","HitPts","HitPtsMax","MagicPts","MagicPtsMax",
      "Luck","Move","DamageBonus","Build"
    },
    "Skills":     { "Skill": [ { "name", "subskill", "value", "occupation" } ] },
    "Weapons":    { /* { name, skillname, damage, ammo, malf } */ },
    "Possessions":{ "item": [ { "description" } ] },
    "Backstory":  { "description","traits","ideology","injurues","people",
                    "phobias","locations","tomes","possessions","encounters" }
} }
```

Interop hazards — all worked around by the Foundry importer, all waiting for a new backend's
importer:

- **Numbers arrive as strings** — everything gets `parseInt`ed on the way in.
- **A typo is part of the contract**: the injuries key is spelled `injurues`. It can't be
  fixed upstream; normalise on ingest.
- **Single-element collections may arrive as objects, not arrays** (`Possessions.item`,
  `Weapons`) — the importer defensively wraps non-arrays. Classic XML→JSON conversion
  artifact; the underlying export is XML-shaped.
- **Skill naming diverges** from the rulebook: `subskill: "None"` rows are *skipped*;
  `"None"` sometimes means "Any"; `Language (Own)` / `Language (Other)` need splitting into
  a base skill plus an `own` flag; `Operate Heavy Machine` must be renamed to `Operate Heavy
  Machinery`; a bare `Throw` becomes `Fighting (Throw)`.
- **Weapon damage carries `+DB` as a literal string suffix** to strip and convert into a
  boolean flag.
- **The daily SAN limit is not exported** — the importer derives
  `dailyLimit = floor(Sanity/5)` on ingest.

If this backend wants adoption, "paste your Dhole's House JSON" is the single
highest-leverage import path, and the normalisation rules above are effectively its spec.

---

## 7. Patterns: borrow vs avoid

**Borrow**

- The per-derived-stat `auto` flag (recompute vs manual override), plus clamp-to-max on
  every recompute.
- The adjustments ledger for skill values
  (`base/personal/occupation/archetype/package/experience`).
- Skills, weapons, spells, tomes, manias as first-class rows — never columns, never
  numbered slots.
- A stable slug namespace (`i.skill.spot-hidden`) decoupled from primary keys, with era +
  locale variants.
- Precomputed inclusive success-band ranges per check, served to clients so nobody
  re-derives rounding.
- The roll as a **persisted aggregate** supporting post-hoc mutation (bonus dice, Luck,
  push) and serialising/deserialising whole.
- `isRolledSuccess` (earned without Luck or dice modifiers) tracked separately from
  `isSuccess`, for experience eligibility.
- A structured `poolModifier: -2..+2` instead of client-supplied dice notation.
- Per-skill `push` capability and per-actor rule toggles (pulp/house rules) as data.
- The compact roll-request DSL: `(subjectType, subjectName, difficulty, diceModifier,
  flags)`.

**Avoid**

- Keep-highest/lowest on raw tens dice (§3.2). Compute totals first, then min/max.
- Numeric columns for damage bonus, armor, spell costs, or skill bases — all can be
  formulas.
- One mega-Actor table with vehicles and creatures crammed into the character shape.
- Deriving the fumble threshold from the raw skill without deciding whether difficulty
  affects it — pick a rule, make it configurable, document it.
- Treating a roll as a stateless function call; half of 7e's rules run *after* the dice
  land.
- Assuming importer input is well-typed, well-spelled, or consistently an array.

---

## 8. Backend implications

Entities this survey implies (names are suggestions, not a schema):

- **Actor** — discriminated by `kind ∈ {investigator, npc, creature, vehicle, container}`.
  Core: `characteristics{str,con,siz,dex,app,int,pow,edu}` each `{value, formula}`;
  `attribs{hp,mp,san,lck,mov,db,build,armor}` each `{value, max?, auto}` with `db`/`armor`
  as strings; `conditions{...}` including `tempoInsane{value, realTime, duration}`.
- **DerivedStatPolicy** — server-side recompute of Build/DB/HP/MP/MOV/SANmax honouring each
  `auto` flag and the pulp/age-penalty toggles; runs on every characteristic write and
  clamps `value ≤ max`.
- **Skill** — owned by an Actor; `{slug, name, skillName, specialization, base(formula),
  adjustments{...}, properties{...}}`; effective value = sum of adjustments (or an
  active-effect override).
- **Weapon / Armor / Item / Spell / Tome / Talent / StatusCondition** — separate typed rows;
  weapons carry per-range-band damage, `ammo` vs `bullets`, `malfunction`, and DB flags.
- **TomeStudyProgress** — actor-scoped `{tomeSlug, initialReading, progress, necessary,
  units, fullStudies, spellsLearned[]}`; spans sessions, triggers Mythos gain + SAN loss on
  completion.
- **Roll / Check** — a persisted aggregate: `{actorId, subject{type,name,id}, threshold,
  flatThresholdModifier, difficulty, bonusCount, penaltyCount, currentPoolModifier,
  flatDiceModifier, luckSpent, groups[] (initial + pushed), successLevel, isSuccess,
  isRolledSuccess, isForced, setSuccess, rollMode, allowPush, appliedDevelopment}` with an
  append-only mutation log.
- **DicePoolResult** (per group) — `{tensDice[], unitDie, candidateTotals[], activeIndex,
  total}`, so the UI can show which tens die won and audits can replay the min/max choice.
- **SanityEvent** — `{actorId, type/source, totalLoss, immunity}` so repeat exposure to the
  same horror can be suppressed; plus the running `dailyLoss` / `dailyLimit` counters and
  their reset boundary.
- **SessionState / Encounter** — combat round + turn order (DEX-based, firearms-first), and
  a separate **Chase** entity with its own participants, MOV/location tracking, and
  per-participant hazards; the existence of a standalone chase-tracker app is evidence this
  doesn't fold into combat.
- **ContentSlug registry** — `type.subtype.slug` → concrete row, resolved per era + locale
  with fallback; the join point for compendium/import content.
- **RuleFlags** — per campaign: pulp double HP, faster recovery, ignore age penalties,
  allow pushing fumbles, fumble-threshold interpretation, default check difficulty. All of
  these are *settings* in the Foundry system, which means real tables disagree and the
  backend must not hardcode.
- **Import adapters** — Dhole's House JSON at minimum, with the §6 normalisation rules;
  ideally a Foundry actor JSON adapter too, since that format is the richest and openly
  documented.

API shape notes: a roll endpoint should take `(actorId, subject, difficulty, diceModifier,
flags)` and return a roll resource carrying its band ranges and available follow-up actions
(`push`, `spendLuck:N`, `addBonusDie`) as server-computed affordances — mirroring
`availableButtons()`. Clients should never decide whether Luck may be spent; that logic has
at least six exclusions.

---

## 9. Open questions

1. **Fumble threshold and difficulty.** Foundry tests the *difficulty-adjusted* threshold
   against 50; the rulebook's plain reading concerns the skill value. Which does this
   project implement, and is it a toggle? (Affects every Hard+ check for skills 50–99.)
2. **Does Luck spending get a house-rule budget?** Foundry requires strictly-greater Luck
   remaining and forbids buying criticals; other tables let you buy a fumble away.
   Configurable?
3. **What resets `san.dailyLoss`?** "A day" is game-fiction time. Does the backend model an
   in-game calendar, or does the Keeper press a "new day" button? Same question for
   temporary-insanity duration in real minutes vs rounds.
4. **Interchange direction.** Import from Dhole's House is clearly worthwhile — but is
   *export* to Foundry/Roll20 a goal? Foundry actor JSON export is feasible; Roll20 has no
   import path.
5. **Adopt CoC IDs verbatim** (`i.skill.spot-hidden`) for compatibility with the Foundry
   compendium ecosystem, or mint our own namespace? Verbatim adoption eases content reuse
   but inherits their subtype taxonomy.
6. **Licensing of seed content.** Skill lists, weapon tables, spell lists: how much can
   ship? The BRP ORC Content Document is royalty-free reusable; CoC-specific content and
   Mythos material is not. Which corpus seeds the default compendium?
7. **Are archetypes / experience packages / pulp talents in scope?** Separate item types in
   Foundry, adding a whole char-gen dimension.
8. **Chase as a first-class entity?** The rules are heavy (location tracking, hazards,
   per-round MOV checks), and a standalone open-source tracker exists precisely because VTTs
   handle it poorly. In or out for v1?
9. **An active-effects equivalent.** Foundry lets effects override any derived value,
   individual skills included. Does this backend need a general modifier layer, or do ad-hoc
   bonuses suffice?
10. **Multi-Keeper / spectator permissions.** Blind rolls, Keeper-only description fields
    (`description.keeper` exists on nearly every entity), and hidden NPC stats all point to
    a field-level visibility model, not row-level.

---

## Sources

**Code and data models**
- Foundry VTT CoC7 system — https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT
  (`develop` branch, system v8.15). Files read:
  `coc7/models/actor/{global,character,npc,vehicle,creature}-system.js`,
  `coc7/models/item/{global,skill,weapon,spell,status,occupation,book}-system.js`,
  `coc7/models/actor/document-class.js`, `coc7/apps/dice-pool.js`, `coc7/apps/check.js`,
  `coc7/apps/link.js`, `coc7/apps/coc-id.js`, `coc7/apps/dholehouse-importer.js`,
  `coc7/apps/actor-importer-dialog.js`, `coc7/constants.js`, `static/system.json`,
  `.github/ABANDONED.md`
- Foundry package listing — https://foundryvtt.com/packages/CoC7
- Roll20 community sheet — https://github.com/Roll20/roll20-character-sheets/blob/master/Call_of_Cthulhu_7th_Ed/coc_7th_ed.html
- Roll20 official CoC7E sheet help (fetch blocked; unverified) — https://help.roll20.net/hc/en-us/articles/360052637253-Call-of-Cthulhu-7E-by-Roll20
- Chase Tracker — https://github.com/coulgreer/call-of-cthulhu_chase-tracker
- CoC 7e Character Creator — https://github.com/apoapostolov/Call-of-Cthulhu-Character-Creator
- `coc-dice-bot` — https://github.com/Asurazpr/coc-dice-bot ; `cthulhubot` — https://github.com/tsabelmann/cthulhubot
- `cochar` (Python char-gen) — https://pypi.org/project/cochar/

**Rules and official references**
- Call of Cthulhu 7th Edition Quick-Start Rules (free, Chaosium) — bonus/penalty dice,
  success levels, combat basics. Local extract used for verification.
- Chaosium Call of Cthulhu wiki, "The Game System" — https://cthulhuwiki.chaosium.com/rules/game-system.html
- CoC 7e Keeper Rulebook — ch. 3 (Characteristics & derived attributes), ch. 5 (Game System:
  success levels, bonus/penalty dice, pushing, opposed rolls), ch. 6 (Combat), ch. 8
  (Sanity). Page numbers not cited precisely here; see `10-…`–`19-…` in this folder for the
  rules-side references.
- Basic Roleplaying ORC Content Document (ORC-licensed d100 rules text) —
  https://www.chaosium.com/content/orclicense/BasicRoleplaying-ORC-Content-Document.pdf
- The Dhole's House — https://www.dholeshouse.org/ (site blocks automated fetch; the schema
  above is reconstructed from the Foundry importer that consumes it) and Chaosium's
  announcement —
  https://www.chaosium.com/blog/welcome-to-the-dholes-house-free-online-toolkit-for-call-of-cthulhu/
- Roll20 forum threads on CoC 7e bonus/penalty dice notation —
  https://app.roll20.net/forum/post/1362863/call-of-cthulhu-7e-bonus-and-penalty-dice ,
  https://app.roll20.net/forum/post/3725946/call-of-cthulhu-7e-bonus-and-penalty-dice-formula
