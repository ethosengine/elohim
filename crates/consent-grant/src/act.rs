//! What the terminal is asking a controller to sign.
//!
//! Acts travel as short strings (OAuth calls them scopes) so a consent screen
//! can list them and a log can name them. Each parses to one variant; a string
//! this crate does not recognise is refused, never ignored, because an ignored
//! act would be consent the person never saw.

use serde::{Deserialize, Serialize};

/// Longest content item id an act may name.
pub const MAX_ITEM_ID_LEN: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum RequestedAct {
    /// Bind this device's key to the person's identity (`device.enroll`).
    EnrollDevice,
    /// Delegate the head of one content root to this device
    /// (`content.head:<item-id>`).
    DelegateHead { item_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActRefusal {
    Unknown,
    ItemIdMalformed,
}

impl ActRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::Unknown => "act_unknown",
            Self::ItemIdMalformed => "act_item_id_malformed",
        }
    }
}

const ENROLL: &str = "device.enroll";
const HEAD_PREFIX: &str = "content.head:";

impl RequestedAct {
    pub fn parse(text: &str) -> Result<Self, ActRefusal> {
        if text == ENROLL {
            return Ok(Self::EnrollDevice);
        }
        if let Some(item_id) = text.strip_prefix(HEAD_PREFIX) {
            let well_formed = !item_id.is_empty()
                && item_id.len() <= MAX_ITEM_ID_LEN
                && item_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
            return if well_formed {
                Ok(Self::DelegateHead {
                    item_id: item_id.to_string(),
                })
            } else {
                Err(ActRefusal::ItemIdMalformed)
            };
        }
        Err(ActRefusal::Unknown)
    }
}

impl std::fmt::Display for RequestedAct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnrollDevice => f.write_str(ENROLL),
            Self::DelegateHead { item_id } => write!(f, "{HEAD_PREFIX}{item_id}"),
        }
    }
}

impl TryFrom<String> for RequestedAct {
    type Error = String;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text).map_err(|r| r.code().to_string())
    }
}

impl From<RequestedAct> for String {
    fn from(act: RequestedAct) -> Self {
        act.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acts_round_trip_through_their_strings() {
        for text in ["device.enroll", "content.head:fct-module-01"] {
            assert_eq!(RequestedAct::parse(text).unwrap().to_string(), text);
        }
    }

    #[test]
    fn an_unknown_act_is_refused_not_ignored() {
        assert_eq!(
            RequestedAct::parse("device.admin"),
            Err(ActRefusal::Unknown)
        );
        assert_eq!(RequestedAct::parse(""), Err(ActRefusal::Unknown));
    }

    #[test]
    fn a_head_act_names_one_plain_item() {
        for bad in [
            "content.head:",
            "content.head:a b",
            "content.head:fct-*",
            "content.head:a/b",
        ] {
            assert_eq!(
                RequestedAct::parse(bad),
                Err(ActRefusal::ItemIdMalformed),
                "{bad}"
            );
        }
        let long = format!("content.head:{}", "a".repeat(MAX_ITEM_ID_LEN + 1));
        assert_eq!(RequestedAct::parse(&long), Err(ActRefusal::ItemIdMalformed));
    }

    #[test]
    fn serde_uses_the_string_form() {
        let json = serde_json::to_string(&RequestedAct::EnrollDevice).unwrap();
        assert_eq!(json, "\"device.enroll\"");
        assert!(serde_json::from_str::<RequestedAct>("\"nope\"").is_err());
    }
}
