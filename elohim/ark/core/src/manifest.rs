//! The runtime declaration and its pure validation and identity operations.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Current runtime-manifest schema version.
pub const MANIFEST_SCHEMA: u32 = 1;

/// Schema key used when a runtime manifest becomes an EPR Manifest payload.
pub const MANIFEST_KIND: &str = "runtime-manifest";

/// A content-addressed declaration of the processes occupying one berth.
///
/// KEPT rather than projected onto [`elohim_epr_rea::model::ProcessSpec`]: that is the VF
/// *knowledge-level* recipe — stages and conversion edges, reusable across runs — while this
/// declares which artifacts occupy one berth on one host. Same word, different level.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "snake_case")]
pub struct RuntimeManifest {
    /// Manifest schema version.
    #[serde(default = "default_manifest_schema")]
    pub schema: u32,
    /// Manifest schema key.
    #[serde(default = "default_manifest_kind")]
    pub kind: String,
    /// CID string of the previous manifest in this lineage.
    #[serde(default)]
    pub supersedes: Option<String>,
    /// EPR reach encoded as its kebab-case string.
    #[serde(default = "default_reach")]
    pub reach: String,
    /// Processes declared in this runtime.
    #[serde(default)]
    pub processes: Vec<ChildSpec>,
    /// Six-field compute contract declaration; declaration is not enforcement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope: Option<RuntimeEnvelope>,
    /// Device archetype selecting the outward renderer's request floor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archetype: Option<String>,
}

impl Default for RuntimeManifest {
    fn default() -> Self {
        Self {
            schema: MANIFEST_SCHEMA,
            kind: MANIFEST_KIND.to_string(),
            supersedes: None,
            reach: default_reach(),
            processes: Vec::new(),
            envelope: None,
            archetype: None,
        }
    }
}

/// Declaration of one child process.
///
/// Named `ChildSpec` rather than `ProcessSpec` because the substrate already owns that word:
/// [`elohim_epr_rea::model::ProcessSpec`] is the VF *knowledge-level* recipe (stages and
/// conversion edges), a different thing at a different level from this berth-local child
/// declaration. The serde key on [`RuntimeManifest`] stays `processes`, so no manifest's CID
/// moves — pinned by `manifest_cid_is_unmoved_by_the_child_spec_rename`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "snake_case")]
pub struct ChildSpec {
    /// Stable process name within the manifest.
    #[serde(default)]
    pub name: String,
    /// Execution model requested for the process.
    #[serde(default)]
    pub kind: ProcessKind,
    /// Immutable or channel-based artifact reference.
    #[serde(default)]
    pub artifact: ArtifactRef,
    /// Process argv, including argv[0].
    #[serde(default)]
    pub argv: Vec<String>,
    /// Environment entries applied after the optional scrub.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Whether the inherited environment is removed before applying `env`.
    #[serde(default = "default_env_scrub")]
    pub env_scrub: bool,
    /// Source connected to the child's standard input.
    #[serde(default)]
    pub stdin: StdinSource,
    /// Ordered readiness ladder.
    #[serde(default)]
    pub readiness: Vec<Probe>,
    /// Restart and shutdown policy.
    #[serde(default)]
    pub policy: ChildPolicy,
    /// Output retention policy.
    #[serde(default)]
    pub listen: Listen,
    /// Provider envelope and shedding hints; declaration is not enforcement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<ProcessQuota>,
}

impl Default for ChildSpec {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: ProcessKind::Native,
            artifact: ArtifactRef::default(),
            argv: Vec::new(),
            env: BTreeMap::new(),
            env_scrub: true,
            stdin: StdinSource::Null,
            readiness: Vec::new(),
            policy: ChildPolicy::default(),
            listen: Listen::default(),
            quota: None,
        }
    }
}

/// The six-field compute contract's declaration (2026-08-29 virtual-peer contract).
/// Declaration is not enforcement; reciprocity events belong to the future enforcing runtime.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "snake_case")]
pub struct RuntimeEnvelope {
    /// Provider's envelope: resources consented to, not enforced here.
    pub bound: ResourceQuota,
    /// Optional scheduler request override; absence uses the device archetype floor.
    /// This is a declaration, not runtime enforcement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests: Option<ResourceQuota>,
    /// Provider's envelope: explicit memory reserved outside child quotas, not inferred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headroom_bytes: Option<u64>,
    /// The measure: declares what memory counts; does not take a measurement.
    pub measure: Measure,
    /// Provider's protected set: named children a future shedder must preserve.
    pub protected: Vec<String>,
    /// Recipient's shed order: named children, disposable first; no shedding occurs here.
    pub shed_order: Vec<String>,
    /// Graded obligations: declared soft/high/hard bands, not enforcement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graded: Option<Bands>,
}

/// The contract's memory measure; declaration is not enforcement.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    /// Committed anon + kernel + shmem; reclaimable page cache is not charged.
    #[default]
    Committed,
}

/// Graded obligations as integer percentages; declaration is not enforcement.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Bands {
    /// Soft obligation: recipient self-throttles at this percentage.
    pub soft_pct: u8,
    /// High obligation: recipient cooperatively sheds at this percentage.
    pub high_pct: u8,
    /// Hard obligation: provider sheds at this percentage.
    pub hard_pct: u8,
}

/// Provider's resource envelope; declaration is not enforcement.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ResourceQuota {
    /// Provider's envelope: memory ceiling in bytes; absent means undeclared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// Provider's envelope: CPU budget in thousandths of a core.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_millis: Option<u32>,
    /// Provider's envelope: process-count ceiling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pids: Option<u32>,
    /// Provider's envelope: disk ceiling in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_bytes: Option<u64>,
}

/// Child's share of the six-field contract; declaration is not enforcement.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProcessQuota {
    /// Provider's envelope: this child's memory ceiling in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_max_bytes: Option<u64>,
    /// Provider's envelope: this child's CPU share in thousandths of a core.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_share_millis: Option<u32>,
    /// Recipient's shedding contract: whether the child is one OOM shed unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oom_group: Option<bool>,
    /// Recipient's shedding contract: requested kernel victim preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oom_score_adj: Option<i16>,
}

/// Process execution model.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    /// A native operating-system process.
    #[default]
    Native,
    /// A process linked into the launcher.
    InProcess,
    /// A WebAssembly process.
    Wasm,
    /// A process delegated to another runtime.
    Delegated,
}

/// Artifact identity and resolution declaration.
///
/// KEPT rather than projected onto [`elohim_epr_rea::model::PinnedRef`]: a `PinnedRef` pins
/// `id@version` (a declared dependency, resolved by a registry), whereas an artifact is pinned
/// by content digest — a strictly stronger and differently-checkable claim, verified against
/// the bytes on disk before spawn.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRef {
    /// A mutable channel, resolved in a later slice.
    Channel {
        /// Channel identifier.
        channel_id: String,
    },
    /// An immutable artifact pinned by its SHA-256 digest.
    Pinned {
        /// Optional artifact CID string.
        #[serde(default)]
        cid: Option<String>,
        /// Mandatory lowercase hexadecimal SHA-256 digest.
        sha256: String,
        /// Optional expected file size.
        #[serde(default)]
        bytes: Option<u64>,
    },
}

impl Default for ArtifactRef {
    fn default() -> Self {
        Self::Pinned {
            cid: None,
            sha256: String::new(),
            bytes: None,
        }
    }
}

/// Source connected to a child's standard input.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StdinSource {
    /// Connect a null input source.
    #[default]
    Null,
    /// Supply the berth's passphrase.
    Passphrase,
}

/// One rung of the readiness ladder.
///
/// KEPT rather than projected onto [`elohim_epr_rea::model::ValidatorRef`]: a validator names
/// a conformance mechanism already enforcing a recipe edge elsewhere, while a probe is a local
/// wait with a patience budget that decides nothing about conformance.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Probe {
    /// Wait for a matching standard-output line.
    StdoutLine {
        /// Substring required in a line.
        contains: String,
        /// Maximum wait for this rung.
        patience_ms: u64,
    },
    /// Wait for a declared berth port to accept TCP connections.
    TcpListen {
        /// Key resolved against `Berth.ports`.
        port_key: String,
        /// Maximum wait for this rung.
        patience_ms: u64,
    },
}

/// Restart, shutdown, intensity, and backoff policy for a child.
///
/// KEPT rather than projected onto [`elohim_epr_rea::model::Commitment`]: a commitment is a
/// promise between two agents, while this is a local supervision rule nobody promised anyone.
/// Its one genuinely economic part — the intensity ceiling — DOES project, via
/// [`ChildPolicy::intensity_bound`] onto [`elohim_epr_rea::model::Bound`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "snake_case")]
pub struct ChildPolicy {
    /// When the child is eligible for restart.
    #[serde(default)]
    pub restart: Restart,
    /// Graceful shutdown behavior.
    #[serde(default)]
    pub shutdown: Shutdown,
    /// Maximum death intensity.
    #[serde(default)]
    pub intensity: Intensity,
    /// Restart delay progression.
    #[serde(default)]
    pub backoff: Backoff,
    /// Consecutive same-cause deaths permitted before give-up.
    #[serde(default = "default_same_cause_limit")]
    pub same_cause_limit: u32,
}

impl Default for ChildPolicy {
    fn default() -> Self {
        Self {
            restart: Restart::Permanent,
            shutdown: Shutdown::default(),
            intensity: Intensity::default(),
            backoff: Backoff::default(),
            same_cause_limit: default_same_cause_limit(),
        }
    }
}

/// Restart eligibility mode.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Restart {
    /// Restart after every termination.
    #[default]
    Permanent,
    /// Restart only after an unclean termination.
    Transient,
    /// Never restart.
    Temporary,
}

/// Graceful shutdown settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "snake_case")]
pub struct Shutdown {
    /// Signal sent for graceful termination.
    #[serde(default = "default_shutdown_signal")]
    pub signal: i32,
    /// Grace period before forced termination.
    #[serde(default = "default_shutdown_grace_ms")]
    pub grace_ms: u64,
}

impl Default for Shutdown {
    fn default() -> Self {
        Self {
            signal: default_shutdown_signal(),
            grace_ms: default_shutdown_grace_ms(),
        }
    }
}

/// Sliding-window child-death limit.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "snake_case")]
pub struct Intensity {
    /// Maximum deaths allowed in the window.
    #[serde(default = "default_max_deaths")]
    pub max_deaths: u32,
    /// Sliding-window width in seconds.
    #[serde(default = "default_intensity_window_s")]
    pub window_s: u64,
}

impl Default for Intensity {
    fn default() -> Self {
        Self {
            max_deaths: default_max_deaths(),
            window_s: default_intensity_window_s(),
        }
    }
}

/// Restart delay progression.
///
/// KEPT with no substrate counterpart checked and rejected: the substrate models a limit
/// ([`elohim_epr_rea::model::Bound`]) and an observation span
/// ([`elohim_epr_rea::stock::Window`]); a delay schedule is neither, and inventing an atom for
/// it would mint vocabulary for a number that never leaves this host.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "snake_case")]
pub struct Backoff {
    /// Minimum restart delay in seconds.
    #[serde(default = "default_backoff_min_s")]
    pub min_s: u64,
    /// Maximum restart delay in seconds.
    #[serde(default = "default_backoff_max_s")]
    pub max_s: u64,
    /// Number of delay steps between the bounds.
    #[serde(default = "default_backoff_steps")]
    pub steps: u32,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            min_s: default_backoff_min_s(),
            max_s: default_backoff_max_s(),
            steps: default_backoff_steps(),
        }
    }
}

/// In-memory output retention settings.
///
/// KEPT with no substrate counterpart: [`elohim_epr_rea::stock::Stock`] is the nearest shape
/// and models a measured level with inflow and outflow, whereas this only says how many lines
/// of text to keep in RAM.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "snake_case")]
pub struct Listen {
    /// Number of lines retained in the output ring.
    #[serde(default = "default_ring_lines")]
    pub ring_lines: usize,
    /// Number of trailing lines copied into a witness.
    #[serde(default = "default_tail_lines")]
    pub tail_lines: usize,
}

impl Default for Listen {
    fn default() -> Self {
        Self {
            ring_lines: default_ring_lines(),
            tail_lines: default_tail_lines(),
        }
    }
}

/// Runtime-manifest decoding, validation, or canonical encoding failure.
#[derive(thiserror::Error, Clone, Debug, PartialEq)]
pub enum ManifestError {
    /// Input was not valid manifest JSON.
    #[error("manifest JSON: {0}")]
    Json(String),
    /// The schema version is unsupported.
    #[error("manifest schema: {0}")]
    Schema(String),
    /// The manifest kind is unsupported.
    #[error("manifest kind: {0}")]
    Kind(String),
    /// A manifest invariant is violated.
    #[error("invalid manifest: {0}")]
    Invalid(String),
    /// Canonical dag-cbor encoding failed.
    #[error("manifest encoding: {0}")]
    Encode(String),
}

impl RuntimeManifest {
    /// Decodes and validates a hand-authored JSON runtime manifest.
    pub fn from_json(s: &str) -> Result<Self, ManifestError> {
        let manifest: Self =
            serde_json::from_str(s).map_err(|error| ManifestError::Json(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Encodes this record as dag-cbor bytes.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ManifestError> {
        serde_ipld_dagcbor::to_vec(self).map_err(|error| ManifestError::Encode(error.to_string()))
    }

    /// Computes this record's canonical CID string.
    pub fn cid(&self) -> Result<String, ManifestError> {
        Ok(elohim_epr::cid::compute_cid(&self.canonical_bytes()?).to_string())
    }

    /// Finds a declared process by name.
    pub fn process(&self, name: &str) -> Option<&ChildSpec> {
        self.processes.iter().find(|process| process.name == name)
    }

    /// Declared child memory parts shared by validation and the REA projection.
    pub(crate) fn memory_quota_parts(&self) -> impl Iterator<Item = u64> + '_ {
        self.processes.iter().filter_map(|child| {
            child
                .quota
                .as_ref()
                .and_then(|quota| quota.memory_max_bytes)
        })
    }

    fn validate(&self) -> Result<(), ManifestError> {
        if self.schema != MANIFEST_SCHEMA {
            return Err(ManifestError::Schema(format!(
                "expected {MANIFEST_SCHEMA}, got {}",
                self.schema
            )));
        }
        if self.kind != MANIFEST_KIND {
            return Err(ManifestError::Kind(format!(
                "expected {MANIFEST_KIND}, got {}",
                self.kind
            )));
        }
        if self.processes.is_empty() {
            return Err(ManifestError::Invalid(
                "at least one process is required".to_string(),
            ));
        }

        let mut names = BTreeSet::new();
        for process in &self.processes {
            if process.name.is_empty() {
                return Err(ManifestError::Invalid(
                    "process name must not be empty".to_string(),
                ));
            }
            if !names.insert(process.name.as_str()) {
                return Err(ManifestError::Invalid(format!(
                    "duplicate process name: {}",
                    process.name
                )));
            }
            if process.argv.is_empty() {
                return Err(ManifestError::Invalid(format!(
                    "process {} has an empty argv",
                    process.name
                )));
            }
            if let ArtifactRef::Pinned { sha256, .. } = &process.artifact {
                let valid = sha256.len() == 64
                    && sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
                if !valid {
                    return Err(ManifestError::Invalid(format!(
                        "process {} pinned artifact sha256 must be 64 lowercase hexadecimal characters",
                        process.name
                    )));
                }
            }
        }

        if let Some(envelope) = &self.envelope {
            for (list, entries) in [
                ("protected", &envelope.protected),
                ("shed_order", &envelope.shed_order),
            ] {
                for name in entries {
                    if !names.contains(name.as_str()) {
                        return Err(ManifestError::Invalid(format!(
                            "{list} names unknown process: {name}"
                        )));
                    }
                }
            }
            for name in &envelope.protected {
                if envelope.shed_order.contains(name) {
                    return Err(ManifestError::Invalid(format!(
                        "process {name} appears in both protected and shed_order"
                    )));
                }
            }
            if let Some(root) = envelope.bound.memory_bytes {
                // u128 keeps the refusal exact even when u64 child quotas overflow u64.
                let children: u128 = self.memory_quota_parts().map(u128::from).sum();
                let headroom = envelope.headroom_bytes.unwrap_or(0);
                let total = children + u128::from(headroom);
                if total > u128::from(root) {
                    return Err(ManifestError::Invalid(format!(
                        "child memory total {children} + headroom {headroom} = {total} exceeds root memory {root}"
                    )));
                }
            }
        }

        Ok(())
    }
}

fn default_manifest_schema() -> u32 {
    MANIFEST_SCHEMA
}

fn default_manifest_kind() -> String {
    MANIFEST_KIND.to_string()
}

fn default_reach() -> String {
    "trusted".to_string()
}

fn default_env_scrub() -> bool {
    true
}

fn default_shutdown_signal() -> i32 {
    2
}

fn default_shutdown_grace_ms() -> u64 {
    20_000
}

fn default_max_deaths() -> u32 {
    5
}

fn default_intensity_window_s() -> u64 {
    300
}

fn default_backoff_min_s() -> u64 {
    1
}

fn default_backoff_max_s() -> u64 {
    60
}

fn default_backoff_steps() -> u32 {
    6
}

fn default_same_cause_limit() -> u32 {
    3
}

fn default_ring_lines() -> usize {
    200
}

fn default_tail_lines() -> usize {
    40
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_manifest_cid_is_unmoved_by_absent_envelope() {
        // Embedded verbatim from the main tree's matthew/ark/manifest.json:
        // local-dev berths are untracked and absent in this worktree. No runtime I/O.
        let manifest = RuntimeManifest::from_json(
            r#"{
  "schema": 1,
  "kind": "runtime-manifest",
  "reach": "trusted",
  "processes": [
    {
      "name": "conductor",
      "kind": "native",
      "artifact": {
        "pinned": {
          "sha256": "ffa40a0c6fab5ce062c4af76328dfe2de143256ddf791a504d72bca698a9ba20"
        }
      },
      "argv": [
        "{artifact}",
        "--piped",
        "--structured=Log",
        "--config-path",
        "{data_root}/conductor-config.yaml"
      ],
      "stdin": "passphrase",
      "readiness": [
        {
          "stdout_line": {
            "contains": "Conductor ready.",
            "patience_ms": 120000
          }
        },
        {
          "tcp_listen": {
            "port_key": "admin_ws",
            "patience_ms": 30000
          }
        }
      ],
      "policy": {
        "shutdown": {
          "signal": 2,
          "grace_ms": 20000
        }
      }
    }
  ]
}
"#,
        )
        .unwrap();
        assert!(manifest.envelope.is_none());
        assert!(manifest.processes.iter().all(|child| child.quota.is_none()));
        assert_eq!(
            manifest.cid().unwrap(),
            "bafyreihagg75knog3e2fkiygpghcgqge35ovzka3vxken2zjleqnerdcaa"
        );
    }

    fn quota_manifest(root: Option<u64>, child: u64, headroom: u64) -> RuntimeManifest {
        let mut manifest = RuntimeManifest::from_json(&minimal_manifest_json()).unwrap();
        manifest.envelope = Some(RuntimeEnvelope {
            bound: ResourceQuota {
                memory_bytes: root,
                ..Default::default()
            },
            headroom_bytes: Some(headroom),
            ..Default::default()
        });
        manifest.processes[0].quota = Some(ProcessQuota {
            memory_max_bytes: Some(child),
            ..Default::default()
        });
        manifest
    }

    #[test]
    fn memory_sum_plus_headroom_over_root_names_totals() {
        let manifest = quota_manifest(Some(100), 90, 11);
        assert_eq!(
            manifest.validate(),
            Err(ManifestError::Invalid(
                "child memory total 90 + headroom 11 = 101 exceeds root memory 100".into()
            ))
        );
    }

    #[test]
    fn memory_sum_at_root_minus_headroom_passes() {
        let mut manifest = quota_manifest(Some(100), 60, 10);
        let mut second = manifest.processes[0].clone();
        second.name = "storage".into();
        second.quota.as_mut().unwrap().memory_max_bytes = Some(30);
        manifest.processes.push(second);
        assert_eq!(manifest.validate(), Ok(()));
    }

    #[test]
    fn undeclared_memory_never_refuses_the_sum() {
        assert_eq!(quota_manifest(None, u64::MAX, u64::MAX).validate(), Ok(()));
    }

    #[test]
    fn memory_sum_cannot_wrap_past_root() {
        let mut manifest = quota_manifest(Some(u64::MAX), u64::MAX, 1);
        assert!(matches!(
            manifest.validate(),
            Err(ManifestError::Invalid(_))
        ));
        let mut child = manifest.processes[0].clone();
        child.name = "storage".into();
        manifest.processes.push(child);
        let error = manifest.validate().unwrap_err().to_string();
        assert!(error.contains("36893488147419103230"));
        assert!(error.contains("36893488147419103231"));
    }

    #[test]
    fn unknown_shed_order_process_is_refused() {
        let mut manifest = quota_manifest(None, 10, 0);
        manifest
            .envelope
            .as_mut()
            .unwrap()
            .shed_order
            .push("missing".into());
        assert_eq!(
            manifest.validate(),
            Err(ManifestError::Invalid(
                "shed_order names unknown process: missing".into()
            ))
        );
    }

    #[test]
    fn unknown_protected_process_and_conflicting_lists_are_refused() {
        let mut manifest = quota_manifest(None, 10, 0);
        let envelope = manifest.envelope.as_mut().unwrap();
        envelope.protected.push("missing".into());
        assert_eq!(
            manifest.validate(),
            Err(ManifestError::Invalid(
                "protected names unknown process: missing".into()
            ))
        );
        let envelope = manifest.envelope.as_mut().unwrap();
        envelope.protected = vec!["conductor".into()];
        envelope.shed_order = vec!["conductor".into()];
        assert_eq!(
            manifest.validate(),
            Err(ManifestError::Invalid(
                "process conductor appears in both protected and shed_order".into()
            ))
        );
    }

    #[test]
    fn envelope_json_dag_cbor_round_trip_keeps_cid() {
        let mut manifest = quota_manifest(Some(100), 90, 10);
        let envelope = manifest.envelope.as_mut().unwrap();
        envelope.bound.cpu_millis = Some(500);
        envelope.bound.pids = Some(32);
        envelope.bound.disk_bytes = Some(4096);
        envelope.graded = Some(Bands {
            soft_pct: 70,
            high_pct: 85,
            hard_pct: 100,
        });
        envelope.protected.push("conductor".into());
        let quota = manifest.processes[0].quota.as_mut().unwrap();
        quota.cpu_share_millis = Some(250);
        quota.oom_group = Some(false);
        quota.oom_score_adj = Some(-100);
        let json = serde_json::to_string(&manifest).unwrap();
        let decoded = RuntimeManifest::from_json(&json).unwrap();
        let bytes = decoded.canonical_bytes().unwrap();
        let from_cbor: RuntimeManifest = serde_ipld_dagcbor::from_slice(&bytes).unwrap();
        assert_eq!(from_cbor, manifest);
        assert_eq!(from_cbor.canonical_bytes().unwrap(), bytes);
        assert_eq!(from_cbor.cid().unwrap(), manifest.cid().unwrap());
        assert_eq!(
            manifest.cid().unwrap(),
            "bafyreifvczj4a37ijg5m3xtklkpdhb4xijmygkqvsva4y7w73o4rxidsaa"
        );
    }

    const SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    /// The CID of `minimal_manifest_json` as it stood at commit 0200ff77c, when the child
    /// declaration was still called `ProcessSpec`. A Rust type name is not on the wire — the
    /// serde key stays `processes` — so the rename must leave this literal untouched, or every
    /// berth's declared manifest is silently re-addressed.
    const S0_MANIFEST_CID: &str = "bafyreighn7z4ph4u2j7jhoat7lvxoxyj2yeudoax4lvhcy7ewwqq4aeqtu";

    fn minimal_manifest_json() -> String {
        format!(
            r#"{{
                "processes": [{{
                    "name": "conductor",
                    "artifact": {{"pinned": {{"sha256": "{SHA256}"}}}},
                    "argv": ["{{artifact}}"]
                }}]
            }}"#
        )
    }

    #[test]
    fn manifest_json_round_trips_and_defaults_apply() {
        let manifest = RuntimeManifest::from_json(&minimal_manifest_json()).unwrap();
        let process = manifest.process("conductor").unwrap();

        assert_eq!(manifest.schema, MANIFEST_SCHEMA);
        assert_eq!(manifest.kind, MANIFEST_KIND);
        assert_eq!(manifest.reach, "trusted");
        assert_eq!(process.kind, ProcessKind::Native);
        assert_eq!(process.policy, ChildPolicy::default());
        assert!(process.env_scrub);
        assert_eq!(process.stdin, StdinSource::Null);
        assert_eq!(process.listen.ring_lines, 200);

        let json = serde_json::to_string(&manifest).unwrap();
        assert_eq!(RuntimeManifest::from_json(&json).unwrap(), manifest);
    }

    #[test]
    fn manifest_cid_is_stable_and_order_insensitive_to_json_whitespace() {
        let compact = format!(
            r#"{{"processes":[{{"name":"conductor","artifact":{{"pinned":{{"sha256":"{SHA256}"}}}},"argv":["{{artifact}}"]}}]}}"#
        );
        let spaced = minimal_manifest_json();

        let compact_cid = RuntimeManifest::from_json(&compact).unwrap().cid().unwrap();
        let spaced_cid = RuntimeManifest::from_json(&spaced).unwrap().cid().unwrap();

        assert_eq!(compact_cid, spaced_cid);
        assert!(compact_cid.starts_with("bafy"));
    }

    #[test]
    fn manifest_cid_is_unmoved_by_the_child_spec_rename() {
        let manifest = RuntimeManifest::from_json(&minimal_manifest_json()).unwrap();

        assert_eq!(manifest.cid().unwrap(), S0_MANIFEST_CID);
        assert_eq!(
            serde_json::to_value(&manifest).unwrap()["processes"][0]["name"],
            "conductor",
            "the wire key is `processes`; only the Rust type was renamed"
        );
    }

    #[test]
    fn manifest_refuses_wrong_kind_duplicate_names_and_bad_sha() {
        let wrong_kind = minimal_manifest_json().replacen(
            "{\n                \"processes\"",
            "{\n                \"kind\": \"other\",\n                \"processes\"",
            1,
        );
        assert!(matches!(
            RuntimeManifest::from_json(&wrong_kind),
            Err(ManifestError::Kind(_))
        ));

        let duplicate = format!(
            r#"{{"processes":[
                {{"name":"same","artifact":{{"pinned":{{"sha256":"{SHA256}"}}}},"argv":["one"]}},
                {{"name":"same","artifact":{{"pinned":{{"sha256":"{SHA256}"}}}},"argv":["two"]}}
            ]}}"#
        );
        assert!(matches!(
            RuntimeManifest::from_json(&duplicate),
            Err(ManifestError::Invalid(_))
        ));

        let bad_sha = minimal_manifest_json().replace(SHA256, "not-a-sha256");
        assert!(matches!(
            RuntimeManifest::from_json(&bad_sha),
            Err(ManifestError::Invalid(_))
        ));

        // Uppercase hex would never equal the driver's lowercase digest (exit 66) and
        // would make one artifact CID-distinct from itself — refused at the manifest.
        let upper_sha = minimal_manifest_json().replace(SHA256, &SHA256.to_ascii_uppercase());
        assert!(matches!(
            RuntimeManifest::from_json(&upper_sha),
            Err(ManifestError::Invalid(_))
        ));

        // `#[serde(default)]` lets a mistyped key yield a nameless process; refused.
        let nameless = minimal_manifest_json().replace("\"name\"", "\"nme\"");
        assert!(matches!(
            RuntimeManifest::from_json(&nameless),
            Err(ManifestError::Invalid(_))
        ));
    }
}
