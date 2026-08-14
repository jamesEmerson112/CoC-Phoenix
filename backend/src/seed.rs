//! Renders `content/starter_pack.json` into idempotent upsert SQL.
//!
//! The JSON file is the single source of truth for bundled content. This SQL is **not** a
//! migration: it is regenerated in memory and applied by [`crate::db::sync_content_pack`] every
//! time the server boots.
//!
//! It used to be `migrations/0002_seed_starter_pack.sql`, rewritten in place by `gen-seed`. That
//! made content unpublishable in two independent ways: sqlx checksums applied migrations, so
//! editing a blurb made the server refuse to boot against every existing database
//! (`VersionMismatch`), and the inserts ended `ON CONFLICT DO NOTHING`, so even a hand-applied
//! re-run could never update a row that was already seeded. Content is data; it belongs outside
//! the migration chain. 0002 is left byte-for-byte as it was applied — it is history.

use crate::content::{PackFile, SkillSlot};

pub const GENERATED_HEADER: &str = "\
-- GENERATED - do not edit by hand, and do not add this to migrations/.
-- Source of truth: backend/content/starter_pack.json
-- Applied on every boot by db::sync_content_pack; dump a copy with `cargo run --bin gen-seed`.
--
-- Bundled starter content for the signup wizard (design spec sections 4 and 5).
-- Every name, blurb, formula, Credit Rating range and base value here is authored by us and
-- ships under the Chaosium Fan Material Policy (license = 'fan').
--
-- Every statement is an upsert keyed on (pack_id, id), so re-applying it publishes content
-- edits instead of silently no-opping on rows that already exist.
";

/// `ON CONFLICT ... DO UPDATE SET` clause for a content table keyed on (pack_id, id).
fn upsert(columns: &[&str]) -> String {
    let assignments: Vec<String> = columns
        .iter()
        .map(|c| format!("  {c} = EXCLUDED.{c}"))
        .collect();
    format!(
        "\nON CONFLICT (pack_id, id) DO UPDATE SET\n{};\n",
        assignments.join(",\n")
    )
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn quote_option(value: Option<&str>) -> String {
    match value {
        Some(v) => quote(v),
        None => "NULL".to_string(),
    }
}

fn jsonb(value: &serde_json::Value) -> String {
    format!("{}::jsonb", quote(&serde_json::to_string(value).unwrap()))
}

fn boolean(value: bool) -> &'static str {
    if value {
        "TRUE"
    } else {
        "FALSE"
    }
}

/// Render the seed migration for a content pack.
pub fn seed_sql(file: &PackFile) -> String {
    let pack_id = file.pack.id.as_str();
    let mut out = String::from(GENERATED_HEADER);

    out.push_str("\nINSERT INTO content_packs (id, name, license, version, era) VALUES\n  (");
    out.push_str(&[
        quote(pack_id),
        quote(&file.pack.name),
        quote(&file.pack.license),
        quote(&file.pack.version),
        quote(&file.pack.era),
    ]
    .join(", "));
    out.push_str(
        ")\nON CONFLICT (id) DO UPDATE SET\n  name = EXCLUDED.name,\n  \
         license = EXCLUDED.license,\n  version = EXCLUDED.version,\n  era = EXCLUDED.era;\n",
    );

    out.push_str(
        "\nINSERT INTO skills \
         (id, pack_id, name, base_kind, base_value, characteristic_affinity, is_credit_rating, sort_order)\nVALUES\n",
    );
    let skill_rows: Vec<String> = file
        .skills
        .iter()
        .enumerate()
        .map(|(i, skill)| {
            format!(
                "  ({}, {}, {}, {}, {}, {}, {}, {})",
                quote(&skill.id),
                quote(pack_id),
                quote(&skill.name),
                quote(skill.base.kind_key()),
                skill.base.fixed_value(),
                quote_option(skill.affinity.map(|a| a.key())),
                boolean(skill.credit_rating),
                i
            )
        })
        .collect();
    out.push_str(&skill_rows.join(",\n"));
    out.push_str(&upsert(&[
        "name",
        "base_kind",
        "base_value",
        "characteristic_affinity",
        "is_credit_rating",
        "sort_order",
    ]));

    out.push_str(
        "\nINSERT INTO careers \
         (id, pack_id, name, blurb, formula_terms, signature_skill, supporting_skills, choice_slots, \
         credit_rating_min, credit_rating_max, bonuses, slots_from_hobbies, sort_order)\nVALUES\n",
    );
    let career_rows: Vec<String> = file
        .careers
        .iter()
        .enumerate()
        .map(|(i, career)| {
            let supporting: Vec<serde_json::Value> = career
                .supporting_skills
                .iter()
                .map(|slot| match slot {
                    SkillSlot::Fixed(id) => serde_json::Value::String(id.clone()),
                    SkillSlot::Choice { choice } => {
                        serde_json::json!({ "choice": choice })
                    }
                })
                .collect();
            format!(
                "  ({}, {}, {}, {},\n   {},\n   {}, {},\n   {},\n   {}, {}, {}, {}, {})",
                quote(&career.id),
                quote(pack_id),
                quote(&career.name),
                quote(&career.blurb),
                jsonb(&serde_json::to_value(&career.formula_terms).unwrap()),
                quote_option(career.signature_skill.as_deref()),
                jsonb(&serde_json::Value::Array(supporting)),
                jsonb(&serde_json::to_value(&career.choice_slots).unwrap()),
                career.credit_rating.min,
                career.credit_rating.max,
                jsonb(&serde_json::to_value(career.bonuses).unwrap()),
                boolean(career.slots_from_hobbies),
                i
            )
        })
        .collect();
    out.push_str(&career_rows.join(",\n"));
    out.push_str(&upsert(&[
        "name",
        "blurb",
        "formula_terms",
        "signature_skill",
        "supporting_skills",
        "choice_slots",
        "credit_rating_min",
        "credit_rating_max",
        "bonuses",
        "slots_from_hobbies",
        "sort_order",
    ]));

    out.push_str(
        "\nINSERT INTO hobbies (id, pack_id, name, blurb, primary_skill, secondary_skill, sort_order)\nVALUES\n",
    );
    let hobby_rows: Vec<String> = file
        .hobbies
        .iter()
        .enumerate()
        .map(|(i, hobby)| {
            format!(
                "  ({}, {}, {}, {}, {}, {}, {})",
                quote(&hobby.id),
                quote(pack_id),
                quote(&hobby.name),
                quote(&hobby.blurb),
                quote(&hobby.primary_skill),
                quote(&hobby.secondary_skill),
                i
            )
        })
        .collect();
    out.push_str(&hobby_rows.join(",\n"));
    out.push_str(&upsert(&[
        "name",
        "blurb",
        "primary_skill",
        "secondary_skill",
        "sort_order",
    ]));

    out
}

/// The upsert SQL for the bundled starter pack.
pub fn starter_seed_sql() -> String {
    seed_sql(&crate::content::starter_pack_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_quotes_in_content_are_escaped_for_sql() {
        let sql = starter_seed_sql();
        assert!(sql.contains("you''re"), "apostrophes must be doubled");
        assert!(!sql.contains("you're"));
    }

    #[test]
    fn every_roster_entry_reaches_the_sql() {
        let file = crate::content::starter_pack_file();
        let sql = starter_seed_sql();
        for career in &file.careers {
            assert!(sql.contains(&format!("('{}'", career.id)), "{}", career.id);
        }
        for hobby in &file.hobbies {
            assert!(sql.contains(&format!("('{}'", hobby.id)), "{}", hobby.id);
        }
        for skill in &file.skills {
            assert!(sql.contains(&format!("('{}'", skill.id)), "{}", skill.id);
        }
    }

    /// The whole point of moving content out of the migration chain: re-applying it has to
    /// publish edits, not silently no-op on rows that were seeded once.
    #[test]
    fn every_content_statement_upserts_rather_than_skipping_existing_rows() {
        let sql = starter_seed_sql();
        assert!(
            !sql.contains("DO NOTHING"),
            "a DO NOTHING clause means edited content can never reach a seeded database"
        );
        assert_eq!(
            sql.matches("ON CONFLICT (pack_id, id) DO UPDATE SET").count(),
            3,
            "skills, careers and hobbies each upsert on the composite key"
        );
        assert!(sql.contains("ON CONFLICT (id) DO UPDATE SET"), "content_packs");
        // Fields a content pass actually edits must be in the update lists.
        for column in ["name", "blurb", "credit_rating_min", "characteristic_affinity"] {
            assert!(
                sql.contains(&format!("{column} = EXCLUDED.{column}")),
                "{column} is not refreshed on conflict"
            );
        }
    }

    /// A second pack (spec section 10: classic-1920s, Gaslight) reuses ids like `dodge`, so the
    /// key has to carry the pack.
    #[test]
    fn the_upsert_key_carries_the_pack_id() {
        assert!(!starter_seed_sql().contains("ON CONFLICT (id) DO UPDATE SET\n  name = EXCLUDED.name,\n  base_kind"));
        assert!(starter_seed_sql().contains("ON CONFLICT (pack_id, id)"));
    }
}
