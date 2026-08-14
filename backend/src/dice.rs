//! Pure dice ceremonies and declared-result validation (spec § 3).
//!
//! Every function here is a pure function of the RNG it is handed, so tests seed a
//! deterministic RNG and assert exact results. No database, no clock, no globals.

use crate::model::{AgeBand, Characteristic, Characteristics, EduCheck, LuckAnswer};
use rand::Rng;
use std::collections::BTreeMap;

/// Characteristic values are always multiples of 5 (every roll is multiplied by 5).
pub const STEP: i32 = 5;
/// A d10 gain from an EDU life-experience check. A d10 reads 1..=10 — a physical die printed
/// 0-9 shows its `0` face for ten, so `0` is never a legal gain.
pub const EDU_GAIN_MIN: i32 = 1;
pub const EDU_GAIN_MAX: i32 = 10;
/// EDU cannot pass this at creation.
pub const EDU_CAP: i32 = 99;

/// Roll `n` six-sided dice, returning the individual faces (kept for the UI's tumble animation).
pub fn roll_dice<R: Rng + ?Sized>(rng: &mut R, n: usize, sides: i32) -> Vec<i32> {
    (0..n).map(|_| rng.gen_range(1..=sides)).collect()
}

/// 3D6 x 5 — STR, CON, DEX, APP, POW. Range 15-90.
pub fn roll_3d6_x5<R: Rng + ?Sized>(rng: &mut R) -> (i32, Vec<i32>) {
    let dice = roll_dice(rng, 3, 6);
    (dice.iter().sum::<i32>() * 5, dice)
}

/// (2D6 + 6) x 5 — SIZ, INT, EDU. Range 40-90.
pub fn roll_2d6_plus6_x5<R: Rng + ?Sized>(rng: &mut R) -> (i32, Vec<i32>) {
    let dice = roll_dice(rng, 2, 6);
    ((dice.iter().sum::<i32>() + 6) * 5, dice)
}

/// A percentile read in the domain 1..=100.
pub fn roll_d100<R: Rng + ?Sized>(rng: &mut R) -> i32 {
    rng.gen_range(1..=100)
}

pub fn roll_d10<R: Rng + ?Sized>(rng: &mut R) -> i32 {
    rng.gen_range(1..=10)
}

/// Ceremony 3a: the eight characteristics, raw (age adjustments are applied by the projection).
pub fn roll_characteristics<R: Rng + ?Sized>(
    rng: &mut R,
) -> (Characteristics, BTreeMap<String, Vec<i32>>) {
    let mut chars = Characteristics::default();
    let mut detail = BTreeMap::new();
    for c in Characteristic::ALL {
        let (value, dice) = if c.is_three_d6() {
            roll_3d6_x5(rng)
        } else {
            roll_2d6_plus6_x5(rng)
        };
        chars.set(c, value);
        detail.insert(c.key().to_string(), dice);
    }
    (chars, detail)
}

/// Ceremony 3b: one d100 + one d10 per check granted by the age band.
///
/// The d10 is rolled up front and stored even when the check misses, so that re-rolling the
/// characteristics later re-derives the same sequence deterministically (spec § 3 principle 5).
pub fn roll_edu_checks<R: Rng + ?Sized>(rng: &mut R, count: usize) -> Vec<EduCheck> {
    (0..count)
        .map(|_| EduCheck {
            d100: roll_d100(rng),
            gain: roll_d10(rng),
        })
        .collect()
}

/// Ceremony 3c: Luck. Young characters roll twice and keep the better (spec § 2).
pub fn roll_luck<R: Rng + ?Sized>(rng: &mut R, band: AgeBand) -> LuckAnswer {
    let count = luck_roll_count(band);
    let rolls: Vec<i32> = (0..count).map(|_| roll_3d6_x5(rng).0).collect();
    let kept = rolls.iter().copied().max();
    LuckAnswer { rolls, kept }
}

pub fn luck_roll_count(band: AgeBand) -> usize {
    if band == AgeBand::Young {
        2
    } else {
        1
    }
}

/// Which of the stored Luck rolls counts (before the Jack of All Trades perk).
pub fn kept_luck(rolls: &[i32], band: AgeBand) -> Option<i32> {
    if rolls.is_empty() {
        return None;
    }
    if band == AgeBand::Young {
        rolls.iter().copied().max()
    } else {
        rolls.first().copied()
    }
}

/// Apply the EDU life-experience sequence to a starting EDU value.
///
/// Strictly sequential: each success raises the bar for the next check. A check applies only
/// when `d100 > current EDU`. EDU is clamped at 99.
pub fn apply_edu_checks(starting_edu: i32, checks: &[EduCheck]) -> (i32, Vec<bool>) {
    let mut edu = starting_edu;
    let mut applied = Vec::with_capacity(checks.len());
    for check in checks {
        if check.d100 > edu {
            edu = (edu + check.gain).min(EDU_CAP);
            applied.push(true);
        } else {
            applied.push(false);
        }
    }
    (edu, applied)
}

/// A human-readable validation failure for declared ("rolled at home") results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceValidationError(pub String);

pub type DiceValidation = Result<(), Vec<DiceValidationError>>;

fn err(message: impl Into<String>) -> DiceValidationError {
    DiceValidationError(message.into())
}

/// Validate a hand-entered characteristic set: multiples of 5, 15-90 for 3D6x5 stats,
/// 40-90 for (2D6+6)x5 stats (spec § 3, manual entry mode).
pub fn validate_declared_characteristics(chars: &Characteristics) -> DiceValidation {
    let mut errors = Vec::new();
    for c in Characteristic::ALL {
        let value = chars.get(c);
        let (lo, hi) = c.declared_range();
        if value % STEP != 0 {
            errors.push(err(format!(
                "{} must be a multiple of {STEP} (got {value})",
                c.key().to_uppercase()
            )));
        }
        if value < lo || value > hi {
            errors.push(err(format!(
                "{} must be between {lo} and {hi} (got {value})",
                c.key().to_uppercase()
            )));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate hand-entered Luck: one roll, or two for Young characters.
pub fn validate_declared_luck(rolls: &[i32], band: AgeBand) -> DiceValidation {
    let mut errors = Vec::new();
    let allowed = luck_roll_count(band);
    if rolls.is_empty() || rolls.len() > allowed {
        errors.push(err(format!(
            "expected {allowed} Luck roll(s) for the {} band (got {})",
            band.key(),
            rolls.len()
        )));
    }
    for value in rolls {
        if value % STEP != 0 {
            errors.push(err(format!(
                "Luck must be a multiple of {STEP} (got {value})"
            )));
        }
        if !(15..=90).contains(value) {
            errors.push(err(format!("Luck must be between 15 and 90 (got {value})")));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate hand-entered EDU checks against the number the age band grants.
pub fn validate_declared_edu_checks(checks: &[EduCheck], allowed: usize) -> DiceValidation {
    let mut errors = Vec::new();
    if checks.len() > allowed {
        errors.push(err(format!(
            "this age band grants {allowed} EDU check(s) (got {})",
            checks.len()
        )));
    }
    for check in checks {
        if !(1..=100).contains(&check.d100) {
            errors.push(err(format!(
                "an EDU check roll must be between 1 and 100 (got {})",
                check.d100
            )));
        }
        if !(EDU_GAIN_MIN..=EDU_GAIN_MAX).contains(&check.gain) {
            errors.push(err(format!(
                "an EDU check gain must be between {EDU_GAIN_MIN} and {EDU_GAIN_MAX} (got {}); \
                 a die face printed '0' counts as 10",
                check.gain
            )));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn seeded() -> StdRng {
        StdRng::seed_from_u64(0xC0C_7E)
    }

    #[test]
    fn three_d6_x5_stays_in_range_and_on_the_step() {
        let mut rng = seeded();
        for _ in 0..2_000 {
            let (value, dice) = roll_3d6_x5(&mut rng);
            assert_eq!(dice.len(), 3);
            assert!((15..=90).contains(&value), "{value}");
            assert_eq!(value % 5, 0);
        }
    }

    #[test]
    fn two_d6_plus6_x5_stays_in_range_and_on_the_step() {
        let mut rng = seeded();
        for _ in 0..2_000 {
            let (value, dice) = roll_2d6_plus6_x5(&mut rng);
            assert_eq!(dice.len(), 2);
            assert!((40..=90).contains(&value), "{value}");
            assert_eq!(value % 5, 0);
        }
    }

    #[test]
    fn server_rolled_characteristics_always_pass_declared_validation() {
        let mut rng = seeded();
        for _ in 0..500 {
            let (chars, detail) = roll_characteristics(&mut rng);
            assert_eq!(detail.len(), 8);
            assert!(validate_declared_characteristics(&chars).is_ok());
        }
    }

    #[test]
    fn young_rolls_luck_twice_and_keeps_the_better() {
        let mut rng = seeded();
        for _ in 0..200 {
            let luck = roll_luck(&mut rng, AgeBand::Young);
            assert_eq!(luck.rolls.len(), 2);
            assert_eq!(luck.kept, Some(*luck.rolls.iter().max().unwrap()));
            assert!(luck.kept.unwrap() >= luck.rolls[0]);
        }
        let luck = roll_luck(&mut rng, AgeBand::Prime);
        assert_eq!(luck.rolls.len(), 1);
    }

    #[test]
    fn kept_luck_ignores_a_stray_second_roll_outside_the_young_band() {
        assert_eq!(kept_luck(&[40, 90], AgeBand::Prime), Some(40));
        assert_eq!(kept_luck(&[40, 90], AgeBand::Young), Some(90));
        assert_eq!(kept_luck(&[], AgeBand::Young), None);
    }

    #[test]
    fn edu_checks_are_sequential_and_raise_the_bar() {
        // Start at 60. First check 71 > 60 succeeds (+9 -> 69); second check 65 <= 69 fails.
        let checks = [
            EduCheck { d100: 71, gain: 9 },
            EduCheck { d100: 65, gain: 8 },
        ];
        let (edu, applied) = apply_edu_checks(60, &checks);
        assert_eq!(edu, 69);
        assert_eq!(applied, vec![true, false]);
    }

    #[test]
    fn edu_checks_clamp_at_99() {
        let checks = [EduCheck { d100: 100, gain: 10 }];
        let (edu, applied) = apply_edu_checks(95, &checks);
        assert_eq!(edu, 99);
        assert_eq!(applied, vec![true]);
    }

    #[test]
    fn an_equal_roll_is_not_an_improvement() {
        let checks = [EduCheck { d100: 60, gain: 10 }];
        let (edu, applied) = apply_edu_checks(60, &checks);
        assert_eq!(edu, 60);
        assert_eq!(applied, vec![false]);
    }

    #[test]
    fn declared_characteristics_reject_off_step_and_out_of_range_values() {
        let mut chars = Characteristics {
            strength: 50,
            con: 50,
            siz: 50,
            dex: 50,
            app: 50,
            pow: 50,
            intelligence: 50,
            edu: 50,
        };
        assert!(validate_declared_characteristics(&chars).is_ok());

        chars.strength = 52; // off the multiple-of-5 step
        let errors = validate_declared_characteristics(&chars).unwrap_err();
        assert!(errors.iter().any(|e| e.0.contains("multiple of 5")));

        chars.strength = 95; // above the 3D6x5 ceiling
        let errors = validate_declared_characteristics(&chars).unwrap_err();
        assert!(errors.iter().any(|e| e.0.contains("between 15 and 90")));

        chars.strength = 50;
        chars.siz = 35; // below the (2D6+6)x5 floor
        let errors = validate_declared_characteristics(&chars).unwrap_err();
        assert!(errors.iter().any(|e| e.0.contains("between 40 and 90")));

        chars.siz = 40;
        chars.edu = 35;
        assert!(validate_declared_characteristics(&chars).is_err());
    }

    #[test]
    fn boundary_values_are_accepted() {
        let chars = Characteristics {
            strength: 15,
            con: 90,
            siz: 40,
            dex: 15,
            app: 90,
            pow: 15,
            intelligence: 90,
            edu: 40,
        };
        assert!(validate_declared_characteristics(&chars).is_ok());
    }

    #[test]
    fn declared_luck_respects_the_band_roll_count() {
        assert!(validate_declared_luck(&[55], AgeBand::Prime).is_ok());
        assert!(validate_declared_luck(&[55, 70], AgeBand::Young).is_ok());
        assert!(validate_declared_luck(&[55, 70], AgeBand::Prime).is_err());
        assert!(validate_declared_luck(&[], AgeBand::Prime).is_err());
        assert!(validate_declared_luck(&[53], AgeBand::Prime).is_err());
        assert!(validate_declared_luck(&[95], AgeBand::Prime).is_err());
    }

    #[test]
    fn declared_edu_checks_respect_the_band_allowance() {
        let ok = [EduCheck { d100: 71, gain: 7 }];
        assert!(validate_declared_edu_checks(&ok, 2).is_ok());
        assert!(validate_declared_edu_checks(&ok, 0).is_err());
        let bad = [EduCheck { d100: 0, gain: 11 }];
        let errors = validate_declared_edu_checks(&bad, 1).unwrap_err();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn a_declared_edu_gain_of_zero_is_rejected_because_no_d10_rolls_it() {
        // 0 is the face a physical d10 printed 0-9 shows for ten. Accepting it silently costs
        // the player the whole gain, so it is refused with an explanation instead.
        let zero = [EduCheck { d100: 99, gain: 0 }];
        let errors = validate_declared_edu_checks(&zero, 1).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].0.contains("counts as 10"), "{}", errors[0].0);

        // The whole legal domain of a d10 is accepted, and it matches what the server rolls.
        for gain in EDU_GAIN_MIN..=EDU_GAIN_MAX {
            let check = [EduCheck { d100: 99, gain }];
            assert!(validate_declared_edu_checks(&check, 1).is_ok(), "{gain}");
        }
    }

    #[test]
    fn server_rolled_edu_checks_always_pass_declared_validation() {
        let mut rng = seeded();
        for _ in 0..500 {
            let checks = roll_edu_checks(&mut rng, 4);
            assert!(validate_declared_edu_checks(&checks, 4).is_ok());
        }
    }

    #[test]
    fn a_seeded_rng_reproduces_the_same_ceremony() {
        let a = roll_characteristics(&mut StdRng::seed_from_u64(7));
        let b = roll_characteristics(&mut StdRng::seed_from_u64(7));
        assert_eq!(a.0, b.0);
        assert_eq!(a.1, b.1);
    }
}
