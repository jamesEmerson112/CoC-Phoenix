# CoC Phoenix — signup backend

Rust + Postgres backend for the player signup wizard
(`docs/design/01-signup-wizard-spec.md`). axum + sqlx + tokio.

The whole design is one idea: **the database stores answers, not sheets.** A character row holds
what the player said (name, age, dice, career, hobbies). The investigator sheet — skill values,
derived stats, Credit Rating, cash — is recomputed by pure functions on every read. Finalizing
snapshots that projection into the row and freezes the answers.

---

## Quick start

```bash
cd backend

# 1. Postgres (db name coc_phoenix, user/password postgres/postgres)
docker compose up -d

# 2. Connection string. Copy .env.example if you prefer a file.
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/coc_phoenix

# 3. Build, test, run. Migrations (schema + starter content) apply on boot.
cargo build
cargo test
cargo run
```

Then `curl http://127.0.0.1:8080/health`.

`DATABASE_URL` defaults to `postgres://postgres:postgres@localhost:5432/coc_phoenix` when unset,
and `BIND_ADDR` to `127.0.0.1:8080`. See `.env.example` for every knob.

**Port 5432 already in use?** `POSTGRES_PORT=5433 docker compose up -d` and point `DATABASE_URL`
at 5433.

**Migrations without booting the server:** they run automatically at startup. To apply them by
hand instead, use `sqlx-cli` (`cargo install sqlx-cli --no-default-features --features postgres`,
then `sqlx migrate run`) and start the server with `RUN_MIGRATIONS=false`. The bundled content
pack is published separately from the migrations, right after them — see **Content** below;
`SYNC_CONTENT=false` skips it.

`cargo build` and `cargo test` never need a database: the code uses the runtime sqlx API
(`sqlx::query` with manual binding) rather than the compile-time `query!` macros. The one
database-backed test in `tests/db_lifecycle.rs` skips itself unless `DATABASE_URL` is set.

---

## Layout

```
backend/
├── Cargo.toml
├── docker-compose.yml            # local Postgres 16, db `coc_phoenix`
├── .env.example
├── content/
│   └── starter_pack.json         # SINGLE SOURCE OF TRUTH for bundled content
├── migrations/
│   ├── 0001_init.sql             # schema
│   ├── 0002_seed_starter_pack.sql# HISTORY: the original seed. Never edit or regenerate.
│   └── 0003_content_pack_keys.sql# (pack_id, id) keys, skill FKs, affinity CHECK
├── src/
│   ├── lib.rs                    # module map + AppState
│   ├── main.rs                   # connect → migrate → load catalogue → serve
│   ├── config.rs                 # env config
│   ├── model.rs                  # Answers, Characteristics, AgeBand, dice records (spec § 9)
│   ├── content.rs                # Career/Hobby/Skill types + the in-memory Catalog
│   ├── dice.rs                   # PURE ceremonies + declared-result validation (spec § 3)
│   ├── derived.rs                # PURE derived values + age arithmetic (spec § 7, § 2)
│   ├── allocator.rs              # PURE allocation ladders, cap, age deductions (spec § 6)
│   ├── projection.rs             # answers + catalogue → Sheet (spec § 2 step 6, § 9)
│   ├── seed.rs                   # renders the JSON as idempotent upsert SQL
│   ├── extract.rs                # Json/Path extractors that fail through the error envelope
│   ├── db.rs                     # Postgres access (runtime sqlx API)
│   ├── error.rs                  # one JSON error envelope
│   ├── bin/gen_seed.rs           # `cargo run --bin gen-seed`
│   └── routes/
│       ├── mod.rs                # router
│       ├── characters.rs         # the wizard endpoints
│       └── content.rs            # career / hobby / skill rosters
└── tests/
    ├── db_lifecycle.rs           # live-DB integration tests (skipped without DATABASE_URL)
    └── spec_suite/               # independent spec conformance suite
```

### Where the spec lives in the code

| Spec section | Module | What it holds |
|---|---|---|
| § 2 age bands, deductions | `derived::age_effects` | EDU checks, physical/APP/MOV costs, Luck rolls per decade |
| § 3 dice ceremonies | `dice` | 3D6×5 / (2D6+6)×5, EDU check sequence, Luck, declared-value validation |
| § 4 / § 4.1 careers | `content/starter_pack.json`, `allocator::allocate_jack` | 17 careers, formula terms, choice slots, Jack of All Trades |
| § 5 hobbies | `content/starter_pack.json`, `allocator::allocate_personal` | 17 hobbies, 70/30 primary/secondary |
| § 6 allocation | `allocator` | ladders, CR-minimum-first, pick-order weighting, 75 cap, age-deduction targeting |
| § 7 derived values | `derived` | HP, MP, SAN, DB/Build, MOV, half/fifth, Dodge and Language (Own) bases |
| § 9 lifecycle + data shape | `model`, `db`, `projection` | the answers record, draft → finalized, the snapshot |

`allocator`, `derived` and `dice` import nothing from `db` or `axum`. They are unit tested with
invented fixture content (lighthouse keepers, kite couriers) so the tests carry no game content
at all.

---

## API

No auth in this pass. `characters.owner_id` is nullable and always `NULL` — see the
`TODO(auth)` in `migrations/0001_init.sql`.

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/health` | liveness |
| `POST` | `/characters` | create an empty draft |
| `GET` | `/characters/:id` | answers + projected sheet |
| `PATCH` | `/characters/:id` | partial answer update (409 once finalized) |
| `POST` | `/characters/:id/roll` | run a dice ceremony, or submit declared results |
| `POST` | `/characters/:id/finalize` | validate, snapshot the sheet, freeze |
| `GET` | `/content/careers` | 17 career cards, optionally sorted by fit |
| `GET` | `/content/hobbies` | 17 hobby cards |
| `GET` | `/content/skills` | skill catalogue (names and bases) |

Every character response is the same envelope:

```jsonc
{
  "id": "…", "ownerId": null, "status": "draft",
  "answers": { … },              // spec § 9 record
  "sheet": { … },                // projected live, or the frozen snapshot
  "sheetIsSnapshot": false,
  "contentPackId": "starter-modern",
  "createdAt": "…", "updatedAt": "…", "finalizedAt": null
}
```

Errors are always `{"error": {"code", "message", "details": []}}`, including the ones raised
before a handler runs. `AppJson` / `AppPath` (`src/extract.rs`) convert axum's extractor
rejections — malformed JSON (400), an unknown field (422), a missing
`Content-Type: application/json` (415), a malformed UUID in the path (400) — into the same
envelope, so `const {error} = await res.json()` is always safe.

### `PATCH /characters/:id`

Accepts any subset of `name`, `pronouns`, `age`, `ageBand`, `careerId`, `careerChoices`,
`hobbyIds`, `backstory`. Unknown fields are rejected. Dice go through `/roll`.
No recomputation is triggered — the sheet is computed on read, so the response already reflects
the change.

```bash
curl -X PATCH localhost:8080/characters/$ID -H 'content-type: application/json' \
  -d '{"name":"Ada Kowalski","pronouns":"she/her","age":48}'
```

Validation: age 15–89; `careerId` and every `hobbyId` must exist; at most 4 hobbies; each
`careerChoices` answer must be a real option of a real slot for the current career (changing
career clears stale choices).

### `POST /characters/:id/roll`

```bash
# server rolls (spec § 3a/3b/3c) — unlimited rerolls while drafting
curl -X POST …/roll -d '{"ceremony":"characteristics"}'
curl -X POST …/roll -d '{"ceremony":"edu_checks"}'   # count comes from the age band
curl -X POST …/roll -d '{"ceremony":"luck"}'         # two rolls for Young

# "I rolled at home" — any subset, validated
curl -X POST …/roll -d '{"ceremony":"declared",
  "characteristics":{"str":50,"con":60,"siz":60,"dex":55,"app":50,"pow":65,"int":70,"edu":75},
  "eduChecks":[{"d100":71,"gain":7}],
  "luck":[40,75]}'
```

Declared values must be multiples of 5, 15–90 for STR/CON/DEX/APP/POW, 40–90 for SIZ/INT/EDU;
Luck 15–90 with at most the number of rolls the band grants; EDU checks at most the band's
allowance, `d100` 1–100 and `gain` **1–10** (a physical d10 printed 0–9 shows its `0` face for
ten — accepting `0` would silently cost the player the whole gain). Failures return every problem
at once in `details`. Unknown fields are rejected, so a misspelled `educhecks` is a 422, never a
200 that quietly drops the values.

`eduChecks` and `luck` require the age question to be answered first.

**Provenance.** `dice.sources` records where each block came from
(`{"characteristics": "declared", "luck": "server"}`); `dice.source` is the summary the spec § 9
record names — `declared` as soon as any block was hand-entered. A mixed record therefore never
claims that hand-entered stats were server-rolled, or the reverse.

### `POST /characters/:id/finalize`

400 with a `details` list if anything is missing (name, age, characteristics, the right number of
EDU checks, **the right number of Luck rolls**, a career, its choice slots, exactly four distinct
hobbies). Otherwise the sheet is snapshotted, `status` becomes `finalized`, and further
`PATCH`/`roll`/`finalize` calls 409.

Luck is checked against the band exactly the way EDU checks are: a Young investigator needs two
rolls (§ 2 "rolled twice, keep the better"), everyone else exactly one. Going back and changing
the age band after rolling therefore asks for a re-roll instead of freezing a single roll into a
snapshot that still advertises `luckRollsGranted: 2`.

The read, the projection and the freeze all happen in one transaction with the row locked
(`db::finalize_atomically`), so the frozen sheet is always the projection of the frozen answers
even if an autosave `PATCH` is in flight — the record is immutable afterwards, so a mismatch
would be permanent. `PATCH` and `/roll` use the same lock (`db::mutate_answers`), because they
rewrite the whole answers document rather than merging.

### `GET /content/careers`

Pass rolled characteristics to sort best-payout-first:

```bash
curl 'localhost:8080/content/careers?str=50&con=60&siz=55&dex=70&app=45&pow=65&int=75&edu=80'
```

Each card carries `fit` (the career pool those characteristics would produce), the human-readable
`formula`, the signature and supporting skills, any `choiceSlots`, the CR range, and
`bonuses`/`slotsFromHobbies` for Jack of All Trades.

---

## Content

`content/starter_pack.json` is the single source of truth for the bundled pack: 17 careers,
17 hobbies, 53 skills. **Edit the JSON and restart — that is the whole loop.** The server renders
it to SQL and upserts it on every boot (`db::sync_content_pack`, `SYNC_CONTENT=false` to skip).

Content is *data*, deliberately outside the checksummed migration chain. It used to be
`migrations/0002_seed_starter_pack.sql`, rewritten in place by `gen-seed`, which made bundled
content effectively unpublishable in two independent ways:

- sqlx checksums applied migrations, so regenerating 0002 made the server refuse to boot against
  every database that had already run it (`MigrateError::VersionMismatch`), including deployed
  ones — recoverable only by deleting the `_sqlx_migrations` row or dropping the database;
- every insert ended `ON CONFLICT DO NOTHING`, so even a hand-applied re-run silently no-opped on
  existing rows. An edited blurb, CR range or skill list could never reach a seeded database.

`migrations/0002_seed_starter_pack.sql` is now **history**: leave it byte-for-byte as it was
applied, never regenerate it, and never add generated content SQL to `migrations/` again.
`0003_content_pack_keys.sql` re-keys the content tables on `(pack_id, id)` so the upserts work
and so a future pack (classic-1920s, Gaslight — spec § 10) can reuse ids like `dodge`.

```bash
cargo run --bin gen-seed                      # print the upsert SQL (review / hand-apply)
cargo run --bin gen-seed content/pack.sql     # ...or dump it; writing into migrations/ is refused
```

Skill references in careers and hobbies are validated when a pack is loaded and before it is
published: a dangling id is a hard `content_error` rather than points that vanish from the sheet
while still counting as pool spent.

All content is written by us and ships under Chaosium's Fan Material Policy
(`content_packs.license = 'fan'`). No rulebook text, tables or stat blocks are reproduced
anywhere in this crate, and the rules-module tests use invented entities only.

Skill base values are our own numbers on a four-tier scheme, documented in the JSON's
`baseTiers`: `common` 20 (anything a modern adult already half-does), `familiar` 10,
`trained` 5, `formal` 1 — plus the two formula bases, Dodge = `DEX // 2` and
Language (Own) = `EDU`, and Credit Rating at 0.

---

## Spec ambiguities and how they were resolved

Every one of these is a documented choice, not a silent guess. Anything that should be escalated
belongs in `docs/context/90-backend-implications.md` § E.

1. **Where the age deduction actually lands.** § 6 says "the characteristic among STR/CON/DEX
   that contributes least to the chosen career's formula and skill list" without defining
   "contributes". Implemented as `score = 10 × (formula multiplier) + (count of the career's
   skills that lean on that characteristic)`, lowest score pays, ties break CON → STR → DEX. The
   per-skill lean is a nullable `characteristic_affinity` column on `skills` (e.g. Climb → STR,
   Stealth → DEX, Survival → CON), which is our own authored data.
2. **The deduction goes to one characteristic, not several.** § 6 is singular ("the
   characteristic"), so the full amount lands on one. If it would push that characteristic below
   1 the remainder cascades to the next-cheapest — a Venerable 80-point deduction has nowhere
   else to go.
3. **Young's "−5 split between STR and SIZ".** "Split" is taken literally: 3 from the
   least-needed and 2 from the other, using the same scoring, with SIZ leading the tie-break (no
   career formula in the roster uses SIZ). Deterministic and stable as the player shops careers.
4. **Jack of All Trades' "best other" and the age deduction are circular** — the pool depends on
   the highest characteristic, the deduction depends on the pool's formula. Resolved by pinning
   the "best other" hint *before* deductions for scoring purposes, then re-picking it on the
   final values when the pool is actually computed. Candidates are the seven non-EDU
   characteristics (SIZ included), ties break STR, CON, SIZ, DEX, APP, POW, INT.
5. **Raw vs post-age characteristics in the answer record.** § 9's example shows "final post-age
   values + raw rolls kept". Because the physical deduction depends on the career (which changes
   while drafting), the stored answers keep only the **raw** rolls; post-age values are part of
   the projection. Declared characteristics are likewise entered raw.
6. **EDU checks are stored as `{d100, gain}` with the d10 always recorded**, even when the check
   misses, so re-rolling characteristics re-derives the same sequence. Whether a check applies is
   decided on projection (`d100 > current EDU`, strictly sequential, clamped at 99). `gain` is
   1–10 for declared checks exactly as for server ones: `0` is not a d10 result, and it is the
   face a die printed 0–9 shows for ten, so accepting it would quietly cost the player 10 EDU.
   Only the checks the band grants are applied — going back from Venerable to Young keeps the
   stored rolls (nothing is destroyed by going back) but spends none of them.
7. **The 75 cap never claws back a base value.** Language (Own) = EDU can start at 90. Final
   value is `max(base, min(75, base + career + personal))`; points lost are not redistributed and
   are reported per skill as `pointsLostToCap`.
8. **Partly-answered drafts do not overspend.** Only the ladder/pick weights that resolve are
   spendable, so a draft with one hobby spends 40% of the personal pool rather than all of it.
   With all four picks this is identical to the spec's arithmetic.
9. **Unallocatable slot counts.** All 17 careers land on the 5/6/7 ladders; any future career with
   a different count falls back to an even split whose percentages still sum to exactly 100, the
   shortfall going to the signature slot (`100 / 3` is 33, not 33.33 — an unadjusted even split
   would silently burn 1% of the pool for 3 slots and 4% for 8).
10. **Career pools smaller than the CR minimum.** Credit Rating is still set to the career's
    minimum (it is what the profession pays) and the skill ladder gets nothing — but the points
    the pool could not cover are published as `pools.careerPoolShortfall` instead of leaving the
    sheet showing `careerPoolSpent` above `careerPool` with no explanation. The ledger always
    reconciles: `sum(career points) == careerPoolSpent == careerPool + careerPoolShortfall`.
    Reachable only when age deductions have gutted a two-characteristic career's formula (an
    80-something MMA fighter, say).
11. **`Firearms (Rifle)` (career 7) and `Firearms (Rifle/Shotgun)` (hobby 3)** are the same
    catalogue row, so a hunting Soldier stacks rather than splitting across two skills.
12. **Cash figures.** § 7 does not give one; the tier *shape* comes from
    `90-backend-implications.md` § B and the actual numbers and the plain-words descriptors are
    ours. Era multipliers are not applied (modern is the only era).
13. **`status` is a column, not a field inside `answers`** — the § 9 example shows it inline, but
    it drives a DB check constraint and the finalize guard. It is echoed at the top level of every
    API response.
14. **Damage Bonus / Build past the printed ladder** uses the open-ended formula from § B
    (`n = (STR+SIZ−45) // 80` → `nD6`, Build `n+1`), which agrees with § 7 on every band it lists.
15. **Luck completeness** requires exactly the number of rolls the band grants, mirroring the EDU
    check gate: two for Young, one otherwise. Declared entry still accepts *fewer* than the
    allowance mid-draft (as EDU checks do), but finalize will not freeze a Young investigator
    with a single roll — "rolled twice, keep the better" is the band's entire compensation for
    EDU −5 and the split STR/SIZ cost, and the record can never be edited afterwards.
16. **Dice provenance is per block.** § 9 models one `diceSource`, but the three ceremonies are
    run separately and mixing them is a first-class flow (declare stats rolled at home, then tap
    the server Luck ceremony for the moment § 3c stages). `dice.sources` records each block;
    `dice.source` remains as the § 9 summary field and reads `declared` if any block was
    hand-entered. Records written before this field exists still load.

Still open upstream (spec § 11, unchanged by this pass): the exact creation-time cap (75 is the
working number), the improvement-check threshold convention, and the printing to cite.

---

## Testing

```bash
cargo test                                     # unit tests + spec suite, no database needed
DATABASE_URL=… cargo test                      # adds the live-DB lifecycle tests
```

`tests/spec_suite/` is an independent conformance suite whose expected values are hand-computed
from the spec alone; where it and the implementation disagree, it encodes the spec.

The live-DB tests additionally cover the two things a single process cannot: that publishing
edited content reaches rows an earlier seed already wrote, and that concurrent edits neither lose
writes nor let a finalized sheet drift from the answers it was frozen with.

Unit tests cover every § 7 formula (including the DB/Build band edges and the MOV `>` vs `>=`
comparison), the 5/6/7 ladders, CR-minimum-first, the 75 cap, the 40/30/20/10 and 70/30 splits
with their remainder rules, Jack of All Trades allocation and its Luck perk, age-deduction
targeting and cascading, declared-dice range validation, and determinism (the same answers
serialize to a byte-identical sheet, before and after a JSON round trip).
