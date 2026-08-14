//! # Independent spec conformance suite
//!
//! Every expected value in this suite is hand-computed from
//! `docs/design/01-signup-wizard-spec.md` alone. Nothing here was read off the
//! implementation or off the implementation's own unit tests: where the two disagree, this
//! suite encodes the SPEC and is expected to go red.
//!
//! Fixtures are invented (an orchard/estate content pack, and invented investigator names).
//! No real people, no licensed entities, no rulebook text.
//!
//! Section references are to the spec:
//!   * §2  age bands
//!   * §3  dice ceremonies and declared-dice validation
//!   * §4 / §4.1 careers and Jack of All Trades
//!   * §5  hobbies
//!   * §6  deterministic allocation
//!   * §7  derived values
//!   * §9  answers-as-inputs / draft lifecycle

mod common;

mod allocation;
mod derivations;
mod determinism;
mod dice_rules;
mod golden_sheets;
mod invariants;
mod roster_and_lifecycle;
