//! §3 principle 5 / §9 — "Deterministic everything. Same answers -> same sheet, always."
//!
//! The stored record is the *answers*; the sheet is a pure projection over them. So projecting
//! twice, and projecting a JSON round-trip of the same answers, must be byte-identical.

use coc_phoenix::content::starter_catalog;
use coc_phoenix::model::Answers;
use coc_phoenix::projection::project;

use crate::common::{chars8, draft, orchard_catalog};

fn a_full_draft() -> Answers {
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
    answers
}

#[test]
fn spec3_identical_answers_project_an_identical_sheet_every_time() {
    let catalog = starter_catalog();
    let answers = a_full_draft();

    let first = project(&catalog, &answers);
    for _ in 0..20 {
        assert_eq!(project(&catalog, &answers), first);
    }
}

#[test]
fn spec9_answers_survive_a_json_round_trip_and_reproject_the_same_sheet() {
    let catalog = starter_catalog();
    let answers = a_full_draft();
    let first = serde_json::to_string(&project(&catalog, &answers)).unwrap();

    let encoded = serde_json::to_string(&answers).unwrap();
    let decoded: Answers = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, answers, "the answers record is the whole story (§9)");

    let again = serde_json::to_string(&project(&catalog, &decoded)).unwrap();
    assert_eq!(again, first);
}

#[test]
fn spec2_going_back_and_changing_an_answer_is_not_destructive() {
    // "changing any earlier answer re-renders everything downstream. Nothing is destroyed by
    // going back." — switching career and switching back must return the original sheet.
    let catalog = orchard_catalog();
    let hobbies = ["kite-days", "star-nights", "cider-tasting", "wall-scrambles"];
    let original = draft(
        "Solveig Amrit-Barlowe",
        48,
        chars8([50, 50, 55, 55, 55, 60, 50, 60]),
        Some("orchard-master"),
        &hobbies,
        &[(70, 5), (20, 5)],
        &[45],
    );
    let before = project(&catalog, &original);

    let mut detour = original.clone();
    detour.career_id = Some("estate-steward".to_string());
    let detoured = project(&catalog, &detour);
    assert_ne!(detoured, before, "the detour must actually change something");

    let mut back = detour.clone();
    back.career_id = Some("orchard-master".to_string());
    assert_eq!(project(&catalog, &back), before);
}

#[test]
fn spec6_the_allocator_uses_no_randomness_across_the_whole_career_roster() {
    let catalog = starter_catalog();
    for career in catalog.careers() {
        let mut answers = draft(
            "Wren Ashcombe-Lyle",
            27,
            chars8([55, 60, 60, 70, 45, 65, 55, 50]),
            Some(&career.id),
            &["bookworm", "poker-nights", "tinkering", "social-butterfly"],
            &[(45, 6)],
            &[55],
        );
        for slot in &career.choice_slots {
            answers
                .career_choices
                .insert(slot.id.clone(), slot.options[0].id.clone());
        }
        let first = project(&catalog, &answers);
        for _ in 0..5 {
            assert_eq!(project(&catalog, &answers), first, "{}", career.id);
        }
        assert!(
            first.completeness.ready_to_finalize,
            "{}: {:?}",
            career.id,
            first.completeness.missing
        );
    }
}
