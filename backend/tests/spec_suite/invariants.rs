//! Brute-force spec invariants, swept over the whole §4 career roster, the §5 hobby roster and
//! every §2 age band, on thousands of deterministically generated answer sets.
//!
//! Nothing here hard-codes an expected number: each assertion re-derives the spec's rule from
//! the sheet's own inputs. A failure means the allocator (§6) or the derivations (§7) broke a
//! rule the spec states in words.

use coc_phoenix::allocator::allocate;
use coc_phoenix::content::{starter_catalog, Career, Catalog, SkillBase, SkillSlot};
use coc_phoenix::derived::{age_effects, fifth, half};
use coc_phoenix::model::{Answers, Characteristic, Characteristics};
use coc_phoenix::projection::{project, Sheet};

use crate::common::draft;

use std::collections::BTreeMap;

// The §6 weights, transcribed from the spec (not imported from the implementation).
const LADDER_5: [i32; 5] = [34, 20, 18, 15, 13];
const LADDER_6: [i32; 6] = [30, 18, 16, 14, 12, 10];
const LADDER_7: [i32; 7] = [28, 16, 14, 12, 11, 10, 9];
const JACK_FIXED: [i32; 4] = [12, 10, 10, 8];
const JACK_HOBBY: [i32; 4] = [24, 16, 12, 8];
const PERSONAL_PICK: [i32; 4] = [40, 30, 20, 10];

fn ladder_for(slots: usize) -> &'static [i32] {
    match slots {
        5 => &LADDER_5,
        6 => &LADDER_6,
        7 => &LADDER_7,
        n => panic!("§6.2 defines no ladder for {n} slots"),
    }
}

/// `floor(pool * pct / 100)` — §6.3 "Round down per skill".
fn share(pool: i32, pct: i32) -> i32 {
    pool * pct / 100
}

/// A tiny deterministic generator, so a failure is always reproducible from its index.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Lcg {
        Lcg(seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1))
    }
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 17
    }
    fn pick(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// §3: 3D6x5 stats are multiples of 5 in 15..=90; (2D6+6)x5 stats are multiples of 5 in 40..=90.
fn random_characteristics(rng: &mut Lcg) -> Characteristics {
    let mut chars = Characteristics::default();
    for c in Characteristic::ALL {
        let (lo, hi) = c.declared_range();
        let steps = ((hi - lo) / 5 + 1) as usize;
        chars.set(c, lo + 5 * rng.pick(steps) as i32);
    }
    chars
}

fn random_hobbies(rng: &mut Lcg, catalog: &Catalog) -> Vec<String> {
    let mut ids: Vec<String> = catalog.hobbies().iter().map(|h| h.id.clone()).collect();
    let mut picked = Vec::with_capacity(4);
    for _ in 0..4 {
        picked.push(ids.remove(rng.pick(ids.len())));
    }
    picked
}

/// A complete, finalizable answer set for `career`, at `age`.
fn random_answers(rng: &mut Lcg, catalog: &Catalog, career: &Career, age: i32) -> Answers {
    let effects = age_effects(age);
    let checks: Vec<(i32, i32)> = (0..effects.edu_checks)
        .map(|_| (1 + rng.pick(100) as i32, 1 + rng.pick(10) as i32))
        .collect();
    let luck: Vec<i32> = (0..effects.luck_rolls)
        .map(|_| 15 + 5 * rng.pick(16) as i32)
        .collect();

    let hobbies = random_hobbies(rng, catalog);
    let hobby_refs: Vec<&str> = hobbies.iter().map(String::as_str).collect();

    let mut answers = draft(
        "Wren Ashcombe-Lyle",
        age,
        random_characteristics(rng),
        Some(&career.id),
        &hobby_refs,
        &checks,
        &luck,
    );
    for slot in &career.choice_slots {
        let option = &slot.options[rng.pick(slot.options.len())];
        answers
            .career_choices
            .insert(slot.id.clone(), option.id.clone());
    }
    answers
}

/// Every skill id the career pays into, in ladder order (signature first).
fn resolved_career_slots(career: &Career, answers: &Answers) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(sig) = &career.signature_skill {
        out.push(sig.clone());
    }
    for slot in &career.supporting_skills {
        match slot {
            SkillSlot::Fixed(id) => out.push(id.clone()),
            SkillSlot::Choice { choice } => {
                if let Some(answer) = answers.career_choices.get(choice) {
                    if let Some(cs) = career.choice_slots.iter().find(|c| &c.id == choice) {
                        if let Some(option) = cs.options.iter().find(|o| &o.id == answer) {
                            out.push(option.skill.clone());
                        }
                    }
                }
            }
        }
    }
    out
}

fn check_one(catalog: &Catalog, career: &Career, answers: &Answers, label: &str) {
    let sheet = project(catalog, answers);
    let allocation = allocate(catalog, answers);
    let chars = allocation.characteristics;

    assert!(
        sheet.completeness.ready_to_finalize,
        "{label}: sweep fixtures must be finalizable, missing {:?}",
        sheet.completeness.missing
    );

    // --- §3 principle 5: deterministic everything ------------------------------------------
    assert_eq!(project(catalog, answers), sheet, "{label}: not deterministic");

    // --- §6: pools ---------------------------------------------------------------------------
    assert_eq!(
        sheet.pools.personal_pool,
        chars.intelligence * 2,
        "{label}: personal pool is INT x 2"
    );
    assert_eq!(
        sheet.pools.personal_pool_spent, sheet.pools.personal_pool,
        "{label}: the whole personal pool is allocated across four hobbies"
    );
    assert_eq!(
        sheet.pools.credit_rating_cost, career.credit_rating.min,
        "{label}: §6.1 Credit Rating = the career's range minimum"
    );
    assert_eq!(
        sheet.credit_rating.value, career.credit_rating.min,
        "{label}: the CR skill carries the CR minimum"
    );
    // A pool too small to buy the career's own Credit Rating minimum is a scenario the spec
    // never describes, so the ledger assertions below are only meaningful above that line.
    let pool_covers_cr = sheet.pools.career_pool >= career.credit_rating.min;
    if pool_covers_cr {
        assert_eq!(
            sheet.pools.career_pool_spent, sheet.pools.career_pool,
            "{label}: §6 spends the career pool down to the point (CR + ladder + remainder)"
        );
    }

    // --- §6: career pool = the formula on FINAL (post-age) characteristics -------------------
    let expected_pool: i32 = career
        .formula_terms
        .iter()
        .map(|t| {
            let c = t
                .characteristic
                .as_characteristic()
                .or(allocation.best_other)
                .expect("best other must be resolved for a best-other term");
            chars.get(c) * t.multiplier
        })
        .sum();
    assert_eq!(
        sheet.pools.career_pool, expected_pool,
        "{label}: career pool must be the §4 formula on post-age values"
    );

    // --- §6.1-6.3 / §4.1: re-derive the whole career ledger from the spec's weights ----------
    let remainder = (sheet.pools.career_pool - career.credit_rating.min).max(0);
    let slots = resolved_career_slots(career, answers);
    let mut expected: BTreeMap<String, i32> = BTreeMap::new();
    let mut spent = 0;

    if career.slots_from_hobbies {
        // §4.1: four fixed everyday skills take 40% as 12/10/10/8...
        for (i, id) in slots.iter().enumerate() {
            let points = share(remainder, JACK_FIXED[i]);
            *expected.entry(id.clone()).or_default() += points;
            spent += points;
        }
        // ...and the player's own hobby primaries take the other 60% as 24/16/12/8.
        for (i, hobby) in sheet.hobbies.iter().enumerate() {
            let points = share(remainder, JACK_HOBBY[i]);
            *expected.entry(hobby.primary_skill.clone()).or_default() += points;
            spent += points;
        }
        // "Remainder to the first-pick primary."
        *expected
            .entry(sheet.hobbies[0].primary_skill.clone())
            .or_default() += remainder - spent;
    } else {
        // §6.2: ladder the remainder, descending, signature first.
        let pcts = ladder_for(slots.len());
        for (i, id) in slots.iter().enumerate() {
            let points = share(remainder, pcts[i]);
            *expected.entry(id.clone()).or_default() += points;
            spent += points;
        }
        // §6.3: "all rounding remainder goes to the signature skill".
        *expected.entry(slots[0].clone()).or_default() += remainder - spent;

        // ...and the result descends.
        let ladder: Vec<i32> = slots.iter().map(|id| expected[id]).collect();
        for pair in ladder.windows(2) {
            assert!(pair[0] >= pair[1], "{label}: §6.2 ladders descend, got {ladder:?}");
        }
    }

    if pool_covers_cr {
        for (id, points) in &expected {
            let got = sheet
                .skills
                .iter()
                .find(|s| &s.id == id)
                .map(|s| s.career_points)
                .unwrap_or(0);
            assert_eq!(got, *points, "{label}: career points for {id}");
        }
    }

    // --- §6: re-derive the personal pool the same way ------------------------------------------
    let pool = sheet.pools.personal_pool;
    let mut expected_personal: BTreeMap<String, i32> = BTreeMap::new();
    let mut personal_spent = 0;
    for (i, hobby) in sheet.hobbies.iter().enumerate() {
        // 1. split by pick order 40/30/20/10
        let hobby_share = share(pool, PERSONAL_PICK[i]);
        // 2. 70/30 within the hobby, rounded down, remainder to the primary
        let secondary = share(hobby_share, 30);
        let primary = hobby_share - secondary;
        *expected_personal
            .entry(hobby.primary_skill.clone())
            .or_default() += primary;
        *expected_personal
            .entry(hobby.secondary_skill.clone())
            .or_default() += secondary;
        personal_spent += hobby_share;
    }
    // 3. global rounding remainder to the first pick's primary
    *expected_personal
        .entry(sheet.hobbies[0].primary_skill.clone())
        .or_default() += pool - personal_spent;

    for (id, points) in &expected_personal {
        let got = sheet
            .skills
            .iter()
            .find(|s| &s.id == id)
            .map(|s| s.personal_points)
            .unwrap_or(0);
        assert_eq!(got, *points, "{label}: personal points for {id}");
    }

    // --- §6: nothing outside the career and the four hobbies is paid a single point ----------
    let mut payable: Vec<String> = if career.slots_from_hobbies {
        let mut v = resolved_career_slots(career, answers);
        v.extend(sheet.hobbies.iter().map(|h| h.primary_skill.clone()));
        v
    } else {
        resolved_career_slots(career, answers)
    };
    payable.extend(
        sheet
            .hobbies
            .iter()
            .flat_map(|h| [h.primary_skill.clone(), h.secondary_skill.clone()]),
    );
    for s in &sheet.skills {
        if !payable.contains(&s.id) {
            assert_eq!(s.career_points, 0, "{label}: {} got stray career points", s.id);
            assert_eq!(s.personal_points, 0, "{label}: {} got stray hobby points", s.id);
        }
    }

    // --- §6: the 75 cap, and no redistribution ------------------------------------------------
    for s in &sheet.skills {
        let uncapped = s.base + s.career_points + s.personal_points;
        let expected = if s.base >= 75 { s.base } else { uncapped.min(75) };
        assert_eq!(s.value, expected, "{label}: {} final value", s.id);
        assert_eq!(
            s.points_lost_to_cap,
            (uncapped - s.value).max(0),
            "{label}: {} points lost to the cap",
            s.id
        );
    }

    // --- §7: every derived value, recomputed from the sheet's own characteristics -----------
    let get = |id: &str| {
        sheet
            .characteristics
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .value
    };
    let (str_v, con, siz, dex, app, pow, int, edu) = (
        get("str"),
        get("con"),
        get("siz"),
        get("dex"),
        get("app"),
        get("pow"),
        get("int"),
        get("edu"),
    );
    let _ = (app, int);
    assert_eq!(sheet.derived.hit_points, (con + siz) / 10, "{label}: HP");
    assert_eq!(sheet.derived.magic_points, pow / 5, "{label}: MP");
    assert_eq!(sheet.derived.sanity, pow.min(99), "{label}: SAN");
    assert_eq!(sheet.derived.dodge_base, dex / 2, "{label}: Dodge base");
    assert_eq!(sheet.derived.language_own_base, edu, "{label}: Language (Own) base");

    let expected_build = match str_v + siz {
        s if s < 65 => -2,
        s if s < 85 => -1,
        s if s < 125 => 0,
        s if s < 165 => 1,
        _ => 2,
    };
    assert_eq!(sheet.derived.build, expected_build, "{label}: Build");

    let expected_move_base = if dex > siz && str_v > siz {
        9
    } else if dex >= siz || str_v >= siz {
        8
    } else {
        7
    };
    assert_eq!(sheet.derived.move_base, expected_move_base, "{label}: MOV base");
    let age = answers.effective_age().unwrap();
    assert_eq!(
        sheet.derived.move_age_penalty,
        age_effects(age).mov_penalty,
        "{label}: MOV age penalty"
    );
    assert_eq!(
        sheet.derived.move_rate,
        expected_move_base - age_effects(age).mov_penalty,
        "{label}: MOV"
    );

    // --- §7: half and fifth everywhere --------------------------------------------------------
    for c in &sheet.characteristics {
        assert_eq!((c.half, c.fifth), (half(c.value), fifth(c.value)), "{label}: {}", c.id);
    }
    for s in &sheet.skills {
        assert_eq!((s.half, s.fifth), (half(s.value), fifth(s.value)), "{label}: {}", s.id);
        // Skill bases are content data except for the two the spec fixes itself.
        match catalog.skill(&s.id).unwrap().base {
            SkillBase::DexHalf => assert_eq!(s.base, dex / 2, "{label}: Dodge base"),
            SkillBase::Edu => assert_eq!(s.base, edu, "{label}: Language (Own) base"),
            SkillBase::Fixed { value } => assert_eq!(s.base, value),
        }
    }

    // --- §2: the age costs actually charged -----------------------------------------------------
    let raw = allocation.raw_characteristics;
    let effects = age_effects(age);
    // The spec states the APP cost but never states a floor, so when the roll is smaller than
    // the cost the only thing it can be held to is "charged everything there was".
    if raw.app > effects.app_reduction {
        assert_eq!(
            raw.app - chars.app,
            effects.app_reduction,
            "{label}: APP reduction (raw APP {})",
            raw.app
        );
    } else {
        assert!(
            raw.app - chars.app <= effects.app_reduction && chars.app >= 0,
            "{label}: APP {} cannot pay more than the {} it owes",
            raw.app,
            effects.app_reduction
        );
    }
    if effects.physical_deduction > 0 && age < 80 {
        let charged = (raw.strength + raw.con + raw.dex) - (str_v + con + dex);
        assert_eq!(
            charged, effects.physical_deduction,
            "{label}: physical deduction total"
        );
        assert_eq!(raw.siz, siz, "{label}: SIZ is not a 'physical' for age purposes");
    }
    if effects.young_deduction > 0 {
        let charged = (raw.strength + raw.siz) - (str_v + siz);
        assert_eq!(charged, 5, "{label}: the Young cost is 5 across STR and SIZ");
        assert_eq!(raw.con, con, "{label}: Young pays nothing from CON");
        assert_eq!(raw.dex, dex, "{label}: Young pays nothing from DEX");
        assert_eq!(raw.edu - 5, edu, "{label}: Young EDU -5 and no checks to win it back");
    }

    // --- §7 / §4.1: Luck ------------------------------------------------------------------------
    let rolls = &answers.dice.as_ref().unwrap().luck.as_ref().unwrap().rolls;
    let base_luck = if effects.band == coc_phoenix::model::AgeBand::Young {
        *rolls.iter().max().unwrap()
    } else {
        rolls[0]
    };
    let perk = career.bonuses.luck.unwrap_or(0);
    assert_eq!(
        sheet.derived.luck,
        Some((base_luck + perk).min(99)),
        "{label}: Luck"
    );
}

fn sheet_ids(sheet: &Sheet) -> Vec<&str> {
    sheet.skills.iter().map(|s| s.id.as_str()).collect()
}

#[test]
fn spec_invariants_hold_across_the_whole_roster_and_every_age_band() {
    let catalog = starter_catalog();
    // One representative age per §2 sub-column, plus both band edges.
    let ages = [
        15, 17, 19, 20, 27, 39, 40, 48, 50, 55, 59, 60, 65, 69, 70, 72, 79, 80, 85, 89,
    ];

    let mut checked = 0usize;
    for (career_index, career) in catalog.careers().iter().enumerate() {
        for (age_index, &age) in ages.iter().enumerate() {
            for draw in 0..14u64 {
                let mut rng = Lcg::new(
                    (career_index as u64) * 10_007 + (age_index as u64) * 101 + draw + 1,
                );
                let answers = random_answers(&mut rng, &catalog, career, age);
                check_one(
                    &catalog,
                    career,
                    &answers,
                    &format!("{} @ {age} draw {draw}", career.id),
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 1_000, "the sweep should be broad, ran {checked}");
}

#[test]
fn spec6_the_sheet_lists_every_catalogue_skill_except_credit_rating() {
    // §6 gives Credit Rating its own block (§2 step 6 lists it separately from the skill list).
    let catalog = starter_catalog();
    let sheet = project(&catalog, &Answers::default());
    let ids = sheet_ids(&sheet);
    assert!(!ids.contains(&"credit-rating"));
    assert_eq!(ids.len(), catalog.skills().len() - 1);
}

