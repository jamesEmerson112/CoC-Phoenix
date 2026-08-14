# Player Signup Wizard — Design Spec (v1)

> Handoff document for the prototype design phase. Self-contained: everything a designer needs is
> in this file. Rules background lives in `docs/context/` (especially `10`, `12`, `18`, `21`) but
> nothing below requires reading it.
>
> **What we're building:** a character-first signup flow for a Call of Cthulhu 7th Edition
> tracker. A brand-new player creates a complete, rules-legal modern-day investigator in **under
> two minutes**, answering four questions and pressing one roll button, with zero rulebook
> knowledge.
> The character is account-scoped; joining a Keeper's campaign happens later by attaching it.

---

## 1. Product frame

- **Character-first.** No campaign, no Keeper, no invite required. A player lands on the app and
  makes an investigator. This is the front door of the whole product.
- **Free hosted web app.** We operate under Chaosium's Fan Material Policy, which permits free
  hosted tools (character generators are its canonical example). Consequences for design:
  - The required fan-policy notice must be displayed (footer / about screen). Verbatim text is in
    `docs/context/21-licensing-and-ip.md`.
  - No Chaosium artwork, logos, trade dress, or book text anywhere. All content in this spec is
    authored by us.
  - Product name is TBD and must not contain "Call of Cthulhu" (trademark). Working title: **CoC
    Phoenix** (internal only).
- **Era: modern day.** Deliberate choice, following the largest CoC community in the world — the
  Japanese scene defaults to modern-day settings, and 7e supports modern natively (`Computer Use`
  and `Electronics` are core skills; cash tables carry modern columns). Era is a content-pack tag:
  a classic-1920s pack is a future add-on, not a v1 dimension.
- **Tone.** Modern, playful-dark. The wizard should feel like a 3 a.m. group chat crossed with a
  public-records office: clean contemporary UI, warm copy, dread at the edges. It is inviting a
  player to create someone the universe is going to hurt.
- **Speed budget:** first-time completion in under 2 minutes on the happy path; a repeat player
  using skip modes in under 45 seconds.

---

## 2. The flow — six steps

```
1 Identity → 2 Age → 3 The Roll → 4 Career → 5 Hobbies → 6 The Sheet
```

The sheet is a **live projection** of the answers: while drafting, changing any earlier answer
re-renders everything downstream. Nothing is destroyed by going back. Finalizing freezes it
(§ 9).

### Step 1 — Identity

- **Name** (required, free text).
- **Pronouns / description** (optional, free text). Sex/gender has **zero mechanical effect** in
  CoC 7e — this field is pure flavor, so no dropdown, no validation, no requirement.
- Optional portrait placeholder (out of scope to build an uploader in v1; leave the slot in the
  layout).

### Step 2 — Age

One question, four bands. Age must be answered **before** the roll so the dice ceremony reveals
final, post-age numbers — never "here are your stats, now subtract."

| Band | Ages | Wisdom | Cost | Special |
|---|---|---|---|---|
| **Young** | 15–19 | 0 EDU checks; EDU −5 | −5 split between STR and SIZ | **Luck rolled twice, keep the better** — the universe protects the young |
| **Prime** *(default)* | 20–39 | 1 EDU check | none | — |
| **Seasoned** | 40–59 | 2 (40s) or 3 (50s) EDU checks | physicals −5 (40s) / −10 (50s), APP −5/−10, MOV −1/−2 | — |
| **Venerable** | 60+ | 4 EDU checks | physicals −20 (60s) / −40 (70s) / −80 (80s), APP −15/−20/−25, MOV −3/−4/−5 | — |

- Within a band the player may type an exact age (defaulting to a representative value: 17, 27,
  48, 65). Exact age selects the sub-column above.
- "Physicals" deductions are split across STR/CON/DEX. Normally player-allocated; **we
  auto-allocate deterministically** — deduct from whatever the eventually-chosen career needs
  least (the allocator runs after all answers are in). No follow-up question.
- **Design intent:** present the trade as flavor, not punishment. The EDU checks are *chances* at
  wisdom (+1D10 each, only if the check succeeds) while the physical cost is guaranteed and
  doubles per decade — that lopsidedness is the CoC fantasy (the frail professor who knows too
  much). Sample band copy: *"Venerable — four chances at wisdom; the body pays for all of them."*
- Age also nudges career fit (§ Step 4): older profiles float scholarly careers to the top, young
  ones float the physical careers. The age question quietly does characterization.

### Step 3 — The Roll

The centerpiece. Three ceremonies in sequence, each skippable via a persistent **"just show me
the results"** fast path.

**3a. Characteristics — the overture.** Eight stats, rolled with their real dice:

| Dice | Stats | Range (×5, after multiplying) |
|---|---|---|
| 3D6 × 5 | STR, CON, DEX, APP, POW | 15–90 |
| (2D6+6) × 5 | SIZ, INT, EDU | 40–90 |

Show actual dice tumbling per stat — the physicality is the fun. Age adjustments are already
baked into the displayed numbers. **Unlimited rerolls**: the reroll button is prominent and
smashing it is a feature, not an exploit.

**3b. EDU life-experience checks — the hidden gem.** For each check granted by the age band:
roll d100; if the roll is **above** current EDU, EDU rises by +1D10 — life taught you something.
Animate each check individually with a beat: *"Did the world educate you? … It did. +7 EDU."*
Older characters get a short sequence of these — the consolation mechanic for aging, on camera.

**3c. Luck — the finale.** Its own screen, its own moment. 3D6 × 5. This is not "how strong are
you" — it is *how the universe feels about you*, and in play Luck is the pool players spend to
un-fail rolls, so this is the number they'll have feelings about all campaign. Stage it with a
beat of drama. Young characters roll twice and keep the better — show both rolls.

**Manual entry mode.** A "I rolled at home" toggle swaps every ceremony for input fields with
light validation (multiples of 5; 15–90 for 3D6×5 stats, 40–90 for 2D6+6 stats; d100 + d10 pairs
for EDU checks; one or two 3D6×5 values for Luck). Physical-dice players are first-class
citizens. (Backend note: these persist as `diceSource: declared` vs `server`.)

### Step 4 — Career

Seventeen careers (roster in § 4), displayed as cards, **sorted by fit** to the rolled
characteristics — each career's points formula is evaluated against the player's stats and the
list orders best-payout-first, with a subtle "made for you" treatment on strong matches. With 17
options, sorting is the navigation, not a nicety.

Card contents: career name, one-line story hook (story-first copy — the Nail Artist card says
*"your clients tell you everything"*, not "grants Listen"), signature skill, and a small
"suits: EDU/APP/…" hint. Selecting shows the full skill list.

**Choice-slot follow-up.** Exactly two careers ask one extra question on selection:

- **Professor** — "What's your field?" → Science (Biology / Chemistry / Physics / Astronomy)
- **Influencer** — "What's your medium?" → Art/Craft (Video / Photography / Music / Writing)

One tap, options are enjoyable, and it only appears for those two.

### Step 5 — Hobbies

"What do you do when you're not working?" — pick **4 of 17** (roster in § 5). Each hobby card
shows its friendly name, a one-liner, and the two skills it trains (primary bold, secondary
small). **Pick order matters and the UI should say so**: *"your first pick is your passion"* —
allocation weights descend by pick order (§ 6).

Overlap with career skills is allowed and encouraged ("doubling down") — the stacking cap (§ 6)
handles balance silently.

### Step 6 — The Sheet

The payoff screen: the full investigator sheet, assembled. Contents:

- The eight characteristics with **half and fifth values** displayed (v // 2 and v // 5 — these
  are what get used constantly in play).
- Derived stats, computed live (formulas in § 7): Hit Points, Magic Points, Sanity, Luck, Damage
  Bonus, Build, Move.
- The skill list with final values, visually distinguishing base / career points / hobby points
  (e.g. stacked bar or annotation) — the player should *see* where their numbers came from.
- Credit Rating with its cash/spending implication in plain words ("comfortable", "struggling").
- **Backstory fate tables (optional):** *"Write your story later — or let fate deal it now."*
  Five d10 rolls, one per category: Ideology & Beliefs · Significant Person (who + why) ·
  Meaningful Location · Treasured Possession · Trait. All table entries are authored by us
  (samples in § 8). For players who freeze at blank text boxes, rolling their personality is the
  most fun path. Skippable, editable, rerollable per-category.
- **Finalize** button → the character freezes (§ 9). Copy should make it feel like signing a
  passport, with one line of dread.

---

## 3. UX principles (bind all steps)

1. **Never silently roll what a player would enjoy rolling.** Derived math (HP, Damage Bonus) is
   computed invisibly; the four dice moments (stats, EDU checks, Luck, fate tables) belong to the
   player.
2. **Every ceremony has a skip.** The second character someone makes should never be forced down
   the scenic route.
3. **Honest dice.** Server rolls first, the animation replays the true result — never theater
   that pretends the outcome is still open after it's known.
4. **Costs are flavor.** Aging penalties, low rolls, cheap careers — present them as
   characterization, never as failure states. There is no bad investigator in this game; there
   are only interesting ways to die.
5. **Deterministic everything.** Same answers → same sheet, always. All auto-allocation rules are
   fixed and documented (§ 6). No hidden randomness outside the dice ceremonies.
6. **Mobile-first sensible.** Card pickers and one-question-per-screen flow should work on a
   phone; the sheet may be richer on desktop.

---

## 4. Career roster (17)

All content authored by us: generic profession names, our own formulas, skill lists, and Credit
Rating ranges written in the rulebook's *style* but not copied from it.

| # | Career | Points formula | Signature skill | Supporting skills | CR range |
|---|---|---|---|---|---|
| 1 | **Doctor** | EDU×4 | Medicine | First Aid, Science (Biology), Science (Pharmacy), Psychology, Language (Latin) | 30–80 |
| 2 | **Professor** | EDU×4 | Library Use | History, Language (Other), Science *(choice)*, Persuade, Psychology | 20–70 |
| 3 | **Antiquarian** | EDU×4 | Appraise | History, Library Use, Language (Other), Spot Hidden, Navigate | 30–70 |
| 4 | **Journalist** | EDU×4 | Fast Talk | Library Use, Psychology, Computer Use, Art/Craft (Photography), Persuade | 9–30 |
| 5 | **Private Investigator** | EDU×2 + DEX×2 | Spot Hidden | Law, Library Use, Disguise, Psychology, Firearms (Handgun), Stealth | 9–30 |
| 6 | **Police Detective** | EDU×2 + INT×2 | Psychology | Firearms (Handgun), Law, Listen, Spot Hidden, Intimidate | 20–50 |
| 7 | **Soldier (Veteran)** | EDU×2 + STR×2 | Firearms (Rifle) | Fighting (Brawl), First Aid, Survival, Stealth, Climb | 9–30 |
| 8 | **Criminal** | EDU×2 + DEX×2 | Locksmith | Sleight of Hand, Stealth, Fighting (Brawl), Fast Talk, Spot Hidden | 5–65 |
| 9 | **Clergy** | EDU×4 | Persuade | Psychology, History, Language (Latin), First Aid, Listen | 9–60 |
| 10 | **Influencer** | APP×2 + EDU×2 | Charm | Art/Craft *(choice)*, Fast Talk, Psychology, Computer Use | 50–99 |
| 11 | **Jack of All Trades** | EDU×2 + best-other×2 | — special, see § 4.1 | Drive Auto, First Aid, Spot Hidden, Listen + hobby flow-in | 9–50 |
| 12 | **Grid Operations Engineer** | EDU×4 | Electrical Repair | Electronics, Computer Use, Mechanical Repair, Spot Hidden, Climb | 30–60 |
| 13 | **Interior Architect** | EDU×2 + APP×2 | Art/Craft (Interior Design) | Appraise, Charm, Spot Hidden, Psychology, Computer Use | 30–70 |
| 14 | **Nail Artist** | APP×2 + DEX×2 | Art/Craft (Cosmetology) | Charm, Listen, Psychology, Fast Talk, Sleight of Hand | 9–40 |
| 15 | **Séance Medium** | POW×2 + APP×2 | Occult | Psychology, Art/Craft (Acting), Charm, Listen, Sleight of Hand | 9–40 |
| 16 | **Funerary Director** | EDU×2 + APP×2 | Psychology | Science (Biology), Persuade, History, Occult, Drive Auto | 20–50 |
| 17 | **Professional MMA Fighter** | STR×2 + DEX×2 | Fighting (Brawl) | Dodge, Intimidate, Jump, Psychology | 9–40 |

**Mirror pairs** (flavor hooks worth using in card copy):

- *Grid Operations Engineer ↔ Séance Medium* — one keeps the city lit; the other speaks with what
  waits in the dark.
- *Interior Architect ↔ Funerary Director* — both arrange interiors; one arranges viewing
  parlors.
- *Nail Artist ↔ Professional MMA Fighter* — hands as art versus hands as weapons.

**Coverage note:** any four random picks yield a functional party — healing (Doctor, Soldier,
Clergy), research (Professor, Antiquarian, Journalist, Engineer), social (Clergy, Influencer,
Nail Artist, Medium), sneaky (Criminal, PI), combat (Soldier, Police, MMA Fighter, Criminal),
bankroll (Influencer). The formula variety (EDU×4, EDU+DEX, EDU+INT, EDU+STR, EDU+APP, APP+DEX,
POW+APP, STR+DEX) is deliberate: different rolled arrays surface different "made for you"
careers.

### 4.1 Jack of All Trades — special rules

The ordinary-person career, and the mechanically distinct one:

- **Formula:** EDU×2 plus ×2 of the player's *highest remaining characteristic* —
  auto-resolved by the allocator, no extra question. The generalist leans into whatever you
  rolled well.
- **Skills:** four fixed everyday skills — Drive Auto, First Aid, Spot Hidden, Listen (the
  "ordinary but observant" cluster) — and the rest of the career pool flows into the player's
  **own hobby picks**. *Your job is unremarkable; your interests are who you are.*
- **Perk: +10 starting Luck** (applied after the Luck roll, capped at 99). Thematically, the
  ordinary person survives the Mythos through sheer dumb luck; mechanically, Luck has no derived
  dependents, making it the cleanest possible bonus. **This perk is a house rule of ours, not
  RAW** — it is stored as an explicit occupation field and must remain toggleable.
- Card copy direction: *"You are nobody in particular. That may be why you're still alive."*

---

## 5. Hobby roster (17, pick 4)

Each hobby is a friendly-named, weighted pointer at real skills in the catalogue — a **primary**
(gets ~70% of the hobby's points) and a **secondary** (~30%). Specializations are pre-named so
no free-text is ever required.

| # | Hobby | Primary skill | Secondary skill |
|---|---|---|---|
| 1 | **Photography** | Art/Craft (Photography) | Spot Hidden |
| 2 | **Amateur boxing** | Fighting (Brawl) | Dodge |
| 3 | **Hunting trips** | Firearms (Rifle/Shotgun) | Track |
| 4 | **Bookworm** | Library Use | History |
| 5 | **Motoring enthusiast** | Drive Auto | Mechanical Repair |
| 6 | **Tinkering** | Mechanical Repair | Electronics |
| 7 | **Occult curiosity** | Occult | Library Use |
| 8 | **People-watching** | Spot Hidden | Psychology |
| 9 | **Social butterfly** | Charm | Listen |
| 10 | **Volunteer first-aider** | First Aid | Psychology |
| 11 | **Poker nights** | Psychology | Sleight of Hand |
| 12 | **Stage magic** | Sleight of Hand | Charm |
| 13 | **Weekend carpenter** | Art/Craft (Carpentry) | Mechanical Repair |
| 14 | **Amateur gastronome** | Art/Craft (Cooking) | Natural World |
| 15 | **Makeup artistry** | Disguise | Charm |
| 16 | **Online gaming** | Computer Use | Psychology |
| 17 | **Urban exploration** | Stealth | Climb |

Notes: Amateur boxing → Dodge is the quietly-best survival pick; Makeup artistry mapping to
**Disguise** makes it the most play-useful hobby on the list (and synergizes with the Nail
Artist); **Urban exploration** is the most on-genre modern hobby imaginable — abandoned
hospitals are where these scenarios happen; **Cthulhu Mythos is never offerable** at creation —
Occult is the only on-theme skill, on purpose. Overlaps between hobbies (two touch Mechanical
Repair, several touch Psychology/Charm) are intended; picking overlapping hobbies means doubling
down.

---

## 6. Allocation rules (deterministic, all ours)

The allocator runs over the complete answer set and always yields the same sheet for the same
answers. These weights are **our design choices**, not RAW; RAW only fixes the pool sizes.

**Career pool** = the career's formula evaluated on final (post-age) characteristics.

1. Set **Credit Rating** to the career's range minimum, paid out of the career pool (RAW: CR
   points come from occupation points).
2. Ladder the remainder over signature + supporting skills, descending:
   - 5 slots (1 signature + 4): **34 / 20 / 18 / 15 / 13 %**
   - 6 slots (1 signature + 5): **30 / 18 / 16 / 14 / 12 / 10 %**
   - 7 slots (1 signature + 6): **28 / 16 / 14 / 12 / 11 / 10 / 9 %**
3. Round down per skill; all rounding remainder goes to the **signature skill**.

**Jack of All Trades pool:** after CR minimum — fixed skills take 40% (Drive Auto 12, First Aid
10, Spot Hidden 10, Listen 8), hobby **primaries** take 60% split by pick order (24 / 16 / 12 /
8 %). Remainder to the first-pick primary.

**Personal pool** = INT × 2.

1. Split across the four hobby picks by pick order: **40 / 30 / 20 / 10 %**.
2. Within each hobby: **70% primary / 30% secondary**, rounded down, remainder to the primary.
3. Global rounding remainder → first pick's primary.

**Stacking & cap:** hobby points add on top of career points for the same skill. Final creation
value is **capped at 75** (⚠ exact cap pending rulebook verification — treat 75 as the working
number). Points lost to the cap are *not* redistributed (simplicity beats optimality; the player
chose to double down).

**Age deductions** (Seasoned/Venerable): subtract from the characteristic among STR/CON/DEX that
contributes least to the chosen career's formula and skill list; ties broken in fixed order
CON → STR → DEX. Documented, deterministic, recomputed if the career changes while drafting.

---

## 7. Derived values the sheet computes

`//` = integer division (floor). All recompute live while drafting.

| Value | Formula |
|---|---|
| Hit Points | `(CON + SIZ) // 10` |
| Magic Points | `POW // 5` |
| Sanity (starting) | `= POW` (max 99) |
| Luck | rolled 3D6×5 (Young: best of two; Jack of All Trades: +10, cap 99) |
| Damage Bonus / Build | from `STR + SIZ`: <65 → −2/−2 · 65–84 → −1/−1 · 85–124 → 0/0 · 125–164 → +1D4/+1 · 165–204 → +1D6/+2 |
| Move (MOV) | 9 if DEX > SIZ **and** STR > SIZ · 8 if DEX ≥ SIZ **or** STR ≥ SIZ · else 7 · minus age penalty (§ 2) |
| Dodge (base) | `DEX // 2` |
| Language (Own) (base) | `= EDU` |
| Half / Fifth values | `v // 2`, `v // 5` for every characteristic and skill |

---

## 8. Backstory fate tables — sample entries

Five categories, one d10 table each, all entries authored by us. Three samples per category to
set the voice (full tables of 10 to be authored during content pass):

- **Ideology & Beliefs:** "Science explains everything — it must." · "A devout spiritualist; the
  dead are merely elsewhere." · "Money is the only honest thing in this world."
- **Significant Person:** "Your sister, whose last message you've left on read for a year." ·
  "The professor who ruined your career and never learned your name." · "A patient/client/
  stranger you failed once, badly."
- **Meaningful Location:** "The lake house, before the accident." · "A reading room where you are
  known by name." · "The alley you don't walk past anymore."
- **Treasured Possession:** "A broken pocket-watch that keeps perfect time twice a day." · "A
  photograph with one face scratched out — you did the scratching." · "An old phone that holds
  one voicemail you never play."
- **Trait:** "You count things when nervous." · "You would die before breaking a promise." ·
  "You laugh at the wrong moments, always."

Tone target: each entry is a story hook with one dark edge. The example composite the flow should
produce: *"a devout spiritualist who treasures a broken pocket-watch and would die for their
sister."*

---

## 9. Draft lifecycle & data shape

**Lifecycle:** `draft → finalized`.

- **Draft:** the stored record is the *answers*, and the sheet is a pure projection over them.
  Change the career → everything downstream re-renders. Nothing is lost by going back.
- **Finalize:** the seed values freeze — career points, personal points, starting Sanity, Luck,
  Dodge and Language (Own) bases. A later edit to a career template does **not** rewrite existing
  finalized characters. Post-finalize editing goes through the (future) advanced editor as
  explicit adjustments, never by re-running the wizard.

**Answers-as-inputs record** (the backend contract the prototype should assume):

```jsonc
{
  "name": "Ada Kowalski",
  "pronouns": "she/her",            // free text, optional
  "ageBand": "seasoned", "age": 48,
  "dice": {
    "source": "server",              // or "declared" (manual entry)
    "characteristics": { "str": 50, "con": 65, "...": 0 },  // final post-age values + raw rolls kept
    "eduChecks": [ { "d100": 71, "gain": 7 } ],
    "luck": { "rolls": [55], "kept": 55 }
  },
  "careerId": "funerary-director",
  "careerChoices": {},               // e.g. { "science": "chemistry" } for Professor
  "hobbyIds": ["occult-curiosity", "people-watching", "poker-nights", "bookworm"],  // order = priority
  "backstory": { "mode": "fate", "rolls": {"ideology": 4, "...": 0}, "overrides": {} },
  "status": "draft"                  // → "finalized"
}
```

Everything else — skill values, derived stats, CR, cash — is computed by the deterministic
allocator (§ 6) + derivations (§ 7). This record is small, replayable, and is the undo story.

**Content structure:** careers and hobbies are rows in a bundled starter **ContentPack**
(license: fan-policy). Careers carry `{formulaTerms, signatureSkill, skills[], choiceSlots[],
creditRating{min,max}, bonuses{luck?}, slotsFromHobbies?}`; hobbies carry `{name, blurb,
skills: [{skillRef, weight}]}`. Skills reference the skill catalogue; specializations are
pre-named catalogue rows.

---

## 10. Out of scope for v1 signup (explicitly)

- Campaigns, Keepers, invites, approval gates (character-first; attach-to-campaign comes later)
- Equipment and weapons beyond what Credit Rating implies in words
- Free-text backstory editing beyond simple overrides of fate results
- Pulp Cthulhu, alternate eras (classic 1920s, Gaslight — future content packs), alternate
  generation methods (point-buy, fixed array)
- The advanced editor (exists as a future door into the same allocation ledger)
- Portrait upload
- Ages outside 15–89 (fall back to Keeper/manual handling later)

---

## 11. Pending verification (do not block the prototype)

- Exact creation-time skill cap (working number: **75**)
- Official quick-fire allocation ladder values (ours in § 6 are self-authored and stay regardless;
  worth comparing once verified)
- "Improvement check" threshold convention (roll above value vs ≥ 96 edge) for EDU checks
- Ruleset text references must eventually cite a specific printing (2014 vs 2023 repagination)

---

## 12. What comes after this doc

1. **Design prototype** of the six-step flow from this spec (this document is the brief).
2. **Backend design** for the signup slice: the answers record, the deterministic allocator, the
   dice service (`server`/`declared`), the starter ContentPack schema, and the draft→finalize
   transition — all shaped so the full tracker (campaigns, rolls, combat, sanity) extends it
   rather than replaces it. Reference reading: `docs/context/90-backend-implications.md`.
