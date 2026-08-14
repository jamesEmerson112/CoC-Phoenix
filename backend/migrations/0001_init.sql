-- Signup slice schema.
--
-- Two halves:
--   * content_packs / skills / careers / hobbies - reference data, seeded by 0002.
--   * characters - the answers-as-inputs record (design spec section 9). The sheet is a
--     projection over `answers` and is written to `sheet` only when the character is finalized.

CREATE TABLE IF NOT EXISTS content_packs (
    id         TEXT PRIMARY KEY,
    name       TEXT        NOT NULL,
    -- 'fan' for bundled starter content under Chaosium's Fan Material Policy.
    license    TEXT        NOT NULL,
    version    TEXT        NOT NULL,
    era        TEXT        NOT NULL DEFAULT 'modern',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS skills (
    id                      TEXT PRIMARY KEY,
    pack_id                 TEXT    NOT NULL REFERENCES content_packs (id) ON DELETE CASCADE,
    name                    TEXT    NOT NULL,
    -- 'fixed' | 'dexHalf' (Dodge = DEX // 2) | 'edu' (Language (Own) = EDU)
    base_kind               TEXT    NOT NULL DEFAULT 'fixed',
    base_value              INTEGER NOT NULL DEFAULT 0,
    -- Which physical characteristic the skill leans on; drives the age-deduction allocator.
    characteristic_affinity TEXT,
    is_credit_rating        BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order              INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT skills_base_kind_check CHECK (base_kind IN ('fixed', 'dexHalf', 'edu'))
);

CREATE INDEX IF NOT EXISTS skills_pack_idx ON skills (pack_id);

CREATE TABLE IF NOT EXISTS careers (
    id                 TEXT PRIMARY KEY,
    pack_id            TEXT    NOT NULL REFERENCES content_packs (id) ON DELETE CASCADE,
    name               TEXT    NOT NULL,
    blurb              TEXT    NOT NULL DEFAULT '',
    -- [{"characteristic":"edu","multiplier":4}]; "bestOther" is Jack of All Trades' term.
    formula_terms      JSONB   NOT NULL,
    -- NULL for Jack of All Trades, which has no signature skill.
    signature_skill    TEXT,
    -- ["history", {"choice":"science"}, ...]
    supporting_skills  JSONB   NOT NULL DEFAULT '[]'::jsonb,
    choice_slots       JSONB   NOT NULL DEFAULT '[]'::jsonb,
    credit_rating_min  INTEGER NOT NULL,
    credit_rating_max  INTEGER NOT NULL,
    -- {"luck": 10} for the Jack of All Trades house rule.
    bonuses            JSONB   NOT NULL DEFAULT '{}'::jsonb,
    slots_from_hobbies BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order         INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT careers_cr_range_check CHECK (credit_rating_min <= credit_rating_max)
);

CREATE INDEX IF NOT EXISTS careers_pack_idx ON careers (pack_id);

CREATE TABLE IF NOT EXISTS hobbies (
    id              TEXT PRIMARY KEY,
    pack_id         TEXT    NOT NULL REFERENCES content_packs (id) ON DELETE CASCADE,
    name            TEXT    NOT NULL,
    blurb           TEXT    NOT NULL DEFAULT '',
    -- ~70% of the hobby's points.
    primary_skill   TEXT    NOT NULL,
    -- ~30%.
    secondary_skill TEXT    NOT NULL,
    sort_order      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS hobbies_pack_idx ON hobbies (pack_id);

CREATE TABLE IF NOT EXISTS characters (
    id              UUID PRIMARY KEY,
    -- TODO(auth): characters become account-scoped once accounts exist; nullable for now and
    -- there is deliberately no users table in this pass.
    owner_id        UUID,
    status          TEXT        NOT NULL DEFAULT 'draft',
    -- The whole answer record (design spec section 9). Never contains the computed sheet.
    answers         JSONB       NOT NULL DEFAULT '{}'::jsonb,
    -- Frozen projection, written once at finalize.
    sheet           JSONB,
    content_pack_id TEXT        NOT NULL DEFAULT 'starter-modern' REFERENCES content_packs (id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finalized_at    TIMESTAMPTZ,
    CONSTRAINT characters_status_check CHECK (status IN ('draft', 'finalized')),
    -- A finalized character always carries its snapshot; a draft never does.
    CONSTRAINT characters_snapshot_check CHECK (
        (status = 'draft' AND sheet IS NULL AND finalized_at IS NULL)
        OR (status = 'finalized' AND sheet IS NOT NULL AND finalized_at IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS characters_owner_idx ON characters (owner_id);
CREATE INDEX IF NOT EXISTS characters_status_idx ON characters (status);
