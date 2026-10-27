//! Settings types. Each enum is a closed preset list; unknown values fail to parse and the
//! loader falls back to the default for that one field.

use chrono::Weekday;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TimeFormat {
    #[default]
    #[serde(rename = "24h")]
    H24,
    #[serde(rename = "12h")]
    H12,
}

impl TimeFormat {
    pub fn hour24(self) -> bool {
        self == TimeFormat::H24
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Accent {
    #[default]
    Red,
    Orange,
    Green,
    Teal,
    Blue,
    Indigo,
    Violet,
    Pink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Neutral {
    #[default]
    Cool,
    Neutral,
    Warm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarColor {
    Red,
    Orange,
    Amber,
    Green,
    Teal,
    Blue,
    Violet,
    Pink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarColors {
    pub work: CalendarColor,
    pub personal: CalendarColor,
    pub side: CalendarColor,
}

impl Default for CalendarColors {
    fn default() -> Self {
        Self { work: CalendarColor::Blue, personal: CalendarColor::Green, side: CalendarColor::Amber }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Appearance {
    pub mode: Mode,
    pub accent: Accent,
    pub neutral: Neutral,
    pub calendar_colors: CalendarColors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub week_start: Weekday,
    pub time_format: TimeFormat,
    pub appearance: Appearance,
}

impl Default for Settings {
    fn default() -> Self {
        Self { week_start: Weekday::Mon, time_format: TimeFormat::default(), appearance: Appearance::default() }
    }
}

/// One field of `obj`, `None` if it is missing or does not parse.
pub(super) fn get<T: serde::de::DeserializeOwned>(obj: Option<&Map<String, Value>>, key: &str) -> Option<T> {
    obj.and_then(|o| o.get(key)).and_then(|v| T::deserialize(v).ok())
}

/// One field of `obj`, or `default` if it is missing or does not parse; a present bad value sets `lossy`.
fn field<T: serde::de::DeserializeOwned>(obj: Option<&Map<String, Value>>, key: &str, default: T, lossy: &mut bool) -> T {
    match obj.and_then(|o| o.get(key)) {
        None => default,
        Some(v) => T::deserialize(v).unwrap_or_else(|_| {
            *lossy = true;
            default
        }),
    }
}

/// Nested object `key`; a present non-object sets `lossy`.
fn object<'a>(obj: Option<&'a Map<String, Value>>, key: &str, lossy: &mut bool) -> Option<&'a Map<String, Value>> {
    match obj.and_then(|o| o.get(key)) {
        None => None,
        Some(Value::Object(m)) => Some(m),
        Some(_) => {
            *lossy = true;
            None
        }
    }
}

/// Sets `lossy` if `obj` has a key outside `known`: it would be dropped by the next save.
pub(super) fn flag_unknown(obj: Option<&Map<String, Value>>, known: &[&str], lossy: &mut bool) {
    *lossy |= obj.is_some_and(|o| o.keys().any(|k| !known.contains(&k.as_str())));
}

impl CalendarColors {
    fn from_json(obj: Option<&Map<String, Value>>, lossy: &mut bool) -> Self {
        let d = Self::default();
        flag_unknown(obj, &["work", "personal", "side"], lossy);
        Self {
            work: field(obj, "work", d.work, lossy),
            personal: field(obj, "personal", d.personal, lossy),
            side: field(obj, "side", d.side, lossy),
        }
    }
}

impl Appearance {
    fn from_json(obj: Option<&Map<String, Value>>, lossy: &mut bool) -> Self {
        let d = Self::default();
        flag_unknown(obj, &["mode", "accent", "neutral", "calendar_colors"], lossy);
        Self {
            mode: field(obj, "mode", d.mode, lossy),
            accent: field(obj, "accent", d.accent, lossy),
            neutral: field(obj, "neutral", d.neutral, lossy),
            calendar_colors: CalendarColors::from_json(object(obj, "calendar_colors", lossy), lossy),
        }
    }
}

impl Settings {
    /// Appearance back to defaults; week start and time format are not appearance.
    pub fn reset_appearance(&mut self) {
        self.appearance = Appearance::default();
    }

    /// Per-field load: a bad or missing value resets only that field. The flag is true if
    /// anything present was replaced or dropped (bad value, non-object, unknown key); a
    /// missing value is not a repair.
    pub fn from_json(v: Option<&Value>) -> (Self, bool) {
        let mut lossy = false;
        let obj = match v {
            None => None,
            Some(Value::Object(m)) => Some(m),
            Some(_) => {
                lossy = true;
                None
            }
        };
        flag_unknown(obj, &["week_start", "time_format", "appearance"], &mut lossy);
        let d = Self::default();
        let s = Self {
            week_start: field(obj, "week_start", d.week_start, &mut lossy),
            time_format: field(obj, "time_format", d.time_format, &mut lossy),
            appearance: Appearance::from_json(object(obj, "appearance", &mut lossy), &mut lossy),
        };
        (s, lossy)
    }
}
