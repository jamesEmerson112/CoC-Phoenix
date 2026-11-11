# CoC Phoenix

A Call of Cthulhu 7th Edition tracker — backend-first, free hosted web app. Currently in the
**design phase**: no application code exists yet, only research and design docs.

## What this is

- A tracker for running CoC 7e games: character creation now, then campaigns, dice, combat,
  sanity, chases.
- **Era: modern day** (following the Japanese CoC community's default). Era is a content-pack
  tag; classic 1920s is a future pack, not a v1 dimension.
- **Priority #1:** the player signup wizard — a new player builds a complete investigator in
  under 2 minutes. Spec: `docs/design/01-signup-wizard-spec.md`.
- Tech stack: **Rust + PostgreSQL** (axum + sqlx + tokio). Backend code lives in `backend/`.

## Repo map

- `docs/context/` — 16 research docs on CoC 7e rules, prior art, and licensing, written for
  backend design. `00-INDEX.md` is the annotated index; `90-backend-implications.md` is the
  synthesis (candidate entities, derived-value formulas, state machines, 127 open questions) and
  the starting point for any backend design work. Rules docs are `10`–`19`, environment `20`–`21`,
  runtime layer the two `30`s.
- `docs/design/` — numbered design specs. `01-signup-wizard-spec.md` is the signup flow +
  starter content roster (17 careers, 17 hobbies, allocation rules) and is the handoff brief for
  the UI prototype. `02-style-references.md` credits the visual references (images stay local in
  the gitignored `docs/design/references/`); `03-style-directions.md` is the chosen visual style.
- `prototype/` — throwaway, browser-only build of the signup wizard (2026-11-10, user-requested):
  the backend's pure rules modules compiled to WebAssembly plus a phone-sized UI. Not the real
  front end. `prototype/README.md` covers building it and what must be kept in sync by hand.

## Decisions already made (don't re-litigate)

- Character-first signup (no campaign/Keeper required to create a character); characters are
  account-scoped, attached to campaigns later.
- Free hosted deployment under Chaosium's Fan Material Policy; bundled starter content ships as a
  fan-policy ContentPack. All other rules-content tables ship empty.
- Sheets are **projections over persisted answers** (answers-as-inputs); allocation is fully
  deterministic (same answers → same sheet); drafts are live, finalize freezes seed values.
- Dice: server rolls by default, manual "rolled at home" entry supported
  (`diceSource: server | declared`); unlimited rerolls at creation.
- Jack of All Trades career carries a +10 Luck perk — an explicit, toggleable house rule.
- Visual style: "1-bit with blood" — black ink on bone, a pixel font (DotGothic16), blood red as
  the one accent (decided 2026-11-10; `docs/design/03-style-directions.md`). Art is generated in
  code or adapted from public-domain prints, never taken from the reference works.

## Working rules

- **The user writes the code themselves** (decided 2026-08-14). Claude's role on this project is
  design discussion, rules lookup, code review, debugging help, and docs — do NOT implement
  features or spawn coding agents unless the user explicitly asks.
- The signup backend in `backend/` was finished via an ultracode workflow on 2026-08-14
  (explicitly user-requested): build clean, **177 tests green** including a 64-test
  spec-derived suite (`backend/tests/spec_suite/`) and 4 live-DB lifecycle tests smoke-tested
  against Dockerized Postgres. Known gaps, deliberate for this pass:
  - **Fan-policy notice is not yet displayed anywhere** (required by
    `docs/context/21-licensing-and-ip.md` before hosting — needs an /about route, API root,
    and README inclusion).
  - **No auth**: `characters.owner_id` is always NULL — any client can read/patch/finalize any
    character by UUID. Do not expose publicly as-is.
  - CORS is `permissive()` — local development only.

- **IP safety is a hard constraint.** Never copy Chaosium text, tables, art, or stat blocks into
  this repo — docs and content are always written in our own words with our own numbers. Test
  fixtures use invented entities only. The required fan-policy notice text lives in
  `docs/context/21-licensing-and-ip.md`.
- The exact rules mechanics (formulas, thresholds, pipelines) are already captured in
  `docs/context/` — check there before re-deriving or guessing a rule. Unresolved rules
  questions live in `90-backend-implications.md` §E; add new ones there rather than deciding
  silently.
- Docs convention: one file per topic, numbered prefix ordering (`NN-topic.md`), design specs in
  `docs/design/`, research/context in `docs/context/`.
- Use forward slashes in paths.
