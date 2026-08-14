//! Content-pack types (spec § 9, "Content structure").
//!
//! Careers, hobbies and skills are *data*, seeded into Postgres by a migration and loaded into
//! this pure in-memory [`Catalog`] at startup. Nothing in this module touches the database, so
//! the allocator and its tests can build a catalogue from invented fixtures.

use crate::model::Characteristic;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How a skill's base value is produced (spec § 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SkillBase {
    /// A plain starting percentage.
    Fixed { value: i32 },
    /// Dodge: `DEX // 2`.
    DexHalf,
    /// Language (Own): `= EDU`.
    Edu,
}

impl SkillBase {
    pub fn kind_key(self) -> &'static str {
        match self {
            SkillBase::Fixed { .. } => "fixed",
            SkillBase::DexHalf => "dexHalf",
            SkillBase::Edu => "edu",
        }
    }

    pub fn fixed_value(self) -> i32 {
        match self {
            SkillBase::Fixed { value } => value,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub base: SkillBase,
    /// Which physical characteristic this skill leans on. Used by the age-deduction allocator
    /// to decide which characteristic a career "needs least" (spec § 6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affinity: Option<Characteristic>,
    /// Exactly one skill is the Credit Rating track.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub credit_rating: bool,
}

/// One term of a career's points formula, e.g. `EDU x 4` or Jack of All Trades' `best-other x 2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaTerm {
    pub characteristic: FormulaCharacteristic,
    pub multiplier: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FormulaCharacteristic {
    #[serde(rename = "str")]
    Str,
    #[serde(rename = "con")]
    Con,
    #[serde(rename = "siz")]
    Siz,
    #[serde(rename = "dex")]
    Dex,
    #[serde(rename = "app")]
    App,
    #[serde(rename = "pow")]
    Pow,
    #[serde(rename = "int")]
    Int,
    #[serde(rename = "edu")]
    Edu,
    /// Spec § 4.1: the highest characteristic other than EDU, resolved by the allocator.
    BestOther,
}

impl FormulaCharacteristic {
    pub fn as_characteristic(self) -> Option<Characteristic> {
        Some(match self {
            FormulaCharacteristic::Str => Characteristic::Str,
            FormulaCharacteristic::Con => Characteristic::Con,
            FormulaCharacteristic::Siz => Characteristic::Siz,
            FormulaCharacteristic::Dex => Characteristic::Dex,
            FormulaCharacteristic::App => Characteristic::App,
            FormulaCharacteristic::Pow => Characteristic::Pow,
            FormulaCharacteristic::Int => Characteristic::Int,
            FormulaCharacteristic::Edu => Characteristic::Edu,
            FormulaCharacteristic::BestOther => return None,
        })
    }
}

/// A career skill slot: either a named skill or a slot the player answers on selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum SkillSlot {
    Fixed(String),
    Choice { choice: String },
}

impl SkillSlot {
    /// Resolve to a concrete skill id given the player's answers to the career's choice slots.
    pub fn resolve(&self, career: &Career, choices: &BTreeMap<String, String>) -> Option<String> {
        match self {
            SkillSlot::Fixed(id) => Some(id.clone()),
            SkillSlot::Choice { choice } => {
                let slot = career.choice_slots.iter().find(|s| &s.id == choice)?;
                let answer = choices.get(choice)?;
                slot.options
                    .iter()
                    .find(|o| &o.id == answer)
                    .map(|o| o.skill.clone())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
    pub skill: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceSlot {
    pub id: String,
    pub prompt: String,
    pub options: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditRatingRange {
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerBonuses {
    /// Jack of All Trades' house-rule perk (spec § 4.1): +10 starting Luck, capped at 99.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub luck: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Career {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub formula_terms: Vec<FormulaTerm>,
    /// `None` for Jack of All Trades, which has no signature skill.
    #[serde(default)]
    pub signature_skill: Option<String>,
    pub supporting_skills: Vec<SkillSlot>,
    #[serde(default)]
    pub choice_slots: Vec<ChoiceSlot>,
    pub credit_rating: CreditRatingRange,
    #[serde(default)]
    pub bonuses: CareerBonuses,
    /// Spec § 4.1: the rest of the career pool flows into the player's own hobby picks.
    #[serde(default)]
    pub slots_from_hobbies: bool,
}

impl Career {
    /// Number of ladder slots: signature (if any) + supporting skills.
    pub fn slot_count(&self) -> usize {
        usize::from(self.signature_skill.is_some()) + self.supporting_skills.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hobby {
    pub id: String,
    pub name: String,
    pub blurb: String,
    /// Takes ~70% of the hobby's points (spec § 5/§ 6).
    pub primary_skill: String,
    /// Takes ~30%.
    pub secondary_skill: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackMeta {
    pub id: String,
    pub name: String,
    /// Always "fan" for bundled starter content (Chaosium Fan Material Policy).
    pub license: String,
    pub version: String,
    pub era: String,
}

/// The shape of `content/starter_pack.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub pack: PackMeta,
    pub skills: Vec<Skill>,
    pub careers: Vec<Career>,
    pub hobbies: Vec<Hobby>,
}

/// An indexed, read-only view of one content pack.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub pack_id: String,
    skills: Vec<Skill>,
    careers: Vec<Career>,
    hobbies: Vec<Hobby>,
    skill_index: BTreeMap<String, usize>,
    career_index: BTreeMap<String, usize>,
    hobby_index: BTreeMap<String, usize>,
}

impl Catalog {
    pub fn new(pack_id: impl Into<String>, skills: Vec<Skill>, careers: Vec<Career>, hobbies: Vec<Hobby>) -> Catalog {
        let skill_index = skills
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.clone(), i))
            .collect();
        let career_index = careers
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.clone(), i))
            .collect();
        let hobby_index = hobbies
            .iter()
            .enumerate()
            .map(|(i, h)| (h.id.clone(), i))
            .collect();
        Catalog {
            pack_id: pack_id.into(),
            skills,
            careers,
            hobbies,
            skill_index,
            career_index,
            hobby_index,
        }
    }

    pub fn from_pack_file(file: PackFile) -> Catalog {
        Catalog::new(file.pack.id, file.skills, file.careers, file.hobbies)
    }

    pub fn skill(&self, id: &str) -> Option<&Skill> {
        self.skill_index.get(id).map(|&i| &self.skills[i])
    }

    pub fn career(&self, id: &str) -> Option<&Career> {
        self.career_index.get(id).map(|&i| &self.careers[i])
    }

    pub fn hobby(&self, id: &str) -> Option<&Hobby> {
        self.hobby_index.get(id).map(|&i| &self.hobbies[i])
    }

    pub fn skills(&self) -> &[Skill] {
        &self.skills
    }

    pub fn careers(&self) -> &[Career] {
        &self.careers
    }

    pub fn hobbies(&self) -> &[Hobby] {
        &self.hobbies
    }

    /// The Credit Rating skill id, if the pack defines one.
    pub fn credit_rating_skill(&self) -> Option<&Skill> {
        self.skills.iter().find(|s| s.credit_rating)
    }
}

/// Every skill id a pack's careers and hobbies point at, with where it came from.
///
/// Nothing downstream consults the catalogue before writing points: the allocator writes into
/// `out[skill_id]` while the projection builds the sheet by walking `catalog.skills()`. A skill
/// id that exists in a career or hobby row but not in the skill table therefore *vanishes* from
/// the sheet while still counting against `careerPoolSpent` / `personalPoolSpent` — the ledger
/// stops reconciling and the player silently loses points. Catching it at load time is the whole
/// defence, so this runs over any pack, however it was loaded.
pub fn dangling_skill_references(catalog: &Catalog) -> Vec<String> {
    let mut problems = Vec::new();
    let check = |owner: &str, role: &str, id: &str, problems: &mut Vec<String>| {
        if catalog.skill(id).is_none() {
            problems.push(format!("{owner}: {role} '{id}' is not in the skill catalogue"));
        }
    };

    for career in catalog.careers() {
        if let Some(sig) = &career.signature_skill {
            check(&career.id, "signature skill", sig, &mut problems);
        }
        for slot in &career.supporting_skills {
            match slot {
                SkillSlot::Fixed(id) => check(&career.id, "supporting skill", id, &mut problems),
                SkillSlot::Choice { choice } => {
                    if !career.choice_slots.iter().any(|s| &s.id == choice) {
                        problems.push(format!(
                            "{}: supporting slot refers to choice '{choice}', which the career \
                             does not define",
                            career.id
                        ));
                    }
                }
            }
        }
        for slot in &career.choice_slots {
            if slot.options.is_empty() {
                problems.push(format!("{}: choice '{}' has no options", career.id, slot.id));
            }
            for option in &slot.options {
                check(
                    &career.id,
                    &format!("choice '{}' option '{}'", slot.id, option.id),
                    &option.skill,
                    &mut problems,
                );
            }
        }
    }

    for hobby in catalog.hobbies() {
        check(&hobby.id, "primary skill", &hobby.primary_skill, &mut problems);
        check(
            &hobby.id,
            "secondary skill",
            &hobby.secondary_skill,
            &mut problems,
        );
    }

    problems
}

/// The bundled starter pack, embedded so tests (and the seed generator) need no database.
pub const STARTER_PACK_JSON: &str = include_str!("../content/starter_pack.json");

/// Parse the bundled starter pack. Panics only if the committed JSON is malformed.
pub fn starter_pack_file() -> PackFile {
    serde_json::from_str(STARTER_PACK_JSON).expect("bundled starter_pack.json is valid")
}

pub fn starter_catalog() -> Catalog {
    Catalog::from_pack_file(starter_pack_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_pack_has_the_spec_roster() {
        let catalog = starter_catalog();
        assert_eq!(catalog.careers().len(), 17, "spec § 4 lists 17 careers");
        assert_eq!(catalog.hobbies().len(), 17, "spec § 5 lists 17 hobbies");
        assert_eq!(catalog.pack_id, "starter-modern");
    }

    #[test]
    fn every_referenced_skill_exists_in_the_catalogue() {
        let catalog = starter_catalog();
        let problems = dangling_skill_references(&catalog);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn a_dangling_skill_reference_is_reported_rather_than_silently_dropped() {
        let file = starter_pack_file();
        let mut hobbies = file.hobbies.clone();
        hobbies[0].primary_skill = "skill-that-does-not-exist".to_string();
        let mut careers = file.careers.clone();
        careers[0].signature_skill = Some("also-missing".to_string());

        let broken = Catalog::new(file.pack.id, file.skills, careers, hobbies);
        let problems = dangling_skill_references(&broken);
        assert_eq!(problems.len(), 2, "{problems:#?}");
        assert!(problems.iter().any(|p| p.contains("skill-that-does-not-exist")));
        assert!(problems.iter().any(|p| p.contains("also-missing")));
    }

    #[test]
    fn ladder_slot_counts_are_all_supported() {
        let catalog = starter_catalog();
        for career in catalog.careers() {
            if career.slots_from_hobbies {
                continue; // Jack of All Trades uses its own ladder.
            }
            let slots = career.slot_count();
            assert!(
                (5..=7).contains(&slots),
                "{} has {slots} slots; spec § 6 only ladders 5, 6 and 7",
                career.id
            );
        }
    }

    #[test]
    fn the_two_choice_slot_careers_are_professor_and_influencer() {
        let catalog = starter_catalog();
        let with_choices: Vec<&str> = catalog
            .careers()
            .iter()
            .filter(|c| !c.choice_slots.is_empty())
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(with_choices, vec!["professor", "influencer"]);
    }

    #[test]
    fn jack_of_all_trades_carries_its_special_fields() {
        let catalog = starter_catalog();
        let jack = catalog.career("jack-of-all-trades").unwrap();
        assert_eq!(jack.signature_skill, None);
        assert!(jack.slots_from_hobbies);
        assert_eq!(jack.bonuses.luck, Some(10));
        assert_eq!(jack.formula_terms.len(), 2);
        assert_eq!(
            jack.formula_terms[1].characteristic,
            FormulaCharacteristic::BestOther
        );
        assert_eq!(jack.supporting_skills.len(), 4);
    }

    #[test]
    fn formula_bases_are_wired_to_dodge_and_language_own() {
        let catalog = starter_catalog();
        assert_eq!(catalog.skill("dodge").unwrap().base, SkillBase::DexHalf);
        assert_eq!(catalog.skill("language-own").unwrap().base, SkillBase::Edu);
        assert!(catalog.credit_rating_skill().is_some());
    }

    /// Career names are display copy and must match the spec's roster cell exactly.
    #[test]
    fn career_display_names_keep_their_accents() {
        let catalog = starter_catalog();
        let medium = catalog.career("seance-medium").expect("seance-medium");
        assert_eq!(medium.name, "Séance Medium", "spec section 4, career row 15");
        assert_eq!(medium.id, "seance-medium", "the id stays ASCII");
    }

    #[test]
    fn skill_ids_are_unique() {
        let catalog = starter_catalog();
        assert_eq!(catalog.skill_index.len(), catalog.skills().len());
        assert_eq!(catalog.career_index.len(), catalog.careers().len());
        assert_eq!(catalog.hobby_index.len(), catalog.hobbies().len());
    }
}
