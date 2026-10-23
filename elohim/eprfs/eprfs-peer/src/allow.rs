//! The crossing allow-list: which of a device's flow events may enter its channel tree, and so
//! reach the person's other devices (spec §3.3).
//!
//! Default deny. An event crosses only when it names exactly one measure on the list and exactly
//! one kind (`reason:<kind>`) listed for that measure, and was authored on this device: an event
//! folded from another device carries a [`VIA_SLOT`] and never re-crosses. Recall sessions,
//! private memory, session transcripts and anything else not listed never cross. Nothing here
//! reads the process environment: the `env:<k>=<v>` slots an event carries are the berth
//! record's own fields, written by `epr flow note --env`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use elohim_epr_rea::FlowEvent;
use serde_json::Value;

use crate::error::PeerError;

/// The slot a folded event carries naming the device it was folded from: `via:<did:key>`.
pub const VIA_SLOT: &str = "via:";

const MEASURE_SLOT: &str = "measure:";
const KIND_SLOT: &str = "reason:";

/// `classifiedAs` is tag first, subject second; the prefixed slots follow (epr-rea
/// `FlowEvent::classified_as`). A subject is never read as a slot.
const FIRST_SLOT: usize = 2;

/// The measures and kinds that cross: `berth.channel.crosses` in `pool-policy.json`, shaped
/// `{"<measure>": ["<kind>", …]}`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossingAllowList {
    crosses: BTreeMap<String, BTreeSet<String>>,
}

impl CrossingAllowList {
    /// The list's place in `pool-policy.json`.
    pub const POLICY_KEYS: [&'static str; 3] = ["berth", "channel", "crosses"];

    /// A list from `(measure, kinds)` pairs.
    pub fn new<M, K>(crosses: impl IntoIterator<Item = (M, K)>) -> CrossingAllowList
    where
        M: Into<String>,
        K: IntoIterator,
        K::Item: Into<String>,
    {
        CrossingAllowList {
            crosses: crosses
                .into_iter()
                .map(|(measure, kinds)| {
                    (measure.into(), kinds.into_iter().map(Into::into).collect())
                })
                .collect(),
        }
    }

    /// The list a parsed `pool-policy.json` declares. A policy with no `berth.channel.crosses`
    /// declares an empty list, which lets nothing cross.
    ///
    /// # Errors
    ///
    /// [`PeerError::Policy`] when `crosses` is present but is not an object of string arrays.
    pub fn from_policy(policy: &Value) -> Result<CrossingAllowList, PeerError> {
        Self::read(policy, Path::new("pool-policy.json"))
    }

    /// The list `path` (a `pool-policy.json`) declares.
    ///
    /// # Errors
    ///
    /// [`PeerError::Policy`] when the file cannot be read, is not JSON, or its list is malformed.
    pub fn from_policy_file(path: &Path) -> Result<CrossingAllowList, PeerError> {
        let invalid = |reason: String| PeerError::Policy {
            path: path.to_path_buf(),
            reason,
        };
        let text = std::fs::read_to_string(path).map_err(|e| invalid(e.to_string()))?;
        let policy: Value = serde_json::from_str(&text).map_err(|e| invalid(e.to_string()))?;
        Self::read(&policy, path)
    }

    fn read(policy: &Value, path: &Path) -> Result<CrossingAllowList, PeerError> {
        let invalid = |reason: &str| PeerError::Policy {
            path: PathBuf::from(path),
            reason: format!("{}: {reason}", Self::POLICY_KEYS.join(".")),
        };
        let Some(crosses) = Self::POLICY_KEYS
            .iter()
            .try_fold(policy, |value, key| value.get(key))
        else {
            return Ok(CrossingAllowList::default());
        };
        let crosses = crosses
            .as_object()
            .ok_or_else(|| invalid("not an object of measure -> [kind]"))?;
        let mut list = BTreeMap::new();
        for (measure, kinds) in crosses {
            let kinds = kinds
                .as_array()
                .ok_or_else(|| invalid("a measure's kinds are not an array"))?
                .iter()
                .map(|kind| kind.as_str().map(str::to_string))
                .collect::<Option<BTreeSet<String>>>()
                .ok_or_else(|| invalid("a kind is not a string"))?;
            list.insert(measure.clone(), kinds);
        }
        Ok(CrossingAllowList { crosses: list })
    }

    /// Whether nothing crosses.
    pub fn is_empty(&self) -> bool {
        self.crosses.values().all(BTreeSet::is_empty)
    }

    /// Whether `event` may enter this device's channel tree (or be folded from another's).
    pub fn admits(&self, event: &FlowEvent) -> bool {
        if via_device(event).is_some() {
            return false;
        }
        let (Some(measure), Some(kind)) = (
            single_slot(event, MEASURE_SLOT),
            single_slot(event, KIND_SLOT),
        ) else {
            return false;
        };
        self.crosses
            .get(measure)
            .is_some_and(|kinds| kinds.contains(kind))
    }
}

/// The device an event was folded from, when it carries a [`VIA_SLOT`].
pub fn via_device(event: &FlowEvent) -> Option<&str> {
    slots(event).find_map(|slot| slot.strip_prefix(VIA_SLOT))
}

fn slots(event: &FlowEvent) -> impl Iterator<Item = &str> {
    event
        .classified_as
        .iter()
        .skip(FIRST_SLOT)
        .map(String::as_str)
}

/// The value of the one slot with `prefix`; `None` when there is none or more than one.
fn single_slot<'e>(event: &'e FlowEvent, prefix: &str) -> Option<&'e str> {
    let mut values = slots(event).filter_map(|slot| slot.strip_prefix(prefix));
    match (values.next(), values.next()) {
        (Some(value), None) => Some(value),
        _ => None,
    }
}
