//! Prototype wrapper: compiles the backend's pure rules modules (unchanged, by path) to wasm and
//! mirrors the HTTP handlers in `routes/` so a browser page can run the same wizard offline.
//! JSON in, JSON out over a tiny C ABI. See prototype/README.md.
#![allow(dead_code)]

#[path = "../../../backend/src/model.rs"]
pub mod model;
#[path = "../../../backend/src/content.rs"]
pub mod content;
#[path = "../../../backend/src/dice.rs"]
pub mod dice;
#[path = "../../../backend/src/derived.rs"]
pub mod derived;
#[path = "../../../backend/src/allocator.rs"]
pub mod allocator;
#[path = "../../../backend/src/projection.rs"]
pub mod projection;

use allocator::career_pool_for;
use content::{Career, Catalog, SkillBase, SkillSlot};
use derived::age_effects;
use model::{AgeBand, Answers, Backstory, Characteristics, DiceBlock, DiceSource, EduCheck, LuckAnswer};
use projection::project;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    static OUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    static CATALOG: Catalog = content::starter_catalog();
}

#[no_mangle]
pub extern "C" fn mem_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

#[no_mangle]
pub unsafe extern "C" fn mem_free(p: *mut u8, cap: usize) {
    drop(Vec::from_raw_parts(p, 0, cap.max(1)));
}

#[no_mangle]
pub extern "C" fn out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

/// Takes a JSON request at (p, len), returns the length of the JSON reply readable at out_ptr().
#[no_mangle]
pub unsafe extern "C" fn call(p: *const u8, len: usize) -> usize {
    let input = std::slice::from_raw_parts(p, len);
    let reply = match serde_json::from_slice::<Value>(input) {
        Ok(req) => CATALOG.with(|cat| dispatch(cat, req)),
        Err(e) => Err(ApiErr::bad(format!("malformed request: {e}"))),
    };
    let body = match reply {
        Ok(v) => json!({ "ok": v }),
        Err(e) => json!({ "error": { "code": e.code, "message": e.message, "details": e.details } }),
    };
    let bytes = serde_json::to_vec(&body).unwrap_or_default();
    let n = bytes.len();
    OUT.with(|o| *o.borrow_mut() = bytes);
    n
}

struct ApiErr {
    code: &'static str,
    message: String,
    details: Vec<String>,
}
impl ApiErr {
    fn bad(m: impl Into<String>) -> Self {
        ApiErr { code: "bad_request", message: m.into(), details: vec![] }
    }
    fn with(m: impl Into<String>, details: Vec<String>) -> Self {
        ApiErr { code: "bad_request", message: m.into(), details }
    }
    fn unprocessable(m: impl Into<String>) -> Self {
        ApiErr { code: "unprocessable", message: m.into(), details: vec![] }
    }
}
type R<T> = Result<T, ApiErr>;

fn field<T: for<'de> Deserialize<'de>>(req: &Value, key: &str) -> R<T> {
    serde_json::from_value(req.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| ApiErr::unprocessable(format!("{key}: {e}")))
}

fn dispatch(cat: &Catalog, req: Value) -> R<Value> {
    let op = req.get("op").and_then(Value::as_str).unwrap_or("");
    match op {
        "careers" => {
            let chars: Option<Characteristics> = field(&req, "characteristics")?;
            Ok(serde_json::to_value(career_cards(cat, chars)).unwrap())
        }
        "hobbies" => Ok(hobby_cards(cat)),
        "skills" => Ok(Value::Array(
            cat.skills()
                .iter()
                .map(|s| json!({ "id": s.id, "name": s.name, "baseKind": s.base.kind_key(), "baseValue": s.base.fixed_value() }))
                .collect(),
        )),
        "project" => {
            let answers: Answers = field(&req, "answers")?;
            Ok(serde_json::to_value(project(cat, &answers)).unwrap())
        }
        "patch" => {
            let answers: Answers = field(&req, "answers")?;
            let patch: AnswersPatch = field(&req, "patch")?;
            let next = apply_patch(cat, answers, patch)?;
            Ok(serde_json::to_value(next).unwrap())
        }
        "roll" => {
            let answers: Answers = field(&req, "answers")?;
            let request: RollRequest = field(&req, "request")?;
            let seed: Vec<u8> = field(&req, "seed")?;
            let seed: [u8; 32] = seed.try_into().map_err(|_| ApiErr::bad("seed must be 32 bytes"))?;
            let next = apply_roll(answers, request, StdRng::from_seed(seed))?;
            Ok(serde_json::to_value(next).unwrap())
        }
        "finalize" => {
            let answers: Answers = field(&req, "answers")?;
            let sheet = project(cat, &answers);
            if !sheet.completeness.ready_to_finalize {
                return Err(ApiErr::with("this investigator is not finished yet", sheet.completeness.missing.clone()));
            }
            Ok(serde_json::to_value(sheet).unwrap())
        }
        other => Err(ApiErr::bad(format!("unknown op '{other}'"))),
    }
}

// ---- mirrors routes/characters.rs -------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnswersPatch {
    name: Option<String>,
    pronouns: Option<String>,
    age_band: Option<AgeBand>,
    age: Option<i32>,
    career_id: Option<String>,
    career_choices: Option<BTreeMap<String, String>>,
    hobby_ids: Option<Vec<String>>,
    backstory: Option<Backstory>,
}

fn apply_patch(cat: &Catalog, mut answers: Answers, patch: AnswersPatch) -> R<Answers> {
    if let Some(name) = patch.name {
        answers.name = Some(name);
    }
    if let Some(pronouns) = patch.pronouns {
        answers.pronouns = Some(pronouns);
    }
    if let Some(age) = patch.age {
        if !(15..=89).contains(&age) {
            return Err(ApiErr::bad("age must be between 15 and 89 in this version"));
        }
        answers.age = Some(age);
        answers.age_band = Some(AgeBand::from_age(age));
    } else if let Some(band) = patch.age_band {
        answers.age_band = Some(band);
        if answers.age.map(AgeBand::from_age) != Some(band) {
            answers.age = Some(band.default_age());
        }
    }
    if let Some(career_id) = patch.career_id {
        if cat.career(&career_id).is_none() {
            return Err(ApiErr::bad(format!("unknown career '{career_id}'")));
        }
        if answers.career_id.as_deref() != Some(career_id.as_str()) {
            answers.career_choices.clear();
        }
        answers.career_id = Some(career_id);
    }
    if let Some(choices) = patch.career_choices {
        let career_id = answers
            .career_id
            .clone()
            .ok_or_else(|| ApiErr::bad("pick a career before answering its choices"))?;
        let career = cat.career(&career_id).ok_or_else(|| ApiErr::bad(format!("unknown career '{career_id}'")))?;
        for (slot_id, option_id) in &choices {
            let slot = career
                .choice_slots
                .iter()
                .find(|s| &s.id == slot_id)
                .ok_or_else(|| ApiErr::bad(format!("'{}' has no choice '{slot_id}'", career.name)))?;
            if !slot.options.iter().any(|o| &o.id == option_id) {
                return Err(ApiErr::with(
                    format!("'{option_id}' is not an option for '{slot_id}'"),
                    slot.options.iter().map(|o| o.id.clone()).collect(),
                ));
            }
        }
        answers.career_choices = choices;
    }
    if let Some(hobby_ids) = patch.hobby_ids {
        if hobby_ids.len() > 4 {
            return Err(ApiErr::bad("pick at most 4 hobbies"));
        }
        let unknown: Vec<String> = hobby_ids.iter().filter(|id| cat.hobby(id).is_none()).cloned().collect();
        if !unknown.is_empty() {
            return Err(ApiErr::with("unknown hobby", unknown));
        }
        answers.hobby_ids = hobby_ids;
    }
    if let Some(backstory) = patch.backstory {
        answers.backstory = Some(backstory);
    }
    Ok(answers)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "ceremony", rename_all = "snake_case", deny_unknown_fields)]
enum RollRequest {
    Characteristics {},
    EduChecks {},
    Luck {},
    #[serde(rename_all = "camelCase")]
    Declared {
        characteristics: Option<Characteristics>,
        edu_checks: Option<Vec<EduCheck>>,
        luck: Option<Vec<i32>>,
    },
}

fn apply_roll(mut answers: Answers, request: RollRequest, mut rng: StdRng) -> R<Answers> {
    let mut dice = answers.dice.clone().unwrap_or_default();
    match request {
        RollRequest::Characteristics {} => {
            let (chars, detail) = dice::roll_characteristics(&mut rng);
            dice.characteristics = Some(chars);
            dice.characteristic_rolls = detail;
            dice.record_source(DiceBlock::Characteristics, DiceSource::Server);
        }
        RollRequest::EduChecks {} => {
            let age = require_age(&answers)?;
            dice.edu_checks = dice::roll_edu_checks(&mut rng, age_effects(age).edu_checks);
            dice.record_source(DiceBlock::EduChecks, DiceSource::Server);
        }
        RollRequest::Luck {} => {
            let band = require_band(&answers)?;
            dice.luck = Some(dice::roll_luck(&mut rng, band));
            dice.record_source(DiceBlock::Luck, DiceSource::Server);
        }
        RollRequest::Declared { characteristics, edu_checks, luck } => {
            if characteristics.is_none() && edu_checks.is_none() && luck.is_none() {
                return Err(ApiErr::bad("declared rolls need characteristics, eduChecks or luck"));
            }
            if let Some(chars) = characteristics {
                validate(dice::validate_declared_characteristics(&chars))?;
                dice.characteristics = Some(chars);
                dice.characteristic_rolls.clear();
                dice.record_source(DiceBlock::Characteristics, DiceSource::Declared);
            }
            if let Some(checks) = edu_checks {
                let age = require_age(&answers)?;
                validate(dice::validate_declared_edu_checks(&checks, age_effects(age).edu_checks))?;
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

fn require_age(a: &Answers) -> R<i32> {
    a.effective_age().ok_or_else(|| ApiErr::bad("answer the age question before rolling"))
}
fn require_band(a: &Answers) -> R<AgeBand> {
    a.effective_band().ok_or_else(|| ApiErr::bad("answer the age question before rolling"))
}
fn validate(result: dice::DiceValidation) -> R<()> {
    result.map_err(|errs| ApiErr::with("those dice results are not legal", errs.into_iter().map(|e| e.0).collect()))
}

// ---- mirrors routes/content.rs ----------------------------------------------------------------

#[derive(Serialize)]
#[serde(untagged)]
enum SlotCard {
    Skill { id: String, name: String },
    Choice { choice: String, prompt: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CareerCard {
    id: String,
    name: String,
    blurb: String,
    formula: String,
    signature_skill: Option<SlotCard>,
    supporting_skills: Vec<SlotCard>,
    choice_slots: Vec<content::ChoiceSlot>,
    credit_rating: content::CreditRatingRange,
    bonuses: content::CareerBonuses,
    slots_from_hobbies: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    fit: Option<i32>,
}

fn formula_text(career: &Career) -> String {
    career
        .formula_terms
        .iter()
        .map(|t| {
            let name = match t.characteristic.as_characteristic() {
                Some(c) => c.key().to_uppercase(),
                None => "best other".to_string(),
            };
            format!("{name} x {}", t.multiplier)
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn name_of(cat: &Catalog, id: &str) -> String {
    cat.skill(id).map(|s| s.name.clone()).unwrap_or_else(|| id.to_string())
}

fn career_cards(cat: &Catalog, chars: Option<Characteristics>) -> Vec<CareerCard> {
    let mut cards: Vec<CareerCard> = cat
        .careers()
        .iter()
        .map(|career| CareerCard {
            id: career.id.clone(),
            name: career.name.clone(),
            blurb: career.blurb.clone(),
            formula: formula_text(career),
            signature_skill: career.signature_skill.as_ref().map(|id| SlotCard::Skill { id: id.clone(), name: name_of(cat, id) }),
            supporting_skills: career
                .supporting_skills
                .iter()
                .map(|slot| match slot {
                    SkillSlot::Fixed(id) => SlotCard::Skill { id: id.clone(), name: name_of(cat, id) },
                    SkillSlot::Choice { choice } => SlotCard::Choice {
                        choice: choice.clone(),
                        prompt: career.choice_slots.iter().find(|s| &s.id == choice).map(|s| s.prompt.clone()).unwrap_or_default(),
                    },
                })
                .collect(),
            choice_slots: career.choice_slots.clone(),
            credit_rating: career.credit_rating,
            bonuses: career.bonuses,
            slots_from_hobbies: career.slots_from_hobbies,
            fit: chars.as_ref().map(|c| career_pool_for(career, c)),
        })
        .collect();
    if chars.is_some() {
        cards.sort_by(|a, b| b.fit.cmp(&a.fit));
    }
    cards
}

fn hobby_cards(cat: &Catalog) -> Value {
    Value::Array(
        cat.hobbies()
            .iter()
            .map(|h| {
                json!({
                    "id": h.id, "name": h.name, "blurb": h.blurb,
                    "primarySkill": { "id": h.primary_skill, "name": name_of(cat, &h.primary_skill) },
                    "secondarySkill": { "id": h.secondary_skill, "name": name_of(cat, &h.secondary_skill) },
                })
            })
            .collect(),
    )
}

#[allow(unused)]
fn _unused(_: SkillBase) {}
