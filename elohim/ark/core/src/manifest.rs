//! The runtime declaration and its pure validation and identity operations.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Current runtime-manifest schema version.
pub const MANIFEST_SCHEMA: u32 = 1;

/// Schema key used when a runtime manifest becomes an EPR Manifest payload.
pub const MANIFEST_KIND: &str = "runtime-manifest";

/// A content-addressed declaration of the processes occupying one berth.
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
    pub processes: Vec<ProcessSpec>,
}

impl Default for RuntimeManifest {
    fn default() -> Self {
        Self {
            schema: MANIFEST_SCHEMA,
            kind: MANIFEST_KIND.to_string(),
            supersedes: None,
            reach: default_reach(),
            processes: Vec::new(),
        }
    }
}

/// Declaration of one child process.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "snake_case")]
pub struct ProcessSpec {
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
}

impl Default for ProcessSpec {
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
        }
    }
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
    pub fn process(&self, name: &str) -> Option<&ProcessSpec> {
        self.processes.iter().find(|process| process.name == name)
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

    const SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
