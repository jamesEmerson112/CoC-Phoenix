//! §3 — the roll, manual entry validation, and the §2 costs that ride on it.
//!
//! §3 manual entry: *"light validation (multiples of 5; 15-90 for 3D6x5 stats, 40-90 for 2D6+6
//! stats; d100 + d10 pairs for EDU checks; one or two 3D6x5 values for Luck)"*.
//! §3b EDU check: *"roll d100; if the roll is **above** current EDU, EDU rises by +1D10"*.

use coc_phoenix::allocator::allocate;
use coc_phoenix::dice::{
    apply_edu_checks, kept_luck, luck_roll_count, validate_declared_characteristics,
    validate_declared_edu_checks, validate_declared_luck,
};
use coc_phoenix::derived::edu_check_count;
use coc_phoenix::model::{AgeBand, Characteristic, Characteristics, EduCheck};

use crate::common::{chars8, draft, orchard_catalog};

/// A legal baseline set: every 3D6x5 stat at 50, every (2D6+6)x5 stat at 50.
fn legal() -> Characteristics {
    chars8([50, 50, 50, 50, 50, 50, 50, 50])
}

// ---- 3D6 x 5 stats: 15..=90 -------------------------------------------------------------

#[test]
fn spec3_three_d6_stats_accept_15_and_90_and_reject_10_and_95() {
    for c in Characteristic::THREE_D6 {
        // 15 is the floor (3 ones x 5) and 90 the ceiling (3 sixes x 5).
        for ok in [15, 40, 90] {
            let mut chars = legal();
            chars.set(c, ok);
            assert!(
                validate_declared_characteristics(&chars).is_ok(),
                "{} = {ok} should be legal",
                c.key()
            );
        }
        // 10 is below the floor, 95 above the ceiling — both on the multiple-of-5 step, so the
        // only thing that can reject them is the range check.
        for bad in [10, 95] {
            let mut chars = legal();
            chars.set(c, bad);
            let errors = validate_declared_characteristics(&chars)
                .expect_err(&format!("{} = {bad} must be rejected", c.key()));
            assert!(
                errors.iter().any(|e| e.0.contains("15") && e.0.contains("90")),
                "{} = {bad}: expected a 15-90 range complaint, got {errors:?}",
                c.key()
            );
        }
    }
}

// ---- (2D6+6) x 5 stats: 40..=90 ---------------------------------------------------------

#[test]
fn spec3_two_d6_plus_six_stats_reject_35_and_accept_40() {
    for c in Characteristic::TWO_D6_PLUS_6 {
        let mut chars = legal();
        chars.set(c, 40);
        assert!(
            validate_declared_characteristics(&chars).is_ok(),
            "{} = 40 is the (2D6+6)x5 floor",
            c.key()
        );

        chars.set(c, 90);
        assert!(
            validate_declared_characteristics(&chars).is_ok(),
            "{} = 90 is the (2D6+6)x5 ceiling",
            c.key()
        );

        // 35 is a legal 3D6x5 value but impossible on (2D6+6)x5.
        chars.set(c, 35);
        let errors = validate_declared_characteristics(&chars)
            .expect_err(&format!("{} = 35 must be rejected", c.key()));
        assert!(
            errors.iter().any(|e| e.0.contains("40") && e.0.contains("90")),
            "{} = 35: expected a 40-90 range complaint, got {errors:?}",
            c.key()
        );

        chars.set(c, 95);
        assert!(
            validate_declared_characteristics(&chars).is_err(),
            "{} = 95 is above the ceiling",
            c.key()
        );
    }
}

#[test]
fn spec3_every_characteristic_must_be_a_multiple_of_five() {
    // Every characteristic is a dice total multiplied by 5, so nothing else can be declared.
    for c in Characteristic::ALL {
        for off_step in [51, 52, 53, 54, 47, 89] {
            let mut chars = legal();
            chars.set(c, off_step);
            let errors = validate_declared_characteristics(&chars)
                .expect_err(&format!("{} = {off_step} must be rejected", c.key()));
            assert!(
                errors.iter().any(|e| e.0.contains("multiple of 5")),
                "{} = {off_step}: expected a multiple-of-5 complaint, got {errors:?}",
                c.key()
            );
        }
    }
}

#[test]
fn spec3_a_fully_legal_declared_set_passes() {
    let extremes = Characteristics {
        strength: 15,
        con: 90,
        siz: 40,
        dex: 15,
        app: 90,
        pow: 15,
        intelligence: 90,
        edu: 40,
    };
    assert!(validate_declared_characteristics(&extremes).is_ok());
}

// ---- Luck: 3D6 x 5, one roll (two for Young) ---------------------------------------------

#[test]
fn spec3_declared_luck_is_a_3d6x5_value_and_the_band_fixes_the_roll_count() {
    // §2: Young rolls Luck twice; every other band rolls once.
    assert_eq!(luck_roll_count(AgeBand::Young), 2);
    assert_eq!(luck_roll_count(AgeBand::Prime), 1);
    assert_eq!(luck_roll_count(AgeBand::Seasoned), 1);
    assert_eq!(luck_roll_count(AgeBand::Venerable), 1);

    assert!(validate_declared_luck(&[15], AgeBand::Prime).is_ok());
    assert!(validate_declared_luck(&[90], AgeBand::Prime).is_ok());
    assert!(validate_declared_luck(&[10], AgeBand::Prime).is_err(), "below 15");
    assert!(validate_declared_luck(&[95], AgeBand::Prime).is_err(), "above 90");
    assert!(validate_declared_luck(&[53], AgeBand::Prime).is_err(), "off the step");
    assert!(validate_declared_luck(&[], AgeBand::Prime).is_err(), "no roll at all");

    // A second roll is only legal for Young.
    assert!(validate_declared_luck(&[40, 75], AgeBand::Young).is_ok());
    assert!(validate_declared_luck(&[40, 75], AgeBand::Prime).is_err());
    assert!(validate_declared_luck(&[40, 75], AgeBand::Seasoned).is_err());
    assert!(validate_declared_luck(&[40, 75], AgeBand::Venerable).is_err());
}

#[test]
fn spec2_young_keeps_the_better_of_two_luck_rolls() {
    // "Luck rolled twice, keep the better — the universe protects the young".
    assert_eq!(kept_luck(&[35, 80], AgeBand::Young), Some(80));
    assert_eq!(kept_luck(&[80, 35], AgeBand::Young), Some(80), "order must not matter");
    assert_eq!(kept_luck(&[45, 45], AgeBand::Young), Some(45));
    assert_eq!(kept_luck(&[], AgeBand::Young), None);

    // Non-Young bands keep their single roll.
    assert_eq!(kept_luck(&[35], AgeBand::Prime), Some(35));
}

#[test]
fn spec2_young_double_luck_reaches_the_sheet() {
    let catalog = orchard_catalog();
    // 17 -> Young, so both rolls stand and the better one is kept.
    let answers = draft(
        "Wren Ashcombe-Lyle",
        17,
        chars8([50, 50, 55, 50, 50, 50, 50, 60]),
        None,
        &[],
        &[],
        &[30, 75],
    );
    assert_eq!(allocate(&catalog, &answers).luck, Some(75));

    // The same two rolls at 27 are not a Young keep-best; only one roll is granted.
    let prime = draft(
        "Wren Ashcombe-Lyle",
        27,
        chars8([50, 50, 55, 50, 50, 50, 50, 60]),
        None,
        &[],
        &[(80, 5)],
        &[30],
    );
    assert_eq!(allocate(&catalog, &prime).luck, Some(30));
}

// ---- EDU life-experience checks ----------------------------------------------------------

#[test]
fn spec2_edu_check_counts_per_age_band() {
    // Young 0 · Prime 1 · 40s 2 · 50s 3 · 60+ 4.
    assert_eq!(edu_check_count(15), 0);
    assert_eq!(edu_check_count(19), 0);
    assert_eq!(edu_check_count(20), 1);
    assert_eq!(edu_check_count(39), 1);
    assert_eq!(edu_check_count(40), 2);
    assert_eq!(edu_check_count(49), 2);
    assert_eq!(edu_check_count(50), 3);
    assert_eq!(edu_check_count(59), 3);
    assert_eq!(edu_check_count(60), 4);
    assert_eq!(edu_check_count(72), 4);
    assert_eq!(edu_check_count(89), 4);
}

#[test]
fn spec3b_a_check_applies_only_when_the_d100_is_strictly_above_current_edu() {
    // "if the roll is **above** current EDU, EDU rises by +1D10"
    let (edu, applied) = apply_edu_checks(60, &[EduCheck { d100: 61, gain: 4 }]);
    assert_eq!((edu, applied), (64, vec![true]), "61 > 60 improves");

    let (edu, applied) = apply_edu_checks(60, &[EduCheck { d100: 60, gain: 4 }]);
    assert_eq!((edu, applied), (60, vec![false]), "60 is not above 60");

    let (edu, applied) = apply_edu_checks(60, &[EduCheck { d100: 59, gain: 4 }]);
    assert_eq!((edu, applied), (60, vec![false]), "59 is below 60");
}

#[test]
fn spec3b_checks_are_sequential_so_each_success_raises_the_bar() {
    // Start at EDU 65 and run the four checks a Venerable investigator is granted.
    //   1) 80 > 65 -> 65 + 7 = 72
    //   2) 60 <= 72 -> no change
    //   3) 95 > 72 -> 72 + 3 = 75
    //   4) 76 > 75 -> 75 + 6 = 81
    let checks = [
        EduCheck { d100: 80, gain: 7 },
        EduCheck { d100: 60, gain: 4 },
        EduCheck { d100: 95, gain: 3 },
        EduCheck { d100: 76, gain: 6 },
    ];
    let (edu, applied) = apply_edu_checks(65, &checks);
    assert_eq!(edu, 81);
    assert_eq!(applied, vec![true, false, true, true]);
}

#[test]
fn spec3_declared_edu_checks_are_d100_plus_d10_pairs_within_the_band_allowance() {
    let good = [EduCheck { d100: 71, gain: 7 }];
    assert!(validate_declared_edu_checks(&good, 2).is_ok());
    assert!(
        validate_declared_edu_checks(&good, 0).is_err(),
        "the Young band grants no checks at all"
    );

    // A d100 is 1..=100 and a d10 gain is at most 10.
    assert!(validate_declared_edu_checks(&[EduCheck { d100: 1, gain: 1 }], 1).is_ok());
    assert!(validate_declared_edu_checks(&[EduCheck { d100: 100, gain: 10 }], 1).is_ok());
    assert!(validate_declared_edu_checks(&[EduCheck { d100: 101, gain: 5 }], 1).is_err());
    assert!(validate_declared_edu_checks(&[EduCheck { d100: 0, gain: 5 }], 1).is_err());
    assert!(validate_declared_edu_checks(&[EduCheck { d100: 50, gain: 11 }], 1).is_err());
}

#[test]
fn spec2_young_pays_five_edu_and_five_across_str_and_siz() {
    let catalog = orchard_catalog();
    // Raw: STR 50, SIZ 60, EDU 65 at age 17.
    let answers = draft(
        "Solveig Amrit-Barlowe",
        17,
        chars8([50, 50, 60, 50, 50, 50, 50, 65]),
        None,
        &[],
        &[],
        &[40, 55],
    );
    let a = allocate(&catalog, &answers);

    assert_eq!(a.characteristics.edu, 60, "EDU -5");

    // "-5 split between STR and SIZ": the split itself is unspecified, but the total is not,
    // and both characteristics have to carry some of it for it to be a split.
    let str_paid = 50 - a.characteristics.strength;
    let siz_paid = 60 - a.characteristics.siz;
    assert_eq!(str_paid + siz_paid, 5, "the Young cost is exactly 5 points");
    assert!(str_paid > 0 && siz_paid > 0, "the 5 is split, not dumped on one stat");

    // Young pays no physical (STR/CON/DEX) deduction and no APP cost.
    assert_eq!(a.characteristics.con, 50);
    assert_eq!(a.characteristics.dex, 50);
    assert_eq!(a.characteristics.app, 50);

    // ...and gets no EDU checks to win any of it back.
    assert!(a.edu_checks_applied.is_empty());
}
