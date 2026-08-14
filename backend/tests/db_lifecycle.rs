//! Database-backed lifecycle test: migrations, the seeded content pack, the answers round-trip
//! and the draft -> finalized freeze.
//!
//! Skipped unless `DATABASE_URL` is set, so `cargo test` stays green without infrastructure:
//!
//! ```text
//! docker compose up -d
//! DATABASE_URL=postgres://postgres:postgres@localhost:5432/coc_phoenix cargo test
//! ```

use coc_phoenix::content::starter_pack_file;
use coc_phoenix::db;
use coc_phoenix::model::Status;
use coc_phoenix::model::{
    AgeBand, Answers, Characteristics, DiceAnswers, DiceSource, EduCheck, LuckAnswer,
};
use coc_phoenix::projection::project;
use sqlx::PgPool;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|v| !v.is_empty())
}

/// Connect, migrate and publish the bundled content, exactly as `main.rs` does on boot.
async fn boot() -> Option<PgPool> {
    let url = database_url()?;
    let pool = db::connect(&url, 8).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    db::sync_content_pack(&pool, &starter_pack_file())
        .await
        .expect("publish content");
    Some(pool)
}

fn finished_answers() -> Answers {
    Answers {
        name: Some("Integration Fixture".to_string()),
        pronouns: Some("they/them".to_string()),
        age: Some(48),
        age_band: Some(AgeBand::Seasoned),
        dice: Some(DiceAnswers {
            source: DiceSource::Declared,
            characteristics: Some(Characteristics {
                strength: 55,
                con: 60,
                siz: 65,
                dex: 50,
                app: 55,
                pow: 70,
                intelligence: 75,
                edu: 80,
            }),
            edu_checks: vec![
                EduCheck { d100: 95, gain: 5 },
                EduCheck { d100: 40, gain: 9 },
            ],
            luck: Some(LuckAnswer {
                rolls: vec![55],
                kept: Some(55),
            }),
            ..Default::default()
        }),
        career_id: Some("antiquarian".to_string()),
        hobby_ids: vec![
            "bookworm".to_string(),
            "people-watching".to_string(),
            "occult-curiosity".to_string(),
            "urban-exploration".to_string(),
        ],
        ..Default::default()
    }
}

#[tokio::test]
async fn migrations_seed_the_starter_pack_and_a_character_can_be_drafted_then_frozen() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL not set - skipping the database lifecycle test");
        return;
    };

    let pool = db::connect(&url, 2).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    db::sync_content_pack(&pool, &starter_pack_file())
        .await
        .expect("publish content");

    // The content sync produced the full spec roster.
    let catalog = db::load_catalog(&pool, db::DEFAULT_PACK_ID)
        .await
        .expect("load catalog");
    assert_eq!(catalog.careers().len(), 17);
    assert_eq!(catalog.hobbies().len(), 17);
    assert!(catalog.skills().len() >= 50);
    let jack = catalog.career("jack-of-all-trades").expect("jack");
    assert_eq!(jack.bonuses.luck, Some(10));
    assert!(jack.slots_from_hobbies);
    assert_eq!(
        catalog.career("professor").unwrap().choice_slots[0].options.len(),
        4
    );

    // Create -> patch -> finalize.
    let created = db::create_character(&pool, db::DEFAULT_PACK_ID)
        .await
        .expect("create");
    assert_eq!(created.status, Status::Draft);
    assert!(created.sheet.is_none(), "drafts never store a sheet");

    let answers = finished_answers();
    let updated = db::update_answers(&pool, created.id, &answers)
        .await
        .expect("update");
    assert_eq!(updated.answers, answers, "answers round-trip through JSONB");
    assert!(updated.sheet.is_none());

    let sheet = project(&catalog, &updated.answers);
    assert!(
        sheet.completeness.ready_to_finalize,
        "{:?}",
        sheet.completeness.missing
    );

    let finalized = db::finalize_character(&pool, created.id, &sheet)
        .await
        .expect("finalize");
    assert_eq!(finalized.status, Status::Finalized);
    assert_eq!(finalized.sheet.as_ref(), Some(&sheet), "snapshot is verbatim");
    assert!(finalized.finalized_at.is_some());

    // Re-reading gives the identical frozen sheet.
    let reread = db::get_character(&pool, created.id).await.expect("get");
    assert_eq!(reread.sheet.as_ref(), Some(&sheet));

    // Finalized answers are frozen.
    let rejected = db::update_answers(&pool, created.id, &Answers::default()).await;
    assert!(rejected.is_err(), "a finalized character must reject edits");
    let refinalize = db::finalize_character(&pool, created.id, &sheet).await;
    assert!(refinalize.is_err(), "finalize is not idempotent by design");

    sqlx::query("DELETE FROM characters WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// The authoring loop that used to be impossible: edit `content/starter_pack.json`, restart, and
/// the change is live. The seed used to be a checksummed migration whose inserts ended
/// `ON CONFLICT DO NOTHING`, so an edit either bricked the boot or silently never landed.
#[tokio::test]
async fn publishing_content_updates_rows_that_were_already_seeded() {
    let Some(pool) = boot().await else {
        eprintln!("DATABASE_URL not set - skipping the content publish test");
        return;
    };

    // The accented display name only ever reaches an existing database through an upsert.
    let catalog = db::load_catalog(&pool, db::DEFAULT_PACK_ID)
        .await
        .expect("load catalog");
    assert_eq!(
        catalog.career("seance-medium").expect("seance-medium").name,
        "Séance Medium",
        "a content edit must reach a database that already ran the original seed"
    );

    // Scribble over a row the way a stale seed would have left it, then republish.
    sqlx::query("UPDATE careers SET name = 'Stale Name', credit_rating_min = 1 WHERE pack_id = $1 AND id = 'seance-medium'")
        .bind(db::DEFAULT_PACK_ID)
        .execute(&pool)
        .await
        .expect("scribble");
    db::sync_content_pack(&pool, &starter_pack_file())
        .await
        .expect("republish");

    let catalog = db::load_catalog(&pool, db::DEFAULT_PACK_ID)
        .await
        .expect("reload catalog");
    let medium = catalog.career("seance-medium").expect("seance-medium");
    assert_eq!(medium.name, "Séance Medium");
    assert_eq!(medium.credit_rating.min, 9, "every column is refreshed");

    // ...and republishing is safe to do on every boot.
    for _ in 0..3 {
        db::sync_content_pack(&pool, &starter_pack_file())
            .await
            .expect("idempotent");
    }
}

/// `update_answers` rewrites the whole answers document, so overlapping edits must serialise or
/// one of them is silently discarded (the wizard autosaves while the player keeps answering).
#[tokio::test]
async fn concurrent_answer_edits_do_not_lose_writes() {
    let Some(pool) = boot().await else {
        eprintln!("DATABASE_URL not set - skipping the concurrency test");
        return;
    };

    let created = db::create_character(&pool, db::DEFAULT_PACK_ID)
        .await
        .expect("create");

    let picks = ["bookworm", "poker-nights", "occult-curiosity", "tinkering"];
    let mut writers = Vec::new();
    for hobby in picks {
        let pool = pool.clone();
        writers.push(tokio::spawn(async move {
            db::mutate_answers(&pool, created.id, move |record| {
                let mut answers = record.answers.clone();
                answers.hobby_ids.push(hobby.to_string());
                Ok(answers)
            })
            .await
        }));
    }
    for writer in writers {
        writer.await.expect("join").expect("write");
    }

    let record = db::get_character(&pool, created.id).await.expect("get");
    let mut got = record.answers.hobby_ids.clone();
    got.sort();
    let mut want: Vec<String> = picks.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(got, want, "every concurrent edit must survive");

    sqlx::query("DELETE FROM characters WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// Spec section 9: the frozen sheet IS the projection of the frozen answers. The record is
/// immutable afterwards, so a write landing between the read and the freeze would corrupt it
/// permanently — an in-flight autosave plus a Finalize tap is enough.
#[tokio::test]
async fn a_finalized_sheet_always_matches_the_answers_it_was_frozen_with() {
    let Some(pool) = boot().await else {
        eprintln!("DATABASE_URL not set - skipping the finalize race test");
        return;
    };
    let catalog = db::load_catalog(&pool, db::DEFAULT_PACK_ID)
        .await
        .expect("load catalog");

    for round in 0..8 {
        let created = db::create_character(&pool, db::DEFAULT_PACK_ID)
            .await
            .expect("create");
        db::update_answers(&pool, created.id, &finished_answers())
            .await
            .expect("seed answers");

        // A debounced autosave still in flight while the player taps Finalize.
        let writer_pool = pool.clone();
        let id = created.id;
        let writer = tokio::spawn(async move {
            for i in 0..40 {
                let _ = db::mutate_answers(&writer_pool, id, move |record| {
                    let mut answers = record.answers.clone();
                    answers.name = Some(format!("Wren Ashcombe-Lyle {i}"));
                    Ok(answers)
                })
                .await;
            }
        });

        let finalized = db::finalize_atomically(&pool, id, |record| {
            Ok(project(&catalog, &record.answers))
        })
        .await
        .expect("finalize");
        writer.await.expect("join");

        let record = db::get_character(&pool, id).await.expect("get");
        assert_eq!(record.status, Status::Finalized);
        let snapshot = record.sheet.as_ref().expect("a finalized row carries a sheet");
        assert_eq!(
            snapshot,
            &project(&catalog, &record.answers),
            "round {round}: the frozen sheet must be the projection of the frozen answers"
        );
        assert_eq!(snapshot.name, record.answers.name, "round {round}");
        assert_eq!(finalized.sheet.as_ref(), Some(snapshot), "round {round}");

        sqlx::query("DELETE FROM characters WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .expect("cleanup");
    }
}
