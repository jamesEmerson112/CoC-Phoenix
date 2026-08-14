//! The deterministic allocator (spec § 6).
//!
//! A pure function from (content catalogue, answers) to an allocation ledger. No database, no
//! RNG, no clock: the same answers always produce the same numbers, which is what makes the
//! sheet a projection rather than stored state.

use crate::content::{Career, Catalog, FormulaTerm, Hobby};
use crate::derived::{age_effects, AgeEffects};
use crate::dice::{apply_edu_checks, kept_luck};
use crate::model::{AgeBand, Answers, Characteristic, Characteristics};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Spec § 6: final creation value is capped at 75 (pending rulebook verification, § 11).
pub const SKILL_CAP: i32 = 75;
/// Luck never exceeds this at creation (spec § 4.1).
pub const LUCK_CAP: i32 = 99;
/// A characteristic is never driven below this by age deductions.
pub const MIN_CHARACTERISTIC: i32 = 1;

/// Career ladders, descending, signature first (spec § 6.2).
pub const LADDER_5: [i32; 5] = [34, 20, 18, 15, 13];
pub const LADDER_6: [i32; 6] = [30, 18, 16, 14, 12, 10];
pub const LADDER_7: [i32; 7] = [28, 16, 14, 12, 11, 10, 9];

/// Jack of All Trades: fixed everyday skills take 40% of the post-CR pool.
pub const JACK_FIXED_PCT: [i32; 4] = [12, 10, 10, 8];
/// ...and the player's own hobby primaries take the other 60%, by pick order.
pub const JACK_HOBBY_PCT: [i32; 4] = [24, 16, 12, 8];

/// Personal interest pool split by hobby pick order (spec § 6, "Personal pool").
pub const PERSONAL_PICK_PCT: [i32; 4] = [40, 30, 20, 10];
/// Within one hobby: primary / secondary.
pub const HOBBY_PRIMARY_PCT: i32 = 70;
pub const HOBBY_SECONDARY_PCT: i32 = 30;

/// The ladder for a given slot count. Counts outside 5-7 fall back to an even split, with the
/// rounding remainder going to the signature skill exactly as the spec'd ladders do.
///
/// The fallback always sums to exactly 100: `100 / n` does not divide evenly for most counts
/// (3 -> 99, 8 -> 96), and any shortfall here would be career points that simply vanish from the
/// ledger, so the leftover percent is handed to slot 0 (the signature) up front.
pub fn ladder(slots: usize) -> Vec<i32> {
    match slots {
        0 => Vec::new(),
        5 => LADDER_5.to_vec(),
        6 => LADDER_6.to_vec(),
        7 => LADDER_7.to_vec(),
        n => {
            let each = 100 / n as i32;
            let mut pcts = vec![each; n];
            pcts[0] += 100 - each * n as i32;
            pcts
        }
    }
}

/// One age-driven change to a characteristic, kept for display ("costs are flavor", § 3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeAdjustment {
    pub characteristic: Characteristic,
    /// Negative for a deduction, positive for an EDU life-experience gain.
    pub delta: i32,
    pub reason: String,
}

/// The complete allocation ledger for one answer set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Allocation {
    /// Exactly what the dice said, before age.
    pub raw_characteristics: Characteristics,
    /// Post-age, post-EDU-check values: what the sheet shows.
    pub characteristics: Characteristics,
    pub adjustments: Vec<AgeAdjustment>,
    /// Which EDU checks actually improved EDU, in order.
    pub edu_checks_applied: Vec<bool>,
    /// Career (occupation) points per skill id, including Credit Rating.
    pub career_points: BTreeMap<String, i32>,
    /// Personal interest points per skill id.
    pub personal_points: BTreeMap<String, i32>,
    pub credit_rating: i32,
    pub career_pool: i32,
    pub career_pool_spent: i32,
    /// How much of the Credit Rating minimum the career pool could not pay for (spec § 6 rule 1
    /// assumes it always can). Zero for every career the roster can actually produce at a
    /// plausible age; non-zero only when age deductions have gutted the formula's
    /// characteristics. The ledger is then
    /// `sum(career_points) == career_pool_spent == career_pool + career_pool_shortfall`.
    pub career_pool_shortfall: i32,
    pub personal_pool: i32,
    pub personal_pool_spent: i32,
    /// Which characteristic answered Jack of All Trades' "best other" term.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_other: Option<Characteristic>,
    /// Final Luck, including the Jack of All Trades perk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub luck: Option<i32>,
    /// The career perk contribution to Luck (0 for every career but one).
    pub luck_bonus: i32,
}

/// Every characteristic Jack of All Trades' "best other" term may pick, in tie-break order.
const BEST_OTHER_CANDIDATES: [Characteristic; 7] = [
    Characteristic::Str,
    Characteristic::Con,
    Characteristic::Siz,
    Characteristic::Dex,
    Characteristic::App,
    Characteristic::Pow,
    Characteristic::Int,
];

/// Spec § 6: ties for the physical age deduction break CON -> STR -> DEX.
const PHYSICAL_DEDUCTION_ORDER: [Characteristic; 3] = [
    Characteristic::Con,
    Characteristic::Str,
    Characteristic::Dex,
];

/// Young characters pay from STR and SIZ instead; SIZ leads the tie-break because no career
/// formula in the roster uses it (resolution of a spec gap, see backend README).
const YOUNG_DEDUCTION_ORDER: [Characteristic; 2] = [Characteristic::Siz, Characteristic::Str];

/// The highest characteristic other than EDU, ties broken by the fixed candidate order.
pub fn best_other(chars: &Characteristics) -> Characteristic {
    let mut best = BEST_OTHER_CANDIDATES[0];
    for c in BEST_OTHER_CANDIDATES {
        if chars.get(c) > chars.get(best) {
            best = c;
        }
    }
    best
}

/// Evaluate a career points formula against a characteristic set.
pub fn evaluate_formula(
    terms: &[FormulaTerm],
    chars: &Characteristics,
    best_other_pick: Characteristic,
) -> i32 {
    terms
        .iter()
        .map(|term| {
            let c = term
                .characteristic
                .as_characteristic()
                .unwrap_or(best_other_pick);
            chars.get(c) * term.multiplier
        })
        .sum()
}

/// The career pool a career would grant these characteristics — used to sort careers by fit.
pub fn career_pool_for(career: &Career, chars: &Characteristics) -> i32 {
    evaluate_formula(&career.formula_terms, chars, best_other(chars))
}

/// Resolve a career's skill slots to concrete skill ids in ladder order (signature first).
/// Unanswered choice slots resolve to `None` and are left unallocated.
pub fn career_slots(career: &Career, answers: &Answers) -> Vec<Option<String>> {
    let mut slots: Vec<Option<String>> = Vec::new();
    if let Some(sig) = &career.signature_skill {
        slots.push(Some(sig.clone()));
    }
    for slot in &career.supporting_skills {
        slots.push(slot.resolve(career, &answers.career_choices));
    }
    slots
}

/// The hobbies a player picked, in pick order, capped at four. Unknown ids keep their slot but
/// resolve to `None` so pick-order weighting is not silently reshuffled.
fn picked_hobbies<'a>(catalog: &'a Catalog, answers: &Answers) -> Vec<Option<&'a Hobby>> {
    answers
        .hobby_ids
        .iter()
        .take(PERSONAL_PICK_PCT.len())
        .map(|id| catalog.hobby(id))
        .collect()
}

/// How much a career "needs" a characteristic — used to pick the cheapest place to take an age
/// deduction from (spec § 6, "Age deductions").
///
/// Formula terms dominate (weighted x10); each career skill that leans on the characteristic
/// adds one point.
fn need_score(
    catalog: &Catalog,
    career: Option<&Career>,
    answers: &Answers,
    best_other_hint: Characteristic,
    c: Characteristic,
) -> i32 {
    let Some(career) = career else {
        return 0;
    };
    let formula: i32 = career
        .formula_terms
        .iter()
        .filter(|t| {
            t.characteristic
                .as_characteristic()
                .unwrap_or(best_other_hint)
                == c
        })
        .map(|t| t.multiplier * 10)
        .sum();

    let mut skill_ids: Vec<String> = career_slots(career, answers).into_iter().flatten().collect();
    if career.slots_from_hobbies {
        for hobby in picked_hobbies(catalog, answers).into_iter().flatten() {
            skill_ids.push(hobby.primary_skill.clone());
        }
    }
    let skills: i32 = skill_ids
        .iter()
        .filter(|id| {
            catalog
                .skill(id)
                .and_then(|s| s.affinity)
                .is_some_and(|a| a == c)
        })
        .count() as i32;

    formula + skills
}

/// Order candidate characteristics cheapest-first for an age deduction. Stable: ties fall back
/// to the fixed order the candidates were given in.
fn deduction_order(
    candidates: &[Characteristic],
    catalog: &Catalog,
    career: Option<&Career>,
    answers: &Answers,
    best_other_hint: Characteristic,
) -> Vec<Characteristic> {
    let mut scored: Vec<(i32, usize, Characteristic)> = candidates
        .iter()
        .enumerate()
        .map(|(i, &c)| (need_score(catalog, career, answers, best_other_hint, c), i, c))
        .collect();
    scored.sort_by_key(|&(score, index, _)| (score, index));
    scored.into_iter().map(|(_, _, c)| c).collect()
}

/// Subtract `plan` amounts in order, cascading any overflow into the following targets so a
/// characteristic never falls below [`MIN_CHARACTERISTIC`].
fn apply_deduction(
    chars: &mut Characteristics,
    order: &[Characteristic],
    plan: &[i32],
    reason: &str,
    log: &mut Vec<AgeAdjustment>,
) {
    let mut carried = 0;
    for (i, &c) in order.iter().enumerate() {
        let wanted = plan.get(i).copied().unwrap_or(0) + carried;
        if wanted <= 0 {
            carried = 0;
            continue;
        }
        let available = (chars.get(c) - MIN_CHARACTERISTIC).max(0);
        let taken = wanted.min(available);
        carried = wanted - taken;
        if taken > 0 {
            chars.add(c, -taken);
            log.push(AgeAdjustment {
                characteristic: c,
                delta: -taken,
                reason: reason.to_string(),
            });
        }
    }
}

/// Split a pool by percentage, floored.
fn share(pool: i32, pct: i32) -> i32 {
    if pool <= 0 || pct <= 0 {
        0
    } else {
        pool * pct / 100
    }
}

/// Run the whole allocation (spec § 6). Safe to call on a half-answered draft.
pub fn allocate(catalog: &Catalog, answers: &Answers) -> Allocation {
    let age = answers.effective_age().unwrap_or(AgeBand::Prime.default_age());
    let effects: AgeEffects = age_effects(age);
    let band = effects.band;
    let dice = answers.dice.clone().unwrap_or_default();
    let raw = dice.characteristics.unwrap_or_default();
    let has_characteristics = dice.characteristics.is_some();
    let career = answers
        .career_id
        .as_deref()
        .and_then(|id| catalog.career(id));

    let mut chars = raw;
    let mut adjustments: Vec<AgeAdjustment> = Vec::new();
    let mut edu_checks_applied: Vec<bool> = Vec::new();

    if has_characteristics {
        // 1. Flat EDU cost (Young only).
        if effects.edu_penalty > 0 {
            apply_deduction(
                &mut chars,
                &[Characteristic::Edu],
                &[effects.edu_penalty],
                "age: EDU",
                &mut adjustments,
            );
        }
        // 2. APP reduction (Seasoned and Venerable).
        if effects.app_reduction > 0 {
            apply_deduction(
                &mut chars,
                &[Characteristic::App],
                &[effects.app_reduction],
                "age: APP",
                &mut adjustments,
            );
        }
        // The "best other" term is pinned before deductions so the choice cannot chase its own
        // tail; the pool below re-picks it on the final values.
        let hint = best_other(&chars);
        // 3. Physical deduction, taken from whatever the career needs least.
        if effects.physical_deduction > 0 {
            let order = deduction_order(&PHYSICAL_DEDUCTION_ORDER, catalog, career, answers, hint);
            apply_deduction(
                &mut chars,
                &order,
                &[effects.physical_deduction],
                "age: physical",
                &mut adjustments,
            );
        }
        // 4. Young pays 5 split across STR and SIZ: 3 from the least-needed, 2 from the other.
        if effects.young_deduction > 0 {
            let order = deduction_order(&YOUNG_DEDUCTION_ORDER, catalog, career, answers, hint);
            let first = effects.young_deduction - effects.young_deduction / 2;
            let second = effects.young_deduction / 2;
            apply_deduction(
                &mut chars,
                &order,
                &[first, second],
                "age: youth",
                &mut adjustments,
            );
        }
        // 5. EDU life-experience checks, strictly sequential.
        //
        // Only the checks the band actually grants are applied. A player who rolls four checks
        // as a Venerable and then goes back and answers "young" keeps the stored rolls (nothing
        // is destroyed by going back, § 2) but the projection must not spend them.
        let granted = effects.edu_checks;
        let checks = &dice.edu_checks[..dice.edu_checks.len().min(granted)];
        if !checks.is_empty() {
            let before = chars.edu;
            let (edu, applied) = apply_edu_checks(before, checks);
            edu_checks_applied = applied;
            if edu != before {
                chars.edu = edu;
                adjustments.push(AgeAdjustment {
                    characteristic: Characteristic::Edu,
                    delta: edu - before,
                    reason: "EDU life experience".to_string(),
                });
            }
        }
    }

    let best_other_pick = career.map(|_| best_other(&chars));

    let mut career_points: BTreeMap<String, i32> = BTreeMap::new();
    let mut credit_rating = 0;
    let mut career_pool = 0;
    let mut career_spent = 0;
    let mut career_shortfall = 0;

    if let (Some(career), true) = (career, has_characteristics) {
        career_pool = evaluate_formula(
            &career.formula_terms,
            &chars,
            best_other_pick.unwrap_or(Characteristic::Str),
        );
        // Spec § 6 rule 1: Credit Rating is the career's range minimum, *paid out of the career
        // pool*. A pool too small to pay for it is a case the spec never describes; the standing
        // is still granted (it is what the profession pays), but the difference is reported as
        // `career_pool_shortfall` instead of being hidden by a `max(0)`. Without it the sheet
        // showed `careerPoolSpent 9` against `careerPool 8` with no explanation and the ledger
        // silently stopped reconciling.
        credit_rating = career.credit_rating.min;
        career_shortfall = (credit_rating - career_pool).max(0);
        let remainder = (career_pool - credit_rating).max(0);
        let spent = if career.slots_from_hobbies {
            allocate_jack(catalog, career, answers, remainder, &mut career_points)
        } else {
            allocate_ladder(career, answers, remainder, &mut career_points)
        };
        if let Some(cr_skill) = catalog.credit_rating_skill() {
            *career_points.entry(cr_skill.id.clone()).or_default() += credit_rating;
        }
        career_spent = credit_rating + spent;
    }

    // Personal interest pool.
    let personal_pool = if has_characteristics {
        chars.intelligence * 2
    } else {
        0
    };
    let mut personal_points: BTreeMap<String, i32> = BTreeMap::new();
    let personal_spent = allocate_personal(catalog, answers, personal_pool, &mut personal_points);

    // Luck, plus the Jack of All Trades house-rule perk.
    let luck_rolls = dice.luck.as_ref().map(|l| l.rolls.clone()).unwrap_or_default();
    let luck_bonus = career.and_then(|c| c.bonuses.luck).unwrap_or(0);
    let luck = kept_luck(&luck_rolls, band).map(|k| (k + luck_bonus).min(LUCK_CAP));

    Allocation {
        raw_characteristics: raw,
        characteristics: chars,
        adjustments,
        edu_checks_applied,
        career_points,
        personal_points,
        credit_rating,
        career_pool,
        career_pool_spent: career_spent,
        career_pool_shortfall: career_shortfall,
        personal_pool,
        personal_pool_spent: personal_spent,
        best_other: best_other_pick,
        luck,
        luck_bonus: if luck.is_some() { luck_bonus } else { 0 },
    }
}

/// The ordinary career ladder (spec § 6.2-6.3). Returns the points actually spent.
fn allocate_ladder(
    career: &Career,
    answers: &Answers,
    remainder: i32,
    out: &mut BTreeMap<String, i32>,
) -> i32 {
    let slots = career_slots(career, answers);
    let pcts = ladder(slots.len());
    let allocated_weight: i32 = slots
        .iter()
        .zip(pcts.iter())
        .filter(|(slot, _)| slot.is_some())
        .map(|(_, pct)| *pct)
        .sum();
    let spendable = share(remainder, allocated_weight);

    let mut total = 0;
    for (slot, pct) in slots.iter().zip(pcts.iter()) {
        if let Some(id) = slot {
            let points = share(remainder, *pct);
            *out.entry(id.clone()).or_default() += points;
            total += points;
        }
    }
    // Rule 3: all rounding remainder goes to the signature skill.
    let leftover = spendable - total;
    if leftover > 0 {
        if let Some(target) = career
            .signature_skill
            .clone()
            .or_else(|| slots.iter().flatten().next().cloned())
        {
            *out.entry(target).or_default() += leftover;
            total += leftover;
        }
    }
    total
}

/// Jack of All Trades (spec § 4.1 + § 6): 40% to four fixed everyday skills, 60% into the
/// player's own hobby primaries by pick order, remainder to the first-pick primary.
fn allocate_jack(
    catalog: &Catalog,
    career: &Career,
    answers: &Answers,
    remainder: i32,
    out: &mut BTreeMap<String, i32>,
) -> i32 {
    let fixed: Vec<Option<String>> = career_slots(career, answers);
    let hobbies = picked_hobbies(catalog, answers);

    let mut allocated_weight = 0;
    let mut total = 0;

    for (i, slot) in fixed.iter().enumerate() {
        let Some(pct) = JACK_FIXED_PCT.get(i).copied() else {
            continue;
        };
        if let Some(id) = slot {
            allocated_weight += pct;
            let points = share(remainder, pct);
            *out.entry(id.clone()).or_default() += points;
            total += points;
        }
    }
    for (i, hobby) in hobbies.iter().enumerate() {
        let Some(pct) = JACK_HOBBY_PCT.get(i).copied() else {
            continue;
        };
        if let Some(hobby) = hobby {
            allocated_weight += pct;
            let points = share(remainder, pct);
            *out.entry(hobby.primary_skill.clone()).or_default() += points;
            total += points;
        }
    }

    let spendable = share(remainder, allocated_weight);
    let leftover = spendable - total;
    if leftover > 0 {
        let target = hobbies
            .iter()
            .flatten()
            .next()
            .map(|h| h.primary_skill.clone())
            .or_else(|| fixed.iter().flatten().next().cloned());
        if let Some(target) = target {
            *out.entry(target).or_default() += leftover;
            total += leftover;
        }
    }
    total
}

/// Personal interest points (spec § 6, "Personal pool").
fn allocate_personal(
    catalog: &Catalog,
    answers: &Answers,
    pool: i32,
    out: &mut BTreeMap<String, i32>,
) -> i32 {
    if pool <= 0 {
        return 0;
    }
    let hobbies = picked_hobbies(catalog, answers);
    let mut allocated_weight = 0;
    let mut total = 0;

    for (i, hobby) in hobbies.iter().enumerate() {
        let Some(pct) = PERSONAL_PICK_PCT.get(i).copied() else {
            continue;
        };
        let Some(hobby) = hobby else { continue };
        allocated_weight += pct;

        let hobby_share = share(pool, pct);
        let primary = share(hobby_share, HOBBY_PRIMARY_PCT);
        let secondary = share(hobby_share, HOBBY_SECONDARY_PCT);
        // Within-hobby rounding remainder goes to the primary.
        let primary = primary + (hobby_share - primary - secondary);

        *out.entry(hobby.primary_skill.clone()).or_default() += primary;
        *out.entry(hobby.secondary_skill.clone()).or_default() += secondary;
        total += hobby_share;
    }

    // Global rounding remainder goes to the first pick's primary.
    let spendable = share(pool, allocated_weight);
    let leftover = spendable - total;
    if leftover > 0 {
        if let Some(first) = hobbies.iter().flatten().next() {
            *out.entry(first.primary_skill.clone()).or_default() += leftover;
            total += leftover;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{
        CareerBonuses, CreditRatingRange, FormulaCharacteristic, Skill, SkillBase, SkillSlot,
    };
    use crate::model::{DiceAnswers, DiceSource, EduCheck, LuckAnswer};

    // ---- invented fixtures (no real-world or licensed entities) -------------------------

    fn skill(id: &str, base: i32) -> Skill {
        Skill {
            id: id.to_string(),
            name: id.to_string(),
            base: SkillBase::Fixed { value: base },
            affinity: None,
            credit_rating: false,
        }
    }

    fn skill_with_affinity(id: &str, base: i32, affinity: Characteristic) -> Skill {
        Skill {
            affinity: Some(affinity),
            ..skill(id, base)
        }
    }

    fn career(
        id: &str,
        terms: Vec<FormulaTerm>,
        signature: Option<&str>,
        supporting: Vec<&str>,
        cr: (i32, i32),
    ) -> Career {
        Career {
            id: id.to_string(),
            name: id.to_string(),
            blurb: String::new(),
            formula_terms: terms,
            signature_skill: signature.map(|s| s.to_string()),
            supporting_skills: supporting
                .into_iter()
                .map(|s| SkillSlot::Fixed(s.to_string()))
                .collect(),
            choice_slots: Vec::new(),
            credit_rating: CreditRatingRange { min: cr.0, max: cr.1 },
            bonuses: CareerBonuses::default(),
            slots_from_hobbies: false,
        }
    }

    fn hobby(id: &str, primary: &str, secondary: &str) -> Hobby {
        Hobby {
            id: id.to_string(),
            name: id.to_string(),
            blurb: String::new(),
            primary_skill: primary.to_string(),
            secondary_skill: secondary.to_string(),
        }
    }

    fn term(c: FormulaCharacteristic, multiplier: i32) -> FormulaTerm {
        FormulaTerm {
            characteristic: c,
            multiplier,
        }
    }

    fn cr_skill() -> Skill {
        Skill {
            credit_rating: true,
            ..skill("standing", 0)
        }
    }

    /// A tiny invented content pack: lighthouse keepers, kite couriers and lamp-tending.
    fn fixture_catalog() -> Catalog {
        let skills = vec![
            cr_skill(),
            skill("lamp-tending", 5),
            skill("fog-reading", 10),
            skill("rope-work", 20),
            skill("tide-tables", 5),
            skill("gull-lore", 1),
            skill("kite-rigging", 5),
            skill_with_affinity("hauling", 20, Characteristic::Str),
            skill_with_affinity("clambering", 20, Characteristic::Dex),
            skill_with_affinity("long-watch", 10, Characteristic::Con),
            skill("stargazing", 5),
            skill("tea-blending", 5),
            skill("whistling", 5),
        ];
        let careers = vec![
            // 6 slots (1 signature + 5).
            career(
                "lighthouse-keeper",
                vec![term(FormulaCharacteristic::Edu, 4)],
                Some("lamp-tending"),
                vec!["fog-reading", "rope-work", "tide-tables", "gull-lore", "long-watch"],
                (20, 60),
            ),
            // 5 slots (1 signature + 4).
            career(
                "kite-courier",
                vec![
                    term(FormulaCharacteristic::Dex, 2),
                    term(FormulaCharacteristic::Edu, 2),
                ],
                Some("kite-rigging"),
                vec!["clambering", "hauling", "fog-reading", "whistling"],
                (10, 40),
            ),
            // 7 slots (1 signature + 6).
            career(
                "harbour-inspector",
                vec![term(FormulaCharacteristic::Edu, 4)],
                Some("tide-tables"),
                vec![
                    "fog-reading",
                    "rope-work",
                    "gull-lore",
                    "lamp-tending",
                    "long-watch",
                    "clambering",
                ],
                (30, 70),
            ),
            // Jack-of-all-trades shaped: no signature, hobby flow-in, Luck perk.
            Career {
                slots_from_hobbies: true,
                bonuses: CareerBonuses { luck: Some(10) },
                ..career(
                    "odd-jobber",
                    vec![
                        term(FormulaCharacteristic::Edu, 2),
                        term(FormulaCharacteristic::BestOther, 2),
                    ],
                    None,
                    vec!["rope-work", "tea-blending", "whistling", "gull-lore"],
                    (9, 50),
                )
            },
        ];
        let hobbies = vec![
            hobby("kite-flying", "kite-rigging", "fog-reading"),
            hobby("night-walks", "stargazing", "long-watch"),
            hobby("tea-tasting", "tea-blending", "gull-lore"),
            hobby("cliff-scrambling", "clambering", "hauling"),
        ];
        Catalog::new("fixture-pack", skills, careers, hobbies)
    }

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

    fn answers_with(career_id: Option<&str>, hobby_ids: &[&str], c: Characteristics, age: i32) -> Answers {
        Answers {
            name: Some("Fixture".to_string()),
            age: Some(age),
            age_band: Some(AgeBand::from_age(age)),
            dice: Some(DiceAnswers {
                source: DiceSource::Server,
                characteristics: Some(c),
                luck: Some(LuckAnswer {
                    rolls: vec![50],
                    kept: Some(50),
                }),
                ..Default::default()
            }),
            career_id: career_id.map(|s| s.to_string()),
            hobby_ids: hobby_ids.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    // ---- ladders ------------------------------------------------------------------------

    #[test]
    fn every_ladder_sums_to_one_hundred() {
        assert_eq!(LADDER_5.iter().sum::<i32>(), 100);
        assert_eq!(LADDER_6.iter().sum::<i32>(), 100);
        assert_eq!(LADDER_7.iter().sum::<i32>(), 100);
        assert_eq!(JACK_FIXED_PCT.iter().sum::<i32>(), 40);
        assert_eq!(JACK_HOBBY_PCT.iter().sum::<i32>(), 60);
        assert_eq!(PERSONAL_PICK_PCT.iter().sum::<i32>(), 100);
        assert_eq!(HOBBY_PRIMARY_PCT + HOBBY_SECONDARY_PCT, 100);
    }

    #[test]
    fn ladder_selection_by_slot_count() {
        assert_eq!(ladder(5), LADDER_5.to_vec());
        assert_eq!(ladder(6), LADDER_6.to_vec());
        assert_eq!(ladder(7), LADDER_7.to_vec());
        assert_eq!(ladder(0), Vec::<i32>::new());
        assert_eq!(ladder(4), vec![25, 25, 25, 25]);
    }

    #[test]
    fn six_slot_ladder_pays_credit_rating_first_then_descends() {
        let catalog = fixture_catalog();
        // EDU 80 -> pool 320. CR minimum 20 -> remainder 300.
        let answers = answers_with(
            Some("lighthouse-keeper"),
            &[],
            chars([50, 50, 50, 50, 50, 50, 50, 80]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.career_pool, 320);
        assert_eq!(a.credit_rating, 20);
        assert_eq!(a.career_points["standing"], 20);
        // 300 * [30,18,16,14,12,10] / 100 = 90,54,48,42,36,30 (no remainder).
        assert_eq!(a.career_points["lamp-tending"], 90);
        assert_eq!(a.career_points["fog-reading"], 54);
        assert_eq!(a.career_points["rope-work"], 48);
        assert_eq!(a.career_points["tide-tables"], 42);
        assert_eq!(a.career_points["gull-lore"], 36);
        assert_eq!(a.career_points["long-watch"], 30);
        assert_eq!(a.career_pool_spent, 320);
    }

    #[test]
    fn five_slot_ladder_and_rounding_remainder_goes_to_the_signature() {
        let catalog = fixture_catalog();
        // DEX 55 + EDU 55 -> (55+55)*2 = 220. CR min 10 -> remainder 210.
        let answers = answers_with(
            Some("kite-courier"),
            &[],
            chars([50, 50, 50, 55, 50, 50, 50, 55]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.career_pool, 220);
        // 210 * [34,20,18,15,13]/100 = 71,42,37,31,27 -> 208, remainder 2 to the signature.
        assert_eq!(a.career_points["kite-rigging"], 71 + 2);
        assert_eq!(a.career_points["clambering"], 42);
        assert_eq!(a.career_points["hauling"], 37);
        assert_eq!(a.career_points["fog-reading"], 31);
        assert_eq!(a.career_points["whistling"], 27);
        let spent: i32 = a.career_points.values().sum();
        assert_eq!(spent, a.career_pool, "the whole pool is spent, to the point");
    }

    #[test]
    fn seven_slot_ladder_spends_the_whole_pool() {
        let catalog = fixture_catalog();
        let answers = answers_with(
            Some("harbour-inspector"),
            &[],
            chars([50, 50, 50, 50, 50, 50, 50, 75]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.career_pool, 300);
        assert_eq!(a.credit_rating, 30);
        // remainder 270 * [28,16,14,12,11,10,9]/100 = 75,43,37,32,29,27,24 = 267, +3 to signature.
        assert_eq!(a.career_points["tide-tables"], 75 + 3);
        assert_eq!(a.career_points["fog-reading"], 43);
        assert_eq!(a.career_points["clambering"], 24);
        let spent: i32 = a.career_points.values().sum();
        assert_eq!(spent, 300);
    }

    #[test]
    fn credit_rating_minimum_is_paid_before_the_ladder() {
        let catalog = fixture_catalog();
        // A tiny pool: EDU 40 -> 160, CR min 20 leaves 140 for skills.
        let answers = answers_with(
            Some("lighthouse-keeper"),
            &[],
            chars([50, 50, 50, 50, 50, 50, 50, 40]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.career_pool, 160);
        assert_eq!(a.credit_rating, 20);
        let skill_points: i32 = a
            .career_points
            .iter()
            .filter(|(id, _)| id.as_str() != "standing")
            .map(|(_, v)| *v)
            .sum();
        assert_eq!(skill_points, 140);
    }

    #[test]
    fn a_pool_smaller_than_the_credit_rating_minimum_never_goes_negative() {
        let catalog = fixture_catalog();
        let mut answers = answers_with(
            Some("harbour-inspector"),
            &[],
            chars([15, 15, 40, 15, 15, 15, 15, 40]),
            27,
        );
        // Force a pathological pool by zeroing EDU through a declared-style override.
        if let Some(d) = answers.dice.as_mut() {
            d.characteristics = Some(chars([15, 15, 40, 15, 15, 15, 15, 5]));
        }
        let a = allocate(&catalog, &answers);
        assert_eq!(a.career_pool, 20);
        // The CR minimum is 30 and is still granted — it is what the profession pays — but the
        // 10 points the pool could not cover are reported, not hidden behind a max(0).
        assert_eq!(a.credit_rating, 30);
        assert_eq!(a.career_points["standing"], 30);
        assert_eq!(a.career_pool_shortfall, 10);
        let skill_points: i32 = a
            .career_points
            .iter()
            .filter(|(id, _)| id.as_str() != "standing")
            .map(|(_, v)| *v)
            .sum();
        assert_eq!(skill_points, 0);
        assert_eq!(a.career_pool_spent, 30);
        assert_eq!(
            a.career_pool_spent,
            a.career_pool + a.career_pool_shortfall,
            "the ledger reconciles, and says where the extra came from"
        );
    }

    /// The ledger has to add up for every career at every age: `career_pool_spent` is exactly
    /// the sum of the points handed out, and never exceeds the pool except by a shortfall the
    /// sheet declares.
    #[test]
    fn the_career_ledger_always_reconciles() {
        let catalog = fixture_catalog();
        for age in [17, 27, 48, 65, 75, 85] {
            for career_id in [
                "lighthouse-keeper",
                "kite-courier",
                "harbour-inspector",
                "odd-jobber",
            ] {
                let answers = answers_with(
                    Some(career_id),
                    &["kite-flying", "night-walks", "tea-tasting", "cliff-scrambling"],
                    chars([15, 15, 40, 15, 15, 15, 15, 40]),
                    age,
                );
                let a = allocate(&catalog, &answers);
                let ledger: i32 = a.career_points.values().sum();
                let label = format!("{career_id} at {age}");
                assert_eq!(ledger, a.career_pool_spent, "{label}");
                assert!(a.career_pool_shortfall >= 0, "{label}");
                assert_eq!(
                    a.career_pool_spent,
                    a.career_pool + a.career_pool_shortfall,
                    "{label}: spent {} against pool {} + shortfall {}",
                    a.career_pool_spent,
                    a.career_pool,
                    a.career_pool_shortfall
                );
            }
        }
    }

    #[test]
    fn the_even_split_fallback_ladder_still_sums_to_one_hundred() {
        for n in 1..=12usize {
            let l = ladder(n);
            assert_eq!(l.len(), n);
            assert_eq!(l.iter().sum::<i32>(), 100, "ladder({n}) = {l:?}");
        }
        // The shortfall lands on slot 0, the signature, exactly as the spec'd ladders do.
        assert_eq!(ladder(3), vec![34, 33, 33]);
        assert_eq!(ladder(8), vec![16, 12, 12, 12, 12, 12, 12, 12]);
    }

    #[test]
    fn an_off_ladder_slot_count_still_spends_the_whole_pool() {
        let catalog = fixture_catalog();
        // Trim the harbour inspector down to three slots (1 signature + 2).
        let mut careers = catalog.careers().to_vec();
        let inspector = careers
            .iter_mut()
            .find(|c| c.id == "harbour-inspector")
            .unwrap();
        inspector.supporting_skills.truncate(2);
        let catalog = Catalog::new(
            "fixture-pack",
            catalog.skills().to_vec(),
            careers,
            catalog.hobbies().to_vec(),
        );

        let answers = answers_with(
            Some("harbour-inspector"),
            &[],
            chars([50, 50, 50, 50, 50, 50, 50, 75]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.career_pool, 300);
        assert_eq!(a.credit_rating, 30);
        // 270 over [34,33,33] = 91,89,89 = 269, +1 rounding remainder to the signature.
        assert_eq!(a.career_points["tide-tables"], 91 + 1);
        let spent: i32 = a.career_points.values().sum();
        assert_eq!(spent, 300, "no career point may silently disappear");
        assert_eq!(a.career_pool_spent, 300);
    }

    #[test]
    fn edu_checks_beyond_the_bands_grant_are_not_applied() {
        let catalog = fixture_catalog();
        // Rolled as a Venerable (4 checks), then the player goes back and answers "young".
        let venerable_checks = vec![
            EduCheck { d100: 99, gain: 10 },
            EduCheck { d100: 99, gain: 10 },
            EduCheck { d100: 99, gain: 10 },
            EduCheck { d100: 99, gain: 10 },
        ];

        let mut answers = answers_with(None, &[], chars([50, 50, 50, 50, 50, 50, 50, 60]), 65);
        if let Some(d) = answers.dice.as_mut() {
            d.edu_checks = venerable_checks.clone();
        }
        assert_eq!(allocate(&catalog, &answers).characteristics.edu, 99);

        answers.age = Some(17);
        answers.age_band = Some(AgeBand::Young);
        let a = allocate(&catalog, &answers);
        // Young grants zero checks: EDU is 60 minus the band's flat -5, and nothing else.
        assert_eq!(a.characteristics.edu, 55);
        assert!(a.edu_checks_applied.is_empty());
        assert!(
            !a.adjustments
                .iter()
                .any(|adj| adj.reason == "EDU life experience"),
            "the sheet must not show a gain the band never granted"
        );

        // Prime grants exactly one, so only the first stored check counts.
        answers.age = Some(27);
        answers.age_band = Some(AgeBand::Prime);
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.edu, 70);
        assert_eq!(a.edu_checks_applied, vec![true]);
    }

    // ---- personal pool -------------------------------------------------------------------

    #[test]
    fn personal_pool_splits_by_pick_order_then_seventy_thirty() {
        let catalog = fixture_catalog();
        // INT 50 -> personal pool 100. Shares: 40 / 30 / 20 / 10.
        let answers = answers_with(
            None,
            &["kite-flying", "night-walks", "tea-tasting", "cliff-scrambling"],
            chars([50, 50, 50, 50, 50, 50, 50, 50]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.personal_pool, 100);
        // 40 -> 28/12, 30 -> 21/9, 20 -> 14/6, 10 -> 7/3.
        assert_eq!(a.personal_points["kite-rigging"], 28);
        assert_eq!(a.personal_points["fog-reading"], 12);
        assert_eq!(a.personal_points["stargazing"], 21);
        assert_eq!(a.personal_points["long-watch"], 9);
        assert_eq!(a.personal_points["tea-blending"], 14);
        assert_eq!(a.personal_points["gull-lore"], 6);
        assert_eq!(a.personal_points["clambering"], 7);
        assert_eq!(a.personal_points["hauling"], 3);
        assert_eq!(a.personal_pool_spent, 100);
    }

    #[test]
    fn personal_pool_rounding_remainders_land_on_the_first_pick_primary() {
        let catalog = fixture_catalog();
        // INT 47 -> pool 94. Shares: 37 / 28 / 18 / 9 = 92, so 2 points of global remainder.
        let answers = answers_with(
            None,
            &["kite-flying", "night-walks", "tea-tasting", "cliff-scrambling"],
            chars([50, 50, 50, 50, 50, 50, 47, 50]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.personal_pool, 94);
        // hobby 1: 37 -> primary 25 (floor 25.9) + within-hobby remainder 1 = 26, secondary 11;
        // plus the 2-point global remainder = 28.
        assert_eq!(a.personal_points["fog-reading"], 11);
        assert_eq!(a.personal_points["kite-rigging"], 26 + 2);
        // hobby 2: 28 -> 19 (+1 remainder) / 8.
        assert_eq!(a.personal_points["stargazing"], 20);
        assert_eq!(a.personal_points["long-watch"], 8);
        // hobby 3: 18 -> 13 (+1) / 5.
        assert_eq!(a.personal_points["tea-blending"], 13);
        assert_eq!(a.personal_points["gull-lore"], 5);
        // hobby 4: 9 -> 7 (+1) / 2.
        assert_eq!(a.personal_points["clambering"], 7);
        assert_eq!(a.personal_points["hauling"], 2);
        assert_eq!(a.personal_pool_spent, 94);
    }

    #[test]
    fn overlapping_hobbies_stack_on_the_same_skill() {
        let catalog = fixture_catalog();
        // Two picks that both train clambering / hauling.
        let answers = answers_with(
            None,
            &["cliff-scrambling", "cliff-scrambling"],
            chars([50, 50, 50, 50, 50, 50, 50, 50]),
            27,
        );
        let a = allocate(&catalog, &answers);
        // 40% share -> 28/12, 30% share -> 21/9.
        assert_eq!(a.personal_points["clambering"], 28 + 21);
        assert_eq!(a.personal_points["hauling"], 12 + 9);
    }

    #[test]
    fn a_partial_draft_only_spends_the_weights_it_has() {
        let catalog = fixture_catalog();
        let answers = answers_with(None, &["kite-flying"], chars([50, 50, 50, 50, 50, 50, 50, 50]), 27);
        let a = allocate(&catalog, &answers);
        assert_eq!(a.personal_pool, 100);
        // Only the 40% first-pick share is spendable while three picks are missing.
        assert_eq!(a.personal_pool_spent, 40);
        assert_eq!(a.personal_points["kite-rigging"], 28);
        assert_eq!(a.personal_points["fog-reading"], 12);
    }

    #[test]
    fn unknown_hobby_ids_do_not_reshuffle_pick_order_weighting() {
        let catalog = fixture_catalog();
        let answers = answers_with(
            None,
            &["not-a-hobby", "night-walks"],
            chars([50, 50, 50, 50, 50, 50, 50, 50]),
            27,
        );
        let a = allocate(&catalog, &answers);
        // night-walks stays the second pick and takes 30%, not 40%.
        assert_eq!(a.personal_points["stargazing"], 21);
        assert_eq!(a.personal_points["long-watch"], 9);
    }

    // ---- Jack of All Trades ---------------------------------------------------------------

    #[test]
    fn jack_of_all_trades_uses_best_other_and_flows_into_hobby_primaries() {
        let catalog = fixture_catalog();
        // EDU 50 x2 = 100; best other is POW 80 x2 = 160 -> pool 260. CR min 9 -> 251.
        let answers = answers_with(
            Some("odd-jobber"),
            &["kite-flying", "night-walks", "tea-tasting", "cliff-scrambling"],
            chars([40, 45, 50, 55, 60, 80, 50, 50]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.best_other, Some(Characteristic::Pow));
        assert_eq!(a.career_pool, 260);
        assert_eq!(a.credit_rating, 9);

        // Fixed 12/10/10/8 of 251 -> 30, 25, 25, 20.
        assert_eq!(a.career_points["rope-work"], 30);
        assert_eq!(a.career_points["tea-blending"] - 25, 30); // 25 fixed + 30 hobby-3 share
        assert_eq!(a.career_points["whistling"], 25);
        assert_eq!(a.career_points["gull-lore"], 20);
        // Hobby primaries 24/16/12/8 of 251 -> 60, 40, 30, 20; remainder 1 to the first primary.
        assert_eq!(a.career_points["kite-rigging"], 60 + 1);
        assert_eq!(a.career_points["stargazing"], 40);
        assert_eq!(a.career_points["clambering"], 20);
        assert_eq!(a.career_pool_spent, 260);
    }

    #[test]
    fn jack_of_all_trades_adds_ten_luck_capped_at_99() {
        let catalog = fixture_catalog();
        let mut answers = answers_with(
            Some("odd-jobber"),
            &["kite-flying"],
            chars([50, 50, 50, 50, 50, 50, 50, 50]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.luck, Some(60), "50 rolled + 10 perk");
        assert_eq!(a.luck_bonus, 10);

        if let Some(d) = answers.dice.as_mut() {
            d.luck = Some(LuckAnswer {
                rolls: vec![90],
                kept: Some(90),
            });
        }
        let a = allocate(&catalog, &answers);
        assert_eq!(a.luck, Some(99), "capped at 99");
    }

    #[test]
    fn other_careers_get_no_luck_perk() {
        let catalog = fixture_catalog();
        let answers = answers_with(
            Some("lighthouse-keeper"),
            &[],
            chars([50, 50, 50, 50, 50, 50, 50, 50]),
            27,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.luck, Some(50));
        assert_eq!(a.luck_bonus, 0);
    }

    #[test]
    fn young_luck_keeps_the_better_of_two_rolls() {
        let catalog = fixture_catalog();
        let mut answers = answers_with(None, &[], chars([50, 50, 50, 50, 50, 50, 50, 50]), 17);
        if let Some(d) = answers.dice.as_mut() {
            d.luck = Some(LuckAnswer {
                rolls: vec![35, 80],
                kept: Some(80),
            });
        }
        let a = allocate(&catalog, &answers);
        assert_eq!(a.luck, Some(80));
    }

    // ---- age deductions --------------------------------------------------------------------

    #[test]
    fn age_deduction_takes_from_the_characteristic_the_career_needs_least() {
        let catalog = fixture_catalog();
        // Kite courier: DEX is in the formula (score 20) and clambering is DEX-flavoured;
        // hauling is STR-flavoured (score 1); nothing is CON-flavoured (score 0) -> CON pays.
        let answers = answers_with(
            Some("kite-courier"),
            &[],
            chars([50, 50, 50, 55, 50, 50, 50, 55]),
            48,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.con, 45, "5 points off CON");
        assert_eq!(a.characteristics.strength, 50);
        assert_eq!(a.characteristics.dex, 55);
    }

    #[test]
    fn age_deduction_avoids_a_career_that_leans_on_con() {
        let catalog = fixture_catalog();
        // Lighthouse keeper's supporting list includes long-watch (CON), so CON scores 1 and
        // STR scores 0 -> STR pays instead.
        let answers = answers_with(
            Some("lighthouse-keeper"),
            &[],
            chars([50, 50, 50, 50, 50, 50, 50, 60]),
            48,
        );
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.strength, 45);
        assert_eq!(a.characteristics.con, 50);
    }

    #[test]
    fn age_deduction_ties_break_con_then_str_then_dex() {
        let catalog = fixture_catalog();
        // No career at all: every score is 0, so the fixed tie order applies and CON pays.
        let answers = answers_with(None, &[], chars([50, 50, 50, 50, 50, 50, 50, 60]), 48);
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.con, 45);
        assert_eq!(a.characteristics.strength, 50);
        assert_eq!(a.characteristics.dex, 50);
    }

    #[test]
    fn a_large_deduction_cascades_down_the_order_without_going_below_one() {
        let catalog = fixture_catalog();
        // An 80-year-old pays 80 points; CON 20 can only give 19 before hitting the floor,
        // STR 30 gives 29, and the last 32 cascade into DEX, which also bottoms out.
        let answers = answers_with(None, &[], chars([30, 20, 50, 25, 50, 50, 50, 60]), 85);
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.con, MIN_CHARACTERISTIC);
        assert_eq!(a.characteristics.strength, MIN_CHARACTERISTIC);
        assert_eq!(a.characteristics.dex, MIN_CHARACTERISTIC);
        assert_eq!(a.characteristics.siz, 50, "SIZ is never touched after the teens");
    }

    #[test]
    fn a_deduction_stops_at_the_floor_and_carries_the_rest_onward() {
        let catalog = fixture_catalog();
        // 60s: 20 points. CON 15 gives 14, the remaining 6 land on STR.
        let answers = answers_with(None, &[], chars([50, 15, 50, 50, 50, 50, 50, 60]), 65);
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.con, 1);
        assert_eq!(a.characteristics.strength, 44);
        assert_eq!(a.characteristics.dex, 50);
    }

    #[test]
    fn seasoned_characters_also_pay_app_and_mov() {
        let catalog = fixture_catalog();
        let answers = answers_with(None, &[], chars([50, 50, 50, 50, 60, 50, 50, 60]), 55);
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.app, 50, "APP -10 in the fifties");
        assert_eq!(a.characteristics.con, 40, "physical -10 in the fifties");
        assert!(a
            .adjustments
            .iter()
            .any(|adj| adj.reason == "age: APP" && adj.delta == -10));
    }

    #[test]
    fn young_characters_pay_edu_and_split_five_across_str_and_siz() {
        let catalog = fixture_catalog();
        let answers = answers_with(None, &[], chars([50, 50, 60, 50, 50, 50, 50, 65]), 17);
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.edu, 60, "EDU -5");
        // SIZ leads the tie-break and takes 3; STR takes 2.
        assert_eq!(a.characteristics.siz, 57);
        assert_eq!(a.characteristics.strength, 48);
        assert_eq!(a.characteristics.siz + a.characteristics.strength, 105);
    }

    #[test]
    fn the_young_split_is_stable_across_careers() {
        let catalog = fixture_catalog();
        // No career formula or skill in the roster leans on SIZ, so SIZ always scores lowest
        // and takes the larger half; the outcome does not drift as the player shops careers.
        for career_id in [None, Some("kite-courier"), Some("lighthouse-keeper")] {
            let answers = answers_with(career_id, &[], chars([50, 50, 60, 50, 50, 50, 50, 65]), 17);
            let a = allocate(&catalog, &answers);
            assert_eq!(a.characteristics.siz, 57, "{career_id:?}");
            assert_eq!(a.characteristics.strength, 48, "{career_id:?}");
        }
    }

    #[test]
    fn edu_checks_apply_after_the_age_edu_cost() {
        let catalog = fixture_catalog();
        let mut answers = answers_with(None, &[], chars([50, 50, 50, 50, 50, 50, 50, 60]), 48);
        if let Some(d) = answers.dice.as_mut() {
            d.edu_checks = vec![
                EduCheck { d100: 75, gain: 6 }, // 75 > 60 -> +6 = 66
                EduCheck { d100: 66, gain: 5 }, // 66 <= 66 -> no gain
            ];
        }
        let a = allocate(&catalog, &answers);
        assert_eq!(a.characteristics.edu, 66);
        assert_eq!(a.edu_checks_applied, vec![true, false]);
    }

    #[test]
    fn the_career_pool_is_evaluated_on_post_age_characteristics() {
        let catalog = fixture_catalog();
        let mut answers = answers_with(
            Some("kite-courier"),
            &[],
            chars([50, 50, 50, 55, 50, 50, 50, 55]),
            48,
        );
        // A 48-year-old kite courier pays 5 from CON, which is not in the formula, so the pool
        // is unchanged; but an EDU check that lands does change it.
        let before = allocate(&catalog, &answers).career_pool;
        assert_eq!(before, (55 + 55) * 2);
        if let Some(d) = answers.dice.as_mut() {
            d.edu_checks = vec![EduCheck { d100: 99, gain: 5 }];
        }
        let after = allocate(&catalog, &answers).career_pool;
        assert_eq!(after, (55 + 60) * 2);
    }

    // ---- determinism --------------------------------------------------------------------

    #[test]
    fn the_same_answers_always_produce_the_same_allocation() {
        let catalog = fixture_catalog();
        let answers = answers_with(
            Some("odd-jobber"),
            &["kite-flying", "night-walks", "tea-tasting", "cliff-scrambling"],
            chars([45, 55, 65, 50, 60, 70, 55, 75]),
            48,
        );
        let first = allocate(&catalog, &answers);
        for _ in 0..25 {
            assert_eq!(allocate(&catalog, &answers), first);
        }
    }

    #[test]
    fn an_empty_draft_allocates_nothing_and_does_not_panic() {
        let catalog = fixture_catalog();
        let a = allocate(&catalog, &Answers::default());
        assert_eq!(a.career_pool, 0);
        assert_eq!(a.personal_pool, 0);
        assert!(a.career_points.is_empty());
        assert!(a.personal_points.is_empty());
        assert_eq!(a.luck, None);
        assert!(a.adjustments.is_empty());
    }

    #[test]
    fn best_other_ties_break_in_the_fixed_candidate_order() {
        let c = chars([70, 70, 70, 70, 70, 70, 70, 90]);
        assert_eq!(best_other(&c), Characteristic::Str);
        let c = chars([10, 70, 70, 15, 15, 15, 15, 90]);
        assert_eq!(best_other(&c), Characteristic::Con);
        // EDU is never the "other" even when it is highest.
        let c = chars([10, 10, 10, 10, 10, 10, 10, 90]);
        assert_eq!(best_other(&c), Characteristic::Str);
    }
}
