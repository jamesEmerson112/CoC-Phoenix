-- Make bundled content editable, and make a second content pack possible.
--
-- Two problems in 0001/0002 this fixes:
--
--   1. skills/careers/hobbies keyed on a GLOBAL `id` even though every runtime query filters by
--      `pack_id`. The spec (section 10) names classic-1920s and Gaslight as future packs, and
--      those must reuse ids like 'dodge' and 'psychology'. With a global key, seeding a second
--      pack does not error - it silently skips every shared row and serves a truncated
--      catalogue. The key is (pack_id, id).
--   2. The seed was a versioned migration, so an edited blurb or Credit Rating range could never
--      reach a database that had already run it. Content is *data*: it now lives outside the
--      checksummed migration chain and is upserted at boot (see db::sync_content_pack). 0002
--      stays exactly as it was applied - it is history, not the source of truth.
--
-- Also adds the integrity the plain-TEXT skill columns were missing.

ALTER TABLE skills   DROP CONSTRAINT IF EXISTS skills_pkey;
ALTER TABLE careers  DROP CONSTRAINT IF EXISTS careers_pkey;
ALTER TABLE hobbies  DROP CONSTRAINT IF EXISTS hobbies_pkey;

ALTER TABLE skills  ADD CONSTRAINT skills_pkey  PRIMARY KEY (pack_id, id);
ALTER TABLE careers ADD CONSTRAINT careers_pkey PRIMARY KEY (pack_id, id);
ALTER TABLE hobbies ADD CONSTRAINT hobbies_pkey PRIMARY KEY (pack_id, id);

-- `base_kind` already had a CHECK; affinity had none, and db.rs used to coerce anything
-- unrecognised to NULL. Affinity picks which characteristic pays the age deduction (section 6),
-- so a mis-cased 'DEX' silently changed HP, Build, MOV and Dodge.
ALTER TABLE skills ADD CONSTRAINT skills_affinity_check CHECK (
    characteristic_affinity IS NULL
    OR characteristic_affinity IN ('str', 'con', 'siz', 'dex', 'app', 'pow', 'int', 'edu')
);

-- Skill references inside the same pack. The JSONB slot lists cannot be constrained here, so
-- db::load_catalog validates those; these cover the plain columns.
ALTER TABLE careers ADD CONSTRAINT careers_signature_skill_fkey
    FOREIGN KEY (pack_id, signature_skill) REFERENCES skills (pack_id, id);
ALTER TABLE hobbies ADD CONSTRAINT hobbies_primary_skill_fkey
    FOREIGN KEY (pack_id, primary_skill) REFERENCES skills (pack_id, id);
ALTER TABLE hobbies ADD CONSTRAINT hobbies_secondary_skill_fkey
    FOREIGN KEY (pack_id, secondary_skill) REFERENCES skills (pack_id, id);
