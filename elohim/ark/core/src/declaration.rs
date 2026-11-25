//! Pure device declarations, acts and projections. No filesystem or runtime authority.
use crate::manifest::{
    ArtifactRef, BuildRole, BuildRoleDeclaration, ChildPolicy, ChildSpec, ManifestError, Probe,
    Restart, RuntimeManifest,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The watched projection key; its value is always rendered from the manifest.
pub const RELEASE_CHANNELS_KEY: &str = "ELOHIM_RELEASE_CHANNELS";
/// How a device participates in a release channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FollowMode {
    /// Verify without adopting.
    #[default]
    Observe,
    /// Adopt a verified candidate through the health ladder.
    Canary,
    /// Adopt an earned release for new sessions.
    Apply,
}
impl FollowMode {
    /// Stable wire spelling.
    pub fn label(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Canary => "canary",
            Self::Apply => "apply",
        }
    }
    /// Unknown modes are refused, never downgraded.
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "observe" => Ok(Self::Observe),
            "canary" => Ok(Self::Canary),
            "apply" => Ok(Self::Apply),
            _ => Err(format!(
                "unknown mode '{raw}'; expected observe, canary or apply"
            )),
        }
    }
    /// Whether verified releases may be offered to an adoption vehicle.
    pub fn applies(self) -> bool {
        self != Self::Observe
    }
}
/// One private device choice, not a network head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Follow {
    /// Runtime channel or dev-scope bin channel.
    pub channel: String,
    /// Explicit participation mode.
    pub mode: FollowMode,
    /// Optional content-addressed release constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
}
/// Tool source provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// Built from the owning checkout.
    Repository,
    /// Built from an attested submodule pin.
    Gitlink,
}
/// Required upstream check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    /// Check provider.
    pub provider: String,
    /// Repository holding the check.
    pub repo: String,
    /// Exact check name.
    pub check: String,
}
/// Source metadata shared with the existing bin registry wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSource {
    /// Checkout or submodule source.
    pub kind: SourceKind,
    /// Cargo manifest for a repository tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    /// Submodule path for a gitlink tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Refs accepted as publication ancestry.
    #[serde(default)]
    pub published_ref: Vec<String>,
    /// Upstream repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// Prefix of upstream binary release tags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_prefix: Option<String>,
    /// Required check before landing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation: Option<Attestation>,
}
/// Executables and launch policy for a declared tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpec {
    /// Files carried by a release, including its launcher where applicable.
    pub bins: Vec<String>,
    /// Command aliases to release files.
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    /// Explicit candidate environment variable names; never values or secrets.
    #[serde(default)]
    pub candidate_env: BTreeMap<String, Vec<String>>,
    /// Argument prefixes identifying daemon launches.
    #[serde(default)]
    pub long_running: Vec<Vec<String>>,
    /// Build and attestation provenance.
    pub source: ToolSource,
}
/// One refused entry in an otherwise readable legacy projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowRefusal {
    /// Exact entry that failed.
    pub entry: String,
    /// Explanation suitable for the storage adapter's existing refusal shape.
    pub detail: String,
}
/// Parsed projection; callers writing authority must refuse any invalid entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedFollows {
    /// Valid, deduplicated declarations.
    pub follows: Vec<Follow>,
    /// Invalid or conflicting declarations.
    pub refused: Vec<FollowRefusal>,
}
/// Parse the historical comma/newline/semicolon grammar in one place.
pub fn parse_followed_channels(raw: &str) -> ParsedFollows {
    let mut out = ParsedFollows::default();
    for entry in raw
        .split([',', '\n', ';'])
        .map(str::trim)
        .filter(|e| !e.is_empty())
    {
        let (channel, mode) = entry
            .split_once('=')
            .map_or((entry, "observe"), |(c, m)| (c.trim(), m.trim()));
        let parsed = if channel.is_empty() {
            Err("empty channel id".into())
        } else {
            FollowMode::parse(mode)
        };
        match parsed {
            Err(detail) => out.refused.push(FollowRefusal {
                entry: entry.into(),
                detail,
            }),
            Ok(mode) => {
                if let Some(existing) = out.follows.iter().find(|f| f.channel == channel) {
                    if existing.mode != mode {
                        out.refused.push(FollowRefusal {
                            entry: entry.into(),
                            detail: format!("channel '{channel}' has conflicting modes"),
                        });
                    }
                } else {
                    out.follows.push(Follow {
                        channel: channel.into(),
                        mode,
                        pin: None,
                    });
                }
            }
        }
    }
    out
}
/// Path-safe identifier shared by tool names and the channel grammar.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}
/// Validate an authored channel; legacy readers may still report older spellings.
pub fn validate_channel(channel: &str) -> Result<(), String> {
    let parts: Vec<_> = channel.split(':').collect();
    if channel.len() <= 256
        && matches!(parts.as_slice(), ["bin", _] | ["runtime", _, _, _])
        && parts.iter().all(|p| valid_name(p))
    {
        Ok(())
    } else {
        Err(format!(
            "invalid channel '{channel}'; expected bin:<tool> or runtime:<class>:<net>:<name>"
        ))
    }
}
/// Validate the device additions without changing historical manifest encodings.
pub fn validate(manifest: &RuntimeManifest) -> Result<(), ManifestError> {
    let invalid = ManifestError::Invalid;
    let mut channels = BTreeSet::new();
    for follow in &manifest.follows {
        validate_channel(&follow.channel).map_err(invalid)?;
        if !channels.insert(&follow.channel) {
            return Err(invalid(format!("duplicate channel {}", follow.channel)));
        }
        if let Some(pin) = &follow.pin {
            let cid = pin
                .parse::<cid::Cid>()
                .map_err(|e| invalid(e.to_string()))?;
            if cid.to_string() != *pin {
                return Err(invalid("pin must be a canonical CID".into()));
            }
        }
    }
    for (name, tool) in &manifest.tools {
        let bins: BTreeSet<_> = tool.bins.iter().collect();
        if !valid_name(name)
            || bins.is_empty()
            || bins.len() != tool.bins.len()
            || tool.bins.iter().any(|b| !valid_name(b))
            || tool
                .aliases
                .iter()
                .any(|(a, b)| !valid_name(a) || !bins.contains(b))
        {
            return Err(invalid(format!("invalid tool declaration: {name}")));
        }
    }
    if let Some(declaration) = &manifest.build_role {
        match (declaration.role, declaration.until.as_deref()) {
            (BuildRole::Builder, Some(until)) => {
                chrono::DateTime::parse_from_rfc3339(until).map_err(|error| {
                    invalid(format!("build role until must be RFC3339: {error}"))
                })?;
            }
            (BuildRole::Builder, None) => {
                return Err(invalid(
                    "builder is a standing commitment and needs an until time".into(),
                ));
            }
            (BuildRole::Carrier, Some(_)) => {
                return Err(invalid(
                    "carrier expiry belongs to the held-tool-watch carry switch, not the manifest"
                        .into(),
                ));
            }
            (BuildRole::Neither, Some(_)) => {
                return Err(invalid("neither carries no standing commitment".into()));
            }
            (BuildRole::Carrier | BuildRole::Neither, None) => {}
        }
    }
    Ok(())
}
/// How long a declared ready line may take to appear. A cold `epr` that opens its stores
/// before it serves is the slowest resident this ladder is written for.
pub const READY_LINE_PATIENCE_MS: u64 = 120_000;
/// The identity rung reads `/proc/<pid>/exe` once the service is ready; it is quick or the
/// artifact is a wrapper that exec'd something else, which the rung exists to catch.
pub const IDENTITY_PATIENCE_MS: u64 = 5_000;

/// How a run act launches its tool. This is an in-memory act, never a wire record: the
/// chain stores the [`RuntimeManifest`] the act produces. Every field left at its default
/// reproduces the historical child declaration exactly, so an argument-less act keeps the
/// manifest CID it always had.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunLaunch {
    /// Arguments appended after `{artifact}`; berth templates resolve at spawn.
    pub args: Vec<String>,
    /// A standard-output substring that declares the child ready. With it the readiness
    /// ladder is `StdoutLine` then `ExecutableIdentity`; without it the child makes no
    /// readiness claim and is supervised for death and shutdown only.
    pub ready_line: Option<String>,
    /// Restart eligibility; `None` keeps the policy default (permanent).
    pub restart: Option<Restart>,
    /// The process name this act declares or stops; `None` is the tool's own name. Acts
    /// replace and stop by this name, so two residents of one tool coexist under two names.
    pub name: Option<String>,
    /// Environment entries for the child, applied after the policy's scrub. Literal values
    /// land in the chain, so secret-named keys are refused (see [`secret_env_key`]).
    pub env: BTreeMap<String, String>,
}
impl RunLaunch {
    /// The process name this act targets, validated as a path-safe identifier.
    fn process_name(&self, tool: &str) -> Result<String, ManifestError> {
        match &self.name {
            None => Ok(tool.to_owned()),
            Some(name) if valid_name(name) => Ok(name.clone()),
            Some(name) => Err(ManifestError::Invalid(format!(
                "invalid process name '{name}'; expected letters, digits, '_', '.' or '-'"
            ))),
        }
    }
    fn child(self, tool: &str) -> Result<ChildSpec, ManifestError> {
        let name = self.process_name(tool)?;
        if self.args.iter().any(|arg| arg.contains('\0')) {
            return Err(ManifestError::Invalid(format!(
                "{name}: an argument carries a NUL byte, which exec cannot pass"
            )));
        }
        // Arguments are berth templates resolved at spawn, like environment values: refuse
        // now what spawn would refuse later, by position (1 is the first after `{artifact}`),
        // never echoing the argument itself.
        if let Some(position) = self
            .args
            .iter()
            .position(|arg| crate::berth::check_template(arg).is_err())
        {
            return Err(ManifestError::Invalid(format!(
                "{name}: argument {} has an unknown or malformed {{…}} placeholder",
                position + 1
            )));
        }
        for (key, value) in &self.env {
            if key.is_empty() || key.contains(['=', '\0']) || value.contains('\0') {
                return Err(ManifestError::Invalid(format!(
                    "{name}: environment entry '{key}' cannot be passed to exec"
                )));
            }
            if secret_env_key(key) {
                return Err(ManifestError::Invalid(format!(
                    "{name}: {key}: secrets are referenced by name only"
                )));
            }
            if loader_env_key(key) {
                return Err(ManifestError::Invalid(format!(
                    "{name}: {key}: loader keys would run code the passport never hashed"
                )));
            }
            // Values are berth templates resolved at spawn; refuse now what spawn would refuse
            // later. The value itself is never echoed.
            if crate::berth::check_template(value).is_err() {
                return Err(ManifestError::Invalid(format!(
                    "{name}: environment entry '{key}' has an unknown or malformed {{…}} placeholder"
                )));
            }
        }
        let readiness = match self.ready_line {
            Some(line) if line.is_empty() => {
                return Err(ManifestError::Invalid(format!(
                    "{tool}: an empty ready line would match any output"
                )))
            }
            Some(contains) => vec![
                Probe::StdoutLine {
                    contains,
                    patience_ms: READY_LINE_PATIENCE_MS,
                },
                Probe::ExecutableIdentity {
                    patience_ms: IDENTITY_PATIENCE_MS,
                },
            ],
            None => Vec::new(),
        };
        let mut policy = ChildPolicy::default();
        if let Some(restart) = self.restart {
            policy.restart = restart;
        }
        let mut argv = vec!["{artifact}".to_owned()];
        argv.extend(self.args);
        Ok(ChildSpec {
            name,
            artifact: ArtifactRef::Channel {
                channel_id: format!("bin:{tool}"),
            },
            argv,
            env: self.env,
            readiness,
            policy,
            ..ChildSpec::default()
        })
    }
}

/// Words that mark an environment key as a secret. A [`ChildSpec`] env entry is a literal
/// value stored in the chain with no by-name reference form, so such keys are refused.
pub const SECRET_ENV_WORDS: [&str; 10] = [
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "PASSPHRASE",
    "CREDENTIAL",
    "PRIVATE_KEY",
    "PRIVATEKEY",
    "API_KEY",
    "APIKEY",
];
/// Whether an environment key names a secret, case-insensitively.
pub fn secret_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    SECRET_ENV_WORDS.iter().any(|word| upper.contains(word))
}

/// Whether an environment key makes the dynamic loader inject code: any `LD_*` or `DYLD_*`
/// key, or `GCONV_PATH`, case-insensitively. A process whose passport claims its bytes must
/// not load bytes nobody hashed.
pub fn loader_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    upper.starts_with("LD_") || upper.starts_with("DYLD_") || upper == "GCONV_PATH"
}

/// Parse a restart policy as a person types it. Unknown words are refused, never defaulted.
pub fn parse_restart(raw: &str) -> Result<Restart, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "permanent" => Ok(Restart::Permanent),
        "transient" => Ok(Restart::Transient),
        "temporary" => Ok(Restart::Temporary),
        _ => Err(format!(
            "unknown restart policy '{raw}'; expected permanent, transient or temporary"
        )),
    }
}

/// An explicit private declaration act. Apply itself only repairs projections.
#[derive(Debug, Clone)]
pub enum ManifestAct {
    /// Replace the device's private build participation declaration.
    BuildRole(BuildRoleDeclaration),
    /// Set or remove one channel.
    Follow {
        channel: String,
        mode: Option<FollowMode>,
        pin: Option<String>,
    },
    /// Run or stop a registered tool using its local current artifact, as the process the
    /// launch names (the tool's name by default). A stop carries at most that name; any other
    /// launch detail on a stop is refused rather than ignored.
    Run {
        tool: String,
        stop: bool,
        launch: RunLaunch,
    },
    /// Replace the follow set from a verified config release.
    ReplaceFollows(Vec<Follow>),
}
/// Produce a new version without performing I/O or changing the supplied head.
pub fn supersede(
    head: &RuntimeManifest,
    act: ManifestAct,
) -> Result<RuntimeManifest, ManifestError> {
    let mut next = head.clone();
    next.supersedes = Some(head.cid()?);
    match act {
        ManifestAct::BuildRole(declaration) => next.build_role = Some(declaration),
        ManifestAct::Follow { channel, mode, pin } => {
            validate_channel(&channel).map_err(ManifestError::Invalid)?;
            let pos = next.follows.iter().position(|f| f.channel == channel);
            match (pos, mode) {
                (Some(i), Some(mode)) => next.follows[i] = Follow { channel, mode, pin },
                (None, Some(mode)) => next.follows.push(Follow { channel, mode, pin }),
                (Some(i), None) => {
                    next.follows.remove(i);
                }
                (None, None) => {}
            }
        }
        ManifestAct::Run { tool, stop, launch } => {
            let spec = next
                .tools
                .get(&tool)
                .ok_or_else(|| ManifestError::Invalid(format!("unknown tool {tool}")))?;
            let name = launch.process_name(&tool)?;
            let detail = RunLaunch {
                name: None,
                ..launch.clone()
            };
            if stop && detail != RunLaunch::default() {
                return Err(ManifestError::Invalid(format!(
                    "a stop of {name} carries no arguments, environment, ready line or restart policy"
                )));
            }
            if !stop && spec.bins.is_empty() {
                return Err(ManifestError::Invalid("tool has no executable".into()));
            }
            // A name never captures another tool's process, by declaration or by stop.
            let channel = format!("bin:{tool}");
            if let Some(existing) = next.processes.iter().find(|p| p.name == name) {
                let held = match &existing.artifact {
                    ArtifactRef::Channel { channel_id } if *channel_id == channel => None,
                    ArtifactRef::Channel { channel_id } => Some(channel_id.clone()),
                    ArtifactRef::Pinned { .. } => Some("a pinned artifact".to_owned()),
                };
                if let Some(held) = held {
                    return Err(ManifestError::Invalid(format!(
                        "process {name} runs {held}, not {channel}; {tool} cannot replace or stop it"
                    )));
                }
            }
            next.processes.retain(|p| p.name != name);
            if !stop {
                next.processes.push(launch.child(&tool)?);
            }
        }
        ManifestAct::ReplaceFollows(follows) => next.follows = follows,
    }
    next.validate()?;
    Ok(next)
}
/// Render the legacy watched list. Pins remain constraints in the manifest.
pub fn render_channels(follows: &[Follow]) -> String {
    follows
        .iter()
        .map(|f| format!("{}={}", f.channel, f.mode.label()))
        .collect::<Vec<_>>()
        .join(",")
}
/// Render the watched TOML projection.
pub fn render_runtime_config(manifest: &RuntimeManifest) -> String {
    format!(
        "{RELEASE_CHANNELS_KEY} = \"{}\"\n",
        render_channels(&manifest.follows)
    )
}
/// Render the existing camelCase executable registry.
pub fn render_bin_tools(manifest: &RuntimeManifest) -> Result<String, ManifestError> {
    serde_json::to_string_pretty(&serde_json::json!({"version":1,"tools":manifest.tools}))
        .map_err(|e| ManifestError::Json(e.to_string()))
}
/// Render the canonical declaration as JSON for filesystem and fleet consumers.
pub fn render_manifest(manifest: &RuntimeManifest) -> Result<String, ManifestError> {
    serde_json::to_string_pretty(manifest).map_err(|e| ManifestError::Json(e.to_string()))
}

/// A declared release pin must be satisfied by the canonical head's content identity.
/// Absence is not a match. The head action hash remains the adoption idempotency key.
pub fn require_release_pin(
    manifest: &RuntimeManifest,
    channel: &str,
    content_cid: Option<&str>,
) -> Result<(), ManifestError> {
    if let Some(pin) = manifest
        .follows
        .iter()
        .find(|f| f.channel == channel)
        .and_then(|f| f.pin.as_deref())
    {
        if content_cid != Some(pin) {
            return Err(ManifestError::Invalid(format!(
                "channel {channel} requires release {pin}; observed {}",
                content_cid.unwrap_or("no content CID")
            )));
        }
    }
    Ok(())
}

/// CamelCase view of the device declaration; existing native manifest bytes retain their ABI.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestView {
    /// CID of the exact native declaration.
    pub manifest_cid: String,
    /// Previous version, or null for a base.
    pub supersedes: Option<String>,
    /// This device's declared build participation.
    pub build_role: Option<BuildRoleDeclaration>,
    /// Channels this device declares.
    pub follows: Vec<Follow>,
    /// Executable registry.
    pub tools: std::collections::BTreeMap<String, ToolSpec>,
    /// Processes declared to run; execution is reported separately by the passport.
    pub processes: Vec<ProcessView>,
}
/// Process declaration view with camelCase artifact addressing.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessView {
    /// Stable process name.
    pub name: String,
    /// Local current channel, if channel-based.
    pub channel_id: Option<String>,
    /// Immutable artifact CID, when supplied.
    pub artifact_cid: Option<String>,
    /// Immutable artifact SHA-256, when pinned.
    pub artifact_sha256: Option<String>,
    /// Declared arguments.
    pub argv: Vec<String>,
}
/// Build a view without I/O or authority changes.
pub fn view(manifest: &RuntimeManifest) -> Result<ManifestView, ManifestError> {
    Ok(ManifestView {
        manifest_cid: manifest.cid()?,
        supersedes: manifest.supersedes.clone(),
        build_role: manifest.build_role.clone(),
        follows: manifest.follows.clone(),
        tools: manifest.tools.clone(),
        processes: manifest
            .processes
            .iter()
            .map(|p| {
                let (channel_id, artifact_cid, artifact_sha256) = match &p.artifact {
                    crate::ArtifactRef::Channel { channel_id } => {
                        (Some(channel_id.clone()), None, None)
                    }
                    crate::ArtifactRef::Pinned { cid, sha256, .. } => {
                        (None, cid.clone(), Some(sha256.clone()))
                    }
                };
                ProcessView {
                    name: p.name.clone(),
                    channel_id,
                    artifact_cid,
                    artifact_sha256,
                    argv: p.argv.clone(),
                }
            })
            .collect(),
    })
}
