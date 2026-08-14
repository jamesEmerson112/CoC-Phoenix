//! §4 / §5 roster conformance, §9 draft-lifecycle gates, and a few §6/§7 corners that only
//! show up on the real 17-career / 17-hobby rosters.
//!
//! The tables below are transcribed from the spec by hand; nothing was read off the shipped
//! content pack.

use std::collections::BTreeMap;

use coc_phoenix::allocator::{allocate, career_pool_for};
use coc_phoenix::content::{starter_catalog, FormulaCharacteristic, SkillSlot};
use coc_phoenix::derived::{fifth, half};
use coc_phoenix::model::{AgeBand, Answers, Characteristic};
use coc_phoenix::projection::project;

use crate::common::{chars8, draft};

use FormulaCharacteristic as F;

// =============================================================================================
// §4 — the 17-career table
// =============================================================================================

/// `(id, formula terms, credit rating min, max, supporting-slot count)`
///
/// Slot count = the spec's "Supporting skills" column; the signature is counted separately.
const CAREERS: &[(&str, &[(F, i32)], i32, i32, usize)] = &[
    ("doctor", &[(F::Edu, 4)], 30, 80, 5),
    ("professor", &[(F::Edu, 4)], 20, 70, 5),
    ("antiquarian", &[(F::Edu, 4)], 30, 70, 5),
    ("journalist", &[(F::Edu, 4)], 9, 30, 5),
    ("private-investigator", &[(F::Edu, 2), (F::Dex, 2)], 9, 30, 6),
    ("police-detective", &[(F::Edu, 2), (F::Int, 2)], 20, 50, 5),
    ("soldier-veteran", &[(F::Edu, 2), (F::Str, 2)], 9, 30, 5),
    ("criminal", &[(F::Edu, 2), (F::Dex, 2)], 5, 65, 5),
    ("clergy", &[(F::Edu, 4)], 9, 60, 5),
    ("influencer", &[(F::App, 2), (F::Edu, 2)], 50, 99, 4),
    ("jack-of-all-trades", &[(F::Edu, 2), (F::BestOther, 2)], 9, 50, 4),
    ("grid-operations-engineer", &[(F::Edu, 4)], 30, 60, 5),
    ("interior-architect", &[(F::Edu, 2), (F::App, 2)], 30, 70, 5),
    ("nail-artist", &[(F::App, 2), (F::Dex, 2)], 9, 40, 5),
    ("seance-medium", &[(F::Pow, 2), (F::App, 2)], 9, 40, 5),
    ("funerary-director", &[(F::Edu, 2), (F::App, 2)], 20, 50, 5),
    ("professional-mma-fighter", &[(F::Str, 2), (F::Dex, 2)], 9, 40, 4),
];

#[test]
fn spec4_the_roster_is_seventeen_careers_with_the_spec_formulas_and_cr_ranges() {
    let catalog = starter_catalog();
    assert_eq!(catalog.careers().len(), 17, "§4 lists 17 careers");

    for &(id, terms, cr_min, cr_max, supporting) in CAREERS {
        let career = catalog
            .career(id)
            .unwrap_or_else(|| panic!("§4 career {id} is missing from the pack"));

        let got: Vec<(F, i32)> = career
            .formula_terms
            .iter()
            .map(|t| (t.characteristic, t.multiplier))
            .collect();
        assert_eq!(got, terms.to_vec(), "{id} points formula");

        assert_eq!(career.credit_rating.min, cr_min, "{id} CR minimum");
        assert_eq!(career.credit_rating.max, cr_max, "{id} CR maximum");
        assert_eq!(
            career.supporting_skills.len(),
            supporting,
            "{id} supporting-skill count"
        );
    }
}

#[test]
fn spec6_every_career_ladders_at_five_six_or_seven_slots() {
    // §6.2 only defines ladders for 5, 6 and 7 slots. Jack of All Trades is explicitly
    // different (§4.1) and is excluded.
    let catalog = starter_catalog();
    for career in catalog.careers() {
        if career.slots_from_hobbies {
            continue;
        }
        assert!(
            career.signature_skill.is_some(),
            "{} has no signature skill but is not the Jack",
            career.id
        );
        let slots = career.slot_count();
        assert!(
            (5..=7).contains(&slots),
            "{} has {slots} ladder slots; §6.2 only defines 5, 6 and 7",
            career.id
        );
    }
}

#[test]
fn spec4_exactly_two_careers_ask_a_choice_slot_question() {
    let catalog = starter_catalog();
    let with_choices: Vec<&str> = catalog
        .careers()
        .iter()
        .filter(|c| !c.choice_slots.is_empty())
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(
        with_choices,
        vec!["professor", "influencer"],
        "§4: 'Exactly two careers ask one extra question on selection'"
    );

    // Professor — "What's your field?" -> Science (Biology / Chemistry / Physics / Astronomy)
    let professor = catalog.career("professor").unwrap();
    assert_eq!(professor.choice_slots.len(), 1);
    let options: Vec<&str> = professor.choice_slots[0]
        .options
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    assert_eq!(options, vec!["biology", "chemistry", "physics", "astronomy"]);

    // Influencer — "What's your medium?" -> Art/Craft (Video / Photography / Music / Writing)
    let influencer = catalog.career("influencer").unwrap();
    assert_eq!(influencer.choice_slots.len(), 1);
    let options: Vec<&str> = influencer.choice_slots[0]
        .options
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    assert_eq!(options, vec!["video", "photography", "music", "writing"]);
}

#[test]
fn spec41_jack_of_all_trades_carries_its_four_special_rules() {
    let catalog = starter_catalog();
    let jack = catalog.career("jack-of-all-trades").unwrap();

    // 1. EDU x2 + best-other x2.
    assert_eq!(jack.formula_terms.len(), 2);
    assert_eq!(jack.formula_terms[1].characteristic, F::BestOther);
    // 2. No signature skill; four fixed everyday skills, in the spec's order.
    assert_eq!(jack.signature_skill, None);
    let fixed: Vec<&str> = jack
        .supporting_skills
        .iter()
        .map(|s| match s {
            SkillSlot::Fixed(id) => id.as_str(),
            SkillSlot::Choice { .. } => panic!("the Jack has no choice slots"),
        })
        .collect();
    assert_eq!(
        fixed,
        vec!["drive-auto", "first-aid", "spot-hidden", "listen"],
        "§4.1: Drive Auto, First Aid, Spot Hidden, Listen"
    );
    // 3. The rest of the pool flows into the player's own hobby picks.
    assert!(jack.slots_from_hobbies);
    // 4. +10 starting Luck, stored as an explicit, toggleable occupation field.
    assert_eq!(jack.bonuses.luck, Some(10));

    // ...and no other career has the perk.
    for career in catalog.careers() {
        if career.id != "jack-of-all-trades" {
            assert_eq!(career.bonuses.luck, None, "{} must not carry a Luck perk", career.id);
        }
    }
}

// =============================================================================================
// §5 — the 17-hobby table
// =============================================================================================

const HOBBIES: &[(&str, &str, &str)] = &[
    ("photography", "art-craft-photography", "spot-hidden"),
    ("amateur-boxing", "fighting-brawl", "dodge"),
    ("hunting-trips", "firearms-rifle-shotgun", "track"),
    ("bookworm", "library-use", "history"),
    ("motoring-enthusiast", "drive-auto", "mechanical-repair"),
    ("tinkering", "mechanical-repair", "electronics"),
    ("occult-curiosity", "occult", "library-use"),
    ("people-watching", "spot-hidden", "psychology"),
    ("social-butterfly", "charm", "listen"),
    ("volunteer-first-aider", "first-aid", "psychology"),
    ("poker-nights", "psychology", "sleight-of-hand"),
    ("stage-magic", "sleight-of-hand", "charm"),
    ("weekend-carpenter", "art-craft-carpentry", "mechanical-repair"),
    ("amateur-gastronome", "art-craft-cooking", "natural-world"),
    ("makeup-artistry", "disguise", "charm"),
    ("online-gaming", "computer-use", "psychology"),
    ("urban-exploration", "stealth", "climb"),
];

#[test]
fn spec5_every_hobby_points_at_its_spec_primary_and_secondary() {
    let catalog = starter_catalog();
    assert_eq!(catalog.hobbies().len(), 17, "§5 lists 17 hobbies");
    for &(id, primary, secondary) in HOBBIES {
        let hobby = catalog
            .hobby(id)
            .unwrap_or_else(|| panic!("§5 hobby {id} is missing from the pack"));
        assert_eq!(hobby.primary_skill, primary, "{id} primary (70%)");
        assert_eq!(hobby.secondary_skill, secondary, "{id} secondary (30%)");
    }
}

#[test]
fn spec5_cthulhu_mythos_is_never_offerable_at_creation() {
    // "Cthulhu Mythos is never offerable at creation — Occult is the only on-theme skill."
    let catalog = starter_catalog();
    let mythos = |id: &str| id.contains("mythos") || id.contains("cthulhu");
    assert!(
        catalog.skills().iter().all(|s| !mythos(&s.id)),
        "no Mythos skill may exist in the creation catalogue"
    );
    for hobby in catalog.hobbies() {
        assert!(!mythos(&hobby.primary_skill));
        assert!(!mythos(&hobby.secondary_skill));
    }
}

// =============================================================================================
// §4 — "sorted by fit": the payout formula is what orders the cards
// =============================================================================================

#[test]
fn spec4_career_fit_is_the_points_formula_evaluated_on_the_players_stats() {
    // A brilliant, unathletic investigator: EDU 90, INT 80, everything physical low.
    let scholar = chars8([25, 30, 50, 30, 30, 40, 80, 90]);
    assert_eq!(career_pool_for(starter_catalog().career("doctor").unwrap(), &scholar), 360);
    assert_eq!(
        career_pool_for(starter_catalog().career("professional-mma-fighter").unwrap(), &scholar),
        110,
        "STR 25 x2 + DEX 30 x2"
    );

    let catalog = starter_catalog();
    let mut ranked: Vec<(&str, i32)> = catalog
        .careers()
        .iter()
        .map(|c| (c.id.as_str(), career_pool_for(c, &scholar)))
        .collect();
    ranked.sort_by_key(|&(id, pool)| (-pool, id));
    assert_eq!(
        ranked[0].1, 360,
        "the best payout for this array is an EDU x4 career"
    );
    assert_eq!(
        ranked.last().unwrap().0,
        "professional-mma-fighter",
        "the worst fit for a 25 STR / 30 DEX investigator"
    );
}

// =============================================================================================
// §6 — age deductions on the real roster
// =============================================================================================

#[test]
fn spec6_age_deductions_avoid_the_characteristic_the_chosen_career_lives_on() {
    let catalog = starter_catalog();
    // 65 -> the 60s: physicals -20.
    let chars = chars8([60, 60, 55, 60, 60, 60, 60, 60]);
    let checks = [(70, 4), (71, 3), (72, 2), (73, 1)];

    // Soldier (Veteran) = EDU x2 + STR x2: STR is what the career is made of, so it must not
    // be the place the 20 points come from.
    let soldier = allocate(
        &catalog,
        &draft("Wren Ashcombe-Lyle", 65, chars, Some("soldier-veteran"), &[], &checks, &[45]),
    );
    assert_eq!(soldier.characteristics.strength, 60, "STR x2 is protected");
    assert_eq!(
        soldier.characteristics.strength + soldier.characteristics.con + soldier.characteristics.dex,
        60 + 60 + 60 - 20,
        "exactly 20 points came off the physicals"
    );

    // Professional MMA Fighter = STR x2 + DEX x2: both are protected, so CON pays.
    let fighter = allocate(
        &catalog,
        &draft("Wren Ashcombe-Lyle", 65, chars, Some("professional-mma-fighter"), &[], &checks, &[45]),
    );
    assert_eq!(fighter.characteristics.strength, 60);
    assert_eq!(fighter.characteristics.dex, 60);
    assert_eq!(fighter.characteristics.con, 40);

    // Private Investigator = EDU x2 + DEX x2: DEX is protected.
    let pi = allocate(
        &catalog,
        &draft("Wren Ashcombe-Lyle", 65, chars, Some("private-investigator"), &[], &checks, &[45]),
    );
    assert_eq!(pi.characteristics.dex, 60, "DEX x2 is protected");
}

#[test]
fn spec2_young_jack_of_all_trades_keeps_the_better_luck_then_caps_at_99() {
    let catalog = starter_catalog();
    // §2 Young: two Luck rolls, keep the better. §4.1: +10, capped at 99.
    let answers = draft(
        "Wren Ashcombe-Lyle",
        17,
        chars8([50, 50, 55, 50, 50, 50, 50, 60]),
        Some("jack-of-all-trades"),
        &["bookworm", "poker-nights", "tinkering", "social-butterfly"],
        &[],
        &[55, 90],
    );
    let a = allocate(&catalog, &answers);
    assert_eq!(a.luck, Some(99), "best of 55/90 is 90, +10 -> 100 clipped to 99");

    let quieter = draft(
        "Wren Ashcombe-Lyle",
        17,
        chars8([50, 50, 55, 50, 50, 50, 50, 60]),
        Some("jack-of-all-trades"),
        &["bookworm", "poker-nights", "tinkering", "social-butterfly"],
        &[],
        &[55, 60],
    );
    assert_eq!(allocate(&catalog, &quieter).luck, Some(70), "60 + 10");
}

// =============================================================================================
// §7 — half and fifth are published for skills too, not just characteristics
// =============================================================================================

#[test]
fn spec7_half_and_fifth_accompany_every_skill_and_characteristic_on_the_sheet() {
    let catalog = starter_catalog();
    let sheet = project(
        &catalog,
        &draft(
            "Ilse Marchetti-Vaughn",
            27,
            chars8([55, 60, 60, 70, 45, 65, 55, 50]),
            Some("police-detective"),
            &["urban-exploration", "amateur-boxing", "bookworm", "amateur-gastronome"],
            &[(45, 6)],
            &[55],
        ),
    );
    for c in &sheet.characteristics {
        assert_eq!(c.half, half(c.value), "{} half", c.id);
        assert_eq!(c.fifth, fifth(c.value), "{} fifth", c.id);
    }
    for s in &sheet.skills {
        assert_eq!(s.half, half(s.value), "{} half", s.id);
        assert_eq!(s.fifth, fifth(s.value), "{} fifth", s.id);
    }
    assert_eq!(sheet.credit_rating.half, half(sheet.credit_rating.value));
    assert_eq!(sheet.credit_rating.fifth, fifth(sheet.credit_rating.value));
}

// =============================================================================================
// §9 — draft lifecycle gates
// =============================================================================================

fn complete() -> Answers {
    let mut answers = draft(
        "Ilse Marchetti-Vaughn",
        27,
        chars8([55, 60, 60, 70, 45, 65, 55, 50]),
        Some("police-detective"),
        &["urban-exploration", "amateur-boxing", "bookworm", "amateur-gastronome"],
        &[(45, 6)],
        &[55],
    );
    answers.pronouns = Some("they/them".to_string());
    answers.career_choices = BTreeMap::new();
    answers
}

#[test]
fn spec5_finalizing_requires_exactly_four_distinct_hobbies() {
    let catalog = starter_catalog();
    assert!(project(&catalog, &complete()).completeness.ready_to_finalize);

    // Three is not four.
    let mut short = complete();
    short.hobby_ids.pop();
    assert!(!project(&catalog, &short).completeness.ready_to_finalize);

    // Five is not four either.
    let mut long = complete();
    long.hobby_ids.push("stage-magic".to_string());
    assert!(!project(&catalog, &long).completeness.ready_to_finalize);

    // "pick 4 of 17" means four different cards.
    let mut duplicated = complete();
    duplicated.hobby_ids[1] = duplicated.hobby_ids[0].clone();
    assert!(!project(&catalog, &duplicated).completeness.ready_to_finalize);
}

#[test]
fn spec4_a_choice_slot_career_cannot_finalize_until_the_question_is_answered() {
    let catalog = starter_catalog();
    let mut answers = complete();
    answers.career_id = Some("influencer".to_string());
    assert!(
        !project(&catalog, &answers).completeness.ready_to_finalize,
        "the medium question is unanswered"
    );

    answers
        .career_choices
        .insert("medium".to_string(), "not-a-medium".to_string());
    assert!(
        !project(&catalog, &answers).completeness.ready_to_finalize,
        "an answer outside the option list is not an answer"
    );

    answers
        .career_choices
        .insert("medium".to_string(), "music".to_string());
    let sheet = project(&catalog, &answers);
    assert!(sheet.completeness.ready_to_finalize);
    // The chosen medium is the slot that got the ladder points; the others got none.
    let music = sheet
        .skills
        .iter()
        .find(|s| s.id == "art-craft-music")
        .unwrap();
    assert!(music.career_points > 0);
    for other in ["art-craft-video", "art-craft-writing"] {
        assert_eq!(
            sheet.skills.iter().find(|s| s.id == other).unwrap().career_points,
            0,
            "{other}"
        );
    }
}

#[test]
fn spec2_the_number_of_edu_checks_must_match_the_age_band() {
    let catalog = starter_catalog();
    // The Prime fixture supplies one check. Ageing into the 50s demands three.
    let mut fifties = complete();
    fifties.age = Some(55);
    fifties.age_band = Some(AgeBand::Seasoned);
    assert!(!project(&catalog, &fifties).completeness.ready_to_finalize);

    if let Some(dice) = fifties.dice.as_mut() {
        dice.edu_checks = vec![
            coc_phoenix::model::EduCheck { d100: 45, gain: 6 },
            coc_phoenix::model::EduCheck { d100: 46, gain: 6 },
            coc_phoenix::model::EduCheck { d100: 47, gain: 6 },
        ];
    }
    assert!(project(&catalog, &fifties).completeness.ready_to_finalize);

    // Young grants none at all, so even one is wrong.
    let mut young = complete();
    young.age = Some(17);
    young.age_band = Some(AgeBand::Young);
    if let Some(dice) = young.dice.as_mut() {
        dice.luck = Some(coc_phoenix::model::LuckAnswer {
            rolls: vec![55, 60],
            kept: Some(60),
        });
    }
    assert!(!project(&catalog, &young).completeness.ready_to_finalize);
    if let Some(dice) = young.dice.as_mut() {
        dice.edu_checks.clear();
    }
    assert!(project(&catalog, &young).completeness.ready_to_finalize);
}

#[test]
fn spec10_ages_outside_fifteen_to_eighty_nine_cannot_finalize() {
    let catalog = starter_catalog();
    for age in [14, 90, 120] {
        let mut answers = complete();
        answers.age = Some(age);
        answers.age_band = Some(AgeBand::from_age(age));
        assert!(
            !project(&catalog, &answers).completeness.ready_to_finalize,
            "age {age} is out of scope for v1 (§10)"
        );
    }
    for age in [15, 89] {
        let mut answers = complete();
        answers.age = Some(age);
        answers.age_band = Some(AgeBand::from_age(age));
        if let Some(dice) = answers.dice.as_mut() {
            let effects = coc_phoenix::derived::age_effects(age);
            dice.edu_checks = (0..effects.edu_checks)
                .map(|i| coc_phoenix::model::EduCheck {
                    d100: 40 + i as i32,
                    gain: 5,
                })
                .collect();
            dice.luck = Some(coc_phoenix::model::LuckAnswer {
                rolls: vec![55; effects.luck_rolls],
                kept: Some(55),
            });
        }
        assert!(
            project(&catalog, &answers).completeness.ready_to_finalize,
            "age {age} is the edge of the supported range: {:?}",
            project(&catalog, &answers).completeness.missing
        );
    }
}

#[test]
fn spec1_a_name_is_required_but_pronouns_never_are() {
    let catalog = starter_catalog();
    let mut nameless = complete();
    nameless.name = Some("   ".to_string());
    assert!(!project(&catalog, &nameless).completeness.ready_to_finalize);

    // "Sex/gender has zero mechanical effect ... no dropdown, no validation, no requirement."
    let mut anonymous = complete();
    anonymous.pronouns = None;
    let with_pronouns = project(&catalog, &complete());
    let without = project(&catalog, &anonymous);
    assert!(without.completeness.ready_to_finalize);
    assert_eq!(
        without.skills, with_pronouns.skills,
        "pronouns must not move a single number"
    );
    assert_eq!(without.derived, with_pronouns.derived);
}

#[test]
fn spec6_best_other_never_answers_with_edu() {
    // §4.1: "x2 of the player's *highest remaining* characteristic" — EDU already has its own
    // term, so it can never also be the "best other", even at 90 against a field of 15s.
    let catalog = starter_catalog();
    let answers = draft(
        "Wren Ashcombe-Lyle",
        27,
        chars8([15, 20, 40, 25, 30, 35, 45, 90]),
        Some("jack-of-all-trades"),
        &["bookworm", "poker-nights", "tinkering", "social-butterfly"],
        &[(20, 5)],
        &[55],
    );
    let a = allocate(&catalog, &answers);
    assert_eq!(a.best_other, Some(Characteristic::Int), "INT 45 is the highest non-EDU stat");
    assert_eq!(a.career_pool, 90 * 2 + 45 * 2);
}
