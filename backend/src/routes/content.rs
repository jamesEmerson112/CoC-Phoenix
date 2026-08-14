//! Wizard content: the career and hobby rosters the picker screens render.

use crate::allocator::career_pool_for;
use crate::content::{Career, SkillBase, SkillSlot};
use crate::error::AppResult;
use crate::model::Characteristics;
use crate::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

/// Optional rolled characteristics, so the roster can be sorted by fit (spec § 2 step 4).
#[derive(Debug, Default, Deserialize)]
pub struct FitQuery {
    #[serde(rename = "str")]
    pub strength: Option<i32>,
    pub con: Option<i32>,
    pub siz: Option<i32>,
    pub dex: Option<i32>,
    pub app: Option<i32>,
    pub pow: Option<i32>,
    #[serde(rename = "int")]
    pub intelligence: Option<i32>,
    pub edu: Option<i32>,
}

impl FitQuery {
    fn characteristics(&self) -> Option<Characteristics> {
        let any = [
            self.strength,
            self.con,
            self.siz,
            self.dex,
            self.app,
            self.pow,
            self.intelligence,
            self.edu,
        ]
        .iter()
        .any(Option::is_some);
        if !any {
            return None;
        }
        Some(Characteristics {
            strength: self.strength.unwrap_or(0),
            con: self.con.unwrap_or(0),
            siz: self.siz.unwrap_or(0),
            dex: self.dex.unwrap_or(0),
            app: self.app.unwrap_or(0),
            pow: self.pow.unwrap_or(0),
            intelligence: self.intelligence.unwrap_or(0),
            edu: self.edu.unwrap_or(0),
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SlotCard {
    Skill { id: String, name: String },
    Choice { choice: String, prompt: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerCard {
    pub id: String,
    pub name: String,
    pub blurb: String,
    /// Human-readable formula, e.g. "EDU x 4".
    pub formula: String,
    pub formula_terms: Vec<crate::content::FormulaTerm>,
    pub signature_skill: Option<SlotCard>,
    pub supporting_skills: Vec<SlotCard>,
    pub choice_slots: Vec<crate::content::ChoiceSlot>,
    pub credit_rating: crate::content::CreditRatingRange,
    pub bonuses: crate::content::CareerBonuses,
    pub slots_from_hobbies: bool,
    /// The career pool these characteristics would produce; present only when the query
    /// carried rolled characteristics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<i32>,
}

fn formula_text(career: &Career) -> String {
    career
        .formula_terms
        .iter()
        .map(|term| {
            let name = match term.characteristic.as_characteristic() {
                Some(c) => c.key().to_uppercase(),
                None => "best other".to_string(),
            };
            format!("{name} x {}", term.multiplier)
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// `GET /content/careers` - optionally `?str=..&con=..&..` to sort best-payout-first.
pub async fn list_careers(
    State(state): State<AppState>,
    Query(query): Query<FitQuery>,
) -> AppResult<Json<Vec<CareerCard>>> {
    let chars = query.characteristics();
    let catalog = &state.catalog;
    let name_of = |id: &str| {
        catalog
            .skill(id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| id.to_string())
    };

    let mut cards: Vec<CareerCard> = catalog
        .careers()
        .iter()
        .map(|career| CareerCard {
            id: career.id.clone(),
            name: career.name.clone(),
            blurb: career.blurb.clone(),
            formula: formula_text(career),
            formula_terms: career.formula_terms.clone(),
            signature_skill: career.signature_skill.as_ref().map(|id| SlotCard::Skill {
                id: id.clone(),
                name: name_of(id),
            }),
            supporting_skills: career
                .supporting_skills
                .iter()
                .map(|slot| match slot {
                    SkillSlot::Fixed(id) => SlotCard::Skill {
                        id: id.clone(),
                        name: name_of(id),
                    },
                    SkillSlot::Choice { choice } => SlotCard::Choice {
                        choice: choice.clone(),
                        prompt: career
                            .choice_slots
                            .iter()
                            .find(|s| &s.id == choice)
                            .map(|s| s.prompt.clone())
                            .unwrap_or_default(),
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
        // Best payout first; ties keep roster order (sort_by is stable).
        cards.sort_by(|a, b| b.fit.cmp(&a.fit));
    }
    Ok(Json(cards))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HobbyCard {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub primary_skill: SlotCard,
    pub secondary_skill: SlotCard,
}

/// `GET /content/hobbies` - the 17 hobbies, pick 4 in priority order.
pub async fn list_hobbies(State(state): State<AppState>) -> AppResult<Json<Vec<HobbyCard>>> {
    let catalog = &state.catalog;
    let name_of = |id: &str| {
        catalog
            .skill(id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let cards = catalog
        .hobbies()
        .iter()
        .map(|hobby| HobbyCard {
            id: hobby.id.clone(),
            name: hobby.name.clone(),
            blurb: hobby.blurb.clone(),
            primary_skill: SlotCard::Skill {
                id: hobby.primary_skill.clone(),
                name: name_of(&hobby.primary_skill),
            },
            secondary_skill: SlotCard::Skill {
                id: hobby.secondary_skill.clone(),
                name: name_of(&hobby.secondary_skill),
            },
        })
        .collect();
    Ok(Json(cards))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCard {
    pub id: String,
    pub name: String,
    /// "fixed", "dexHalf" or "edu".
    pub base_kind: String,
    pub base_value: i32,
    pub credit_rating: bool,
}

/// `GET /content/skills` - the skill catalogue, for rendering skill names client-side.
pub async fn list_skills(State(state): State<AppState>) -> AppResult<Json<Vec<SkillCard>>> {
    let cards = state
        .catalog
        .skills()
        .iter()
        .map(|skill| SkillCard {
            id: skill.id.clone(),
            name: skill.name.clone(),
            base_kind: skill.base.kind_key().to_string(),
            base_value: match skill.base {
                SkillBase::Fixed { value } => value,
                _ => 0,
            },
            credit_rating: skill.credit_rating,
        })
        .collect();
    Ok(Json(cards))
}
