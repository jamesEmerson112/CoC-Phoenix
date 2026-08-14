//! §6 — the deterministic allocator, on an invented content pack whose skill bases are all 0
//! (so a projected value is exactly the points allocated).
//!
//! Spec text being encoded:
//!   * Career pool = the career's formula on **final (post-age)** characteristics.
//!   * 1. Credit Rating = the career's range minimum, paid out of the career pool.
//!   * 2. Ladder the remainder, descending:
//!        5 slots 34/20/18/15/13 · 6 slots 30/18/16/14/12/10 · 7 slots 28/16/14/12/11/10/9
//!   * 3. Round down per skill; all rounding remainder goes to the signature skill.
//!   * Jack: fixed skills 40% (12/10/10/8), hobby primaries 60% (24/16/12/8),
//!     remainder to the first-pick primary.
//!   * Personal pool = INT x 2, split 40/30/20/10 by pick order, 70/30 within a hobby,
//!     rounding remainder to the primary, global remainder to the first pick's primary.
//!   * Stacking cap 75, points lost to the cap are NOT redistributed.
//!   * Age deductions come off whichever of STR/CON/DEX the career needs least,
//!     ties broken CON -> STR -> DEX.

use coc_phoenix::allocator::allocate;
use coc_phoenix::projection::project;

use crate::common::{chars8, draft, orchard_catalog, value};

const ALL_FOUR: [&str; 4] = ["kite-days", "star-nights", "cider-tasting", "wall-scrambles"];

// =============================================================================================
// §6.2 — the three ladders
// =============================================================================================

#[test]
fn spec6_six_slot_ladder_with_no_rounding_remainder() {
    let catalog = orchard_catalog();
    // orchard-master = EDU x4. EDU 80 -> pool 320. CR minimum 20 -> 300 to ladder.
    //   30% -> 90   18% -> 54   16% -> 48   14% -> 42   12% -> 36   10% -> 30
    //   90+54+48+42+36+30 = 300 exactly, so the signature gets no rounding bonus.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([50, 50, 50, 50, 50, 50, 50, 80]),
        Some("orchard-master"),
        &[],
        &[(20, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);

    assert_eq!(a.career_pool, 320, "EDU 80 x 4");
    assert_eq!(a.credit_rating, 20, "orchard-master CR minimum");
    assert_eq!(a.career_points["standing"], 20);
    assert_eq!(a.career_points["orchard-lore"], 90, "signature, 30%");
    assert_eq!(a.career_points["graft-work"], 54, "18%");
    assert_eq!(a.career_points["cider-pressing"], 48, "16%");
    assert_eq!(a.career_points["hive-tending"], 42, "14%");
    assert_eq!(a.career_points["weather-reading"], 36, "12%");
    assert_eq!(a.career_points["market-haggling"], 30, "10%");
    assert_eq!(a.career_pool_spent, 320, "CR + ladder consumes the whole pool");
}

#[test]
fn spec6_five_slot_ladder_and_its_rounding_remainder() {
    let catalog = orchard_catalog();
    // market-carter = DEX x2 + EDU x2. DEX 55, EDU 55 -> (55+55) x 2 = 220.
    // CR minimum 10 -> 210 to ladder.
    //   34% -> 71.4  -> 71
    //   20% -> 42.0  -> 42
    //   18% -> 37.8  -> 37
    //   15% -> 31.5  -> 31
    //   13% -> 27.3  -> 27
    //   sum 208, so 2 points of rounding remainder go to the signature: 71 + 2 = 73.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([50, 50, 50, 55, 50, 50, 50, 55]),
        Some("market-carter"),
        &[],
        &[(20, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);

    assert_eq!(a.career_pool, 220);
    assert_eq!(a.credit_rating, 10);
    assert_eq!(a.career_points["cart-driving"], 73, "71 + 2 rounding remainder");
    assert_eq!(a.career_points["ladder-work"], 42);
    assert_eq!(a.career_points["fence-mending"], 37);
    assert_eq!(a.career_points["weather-reading"], 31);
    assert_eq!(a.career_points["tune-humming"], 27);
    assert_eq!(a.career_pool_spent, 220);
    let total: i32 = a.career_points.values().sum();
    assert_eq!(total, 220, "nothing evaporates on the way out of the pool");
}

#[test]
fn spec6_seven_slot_ladder_and_its_rounding_remainder() {
    let catalog = orchard_catalog();
    // estate-steward = EDU x4. EDU 75 -> pool 300. CR minimum 30 -> 270 to ladder.
    //   28% -> 75.6 -> 75      16% -> 43.2 -> 43     14% -> 37.8 -> 37
    //   12% -> 32.4 -> 32      11% -> 29.7 -> 29     10% -> 27.0 -> 27
    //    9% -> 24.3 -> 24
    //   sum 267, so 3 points of rounding remainder go to the signature: 75 + 3 = 78.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([50, 50, 50, 50, 50, 50, 50, 75]),
        Some("estate-steward"),
        &[],
        &[(20, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);

    assert_eq!(a.career_pool, 300);
    assert_eq!(a.credit_rating, 30);
    assert_eq!(a.career_points["market-haggling"], 78, "75 + 3 rounding remainder");
    assert_eq!(a.career_points["orchard-lore"], 43);
    assert_eq!(a.career_points["graft-work"], 37);
    assert_eq!(a.career_points["hive-tending"], 32);
    assert_eq!(a.career_points["weather-reading"], 29);
    assert_eq!(a.career_points["night-watch"], 27);
    assert_eq!(a.career_points["ladder-work"], 24);
    assert_eq!(a.career_pool_spent, 300);
}

#[test]
fn spec6_credit_rating_minimum_is_taken_off_the_pool_before_the_ladder() {
    let catalog = orchard_catalog();
    // EDU 40 -> pool 160. CR minimum 20 -> only 140 reaches the skills.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([50, 50, 50, 50, 50, 50, 50, 40]),
        Some("orchard-master"),
        &[],
        &[(20, 5)],
        &[45],
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
    assert_eq!(
        skill_points, 140,
        "the ladder only ever sees pool - CR minimum"
    );

    // And the sheet shows the CR value in its own right.
    let sheet = project(&catalog, &answers);
    assert_eq!(sheet.credit_rating.value, 20);
    assert_eq!(sheet.pools.credit_rating_cost, 20);
}

// =============================================================================================
// §6 — the personal (hobby) pool
// =============================================================================================

#[test]
fn spec6_personal_pool_is_int_times_two_split_forty_thirty_twenty_ten() {
    let catalog = orchard_catalog();
    // INT 50 -> personal pool 100. Pick shares: 40 / 30 / 20 / 10, no remainder anywhere.
    //   pick 1 (40): primary 28, secondary 12
    //   pick 2 (30): primary 21, secondary  9
    //   pick 3 (20): primary 14, secondary  6
    //   pick 4 (10): primary  7, secondary  3
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([50, 50, 50, 50, 50, 50, 50, 50]),
        None,
        &ALL_FOUR,
        &[(20, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);

    assert_eq!(a.personal_pool, 100, "INT x 2");
    assert_eq!(a.personal_points["tune-humming"], 28);
    assert_eq!(a.personal_points["weather-reading"], 12);
    assert_eq!(a.personal_points["seed-saving"], 21);
    assert_eq!(a.personal_points["night-watch"], 9);
    assert_eq!(a.personal_points["beeswax-craft"], 14);
    assert_eq!(a.personal_points["hive-tending"], 6);
    assert_eq!(a.personal_points["ladder-work"], 7);
    assert_eq!(a.personal_points["fence-mending"], 3);
    assert_eq!(a.personal_pool_spent, 100);
}

#[test]
fn spec6_personal_pool_remainders_land_on_primaries_then_on_the_first_pick() {
    let catalog = orchard_catalog();
    // INT 47 -> personal pool 94.
    //   pick shares: 40% -> 37.6 -> 37 · 30% -> 28.2 -> 28 · 20% -> 18.8 -> 18 · 10% -> 9.4 -> 9
    //   sum 92, so 2 points of GLOBAL remainder go to the first pick's primary.
    //   within pick 1 (37): 70% -> 25.9 -> 25, 30% -> 11.1 -> 11; 25+11 = 36, +1 to primary = 26
    //   within pick 2 (28): 70% -> 19.6 -> 19, 30% ->  8.4 ->  8; +1 to primary = 20
    //   within pick 3 (18): 70% -> 12.6 -> 12, 30% ->  5.4 ->  5; +1 to primary = 13
    //   within pick 4  (9): 70% ->  6.3 ->  6, 30% ->  2.7 ->  2; +1 to primary =  7
    //   first-pick primary finally: 26 + 2 = 28
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([50, 50, 50, 50, 50, 50, 47, 50]),
        None,
        &ALL_FOUR,
        &[(20, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);

    assert_eq!(a.personal_pool, 94);
    assert_eq!(a.personal_points["tune-humming"], 28, "26 + 2 global remainder");
    assert_eq!(a.personal_points["weather-reading"], 11);
    assert_eq!(a.personal_points["seed-saving"], 20);
    assert_eq!(a.personal_points["night-watch"], 8);
    assert_eq!(a.personal_points["beeswax-craft"], 13);
    assert_eq!(a.personal_points["hive-tending"], 5);
    assert_eq!(a.personal_points["ladder-work"], 7);
    assert_eq!(a.personal_points["fence-mending"], 2);
    assert_eq!(a.personal_pool_spent, 94, "every point of the pool is spent");
}

#[test]
fn spec5_pick_order_is_priority_so_reordering_the_same_four_hobbies_changes_the_sheet() {
    let catalog = orchard_catalog();
    // "your first pick is your passion" — the same four hobbies in a different order must not
    // produce the same numbers.
    let chars = chars8([50, 50, 50, 50, 50, 50, 50, 50]);
    let forward = draft("Bettine Okonkwo-Sable", 27, chars, None, &ALL_FOUR, &[(20, 5)], &[45]);
    let reversed_ids = ["wall-scrambles", "cider-tasting", "star-nights", "kite-days"];
    let reversed = draft("Bettine Okonkwo-Sable", 27, chars, None, &reversed_ids, &[(20, 5)], &[45]);

    let a = allocate(&catalog, &forward);
    let b = allocate(&catalog, &reversed);

    assert_eq!(a.personal_points["tune-humming"], 28, "first pick takes 40%");
    assert_eq!(b.personal_points["tune-humming"], 7, "last pick takes 10%");
    assert_eq!(a.personal_points["ladder-work"], 7);
    assert_eq!(b.personal_points["ladder-work"], 28);
}

// =============================================================================================
// §6 — the 75 cap, without redistribution
// =============================================================================================

#[test]
fn spec6_a_single_skill_over_seventy_five_is_capped_and_nothing_is_redistributed() {
    let catalog = orchard_catalog();
    // orchard-master with EDU 90 -> pool 360, CR 20 -> 340 to the ladder.
    //   30% -> 102   18% -> 61.2 -> 61   16% -> 54.4 -> 54
    //   14% -> 47.6 -> 47   12% -> 40.8 -> 40   10% -> 34
    //   sum 338, +2 rounding remainder to the signature = 104.
    // Signature base 0 + 104 = 104 -> capped at 75, losing 29 points that go nowhere.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([50, 50, 50, 50, 50, 50, 50, 90]),
        Some("orchard-master"),
        &[],
        &[(20, 5)],
        &[45],
    );
    let sheet = project(&catalog, &answers);

    let signature = sheet
        .skills
        .iter()
        .find(|s| s.id == "orchard-lore")
        .expect("orchard-lore");
    assert_eq!(signature.career_points, 104, "the ledger still records what was spent");
    assert_eq!(signature.value, 75, "the creation cap");
    assert!(signature.capped);
    assert_eq!(signature.points_lost_to_cap, 29);

    // Not one of the lost points appears anywhere else.
    assert_eq!(value(&sheet, "graft-work"), 61);
    assert_eq!(value(&sheet, "cider-pressing"), 54);
    assert_eq!(value(&sheet, "hive-tending"), 47);
    assert_eq!(value(&sheet, "weather-reading"), 40);
    assert_eq!(value(&sheet, "market-haggling"), 34);
    assert_eq!(sheet.credit_rating.value, 20);
}

#[test]
fn spec6_career_and_hobby_points_stack_on_one_skill_up_to_the_cap() {
    let catalog = orchard_catalog();
    // market-carter (DEX 55 / EDU 55 -> pool 220, CR 10 -> 210) gives ladder-work 42 (20%).
    // The first hobby pick, wall-scrambles, has ladder-work as its primary.
    // INT 60 -> personal pool 120; first pick 40% = 48; 70% of 48 = 33.6 -> 33, +1 within-hobby
    // remainder = 34.
    // ladder-work therefore reaches 0 + 42 + 34 = 76, one point over the cap.
    let hobbies = ["wall-scrambles", "kite-days", "star-nights", "cider-tasting"];
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([50, 50, 50, 55, 50, 50, 60, 55]),
        Some("market-carter"),
        &hobbies,
        &[(20, 5)],
        &[45],
    );
    let sheet = project(&catalog, &answers);

    let stacked = sheet
        .skills
        .iter()
        .find(|s| s.id == "ladder-work")
        .expect("ladder-work");
    assert_eq!(stacked.career_points, 42);
    assert_eq!(stacked.personal_points, 34);
    assert_eq!(stacked.value, 75, "76 clipped to the cap");
    assert!(stacked.capped);
    assert_eq!(stacked.points_lost_to_cap, 1);

    // The lost point is not handed to the hobby's secondary, nor to any other skill.
    assert_eq!(value(&sheet, "fence-mending"), 37 + 14);
    assert_eq!(value(&sheet, "cart-driving"), 73);
    assert_eq!(value(&sheet, "weather-reading"), 31 + 10);
    assert_eq!(value(&sheet, "tune-humming"), 27 + 26);
    assert_eq!(value(&sheet, "seed-saving"), 17);
    assert_eq!(value(&sheet, "night-watch"), 7);
    assert_eq!(value(&sheet, "beeswax-craft"), 9);
    assert_eq!(value(&sheet, "hive-tending"), 3);

    // The pools are still fully spent — the cap does not refund.
    assert_eq!(sheet.pools.personal_pool, 120);
    assert_eq!(sheet.pools.personal_pool_spent, 120);
    assert_eq!(sheet.pools.career_pool, 220);
    assert_eq!(sheet.pools.career_pool_spent, 220);
}

#[test]
fn spec6_allocated_points_never_push_any_skill_past_seventy_five() {
    let catalog = orchard_catalog();
    // A deliberately absurd sheet: maximum pools, every hobby doubling down on career skills.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([90, 90, 90, 90, 90, 90, 90, 90]),
        Some("estate-steward"),
        &ALL_FOUR,
        &[(20, 5)],
        &[45],
    );
    let sheet = project(&catalog, &answers);
    for s in &sheet.skills {
        // Every base in this fixture pack is 0, so nothing here can legitimately start above
        // the cap; the cap must bind on every single skill.
        assert!(
            s.value <= 75,
            "{} reached {} — §6 caps creation values at 75",
            s.id,
            s.value
        );
    }
}

// =============================================================================================
// §4.1 / §6 — Jack of All Trades
// =============================================================================================

#[test]
fn spec41_jack_pool_is_edu_x2_plus_best_other_x2() {
    let catalog = orchard_catalog();
    // EDU 40 x2 = 80; the highest remaining characteristic is POW 55 -> 110. Pool = 190.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([45, 45, 50, 45, 40, 55, 40, 40]),
        Some("odd-hand"),
        &ALL_FOUR,
        &[(20, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);
    assert_eq!(a.career_pool, 190);
    assert_eq!(
        a.best_other,
        Some(coc_phoenix::model::Characteristic::Pow),
        "EDU is never its own 'best other'"
    );

    // EDU is excluded from the "highest remaining" search even when it is the highest stat.
    let edu_highest = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([45, 45, 50, 45, 40, 55, 40, 90]),
        Some("odd-hand"),
        &ALL_FOUR,
        &[(20, 5)],
        &[45],
    );
    let b = allocate(&catalog, &edu_highest);
    assert_eq!(b.best_other, Some(coc_phoenix::model::Characteristic::Pow));
    assert_eq!(b.career_pool, 90 * 2 + 55 * 2);
}

#[test]
fn spec41_jack_splits_forty_percent_fixed_and_sixty_percent_into_hobby_primaries() {
    let catalog = orchard_catalog();
    // Pool 190 (EDU 40 x2 + POW 55 x2), CR minimum 9 -> 181 to spend.
    //   fixed 12% -> 21.72 -> 21 · 10% -> 18.1 -> 18 · 10% -> 18 · 8% -> 14.48 -> 14   (= 71)
    //   hobby 24% -> 43.44 -> 43 · 16% -> 28.96 -> 28 · 12% -> 21.72 -> 21 · 8% -> 14  (= 106)
    //   71 + 106 = 177, so 4 points of remainder go to the FIRST-PICK PRIMARY: 43 + 4 = 47.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars8([45, 45, 50, 45, 40, 55, 40, 40]),
        Some("odd-hand"),
        &ALL_FOUR,
        &[(35, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);

    assert_eq!(a.credit_rating, 9);
    // The four fixed everyday skills.
    assert_eq!(a.career_points["graft-work"], 21, "fixed slot 1, 12%");
    assert_eq!(a.career_points["seed-saving"], 14 + 28, "fixed slot 4 (8%) + hobby 2 primary (16%)");
    // beeswax-craft is fixed slot 2 (10%) and also hobby 3's primary (12%): 18 + 21.
    assert_eq!(a.career_points["beeswax-craft"], 18 + 21);
    // tune-humming is fixed slot 3 (10%) and also hobby 1's primary (24%) + remainder 4.
    assert_eq!(a.career_points["tune-humming"], 18 + 43 + 4);
    // hobby 4's primary is ladder-work, which is not one of the fixed skills.
    assert_eq!(a.career_points["ladder-work"], 14);
    assert_eq!(a.career_pool_spent, 190, "CR 9 + 181 spent");
}

#[test]
fn spec41_jack_adds_ten_luck_capped_at_ninety_nine() {
    let catalog = orchard_catalog();
    let chars = chars8([45, 45, 50, 45, 40, 55, 40, 40]);

    let modest = draft("Bettine Okonkwo-Sable", 27, chars, Some("odd-hand"), &ALL_FOUR, &[(35, 5)], &[55]);
    let a = allocate(&catalog, &modest);
    assert_eq!(a.luck, Some(65), "55 rolled + 10 perk");
    assert_eq!(a.luck_bonus, 10);

    let lucky = draft("Bettine Okonkwo-Sable", 27, chars, Some("odd-hand"), &ALL_FOUR, &[(35, 5)], &[90]);
    assert_eq!(allocate(&catalog, &lucky).luck, Some(99), "90 + 10 clipped to 99");

    // No other career grants it.
    let ordinary = draft(
        "Bettine Okonkwo-Sable",
        27,
        chars,
        Some("orchard-master"),
        &ALL_FOUR,
        &[(35, 5)],
        &[55],
    );
    let c = allocate(&catalog, &ordinary);
    assert_eq!(c.luck, Some(55));
    assert_eq!(c.luck_bonus, 0);
}

// =============================================================================================
// §6 — age deductions
// =============================================================================================

#[test]
fn spec6_age_deduction_tie_breaks_con_then_str_then_dex() {
    let catalog = orchard_catalog();
    // orchard-master's formula is EDU x4 and none of its six skills lean on a physical
    // characteristic, so STR, CON and DEX are all equally unneeded -> CON pays first.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        48,
        chars8([50, 50, 50, 50, 50, 50, 50, 60]),
        Some("orchard-master"),
        &[],
        &[(70, 5), (20, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);
    assert_eq!(a.characteristics.con, 45, "the whole -5 comes off CON");
    assert_eq!(a.characteristics.strength, 50);
    assert_eq!(a.characteristics.dex, 50);
}

#[test]
fn spec6_age_deduction_avoids_what_the_career_actually_needs() {
    let catalog = orchard_catalog();
    // estate-steward's skill list contains night-watch (CON) and ladder-work (DEX) but nothing
    // that leans on STR, so STR is the least-needed and pays the -5.
    let answers = draft(
        "Bettine Okonkwo-Sable",
        48,
        chars8([50, 50, 50, 50, 50, 50, 50, 60]),
        Some("estate-steward"),
        &[],
        &[(70, 5), (20, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);
    assert_eq!(a.characteristics.strength, 45, "STR is the cheapest place to take it");
    assert_eq!(a.characteristics.con, 50);
    assert_eq!(a.characteristics.dex, 50);
}

#[test]
fn spec6_age_deduction_is_recomputed_when_the_career_changes_mid_draft() {
    let catalog = orchard_catalog();
    let chars = chars8([50, 50, 50, 50, 50, 50, 50, 60]);
    let checks = [(70, 5), (20, 5)];

    let as_orchard_master = allocate(
        &catalog,
        &draft("Bettine Okonkwo-Sable", 48, chars, Some("orchard-master"), &[], &checks, &[45]),
    );
    let as_steward = allocate(
        &catalog,
        &draft("Bettine Okonkwo-Sable", 48, chars, Some("estate-steward"), &[], &checks, &[45]),
    );

    assert_eq!(as_orchard_master.characteristics.con, 45);
    assert_eq!(as_orchard_master.characteristics.strength, 50);
    assert_eq!(as_steward.characteristics.con, 50);
    assert_eq!(as_steward.characteristics.strength, 45);
}

#[test]
fn spec2_seasoned_and_venerable_pay_app_as_well_as_physicals() {
    let catalog = orchard_catalog();
    let chars = chars8([60, 60, 60, 60, 60, 60, 60, 60]);

    // 40s: physicals -5, APP -5.
    let a = allocate(
        &catalog,
        &draft("Bettine Okonkwo-Sable", 48, chars, None, &[], &[(70, 5), (20, 5)], &[45]),
    );
    assert_eq!(a.characteristics.app, 55);
    assert_eq!(a.characteristics.con, 55);

    // 50s: physicals -10, APP -10.
    let b = allocate(
        &catalog,
        &draft("Bettine Okonkwo-Sable", 55, chars, None, &[], &[(70, 5), (20, 5), (21, 5)], &[45]),
    );
    assert_eq!(b.characteristics.app, 50);
    assert_eq!(b.characteristics.con, 50);

    // 60s: physicals -20, APP -15.
    let c = allocate(
        &catalog,
        &draft("Bettine Okonkwo-Sable", 65, chars, None, &[], &[(70, 5), (20, 5), (21, 5), (22, 5)], &[45]),
    );
    assert_eq!(c.characteristics.app, 45);
    assert_eq!(c.characteristics.con, 40);
}

#[test]
fn spec6_the_career_pool_is_evaluated_on_post_age_characteristics() {
    let catalog = orchard_catalog();
    // A 48-year-old market-carter: DEX is in the formula, and the -5 is steered away from it
    // (nothing in the pack makes CON valuable to this career), so the pool holds at 220...
    let answers = draft(
        "Bettine Okonkwo-Sable",
        48,
        chars8([50, 50, 50, 55, 50, 50, 50, 55]),
        Some("market-carter"),
        &[],
        &[(20, 5), (21, 5)],
        &[45],
    );
    let a = allocate(&catalog, &answers);
    assert_eq!(a.characteristics.dex, 55);
    assert_eq!(a.career_pool, (55 + 55) * 2);

    // ...but a successful EDU check raises EDU before the formula runs, so the pool grows.
    let educated = draft(
        "Bettine Okonkwo-Sable",
        48,
        chars8([50, 50, 50, 55, 50, 50, 50, 55]),
        Some("market-carter"),
        &[],
        &[(99, 5), (21, 5)],
        &[45],
    );
    let b = allocate(&catalog, &educated);
    assert_eq!(b.characteristics.edu, 60);
    assert_eq!(b.career_pool, (55 + 60) * 2, "EDU 60 x2 + DEX 55 x2");
}
