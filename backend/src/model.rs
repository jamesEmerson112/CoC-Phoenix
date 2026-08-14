//! Answers-as-inputs data shapes (spec § 9).
//!
//! Everything a character *is* lives in [`Answers`]. The sheet is never stored while drafting —
//! it is projected from these answers by pure functions (`allocator.rs`, `derived.rs`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The eight rolled characteristics, in sheet order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Characteristic {
    Str,
    Con,
    Siz,
    Dex,
    App,
    Pow,
    Int,
    Edu,
}

impl Characteristic {
    pub const ALL: [Characteristic; 8] = [
        Characteristic::Str,
        Characteristic::Con,
        Characteristic::Siz,
        Characteristic::Dex,
        Characteristic::App,
        Characteristic::Pow,
        Characteristic::Int,
        Characteristic::Edu,
    ];

    /// Rolled with 3D6 x 5 (range 15-90). The rest use (2D6+6) x 5 (range 40-90).
    pub const THREE_D6: [Characteristic; 5] = [
        Characteristic::Str,
        Characteristic::Con,
        Characteristic::Dex,
        Characteristic::App,
        Characteristic::Pow,
    ];

    pub const TWO_D6_PLUS_6: [Characteristic; 3] =
        [Characteristic::Siz, Characteristic::Int, Characteristic::Edu];

    pub fn key(self) -> &'static str {
        match self {
            Characteristic::Str => "str",
            Characteristic::Con => "con",
            Characteristic::Siz => "siz",
            Characteristic::Dex => "dex",
            Characteristic::App => "app",
            Characteristic::Pow => "pow",
            Characteristic::Int => "int",
            Characteristic::Edu => "edu",
        }
    }

    pub fn from_key(key: &str) -> Option<Characteristic> {
        Characteristic::ALL.into_iter().find(|c| c.key() == key)
    }

    pub fn is_three_d6(self) -> bool {
        Characteristic::THREE_D6.contains(&self)
    }

    /// Inclusive legal range for a declared (hand-rolled) value, before age adjustment.
    pub fn declared_range(self) -> (i32, i32) {
        if self.is_three_d6() {
            (15, 90)
        } else {
            (40, 90)
        }
    }
}

/// A full set of characteristic values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Characteristics {
    #[serde(rename = "str")]
    pub strength: i32,
    pub con: i32,
    pub siz: i32,
    pub dex: i32,
    pub app: i32,
    pub pow: i32,
    #[serde(rename = "int")]
    pub intelligence: i32,
    pub edu: i32,
}

impl Characteristics {
    pub fn get(&self, c: Characteristic) -> i32 {
        match c {
            Characteristic::Str => self.strength,
            Characteristic::Con => self.con,
            Characteristic::Siz => self.siz,
            Characteristic::Dex => self.dex,
            Characteristic::App => self.app,
            Characteristic::Pow => self.pow,
            Characteristic::Int => self.intelligence,
            Characteristic::Edu => self.edu,
        }
    }

    pub fn set(&mut self, c: Characteristic, v: i32) {
        match c {
            Characteristic::Str => self.strength = v,
            Characteristic::Con => self.con = v,
            Characteristic::Siz => self.siz = v,
            Characteristic::Dex => self.dex = v,
            Characteristic::App => self.app = v,
            Characteristic::Pow => self.pow = v,
            Characteristic::Int => self.intelligence = v,
            Characteristic::Edu => self.edu = v,
        }
    }

    pub fn add(&mut self, c: Characteristic, delta: i32) {
        let v = self.get(c) + delta;
        self.set(c, v);
    }
}

/// Age bands (spec § 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgeBand {
    Young,
    Prime,
    Seasoned,
    Venerable,
}

impl AgeBand {
    /// Ages outside 15-89 are out of scope for v1 (spec § 10); they clamp to the nearest band.
    pub fn from_age(age: i32) -> AgeBand {
        match age {
            i32::MIN..=19 => AgeBand::Young,
            20..=39 => AgeBand::Prime,
            40..=59 => AgeBand::Seasoned,
            _ => AgeBand::Venerable,
        }
    }

    /// The representative age the wizard offers when the player picks a band but no exact age.
    pub fn default_age(self) -> i32 {
        match self {
            AgeBand::Young => 17,
            AgeBand::Prime => 27,
            AgeBand::Seasoned => 48,
            AgeBand::Venerable => 65,
        }
    }

    pub fn range(self) -> (i32, i32) {
        match self {
            AgeBand::Young => (15, 19),
            AgeBand::Prime => (20, 39),
            AgeBand::Seasoned => (40, 59),
            AgeBand::Venerable => (60, 89),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            AgeBand::Young => "young",
            AgeBand::Prime => "prime",
            AgeBand::Seasoned => "seasoned",
            AgeBand::Venerable => "venerable",
        }
    }
}

/// Where a die result came from (spec § 3, "I rolled at home").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DiceSource {
    #[default]
    Server,
    Declared,
}

/// Provenance per dice block. The three ceremonies are run separately and a player may mix
/// them — declare characteristics rolled at home, then tap the server Luck ceremony for the
/// animation (spec § 3c stages Luck as its own moment). A single flag cannot describe that
/// record honestly, so the truth is recorded per block and [`DiceAnswers::source`] summarises it.
///
/// `None` means the block has not been rolled or declared yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DiceSources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characteristics: Option<DiceSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edu_checks: Option<DiceSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub luck: Option<DiceSource>,
}

impl DiceSources {
    pub fn is_empty(&self) -> bool {
        self.characteristics.is_none() && self.edu_checks.is_none() && self.luck.is_none()
    }

    fn each(&self) -> [Option<DiceSource>; 3] {
        [self.characteristics, self.edu_checks, self.luck]
    }

    /// The single-flag summary the spec's § 9 record carries: `declared` as soon as any part of
    /// the record was hand-entered, `server` when every recorded block came from the server.
    /// Never claims a hand-entered block was server-rolled.
    pub fn summary(&self) -> DiceSource {
        if self.each().iter().flatten().any(|s| *s == DiceSource::Declared) {
            DiceSource::Declared
        } else {
            DiceSource::Server
        }
    }
}

/// One EDU life-experience check. `gain` is the 1D10 that was rolled; whether it is *applied*
/// is re-derived on projection (`d100 > current EDU`), so the record stays replayable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EduCheck {
    pub d100: i32,
    pub gain: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LuckAnswer {
    /// One roll, or two for Young characters (keep the better).
    pub rolls: Vec<i32>,
    /// Echo of what the ceremony kept. The projection recomputes this; it is stored for replay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kept: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DiceAnswers {
    /// Summary provenance for the whole record; see [`DiceSources::summary`]. Kept because the
    /// spec § 9 record names it, but `sources` is the authority.
    pub source: DiceSource,
    /// Per-block provenance (spec § 3 principle 3, "honest dice").
    #[serde(skip_serializing_if = "DiceSources::is_empty")]
    pub sources: DiceSources,
    /// RAW rolled characteristics, *before* any age adjustment. Age deductions depend on the
    /// chosen career (spec § 6) and are therefore applied by the projection, not stored here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characteristics: Option<Characteristics>,
    /// Per-characteristic individual dice, kept so the UI can replay the tumble.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub characteristic_rolls: BTreeMap<String, Vec<i32>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub edu_checks: Vec<EduCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub luck: Option<LuckAnswer>,
}

impl DiceAnswers {
    /// Record where one block's numbers came from and refresh the summary flag.
    pub fn record_source(&mut self, block: DiceBlock, source: DiceSource) {
        match block {
            DiceBlock::Characteristics => self.sources.characteristics = Some(source),
            DiceBlock::EduChecks => self.sources.edu_checks = Some(source),
            DiceBlock::Luck => self.sources.luck = Some(source),
        }
        self.source = self.sources.summary();
    }
}

/// The three dice ceremonies, as separately-sourced blocks of the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiceBlock {
    Characteristics,
    EduChecks,
    Luck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Backstory {
    /// "fate" (rolled) or "manual".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    pub rolls: BTreeMap<String, i32>,
    pub overrides: BTreeMap<String, String>,
}

/// The persisted answer record (spec § 9). `status` lives in its own column, not in here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Answers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronouns: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_band: Option<AgeBand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dice: Option<DiceAnswers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub career_id: Option<String>,
    pub career_choices: BTreeMap<String, String>,
    /// Pick order is priority (spec § 5/§ 6).
    pub hobby_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backstory: Option<Backstory>,
}

impl Answers {
    /// Effective age: the typed age if present, else the band's representative age.
    pub fn effective_age(&self) -> Option<i32> {
        self.age.or_else(|| self.age_band.map(|b| b.default_age()))
    }

    /// Effective band: derived from the typed age when present (age wins over band).
    pub fn effective_band(&self) -> Option<AgeBand> {
        match (self.age, self.age_band) {
            (Some(age), _) => Some(AgeBand::from_age(age)),
            (None, band) => band,
        }
    }

    pub fn dice_ref(&self) -> Option<&DiceAnswers> {
        self.dice.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Draft,
    Finalized,
}

impl Status {
    pub fn key(self) -> &'static str {
        match self {
            Status::Draft => "draft",
            Status::Finalized => "finalized",
        }
    }

    pub fn from_key(key: &str) -> Option<Status> {
        match key {
            "draft" => Some(Status::Draft),
            "finalized" => Some(Status::Finalized),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_bands_cover_the_spec_table() {
        assert_eq!(AgeBand::from_age(15), AgeBand::Young);
        assert_eq!(AgeBand::from_age(19), AgeBand::Young);
        assert_eq!(AgeBand::from_age(20), AgeBand::Prime);
        assert_eq!(AgeBand::from_age(39), AgeBand::Prime);
        assert_eq!(AgeBand::from_age(40), AgeBand::Seasoned);
        assert_eq!(AgeBand::from_age(59), AgeBand::Seasoned);
        assert_eq!(AgeBand::from_age(60), AgeBand::Venerable);
        assert_eq!(AgeBand::from_age(89), AgeBand::Venerable);
    }

    #[test]
    fn typed_age_overrides_a_stale_band() {
        let answers = Answers {
            age: Some(65),
            age_band: Some(AgeBand::Prime),
            ..Default::default()
        };
        assert_eq!(answers.effective_band(), Some(AgeBand::Venerable));
        assert_eq!(answers.effective_age(), Some(65));
    }

    #[test]
    fn band_without_age_uses_the_representative_age() {
        let answers = Answers {
            age_band: Some(AgeBand::Seasoned),
            ..Default::default()
        };
        assert_eq!(answers.effective_age(), Some(48));
    }

    #[test]
    fn characteristics_round_trip_with_spec_json_keys() {
        let json = r#"{"str":50,"con":65,"siz":60,"dex":55,"app":45,"pow":70,"int":75,"edu":80}"#;
        let c: Characteristics = serde_json::from_str(json).unwrap();
        assert_eq!(c.strength, 50);
        assert_eq!(c.intelligence, 75);
        assert_eq!(serde_json::to_string(&c).unwrap(), json);
    }

    #[test]
    fn mixed_provenance_is_recorded_per_block_and_never_relabelled() {
        let mut dice = DiceAnswers::default();
        dice.record_source(DiceBlock::Characteristics, DiceSource::Declared);
        assert_eq!(dice.sources.characteristics, Some(DiceSource::Declared));
        assert_eq!(dice.source, DiceSource::Declared);

        // Tapping the server Luck ceremony afterwards must not restamp the hand-entered stats.
        dice.record_source(DiceBlock::Luck, DiceSource::Server);
        assert_eq!(dice.sources.characteristics, Some(DiceSource::Declared));
        assert_eq!(dice.sources.luck, Some(DiceSource::Server));
        assert_eq!(dice.sources.edu_checks, None, "an untouched block stays unknown");
        assert_eq!(
            dice.source,
            DiceSource::Declared,
            "the summary stays honest: part of this record was hand-entered"
        );

        // ...and the reverse: server stats plus one declared Luck.
        let mut dice = DiceAnswers::default();
        dice.record_source(DiceBlock::Characteristics, DiceSource::Server);
        assert_eq!(dice.source, DiceSource::Server);
        dice.record_source(DiceBlock::Luck, DiceSource::Declared);
        assert_eq!(dice.sources.characteristics, Some(DiceSource::Server));
        assert_eq!(dice.source, DiceSource::Declared);
    }

    #[test]
    fn a_dice_record_without_per_block_sources_still_round_trips() {
        // Records written before per-block provenance existed keep working.
        let json = r#"{"source":"declared"}"#;
        let dice: DiceAnswers = serde_json::from_str(json).unwrap();
        assert_eq!(dice.source, DiceSource::Declared);
        assert!(dice.sources.is_empty());
        assert_eq!(serde_json::to_string(&dice).unwrap(), json);
    }

    #[test]
    fn declared_ranges_match_the_dice_used() {
        assert_eq!(Characteristic::Str.declared_range(), (15, 90));
        assert_eq!(Characteristic::Pow.declared_range(), (15, 90));
        assert_eq!(Characteristic::Siz.declared_range(), (40, 90));
        assert_eq!(Characteristic::Edu.declared_range(), (40, 90));
    }
}
