//! Derived values (spec § 7) and the age-band arithmetic behind them (spec § 2).
//!
//! Pure integer maths, no I/O. `//` in the spec is Rust's `/` on non-negative integers.

use crate::model::AgeBand;
use serde::{Deserialize, Serialize};

/// `v // 2` — the half value used constantly in play.
pub fn half(v: i32) -> i32 {
    v.div_euclid(2)
}

/// `v // 5` — the fifth value.
pub fn fifth(v: i32) -> i32 {
    v.div_euclid(5)
}

/// Hit Points: `(CON + SIZ) // 10`.
pub fn hit_points(con: i32, siz: i32) -> i32 {
    (con + siz).div_euclid(10)
}

/// Magic Points: `POW // 5`.
pub fn magic_points(pow: i32) -> i32 {
    pow.div_euclid(5)
}

/// Starting Sanity: `= POW`, capped at 99.
pub fn starting_sanity(pow: i32) -> i32 {
    pow.min(99)
}

/// Damage Bonus and Build, from `STR + SIZ` (spec § 7).
///
/// The top band is implemented as the open-ended formula from
/// `docs/context/90-backend-implications.md` § B rather than a lookup table, so oversized
/// entities keep working: `n = (sum - 45) // 80` gives `nD6` with Build `n + 1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageBonus {
    pub damage_bonus: String,
    pub build: i32,
}

pub fn damage_bonus_and_build(str_value: i32, siz: i32) -> DamageBonus {
    let sum = str_value + siz;
    let (damage_bonus, build) = if sum < 65 {
        ("-2".to_string(), -2)
    } else if sum < 85 {
        ("-1".to_string(), -1)
    } else if sum < 125 {
        ("0".to_string(), 0)
    } else if sum < 165 {
        ("+1D4".to_string(), 1)
    } else {
        let n = (sum - 45).div_euclid(80);
        (format!("+{n}D6"), n + 1)
    };
    DamageBonus {
        damage_bonus,
        build,
    }
}

/// Move rate before the age penalty: 9 when DEX and STR both beat SIZ, 8 when either matches
/// or beats it, otherwise 7.
pub fn move_base(str_value: i32, dex: i32, siz: i32) -> i32 {
    if dex > siz && str_value > siz {
        9
    } else if dex >= siz || str_value >= siz {
        8
    } else {
        7
    }
}

/// Move rate after the age penalty (spec § 2: -1 in the 40s rising to -5 in the 80s).
pub fn move_rate(str_value: i32, dex: i32, siz: i32, age: i32) -> i32 {
    (move_base(str_value, dex, siz) - age_effects(age).mov_penalty).max(0)
}

/// Everything an age implies (spec § 2). All values are positive magnitudes to subtract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeEffects {
    pub band: AgeBand,
    /// Number of EDU life-experience checks granted.
    pub edu_checks: usize,
    /// Flat EDU reduction (Young only).
    pub edu_penalty: i32,
    /// Points to remove from STR/CON/DEX (Seasoned and Venerable).
    pub physical_deduction: i32,
    /// Points to remove from STR and SIZ (Young).
    pub young_deduction: i32,
    /// APP reduction.
    pub app_reduction: i32,
    /// MOV penalty.
    pub mov_penalty: i32,
    /// Luck rolls granted (Young rolls twice and keeps the better).
    pub luck_rolls: usize,
}

/// Age arithmetic. Ages below 20 are Young; 90+ is out of scope for v1 and clamps to the 80s.
pub fn age_effects(age: i32) -> AgeEffects {
    let band = AgeBand::from_age(age);
    let decade = age.div_euclid(10).clamp(1, 8);
    match band {
        AgeBand::Young => AgeEffects {
            band,
            edu_checks: 0,
            edu_penalty: 5,
            physical_deduction: 0,
            young_deduction: 5,
            app_reduction: 0,
            mov_penalty: 0,
            luck_rolls: 2,
        },
        AgeBand::Prime => AgeEffects {
            band,
            edu_checks: 1,
            edu_penalty: 0,
            physical_deduction: 0,
            young_deduction: 0,
            app_reduction: 0,
            mov_penalty: 0,
            luck_rolls: 1,
        },
        AgeBand::Seasoned | AgeBand::Venerable => AgeEffects {
            band,
            // min(4, (age // 10) - 2): 2 in the 40s, 3 in the 50s, 4 from 60 up.
            edu_checks: (decade - 2).clamp(0, 4) as usize,
            edu_penalty: 0,
            // 5 * 2^((age // 10) - 4): 5, 10, 20, 40, 80.
            physical_deduction: 5 * 2i32.pow((decade - 4).max(0) as u32),
            young_deduction: 0,
            // ((age // 10) - 3) * 5: 5, 10, 15, 20, 25.
            app_reduction: (decade - 3) * 5,
            mov_penalty: decade - 3,
            luck_rolls: 1,
        },
    }
}

pub fn edu_check_count(age: i32) -> usize {
    age_effects(age).edu_checks
}

/// Credit Rating standard of living, in our own words and our own numbers (spec § 6 sets the
/// value; `docs/context/90-backend-implications.md` § B supplies the tier shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    pub credit_rating: i32,
    pub descriptor: String,
    /// Cash on hand for a typical week.
    pub spending_level: i32,
    /// Everything else you could liquidate.
    pub assets: i32,
}

pub fn wallet(credit_rating: i32) -> Wallet {
    let cr = credit_rating.clamp(0, 99);
    let (descriptor, spending_level, assets) = match cr {
        0 => ("Penniless - you own what you are wearing.", 0, 0),
        1..=9 => ("Poor - rent is a monthly emergency.", cr, cr * 10),
        10..=49 => ("Average - you get by, with nothing spare.", cr * 2, cr * 50),
        50..=89 => (
            "Comfortable - you can absorb a bad week.",
            cr * 5,
            cr * 500,
        ),
        90..=98 => (
            "Wealthy - money is not one of your problems.",
            cr * 20,
            cr * 2000,
        ),
        _ => ("Rich - you have people for this.", 2500, 5_000_000),
    };
    Wallet {
        credit_rating: cr,
        descriptor: descriptor.to_string(),
        spending_level,
        assets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_and_fifth_floor() {
        assert_eq!(half(75), 37);
        assert_eq!(half(0), 0);
        assert_eq!(fifth(75), 15);
        assert_eq!(fifth(4), 0);
        assert_eq!(fifth(99), 19);
    }

    #[test]
    fn hit_points_floor_the_tenth() {
        assert_eq!(hit_points(50, 50), 10);
        assert_eq!(hit_points(55, 54), 10); // 109 // 10
        assert_eq!(hit_points(15, 40), 5); // 55 // 10
        assert_eq!(hit_points(0, 0), 0);
    }

    #[test]
    fn magic_points_and_sanity_track_pow() {
        assert_eq!(magic_points(75), 15);
        assert_eq!(magic_points(74), 14);
        assert_eq!(starting_sanity(75), 75);
        assert_eq!(starting_sanity(99), 99);
        assert_eq!(starting_sanity(120), 99);
    }

    #[test]
    fn damage_bonus_banding_edges() {
        // Every boundary in the spec § 7 ladder.
        assert_eq!(damage_bonus_and_build(20, 44).build, -2); // 64
        assert_eq!(damage_bonus_and_build(20, 44).damage_bonus, "-2");
        assert_eq!(damage_bonus_and_build(25, 40).build, -1); // 65
        assert_eq!(damage_bonus_and_build(40, 44).build, -1); // 84
        assert_eq!(damage_bonus_and_build(45, 40).build, 0); // 85
        assert_eq!(damage_bonus_and_build(60, 64).build, 0); // 124
        assert_eq!(damage_bonus_and_build(60, 65).damage_bonus, "+1D4"); // 125
        assert_eq!(damage_bonus_and_build(60, 65).build, 1);
        assert_eq!(damage_bonus_and_build(80, 84).build, 1); // 164
        assert_eq!(damage_bonus_and_build(80, 85).damage_bonus, "+1D6"); // 165
        assert_eq!(damage_bonus_and_build(80, 85).build, 2);
        assert_eq!(damage_bonus_and_build(100, 104).build, 2); // 204
    }

    #[test]
    fn damage_bonus_is_open_ended_past_the_printed_table() {
        // 205 -> n = (205 - 45) // 80 = 2 -> +2D6 / Build 3.
        let big = damage_bonus_and_build(105, 100);
        assert_eq!(big.damage_bonus, "+2D6");
        assert_eq!(big.build, 3);
        // 285 -> n = 3.
        let bigger = damage_bonus_and_build(145, 140);
        assert_eq!(bigger.damage_bonus, "+3D6");
        assert_eq!(bigger.build, 4);
    }

    #[test]
    fn move_comparisons_treat_ties_correctly() {
        // Both strictly greater -> 9.
        assert_eq!(move_base(60, 60, 50), 9);
        // DEX ties SIZ, STR below -> 8 (the "or equal" branch).
        assert_eq!(move_base(40, 50, 50), 8);
        // STR ties SIZ, DEX below -> 8.
        assert_eq!(move_base(50, 40, 50), 8);
        // DEX equals SIZ and STR equals SIZ -> 8, not 9.
        assert_eq!(move_base(50, 50, 50), 8);
        // Both below -> 7.
        assert_eq!(move_base(45, 45, 50), 7);
        // One strictly above but the other below -> 8.
        assert_eq!(move_base(80, 40, 50), 8);
    }

    #[test]
    fn move_rate_subtracts_the_age_penalty_and_floors_at_zero() {
        assert_eq!(move_rate(60, 60, 50, 27), 9);
        assert_eq!(move_rate(60, 60, 50, 45), 8);
        assert_eq!(move_rate(60, 60, 50, 55), 7);
        assert_eq!(move_rate(60, 60, 50, 65), 6);
        assert_eq!(move_rate(60, 60, 50, 75), 5);
        assert_eq!(move_rate(60, 60, 50, 85), 4);
        assert_eq!(move_rate(15, 15, 90, 85), 2); // base 7 - 5
    }

    #[test]
    fn age_effects_match_the_spec_table() {
        let young = age_effects(17);
        assert_eq!(young.band, AgeBand::Young);
        assert_eq!(young.edu_checks, 0);
        assert_eq!(young.edu_penalty, 5);
        assert_eq!(young.young_deduction, 5);
        assert_eq!(young.luck_rolls, 2);
        assert_eq!(young.app_reduction, 0);

        let prime = age_effects(27);
        assert_eq!(prime.edu_checks, 1);
        assert_eq!(prime.physical_deduction, 0);
        assert_eq!(prime.app_reduction, 0);
        assert_eq!(prime.mov_penalty, 0);
        assert_eq!(prime.luck_rolls, 1);

        let forties = age_effects(48);
        assert_eq!(forties.edu_checks, 2);
        assert_eq!(forties.physical_deduction, 5);
        assert_eq!(forties.app_reduction, 5);
        assert_eq!(forties.mov_penalty, 1);

        let fifties = age_effects(55);
        assert_eq!(fifties.edu_checks, 3);
        assert_eq!(fifties.physical_deduction, 10);
        assert_eq!(fifties.app_reduction, 10);
        assert_eq!(fifties.mov_penalty, 2);

        let sixties = age_effects(65);
        assert_eq!(sixties.band, AgeBand::Venerable);
        assert_eq!(sixties.edu_checks, 4);
        assert_eq!(sixties.physical_deduction, 20);
        assert_eq!(sixties.app_reduction, 15);
        assert_eq!(sixties.mov_penalty, 3);

        let seventies = age_effects(72);
        assert_eq!(seventies.edu_checks, 4);
        assert_eq!(seventies.physical_deduction, 40);
        assert_eq!(seventies.app_reduction, 20);
        assert_eq!(seventies.mov_penalty, 4);

        let eighties = age_effects(85);
        assert_eq!(eighties.edu_checks, 4);
        assert_eq!(eighties.physical_deduction, 80);
        assert_eq!(eighties.app_reduction, 25);
        assert_eq!(eighties.mov_penalty, 5);
    }

    #[test]
    fn band_boundaries_flip_exactly_on_the_decade() {
        assert_eq!(age_effects(39).edu_checks, 1);
        assert_eq!(age_effects(40).edu_checks, 2);
        assert_eq!(age_effects(40).physical_deduction, 5);
        assert_eq!(age_effects(59).physical_deduction, 10);
        assert_eq!(age_effects(60).physical_deduction, 20);
        assert_eq!(age_effects(19).edu_penalty, 5);
        assert_eq!(age_effects(20).edu_penalty, 0);
    }

    #[test]
    fn wallet_tiers_read_in_plain_words() {
        assert_eq!(wallet(0).spending_level, 0);
        assert!(wallet(0).descriptor.starts_with("Penniless"));
        assert_eq!(wallet(9).spending_level, 9);
        assert_eq!(wallet(9).assets, 90);
        assert_eq!(wallet(30).spending_level, 60);
        assert_eq!(wallet(30).assets, 1500);
        assert_eq!(wallet(50).spending_level, 250);
        assert_eq!(wallet(50).assets, 25_000);
        assert_eq!(wallet(90).spending_level, 1800);
        assert_eq!(wallet(99).assets, 5_000_000);
        assert!(wallet(75).descriptor.starts_with("Comfortable"));
    }
}
