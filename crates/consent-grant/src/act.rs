//! What the terminal is asking a controller to agree to.
//!
//! Acts travel as short strings so a consent screen can list them and a log can
//! name them. Each parses to one variant; a string this crate does not
//! recognise is refused, never ignored, because an ignored act would be consent
//! the person never saw.
//!
//! Both acts here are recognition: they say whose device this is. Neither says
//! what the device may do to any content.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum RequestedAct {
    /// Bind this device's node key to the person's identity (`device.enroll`).
    EnrollDevice,
    /// Also bind the device's cryptographic root, the key that signs what the
    /// device produces, so the provenance of its bytes resolves to the person
    /// (`device.bind-root`). Only alongside [`Self::EnrollDevice`].
    BindDeviceRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActRefusal {
    Unknown,
}

impl ActRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::Unknown => "act_unknown",
        }
    }
}

const ENROLL: &str = "device.enroll";
const BIND_ROOT: &str = "device.bind-root";

impl RequestedAct {
    pub fn parse(text: &str) -> Result<Self, ActRefusal> {
        match text {
            ENROLL => Ok(Self::EnrollDevice),
            BIND_ROOT => Ok(Self::BindDeviceRoot),
            _ => Err(ActRefusal::Unknown),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnrollDevice => ENROLL,
            Self::BindDeviceRoot => BIND_ROOT,
        }
    }
}

impl std::fmt::Display for RequestedAct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
        act.as_str().to_string()
    }
}

/// Whether `acts` is a coherent set: no repeats, and a device root is bound
/// only for a device that is being enrolled.
pub(crate) fn coherent(acts: &[RequestedAct]) -> bool {
    let distinct = acts
        .iter()
        .enumerate()
        .all(|(i, act)| !acts[..i].contains(act));
    let root_needs_enrollment =
        !acts.contains(&RequestedAct::BindDeviceRoot) || acts.contains(&RequestedAct::EnrollDevice);
    distinct && root_needs_enrollment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acts_round_trip_through_their_strings() {
        for text in ["device.enroll", "device.bind-root"] {
            assert_eq!(RequestedAct::parse(text).unwrap().to_string(), text);
        }
    }

    #[test]
    fn an_unknown_act_is_refused_not_ignored() {
        for text in ["device.admin", "content.head:item", ""] {
            assert_eq!(
                RequestedAct::parse(text),
                Err(ActRefusal::Unknown),
                "{text}"
            );
        }
    }

    #[test]
    fn serde_uses_the_string_form() {
        let json = serde_json::to_string(&RequestedAct::EnrollDevice).unwrap();
        assert_eq!(json, "\"device.enroll\"");
        assert!(serde_json::from_str::<RequestedAct>("\"nope\"").is_err());
    }

    #[test]
    fn a_device_root_is_bound_only_with_enrollment() {
        use RequestedAct::{BindDeviceRoot, EnrollDevice};
        assert!(coherent(&[EnrollDevice]));
        assert!(coherent(&[EnrollDevice, BindDeviceRoot]));
        assert!(!coherent(&[BindDeviceRoot]));
        assert!(!coherent(&[EnrollDevice, EnrollDevice]));
    }
}
