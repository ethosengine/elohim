//! Private manifest chain and its projections. This is the only runtime-config writer.
//! Immutable versions precede atomic head replacement; boot can replay a committed head.
use crate::bin_store::{files, Refusal, Registry, Result};
use ark_core::{
    declaration::{self, ManifestAct},
    RuntimeManifest,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

/// The offline workspace default is carried with the executable.
pub const WORKSPACE_BASE: &str = include_str!(
    "../../../../genesis/orchestrator/manifests/runtime/workspace-device.manifest.json"
);

fn invalid(error: impl std::fmt::Display) -> Refusal {
    Refusal::new("manifest-invalid", error.to_string())
}
/// Parse the committed base, without creating device state.
pub fn workspace_base() -> Result<RuntimeManifest> {
    RuntimeManifest::from_json(WORKSPACE_BASE).map_err(invalid)
}

/// Destinations explicitly owned by this device, never inferred from another checkout.
#[derive(Debug, Clone)]
pub struct Projections {
    /// Watched node-config file; None for a device without a storage node.
    pub runtime_config: Option<PathBuf>,
    /// Registry beside the launchers; None for a fleet node without CLI tools.
    pub bin_tools: Option<PathBuf>,
}
// Both values are chain state: committing an act also acknowledges its deployment base.
#[derive(serde::Serialize, serde::Deserialize)]
struct Head {
    cid: String,
    base: String,
}
/// One steward device's declaration store.
#[derive(Debug, Clone)]
pub struct ManifestStore {
    root: PathBuf,
    base: RuntimeManifest,
    explicit_base: bool,
    projections: Projections,
}
impl ManifestStore {
    /// Bind a private directory, a committed base and explicit projection paths.
    pub fn new(
        root: impl Into<PathBuf>,
        base: RuntimeManifest,
        projections: Projections,
    ) -> Result<Self> {
        base.validate().map_err(invalid)?;
        validate_secrets(&base)?;
        Ok(Self {
            root: root.into(),
            base,
            explicit_base: true,
            projections,
        })
    }
    /// Construct a workspace declaration store without network or live-store access.
    pub fn workspace(root: impl Into<PathBuf>, projections: Projections) -> Result<Self> {
        Self::new(root, workspace_base()?, projections)
    }
    /// Bind an explicit upgrade, or recover the chain's own verified base. A saved
    /// projection supplies a default only before the chain has a head.
    pub fn from_source(
        root: impl Into<PathBuf>,
        source: Option<&Path>,
        projections: Projections,
    ) -> Result<Self> {
        let root = root.into();
        if let Some(path) = source {
            return Self::new(
                root,
                RuntimeManifest::from_json(&fs::read_to_string(path)?).map_err(invalid)?,
                projections,
            );
        }
        let mut store = Self::workspace(root, projections)?;
        store.explicit_base = false;
        store.base = store.base()?;
        Ok(store)
    }
    fn base(&self) -> Result<RuntimeManifest> {
        if !self.explicit_base {
            if let Some(head) = self.head()? {
                return self.version(&head.base);
            }
            let base = match fs::read_to_string(self.root.join("runtime-manifest.json")) {
                Ok(text) => RuntimeManifest::from_json(&text).map_err(invalid)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => workspace_base()?,
                Err(e) => return Err(e.into()),
            };
            base.validate().map_err(invalid)?;
            validate_secrets(&base)?;
            return Ok(base);
        }
        Ok(self.base.clone())
    }
    fn chain(&self) -> PathBuf {
        self.root.join("manifest")
    }
    /// Read and verify the current immutable version; only an absent head uses the base.
    pub fn read(&self) -> Result<RuntimeManifest> {
        let Some(head) = self.head()? else {
            return self.base();
        };
        self.version(&head.cid)
    }
    fn version(&self, cid: &str) -> Result<RuntimeManifest> {
        let bytes = fs::read(self.chain().join(format!("{cid}.cbor")))?;
        let manifest: RuntimeManifest = RuntimeManifest::from_cbor(&bytes).map_err(invalid)?;
        manifest.validate().map_err(invalid)?;
        validate_secrets(&manifest)?;
        if manifest.cid().map_err(invalid)? != cid {
            return Err(Refusal::new("manifest-cid-mismatch", cid));
        }
        Ok(manifest)
    }
    fn head(&self) -> Result<Option<Head>> {
        match fs::read(self.chain().join("head")) {
            Ok(bytes) => {
                let head: Head = serde_json::from_slice(&bytes).map_err(invalid)?;
                files::require_cid(&head.cid)?;
                files::require_cid(&head.base)?;
                Ok(Some(head))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    /// Serialize an act and its projections under the same chain lock.
    pub fn act(&self, act: ManifestAct) -> Result<RuntimeManifest> {
        let _lock = files::Lock::acquire(&self.chain(), Duration::from_secs(30))?;
        self.write_act(act, None)
    }
    fn write_act(&self, act: ManifestAct, payload: Option<&str>) -> Result<RuntimeManifest> {
        let head = self.reconcile_base()?;
        let next = declaration::supersede(&head, act).map_err(invalid)?;
        validate_secrets(&next)?;
        self.carry_config(&next, payload)?;
        self.commit(&next)?;
        self.project(&next)?;
        Ok(next)
    }
    /// Legacy callers can change a mode but cannot express removal of a pin.
    pub fn follow_mode(
        &self,
        channel: String,
        mode: Option<declaration::FollowMode>,
    ) -> Result<RuntimeManifest> {
        let _lock = files::Lock::acquire(&self.chain(), Duration::from_secs(30))?;
        let pin = self
            .reconcile_base()?
            .follows
            .into_iter()
            .find(|f| f.channel == channel)
            .and_then(|f| f.pin);
        self.write_act(ManifestAct::Follow { channel, mode, pin }, None)
    }
    fn commit(&self, manifest: &RuntimeManifest) -> Result<()> {
        let base = self.base()?;
        let base_cid = base.cid().map_err(invalid)?;
        files::atomic_bytes(
            &self.chain().join(format!("{base_cid}.cbor")),
            &base.canonical_bytes().map_err(invalid)?,
            0o600,
        )?;
        let cid = manifest.cid().map_err(invalid)?;
        files::atomic_bytes(
            &self.chain().join(format!("{cid}.cbor")),
            &manifest.canonical_bytes().map_err(invalid)?,
            0o600,
        )?;
        files::atomic_json(
            &self.chain().join("head"),
            &Head {
                cid,
                base: base_cid,
            },
        )
    }
    /// Repair projections after a crash or boot; an explicitly changed base is an act.
    pub fn apply(&self) -> Result<RuntimeManifest> {
        let _lock = files::Lock::acquire(&self.chain(), Duration::from_secs(30))?;
        let manifest = self.reconcile_base()?;
        self.project(&manifest)?;
        Ok(manifest)
    }
    fn reconcile_base(&self) -> Result<RuntimeManifest> {
        self.claim_projections()?;
        let mut manifest = self.read()?;
        // An explicitly supplied deployment base is a new declaration, not an edit of
        // the projection. Retain the private head in its supersedes chain on upgrade.
        if let Some(head) = self.head()? {
            if self.explicit_base && head.base != self.base.cid().map_err(invalid)? {
                manifest = self.base.clone();
                manifest.supersedes = Some(head.cid);
                self.carry_config(&manifest, None)?;
                self.commit(&manifest)?;
            }
        }
        Ok(manifest)
    }
    /// Seed missing settings and reconcile explicitly changed deployment settings.
    /// An unchanged seed preserves private config releases; follows always come from head.
    pub fn seed_settings(&self, payload: &str) -> Result<()> {
        validate_config_secrets(payload)?;
        let _lock = files::Lock::acquire(&self.chain(), Duration::from_secs(30))?;
        let head = self.reconcile_base()?;
        let seed = self.root.join("runtime-config-seed.toml");
        let previous = match fs::read_to_string(&seed) {
            Ok(previous) => Some(previous),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        if previous
            .as_deref()
            .is_some_and(|previous| previous != payload)
        {
            self.write_act(ManifestAct::ReplaceFollows(head.follows), Some(payload))?;
            return files::atomic_bytes(&seed, payload.as_bytes(), 0o600);
        }
        let replay = self
            .chain()
            .join(format!("{}.runtime-config", head.cid().map_err(invalid)?));
        if !replay.exists()
            && !self
                .projections
                .runtime_config
                .as_ref()
                .is_some_and(|path| path.exists())
        {
            files::atomic_bytes(&replay, payload.as_bytes(), 0o600)?;
        }
        files::atomic_bytes(&seed, payload.as_bytes(), 0o600)
    }

    /// Replace the declared set using the shared legacy grammar; refuse partial imports.
    pub fn follow_list(&self, raw: &str) -> Result<RuntimeManifest> {
        let mut parsed = declaration::parse_followed_channels(raw);
        if !parsed.refused.is_empty() {
            return Err(invalid(format!("invalid follows: {:?}", parsed.refused)));
        }
        let _lock = files::Lock::acquire(&self.chain(), Duration::from_secs(30))?;
        self.retain_pins(&mut parsed.follows)?;
        self.write_act(ManifestAct::ReplaceFollows(parsed.follows), None)
    }
    fn retain_pins(&self, follows: &mut [declaration::Follow]) -> Result<()> {
        let head = self.reconcile_base()?;
        for follow in follows {
            follow.pin = head
                .follows
                .iter()
                .find(|old| old.channel == follow.channel)
                .and_then(|old| old.pin.clone());
        }
        Ok(())
    }

    /// Apply a verified config document through the same chain writer. Non-follow settings
    /// retain their elohim-settings home; the manifest is authoritative for its follow set.
    pub fn config_release(&self, payload: &str) -> Result<RuntimeManifest> {
        validate_config_secrets(payload)?;
        let _lock = files::Lock::acquire(&self.chain(), Duration::from_secs(30))?;
        let parsed = elohim_settings::runtime_config::parse(payload);
        let raw = parsed
            .get(declaration::RELEASE_CHANNELS_KEY)
            .map(String::as_str)
            .unwrap_or("");
        let mut follows = declaration::parse_followed_channels(raw);
        if !follows.refused.is_empty() {
            return Err(invalid(format!(
                "invalid follow entries: {:?}",
                follows.refused
            )));
        }
        self.retain_pins(&mut follows.follows)?;
        self.write_act(ManifestAct::ReplaceFollows(follows.follows), Some(payload))
    }
    // Settings retain their existing authority. This private replay input is durable before
    // head advancement so an interrupted config release can finish at boot.
    fn carry_config(&self, next: &RuntimeManifest, payload: Option<&str>) -> Result<()> {
        let prior = next
            .supersedes
            .as_ref()
            .map(|cid| self.chain().join(format!("{cid}.runtime-config")));
        let inherited = match prior {
            Some(path) => match fs::read_to_string(path) {
                Ok(s) => Some(s),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            },
            None => None,
        };
        if let Some(text) = payload.or(inherited.as_deref()) {
            validate_config_secrets(text)?;
            files::atomic_bytes(
                &self
                    .chain()
                    .join(format!("{}.runtime-config", next.cid().map_err(invalid)?)),
                text.as_bytes(),
                0o600,
            )?;
        }
        Ok(())
    }
    fn claim_projections(&self) -> Result<()> {
        fs::create_dir_all(self.chain())?;
        let owner = fs::canonicalize(self.chain())?;
        for path in [
            Some(self.root.join("runtime-manifest.json")),
            self.projections.runtime_config.clone(),
            self.projections.bin_tools.clone(),
        ]
        .into_iter()
        .flatten()
        {
            claim_projection(&path, &owner)?;
        }
        Ok(())
    }
    fn project(&self, manifest: &RuntimeManifest) -> Result<()> {
        self.claim_projections()?;
        files::atomic_bytes(
            &self.root.join("runtime-manifest.json"),
            declaration::render_manifest(&self.base()?)
                .map_err(invalid)?
                .as_bytes(),
            0o600,
        )?;
        if let Some(path) = &self.projections.runtime_config {
            let replay = self.chain().join(format!(
                "{}.runtime-config",
                manifest.cid().map_err(invalid)?
            ));
            let old = match fs::read_to_string(replay) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    match fs::read_to_string(path) {
                        Ok(s) => s,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                        Err(e) => return Err(e.into()),
                    }
                }
                Err(e) => return Err(e.into()),
            };
            let next = elohim_settings::runtime_config::rewrite_key_line(
                &old,
                declaration::RELEASE_CHANNELS_KEY,
                Some(&declaration::render_channels(&manifest.follows)),
            );
            files::atomic_bytes(path, next.as_bytes(), 0o600)?;
        }
        if let Some(path) = &self.projections.bin_tools {
            write_registry(path, &registry(manifest)?)?;
        }
        Ok(())
    }
}
// ChildSpec.env has no by-name reference form. Do not persist literal secrets in a chain.
fn validate_secrets(manifest: &RuntimeManifest) -> Result<()> {
    for process in &manifest.processes {
        for key in process.env.keys() {
            validate_env_key(key)?;
        }
    }
    Ok(())
}
fn validate_config_secrets(payload: &str) -> Result<()> {
    for key in elohim_settings::runtime_config::parse(payload).keys() {
        validate_env_key(key)?;
    }
    Ok(())
}
fn validate_env_key(key: &str) -> Result<()> {
    let upper = key.to_ascii_uppercase();
    if [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSPHRASE",
        "CREDENTIAL",
        "PRIVATE_KEY",
        "API_KEY",
    ]
    .iter()
    .any(|name| upper.contains(name))
    {
        return Err(invalid(format!(
            "{key}: secrets are referenced by name only"
        )));
    }
    Ok(())
}
fn owner_marker(path: &Path) -> PathBuf {
    let mut marker = path.as_os_str().to_owned();
    marker.push(".owner");
    marker.into()
}
fn claim_projection(path: &Path, owner: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("projection has no parent"))?;
    fs::create_dir_all(parent)?;
    let _lock = files::Lock::acquire(parent, Duration::from_secs(30))?;
    let marker = owner_marker(path);
    match fs::read_to_string(&marker) {
        Ok(existing) if Path::new(&existing) == owner => Ok(()),
        Ok(existing) => Err(Refusal::new(
            "projection-owner-conflict",
            format!(
                "{} is owned by chain {}; writer {} refused",
                path.display(),
                existing,
                owner.display()
            ),
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            files::atomic_bytes(&marker, owner.as_os_str().as_encoded_bytes(), 0o600)
        }
        Err(e) => Err(e.into()),
    }
}
/// Storage boot decision, shared with boot regression tests. A declaration needs a watcher;
/// a watcher with private chain ownership repairs even when its head has been deleted.
pub fn boot_runtime_config(
    path: Option<&Path>,
    base: Option<&Path>,
    seed: Option<&str>,
) -> Result<()> {
    match path {
        Some(path) => repair_runtime_config(path, base, seed),
        None if base.is_some() => Err(invalid(
            "declared runtime requires ELOHIM_RUNTIME_CONFIG_PATH",
        )),
        None => Ok(()),
    }
}
/// Repair a watched projection if a chain or an explicit deployment declaration exists.
pub fn repair_runtime_config(path: &Path, base: Option<&Path>, seed: Option<&str>) -> Result<()> {
    let Some(store) = discover_runtime_config(path, base)? else {
        return Ok(());
    };
    if let Some(seed) = seed {
        store.seed_settings(seed)?;
    }
    store.apply()?;
    Ok(())
}
/// Discover the declaration boot and adoption share, including an external projection's owner.
/// Absence is a legacy unowned config; a declared but unreadable owner must refuse.
pub fn discover_runtime_config(path: &Path, base: Option<&Path>) -> Result<Option<ManifestStore>> {
    let declared = base.is_some()
        || owner_marker(path).exists()
        || path.parent().is_some_and(|root| {
            root.join("manifest").is_dir() || root.join("runtime-manifest.json").is_file()
        });
    declared
        .then(|| for_runtime_config_with_base(path, base))
        .transpose()
}
/// Adapt the declared tool specs to the existing validated registry reader.
pub fn registry(manifest: &RuntimeManifest) -> Result<Registry> {
    serde_json::from_str(&declaration::render_bin_tools(manifest).map_err(invalid)?)
        .map_err(Into::into)
}
/// The registry projection writer; launcher installation never rolls this authority back.
pub fn write_registry(path: &Path, registry: &Registry) -> Result<()> {
    files::atomic_json(path, registry)
}
/// A node-local caller binds the chain beside its watched file, not to a developer's device.
pub fn for_runtime_config(path: &Path) -> Result<ManifestStore> {
    let base = std::env::var_os("ELOHIM_RUNTIME_MANIFEST_PATH")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from);
    for_runtime_config_with_base(path, base.as_deref())
}
/// Explicit fixture/node constructor: never consult ambient runtime manifest authority.
pub fn for_runtime_config_with_base(path: &Path, base: Option<&Path>) -> Result<ManifestStore> {
    let marker = owner_marker(path);
    let root = match fs::read_to_string(marker) {
        Ok(owner) => PathBuf::from(owner)
            .parent()
            .ok_or_else(|| invalid("owner chain has no parent"))?
            .to_owned(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => path
            .parent()
            .ok_or_else(|| invalid("runtime config has no parent"))?
            .to_owned(),
        Err(e) => return Err(e.into()),
    };
    let base_path = base
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runtime-manifest.json"));
    if base.is_none() && !base_path.exists() {
        // A node with existing configuration cannot guess that it is a workspace device.
        // The harness/render path must install its committed base before its first act.
        if path.is_file() && !root.join("manifest/head").is_file() {
            return Err(Refusal::new(
                "manifest-base-absent",
                format!(
                    "declare {} before changing the node follow set",
                    base_path.display()
                ),
            ));
        }
    }
    ManifestStore::from_source(
        root,
        base,
        Projections {
            runtime_config: Some(path.into()),
            bin_tools: None,
        },
    )
}

pub use ark_core::declaration::{view, ManifestView, ProcessView};

/// Reconstruct applied tools from verified custody. A remote runtime's unknown application
/// remains null; callers with observed channel receipts can supply them explicitly.
pub fn observe(
    manifest: &RuntimeManifest,
    bin_root: &Path,
    applied: &std::collections::BTreeMap<String, String>,
) -> Result<ark_core::passport::RuntimeObservation> {
    let registry = registry(manifest)?;
    observe_with(manifest, applied, |tool| {
        crate::bin_store::Store::new(bin_root.join(tool), tool, &registry)
    })
}

/// Observe the same selected custody used by the native driver, including explicit
/// per-tool store overrides. This is for a running runtime, never an explicit-root view.
pub fn observe_local(manifest: &RuntimeManifest) -> Result<ark_core::passport::RuntimeObservation> {
    let registry = Registry::discover();
    observe_with(manifest, &Default::default(), |tool| match &registry {
        Ok(registry) => crate::bin_store::Store::from_environment(tool, registry),
        Err(error) => Err(error.clone()),
    })
}

fn observe_with(
    manifest: &RuntimeManifest,
    applied: &std::collections::BTreeMap<String, String>,
    store: impl Fn(&str) -> Result<crate::bin_store::Store>,
) -> Result<ark_core::passport::RuntimeObservation> {
    use ark_core::passport::{FollowPassport, RuntimeObservation, ToolPassport};
    let tools: Vec<_> = manifest
        .tools
        .keys()
        .map(|tool| {
            let current = store(tool)
                .and_then(|store| store.current_artifact())
                .ok()
                .map(|(cid, _)| cid);
            ToolPassport {
                tool: tool.clone(),
                applied_release: current,
            }
        })
        .collect();
    let follows = manifest
        .follows
        .iter()
        .map(|f| FollowPassport {
            channel: f.channel.clone(),
            mode: f.mode,
            applied_release: f
                .channel
                .strip_prefix("bin:")
                .and_then(|name| tools.iter().find(|t| t.tool == name))
                .and_then(|t| t.applied_release.clone())
                .or_else(|| applied.get(&f.channel).cloned()),
        })
        .collect();
    Ok(RuntimeObservation { follows, tools })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_core::declaration::FollowMode;

    #[test]
    fn committed_base_survives_a_failed_projection_after_an_act() {
        let dir = tempfile::tempdir().unwrap();
        let projections = Projections {
            runtime_config: None,
            bin_tools: None,
        };
        let a = ManifestStore::workspace(dir.path(), projections.clone()).unwrap();
        a.act(ManifestAct::Follow {
            channel: "bin:old".into(),
            mode: Some(FollowMode::Observe),
            pin: None,
        })
        .unwrap();
        let saved = dir.path().join("runtime-manifest.json");
        let saved_a = fs::read(&saved).unwrap();
        let mut base = workspace_base().unwrap();
        base.follows[0].mode = FollowMode::Canary;
        let b = ManifestStore::new(dir.path(), base.clone(), projections).unwrap();
        let committed;
        let head;
        {
            let _lock = files::Lock::acquire(&b.chain(), Duration::from_secs(30)).unwrap();
            // Drive the act's commit boundary with saved A still readable throughout
            // reconciliation. This lets the test stop exactly before project().
            committed = declaration::supersede(
                &b.reconcile_base().unwrap(),
                ManifestAct::Follow {
                    channel: "bin:new".into(),
                    mode: Some(FollowMode::Observe),
                    pin: None,
                },
            )
            .unwrap();
            b.commit(&committed).unwrap();
            head = fs::read(b.chain().join("head")).unwrap();
            assert_eq!(fs::read(&saved).unwrap(), saved_a);
            assert_eq!(committed.follows[0].mode, FollowMode::Canary);
            assert!(committed.follows.iter().any(|f| f.channel == "bin:new"));
            // The projection's rename fails even under root. No runtime failure switch.
            fs::remove_file(&saved).unwrap();
            fs::create_dir(&saved).unwrap();
            assert!(b.project(&committed).is_err());
            fs::remove_dir(&saved).unwrap();
            fs::write(&saved, saved_a).unwrap();
        }
        assert_eq!(b.apply().unwrap(), committed);
        assert_eq!(fs::read(b.chain().join("head")).unwrap(), head);
        let record: serde_json::Value = serde_json::from_slice(&head).unwrap();
        assert_eq!(record["base"], base.cid().unwrap());
    }
}
