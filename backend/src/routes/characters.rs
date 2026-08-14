//! The signup wizard's endpoints. Answers go in; the sheet is projected on the way out.

use crate::db::{self, CharacterRecord};
use crate::derived::age_effects;
use crate::dice;
use crate::error::{AppError, AppResult};
use crate::extract::{AppJson, AppPath};
use crate::model::{
    AgeBand, Answers, Backstory, Characteristics, DiceBlock, DiceSource, EduCheck, LuckAnswer,
    Status,
};
use crate::projection::{project, Sheet};
use crate::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterResponse {
    pub id: Uuid,
    /// TODO(auth): always null until accounts exist.
    pub owner_id: Option<Uuid>,
    pub status: Status,
    pub answers: Answers,
    /// Projected live for drafts; the frozen snapshot once finalized.
    pub sheet: Sheet,
    pub sheet_is_snapshot: bool,
    pub content_pack_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finalized_at: Option<DateTime<Utc>>,
}

fn respond(state: &AppState, record: CharacterRecord) -> CharacterResponse {
    let (sheet, sheet_is_snapshot) = match (record.status, record.sheet.clone()) {
        (Status::Finalized, Some(snapshot)) => (snapshot, true),
        _ => (project(&state.catalog, &record.answers), false),
    };
    CharacterResponse {
        id: record.id,
        owner_id: record.owner_id,
        status: record.status,
        answers: record.answers,
        sheet,
        sheet_is_snapshot,
        content_pack_id: record.content_pack_id,
        created_at: record.created_at,
        updated_at: record.updated_at,
        finalized_at: record.finalized_at,
    }
}

/// `POST /characters` - a brand new empty draft.
pub async fn create_character(
    State(state): State<AppState>,
) -> AppResult<Json<CharacterResponse>> {
    let record = db::create_character(&state.pool, &state.catalog.pack_id).await?;
    Ok(Json(respond(&state, record)))
}

/// `GET /characters/:id` - answers plus the projected sheet.
pub async fn get_character(
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
) -> AppResult<Json<CharacterResponse>> {
    let record = db::get_character(&state.pool, id).await?;
    Ok(Json(respond(&state, record)))
}

/// A partial answer update. Absent fields are left alone; dice go through `/roll`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswersPatch {
    pub name: Option<String>,
    pub pronouns: Option<String>,
    pub age_band: Option<AgeBand>,
    pub age: Option<i32>,
    pub career_id: Option<String>,
    pub career_choices: Option<BTreeMap<String, String>>,
    pub hobby_ids: Option<Vec<String>>,
    pub backstory: Option<Backstory>,
}

/// `PATCH /characters/:id` - partial update of the answers. Rejected once finalized.
///
/// No recomputation is triggered: the sheet is computed on read, so the next `GET` already
/// reflects the change (spec § 2, "the sheet is a live projection").
pub async fn patch_character(
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(patch): AppJson<AnswersPatch>,
) -> AppResult<Json<CharacterResponse>> {
    // Read and write inside one transaction: `update_answers` rewrites the whole answers
    // document, so an unlocked read-modify-write drops a concurrent field autosave.
    let record = db::mutate_answers(&state.pool, id, |record| {
        guard_draft(record)?;
        apply_patch(&state, record, patch)
    })
    .await?;
    Ok(Json(respond(&state, record)))
}

/// Fold a patch into the stored answers, validating every reference against the content pack.
/// Pure: it is called inside the transaction that holds the row lock.
fn apply_patch(
    state: &AppState,
    record: &CharacterRecord,
    patch: AnswersPatch,
) -> AppResult<Answers> {
    let mut answers = record.answers.clone();

    if let Some(name) = patch.name {
        answers.name = Some(name);
    }
    if let Some(pronouns) = patch.pronouns {
        answers.pronouns = Some(pronouns);
    }
    if let Some(age) = patch.age {
        if !(15..=89).contains(&age) {
            return Err(AppError::bad_request(
                "age must be between 15 and 89 in this version",
            ));
        }
        answers.age = Some(age);
        answers.age_band = Some(AgeBand::from_age(age));
    } else if let Some(band) = patch.age_band {
        answers.age_band = Some(band);
        // A band without an exact age uses the band's representative age.
        if answers.age.map(AgeBand::from_age) != Some(band) {
            answers.age = Some(band.default_age());
        }
    }

    if let Some(career_id) = patch.career_id {
        if state.catalog.career(&career_id).is_none() {
            return Err(AppError::bad_request(format!(
                "unknown career '{career_id}'"
            )));
        }
        // Choices belong to the career that asked the question.
        if answers.career_id.as_deref() != Some(career_id.as_str()) {
            answers.career_choices.clear();
        }
        answers.career_id = Some(career_id);
    }

    if let Some(choices) = patch.career_choices {
        let career_id = answers
            .career_id
            .as_deref()
            .ok_or_else(|| AppError::bad_request("pick a career before answering its choices"))?;
        let career = state
            .catalog
            .career(career_id)
            .ok_or_else(|| AppError::bad_request(format!("unknown career '{career_id}'")))?;
        for (slot_id, option_id) in &choices {
            let slot = career
                .choice_slots
                .iter()
                .find(|s| &s.id == slot_id)
                .ok_or_else(|| {
                    AppError::bad_request(format!("'{}' has no choice '{slot_id}'", career.name))
                })?;
            if !slot.options.iter().any(|o| &o.id == option_id) {
                return Err(AppError::with_details(
                    format!("'{option_id}' is not an option for '{slot_id}'"),
                    slot.options.iter().map(|o| o.id.clone()).collect(),
                ));
            }
        }
        answers.career_choices = choices;
    }

    if let Some(hobby_ids) = patch.hobby_ids {
        if hobby_ids.len() > 4 {
            return Err(AppError::bad_request("pick at most 4 hobbies"));
        }
        let unknown: Vec<String> = hobby_ids
            .iter()
            .filter(|id| state.catalog.hobby(id).is_none())
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return Err(AppError::with_details("unknown hobby", unknown));
        }
        answers.hobby_ids = hobby_ids;
    }

    if let Some(backstory) = patch.backstory {
        answers.backstory = Some(backstory);
    }

    Ok(answers)
}

/// Which dice ceremony to run (spec § 3). `declared` carries hand-rolled results instead.
///
/// Unknown fields are refused exactly as they are on `PATCH`: a misspelled `educhecks` that
/// deserialized to `None` would answer 200 while silently discarding a physical-dice player's
/// hand-rolled results, which they would only discover at finalize.
#[derive(Debug, Deserialize)]
#[serde(tag = "ceremony", rename_all = "snake_case", deny_unknown_fields)]
pub enum RollRequest {
    /// 3a - the eight characteristics. (Empty-braced so `deny_unknown_fields` applies: a unit
    /// variant of an internally-tagged enum silently swallows whatever else is in the object.)
    Characteristics {},
    /// 3b - the EDU life-experience checks the age band grants.
    EduChecks {},
    /// 3c - Luck (twice, keeping the better, for Young characters).
    Luck {},
    /// "I rolled at home": any subset of the three, validated.
    #[serde(rename_all = "camelCase")]
    Declared {
        characteristics: Option<Characteristics>,
        edu_checks: Option<Vec<EduCheck>>,
        luck: Option<Vec<i32>>,
    },
}

/// `POST /characters/:id/roll` - server-side ceremonies, or validated declared results.
///
/// Unlimited rerolls while drafting (spec § 3).
pub async fn roll(
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
    AppJson(request): AppJson<RollRequest>,
) -> AppResult<Json<CharacterResponse>> {
    let record = db::mutate_answers(&state.pool, id, |record| {
        guard_draft(record)?;
        apply_roll(record, request)
    })
    .await?;
    Ok(Json(respond(&state, record)))
}

/// Run one ceremony (or record one set of declared results) against the stored answers.
fn apply_roll(record: &CharacterRecord, request: RollRequest) -> AppResult<Answers> {
    let mut answers = record.answers.clone();
    let mut dice = answers.dice.clone().unwrap_or_default();
    // `StdRng` (unlike `ThreadRng`) is `Send`, which the handler future needs. Seeded from the
    // OS: the ceremony is honest, and the result is recorded before any animation replays it.
    let mut rng = StdRng::from_entropy();

    match request {
        RollRequest::Characteristics {} => {
            let (chars, detail) = dice::roll_characteristics(&mut rng);
            dice.characteristics = Some(chars);
            dice.characteristic_rolls = detail;
            dice.record_source(DiceBlock::Characteristics, DiceSource::Server);
        }
        RollRequest::EduChecks {} => {
            let age = require_age(&answers)?;
            let count = age_effects(age).edu_checks;
            dice.edu_checks = dice::roll_edu_checks(&mut rng, count);
            dice.record_source(DiceBlock::EduChecks, DiceSource::Server);
        }
        RollRequest::Luck {} => {
            let band = require_band(&answers)?;
            dice.luck = Some(dice::roll_luck(&mut rng, band));
            dice.record_source(DiceBlock::Luck, DiceSource::Server);
        }
        RollRequest::Declared {
            characteristics,
            edu_checks,
            luck,
        } => {
            if characteristics.is_none() && edu_checks.is_none() && luck.is_none() {
                return Err(AppError::bad_request(
                    "declared rolls need characteristics, eduChecks or luck",
                ));
            }
            // Provenance is recorded per block: a declared characteristics set followed by a
            // server Luck ceremony (or the reverse) must not restamp the block it never touched.
            if let Some(chars) = characteristics {
                validate(dice::validate_declared_characteristics(&chars))?;
                dice.characteristics = Some(chars);
                dice.characteristic_rolls.clear();
                dice.record_source(DiceBlock::Characteristics, DiceSource::Declared);
            }
            if let Some(checks) = edu_checks {
                let age = require_age(&answers)?;
                validate(dice::validate_declared_edu_checks(
                    &checks,
                    age_effects(age).edu_checks,
                ))?;
                dice.edu_checks = checks;
                dice.record_source(DiceBlock::EduChecks, DiceSource::Declared);
            }
            if let Some(rolls) = luck {
                let band = require_band(&answers)?;
                validate(dice::validate_declared_luck(&rolls, band))?;
                let kept = dice::kept_luck(&rolls, band);
                dice.luck = Some(LuckAnswer { rolls, kept });
                dice.record_source(DiceBlock::Luck, DiceSource::Declared);
            }
        }
    }

    answers.dice = Some(dice);
    Ok(answers)
}

/// `POST /characters/:id/finalize` - validate, snapshot the sheet, freeze the answers.
pub async fn finalize(
    State(state): State<AppState>,
    AppPath(id): AppPath<Uuid>,
) -> AppResult<Json<CharacterResponse>> {
    // Read, project and freeze inside one transaction, with the row locked for the duration.
    // The frozen sheet has to be the projection of the frozen answers (spec § 9) and the record
    // is immutable afterwards, so a PATCH landing between the read and the UPDATE would corrupt
    // it permanently — an in-flight autosave plus a Finalize tap is enough to trigger it.
    let record = db::finalize_atomically(&state.pool, id, |record| {
        guard_draft(record)?;

        // The projection carries the Jack of All Trades +10 Luck perk and every allocation rule,
        // so the snapshot is simply the sheet the player has been looking at.
        let sheet = project(&state.catalog, &record.answers);
        if !sheet.completeness.ready_to_finalize {
            return Err(AppError::with_details(
                "this investigator is not finished yet",
                sheet.completeness.missing.clone(),
            ));
        }
        Ok(sheet)
    })
    .await?;
    Ok(Json(respond(&state, record)))
}

fn guard_draft(record: &CharacterRecord) -> AppResult<()> {
    if record.status == Status::Finalized {
        return Err(AppError::Conflict(format!(
            "character {} is finalized; answers are frozen",
            record.id
        )));
    }
    Ok(())
}

fn require_age(answers: &Answers) -> AppResult<i32> {
    answers
        .effective_age()
        .ok_or_else(|| AppError::bad_request("answer the age question before rolling"))
}

fn require_band(answers: &Answers) -> AppResult<AgeBand> {
    answers
        .effective_band()
        .ok_or_else(|| AppError::bad_request("answer the age question before rolling"))
}

fn validate(result: dice::DiceValidation) -> AppResult<()> {
    result.map_err(|errors| {
        AppError::with_details(
            "those dice results are not legal",
            errors.into_iter().map(|e| e.0).collect(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Characteristics, DiceAnswers, DiceSources};

    fn draft_record(answers: Answers) -> CharacterRecord {
        let now = Utc::now();
        CharacterRecord {
            id: Uuid::nil(),
            owner_id: None,
            status: Status::Draft,
            answers,
            sheet: None,
            content_pack_id: "starter-modern".to_string(),
            created_at: now,
            updated_at: now,
            finalized_at: None,
        }
    }

    fn legal_characteristics() -> Characteristics {
        Characteristics {
            strength: 50,
            con: 60,
            siz: 60,
            dex: 55,
            app: 50,
            pow: 65,
            intelligence: 70,
            edu: 75,
        }
    }

    fn prime_draft() -> CharacterRecord {
        draft_record(Answers {
            age: Some(27),
            age_band: Some(AgeBand::Prime),
            ..Default::default()
        })
    }

    fn sources(answers: &Answers) -> DiceSources {
        answers.dice.as_ref().unwrap().sources
    }

    #[test]
    fn a_server_ceremony_after_declared_dice_does_not_relabel_the_hand_entered_block() {
        // "I rolled my stats at home", then tap the server Luck ceremony for the animation.
        let answers = apply_roll(
            &prime_draft(),
            RollRequest::Declared {
                characteristics: Some(legal_characteristics()),
                edu_checks: None,
                luck: None,
            },
        )
        .unwrap();
        assert_eq!(sources(&answers).characteristics, Some(DiceSource::Declared));

        let answers = apply_roll(&draft_record(answers), RollRequest::Luck {}).unwrap();
        assert_eq!(
            sources(&answers).characteristics,
            Some(DiceSource::Declared),
            "the eight hand-entered stats must not be restamped as server-rolled"
        );
        assert_eq!(sources(&answers).luck, Some(DiceSource::Server));
        assert_eq!(answers.dice.as_ref().unwrap().source, DiceSource::Declared);
    }

    #[test]
    fn declaring_only_luck_does_not_relabel_server_rolled_characteristics() {
        let answers = apply_roll(&prime_draft(), RollRequest::Characteristics {}).unwrap();
        assert_eq!(sources(&answers).characteristics, Some(DiceSource::Server));
        assert_eq!(answers.dice.as_ref().unwrap().source, DiceSource::Server);

        let answers = apply_roll(
            &draft_record(answers),
            RollRequest::Declared {
                characteristics: None,
                edu_checks: None,
                luck: Some(vec![55]),
            },
        )
        .unwrap();
        assert_eq!(
            sources(&answers).characteristics,
            Some(DiceSource::Server),
            "eight server-rolled stats must not be relabelled as self-reported"
        );
        assert_eq!(sources(&answers).luck, Some(DiceSource::Declared));
    }

    #[test]
    fn an_untouched_block_carries_no_provenance_claim() {
        let answers = apply_roll(&prime_draft(), RollRequest::Luck {}).unwrap();
        let sources = sources(&answers);
        assert_eq!(sources.luck, Some(DiceSource::Server));
        assert_eq!(sources.characteristics, None);
        assert_eq!(sources.edu_checks, None);
    }

    #[test]
    fn a_declared_edu_check_with_a_zero_gain_is_rejected() {
        let error = apply_roll(
            &prime_draft(),
            RollRequest::Declared {
                characteristics: None,
                edu_checks: Some(vec![EduCheck { d100: 99, gain: 0 }]),
                luck: None,
            },
        )
        .unwrap_err();
        assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn a_dice_answers_default_claims_nothing() {
        assert!(DiceAnswers::default().sources.is_empty());
    }

    #[test]
    fn a_misspelled_declared_field_is_refused_rather_than_silently_dropped() {
        // The exact typo a hand-rolling player makes: snake_case instead of camelCase.
        let body = r#"{"ceremony":"declared","luck":[55],"educhecks":[{"d100":71,"gain":7}]}"#;
        let error = serde_json::from_str::<RollRequest>(body).unwrap_err();
        assert!(
            error.to_string().contains("educhecks"),
            "the error must name the field: {error}"
        );

        // ...and the correctly-spelled body still parses.
        let body = r#"{"ceremony":"declared","luck":[55],"eduChecks":[{"d100":71,"gain":7}]}"#;
        let request: RollRequest = serde_json::from_str(body).unwrap();
        match request {
            RollRequest::Declared { edu_checks, luck, .. } => {
                assert_eq!(edu_checks.unwrap().len(), 1);
                assert_eq!(luck.unwrap(), vec![55]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn stray_fields_on_a_server_ceremony_are_refused_too() {
        assert!(serde_json::from_str::<RollRequest>(r#"{"ceremony":"luck"}"#).is_ok());
        assert!(serde_json::from_str::<RollRequest>(r#"{"ceremony":"characteristics"}"#).is_ok());
        assert!(serde_json::from_str::<RollRequest>(r#"{"ceremony":"edu_checks"}"#).is_ok());
        assert!(
            serde_json::from_str::<RollRequest>(r#"{"ceremony":"luck","rolls":[55]}"#).is_err(),
            "a server ceremony carries no payload; a stray one means the client is confused"
        );
    }

    #[test]
    fn an_unknown_ceremony_is_refused() {
        assert!(serde_json::from_str::<RollRequest>(r#"{"ceremony":"vibes"}"#).is_err());
    }

    #[test]
    fn a_patch_with_an_unknown_field_is_refused() {
        assert!(serde_json::from_str::<AnswersPatch>(r#"{"name":"Ada"}"#).is_ok());
        assert!(serde_json::from_str::<AnswersPatch>(r#"{"nmae":"Ada"}"#).is_err());
    }
}
