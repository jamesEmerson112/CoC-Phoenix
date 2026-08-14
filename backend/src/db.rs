//! Postgres access. Deliberately uses the runtime sqlx API (`sqlx::query` / manual binding)
//! rather than the compile-time `query!` macros, so `cargo build` and `cargo test` never need a
//! live database or a `DATABASE_URL`.

use crate::content::{Career, Catalog, ChoiceSlot, Hobby, Skill, SkillBase, SkillSlot};
use crate::content::{CareerBonuses, CreditRatingRange, FormulaTerm};
use crate::error::{AppError, AppResult};
use crate::model::{Answers, Characteristic, Status};
use crate::projection::Sheet;
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{PgExecutor, Row};
use std::time::Duration;
use uuid::Uuid;

pub const DEFAULT_PACK_ID: &str = "starter-modern";

pub async fn connect(database_url: &str, max_connections: u32) -> AppResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Apply the migrations embedded at compile time from `backend/migrations`.
pub async fn migrate(pool: &PgPool) -> AppResult<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Content(format!("migration failed: {e}")))
}

/// A character row: the answers, the status, and (only once finalized) the frozen sheet.
#[derive(Debug, Clone)]
pub struct CharacterRecord {
    pub id: Uuid,
    /// TODO(auth): characters are account-scoped once accounts exist; nullable until then.
    pub owner_id: Option<Uuid>,
    pub status: Status,
    pub answers: Answers,
    pub sheet: Option<Sheet>,
    pub content_pack_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finalized_at: Option<DateTime<Utc>>,
}

const CHARACTER_COLUMNS: &str =
    "id, owner_id, status, answers, sheet, content_pack_id, created_at, updated_at, finalized_at";

fn character_from_row(row: &sqlx::postgres::PgRow) -> AppResult<CharacterRecord> {
    let status: String = row.try_get("status")?;
    let answers: serde_json::Value = row.try_get("answers")?;
    let sheet: Option<serde_json::Value> = row.try_get("sheet")?;
    Ok(CharacterRecord {
        id: row.try_get("id")?,
        owner_id: row.try_get("owner_id")?,
        status: Status::from_key(&status)
            .ok_or_else(|| AppError::Content(format!("unknown character status '{status}'")))?,
        answers: serde_json::from_value(answers)?,
        sheet: sheet.map(serde_json::from_value).transpose()?,
        content_pack_id: row.try_get("content_pack_id")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        finalized_at: row.try_get("finalized_at")?,
    })
}

pub async fn create_character<'e, E>(exec: E, pack_id: &str) -> AppResult<CharacterRecord>
where
    E: PgExecutor<'e>,
{
    let sql = format!(
        "INSERT INTO characters (id, answers, content_pack_id) VALUES ($1, $2, $3) RETURNING {CHARACTER_COLUMNS}"
    );
    let row = sqlx::query(&sql)
        .bind(Uuid::new_v4())
        .bind(serde_json::to_value(Answers::default())?)
        .bind(pack_id)
        .fetch_one(exec)
        .await?;
    character_from_row(&row)
}

pub async fn get_character<'e, E>(exec: E, id: Uuid) -> AppResult<CharacterRecord>
where
    E: PgExecutor<'e>,
{
    let sql = format!("SELECT {CHARACTER_COLUMNS} FROM characters WHERE id = $1");
    let row = sqlx::query(&sql)
        .bind(id)
        .fetch_optional(exec)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("character {id}")))?;
    character_from_row(&row)
}

/// Read a character inside a transaction, holding a row lock until it commits.
///
/// Every mutation is a read-modify-write over the whole `answers` document, so without the lock
/// two overlapping calls lose one another's writes, and `finalize` can snapshot a sheet that is
/// no longer the projection of the stored answers — permanently, because the record freezes
/// (spec § 9: the frozen sheet *is* the projection of the frozen answers).
pub async fn get_character_for_update(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
) -> AppResult<CharacterRecord> {
    let sql = format!("SELECT {CHARACTER_COLUMNS} FROM characters WHERE id = $1 FOR UPDATE");
    let row = sqlx::query(&sql)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("character {id}")))?;
    character_from_row(&row)
}

/// Persist a new answer set. Refuses to touch a finalized character (spec § 9).
pub async fn update_answers<'e, E>(
    exec: E,
    id: Uuid,
    answers: &Answers,
) -> AppResult<CharacterRecord>
where
    E: PgExecutor<'e>,
{
    let sql = format!(
        "UPDATE characters SET answers = $2, updated_at = now() \
         WHERE id = $1 AND status = 'draft' RETURNING {CHARACTER_COLUMNS}"
    );
    let row = sqlx::query(&sql)
        .bind(id)
        .bind(serde_json::to_value(answers)?)
        .fetch_optional(exec)
        .await?
        .ok_or_else(|| AppError::Conflict(format!("character {id} is finalized and frozen")))?;
    character_from_row(&row)
}

/// Freeze the character: store the computed sheet and flip the status (spec § 9).
pub async fn finalize_character<'e, E>(
    exec: E,
    id: Uuid,
    sheet: &Sheet,
) -> AppResult<CharacterRecord>
where
    E: PgExecutor<'e>,
{
    let sql = format!(
        "UPDATE characters SET status = 'finalized', sheet = $2, finalized_at = now(), \
         updated_at = now() WHERE id = $1 AND status = 'draft' RETURNING {CHARACTER_COLUMNS}"
    );
    let row = sqlx::query(&sql)
        .bind(id)
        .bind(serde_json::to_value(sheet)?)
        .fetch_optional(exec)
        .await?
        .ok_or_else(|| AppError::Conflict(format!("character {id} is already finalized")))?;
    character_from_row(&row)
}

/// Read the answers, let `edit` produce the next version, and write it — all inside one
/// transaction with the row locked.
///
/// `update_answers` rewrites the whole `answers` document rather than merging, so two overlapping
/// edits (the wizard's debounced name autosave and a `/roll`, say) would otherwise silently
/// discard one of them.
pub async fn mutate_answers<F>(pool: &PgPool, id: Uuid, edit: F) -> AppResult<CharacterRecord>
where
    F: FnOnce(&CharacterRecord) -> AppResult<Answers>,
{
    let mut tx = pool.begin().await?;
    let record = get_character_for_update(&mut tx, id).await?;
    let answers = edit(&record)?;
    let record = update_answers(&mut *tx, id, &answers).await?;
    tx.commit().await?;
    Ok(record)
}

/// Project the sheet and freeze it in the same transaction that read the answers.
///
/// Spec § 9 makes the frozen sheet the projection of the frozen answers, and the record is
/// immutable afterwards — so a write landing between an unlocked read and the freeze would
/// corrupt it permanently, with no API left to reconcile the two.
pub async fn finalize_atomically<F>(
    pool: &PgPool,
    id: Uuid,
    make_sheet: F,
) -> AppResult<CharacterRecord>
where
    F: FnOnce(&CharacterRecord) -> AppResult<Sheet>,
{
    let mut tx = pool.begin().await?;
    let record = get_character_for_update(&mut tx, id).await?;
    let sheet = make_sheet(&record)?;
    let record = finalize_character(&mut *tx, id, &sheet).await?;
    tx.commit().await?;
    Ok(record)
}

/// Publish a content pack into the database, idempotently.
///
/// Content is *data*, not schema: keeping the seed inside the checksummed migration chain meant
/// that editing a career blurb made sqlx refuse to boot against every database that had already
/// applied it (`VersionMismatch`), while the `ON CONFLICT DO NOTHING` inserts meant a re-run
/// could never update a seeded row anyway. This runs after the migrations on every boot and
/// upserts on `(pack_id, id)`, so a content edit reaches existing databases and a second pack
/// can share skill ids with the first.
///
/// Rows in the database that the pack no longer defines are left alone; removing content is a
/// deliberate act, not something a boot should do behind the operator's back.
pub async fn sync_content_pack(pool: &PgPool, file: &crate::content::PackFile) -> AppResult<()> {
    let problems = crate::content::dangling_skill_references(&Catalog::from_pack_file(file.clone()));
    if !problems.is_empty() {
        return Err(AppError::Content(format!(
            "content pack '{}' has {} broken skill reference(s) and was not published: {}",
            file.pack.id,
            problems.len(),
            problems.join("; ")
        )));
    }

    let sql = crate::seed::seed_sql(file);
    // Multi-statement SQL with no bind parameters: sqlx sends this over the simple query
    // protocol, which wraps the whole script in one implicit transaction.
    sqlx::Executor::execute(pool, sql.as_str())
        .await
        .map_err(|e| AppError::Content(format!("publishing content pack failed: {e}")))?;
    Ok(())
}

/// Load one content pack into the pure in-memory catalogue used by the allocator.
pub async fn load_catalog(pool: &PgPool, pack_id: &str) -> AppResult<Catalog> {
    let skill_rows = sqlx::query(
        "SELECT id, name, base_kind, base_value, characteristic_affinity, is_credit_rating \
         FROM skills WHERE pack_id = $1 ORDER BY sort_order, id",
    )
    .bind(pack_id)
    .fetch_all(pool)
    .await?;

    let mut skills = Vec::with_capacity(skill_rows.len());
    for row in &skill_rows {
        let base_kind: String = row.try_get("base_kind")?;
        let base_value: i32 = row.try_get("base_value")?;
        let base = match base_kind.as_str() {
            "fixed" => SkillBase::Fixed { value: base_value },
            "dexHalf" => SkillBase::DexHalf,
            "edu" => SkillBase::Edu,
            other => return Err(AppError::Content(format!("unknown skill base '{other}'"))),
        };
        let id: String = row.try_get("id")?;
        // An unrecognised affinity must not silently become `None`: affinity is what
        // `need_score` counts to choose the age-deduction target (spec § 6), so one mis-cased
        // row would move the deduction onto a different characteristic and change HP, Build,
        // MOV and Dodge with nothing logged anywhere. `base_kind` is already strict; match it.
        let affinity: Option<String> = row.try_get("characteristic_affinity")?;
        let affinity = match affinity.as_deref() {
            None => None,
            Some(key) => Some(Characteristic::from_key(key).ok_or_else(|| {
                AppError::Content(format!(
                    "skill '{id}' has an unknown characteristic affinity '{key}'"
                ))
            })?),
        };
        skills.push(Skill {
            id,
            name: row.try_get("name")?,
            base,
            affinity,
            credit_rating: row.try_get("is_credit_rating")?,
        });
    }

    let career_rows = sqlx::query(
        "SELECT id, name, blurb, formula_terms, signature_skill, supporting_skills, choice_slots, \
         credit_rating_min, credit_rating_max, bonuses, slots_from_hobbies \
         FROM careers WHERE pack_id = $1 ORDER BY sort_order, id",
    )
    .bind(pack_id)
    .fetch_all(pool)
    .await?;

    let mut careers = Vec::with_capacity(career_rows.len());
    for row in &career_rows {
        let formula_terms: serde_json::Value = row.try_get("formula_terms")?;
        let supporting: serde_json::Value = row.try_get("supporting_skills")?;
        let choice_slots: serde_json::Value = row.try_get("choice_slots")?;
        let bonuses: serde_json::Value = row.try_get("bonuses")?;
        careers.push(Career {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            blurb: row.try_get("blurb")?,
            formula_terms: serde_json::from_value::<Vec<FormulaTerm>>(formula_terms)?,
            signature_skill: row.try_get("signature_skill")?,
            supporting_skills: serde_json::from_value::<Vec<SkillSlot>>(supporting)?,
            choice_slots: serde_json::from_value::<Vec<ChoiceSlot>>(choice_slots)?,
            credit_rating: CreditRatingRange {
                min: row.try_get("credit_rating_min")?,
                max: row.try_get("credit_rating_max")?,
            },
            bonuses: serde_json::from_value::<CareerBonuses>(bonuses)?,
            slots_from_hobbies: row.try_get("slots_from_hobbies")?,
        });
    }

    let hobby_rows = sqlx::query(
        "SELECT id, name, blurb, primary_skill, secondary_skill \
         FROM hobbies WHERE pack_id = $1 ORDER BY sort_order, id",
    )
    .bind(pack_id)
    .fetch_all(pool)
    .await?;

    let mut hobbies = Vec::with_capacity(hobby_rows.len());
    for row in &hobby_rows {
        hobbies.push(Hobby {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            blurb: row.try_get("blurb")?,
            primary_skill: row.try_get("primary_skill")?,
            secondary_skill: row.try_get("secondary_skill")?,
        });
    }

    if careers.is_empty() || hobbies.is_empty() || skills.is_empty() {
        return Err(AppError::Content(format!(
            "content pack '{pack_id}' is empty - has the content sync run?"
        )));
    }

    let catalog = Catalog::new(pack_id.to_string(), skills, careers, hobbies);
    // The skill columns are plain TEXT, so the database cannot catch a dangling reference for
    // us. Refuse the pack loudly instead of serving a sheet whose ledger does not reconcile.
    let problems = crate::content::dangling_skill_references(&catalog);
    if !problems.is_empty() {
        return Err(AppError::Content(format!(
            "content pack '{pack_id}' has {} broken skill reference(s): {}",
            problems.len(),
            problems.join("; ")
        )));
    }
    Ok(catalog)
}
