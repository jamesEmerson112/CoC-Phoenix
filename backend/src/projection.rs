//! The sheet projection (spec § 2 "live projection", § 6 stacking cap, § 7 derived values).
//!
//! `answers + catalogue -> sheet`, pure. Nothing here is ever stored while a character is a
//! draft; on finalize the result is snapshotted verbatim.

use crate::allocator::{allocate, Allocation, SKILL_CAP};
use crate::content::{Catalog, SkillBase};
use crate::derived::{
    age_effects, damage_bonus_and_build, fifth, half, hit_points, magic_points, move_base,
    move_rate, starting_sanity, wallet, Wallet,
};
use crate::model::{AgeBand, Answers, Characteristic};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueView {
    pub id: String,
    pub name: String,
    pub value: i32,
    pub half: i32,
    pub fifth: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillView {
    pub id: String,
    pub name: String,
    /// Starting percentage before any allocation.
    pub base: i32,
    pub career_points: i32,
    pub personal_points: i32,
    pub value: i32,
    pub half: i32,
    pub fifth: i32,
    /// True when the 75 cap swallowed points (spec § 6: they are not redistributed).
    pub capped: bool,
    /// Points lost to the cap, shown so the player can see the cost of doubling down.
    pub points_lost_to_cap: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivedView {
    pub hit_points: i32,
    pub magic_points: i32,
    pub sanity: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub luck: Option<i32>,
    pub luck_bonus: i32,
    pub damage_bonus: String,
    pub build: i32,
    pub move_rate: i32,
    pub move_base: i32,
    pub move_age_penalty: i32,
    pub dodge_base: i32,
    pub language_own_base: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolsView {
    pub career_pool: i32,
    pub career_pool_spent: i32,
    /// Points granted beyond what the pool held, because the career's Credit Rating minimum is
    /// paid whatever the pool can afford (spec § 6 rule 1). Almost always 0; when it is not, the
    /// sheet says so rather than quietly showing `spent > pool`.
    ///
    /// `default` so snapshots frozen before this field existed still deserialize — a finalized
    /// sheet is immutable and must keep loading forever.
    #[serde(default)]
    pub career_pool_shortfall: i32,
    pub personal_pool: i32,
    pub personal_pool_spent: i32,
    pub credit_rating_cost: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerView {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub pool: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_other: Option<Characteristic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub luck_bonus: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HobbyView {
    pub id: String,
    pub name: String,
    pub blurb: String,
    /// 1-based pick order; the first pick is the passion.
    pub pick: usize,
    pub primary_skill: String,
    pub secondary_skill: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Completeness {
    pub ready_to_finalize: bool,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeView {
    pub age: i32,
    pub band: AgeBand,
    pub edu_checks_granted: usize,
    pub luck_rolls_granted: usize,
}

/// The full investigator sheet (spec § 2 step 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub name: Option<String>,
    pub pronouns: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<AgeView>,
    pub characteristics: Vec<ValueView>,
    pub derived: DerivedView,
    pub skills: Vec<SkillView>,
    pub credit_rating: CreditRatingView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub career: Option<CareerView>,
    pub hobbies: Vec<HobbyView>,
    pub pools: PoolsView,
    /// What age did to the rolled numbers, for the "costs are flavor" display.
    pub age_adjustments: Vec<crate::allocator::AgeAdjustment>,
    pub completeness: Completeness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditRatingView {
    pub value: i32,
    pub half: i32,
    pub fifth: i32,
    #[serde(flatten)]
    pub wallet: Wallet,
}

fn characteristic_name(c: Characteristic) -> &'static str {
    match c {
        Characteristic::Str => "Strength",
        Characteristic::Con => "Constitution",
        Characteristic::Siz => "Size",
        Characteristic::Dex => "Dexterity",
        Characteristic::App => "Appearance",
        Characteristic::Pow => "Power",
        Characteristic::Int => "Intelligence",
        Characteristic::Edu => "Education",
    }
}

/// Project a sheet from answers. Safe on an empty draft.
pub fn project(catalog: &Catalog, answers: &Answers) -> Sheet {
    let allocation = allocate(catalog, answers);
    project_with(catalog, answers, &allocation)
}

/// Project a sheet from an allocation you already computed.
pub fn project_with(catalog: &Catalog, answers: &Answers, allocation: &Allocation) -> Sheet {
    let chars = allocation.characteristics;
    let age = answers.effective_age();
    let effects = age.map(age_effects);

    let characteristics = Characteristic::ALL
        .into_iter()
        .map(|c| {
            let value = chars.get(c);
            ValueView {
                id: c.key().to_string(),
                name: characteristic_name(c).to_string(),
                value,
                half: half(value),
                fifth: fifth(value),
            }
        })
        .collect();

    let db = damage_bonus_and_build(chars.strength, chars.siz);
    let derived = DerivedView {
        hit_points: hit_points(chars.con, chars.siz),
        magic_points: magic_points(chars.pow),
        sanity: starting_sanity(chars.pow),
        luck: allocation.luck,
        luck_bonus: allocation.luck_bonus,
        damage_bonus: db.damage_bonus,
        build: db.build,
        move_rate: move_rate(
            chars.strength,
            chars.dex,
            chars.siz,
            age.unwrap_or(AgeBand::Prime.default_age()),
        ),
        move_base: move_base(chars.strength, chars.dex, chars.siz),
        move_age_penalty: effects.map(|e| e.mov_penalty).unwrap_or(0),
        dodge_base: half(chars.dex),
        language_own_base: chars.edu,
    };

    let cr_skill_id = catalog.credit_rating_skill().map(|s| s.id.clone());
    let mut skills = Vec::with_capacity(catalog.skills().len());
    for skill in catalog.skills() {
        if Some(&skill.id) == cr_skill_id.as_ref() {
            continue; // Credit Rating gets its own block on the sheet.
        }
        let base = match skill.base {
            SkillBase::Fixed { value } => value,
            SkillBase::DexHalf => half(chars.dex),
            SkillBase::Edu => chars.edu,
        };
        let career_points = allocation
            .career_points
            .get(&skill.id)
            .copied()
            .unwrap_or(0);
        let personal_points = allocation
            .personal_points
            .get(&skill.id)
            .copied()
            .unwrap_or(0);
        let uncapped = base + career_points + personal_points;
        // The cap limits what allocation can buy; it never claws back a base value that is
        // already above it (Language (Own) = EDU can start at 90).
        let value = uncapped.min(SKILL_CAP).max(base);
        skills.push(SkillView {
            id: skill.id.clone(),
            name: skill.name.clone(),
            base,
            career_points,
            personal_points,
            value,
            half: half(value),
            fifth: fifth(value),
            capped: uncapped > value,
            points_lost_to_cap: (uncapped - value).max(0),
        });
    }

    let credit_rating_value = cr_skill_id
        .as_ref()
        .and_then(|id| allocation.career_points.get(id))
        .copied()
        .unwrap_or(0);
    let credit_rating = CreditRatingView {
        value: credit_rating_value,
        half: half(credit_rating_value),
        fifth: fifth(credit_rating_value),
        wallet: wallet(credit_rating_value),
    };

    let career = answers
        .career_id
        .as_deref()
        .and_then(|id| catalog.career(id))
        .map(|career| CareerView {
            id: career.id.clone(),
            name: career.name.clone(),
            blurb: career.blurb.clone(),
            pool: allocation.career_pool,
            best_other: allocation.best_other,
            luck_bonus: career.bonuses.luck,
        });

    let hobbies = answers
        .hobby_ids
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            catalog.hobby(id).map(|hobby| HobbyView {
                id: hobby.id.clone(),
                name: hobby.name.clone(),
                blurb: hobby.blurb.clone(),
                pick: i + 1,
                primary_skill: hobby.primary_skill.clone(),
                secondary_skill: hobby.secondary_skill.clone(),
            })
        })
        .collect();

    Sheet {
        name: answers.name.clone(),
        pronouns: answers.pronouns.clone(),
        age: age.zip(effects).map(|(age, e)| AgeView {
            age,
            band: e.band,
            edu_checks_granted: e.edu_checks,
            luck_rolls_granted: e.luck_rolls,
        }),
        characteristics,
        derived,
        skills,
        credit_rating,
        career,
        hobbies,
        pools: PoolsView {
            career_pool: allocation.career_pool,
            career_pool_spent: allocation.career_pool_spent,
            career_pool_shortfall: allocation.career_pool_shortfall,
            personal_pool: allocation.personal_pool,
            personal_pool_spent: allocation.personal_pool_spent,
            credit_rating_cost: allocation.credit_rating,
        },
        age_adjustments: allocation.adjustments.clone(),
        completeness: completeness(catalog, answers),
    }
}

/// Spec § 9: what still has to be answered before the character can be frozen.
pub fn completeness(catalog: &Catalog, answers: &Answers) -> Completeness {
    let mut missing = Vec::new();

    if answers.name.as_deref().map(str::trim).unwrap_or("").is_empty() {
        missing.push("name".to_string());
    }

    match answers.effective_age() {
        None => missing.push("age".to_string()),
        Some(age) if !(15..=89).contains(&age) => {
            missing.push("age must be between 15 and 89".to_string())
        }
        Some(_) => {}
    }

    let dice = answers.dice_ref();
    match dice {
        None => missing.push("dice".to_string()),
        Some(dice) => {
            if dice.characteristics.is_none() {
                missing.push("characteristics".to_string());
            }
            if let Some(age) = answers.effective_age() {
                let effects = age_effects(age);
                if dice.edu_checks.len() != effects.edu_checks {
                    missing.push(format!(
                        "eduChecks (expected {}, have {})",
                        effects.edu_checks,
                        dice.edu_checks.len()
                    ));
                }
            }
            // Luck is checked against the band the same way EDU checks are. Young's entire
            // compensation for EDU -5 and the split STR/SIZ cost is "rolled twice, keep the
            // better" (spec § 2), and a band changed after the roll would otherwise freeze a
            // single roll into a snapshot that still advertises `luckRollsGranted: 2`.
            let luck_rolls = dice.luck.as_ref().map(|l| l.rolls.len()).unwrap_or(0);
            match answers.effective_band() {
                Some(band) => {
                    let expected = crate::dice::luck_roll_count(band);
                    if luck_rolls != expected {
                        missing.push(format!("luck (expected {expected}, have {luck_rolls})"));
                    }
                }
                None if luck_rolls == 0 => missing.push("luck".to_string()),
                None => {}
            }
        }
    }

    match answers.career_id.as_deref() {
        None => missing.push("career".to_string()),
        Some(id) => match catalog.career(id) {
            None => missing.push(format!("career '{id}' is not in the content pack")),
            Some(career) => {
                for slot in &career.choice_slots {
                    match answers.career_choices.get(&slot.id) {
                        None => missing.push(format!("careerChoices.{}", slot.id)),
                        Some(answer) => {
                            if !slot.options.iter().any(|o| &o.id == answer) {
                                missing.push(format!(
                                    "careerChoices.{} = '{answer}' is not an option",
                                    slot.id
                                ));
                            }
                        }
                    }
                }
            }
        },
    }

    if answers.hobby_ids.len() != 4 {
        missing.push(format!(
            "exactly 4 hobbies (have {})",
            answers.hobby_ids.len()
        ));
    }
    for id in &answers.hobby_ids {
        if catalog.hobby(id).is_none() {
            missing.push(format!("hobby '{id}' is not in the content pack"));
        }
    }
    let mut unique = answers.hobby_ids.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != answers.hobby_ids.len() {
        missing.push("hobbies must be four different picks".to_string());
    }

    Completeness {
        ready_to_finalize: missing.is_empty(),
        missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::starter_catalog;
    use crate::model::{Characteristics, DiceAnswers, DiceSource, EduCheck, LuckAnswer};

    fn chars(values: [i32; 8]) -> Characteristics {
        Characteristics {
            strength: values[0],
            con: values[1],
            siz: values[2],
            dex: values[3],
            app: values[4],
            pow: values[5],
            intelligence: values[6],
            edu: values[7],
        }
    }

    /// A complete, finalizable answer set built on the bundled starter pack.
    fn complete_answers() -> Answers {
        Answers {
            name: Some("Ada Kowalski".to_string()),
            pronouns: Some("she/her".to_string()),
            age: Some(27),
            age_band: Some(AgeBand::Prime),
            dice: Some(DiceAnswers {
                source: DiceSource::Server,
                characteristics: Some(chars([50, 60, 55, 65, 55, 70, 75, 80])),
                edu_checks: vec![EduCheck { d100: 95, gain: 4 }],
                luck: Some(LuckAnswer {
                    rolls: vec![55],
                    kept: Some(55),
                }),
                ..Default::default()
            }),
            career_id: Some("funerary-director".to_string()),
            hobby_ids: vec![
                "occult-curiosity".to_string(),
                "people-watching".to_string(),
                "poker-nights".to_string(),
                "bookworm".to_string(),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_complete_draft_is_ready_to_finalize() {
        let catalog = starter_catalog();
        let sheet = project(&catalog, &complete_answers());
        assert!(
            sheet.completeness.ready_to_finalize,
            "{:?}",
            sheet.completeness.missing
        );
    }

    #[test]
    fn completeness_lists_everything_an_empty_draft_still_needs() {
        let catalog = starter_catalog();
        let sheet = project(&catalog, &Answers::default());
        let missing = sheet.completeness.missing;
        assert!(missing.contains(&"name".to_string()));
        assert!(missing.contains(&"age".to_string()));
        assert!(missing.contains(&"dice".to_string()));
        assert!(missing.contains(&"career".to_string()));
        assert!(missing.iter().any(|m| m.starts_with("exactly 4 hobbies")));
    }

    #[test]
    fn an_unanswered_choice_slot_blocks_finalizing() {
        let catalog = starter_catalog();
        let mut answers = complete_answers();
        answers.career_id = Some("professor".to_string());
        let sheet = project(&catalog, &answers);
        assert!(sheet
            .completeness
            .missing
            .contains(&"careerChoices.science".to_string()));

        answers
            .career_choices
            .insert("science".to_string(), "chemistry".to_string());
        let sheet = project(&catalog, &answers);
        assert!(sheet.completeness.ready_to_finalize);
        // The chosen science is the skill that got the ladder points.
        let chemistry = sheet
            .skills
            .iter()
            .find(|s| s.id == "science-chemistry")
            .unwrap();
        assert!(chemistry.career_points > 0);
        let biology = sheet
            .skills
            .iter()
            .find(|s| s.id == "science-biology")
            .unwrap();
        assert_eq!(biology.career_points, 0);
    }

    #[test]
    fn duplicate_hobbies_block_finalizing() {
        let catalog = starter_catalog();
        let mut answers = complete_answers();
        answers.hobby_ids = vec![
            "bookworm".to_string(),
            "bookworm".to_string(),
            "poker-nights".to_string(),
            "tinkering".to_string(),
        ];
        let sheet = project(&catalog, &answers);
        assert!(sheet
            .completeness
            .missing
            .contains(&"hobbies must be four different picks".to_string()));
    }

    #[test]
    fn the_wrong_number_of_edu_checks_blocks_finalizing() {
        let catalog = starter_catalog();
        let mut answers = complete_answers();
        answers.age = Some(48); // seasoned: two checks
        let sheet = project(&catalog, &answers);
        assert!(sheet
            .completeness
            .missing
            .iter()
            .any(|m| m.starts_with("eduChecks (expected 2")));
    }

    #[test]
    fn a_young_character_cannot_be_frozen_with_a_single_luck_roll() {
        let catalog = starter_catalog();
        let mut answers = complete_answers();
        // Rolled Luck as a Prime (one roll), then went back and answered "young".
        answers.age = Some(17);
        answers.age_band = Some(AgeBand::Young);
        answers.dice.as_mut().unwrap().edu_checks.clear(); // Young grants none
        let sheet = project(&catalog, &answers);
        assert_eq!(sheet.age.as_ref().unwrap().luck_rolls_granted, 2);
        assert!(
            sheet
                .completeness
                .missing
                .contains(&"luck (expected 2, have 1)".to_string()),
            "{:?}",
            sheet.completeness.missing
        );
        assert!(!sheet.completeness.ready_to_finalize);

        // Re-rolling both makes it finalizable, and the better roll is the one that counts.
        answers.dice.as_mut().unwrap().luck = Some(LuckAnswer {
            rolls: vec![55, 80],
            kept: Some(80),
        });
        let sheet = project(&catalog, &answers);
        assert!(
            sheet.completeness.ready_to_finalize,
            "{:?}",
            sheet.completeness.missing
        );
        assert_eq!(sheet.derived.luck, Some(80));
    }

    #[test]
    fn a_stray_second_luck_roll_outside_the_young_band_blocks_finalizing() {
        let catalog = starter_catalog();
        let mut answers = complete_answers();
        answers.dice.as_mut().unwrap().luck = Some(LuckAnswer {
            rolls: vec![55, 80],
            kept: Some(80),
        });
        let sheet = project(&catalog, &answers);
        assert!(sheet
            .completeness
            .missing
            .contains(&"luck (expected 1, have 2)".to_string()));
    }

    #[test]
    fn edu_checks_from_a_wider_band_do_not_leak_into_the_live_sheet() {
        let catalog = starter_catalog();
        let mut answers = complete_answers();
        answers.age = Some(65); // Venerable: four checks
        answers.age_band = Some(AgeBand::Venerable);
        answers.dice.as_mut().unwrap().edu_checks = vec![EduCheck { d100: 99, gain: 8 }; 4];
        let venerable = project(&catalog, &answers);
        assert!(venerable.derived.language_own_base > 80, "checks did apply");

        // Going back to Young must re-render the sheet without those four checks.
        answers.age = Some(17);
        answers.age_band = Some(AgeBand::Young);
        let young = project(&catalog, &answers);
        assert_eq!(young.age.as_ref().unwrap().edu_checks_granted, 0);
        assert_eq!(young.derived.language_own_base, 75, "EDU 80 minus the Young -5");
        assert!(
            !young
                .age_adjustments
                .iter()
                .any(|a| a.reason == "EDU life experience"),
            "{:?}",
            young.age_adjustments
        );
    }

    #[test]
    fn derived_values_follow_the_spec_seven_table() {
        let catalog = starter_catalog();
        let sheet = project(&catalog, &complete_answers());
        // CON 60, SIZ 55 -> 11 HP. POW 70 -> 14 MP, 70 SAN.
        assert_eq!(sheet.derived.hit_points, 11);
        assert_eq!(sheet.derived.magic_points, 14);
        assert_eq!(sheet.derived.sanity, 70);
        // STR 50 + SIZ 55 = 105 -> 0 / 0.
        assert_eq!(sheet.derived.damage_bonus, "0");
        assert_eq!(sheet.derived.build, 0);
        // DEX 65 > SIZ 55 but STR 50 < SIZ -> 8.
        assert_eq!(sheet.derived.move_base, 8);
        assert_eq!(sheet.derived.move_rate, 8);
        assert_eq!(sheet.derived.dodge_base, 32);
        // EDU 80, and the 95 check beats it for +4.
        assert_eq!(sheet.derived.language_own_base, 84);
        assert_eq!(sheet.derived.luck, Some(55));
    }

    #[test]
    fn half_and_fifth_are_published_for_every_characteristic() {
        let catalog = starter_catalog();
        let sheet = project(&catalog, &complete_answers());
        let pow = sheet.characteristics.iter().find(|c| c.id == "pow").unwrap();
        assert_eq!(pow.value, 70);
        assert_eq!(pow.half, 35);
        assert_eq!(pow.fifth, 14);
    }

    #[test]
    fn dodge_and_language_own_use_their_formula_bases() {
        let catalog = starter_catalog();
        let sheet = project(&catalog, &complete_answers());
        let dodge = sheet.skills.iter().find(|s| s.id == "dodge").unwrap();
        assert_eq!(dodge.base, 32); // DEX 65 // 2
        let language = sheet.skills.iter().find(|s| s.id == "language-own").unwrap();
        assert_eq!(language.base, 84); // = EDU
        assert_eq!(language.value, 84, "a base above the cap is never clawed back");
        assert!(!language.capped);
    }

    #[test]
    fn stacked_points_are_capped_at_seventy_five_and_not_redistributed() {
        let catalog = starter_catalog();
        let mut answers = complete_answers();
        // Funerary Director's signature is Psychology; People-watching and Poker nights both
        // pour more Psychology on top.
        answers.hobby_ids = vec![
            "poker-nights".to_string(),
            "people-watching".to_string(),
            "online-gaming".to_string(),
            "volunteer-first-aider".to_string(),
        ];
        let sheet = project(&catalog, &answers);
        let psychology = sheet.skills.iter().find(|s| s.id == "psychology").unwrap();
        let uncapped =
            psychology.base + psychology.career_points + psychology.personal_points;
        assert!(uncapped > SKILL_CAP, "the fixture must actually overflow");
        assert_eq!(psychology.value, SKILL_CAP);
        assert!(psychology.capped);
        assert_eq!(psychology.points_lost_to_cap, uncapped - SKILL_CAP);
        // Nothing was handed to another skill to compensate.
        assert_eq!(sheet.pools.personal_pool_spent, sheet.pools.personal_pool);
    }

    #[test]
    fn credit_rating_gets_its_own_block_with_plain_words() {
        let catalog = starter_catalog();
        let sheet = project(&catalog, &complete_answers());
        assert_eq!(sheet.credit_rating.value, 20); // Funerary Director minimum
        assert_eq!(sheet.credit_rating.half, 10);
        assert!(sheet.credit_rating.wallet.descriptor.starts_with("Average"));
        assert!(sheet.skills.iter().all(|s| s.id != "credit-rating"));
    }

    #[test]
    fn the_same_answers_project_the_identical_sheet_every_time() {
        let catalog = starter_catalog();
        let answers = complete_answers();
        let first = serde_json::to_string(&project(&catalog, &answers)).unwrap();
        for _ in 0..10 {
            assert_eq!(
                serde_json::to_string(&project(&catalog, &answers)).unwrap(),
                first
            );
        }
        // ...and a re-parsed answer record projects the same sheet again.
        let round_tripped: Answers =
            serde_json::from_str(&serde_json::to_string(&answers).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_string(&project(&catalog, &round_tripped)).unwrap(),
            first
        );
    }

    #[test]
    fn jack_of_all_trades_projects_the_luck_perk_onto_the_sheet() {
        let catalog = starter_catalog();
        let mut answers = complete_answers();
        answers.career_id = Some("jack-of-all-trades".to_string());
        let sheet = project(&catalog, &answers);
        assert_eq!(sheet.derived.luck, Some(65)); // 55 rolled + 10
        assert_eq!(sheet.derived.luck_bonus, 10);
        assert_eq!(sheet.career.as_ref().unwrap().luck_bonus, Some(10));
        assert!(sheet.completeness.ready_to_finalize);
        // Career points flowed into the first hobby's primary skill (Occult).
        let occult = sheet.skills.iter().find(|s| s.id == "occult").unwrap();
        assert!(occult.career_points > 0);
        assert!(occult.personal_points > 0);
    }

    #[test]
    fn a_sheet_round_trips_through_json_for_the_finalize_snapshot() {
        let catalog = starter_catalog();
        let sheet = project(&catalog, &complete_answers());
        let json = serde_json::to_string(&sheet).unwrap();
        let back: Sheet = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sheet);
    }
}
