//! Content-addressed child-death witnesses and their incidents.

use elohim_compute::Refusal;
use serde::{Deserialize, Serialize};

use crate::{exit::ExitClass, GiveUpReason, Intent, Passport, ProcessSample, RestartVerdict};

/// Record kind carried by every S0 death witness.
pub const WITNESS_KIND: &str = "death-witness";

/// A content-addressed account of one observed child death.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DeathWitness {
    /// Death-witness schema version.
    pub schema: u32,
    /// Record kind; every S0 witness carries [`WITNESS_KIND`].
    pub kind: String,
    /// CID string of the incident containing this death.
    pub incident: String,
    /// Name of the child process.
    pub process: String,
    /// Berth incarnation in which the child died.
    pub incarnation: u64,
    /// Operating-system process identifier of the child.
    pub pid: u32,
    /// SHA-256 digest of the artifact actually started.
    pub artifact_sha256: String,
    /// Resolved local path of the artifact actually started.
    pub artifact_path: String,
    /// Child start time.
    pub started_at_epoch_ms: u64,
    /// Child death time.
    pub died_at_epoch_ms: u64,
    /// Observed child uptime.
    pub uptime_ms: u64,
    /// Normalized termination cause.
    pub exit: ExitClass,
    /// Retained standard-error tail, oldest first.
    pub last_stderr: Vec<String>,
    /// Retained standard-output tail, oldest first.
    pub last_stdout: Vec<String>,
    /// Best-effort resource measurements at death.
    pub sample: Option<ProcessSample>,
    /// Ark's last recorded decision about the child.
    pub last_intent: Option<Intent>,
    /// Berth passport as it stood at death.
    pub passport: Passport,
    /// Restart decision, filled after the write-ahead witness is durable.
    pub verdict: Option<RestartVerdict>,
    /// Refusal paired with a give-up verdict, including the honored limit owner.
    pub refusal: Option<Refusal>,
}

/// Encoding failure while deriving a witness's canonical identity.
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
pub enum WitnessError {
    /// Canonical dag-cbor encoding failed.
    #[error("witness encoding: {0}")]
    Encode(String),
}

impl DeathWitness {
    /// Encodes the entire witness as canonical dag-cbor bytes.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, WitnessError> {
        serde_ipld_dagcbor::to_vec(self).map_err(|error| WitnessError::Encode(error.to_string()))
    }

    /// Computes the CID string of the entire canonical witness.
    pub fn cid(&self) -> Result<String, WitnessError> {
        Ok(elohim_epr::cid::compute_cid(&self.canonical_bytes()?).to_string())
    }
}

/// An incident joining one or more deaths until readiness, give-up, or stop.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Incident {
    /// Content-derived incident CID string.
    pub id: String,
    /// Process whose deaths belong to the incident.
    pub process: String,
    /// Wall-clock time at which the incident opened.
    pub opened_at_epoch_ms: u64,
    /// Berth incarnation at incident open.
    pub incarnation_at_open: u64,
    /// Witness CID strings in observation order.
    pub witnesses: Vec<String>,
    /// Terminal incident outcome, when closed.
    pub closed: Option<IncidentClose>,
}

/// The terminal outcome of an incident.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentClose {
    /// The process recovered and completed readiness again.
    ReadyAgain { at_epoch_ms: u64 },
    /// Restart policy permanently gave up.
    GaveUp {
        at_epoch_ms: u64,
        reason: GiveUpReason,
    },
    /// The process was intentionally stopped.
    Stopped { at_epoch_ms: u64 },
}

impl Incident {
    /// Opens an incident whose identity is derived from process, time, and incarnation.
    pub fn open(process: &str, at_epoch_ms: u64, incarnation: u64) -> Self {
        let identity = (process, at_epoch_ms, incarnation);
        let bytes = serde_ipld_dagcbor::to_vec(&identity)
            .expect("an incident identity tuple always has a dag-cbor representation");
        let id = elohim_epr::cid::compute_cid(&bytes).to_string();
        Self {
            id,
            process: process.to_string(),
            opened_at_epoch_ms: at_epoch_ms,
            incarnation_at_open: incarnation,
            witnesses: Vec::new(),
            closed: None,
        }
    }

    /// Returns true until a terminal outcome is recorded.
    pub fn is_open(&self) -> bool {
        self.closed.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EffectiveTier, ExitClass, Intent, IntentAction, Passport, ProcessPassport, ProcessSample,
        RestartVerdict,
    };
    use elohim_compute::{LimitOwner, Refusal};

    fn passport() -> Passport {
        Passport {
            schema: 1,
            kind: "runtime-passport".to_string(),
            manifest: "bafy-manifest".to_string(),
            node: Some("bafy-node".to_string()),
            incarnation: 3,
            ark_version: "0.1.0".to_string(),
            processes: vec![ProcessPassport {
                name: "conductor".to_string(),
                artifact_sha256: "ab".repeat(32),
                artifact_path: "/opt/elohim/conductor".to_string(),
                pid: Some(42),
                started_at_epoch_ms: Some(1_000),
                ready: true,
                effective_tier: EffectiveTier::None,
                deaths_in_window: 1,
            }],
            last_verdict: Some(RestartVerdict::Restart {
                after_s: 2,
                attempt: 1,
            }),
            updated_at_epoch_ms: 2_000,
        }
    }

    fn witness() -> DeathWitness {
        DeathWitness {
            schema: 1,
            kind: WITNESS_KIND.to_string(),
            incident: "bafy-incident".to_string(),
            process: "conductor".to_string(),
            incarnation: 3,
            pid: 42,
            artifact_sha256: "ab".repeat(32),
            artifact_path: "/opt/elohim/conductor".to_string(),
            started_at_epoch_ms: 1_000,
            died_at_epoch_ms: 2_000,
            uptime_ms: 1_000,
            exit: ExitClass::Signaled {
                signal: 9,
                core_dumped: false,
            },
            last_stderr: vec!["fatal".to_string()],
            last_stdout: vec!["ready".to_string()],
            sample: Some(ProcessSample {
                max_rss_bytes: Some(1024),
                rss_bytes: Some(512),
                user_us: Some(10),
                system_us: Some(20),
                fds: Some(3),
                threads: Some(4),
                io_read_bytes: Some(5),
                io_write_bytes: Some(6),
                oom_score_adj: Some(0),
            }),
            last_intent: Some(Intent {
                at_epoch_ms: 900,
                incarnation: 3,
                process: "conductor".to_string(),
                action: IntentAction::Spawn,
                reason: "initial start".to_string(),
            }),
            passport: passport(),
            verdict: Some(RestartVerdict::Stop),
            refusal: Some(Refusal::gate(
                LimitOwner::Commitment,
                "same-cause",
                "commitment restart limit reached",
            )),
        }
    }

    #[test]
    fn witness_cid_changes_when_any_field_changes_and_is_stable_otherwise() {
        let original = witness();
        let original_cid = original.cid().unwrap();
        assert_eq!(original_cid, original.clone().cid().unwrap());

        macro_rules! assert_field_changes_cid {
            ($field:ident, $value:expr) => {{
                let mut changed = original.clone();
                changed.$field = $value;
                assert_ne!(
                    original_cid,
                    changed.cid().unwrap(),
                    "changing {} must change the witness CID",
                    stringify!($field)
                );
            }};
        }

        assert_field_changes_cid!(schema, 2);
        assert_field_changes_cid!(kind, "other-kind".to_string());
        assert_field_changes_cid!(incident, "bafy-other-incident".to_string());
        assert_field_changes_cid!(process, "storage".to_string());
        assert_field_changes_cid!(incarnation, 4);
        assert_field_changes_cid!(pid, 43);
        assert_field_changes_cid!(artifact_sha256, "cd".repeat(32));
        assert_field_changes_cid!(artifact_path, "/opt/elohim/other".to_string());
        assert_field_changes_cid!(started_at_epoch_ms, 1_001);
        assert_field_changes_cid!(died_at_epoch_ms, 2_001);
        assert_field_changes_cid!(uptime_ms, 1_001);
        assert_field_changes_cid!(exit, ExitClass::Exited { code: 1 });
        assert_field_changes_cid!(last_stderr, vec!["different".to_string()]);
        assert_field_changes_cid!(last_stdout, vec!["different".to_string()]);
        assert_field_changes_cid!(sample, None);
        assert_field_changes_cid!(last_intent, None);
        let mut changed_passport = original.passport.clone();
        changed_passport.updated_at_epoch_ms += 1;
        assert_field_changes_cid!(passport, changed_passport);
        assert_field_changes_cid!(verdict, None);
        assert_field_changes_cid!(refusal, None);
    }

    #[test]
    fn witness_json_carries_kind_death_witness_and_tagged_exit() {
        let json = serde_json::to_value(witness()).unwrap();

        assert_eq!(json["kind"], "death-witness");
        assert_eq!(json["exit"]["class"], "signaled");
        assert_eq!(json["exit"]["signal"], 9);
        assert_eq!(json["exit"]["core_dumped"], false);
        assert_eq!(json["refusal"]["limit_owner"], "commitment");
    }

    #[test]
    fn incident_id_is_content_derived() {
        let incident = Incident::open("conductor", 1_000, 3);
        let bytes = serde_ipld_dagcbor::to_vec(&("conductor", 1_000_u64, 3_u64)).unwrap();
        let expected = elohim_epr::cid::compute_cid(&bytes).to_string();

        assert_eq!(incident.id, expected);
        assert_eq!(incident, Incident::open("conductor", 1_000, 3));
        assert_ne!(incident.id, Incident::open("storage", 1_000, 3).id);
        assert_ne!(incident.id, Incident::open("conductor", 1_001, 3).id);
        assert_ne!(incident.id, Incident::open("conductor", 1_000, 4).id);
        assert!(incident.is_open());
    }
}
