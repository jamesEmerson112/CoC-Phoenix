//! §7 — derived values, computed straight off the spec table.
//!
//! | Value            | Formula                                                            |
//! |------------------|--------------------------------------------------------------------|
//! | Hit Points       | `(CON + SIZ) // 10`                                                |
//! | Magic Points     | `POW // 5`                                                          |
//! | Sanity (start)   | `= POW` (max 99)                                                    |
//! | Damage/Build     | STR+SIZ: <65 -2/-2 · 65-84 -1/-1 · 85-124 0/0 · 125-164 +1D4/+1 ·   |
//! |                  | 165-204 +1D6/+2                                                     |
//! | Move (MOV)       | 9 if DEX > SIZ **and** STR > SIZ · 8 if DEX >= SIZ **or** STR >= SIZ|
//! |                  | · else 7 · minus the §2 age penalty                                 |
//! | Dodge (base)     | `DEX // 2`                                                          |
//! | Language (Own)   | `= EDU`                                                             |
//! | Half / Fifth     | `v // 2`, `v // 5`                                                  |

use coc_phoenix::derived::{
    age_effects, damage_bonus_and_build, fifth, half, hit_points, magic_points, move_base,
    move_rate, starting_sanity,
};

// ---- floor division ---------------------------------------------------------------------

#[test]
fn spec7_half_and_fifth_floor_on_odd_values() {
    // `//` in the spec is floor division; 75 // 2 = 37 (not 37.5, not 38).
    assert_eq!(half(75), 37);
    assert_eq!(half(1), 0);
    assert_eq!(half(45), 22);
    assert_eq!(half(55), 27);
    assert_eq!(half(81), 40);
    assert_eq!(half(99), 49);

    // 99 // 5 = 19 (not 19.8, not 20).
    assert_eq!(fifth(99), 19);
    assert_eq!(fifth(4), 0);
    assert_eq!(fifth(5), 1);
    assert_eq!(fifth(9), 1);
    assert_eq!(fifth(81), 16);
    assert_eq!(fifth(65), 13);
}

#[test]
fn spec7_hit_points_floor_the_tenth_of_con_plus_siz() {
    assert_eq!(hit_points(60, 60), 12); // 120 // 10
    assert_eq!(hit_points(45, 50), 9); //   95 // 10 = 9, not 9.5
    assert_eq!(hit_points(25, 60), 8); //   85 // 10 = 8
    assert_eq!(hit_points(15, 40), 5); //   55 // 10 = 5
    assert_eq!(hit_points(90, 90), 18);
}

#[test]
fn spec7_magic_points_are_a_fifth_of_pow() {
    assert_eq!(magic_points(65), 13);
    assert_eq!(magic_points(55), 11);
    assert_eq!(magic_points(69), 13); // floors
    assert_eq!(magic_points(15), 3);
}

#[test]
fn spec7_starting_sanity_equals_pow_and_never_exceeds_99() {
    assert_eq!(starting_sanity(65), 65);
    assert_eq!(starting_sanity(90), 90);
    assert_eq!(starting_sanity(99), 99);
    assert_eq!(starting_sanity(105), 99);
}

// ---- damage bonus / build band EDGES ----------------------------------------------------

#[test]
fn spec7_damage_bonus_and_build_at_every_band_edge() {
    // The spec bands STR+SIZ as: <65 · 65-84 · 85-124 · 125-164 · 165-204.
    // Each edge is probed from both sides.
    let cases: [(i32, i32, &str, i32); 11] = [
        // (STR, SIZ, damage bonus, build)   sum
        (24, 40, "-2", -2),   //  64 — last value below 65
        (25, 40, "-1", -1),   //  65 — first of the -1 band
        (44, 40, "-1", -1),   //  84 — last of the -1 band
        (45, 40, "0", 0),     //  85 — first of the 0 band
        (84, 40, "0", 0),     // 124 — last of the 0 band
        (85, 40, "+1D4", 1),  // 125 — first of the +1D4 band
        (124, 40, "+1D4", 1), // 164 — last of the +1D4 band
        (125, 40, "+1D6", 2), // 165 — first of the +1D6 band
        (164, 40, "+1D6", 2), // 204 — last printed value
        (15, 40, "-2", -2),   //  55 — a legal rolled minimum
        (30, 34, "-2", -2),   //  64 again, from a different split
    ];
    for (str_v, siz, expected_db, expected_build) in cases {
        let got = damage_bonus_and_build(str_v, siz);
        assert_eq!(
            got.damage_bonus, expected_db,
            "STR {str_v} + SIZ {siz} = {}",
            str_v + siz
        );
        assert_eq!(
            got.build, expected_build,
            "STR {str_v} + SIZ {siz} = {}",
            str_v + siz
        );
    }

    // The largest legally rollable pair: 90 + 90 = 180, inside the 165-204 band.
    let max_roll = damage_bonus_and_build(90, 90);
    assert_eq!(max_roll.damage_bonus, "+1D6");
    assert_eq!(max_roll.build, 2);
}

// ---- MOV: strict vs inclusive comparisons ------------------------------------------------

#[test]
fn spec7_move_uses_strict_greater_for_nine_and_inclusive_for_eight() {
    // 9 requires BOTH strictly greater than SIZ.
    assert_eq!(move_base(60, 60, 50), 9);
    assert_eq!(move_base(51, 51, 50), 9);

    // A single tie kills the 9 and lands on 8 (the ">=" branch).
    assert_eq!(move_base(50, 60, 50), 8, "STR ties SIZ");
    assert_eq!(move_base(60, 50, 50), 8, "DEX ties SIZ");
    assert_eq!(move_base(50, 50, 50), 8, "both tie SIZ");

    // One strictly above, the other strictly below -> still 8.
    assert_eq!(move_base(80, 40, 50), 8);
    assert_eq!(move_base(40, 80, 50), 8);

    // Both strictly below -> 7.
    assert_eq!(move_base(49, 49, 50), 7);
    assert_eq!(move_base(15, 15, 90), 7);
}

#[test]
fn spec7_move_subtracts_the_section2_age_penalty() {
    // §2: MOV -1 (40s) / -2 (50s) / -3 (60s) / -4 (70s) / -5 (80s); no penalty before 40.
    let (str_v, dex, siz) = (60, 60, 50); // base 9
    assert_eq!(move_rate(str_v, dex, siz, 17), 9, "Young: no MOV cost");
    assert_eq!(move_rate(str_v, dex, siz, 27), 9, "Prime: no MOV cost");
    assert_eq!(move_rate(str_v, dex, siz, 39), 9);
    assert_eq!(move_rate(str_v, dex, siz, 40), 8);
    assert_eq!(move_rate(str_v, dex, siz, 49), 8);
    assert_eq!(move_rate(str_v, dex, siz, 50), 7);
    assert_eq!(move_rate(str_v, dex, siz, 59), 7);
    assert_eq!(move_rate(str_v, dex, siz, 60), 6);
    assert_eq!(move_rate(str_v, dex, siz, 70), 5);
    assert_eq!(move_rate(str_v, dex, siz, 80), 4);
    assert_eq!(move_rate(str_v, dex, siz, 89), 4);

    // A base-7 octogenarian: 7 - 5 = 2.
    assert_eq!(move_rate(15, 15, 90, 85), 2);
}

// ---- §2 age table (the numbers §7's MOV row points at) ----------------------------------

#[test]
fn spec2_age_band_costs_match_the_table() {
    // Young 15-19: 0 EDU checks, EDU -5, -5 split between STR and SIZ, Luck rolled twice.
    for age in [15, 17, 19] {
        let e = age_effects(age);
        assert_eq!(e.edu_checks, 0, "age {age}");
        assert_eq!(e.edu_penalty, 5, "age {age}");
        assert_eq!(e.young_deduction, 5, "age {age}");
        assert_eq!(e.physical_deduction, 0, "age {age}");
        assert_eq!(e.app_reduction, 0, "age {age}");
        assert_eq!(e.mov_penalty, 0, "age {age}");
        assert_eq!(e.luck_rolls, 2, "age {age}");
    }

    // Prime 20-39: one EDU check, no costs at all, one Luck roll.
    for age in [20, 27, 39] {
        let e = age_effects(age);
        assert_eq!(e.edu_checks, 1, "age {age}");
        assert_eq!(e.edu_penalty, 0, "age {age}");
        assert_eq!(e.young_deduction, 0, "age {age}");
        assert_eq!(e.physical_deduction, 0, "age {age}");
        assert_eq!(e.app_reduction, 0, "age {age}");
        assert_eq!(e.mov_penalty, 0, "age {age}");
        assert_eq!(e.luck_rolls, 1, "age {age}");
    }

    // Seasoned 40s: 2 checks, physicals -5, APP -5, MOV -1.
    for age in [40, 48, 49] {
        let e = age_effects(age);
        assert_eq!(e.edu_checks, 2, "age {age}");
        assert_eq!(e.physical_deduction, 5, "age {age}");
        assert_eq!(e.app_reduction, 5, "age {age}");
        assert_eq!(e.mov_penalty, 1, "age {age}");
        assert_eq!(e.luck_rolls, 1, "age {age}");
    }

    // Seasoned 50s: 3 checks, physicals -10, APP -10, MOV -2.
    for age in [50, 55, 59] {
        let e = age_effects(age);
        assert_eq!(e.edu_checks, 3, "age {age}");
        assert_eq!(e.physical_deduction, 10, "age {age}");
        assert_eq!(e.app_reduction, 10, "age {age}");
        assert_eq!(e.mov_penalty, 2, "age {age}");
    }

    // Venerable 60s / 70s / 80s: 4 checks throughout; physicals -20/-40/-80,
    // APP -15/-20/-25, MOV -3/-4/-5.
    for (age, physical, app, mov) in [(60, 20, 15, 3), (65, 20, 15, 3), (72, 40, 20, 4), (85, 80, 25, 5)] {
        let e = age_effects(age);
        assert_eq!(e.edu_checks, 4, "age {age}");
        assert_eq!(e.physical_deduction, physical, "age {age}");
        assert_eq!(e.app_reduction, app, "age {age}");
        assert_eq!(e.mov_penalty, mov, "age {age}");
        assert_eq!(e.luck_rolls, 1, "age {age}");
    }
}

#[test]
fn spec2_band_edges_flip_on_the_exact_year() {
    // 19 is the last Young year, 20 the first Prime year.
    assert_eq!(age_effects(19).edu_penalty, 5);
    assert_eq!(age_effects(19).luck_rolls, 2);
    assert_eq!(age_effects(20).edu_penalty, 0);
    assert_eq!(age_effects(20).luck_rolls, 1);

    // 39 is the last Prime year, 40 the first Seasoned year.
    assert_eq!(age_effects(39).physical_deduction, 0);
    assert_eq!(age_effects(40).physical_deduction, 5);

    // 59 is the last Seasoned year, 60 the first Venerable year.
    assert_eq!(age_effects(59).physical_deduction, 10);
    assert_eq!(age_effects(59).app_reduction, 10);
    assert_eq!(age_effects(60).physical_deduction, 20);
    assert_eq!(age_effects(60).app_reduction, 15);
}
