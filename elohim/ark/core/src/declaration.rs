//! Pure device declarations, acts and projections. No filesystem or runtime authority.
use crate::manifest::{ArtifactRef, ChildSpec, ManifestError, RuntimeManifest};
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
    Ok(())
}
/// An explicit private declaration act. Apply itself only repairs projections.
#[derive(Debug, Clone)]
pub enum ManifestAct {
    /// Set or remove one channel.
    Follow {
        channel: String,
        mode: Option<FollowMode>,
        pin: Option<String>,
    },
    /// Run or stop a registered tool using its local current artifact.
    Run { tool: String, stop: bool },
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
        ManifestAct::Run { tool, stop } => {
            let spec = next
                .tools
                .get(&tool)
                .ok_or_else(|| ManifestError::Invalid(format!("unknown tool {tool}")))?;
            next.processes.retain(|p| p.name != tool);
            if !stop {
                next.processes.push(ChildSpec {
                    name: tool.clone(),
                    artifact: ArtifactRef::Channel {
                        channel_id: format!("bin:{tool}"),
                    },
                    argv: vec!["{artifact}".into()],
                    ..ChildSpec::default()
                });
                if spec.bins.is_empty() {
                    return Err(ManifestError::Invalid("tool has no executable".into()));
                }
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
