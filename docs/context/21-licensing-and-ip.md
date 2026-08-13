# Licensing & IP Constraints for a CoC 7e Tracker Backend

> **Not legal advice.** Engineering-facing research so the data model gets designed around the constraints instead of retrofitted to them. Anything with money involved should be confirmed with Chaosium (`licensing@chaosium.com`) or a lawyer.

## Overview

Call of Cthulhu 7th Edition is **closed IP**. No CoC SRD exists, no OGL/ORC release of the Keeper Rulebook, no free-content grant over Mythos monsters, spells, tomes, occupations, or the rulebook's tables. What *is* open is the neighbouring engine: Chaosium released **Basic Roleplaying: Universal Game Engine (BRP-UGE)** under the **ORC License**, and that document happens to contain a d100 core, a full skill list, combat, and an optional **Sanity system** mechanically close to (though not identical to) CoC's. Everything else a CoC tracker touches sits under Chaosium's **Fan Material Policy**: non-commercial, attribution-notice required, no app-store/retail distribution — and, crucially for this project, it **explicitly excludes "software, apps, and VTTs… any item a user would download, install, and/or run"** while explicitly permitting **web-based character sheets that autofill or calculate**. The architectural consequence: the backend ships *structure and math*, never *rulebook text*; ORC-derived content can seed real default data (with an ORC notice); CoC-specific content must remain an empty, user-populated content layer. This doc maps which mechanics fall on which side of the line and what that forces into the schema.

---

## 1. The four legal tracks

| Track | Grants | Can you charge? | Can you ship installable software? | Fit for this project |
|---|---|---|---|---|
| **Fan Material Policy** | Use of Chaosium trademarks/copyright for free fan content, incl. web character sheets & calculators | **No** — not for access, not via retail/app stores | **No** (web-hosted only) | Primary track for a free hosted web tool |
| **Community Content (Miskatonic Repository)** | Publish CoC 7e scenarios/settings/spells on DriveThruRPG, may sell | Yes (rev-share) | **No** — code/apps not accepted; PDFs and assets only | Irrelevant to a backend; relevant only if scenario data ever ships |
| **BRP-UGE under ORC License** | Full BRP:UGE text for personal *and commercial* use, royalty-free | **Yes** | **Yes** (it licenses content, not a distribution channel) | The only track permitting real shippable rules data |
| **Commercial / Small Publisher Limited License** | Negotiated; SPLL is a flat annual fee capped at ~US$2,000/yr gross | Yes | Case-by-case, application required | Needed if the tracker is ever monetized or shipped as installable software |

Notable: Chaosium states it holds **exclusive agreements in some categories** (computer games and certain VTT platforms among them), so a commercial license for a CoC digital tool may simply be unavailable, however good the proposal. Roll20, Foundry, and Alchemy carry *officially licensed* CoC content today — that territory is claimed.

---

## 2. Fan Material Policy — the operative rules

Source: <https://www.chaosium.com/fan-material-policy/> and the Q&A at <https://www.chaosium.com/fan-use-and-licensing-q-a/>.

**Permitted**
- Your own character sheets, including **web-based sheets that autofill or calculate numeric values** — the explicit blessing a hosted investigator tracker leans on.
- Referencing names of entities, places, deities, concepts, plots, and events, with attribution.
- Quoting *small* extracts (a sentence or two) with a source credit.
- Custom props/handouts, fan fiction, original fan art.

**Prohibited (hard blockers for a backend)**
- **Charging for access.** No subscriptions, no "pro tiers," no paid hosting of the tool itself.
- **Retail/app-store distribution**, *even free* — DriveThruRPG, Amazon, Google Play, iTunes, and the App Store are named explicitly.
- **Software / apps / VTTs "a user would download, install, and/or run."** A self-hosted Docker image or desktop client sits on the wrong side of that sentence; a hosted web service sits on the right side.
- Chaosium **logos**; imitating Chaosium **trade dress** (page templates, art style, fonts).
- Modifying Chaosium artwork or maps (resizing only; maps may also be cropped), with a legible `© <year> Chaosium Inc.` beside the image.
- Implying endorsement or official status.
- Crowdfunding.

**Allowed indirect monetization**: banner ads on the site, a tip jar (Ko-fi etc.), Patreon for *work-in-progress extracts* only — never gating the tool.

**Required notice** — ship verbatim in an `/about`, at the API root, and in the repo README:

```
This <site/tool> uses trademarks and/or copyrights owned by Chaosium Inc/Moon Design
Publications LLC, which are used under Chaosium Inc's Fan Material Policy. We are
expressly prohibited from charging you to use or access this content. This <site/tool>
is not published, endorsed, or specifically approved by Chaosium Inc. For more
information about Chaosium Inc's products, please visit www.chaosium.com.
```

**Engineering read:** the fan policy constrains the *deployment model*, not just the content. Free hosted SaaS is inside it. "Download and run your own instance" is outside it as written — even though the Foundry CoC7 system does exactly that and is tolerated (§7). Treat self-hosting as a deliberate risk decision, never an accident of packaging.

---

## 3. BRP-UGE under ORC — what you actually get

The ORC Content Document is essentially the full BRP:UGE text, usable personally and commercially, royalty-free. Chapters: Introduction, Characters, Skills, Powers, System, Combat, Spot Rules, Equipment, Gamemastering, Settings, Creatures.

**Reserved (NOT licensed) even inside BRP-UGE:** the trademarked names — *Call of Cthulhu*, *Chaosium*, *RuneQuest*, *Pendragon*, *Superworld*, *Fantasy World*, *Futureworld* — plus all artwork, illustrations, runes, and non-alphanumeric glyphs.

**ORC obligations that touch the codebase**
- Include an **ORC Notice** identifying the licensed material and crediting Chaosium Inc., Jason Durall and Steve Perrin; ship or link the ORC License text (registered at the Library of Congress, TX 9-307-067).
- ORC is **perpetual and irrevocable** for released material — no OGL-style rug-pull exposure.
- ORC is **share-alike for game mechanics**: mechanics published in an ORC-derived work become ORC-licensed themselves. Your *source code* is not "game mechanics text," but new rules content authored and shipped as data plausibly is. Keep first-party rules text in a clearly delimited pack so the ORC boundary stays auditable.
- The "Powered by BRP" logo is recommended, not required.

### 3.1 ORC-covered mechanics (safe to ship as real seed data)

These are the BRP-UGE formulas, not CoC 7e's. They differ, and silently mixing them is the number-one correctness trap.

- **Success ladder (5 tiers):** fumble / failure / success / **special ≤ ⌊skill/5⌉** / **critical ≤ ⌊skill/20⌉**, rounding at .5. The fumble range = 5% of the failure chance, so it slides with skill (roughly 96–00 at low skill, tightening to 00 alone at very high skill). `00` always fumbles. Any skill with base ≥5% always succeeds on 01–05.
- **Difficulty grades:** Automatic (no roll, no experience check) / Easy (**×2**, no experience check) / Average (×1) / Difficult (**÷2, round up**) / Impossible (auto-fail; the GM may allow a flat 01%).
- **Modifier ordering:** difficulty multiplier first, then situational flat modifiers; "permanent" skill bonuses fold in *before* the multiplier.
- **Characteristic rolls:** default ×5 (Idea = INT×5, Know = EDU×5, Luck = POW×5); Easy variants at ×10.
- **Resistance table** (active vs passive characteristic), opposed rolls, cooperative rolls, combined rolls.
- **Derived stats:** hit points, power points, damage modifier from STR+SIZ, experience bonus, optional fatigue points (STR+CON).
- **Sanity (optional system)** — §3.2.
- **A generic creature roster** (centaur, ghoul, zombie, vampire, grey alien, robot…). **No Mythos entities.** Nothing ORC-licensed can ship as a Deep One or a shoggoth.
- **A skill list** under BRP names: `Spot`, `Research`, `Status`, `Insight`, `Psychotherapy`, `Knowledge (…)`, `Fast Talk`, `Listen`, `Stealth`, `Track`. These are *not* the CoC names (`Spot Hidden`, `Library Use`, `Credit Rating`, `Psychoanalysis`).

### 3.2 The ORC Sanity system (usable) vs CoC 7e Sanity (not)

The single most valuable finding for a tracker, since Sanity is the state machine you most want default data for.

**ORC / BRP-UGE version — shippable:**
- `SAN_start = POW × 5`; SAN does not follow later POW changes. Cap 99.
- `TIS` (Temporary Insanity Score) = `⌊SAN_start / 2⌋`, **fixed for life** (GM option: `TIS = POW` for a far twitchier game).
- SAN roll: d100 ≤ current SAN. Failure → roll the loss; some sources charge SAN even on success (the `x/y` notation, e.g. `1/1D6`).
- **Temporary insanity trigger:** cumulative SAN loss ≥ TIS within a **5-minute turn**. Then 1D6 on a 6-entry effect table and 1D8 on a duration table (5 minutes … 2 weeks).
- **Indefinite insanity threshold:** losing ≥ 1/5 of *current* SAN within one **game hour**.
- `SAN = 0` → permanent insanity; the character retires / becomes GM-controlled.
- **Max SAN erosion:** `SAN_max = 100 − Knowledge(Blasphemous Lore)`, and it can fall below the character's starting SAN.
- Recovery: half the threatened loss for defeating the source; Psychotherapy success = `1D3−1`; one-time `2D6` on first reaching 91% in any skill; drugs/powers at GM discretion. Never above `SAN_max`.
- SAN-loss table shape (reworded, illustrative — this is ORC content and needs the ORC notice if shipped):

  | Source | Success / Failure |
  |---|---|
  | Sudden shock, something grotesque | 0 / 1 |
  | Finding a corpse | 0 / 1D3 |
  | Witnessing a gruesome death | 1 / 1D6 |
  | Undergoing a horrible transformation | 1D6 / 2D10 |
  | An alien god | 1D10 / 1D100 |

**CoC 7e version — must NOT ship as data:**
- Starting SAN equals POW (not POW×5); max SAN is `99 − Cthulhu Mythos%`.
- Temporary insanity triggers on losing **5+ SAN from a single roll** — not per-turn accumulation against a fixed score.
- The bout of madness has *two* resolutions — real-time (rounds) and summary (hours) — each with its own d10 table, plus underlying-insanity and phobia/mania d100 tables.
- Indefinite insanity is 1/5 of current SAN lost within a **game day**, with recovery framed in months and Psychoanalysis.
- The CoC numbers, the tables, and the table *entries* are all Chaosium's expression. The **thresholds themselves** (5, one-fifth, 99 − Mythos) are unprotectable facts your engine may compute; the **prose and complete table contents** are not yours to ship.

### 3.3 Mechanics that are CoC 7e-only (no open-license cover at all)

Implementable as code, but with zero shippable content — and never presented as Chaosium-sanctioned:

- **Success tiers:** Regular (≤ skill), **Hard (≤ ⌊skill/2⌋)**, **Extreme (≤ ⌊skill/5⌋)**, Critical (01), Fumble (96–00 at skill < 50, else 100). A *different ladder* from BRP's special/critical.
- **Bonus / penalty dice** — extra tens dice with best/worst kept, shared ones die. No BRP analogue.
- **Pushed rolls** — one re-attempt on narrative justification with escalated failure consequences.
- **Luck as a spendable pool** (buying down the roll tier), Luck recovery, Pulp Cthulhu variants.
- **Build & Damage Bonus** ladder from STR+SIZ (CoC's specific banding), **MOV** derived from STR/DEX vs SIZ with age penalties.
- **Combat:** fighting manoeuvres, outnumbering bonus dice, firearm malfunction values, major wound = damage ≥ ⌊max HP / 2⌋, the dying/unconscious/first-aid state machine.
- **Chases** (Ch. 7) — the whole MOV/location/hazard subsystem is CoC 7e-specific.
- **Investigator creation:** the occupation list, occupation skill-point formulas, Credit Rating bands, backstory/Key Connection.
- **Skill names & base chances** as a set: `Spot Hidden 25`, `Library Use 20`, `Credit Rating 0`, `Cthulhu Mythos 0`, `Psychoanalysis 1`, `Dodge = DEX/2`, `Language (Own) = EDU`.

---

## 4. The idea/expression line, in practice

- **Game mechanics, systems, and methods of operation are not copyrightable** (US: 17 U.S.C. §102(b), the *Baker v. Selden* lineage). A rules engine computing a Hard success as `roll ≤ floor(skill/2)` implements an uncopyrightable procedure.
- **The rulebook's text is copyrighted.** Search-and-replace paraphrase remains infringement when the structure and phrasing survive.
- **Tables are the danger zone.** Individual numbers are facts; a curated d100 phobia table is a *compilation* with selection and arrangement, its entries short expressive phrases. Reproducing whole tables is the classic way fan tools receive letters.
- **Names are trademarks, not copyright.** "Call of Cthulhu," "Chaosium," "Miskatonic Repository" are marks. Never name the product, package, npm scope, domain, or API namespace after them. `CoC Phoenix` as a project codename is fine; `Call of Cthulhu Tracker` on a public landing page is not. Nominative fair use covers "compatible with Call of Cthulhu 7th Edition" *when* paired with the fan-policy disclaimer and no logo use.
- **Lovecraft is mostly public domain** — he died in 1937, so his own writings are broadly PD. But Chaosium's position is that storylines, names, creatures, characters, descriptions, and depictions *it* created are its IP, and other Mythos contributors (Bloch d. 1994, Lumley, Campbell, Petersen) still hold rights. Chaosium explicitly refuses to enumerate what is PD. **Never build a "PD Mythos bestiary" feature on the assumption the names are free** — the *names* may be; the *CoC stat blocks and SAN-loss values* are not.

---

## 5. Content classification for the schema

| Class | Examples | Ships in repo/DB image? |
|---|---|---|
| 🟢 **Green — engine** | Dice notation parser, tier calculators, initiative/DEX ordering, SAN state machine, wound state machine, chase resolution logic | Yes. Code only. |
| 🟢 **Green — ORC data** | BRP skill list + base %, BRP SAN loss/insanity tables, BRP generic creatures, difficulty multipliers | Yes, in a pack tagged `license: ORC`, ORC notice attached |
| 🟡 **Amber — bare identifiers** | CoC skill *names* with base % and no description; phobia/mania *names* with a one-word gloss; occupation names | Defensible (short, factual, functional), but keep it to names+numbers, never descriptions; consider an optional user-installed pack |
| 🔴 **Red — never** | Rulebook prose in any `description` field; complete d100 phobia/mania/madness tables; Mythos monster stat blocks; spell descriptions and costs; tome entries; occupation write-ups; scenario text; any Chaosium artwork, map, icon, logo, font, or sheet PDF | No. Not in seeds, fixtures, test data, `.sql` dumps, or the git history |

**Test data is a real leak vector.** A fixture containing a genuine Chaosium monster stat block, or a snapshot test holding rulebook flavour text, is distributed content. Use obviously-invented entities (`Test Horror`, `Dr. Placeholder`) in every fixture.

---

## 6. Empty-data-model architecture (the compliance pattern)

The proven approach, used by every surviving CoC fan tool:

1. **Content and engine are separate layers.** The engine knows *how* a SAN check resolves; it does not know that a shoggoth costs `1D6/1D20`.
2. **Ship the schema, not the content.** Monster, spell, tome, weapon, and occupation tables exist and are **empty on first boot**.
3. **User-supplied content, per-tenant.** Keepers type in (or import) values from books they own. Rows belong to a tenant/user — never global.
4. **A provenance flag on every content row** — `source: 'orc' | 'user' | 'community'` — so the app can (a) attach the right notice, (b) refuse to publish `user` rows, (c) purge a class of content wholesale if Chaosium ever objects.
5. **Sharing is opt-in and scoped.** A "share my bestiary publicly" feature turns a private transcription into public distribution of Chaosium's expression. Default sharing to *within-campaign only*.
6. **Import, don't bundle.** Accept a JSON/YAML content-pack upload. Never host a "starter pack" of rulebook content — and never link one.
7. **Descriptions point at the book.** The Foundry convention — a description field reading, in full, "See the Call of Cthulhu 7th Ed Core Rulebook" — is a good default value and a good nudge toward buying the book.
8. **Samples, not sets.** Where a table is needed for the UI to make sense, ship a handful of illustrative rows labelled "example" — never the full published table.

---

## 7. How the existing tools stay compliant

**Foundry VTT CoC7 system** (Miskatonic Investigative Society) — the reference implementation.
- Code under **GPLv3**; Foundry integration under Foundry's Limited License Agreement for Module Development; Chaosium IP under the **Fan Material Policy**.
- The README states no commercial-use rights are granted and no part may be reproduced or distributed for use outside the Foundry platform.
- **"Does not include any materials from the official books."** Users enter weapons, skills, and items themselves.
- What it *does* ship (`compendiums/*.yaml`): skill entries as `name + base% + flags + era tags` with `description: "See the Call of Cthulhu - 7th Ed Core Rulebook"`; phobia/mania entries as a name plus a two-word gloss; a **sample** phobia roll table of ~8 entries pointing at Keeper Rulebook p. 160 instead of the full d100 list; example weapons and NPCs.
- A tension worth naming: this *is* downloadable, installable software — exactly what the fan policy text excludes. It survives on tolerance plus the Foundry platform agreement, not on a clean reading of the policy.

**The Dhole's House** — a free web toolkit and character generator Chaosium publicly promotes as the *official* CoC character generator. Web-only, free, Chaosium-endorsed. This is the shape the fan policy was written to permit, and the closest precedent for a hosted tracker.

**Roll20 / Foundry / Alchemy official content** — genuine commercial licenses: converted sourcebooks, tokens, drag-and-drop monsters. The only lawful way to ship actual CoC content, and it takes a negotiated agreement.

**Miskatonic Repository** — DriveThruRPG's community programme for CoC 7e. Creators may use rules and setting material from **7th Edition** Chaosium books (not 1st–6th), must use programme art or their own with attribution, **no AI-generated art** (since 2023-03-31), and follow DriveThru's AI-writing rules. **It does not accept software, code, or VTT plugins** — graphical assets only. Not a distribution route for this project, though relevant if a companion scenario ever ships.

---

## 8. Backend implications

**Deployment / product shape**
- Target a **free, hosted, web-accessed** service. Publish no installable artifacts (desktop app, mobile app, store listing) and charge nothing for access absent a commercial or Small Publisher licence.
- If self-hosting is a requirement, split the repo: an **engine + schema** package (Apache/MIT/GPL, zero Chaosium expression, arguably fine to distribute) and a **deployment** that stays hosted-only. Document that no CoC content is bundled.
- Reserve a legal-notice surface: `GET /api/meta/legal` returning the fan-policy notice, the ORC notice, and the current content-pack manifest.

**Entities the licence position forces into existence**
- `ContentPack { id, name, license: 'ORC'|'FAN'|'USER'|'THIRD_PARTY', notice_text, source_url, version, is_bundled }` — every content row FKs to a pack. This is the audit boundary and the kill switch.
- `Ruleset { id, family: 'COC7'|'BRP_UGE'|'PULP', version }` — because ORC and CoC use **different success ladders and different SAN math**, ruleset is a first-class discriminator on campaigns, never a global config flag.
- `SkillDefinition { key, display_name, base_percent, is_pushable, specialization_of, pack_id }` — **no** `description` beyond a book pointer; `base_percent` nullable so a user-created skill can exist without one.
- `Creature` / `Spell` / `Tome` / `Weapon` / `Occupation` — full schemas, **zero bundled rows**, non-null `owner_id`, `visibility ∈ {private, campaign, public}` with `public` blocked whenever `pack.license = 'USER'`.
- `SanityState { current, start, max_cap, cap_source, temp_insanity_score, indefinite_until, bout_state }` — model both threshold styles: an accumulation window (ORC: ≥TIS per 5-minute turn; CoC: ≥1/5 current SAN per game day) *and* a single-event trigger (CoC: ≥5 SAN from one roll). Which means SAN loss must be an **event log**, not merely a counter.
- `SanLossEvent { investigator_id, amount, source_ref, occurred_at_game_time, roll_result, window_id }` — windowed aggregation is how both rule families are implemented; store game-time, never wall-clock.
- `RollResult { ruleset, skill_value, effective_value, difficulty_grade, bonus_dice, penalty_dice, tens_dice[], ones_die, tier, was_pushed }` — the tier enum differs per ruleset (`REGULAR/HARD/EXTREME/CRITICAL/FUMBLE` vs `SUCCESS/SPECIAL/CRITICAL/FUMBLE`); keep it ruleset-scoped, never shared.
- `ImportJob { user_id, pack_id, format, checksum, accepted_rows, rejected_rows }` — user content arrives by upload, so validation, size limits, and rejection reporting are product surface, not an afterthought.

**Behavioural constraints**
- **Rounding is per-ruleset.** BRP rounds special/critical at .5 (round-half-up); CoC floors its Hard/Extreme thresholds. Encode as a ruleset strategy — never one global rounding helper.
- **No content in migrations.** Seed migrations create tables and ORC packs only. Any migration inserting CoC content is a licence bug — add a CI check that greps seed/fixture files against a denylist of Chaosium-specific terms.
- **Search/index must not become a distribution channel.** Full-text indexing user-entered monster text across tenants builds a public reproduction of the bestiary. Scope indexes per tenant.
- **A takedown affordance.** `DELETE /packs/{id}` cascading to every row, plus per-user export, so a compliance request resolves in one operation without destroying users' own work.
- **Naming discipline in code and artefacts.** Package names, Docker image names, OpenAPI `info.title`, and the public domain name should avoid "Call of Cthulhu" as a product identifier; use it only inside a compatibility statement next to the disclaimer.

---

## 9. Open questions

1. **Deployment model.** A hosted free service (clearly inside the fan policy) or a self-hostable server (outside the policy's literal wording, tolerated in practice for Foundry)? This changes packaging, docs, and risk posture, and belongs before the first release — not after.
2. **Does Chaosium consider a Keeper-side *session tracker* a "character sheet"?** The policy blesses web character sheets and calculators; a combat/initiative/NPC tracker is adjacent but unnamed. Worth a direct email to `licensing@chaosium.com` — Dhole's House earned its endorsement by asking.
3. **Is the Small Publisher Limited License usable for software?** It is written around zines, con scenarios, and props, mentioning app-store channels only in passing. Ambiguous for a digital tool; confirm before any revenue intent.
4. **ORC share-alike scope.** If the backend ships ORC-derived seed data, does first-party rules content we author become ORC-licensed mechanics? Where exactly does "code" end and "game mechanics in a published work" begin?
5. **Dual-ruleset ambition.** Supporting BRP-UGE as a fully populated first-class ruleset (legal, shippable content) beside CoC 7e as an empty-shell ruleset is attractive but roughly doubles the rules-engine surface. In or out for v1?
6. **The amber-class data call.** Bundle CoC skill names + base percentages (the Foundry precedent), or make users import even those? The former is far better UX; the latter is unambiguously safe.
7. **User-generated content moderation.** If Keepers can share bestiaries campaign-wide or publicly, what stops someone uploading the entire Malleus Monstrorum? Upload-volume heuristics, or a review gate on public sharing?
8. **A trademark-safe public name** for the project — and whether the compatibility statement ("for use with Call of Cthulhu 7th Edition") is worth the trademark exposure versus a vaguer "1920s investigative horror d100."
9. **Which CoC printing to target** — the 2014 hardcover and the 2023 revised 7e differ in pagination and some errata; book-pointer strings and any UI page references need a printing field.

---

## Sources

**Chaosium policy**
- Fan Material Policy — <https://www.chaosium.com/fan-material-policy/>
- Fan-Use and Licensing Q&A — <https://www.chaosium.com/fan-use-and-licensing-q-a/>
- Fan-Use and Licensing (overview of the four tracks) — <https://www.chaosium.com/fan-use-and-licensing/>
- Commercial Licensing — <https://www.chaosium.com/commercial-licensing/>
- Small Publisher Limited License — <https://www.chaosium.com/small-publisher-limited-license/>
- Miskatonic Repository — <https://www.chaosium.com/miskatonic-repository/>
- "Isn't the Cthulhu Mythos in the public domain?" — <https://www.chaosium.com/blogfrom-the-qa-isnt-the-cthulhu-mythos-in-the-public-domain/>
- AI art policy for community content programmes — <https://www.chaosium.com/blogai-art-and-chaosium-community-content-programs-miskatonic-repositoryjonstown-compendiumexplorers-society/>
- Conditions of Use — <https://www.chaosium.com/conditions-of-use/>

**ORC / BRP**
- Chaosium ORC License page (reserved terms, attribution) — <https://www.chaosium.com/orc-license/>
- BRP System Reference Document — <https://www.chaosium.com/brp-system-reference-document/>
- BRP-UGE SRD (HTML) — <https://brpugesrd.xyz/> ; source repo — <https://github.com/raleel/brpugesrd>
  - `rules/en/0000_Front_Matter.md` (ORC notice, credits)
  - `rules/en/0500_System.md` (success tiers, difficulty multipliers, resistance table)
  - `rules/en/0200_Characters.md` §"Sanity Points (Max = POW×5)"
  - `rules/en/1000_Settings.md` §"Sanity (Option)" (TIS, temporary insanity tables, max SAN, restoration, SAN-loss table)
  - `rules/en/1100_Creatures.md` (generic creature roster — no Mythos entities)
- ORC License text and AxE (Paizo) — <https://paizo.com/orclicense>, <https://downloads.paizo.com/ORC_AxE_FINAL.pdf>

**Existing tools**
- Foundry VTT CoC7 system — <https://github.com/Miskatonic-Investigative-Society/CoC7-FoundryVTT> (README licence section; `compendiums/en-skills.yaml`, `en-wiki-phobias-and-manias.yaml`, `en-sanity-tables-examples.yaml`)
- The Dhole's House — <https://www.chaosium.com/blog/welcome-to-the-dholes-house-free-online-toolkit-for-call-of-cthulhu/>
- Roll20 CoC hub / Miskatonic Repository on Roll20 — <https://pages.roll20.net/call-of-cthulhu>

**Rulebook references** (citation only — no text reproduced)
- *Call of Cthulhu 7th Edition Keeper Rulebook*, Chaosium (CHA23135). Ch. "Game System" (core d100 ladder, bonus/penalty dice, pushed rolls, Luck), Ch. "Combat", Ch. "Chases", Ch. "Sanity" — sample phobia table at **p. 160** (per the Foundry system's own citation). Page numbers refer to the 2014 hardcover; the 2023 revised printing repaginates.
- *Call of Cthulhu 7th Edition Investigator Handbook* (CHA23136) — occupations, skill descriptions, backstory/Key Connection.
- *Call of Cthulhu 7th Edition Quick-Start Rules* — free download at <https://www.chaosium.com/cthulhu-quickstart/>. **Free ≠ open**: © Chaosium, all rights reserved, no redistribution or reuse licence.
