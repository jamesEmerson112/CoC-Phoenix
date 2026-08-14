//! Three complete, hand-computed investigator sheets.
//!
//! Every number below was derived from the spec (§2, §4, §5, §6, §7) with the arithmetic written
//! out in the comments. The characters are invented; the careers and hobbies are the §4/§5
//! rosters as shipped in the bundled starter pack, and the skill *base* percentages are content
//! data read from that pack (the spec fixes only Dodge = DEX // 2 and Language (Own) = EDU).

use coc_phoenix::content::starter_catalog;
use coc_phoenix::model::Characteristic;
use coc_phoenix::projection::project;

use crate::common::{
    assert_only_these_were_touched, career_pts, characteristic, chars8, draft, personal_pts, value,
};

// =============================================================================================
// GOLDEN SHEET 1 — "Ilse Marchetti-Vaughn", 27, Police Detective
// =============================================================================================
//
// §2  Prime (20-39): one EDU check, no deductions, one Luck roll.
// §3  Rolled: STR 55 · CON 60 · SIZ 60 · DEX 70 · APP 45 · POW 65 · INT 55 · EDU 50
//     EDU check d100 45; 45 <= 50 so it fails and EDU stays 50.
//     Luck 3D6x5 = 55.
//
// §4  Police Detective = EDU x2 + INT x2 = (50 + 55) x 2 = 210.
//     CR range minimum 20 -> 190 for the ladder (§6.1).
//     Six slots (Psychology + 5), ladder 30/18/16/14/12/10 of 190:
//         Psychology         190 * 30 / 100 = 57.0  -> 57
//         Firearms (Handgun) 190 * 18 / 100 = 34.2  -> 34
//         Law                190 * 16 / 100 = 30.4  -> 30
//         Listen             190 * 14 / 100 = 26.6  -> 26
//         Spot Hidden        190 * 12 / 100 = 22.8  -> 22
//         Intimidate         190 * 10 / 100 = 19.0  -> 19
//         sum 188 -> 2 points of rounding remainder to the signature: Psychology 59.
//
// §6  Personal pool = INT x 2 = 110, split 40/30/20/10 = 44 / 33 / 22 / 11 (sum 110, no
//     global remainder). Within each hobby 70/30, remainder to the primary:
//         1 Urban exploration  44 -> Stealth        30 (+1) = 31 · Climb          13
//         2 Amateur boxing     33 -> Fighting(Brawl)23 (+1) = 24 · Dodge           9
//         3 Bookworm           22 -> Library Use    15 (+1) = 16 · History         6
//         4 Amateur gastronome 11 -> A/C (Cooking)   7 (+1) =  8 · Natural World   3
//
// §7  HP (60 + 60) // 10 = 12 · MP 65 // 5 = 13 · SAN = POW = 65
//     STR + SIZ = 115 -> 85-124 band -> 0 / 0
//     MOV: DEX 70 > SIZ 60 but STR 55 is not > 60, so not 9; DEX >= SIZ -> 8; no age penalty.
//     Dodge base = 70 // 2 = 35 · Language (Own) base = EDU = 50
// =============================================================================================

#[test]
fn golden_sheet_one_ordinary_career() {
    let catalog = starter_catalog();
    let answers = draft(
        "Ilse Marchetti-Vaughn",
        27,
        chars8([55, 60, 60, 70, 45, 65, 55, 50]),
        Some("police-detective"),
        &[
            "urban-exploration",
            "amateur-boxing",
            "bookworm",
            "amateur-gastronome",
        ],
        &[(45, 6)],
        &[55],
    );
    let sheet = project(&catalog, &answers);
    assert!(
        sheet.completeness.ready_to_finalize,
        "still missing: {:?}",
        sheet.completeness.missing
    );

    // -- characteristics, with half and fifth (§7) --------------------------------------
    for (id, v, h, f) in [
        ("str", 55, 27, 11),
        ("con", 60, 30, 12),
        ("siz", 60, 30, 12),
        ("dex", 70, 35, 14),
        ("app", 45, 22, 9),
        ("pow", 65, 32, 13),
        ("int", 55, 27, 11),
        ("edu", 50, 25, 10),
    ] {
        let c = sheet.characteristics.iter().find(|c| c.id == id).unwrap();
        assert_eq!((c.value, c.half, c.fifth), (v, h, f), "{id}");
    }
    assert!(
        sheet.age_adjustments.is_empty(),
        "Prime costs nothing and the EDU check failed: {:?}",
        sheet.age_adjustments
    );

    // -- pools (§6) ----------------------------------------------------------------------
    assert_eq!(sheet.pools.career_pool, 210);
    assert_eq!(sheet.pools.credit_rating_cost, 20);
    assert_eq!(sheet.pools.career_pool_spent, 210);
    assert_eq!(sheet.pools.personal_pool, 110);
    assert_eq!(sheet.pools.personal_pool_spent, 110);
    assert_eq!(sheet.credit_rating.value, 20);
    assert_eq!(sheet.credit_rating.half, 10);
    assert_eq!(sheet.credit_rating.fifth, 4);

    // -- career ladder ---------------------------------------------------------------------
    assert_eq!(career_pts(&sheet, "psychology"), 59, "57 + 2 remainder");
    assert_eq!(career_pts(&sheet, "firearms-handgun"), 34);
    assert_eq!(career_pts(&sheet, "law"), 30);
    assert_eq!(career_pts(&sheet, "listen"), 26);
    assert_eq!(career_pts(&sheet, "spot-hidden"), 22);
    assert_eq!(career_pts(&sheet, "intimidate"), 19);

    // -- personal pool ----------------------------------------------------------------------
    assert_eq!(personal_pts(&sheet, "stealth"), 31);
    assert_eq!(personal_pts(&sheet, "climb"), 13);
    assert_eq!(personal_pts(&sheet, "fighting-brawl"), 24);
    assert_eq!(personal_pts(&sheet, "dodge"), 9);
    assert_eq!(personal_pts(&sheet, "library-use"), 16);
    assert_eq!(personal_pts(&sheet, "history"), 6);
    assert_eq!(personal_pts(&sheet, "art-craft-cooking"), 8);
    assert_eq!(personal_pts(&sheet, "natural-world"), 3);

    // -- final skill values (base + career + personal, nothing over the cap) ---------------
    for (id, expected) in [
        ("psychology", 10 + 59),
        ("firearms-handgun", 10 + 34),
        ("law", 5 + 30),
        ("listen", 20 + 26),
        ("spot-hidden", 20 + 22),
        ("intimidate", 10 + 19),
        ("stealth", 20 + 31),
        ("climb", 20 + 13),
        ("fighting-brawl", 20 + 24),
        ("dodge", 35 + 9),
        ("library-use", 20 + 16),
        ("history", 5 + 6),
        ("art-craft-cooking", 5 + 8),
        ("natural-world", 10 + 3),
        ("language-own", 50),
    ] {
        assert_eq!(value(&sheet, id), expected, "{id}");
    }
    assert!(
        sheet.skills.iter().all(|s| !s.capped),
        "no skill on this sheet reaches 75"
    );
    assert_only_these_were_touched(
        &sheet,
        &[
            "psychology",
            "firearms-handgun",
            "law",
            "listen",
            "spot-hidden",
            "intimidate",
            "stealth",
            "climb",
            "fighting-brawl",
            "dodge",
            "library-use",
            "history",
            "art-craft-cooking",
            "natural-world",
        ],
    );

    // -- derived (§7) -----------------------------------------------------------------------
    assert_eq!(sheet.derived.hit_points, 12);
    assert_eq!(sheet.derived.magic_points, 13);
    assert_eq!(sheet.derived.sanity, 65);
    assert_eq!(sheet.derived.luck, Some(55));
    assert_eq!(sheet.derived.luck_bonus, 0);
    assert_eq!(sheet.derived.damage_bonus, "0");
    assert_eq!(sheet.derived.build, 0);
    assert_eq!(sheet.derived.move_base, 8);
    assert_eq!(sheet.derived.move_age_penalty, 0);
    assert_eq!(sheet.derived.move_rate, 8);
    assert_eq!(sheet.derived.dodge_base, 35);
    assert_eq!(sheet.derived.language_own_base, 50);
}

// =============================================================================================
// GOLDEN SHEET 2 — "Teodor Blanchflower-Ng", 27, Jack of All Trades
// =============================================================================================
//
// §3  Rolled: STR 45 · CON 45 · SIZ 50 · DEX 45 · APP 40 · POW 55 · INT 40 · EDU 40
//     EDU check d100 35; 35 <= 40 so it fails. Luck 3D6x5 = 70.
//
// §4.1 Pool = EDU x2 + (highest remaining characteristic) x2. The highest non-EDU stat is
//     POW 55, so the pool is 40 x 2 + 55 x 2 = 190. CR minimum 9 -> 181 to spend.
//     Fixed everyday skills take 40% as 12/10/10/8 of 181:
//         Drive Auto  181 * 12 / 100 = 21.72 -> 21
//         First Aid   181 * 10 / 100 = 18.10 -> 18
//         Spot Hidden                          18
//         Listen      181 *  8 / 100 = 14.48 -> 14      (subtotal 71)
//     Hobby primaries take 60% as 24/16/12/8 of 181:
//         1 Occult                181 * 24 / 100 = 43.44 -> 43
//         2 Psychology            181 * 16 / 100 = 28.96 -> 28
//         3 Stealth               181 * 12 / 100 = 21.72 -> 21
//         4 A/C (Photography)     181 *  8 / 100 = 14.48 -> 14   (subtotal 106)
//     71 + 106 = 177 -> 4 points of remainder to the FIRST-PICK PRIMARY: Occult 47.
//     Perk: +10 Luck (cap 99) -> 70 + 10 = 80.
//
// §6  Personal pool = INT x 2 = 80, split 32 / 24 / 16 / 8:
//         1 Occult curiosity   32 -> Occult      22 (+1) = 23 · Library Use   9
//         2 Poker nights       24 -> Psychology  16 (+1) = 17 · Sleight       7
//         3 Urban exploration  16 -> Stealth     11 (+1) = 12 · Climb         4
//         4 Photography         8 -> A/C(Photo)   5 (+1) =  6 · Spot Hidden   2
//
//     Occult finishes at base 5 + 47 + 23 = 75 — exactly on the cap, so nothing is lost.
//
// §7  HP (45 + 50) // 10 = 9 · MP 55 // 5 = 11 · SAN 55
//     STR + SIZ = 95 -> 0 / 0
//     MOV: neither DEX 45 nor STR 45 reaches SIZ 50 -> 7, no age penalty.
//     Dodge base = 45 // 2 = 22 · Language (Own) = 40
// =============================================================================================

#[test]
fn golden_sheet_two_jack_of_all_trades() {
    let catalog = starter_catalog();
    let answers = draft(
        "Teodor Blanchflower-Ng",
        27,
        chars8([45, 45, 50, 45, 40, 55, 40, 40]),
        Some("jack-of-all-trades"),
        &[
            "occult-curiosity",
            "poker-nights",
            "urban-exploration",
            "photography",
        ],
        &[(35, 6)],
        &[70],
    );
    let sheet = project(&catalog, &answers);
    assert!(
        sheet.completeness.ready_to_finalize,
        "still missing: {:?}",
        sheet.completeness.missing
    );

    // -- §4.1 pool and "best other" ---------------------------------------------------------
    let career = sheet.career.as_ref().expect("career view");
    assert_eq!(career.id, "jack-of-all-trades");
    assert_eq!(career.pool, 190, "EDU 40 x2 + POW 55 x2");
    assert_eq!(career.best_other, Some(Characteristic::Pow));
    assert_eq!(career.luck_bonus, Some(10));
    assert_eq!(sheet.pools.credit_rating_cost, 9);
    assert_eq!(sheet.pools.career_pool_spent, 190);
    assert_eq!(sheet.pools.personal_pool, 80);
    assert_eq!(sheet.pools.personal_pool_spent, 80);
    assert_eq!(sheet.credit_rating.value, 9);

    // -- §4.1 fixed 12/10/10/8 and hobby-primary 24/16/12/8 ---------------------------------
    assert_eq!(career_pts(&sheet, "drive-auto"), 21);
    assert_eq!(career_pts(&sheet, "first-aid"), 18);
    assert_eq!(career_pts(&sheet, "spot-hidden"), 18);
    assert_eq!(career_pts(&sheet, "listen"), 14);
    assert_eq!(career_pts(&sheet, "occult"), 47, "43 + 4 remainder");
    assert_eq!(career_pts(&sheet, "psychology"), 28);
    assert_eq!(career_pts(&sheet, "stealth"), 21);
    assert_eq!(career_pts(&sheet, "art-craft-photography"), 14);

    // -- personal pool -----------------------------------------------------------------------
    assert_eq!(personal_pts(&sheet, "occult"), 23);
    assert_eq!(personal_pts(&sheet, "library-use"), 9);
    assert_eq!(personal_pts(&sheet, "psychology"), 17);
    assert_eq!(personal_pts(&sheet, "sleight-of-hand"), 7);
    assert_eq!(personal_pts(&sheet, "stealth"), 12);
    assert_eq!(personal_pts(&sheet, "climb"), 4);
    assert_eq!(personal_pts(&sheet, "art-craft-photography"), 6);
    assert_eq!(personal_pts(&sheet, "spot-hidden"), 2);

    // -- final skill values -------------------------------------------------------------------
    for (id, expected) in [
        ("drive-auto", 20 + 21),
        ("first-aid", 20 + 18),
        ("spot-hidden", 20 + 18 + 2),
        ("listen", 20 + 14),
        ("occult", 5 + 47 + 23),
        ("psychology", 10 + 28 + 17),
        ("stealth", 20 + 21 + 12),
        ("art-craft-photography", 5 + 14 + 6),
        ("library-use", 20 + 9),
        ("sleight-of-hand", 10 + 7),
        ("climb", 20 + 4),
        ("dodge", 22),
        ("language-own", 40),
    ] {
        assert_eq!(value(&sheet, id), expected, "{id}");
    }
    let occult = sheet.skills.iter().find(|s| s.id == "occult").unwrap();
    assert_eq!(occult.value, 75, "exactly on the cap");
    assert!(!occult.capped, "landing on 75 is not overflowing it");
    assert_eq!(occult.points_lost_to_cap, 0);

    assert_only_these_were_touched(
        &sheet,
        &[
            "drive-auto",
            "first-aid",
            "spot-hidden",
            "listen",
            "occult",
            "psychology",
            "stealth",
            "art-craft-photography",
            "library-use",
            "sleight-of-hand",
            "climb",
        ],
    );

    // -- derived -------------------------------------------------------------------------------
    assert_eq!(sheet.derived.hit_points, 9);
    assert_eq!(sheet.derived.magic_points, 11);
    assert_eq!(sheet.derived.sanity, 55);
    assert_eq!(sheet.derived.luck, Some(80), "70 rolled + the 10-point perk");
    assert_eq!(sheet.derived.luck_bonus, 10);
    assert_eq!(sheet.derived.damage_bonus, "0");
    assert_eq!(sheet.derived.build, 0);
    assert_eq!(sheet.derived.move_base, 7);
    assert_eq!(sheet.derived.move_rate, 7);
    assert_eq!(sheet.derived.dodge_base, 22);
    assert_eq!(sheet.derived.language_own_base, 40);
}

// =============================================================================================
// GOLDEN SHEET 3 — "Ondrej Valparaiso-Kim", 65, Professor (Chemistry)
// =============================================================================================
//
// §2  Venerable, 60s: 4 EDU checks · physicals -20 · APP -15 · MOV -3.
// §3  Rolled: STR 50 · CON 45 · SIZ 60 · DEX 55 · APP 60 · POW 65 · INT 60 · EDU 65
//     APP 60 - 15 = 45.
//     §6 physical deduction: Professor's formula is EDU x4 and none of Library Use, History,
//     Language (Other), Science (Chemistry), Persuade or Psychology leans on a physical
//     characteristic, so STR/CON/DEX are equally unneeded and the CON -> STR -> DEX tie order
//     puts the whole -20 on CON: 45 - 20 = 25.
//     EDU checks, sequential, each applying only when d100 > current EDU:
//         80 > 65 -> 65 + 7 = 72
//         60 <= 72 -> no gain
//         95 > 72 -> 72 + 3 = 75
//         76 > 75 -> 75 + 6 = 81
//     Luck 3D6x5 = 40 (Venerable rolls once).
//
// §4  Professor = EDU x4 = 81 x 4 = 324 on the POST-AGE EDU. CR minimum 20 -> 304.
//     Six slots, 30/18/16/14/12/10 of 304:
//         Library Use          91.20 -> 91
//         History              54.72 -> 54
//         Language (Other)     48.64 -> 48
//         Science (Chemistry)  42.56 -> 42
//         Persuade             36.48 -> 36
//         Psychology           30.40 -> 30
//         sum 301 -> 3 remainder to the signature: Library Use 94.
//
// §6  Personal pool = INT x 2 = 120, split 48 / 36 / 24 / 12:
//         1 Bookworm          48 -> Library Use 33 (+1) = 34 · History        14
//         2 Occult curiosity  36 -> Occult      25 (+1) = 26 · Library Use    10
//         3 People-watching   24 -> Spot Hidden 16 (+1) = 17 · Psychology      7
//         4 Poker nights      12 -> Psychology   8 (+1) =  9 · Sleight         3
//
//     Career/hobby overlap on Library Use: base 20 + 94 career + 44 personal = 158, clipped to
//     the 75 cap with 83 points lost and NOT redistributed. History lands at 5 + 54 + 14 = 73,
//     just under the cap, and keeps every point.
//
// §7  HP (25 + 60) // 10 = 8 · MP 65 // 5 = 13 · SAN 65
//     STR + SIZ = 110 -> 0 / 0
//     MOV: DEX 55 and STR 50 are both below SIZ 60 -> base 7, minus the 60s penalty 3 -> 4.
//     Dodge base = 55 // 2 = 27 · Language (Own) base = EDU = 81
// =============================================================================================

#[test]
fn golden_sheet_three_venerable_with_deductions_and_a_capped_overlap() {
    let catalog = starter_catalog();
    let mut answers = draft(
        "Ondrej Valparaiso-Kim",
        65,
        chars8([50, 45, 60, 55, 60, 65, 60, 65]),
        Some("professor"),
        &[
            "bookworm",
            "occult-curiosity",
            "people-watching",
            "poker-nights",
        ],
        &[(80, 7), (60, 4), (95, 3), (76, 6)],
        &[40],
    );
    answers
        .career_choices
        .insert("science".to_string(), "chemistry".to_string());

    let sheet = project(&catalog, &answers);
    assert!(
        sheet.completeness.ready_to_finalize,
        "still missing: {:?}",
        sheet.completeness.missing
    );

    // -- §2 age view --------------------------------------------------------------------------
    let age = sheet.age.as_ref().expect("age view");
    assert_eq!(age.age, 65);
    assert_eq!(age.edu_checks_granted, 4);
    assert_eq!(age.luck_rolls_granted, 1);

    // -- post-age characteristics ---------------------------------------------------------------
    for (id, v, h, f) in [
        ("str", 50, 25, 10),
        ("con", 25, 12, 5),
        ("siz", 60, 30, 12),
        ("dex", 55, 27, 11),
        ("app", 45, 22, 9),
        ("pow", 65, 32, 13),
        ("int", 60, 30, 12),
        ("edu", 81, 40, 16),
    ] {
        let c = sheet.characteristics.iter().find(|c| c.id == id).unwrap();
        assert_eq!((c.value, c.half, c.fifth), (v, h, f), "{id}");
    }
    assert_eq!(characteristic(&sheet, "con"), 25, "the -20 landed entirely on CON");

    // The ledger of what age did, summed per characteristic.
    let delta = |c: Characteristic| -> i32 {
        sheet
            .age_adjustments
            .iter()
            .filter(|a| a.characteristic == c)
            .map(|a| a.delta)
            .sum()
    };
    assert_eq!(delta(Characteristic::App), -15);
    assert_eq!(delta(Characteristic::Con), -20);
    assert_eq!(delta(Characteristic::Edu), 16, "65 -> 81 across four checks");
    assert_eq!(delta(Characteristic::Str), 0);
    assert_eq!(delta(Characteristic::Dex), 0);

    // -- pools --------------------------------------------------------------------------------
    assert_eq!(sheet.pools.career_pool, 324, "EDU x4 on the POST-age EDU 81");
    assert_eq!(sheet.pools.credit_rating_cost, 20);
    assert_eq!(sheet.pools.career_pool_spent, 324);
    assert_eq!(sheet.pools.personal_pool, 120);
    assert_eq!(sheet.pools.personal_pool_spent, 120);
    assert_eq!(sheet.credit_rating.value, 20);

    // -- career ladder, with the choice slot resolved to Chemistry ---------------------------
    assert_eq!(career_pts(&sheet, "library-use"), 94, "91 + 3 remainder");
    assert_eq!(career_pts(&sheet, "history"), 54);
    assert_eq!(career_pts(&sheet, "language-other"), 48);
    assert_eq!(career_pts(&sheet, "science-chemistry"), 42);
    assert_eq!(career_pts(&sheet, "persuade"), 36);
    assert_eq!(career_pts(&sheet, "psychology"), 30);
    for unchosen in ["science-biology", "science-physics", "science-astronomy"] {
        assert_eq!(career_pts(&sheet, unchosen), 0, "{unchosen} was not the chosen field");
    }

    // -- personal pool --------------------------------------------------------------------------
    assert_eq!(personal_pts(&sheet, "library-use"), 34 + 10);
    assert_eq!(personal_pts(&sheet, "history"), 14);
    assert_eq!(personal_pts(&sheet, "occult"), 26);
    assert_eq!(personal_pts(&sheet, "spot-hidden"), 17);
    assert_eq!(personal_pts(&sheet, "psychology"), 7 + 9);
    assert_eq!(personal_pts(&sheet, "sleight-of-hand"), 3);

    // -- final skill values, including the cap --------------------------------------------------
    let library = sheet.skills.iter().find(|s| s.id == "library-use").unwrap();
    assert_eq!(library.base, 20);
    assert_eq!(library.value, 75, "20 + 94 + 44 = 158, clipped");
    assert!(library.capped);
    assert_eq!(library.points_lost_to_cap, 83);

    for (id, expected) in [
        ("history", 5 + 54 + 14),
        ("language-other", 1 + 48),
        ("science-chemistry", 1 + 42),
        ("persuade", 10 + 36),
        ("psychology", 10 + 30 + 16),
        ("occult", 5 + 26),
        ("spot-hidden", 20 + 17),
        ("sleight-of-hand", 10 + 3),
        ("dodge", 27),
    ] {
        assert_eq!(value(&sheet, id), expected, "{id}");
    }
    assert_eq!(value(&sheet, "history"), 73, "just under the cap, so nothing is lost");
    assert!(!sheet.skills.iter().find(|s| s.id == "history").unwrap().capped);

    // Points lost to the cap went nowhere: exactly one skill is capped on this sheet.
    assert_eq!(
        sheet.skills.iter().filter(|s| s.capped).count(),
        1,
        "only Library Use overflowed"
    );

    assert_only_these_were_touched(
        &sheet,
        &[
            "library-use",
            "history",
            "language-other",
            "science-chemistry",
            "persuade",
            "psychology",
            "occult",
            "spot-hidden",
            "sleight-of-hand",
            // Language (Own) receives no allocation; its base is EDU = 81, which §7 sets
            // directly. The §6 cap governs what allocation may *buy*, so it is checked
            // separately below rather than through the untouched-skill rule.
            "language-own",
        ],
    );
    assert_eq!(
        crate::common::base(&sheet, "language-own"),
        81,
        "§7: Language (Own) base = EDU"
    );

    // -- derived, including the MOV age penalty ------------------------------------------------
    assert_eq!(sheet.derived.hit_points, 8);
    assert_eq!(sheet.derived.magic_points, 13);
    assert_eq!(sheet.derived.sanity, 65);
    assert_eq!(sheet.derived.luck, Some(40));
    assert_eq!(sheet.derived.luck_bonus, 0);
    assert_eq!(sheet.derived.damage_bonus, "0");
    assert_eq!(sheet.derived.build, 0);
    assert_eq!(sheet.derived.move_base, 7);
    assert_eq!(sheet.derived.move_age_penalty, 3);
    assert_eq!(sheet.derived.move_rate, 4);
    assert_eq!(sheet.derived.dodge_base, 27);
    assert_eq!(sheet.derived.language_own_base, 81);
}
