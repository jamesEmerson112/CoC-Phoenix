//! Invented fixtures shared by the spec suite.
//!
//! The "orchard estate" content pack below is entirely made up for testing. Every skill base is
//! **0** so that a projected skill value is exactly the number of points the allocator handed it
//! — which keeps the §6 arithmetic assertions free of unrelated content data.

#![allow(dead_code)]

use coc_phoenix::content::{
    Career, CareerBonuses, Catalog, ChoiceSlot, CreditRatingRange, FormulaCharacteristic,
    FormulaTerm, Hobby, Skill, SkillBase, SkillSlot,
};
use coc_phoenix::model::{
    AgeBand, Answers, Characteristic, Characteristics, DiceAnswers, DiceSource, EduCheck,
    LuckAnswer,
};
use coc_phoenix::projection::Sheet;

// ---------------------------------------------------------------------------------------------
// builders
// ---------------------------------------------------------------------------------------------

pub fn skill(id: &str, base: i32) -> Skill {
    Skill {
        id: id.to_string(),
        name: id.to_string(),
        base: SkillBase::Fixed { value: base },
        affinity: None,
        credit_rating: false,
    }
}

pub fn skill_aff(id: &str, base: i32, affinity: Characteristic) -> Skill {
    Skill {
        affinity: Some(affinity),
        ..skill(id, base)
    }
}

pub fn cr_skill(id: &str) -> Skill {
    Skill {
        credit_rating: true,
        ..skill(id, 0)
    }
}

pub fn term(c: FormulaCharacteristic, multiplier: i32) -> FormulaTerm {
    FormulaTerm {
        characteristic: c,
        multiplier,
    }
}

pub fn career(
    id: &str,
    terms: Vec<FormulaTerm>,
    signature: Option<&str>,
    supporting: &[&str],
    cr_min: i32,
    cr_max: i32,
) -> Career {
    Career {
        id: id.to_string(),
        name: id.to_string(),
        blurb: String::new(),
        formula_terms: terms,
        signature_skill: signature.map(str::to_string),
        supporting_skills: supporting
            .iter()
            .map(|s| SkillSlot::Fixed(s.to_string()))
            .collect(),
        choice_slots: Vec::<ChoiceSlot>::new(),
        credit_rating: CreditRatingRange {
            min: cr_min,
            max: cr_max,
        },
        bonuses: CareerBonuses::default(),
        slots_from_hobbies: false,
    }
}

pub fn hobby(id: &str, primary: &str, secondary: &str) -> Hobby {
    Hobby {
        id: id.to_string(),
        name: id.to_string(),
        blurb: String::new(),
        primary_skill: primary.to_string(),
        secondary_skill: secondary.to_string(),
    }
}

pub fn chars8(v: [i32; 8]) -> Characteristics {
    Characteristics {
        strength: v[0],
        con: v[1],
        siz: v[2],
        dex: v[3],
        app: v[4],
        pow: v[5],
        intelligence: v[6],
        edu: v[7],
    }
}

/// An invented answers record (§9 shape). `edu_checks` is `(d100, gain)` pairs.
pub fn draft(
    name: &str,
    age: i32,
    c: Characteristics,
    career_id: Option<&str>,
    hobbies: &[&str],
    edu_checks: &[(i32, i32)],
    luck_rolls: &[i32],
) -> Answers {
    Answers {
        name: Some(name.to_string()),
        pronouns: None,
        age: Some(age),
        age_band: Some(AgeBand::from_age(age)),
        dice: Some(DiceAnswers {
            source: DiceSource::Server,
            characteristics: Some(c),
            edu_checks: edu_checks
                .iter()
                .map(|&(d100, gain)| EduCheck { d100, gain })
                .collect(),
            luck: Some(LuckAnswer {
                rolls: luck_rolls.to_vec(),
                kept: luck_rolls.iter().copied().max(),
            }),
            ..Default::default()
        }),
        career_id: career_id.map(str::to_string),
        hobby_ids: hobbies.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------------------------
// the invented "orchard estate" content pack
// ---------------------------------------------------------------------------------------------

/// Careers:
///   * `orchard-master`   EDU x4        — 6 ladder slots (1 signature + 5) — CR 20..60
///   * `market-carter`    DEX x2 + EDU x2 — 5 ladder slots (1 signature + 4) — CR 10..40
///   * `estate-steward`   EDU x4        — 7 ladder slots (1 signature + 6) — CR 30..70
///   * `odd-hand`         EDU x2 + best-other x2 — Jack-of-All-Trades shaped — CR 9..50
///
/// Physical affinities are deliberately sparse so the §6 age-deduction tie order is testable.
pub fn orchard_catalog() -> Catalog {
    let skills = vec![
        cr_skill("standing"),
        skill("orchard-lore", 0),
        skill("graft-work", 0),
        skill("cider-pressing", 0),
        skill("hive-tending", 0),
        skill("weather-reading", 0),
        skill("market-haggling", 0),
        skill("cart-driving", 0),
        skill_aff("fence-mending", 0, Characteristic::Str),
        skill_aff("ladder-work", 0, Characteristic::Dex),
        skill_aff("night-watch", 0, Characteristic::Con),
        skill("seed-saving", 0),
        skill("beeswax-craft", 0),
        skill("tune-humming", 0),
    ];

    let careers = vec![
        career(
            "orchard-master",
            vec![term(FormulaCharacteristic::Edu, 4)],
            Some("orchard-lore"),
            &[
                "graft-work",
                "cider-pressing",
                "hive-tending",
                "weather-reading",
                "market-haggling",
            ],
            20,
            60,
        ),
        career(
            "market-carter",
            vec![
                term(FormulaCharacteristic::Dex, 2),
                term(FormulaCharacteristic::Edu, 2),
            ],
            Some("cart-driving"),
            &[
                "ladder-work",
                "fence-mending",
                "weather-reading",
                "tune-humming",
            ],
            10,
            40,
        ),
        career(
            "estate-steward",
            vec![term(FormulaCharacteristic::Edu, 4)],
            Some("market-haggling"),
            &[
                "orchard-lore",
                "graft-work",
                "hive-tending",
                "weather-reading",
                "night-watch",
                "ladder-work",
            ],
            30,
            70,
        ),
        Career {
            slots_from_hobbies: true,
            bonuses: CareerBonuses { luck: Some(10) },
            ..career(
                "odd-hand",
                vec![
                    term(FormulaCharacteristic::Edu, 2),
                    term(FormulaCharacteristic::BestOther, 2),
                ],
                None,
                &["graft-work", "beeswax-craft", "tune-humming", "seed-saving"],
                9,
                50,
            )
        },
    ];

    let hobbies = vec![
        hobby("kite-days", "tune-humming", "weather-reading"),
        hobby("star-nights", "seed-saving", "night-watch"),
        hobby("cider-tasting", "beeswax-craft", "hive-tending"),
        hobby("wall-scrambles", "ladder-work", "fence-mending"),
    ];

    Catalog::new("orchard-fixture", skills, careers, hobbies)
}

// ---------------------------------------------------------------------------------------------
// sheet accessors
// ---------------------------------------------------------------------------------------------

pub fn value(sheet: &Sheet, id: &str) -> i32 {
    sheet
        .skills
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("no skill {id} on the sheet"))
        .value
}

pub fn base(sheet: &Sheet, id: &str) -> i32 {
    sheet
        .skills
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("no skill {id} on the sheet"))
        .base
}

pub fn career_pts(sheet: &Sheet, id: &str) -> i32 {
    sheet
        .skills
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("no skill {id} on the sheet"))
        .career_points
}

pub fn personal_pts(sheet: &Sheet, id: &str) -> i32 {
    sheet
        .skills
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("no skill {id} on the sheet"))
        .personal_points
}

pub fn characteristic(sheet: &Sheet, id: &str) -> i32 {
    sheet
        .characteristics
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("no characteristic {id}"))
        .value
}

/// Assert that no skill outside `touched` received a single point, and that its final value is
/// still exactly its base. This is what makes a "golden sheet" complete rather than partial.
pub fn assert_only_these_were_touched(sheet: &Sheet, touched: &[&str]) {
    for s in &sheet.skills {
        if touched.contains(&s.id.as_str()) {
            continue;
        }
        assert_eq!(s.career_points, 0, "{} should have no career points", s.id);
        assert_eq!(
            s.personal_points, 0,
            "{} should have no personal points",
            s.id
        );
        assert_eq!(s.value, s.base, "{} should still be at its base", s.id);
        assert!(!s.capped, "{} should not be capped", s.id);
    }
}
