//! Detail level for compute report filtering.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Controls how much detail a compute report includes.
///
/// Ordered from least to most verbose. The `filter()` method on
/// `ComputeReport` strips fields above the requested level.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl fmt::Display for DetailLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

impl FromStr for DetailLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            other => Err(format!("unknown detail level: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_info() {
        assert_eq!(DetailLevel::default(), DetailLevel::Info);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", DetailLevel::Error), "error");
        assert_eq!(format!("{}", DetailLevel::Warn), "warn");
        assert_eq!(format!("{}", DetailLevel::Info), "info");
        assert_eq!(format!("{}", DetailLevel::Debug), "debug");
        assert_eq!(format!("{}", DetailLevel::Trace), "trace");
    }

    #[test]
    fn test_from_str_basic() {
        assert_eq!("error".parse::<DetailLevel>().unwrap(), DetailLevel::Error);
        assert_eq!("warn".parse::<DetailLevel>().unwrap(), DetailLevel::Warn);
        assert_eq!("info".parse::<DetailLevel>().unwrap(), DetailLevel::Info);
        assert_eq!("debug".parse::<DetailLevel>().unwrap(), DetailLevel::Debug);
        assert_eq!("trace".parse::<DetailLevel>().unwrap(), DetailLevel::Trace);
    }

    #[test]
    fn test_from_str_warning_alias() {
        assert_eq!("warning".parse::<DetailLevel>().unwrap(), DetailLevel::Warn);
    }

    #[test]
    fn test_from_str_case_insensitive() {
        assert_eq!("ERROR".parse::<DetailLevel>().unwrap(), DetailLevel::Error);
        assert_eq!("Info".parse::<DetailLevel>().unwrap(), DetailLevel::Info);
        assert_eq!("DEBUG".parse::<DetailLevel>().unwrap(), DetailLevel::Debug);
        assert_eq!("Warning".parse::<DetailLevel>().unwrap(), DetailLevel::Warn);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("garbage".parse::<DetailLevel>().is_err());
        assert!("".parse::<DetailLevel>().is_err());
    }

    #[test]
    fn test_ordering() {
        assert!(DetailLevel::Error < DetailLevel::Warn);
        assert!(DetailLevel::Warn < DetailLevel::Info);
        assert!(DetailLevel::Info < DetailLevel::Debug);
        assert!(DetailLevel::Debug < DetailLevel::Trace);
    }

    #[test]
    fn test_serde_roundtrip() {
        let level = DetailLevel::Debug;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"debug\"");
        let deserialized: DetailLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, DetailLevel::Debug);
    }

    #[test]
    fn test_serde_all_variants() {
        for (variant, expected) in [
            (DetailLevel::Error, "\"error\""),
            (DetailLevel::Warn, "\"warn\""),
            (DetailLevel::Info, "\"info\""),
            (DetailLevel::Debug, "\"debug\""),
            (DetailLevel::Trace, "\"trace\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }
}
