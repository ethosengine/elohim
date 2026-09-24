//! Cursor announcement wire format + topic mapping for the observation plane.
//!
//! Cursor announcements are gossiped via libp2p gossipsub when an observer's
//! iroh-blob log advances. Receivers pull the new log segment via the iroh
//! observation-log ALPN (Task 4.5) — only metadata (~200 bytes) crosses
//! gossipsub, not payloads. See spec §5.2.

use crate::observation::wire::Observation;
use crate::p2p::topics::OBSERVATION_GOSSIP_TOPIC_PREFIX;
use crate::services::observation_kinds::ObservationKindDeclaration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CursorAnnouncement {
    pub observer_cid: String,
    pub kind: String,
    pub log_cid: String,
    pub latest_offset: u64,
    pub subject_cid: Option<String>,
    pub observed_at_window: i64,
}

/// Map an observation_kind to its gossipsub topic.
/// `"<namespace>:<name>"` → `"elohim/observations/<namespace>"`.
/// Malformed kinds (no colon) fall back to using the whole string as the namespace.
pub fn observation_topic(kind: &str) -> String {
    let namespace = kind.split(':').next().unwrap_or("unknown");
    format!("{}{}", OBSERVATION_GOSSIP_TOPIC_PREFIX, namespace)
}

/// Width of [`CursorAnnouncement::observed_at_window`], in seconds. An
/// announcement carries the hour an observation fell in, never its instant.
pub const OBSERVED_AT_WINDOW_SECONDS: i64 = 3600;

/// The gossip gate: the cursor announcement an appended observation may make,
/// or `None` when its kind's manifest-declared reach is `agent-private`.
///
/// Every self-authored append calls this on the write path (after
/// `ObservationManagerBackend::append_local` stamps the log head), so the gate
/// sits on the one chokepoint even though nothing publishes announcements yet
/// ([`publish_announcement`] has no caller). Both outcomes are counted —
/// `elohim_observation_cursor_suppressed_total{kind}` and
/// `elohim_observation_cursor_announced_total{kind}` — so the invariant
/// "an agent-private kind is never announced" is readable from `/metrics`.
///
/// `latest_offset` is the log head (the next offset to be written), the same
/// value `observation_logs.latest_offset` records.
pub fn announcement_for(
    decl: &ObservationKindDeclaration,
    obs: &Observation,
) -> Option<CursorAnnouncement> {
    if decl.is_agent_private() {
        crate::metrics::inc_observation_cursor_suppressed(&decl.kind);
        return None;
    }
    crate::metrics::inc_observation_cursor_announced(&decl.kind);
    Some(CursorAnnouncement {
        observer_cid: obs.observer_cid.clone(),
        kind: obs.observation_kind.clone(),
        log_cid: obs.log_cid.clone(),
        latest_offset: obs.log_offset + 1,
        subject_cid: obs.subject_cid.clone(),
        observed_at_window: obs.observed_at.div_euclid(OBSERVED_AT_WINDOW_SECONDS)
            * OBSERVED_AT_WINDOW_SECONDS,
    })
}

/// Publish a [`CursorAnnouncement`] to its kind-derived gossipsub topic.
///
/// The announcement is MessagePack-encoded (`rmp_serde::to_vec`); the resulting
/// bytes are published to `"elohim/observations/<namespace>"`.  Only metadata
/// (~200 bytes) crosses gossipsub — payloads are pulled via the iroh
/// observation-log ALPN (Task 4.5).  Best-effort: callers should log errors
/// rather than treating them as fatal (no peers subscribed is the common case
/// during startup).
pub fn publish_announcement(
    gossipsub: &mut libp2p::gossipsub::Behaviour,
    announcement: &CursorAnnouncement,
) -> Result<libp2p::gossipsub::MessageId, Box<dyn std::error::Error>> {
    let topic = libp2p::gossipsub::IdentTopic::new(observation_topic(&announcement.kind));
    let bytes = rmp_serde::to_vec(announcement)?;
    Ok(gossipsub.publish(topic, bytes)?)
}

// The test path is `…::observation::gossip_gate::…` on purpose: the
// `attention-witnessed-privately` habit's check is
// `cargo test --lib -- observation::gossip_gate`, and a filter that matched
// nothing would pass vacuously.
#[cfg(test)]
mod observation {
    mod gossip_gate {
        use super::super::announcement_for;
        use crate::metrics::{OBSERVATION_CURSOR_ANNOUNCED, OBSERVATION_CURSOR_SUPPRESSED};
        use crate::observation::wire::Observation;
        use crate::services::observation_kinds::ObservationKindDeclaration;
        use std::collections::BTreeMap;

        fn decl(kind: &str, reach: &str) -> ObservationKindDeclaration {
            ObservationKindDeclaration {
                kind: kind.to_string(),
                namespace: "elohim/observations/test".to_string(),
                schema: BTreeMap::new(),
                retention_class: "contextual".to_string(),
                reach: reach.to_string(),
                diversity_threshold: None,
                graduates_to: None,
                graduation_window_seconds: None,
                graduation_policy: None,
            }
        }

        fn obs(kind: &str) -> Observation {
            Observation {
                observer_cid: "human-jessica".into(),
                log_cid: "blake3:abc".into(),
                log_offset: 4,
                observed_at: 1_790_003_725,
                seq: 5,
                observation_kind: kind.into(),
                subject_cid: Some("bafy-node".into()),
                subject_kind: Some("content".into()),
                payload_json: "{}".into(),
                observer_household_cid: None,
                observer_collective_cid: None,
                observer_region: None,
                observer_archetype: None,
                observer_compute_class: None,
                signature: vec![],
            }
        }

        #[test]
        fn agent_private_kind_yields_no_announcement() {
            // A kind label no other test touches, so the deltas are exact.
            let kind = "test:gossip-gate-private";
            let suppressed = || {
                OBSERVATION_CURSOR_SUPPRESSED
                    .with_label_values(&[kind])
                    .get()
            };
            let announced = || {
                OBSERVATION_CURSOR_ANNOUNCED
                    .with_label_values(&[kind])
                    .get()
            };
            let (s0, a0) = (suppressed(), announced());

            assert_eq!(
                announcement_for(&decl(kind, "agent-private"), &obs(kind)),
                None
            );

            assert_eq!(suppressed(), s0 + 1, "the suppression is counted");
            assert_eq!(announced(), a0, "nothing is announced");
        }

        #[test]
        fn non_private_kind_yields_announcement() {
            let kind = "test:gossip-gate-commons";
            let announced = || {
                OBSERVATION_CURSOR_ANNOUNCED
                    .with_label_values(&[kind])
                    .get()
            };
            let suppressed = || {
                OBSERVATION_CURSOR_SUPPRESSED
                    .with_label_values(&[kind])
                    .get()
            };
            let (s0, a0) = (suppressed(), announced());

            let ann = announcement_for(&decl(kind, "commons"), &obs(kind))
                .expect("a non-private kind is announced");
            assert_eq!(ann.observer_cid, "human-jessica");
            assert_eq!(ann.kind, kind);
            assert_eq!(ann.log_cid, "blake3:abc");
            // The head: the next offset to be written, as `observation_logs` records it.
            assert_eq!(ann.latest_offset, 5);
            assert_eq!(ann.subject_cid.as_deref(), Some("bafy-node"));
            // A coarse window, never the exact instant.
            assert_eq!(ann.observed_at_window, 1_790_002_800);

            assert_eq!(announced(), a0 + 1);
            assert_eq!(suppressed(), s0);
        }
    }
}
