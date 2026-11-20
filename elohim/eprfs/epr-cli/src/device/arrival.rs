//! Arrival is a reconstructed local view (C); it creates no peer facts or DHT heads.
use crate::{
    actor, device_key,
    flow::memory::recall::{embedder, index},
    git,
};
use elohim_device_runtime::{
    bin_store::{Refusal, Registry, Store},
    layout::{self, DeviceLayout, Field},
    session,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

/// Whether a condition works, lacks an enhancement, prevents work, or cannot be observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// The condition is met.
    Ok,
    /// Local work remains possible without this enhancement.
    Degraded,
    /// The device cannot safely perform its work.
    Refused,
    /// The fact cannot currently be observed (never assumed false).
    Unknown,
}
/// One arrival condition and its actionable remedy, in dependency order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// Stable human-readable condition name.
    pub name: String,
    /// Observed condition.
    pub state: State,
    /// Evidence or the reason it could not be obtained.
    pub detail: String,
    /// Command or action that restores the condition.
    pub cure: Option<String>,
}
impl Check {
    fn new(name: &str, state: State, detail: impl Into<String>, cure: Option<String>) -> Self {
        Self {
            name: name.into(),
            state,
            detail: detail.into(),
            cure,
        }
    }
}
impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self.state {
            State::Ok => "ok",
            State::Degraded => "degraded",
            State::Refused => "REFUSED",
            State::Unknown => "unknown",
        };
        write!(f, "{state} {}: {}", self.name, self.detail)?;
        if let Some(cure) = &self.cure {
            write!(f, " — {cure}")?;
        }
        Ok(())
    }
}

/// Read this device's arrival conditions; never enroll, install, build or write during a status
/// read.
pub fn status() -> Vec<Check> {
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    status_at(&git::checkout(&here).unwrap_or(here), false)
}

/// The arrival conditions at `root`. `remember` (only `epr device arrive`) also records this
/// host's sysroot observation, so a later read can say when it changed.
pub(crate) fn status_at(root: &Path, remember: bool) -> Vec<Check> {
    let identity = session::resolve();
    let registry = Registry::discover();
    let env = |key: &str| std::env::var(key).ok();
    let binary = |tool| {
        match &registry {
        Ok(registry) => match Store::from_environment(tool, registry) {
            Ok(store) => tool_check(tool, &store, &identity, &env),
            Err(error) => Check::new(
                &format!("{tool} binary"),
                State::Refused,
                error.to_string(),
                Some(format!("epr bin bootstrap --tool {tool}")),
            ),
        },
        Err(error) => Check::new(
            &format!("{tool} binary"),
            State::Refused,
            error.to_string(),
            Some("restore genesis/orchestrator/manifests/runtime/workspace-device.manifest.json or set EPR_BIN_TOOLS".into()),
        ),
    }
    };
    let key = device_key::resolve_path();
    let handle = sole_handle(root).unwrap_or_else(|| "<handle>".into());
    // The same resolvers every other device command reads: status names, never re-derives.
    let device_layout = resolve_layout(root);
    let persistence =
        crate::session_envelope::resolve_persistence_with(&|name: &str| std::env::var_os(name))
            .map(|persistence| describe_basis(persistence.basis()));
    let key_check = match &key {
        Ok(path) => key_check(path, &handle, &persistence),
        Err(error) => Check::new(
            "device key",
            State::Refused,
            error.to_string(),
            Some("set XDG_CONFIG_HOME to this device's persistent config directory".into()),
        ),
    };
    let current = key
        .as_deref()
        .ok()
        .map_or(actor::DeviceState::Unwitnessed, |key| {
            actor::device_state_read_only(root, key)
        });
    let published = published_refs(registry.as_ref().ok());
    let mut checks = vec![
        binary("epr"),
        binary("brit"),
        key_check,
        roster_check(root, &current, &handle, &published),
        model_check(root),
        fold_check(
            index::status(root, index::EmbedderChoice::Pinned),
            lag_limit(root),
        ),
        match berth_dir(&env, device_layout.as_ref().ok()) {
            Ok(dir) => berth_check(&dir, identity.session.as_deref()),
            Err(why) => Check::new(
                "berth moored",
                State::Degraded,
                why,
                Some(format!(
                    "set {} to this device's state directory",
                    Field::StateDir.env()
                )),
            ),
        },
        repository_check(root),
        peer_daemon_check(root),
        sibling_check(root, &current),
        supervisor_check(root),
        // Restart-arrival facts (backlog container-restart-strands-session-envelope).
        toolchain_check(root, &env),
        sysroot::line(
            &env,
            device_layout
                .as_ref()
                .map_err(|refusal| refusal.to_string())
                .and_then(|l| l.state_dir().map_err(|refusal| refusal.to_string())),
            remember,
        ),
        hooks_check(root),
        brit_trust_check(root),
        packs_check(root),
        layout_check(device_layout),
    ];
    // Always last: what the person does next, read from the lines above.
    let next = next_step(&checks);
    checks.push(next);
    checks
}

/// The conditions of the joining ceremony, in the order a person resolves them: the tool, the
/// key (and the persistence its line names), the roster (enroll · authorize · bind, then
/// publish), the peer, then the exchange with a sibling.
const CEREMONY: [&str; 5] = [
    "epr binary",
    "device key",
    ROSTER_CHECK,
    "peer daemon",
    SIBLING_CHECK,
];
const NEXT_CHECK: &str = "next step";

/// The first unresolved ceremony condition's cure, repeated verbatim, or that nothing is owed.
/// Never refuses: it restates a line above, and the exit status is that line's to set.
fn next_step(checks: &[Check]) -> Check {
    let ceremony: Vec<&Check> = CEREMONY
        .iter()
        .filter_map(|name| checks.iter().find(|c| c.name == *name))
        .collect();
    if let Some(owed) = ceremony
        .iter()
        .find(|c| c.state != State::Ok && c.cure.is_some())
    {
        let state = if owed.state == State::Unknown {
            State::Unknown
        } else {
            State::Degraded
        };
        return Check::new(NEXT_CHECK, state, owed.name.clone(), owed.cure.clone());
    }
    // Unresolved with no cure is a fact, not a debt: a person with one device owes no sibling.
    let detail = match ceremony.iter().find(|c| c.state != State::Ok) {
        Some(open) => format!("nothing owed; this device is arrived ({})", open.detail),
        None => "nothing owed; this device is arrived and exchanging".to_owned(),
    };
    Check::new(NEXT_CHECK, State::Ok, detail, None)
}

/// The remote-tracking refs the device declares as published: the epr tool's
/// `source.publishedRef` (the refs `epr bin land` accepts as landed), narrowed to the ones a
/// fetch writes. A local branch is never publication; only what the shared branch carried at the
/// last fetch is.
fn published_refs(registry: Option<&Registry>) -> Vec<String> {
    registry
        .and_then(|registry| registry.tool("epr").ok())
        .and_then(|tool| tool.source["publishedRef"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .filter(|reference| reference.starts_with("refs/remotes/"))
        .map(str::to_owned)
        .collect()
}

/// Where this device's roster row stands against the published refs, as of the last fetch.
#[derive(Debug, PartialEq, Eq)]
enum Publication {
    /// The row is on this published ref.
    Published(String),
    /// The row is committed here (`true`) or only in the working tree (`false`), not on the ref.
    Local { reference: String, committed: bool },
    /// None of these refs has been fetched here.
    NeverFetched(Vec<String>),
    /// The device declares no published ref.
    Undeclared,
    /// git could not read the repository.
    Unreadable(String),
}

/// Whether `reference` names a commit here: `Ok(false)` when it is absent, `Err` when git
/// refused to read the repository. A read: no fetch, no write.
fn ref_exists(root: &Path, reference: &str) -> Result<bool, String> {
    let output = git::command(root)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{reference}^{{commit}}"))
        .output()
        .map_err(|e| format!("git: {e}"))?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()),
    }
}

/// The roster file's bytes at `revision`, or `None` when that revision does not carry it.
fn roster_at(root: &Path, revision: &str, rel: &str) -> Option<String> {
    let output = git::command(root)
        .arg("show")
        .arg(format!("{revision}:{rel}"))
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn publication(root: &Path, handle: &str, did: &str, refs: &[String]) -> Publication {
    if refs.is_empty() {
        return Publication::Undeclared;
    }
    let rel = actor::roster_rel(handle);
    let lists = |revision: &str| {
        roster_at(root, revision, &rel)
            .is_some_and(|text| actor::roster_text_lists(handle, &text, did))
    };
    let mut fetched = None;
    for reference in refs {
        match ref_exists(root, reference) {
            Ok(true) if lists(reference) => return Publication::Published(reference.clone()),
            Ok(true) => {
                fetched.get_or_insert_with(|| reference.clone());
            }
            Ok(false) => {}
            Err(why) => return Publication::Unreadable(why),
        }
    }
    match fetched {
        Some(reference) => Publication::Local {
            reference,
            committed: lists("HEAD"),
        },
        None => Publication::NeverFetched(refs.to_vec()),
    }
}

/// `refs/remotes/origin/dev` → `origin/dev`.
fn short_ref(reference: &str) -> &str {
    reference.strip_prefix("refs/remotes/").unwrap_or(reference)
}

/// The roster line once this device stands: published, local only, or not knowable offline.
fn publication_check(standing: &actor::Standing, publication: Publication) -> Check {
    let described = standing.describe();
    let handle = &standing.handle;
    let rel = actor::roster_rel(handle);
    match publication {
        Publication::Published(reference) => Check::new(
            ROSTER_CHECK,
            State::Ok,
            format!(
                "{described}; published on {} (as of the last fetch)",
                short_ref(&reference)
            ),
            None,
        ),
        Publication::Local {
            reference,
            committed,
        } => Check::new(
            ROSTER_CHECK,
            State::Degraded,
            format!(
                "{described}; {handle}'s roster row for this device is local only ({}), not on \
                 {} (as of the last fetch), so no other device of {handle}'s can recognise it yet",
                if committed {
                    "in a local commit"
                } else {
                    "in the working tree"
                },
                short_ref(&reference)
            ),
            Some(if committed {
                format!("publish the roster rows through the normal gate ({rel} is committed here)")
            } else {
                format!("commit {rel}, then publish the roster rows through the normal gate")
            }),
        ),
        Publication::NeverFetched(refs) => {
            Check::new(
                ROSTER_CHECK,
                State::Unknown,
                format!(
                "{described}; {} has never been fetched here, so whether {handle}'s roster row \
                 for this device is published is unknown",
                refs.iter().map(|r| short_ref(r)).collect::<Vec<_>>().join(", ")
            ),
                Some("fetch the integration branch".into()),
            )
        }
        Publication::Undeclared => Check::new(
            ROSTER_CHECK,
            State::Unknown,
            format!(
                "{described}; the device manifest declares no published remote-tracking ref \
                 (the epr tool's source.publishedRef), so publication is unknown"
            ),
            Some(
                "declare the integration branch's remote-tracking ref in the epr tool's \
                  source.publishedRef"
                    .into(),
            ),
        ),
        Publication::Unreadable(why) => Check::new(
            ROSTER_CHECK,
            State::Unknown,
            format!("{described}; git could not read the published branch: {why}"),
            Some("cure the repository trusted line first".into()),
        ),
    }
}

const ROSTER_CHECK: &str = "roster bound";
const SIBLING_CHECK: &str = "sibling exchange";

/// Whether a sibling device of the same person has exchanged heads with this one, read from the
/// head table the peer daemon saved — never a dial, never a daemon start. A roster that names a
/// sibling's endpoint is not evidence of exchange; only a folded head and its acknowledgement are.
fn sibling_check(root: &Path, current: &actor::DeviceState) -> Check {
    let actor::DeviceState::Standing(standing) = current else {
        return Check::new(
            SIBLING_CHECK,
            State::Degraded,
            "this device stands on no roster yet, so it has no sibling to exchange with",
            None,
        );
    };
    let siblings = actor::roster_siblings(root, &standing.handle, &standing.device);
    exchange_check(&siblings, &standing.device, saved_heads())
}

/// The peer daemon's saved head table: `Ok(None)` when none has been saved.
#[cfg(feature = "peer")]
fn saved_heads() -> Result<Option<eprfs_peer::HeadTable>, String> {
    let dir = crate::peer::peer_dir().map_err(|e| e.to_string())?;
    if !dir.join(eprfs_peer::HeadTable::FILE_NAME).is_file() {
        return Ok(None);
    }
    eprfs_peer::HeadTable::load(&dir)
        .map(Some)
        .map_err(|e| e.to_string())
}
#[cfg(not(feature = "peer"))]
fn saved_heads() -> Result<Option<()>, String> {
    Err("this epr was built without the peer feature, so it cannot read the peer's status".into())
}

#[cfg(not(feature = "peer"))]
fn exchange_check(siblings: &[String], _did: &str, heads: Result<Option<()>, String>) -> Check {
    if siblings.is_empty() {
        return no_sibling();
    }
    Check::new(
        SIBLING_CHECK,
        State::Unknown,
        heads.err().unwrap_or_default(),
        None,
    )
}

fn no_sibling() -> Check {
    Check::new(
        SIBLING_CHECK,
        State::Degraded,
        "this device has no sibling in the roster",
        None,
    )
}

/// The exchange as `heads` records it, for this device `did` and its roster `siblings`.
#[cfg(feature = "peer")]
fn exchange_check(
    siblings: &[String],
    did: &str,
    heads: Result<Option<eprfs_peer::HeadTable>, String>,
) -> Check {
    if siblings.is_empty() {
        return no_sibling();
    }
    let table = match heads {
        Ok(Some(table)) => table,
        Ok(None) => {
            return Check::new(
                SIBLING_CHECK,
                State::Unknown,
                "no peer status has been saved on this device, so nothing its siblings sent \
                 has been read",
                Some("start the peer on demand: epr peer serve".into()),
            )
        }
        Err(why) => {
            return Check::new(
                SIBLING_CHECK,
                State::Unknown,
                why,
                Some("restore access to the epr peer directory".into()),
            )
        }
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    let ago = |at: u64| now.saturating_sub(at) / 1000;
    let heads: Vec<(&String, &eprfs_peer::DeviceHead)> = siblings
        .iter()
        .filter_map(|sibling| table.devices.get(sibling).map(|head| (sibling, head)))
        .collect();
    let acking = heads
        .iter()
        .filter_map(|(sibling, head)| head.folded.get(did).map(|seq| (sibling, head, seq)))
        .max_by_key(|(_, head, _)| head.folded_at);
    if let Some((sibling, head, seq)) = acking {
        return Check::new(
            SIBLING_CHECK,
            State::Ok,
            format!(
                "{} acknowledges this device's head to seq {seq}; its own head (seq {}) was \
                 folded here {}s ago",
                actor::short_did(sibling),
                head.seq,
                ago(head.folded_at)
            ),
            None,
        );
    }
    let named = siblings
        .iter()
        .map(|s| actor::short_did(s))
        .collect::<Vec<_>>()
        .join(", ");
    match heads.iter().max_by_key(|(_, head)| head.folded_at) {
        None => Check::new(
            SIBLING_CHECK,
            State::Degraded,
            format!(
                "the roster lists {named}, but no verified head from a sibling has been folded \
                 here: nothing has been exchanged"
            ),
            Some(format!(
                "keep epr peer serve running here and on {named}; epr peer status shows each \
                 head as it folds"
            )),
        ),
        Some((sibling, head)) => Check::new(
            SIBLING_CHECK,
            State::Degraded,
            format!(
                "{} has a verified head here (seq {}, folded {}s ago), but no sibling \
                 acknowledges this device's head",
                actor::short_did(sibling),
                head.seq,
                ago(head.folded_at)
            ),
            Some(format!(
                "keep epr peer serve running here until {} folds this device's head; epr peer \
                 status shows its acknowledgement",
                actor::short_did(sibling)
            )),
        ),
    }
}

/// The device layout as a command run from `root` resolves it (the real environment and
/// filesystem; the checkout discovered from `root`). A read: nothing is created.
fn resolve_layout(root: &Path) -> Result<DeviceLayout, Refusal> {
    let env = |name: &str| std::env::var_os(name);
    let exists = |path: &Path| path.exists();
    layout::resolve_with(&layout::Host {
        env: &env,
        exists: &exists,
        cwd: Some(root.to_owned()),
    })
}

/// How a report names the basis a native session would open on.
fn describe_basis(basis: &crate::session_envelope::PersistenceBasis) -> String {
    match basis {
        crate::session_envelope::PersistenceBasis::ExplicitMount => basis.name().to_owned(),
        crate::session_envelope::PersistenceBasis::DeclaredDurableHome { declared_at } => {
            format!("{}, declared {declared_at}", basis.name())
        }
    }
}

const LAYOUT_CHECK: &str = "device layout";
/// The report label of each field, in the order the layout line names them.
const LAYOUT_LABELS: [(Field, &str); 5] = [
    (Field::ConfigDir, "config"),
    (Field::BinStore, "bin-store"),
    (Field::StateDir, "state"),
    (Field::CargoPool, "pool"),
    (Field::ProjectRoot, "checkout"),
];

/// Where this device keeps its things, each with the reason it was chosen. An override that
/// disagrees with the saved layout degrades (gates set them on purpose); a startup-critical
/// destination this user cannot create refuses, because every launch would then fail.
fn layout_check(resolved: Result<DeviceLayout, Refusal>) -> Check {
    let device_layout =
        match resolved {
            Ok(device_layout) => device_layout,
            Err(error) => return Check::new(
                LAYOUT_CHECK,
                State::Refused,
                format!("{}: {}", error.kind, error.detail),
                Some(
                    "correct the named variable or the saved layout.json, then epr device layout \
                     show"
                        .into(),
                ),
            ),
        };
    let places = LAYOUT_LABELS
        .iter()
        .map(|(field, label)| format!("{label} {}", device_layout.get(*field).describe()))
        .collect::<Vec<_>>()
        .join(" · ");
    let blocked: Vec<(Field, String)> = [
        (Field::BinStore, device_layout.bin_store()),
        (Field::StateDir, device_layout.state_dir()),
    ]
    .into_iter()
    .filter_map(|(field, path)| {
        path.and_then(|path| layout::check_creatable(field.name(), path))
            .err()
            .map(|error| (field, format!("{}: {}", error.kind, error.detail)))
    })
    .collect();
    if !blocked.is_empty() {
        let variables: Vec<&str> = blocked.iter().map(|(field, _)| field.env()).collect();
        let fields: Vec<String> = blocked
            .iter()
            .map(|(field, _)| format!("--field {}", field.name()))
            .collect();
        let why: Vec<&str> = blocked.iter().map(|(_, why)| why.as_str()).collect();
        return Check::new(
            LAYOUT_CHECK,
            State::Refused,
            format!("{places}; {}", why.join("; ")),
            Some(format!(
                "set {} to a directory this user can create and write, then epr device layout \
                 save {}",
                variables.join(" and "),
                fields.join(" ")
            )),
        );
    }
    if device_layout.differs.is_empty() {
        return Check::new(LAYOUT_CHECK, State::Ok, places, None);
    }
    let differs: Vec<String> = device_layout.differs.iter().map(|d| d.line()).collect();
    Check::new(
        LAYOUT_CHECK,
        State::Degraded,
        format!("{places}; {}", differs.join("; ")),
        Some(
            "epr device layout save to remember the variable's place, or unset the variable to \
             use the saved one"
                .into(),
        ),
    )
}

/// The toolchain `pool-policy.json` pins for every cargo gate (`cargo_env_overrides["*"]`).
fn gate_toolchain(root: &Path) -> Option<String> {
    fs::read(root.join("genesis/agentic/pool-policy.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|policy| {
            policy["cargo_env_overrides"]["*"]["RUSTUP_TOOLCHAIN"]
                .as_str()
                .map(str::to_owned)
        })
}

#[path = "arrival_sysroot.rs"]
mod sysroot;

/// The Rust toolchain the local gate pins (`pool-policy.json` `cargo_env_overrides["*"]`),
/// looked up in rustup's home. `$HOME` is wiped on a workspace restart, and a rustup home under
/// it loses the pin; the gate then refuses every cargo project. A read: never installs.
fn toolchain_check(root: &Path, env: &dyn Fn(&str) -> Option<String>) -> Check {
    let Some(pin) = gate_toolchain(root) else {
        return Check::new(
            "rust toolchain",
            State::Ok,
            "this checkout pins no gate toolchain",
            None,
        );
    };
    let home = env("RUSTUP_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".rustup")));
    let Some(home) = home else {
        return Check::new(
            "rust toolchain",
            State::Unknown,
            format!("pin {pin}; neither RUSTUP_HOME nor HOME is set"),
            Some("set RUSTUP_HOME to rustup's home".into()),
        );
    };
    let toolchains = home.join("toolchains");
    let entries = match fs::read_dir(&toolchains) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return toolchain_missing(&pin, &toolchains)
        }
        Err(e) => {
            return Check::new(
                "rust toolchain",
                State::Unknown,
                format!("pin {pin}; {}: {e}", toolchains.display()),
                None,
            )
        }
    };
    let prefix = format!("{pin}-");
    let installed = entries.filter_map(Result::ok).find(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        (*name == *pin || name.starts_with(&prefix)) && entry.path().join("bin/cargo").is_file()
    });
    match installed {
        Some(entry) => Check::new(
            "rust toolchain",
            State::Ok,
            format!("pin {pin} at {}", entry.path().display()),
            None,
        ),
        None => toolchain_missing(&pin, &toolchains),
    }
}
fn toolchain_missing(pin: &str, toolchains: &Path) -> Check {
    Check::new(
        "rust toolchain",
        State::Degraded,
        format!(
            "gate pin {pin} is not installed under {}; every cargo gate refuses",
            toolchains.display()
        ),
        Some(format!(
            "rustup toolchain install {pin} --profile minimal --component clippy,rustfmt"
        )),
    )
}

/// Hooks `.husky/hooks-path.mjs` requires in `core.hooksPath` (its `GOVERNED_HOOKS` plus the
/// husky runner `h`). Keep the two lists equal.
const GOVERNED_HOOKS: [&str; 6] = [
    "pre-push",
    "pre-commit",
    "pre-merge-commit",
    "prepare-commit-msg",
    "commit-msg",
    "h",
];

/// Whether this checkout's governed hooks run: `core.hooksPath` is absolute (a relative one is
/// resolved against each worktree, where no shims exist) and holds every governed hook. Without
/// them a push runs no gate, so this refuses. A checkout with no `.husky/hooks-path.mjs`
/// declares no governed hooks.
fn hooks_check(root: &Path) -> Check {
    let cure = Some("node .husky/hooks-path.mjs (after pnpm install)".to_owned());
    if !root.join(".husky/hooks-path.mjs").is_file() {
        return Check::new(
            "git hooks",
            State::Ok,
            "this checkout declares no governed hooks",
            None,
        );
    }
    let output = git::command(root)
        .args(["config", "--get", "core.hooksPath"])
        .output();
    let value = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_owned(),
        // `git config --get` exits 1 when the key is unset, 128 when git refuses the repository.
        Ok(out) if out.status.code() == Some(1) => String::new(),
        Ok(out) => {
            return Check::new(
                "git hooks",
                State::Unknown,
                format!(
                    "git could not read core.hooksPath: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
                Some("cure the repository trusted line first".into()),
            )
        }
        Err(e) => {
            return Check::new("git hooks", State::Unknown, format!("git: {e}"), None);
        }
    };
    if value.is_empty() {
        return Check::new(
            "git hooks",
            State::Refused,
            "core.hooksPath is unset; pushes run no pre-push gate",
            cure,
        );
    }
    let dir = PathBuf::from(&value);
    if !dir.is_absolute() {
        return Check::new(
            "git hooks",
            State::Refused,
            format!("core.hooksPath {value} is relative; a worktree push runs no pre-push gate"),
            cure,
        );
    }
    let missing: Vec<_> = GOVERNED_HOOKS
        .iter()
        .filter(|hook| !crate::doctor::is_executable(&dir.join(hook)))
        .copied()
        .collect();
    if missing.is_empty() {
        Check::new(
            "git hooks",
            State::Ok,
            format!("core.hooksPath {value}"),
            None,
        )
    } else {
        Check::new(
            "git hooks",
            State::Refused,
            format!(
                "core.hooksPath {value} lacks executable {}",
                missing.join(", ")
            ),
            cure,
        )
    }
}

/// This checkout's own git directory: `.git` itself, or the `gitdir:` its `.git` file names.
fn private_git_dir(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let text = fs::read_to_string(&dot_git).ok()?;
    let gitdir = PathBuf::from(
        text.lines()
            .find_map(|l| l.strip_prefix("gitdir: "))?
            .trim(),
    );
    Some(if gitdir.is_absolute() {
        gitdir
    } else {
        root.join(gitdir)
    })
}

/// The repository's shared object directory: `.git` itself, or for a linked worktree the
/// `commondir` its git directory names.
fn common_git_dir(root: &Path) -> Option<PathBuf> {
    let gitdir = private_git_dir(root)?;
    match fs::read_to_string(gitdir.join("commondir")) {
        Ok(common) => {
            let common = PathBuf::from(common.trim());
            Some(if common.is_absolute() {
                common
            } else {
                gitdir.join(common)
            })
        }
        Err(_) => Some(gitdir),
    }
}

/// gix's `check_safe_directories` over values in effective order (system, then global): `*`
/// grants, an empty value resets, a trailing `/*` grants by prefix, otherwise an exact realpath
/// match. Relative entries are ignored, as gix ignores them.
fn safe_directory_grants(test_dir: &Path, values: &[String]) -> bool {
    let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_owned());
    let test_dir = real(test_dir);
    let mut safe = false;
    for value in values {
        let value = value.trim();
        if value == "*" {
            safe = true;
            continue;
        }
        if value.is_empty() {
            safe = false;
            continue;
        }
        if safe {
            continue;
        }
        let dir = Path::new(value);
        if !dir.is_absolute() {
            continue;
        }
        if let Some(prefix) = value.strip_suffix("/*") {
            safe = test_dir.strip_prefix(real(Path::new(prefix))).is_ok();
        } else {
            safe = real(dir) == test_dir;
        }
    }
    safe
}

/// Whether brit (gitoxide) opens this repository at full trust, by gix's own rule
/// (`gix/src/open/repository.rs`): the git directory and the worktree are both owned by this
/// user, or the effective global/system `safe.directory` list grants the worktree. This device's
/// own declaration (`epr device trust`) reaches only git, never brit. At reduced trust brit
/// ignores `core.hooksPath` (its commits run no hooks) and caps object index slots at 32.
fn brit_trust(root: &Path) -> Option<(bool, String)> {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: geteuid takes no arguments, touches no memory and cannot fail (POSIX).
    let euid = unsafe { libc::geteuid() };
    let gitdir = private_git_dir(root)?;
    let foreign: Vec<_> = [gitdir.as_path(), root]
        .into_iter()
        .filter_map(|path| {
            let uid = fs::metadata(path).ok()?.uid();
            (uid != euid).then(|| format!("{} (uid {uid})", path.display()))
        })
        .collect();
    if foreign.is_empty() && fs::metadata(&gitdir).is_ok() && fs::metadata(root).is_ok() {
        return Some((
            true,
            "git directory and worktree are owned by this user".into(),
        ));
    }
    let mut values = Vec::new();
    for scope in ["--system", "--global"] {
        if let Ok(out) = git::native_command(root)
            .args(["config", scope, "--get-all", "safe.directory"])
            .output()
        {
            values.extend(
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(str::to_owned),
            );
        }
    }
    if safe_directory_grants(root, &values) {
        return Some((
            true,
            "the effective safe.directory list grants this worktree".into(),
        ));
    }
    Some((
        false,
        format!(
            "not owned by this user ({}) and the effective global/system safe.directory list does not grant it",
            foreign.join(", ")
        ),
    ))
}
fn brit_trust_check(root: &Path) -> Check {
    match brit_trust(root) {
        Some((true, detail)) => Check::new("brit trust", State::Ok, detail, None),
        Some((false, detail)) => Check::new(
            "brit trust",
            State::Degraded,
            format!("{detail}; brit runs at reduced trust (no hooks, 32 index slots)"),
            Some("git config --global --add safe.directory '*' (devfile setup-git)".into()),
        ),
        None => Check::new(
            "brit trust",
            State::Unknown,
            "the repository's git directory could not be read",
            None,
        ),
    }
}

/// gix's object-index slot cap at reduced trust (`gix/src/open/options.rs`,
/// `resolve_object_store_slots`). Full trust sizes slots from disk.
const REDUCED_TRUST_SLOTS: usize = 32;

/// Pack count against the reduced-trust slot cap. Over it, every brit read fails ("too few index
/// slots") whenever trust falls to reduced, as it does on each restart before setup-git runs.
fn packs_check(root: &Path) -> Check {
    let Some(common) = common_git_dir(root) else {
        return Check::new(
            "object packs",
            State::Unknown,
            "the repository's git directory could not be read",
            None,
        );
    };
    let packs = match fs::read_dir(common.join("objects/pack")) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "idx"))
            .count(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => {
            return Check::new("object packs", State::Unknown, e.to_string(), None);
        }
    };
    let trust = match brit_trust(root) {
        Some((true, _)) => "full trust now",
        Some((false, _)) => "reduced trust now",
        None => "trust unknown",
    };
    let detail =
        format!("{packs} pack index(es); reduced-trust cap {REDUCED_TRUST_SLOTS} ({trust})");
    if packs < REDUCED_TRUST_SLOTS {
        Check::new("object packs", State::Ok, detail, None)
    } else {
        Check::new(
            "object packs",
            State::Degraded,
            format!("{detail}; brit reads fail at reduced trust"),
            Some("git repack -a -d".into()),
        )
    }
}

/// Whether ark runs the device's declared manifest at its head with every child ready.
/// Absence degrades and never refuses: local work goes on without a supervisor, losing only
/// restart and witnessed deaths (local-first). Like every arrival line, it is a read.
fn supervisor_check(root: &Path) -> Check {
    let (ok, detail, cure) = super::supervise::check(root);
    Check::new(
        "device supervisor",
        if ok { State::Ok } else { State::Degraded },
        detail,
        cure,
    )
}

/// Whether this device's `epr peer serve` holds its serve lock, probed without creating the
/// file. A flock is released by the kernel however its holder ends, container teardown
/// included, so unlike a recorded pid it cannot read a previous container's daemon as live.
#[cfg(feature = "peer")]
fn serve_liveness() -> std::result::Result<(bool, Option<String>), String> {
    let (path, held) = serve_lock_probe()?;
    let dir = path.parent().unwrap_or(&path).to_owned();
    if !held && !path.exists() {
        return Ok((false, None));
    }
    // Evidence, not state: a head is refolded only when its sibling's root moves, so an idle
    // sibling ages here while every daemon is healthy.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    let newest = eprfs_peer::HeadTable::load(&dir)
        .ok()
        .and_then(|table| table.devices.values().map(|h| h.folded_at).max())
        .map(|at| {
            format!(
                "newest sibling head folded {}s ago",
                now.saturating_sub(at) / 1000
            )
        });
    Ok((held, newest))
}
#[cfg(not(feature = "peer"))]
fn serve_liveness() -> std::result::Result<(bool, Option<String>), String> {
    Err("this epr was built without the peer feature".into())
}

/// `<peer dir>/serve.lock` and whether a process holds it now, by a momentary shared flock
/// that never creates the file (absent reads as free). `epr device supervise` asks the same
/// question before it starts a declared `epr peer serve`.
#[cfg(feature = "peer")]
pub(super) fn serve_lock_probe() -> std::result::Result<(std::path::PathBuf, bool), String> {
    use rustix::fs::{flock, FlockOperation};
    let dir = crate::peer::peer_dir().map_err(|e| e.to_string())?;
    let path = dir.join(crate::peer::ServeLock::FILE_NAME);
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((path, false)),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let held = match flock(&file, FlockOperation::NonBlockingLockShared) {
        // Shared and dropped at once; `ServeLock::acquire` retries a contended exclusive take,
        // so this momentary probe does not read as a live daemon to a starting `serve`.
        Ok(()) => false,
        Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => true,
        Err(errno) => return Err(format!("{}: {errno}", path.display())),
    };
    drop(file);
    Ok((path, held))
}
#[cfg(not(feature = "peer"))]
pub(super) fn serve_lock_probe() -> std::result::Result<(std::path::PathBuf, bool), String> {
    Err("this epr was built without the peer feature, so it cannot find the serve lock".into())
}

/// The cure for a silent peer daemon: the supervisor when the head declares `epr peer
/// serve`, else the devfile command that starts one by hand.
fn peer_daemon_cure() -> String {
    let declared = super::declaration::store(None, None, None)
        .and_then(|s| s.read().map_err(|e| e.to_string()))
        .ok()
        .and_then(|head| super::supervise::declared_peer_serve(&head).map(|p| p.name.clone()));
    match declared {
        Some(name) => format!("epr device supervise (the device head declares `{name}`)"),
        // A workspace container starts its peer from the devfile, which carries the relay
        // settings. A person's own machine has no devfile: it starts the peer when it wants one.
        None if crate::session_envelope::on_ephemeral_home_platform() => {
            "start the devfile peer-serve command (it carries EPR_PEER_RELAYS)".into()
        }
        None => "start the peer on demand: epr peer serve".into(),
    }
}

fn peer_daemon_check(root: &Path) -> Check {
    // A release held for failing readiness is the first thing this line says, serving or not:
    // the device runs something other than what it landed, or nothing (row 35).
    let reading = super::supervise::restart_reading(root);
    if let Some(super::supervise::Reading::Held { detail, cure }) = reading {
        return Check::new("peer daemon", State::Degraded, detail, Some(cure));
    }
    let restarted = match reading {
        Some(super::supervise::Reading::Restarted(suffix)) => Some(suffix),
        _ => None,
    };
    match serve_liveness() {
        Ok((false, evidence)) => Check::new(
            "peer daemon",
            State::Degraded,
            format!(
                "no epr peer serve holds this device's serve lock; admitted siblings read it as silent{}",
                evidence.map(|e| format!(" ({e})")).unwrap_or_default()
            ),
            Some(peer_daemon_cure()),
        ),
        Ok((true, evidence)) => peer_daemon_version_check(evidence, restarted),
        Err(error) => Check::new(
            "peer daemon",
            State::Unknown,
            error,
            Some("restore access to the epr peer directory".into()),
        ),
    }
}

/// `restarted` is the newest restart's receipt suffix, said only on a line serving at current.
fn peer_daemon_version_check(evidence: Option<String>, restarted: Option<String>) -> Check {
    let evidence = evidence.map(|e| format!("; {e}")).unwrap_or_default();
    let observed = (|| {
        let store = crate::bin::daemon_store()?;
        let leases = store.live_daemons(&crate::bin::daemon_proc_root())?;
        let head = store.resolve_current(None).ok().and_then(|r| r.cid);
        Ok::<_, elohim_device_runtime::bin_store::Refusal>((leases, head))
    })();
    match observed {
        Ok((leases, head)) => {
            let stale: Vec<_> = leases
                .into_iter()
                .map(|pin| pin.running_cid.unwrap_or(pin.cid))
                .filter(|cid| Some(cid) != head.as_ref())
                .collect();
            if stale.is_empty() {
                Check::new(
                    "peer daemon",
                    State::Ok,
                    format!(
                        "serving at current{}{evidence}",
                        restarted.unwrap_or_default()
                    ),
                    None,
                )
            } else {
                Check::new(
                    "peer daemon",
                    State::Degraded,
                    format!(
                        "running {}; current {}{evidence}",
                        stale.join(", "),
                        head.as_deref().unwrap_or("absent")
                    ),
                    Some("restart epr peer serve to follow current".into()),
                )
            }
        }
        Err(error) => Check::new(
            "peer daemon",
            State::Unknown,
            error.to_string(),
            Some("restore access to the epr bin store".into()),
        ),
    }
}

fn tool_check(
    tool: &str,
    store: &Store,
    identity: &session::SessionIdentity,
    env: &dyn Fn(&str) -> Option<String>,
) -> Check {
    let name = format!("{tool} binary");
    let cure = Some(format!("epr bin bootstrap --tool {tool}"));
    let shadows = store.shadows(&env("PATH").unwrap_or_default());
    if !shadows.is_empty() {
        let paths: Vec<_> = shadows.iter().filter_map(|s| s["path"].as_str()).collect();
        return Check::new(
            &name,
            State::Refused,
            format!("PATH reaches {} before the launcher", paths.join(", ")),
            Some("epr device arrive; remove any remaining named PATH shadow".into()),
        );
    }
    match store.status(identity, None, false, env) {
        Ok(view)
            if view
                .resolved
                .as_ref()
                .is_some_and(|p| crate::doctor::is_executable(p)) =>
        {
            let running = if tool == "epr" {
                std::env::current_exe()
                    .ok()
                    .and_then(|path| store.executable_cid(&path).ok())
            } else {
                view.resolved_cid.clone()
            };
            let detail = format!(
                "{} vs head {} ({}{})",
                running.as_deref().unwrap_or("candidate"),
                view.head.as_deref().unwrap_or("none"),
                view.source,
                if view.behind { ", pinned behind" } else { "" }
            );
            let declared_run = running == view.resolved_cid
                && matches!(
                    view.source.as_str(),
                    "candidate" | "explicit-pin" | "session-pin"
                );
            let behind_unpinned = running != view.head && !declared_run;
            if !view.verified_current || behind_unpinned {
                Check::new(
                    &name,
                    State::Degraded,
                    detail,
                    Some(format!(
                        "epr device arrive (or declare {tool}'s candidate/pin)"
                    )),
                )
            } else {
                Check::new(&name, State::Ok, detail, None)
            }
        }
        Ok(_) => Check::new(
            &name,
            absent_state(tool),
            "no executable candidate, pin, or committed head",
            cure,
        ),
        Err(error) => Check::new(&name, absent_state(tool), error.to_string(), cure),
    }
}

/// A device cannot work without its own `epr`; every other followed tool is an enhancement
/// (brit's absence costs the gate its `rakia affected` oracle, which falls back to path
/// selection), so its absence degrades instead of refusing (local-first, ground rule 2).
fn absent_state(tool: &str) -> State {
    if tool == "epr" {
        State::Refused
    } else {
        State::Degraded
    }
}
/// The key, and the basis a native session would open on beside it (`persistence`: the basis
/// named, or the persistence refusal as `<code>: <cure>`). A bound key whose home is not known
/// to survive a restart degrades: the device can act, but cannot keep a session.
fn key_check(path: &Path, handle: &str, persistence: &Result<String, String>) -> Check {
    match device_key::DeviceKey::load(path) {
        Ok(_) => match persistence {
            Ok(basis) => Check::new(
                "device key",
                State::Ok,
                format!("{} (persistence {basis})", path.display()),
                None,
            ),
            Err(refusal) => {
                let (code, cure) = refusal.split_once(": ").unwrap_or(("", refusal.as_str()));
                Check::new(
                    "device key",
                    State::Degraded,
                    format!(
                        "{}; native sessions cannot open here ({})",
                        path.display(),
                        if code.is_empty() {
                            "persistence refused"
                        } else {
                            code
                        }
                    ),
                    Some(cure.to_owned()),
                )
            }
        },
        Err(error) => Check::new(
            "device key",
            State::Refused,
            format!("{}: {error}", path.display()),
            Some(format!("epr actor device enroll --handle {handle}")),
        ),
    }
}
fn sole_handle(root: &Path) -> Option<String> {
    let mut handles: Vec<_> = fs::read_dir(root.join(actor::PARTICIPANTS_REL))
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension()?.to_str()? == "jsonl")
                .then(|| path.file_stem()?.to_str().map(str::to_owned))
                .flatten()
        })
        .filter(|name| !name.starts_with('.'))
        .collect();
    (handles.len() == 1).then(|| handles.remove(0))
}
/// Whether the person's roster lists this device, and once it does, whether the shared branch
/// carried that row at the last fetch: a binding only this device has seen is not yet one any
/// sibling can recognise. Reads the already-fetched refs; never fetches.
fn roster_check(
    root: &Path,
    current: &actor::DeviceState,
    handle: &str,
    published: &[String],
) -> Check {
    if let actor::DeviceState::Standing(standing) = current {
        if !actor::device_enrolled(root, &standing.handle, &standing.device) {
            // Standing read from this checkout's records with no row of its own: nothing to publish.
            return Check::new(ROSTER_CHECK, State::Ok, standing.describe(), None);
        }
        let found = publication(root, &standing.handle, &standing.device, published);
        return publication_check(standing, found);
    }
    let pending = root
        .join(actor::PARTICIPANTS_REL)
        .join(format!(".enroll-{handle}.json"))
        .is_file();
    let first = !root
        .join(actor::PARTICIPANTS_REL)
        .join(format!("{handle}.jsonl"))
        .is_file();
    let cure = if first {
        // A human claim alone does not create a roster; the present agent witnesses the first device.
        format!("first device here: epr actor claim --as human:{handle} --session <session>; then epr actor witness --subject human:{handle} --as agent:<role>@<model> --session <session> --basis '<human present>'; commit {}/{handle}.jsonl", actor::PARTICIPANTS_REL)
    } else {
        format!("epr actor device enroll --handle {handle} here; on a device already in {handle}'s roster run epr actor device authorize '<request>'; then here epr actor device bind '<authorization>' and commit {}/{handle}.jsonl", actor::PARTICIPANTS_REL)
    };
    Check::new(
        ROSTER_CHECK,
        State::Refused,
        if matches!(current, actor::DeviceState::RootContested(_)) {
            "roster root contested; review the pinned root before enrollment"
        } else if pending {
            "enrollment pending"
        } else {
            "this device has no roster standing"
        },
        Some(cure),
    )
}
fn model_check(root: &Path) -> Check {
    let manifest = embedder::ModelManifest::load(&root.join(embedder::MODEL_MANIFEST_REL));
    let detail = match manifest {
        Ok(manifest) => match manifest.resolve_model_dir() {
            Some(path) if path.join("model.onnx").is_file() && path.join("tokenizer.json").is_file() => return Check::new("embed model", State::Ok, path.display().to_string(), None),
            _ => "no model.onnx + tokenizer.json at the model's resolved directory; semantic recall degrades".into(),
        },
        Err(error) => format!("model manifest unavailable: {error}; semantic recall degrades"),
    };
    Check::new(
        "embed model",
        State::Degraded,
        detail,
        Some("genesis/agentic/bin/embed-model-provision".into()),
    )
}
fn lag_limit(root: &Path) -> u64 {
    fs::read(root.join(".epr-meta/elohim/algorithms/recall-semantic-index.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value["foldLag"]["limit"].as_u64())
        .unwrap_or(25)
}
fn fold_check(value: crate::flow::FlowResult<Value>, limit: u64) -> Check {
    let cure = "epr flow memory index fold --max-files 1";
    let value = match value {
        Ok(value) => value,
        Err(error) => {
            return Check::new(
                "fold attested",
                State::Degraded,
                error.to_string(),
                Some(cure.into()),
            )
        }
    };
    let state = value["last"]["state"]["state"]
        .as_str()
        .or_else(|| value["last"]["state"].as_str())
        .unwrap_or("unfolded");
    if state == "failed" {
        return Check::new(
            "fold attested",
            State::Degraded,
            format!(
                "last fold failed: {}",
                value["last"]["state"]["why"]
                    .as_str()
                    .unwrap_or("unknown reason")
            ),
            Some(format!("cure embed model, then {cure}")),
        );
    }
    match value["lag"].as_u64() {
        Some(lag) => Check::new(
            "fold attested",
            if state == "complete" && lag <= limit {
                State::Ok
            } else {
                State::Degraded
            },
            format!("{state}, {lag} files behind (limit {limit})"),
            if state == "complete" && lag <= limit {
                None
            } else {
                Some(format!(
                    "epr flow memory index fold --max-files {}",
                    if lag > limit {
                        lag.saturating_add(50).max(100)
                    } else {
                        1
                    }
                ))
            },
        ),
        None => Check::new(
            "fold attested",
            State::Degraded,
            "no attested fold lag; lexical recall remains available",
            Some(cure.into()),
        ),
    }
}
/// The berth's moorings, found as `genesis/agentic/bin/berth` finds its store: `BERTH_DIR`,
/// else `$CLAUDE_CONFIG_DIR/berth`, else `<the layout's state directory>/berth`.
fn berth_dir(
    env: &dyn Fn(&str) -> Option<String>,
    device_layout: Option<&DeviceLayout>,
) -> Result<PathBuf, String> {
    let set = |name: &str| env(name).filter(|value| !value.is_empty());
    let store = match (set("BERTH_DIR"), set("CLAUDE_CONFIG_DIR")) {
        (Some(store), _) => PathBuf::from(store),
        (None, Some(config)) => PathBuf::from(config).join("berth"),
        (None, None) => device_layout
            .ok_or("the device layout did not resolve")
            .and_then(|l| {
                l.state_dir()
                    .map_err(|_| "this device names no state directory")
            })
            .map_err(|why| format!("no berth store: {why}"))?
            .join("berth"),
    };
    Ok(store.join("moorings"))
}
fn berth_check(dir: &Path, session: Option<&str>) -> Check {
    let records: Vec<Value> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| fs::read(e.path()).ok())
        .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
        .collect();
    let found = records.iter().any(|value| {
        value["session"]
            .as_str()
            .is_some_and(|s| session.is_none_or(|expected| s == expected))
    });
    if found {
        Check::new(
            "berth moored",
            State::Ok,
            session.map_or_else(
                || format!("{} mooring(s), no session named", records.len()),
                |s| format!("session {s} moored"),
            ),
            None,
        )
    } else {
        Check::new(
            "berth moored",
            State::Degraded,
            "no mooring for this session; coordinated capacity unavailable",
            Some(format!(
                "berth moor --session {} --runtime {}",
                session.unwrap_or("<id>"),
                session::resolve().runtime
            )),
        )
    }
}
pub(crate) fn repository_check(root: &Path) -> Check {
    match git::accepted(root) {
        Ok(true) => Check::new(
            "repository trusted",
            State::Ok,
            format!(
                "git accepts {} natively or by this device's declaration",
                root.display()
            ),
            None,
        ),
        result => Check::new(
            "repository trusted",
            State::Refused,
            match result {
                Err(e) => e.to_string(),
                _ => format!(
                    "git does not accept {}{}",
                    root.display(),
                    git::trust_unapplied(root)
                        .map(|why| format!("; no declared trust applied: {why}"))
                        .unwrap_or_default()
                ),
            },
            Some(format!(
                "epr device trust '{}'",
                root.display().to_string().replace('\'', "'\\''")
            )),
        ),
    }
}

pub(super) fn run(args: &[String], arrive: bool) -> Result<ExitCode, crate::error::Error> {
    let mut root = None;
    let mut json = false;
    let mut strict = false;
    let mut worker = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => {
                root = Some(PathBuf::from(iter.next().ok_or_else(|| {
                    crate::error::Error::InvalidArguments("--root needs a path".into())
                })?))
            }
            "--json" => json = true,
            "--strict" if arrive => strict = true,
            "--worker" if arrive => worker = true,
            _ => {
                return Err(crate::error::Error::InvalidArguments(format!(
                    "unknown device arrival flag {arg}"
                )))
            }
        }
    }
    let here = std::env::current_dir().map_err(|source| crate::error::Error::Read {
        path: PathBuf::from("."),
        source,
    })?;
    let root = root.unwrap_or_else(|| git::checkout(&here).unwrap_or(here));
    if arrive {
        let result = if worker {
            super::arrival_work::work(&root)
        } else {
            super::arrival_work::start(&root)
        };
        if let Err(error) = result {
            eprintln!("device arrival (continuing): {error}");
        }
        if worker {
            return Ok(ExitCode::SUCCESS);
        }
    }
    let checks = status_at(&root, arrive);
    if json {
        println!("{}", serde_json::to_string(&checks)?);
    } else {
        for check in &checks {
            println!("{check}");
        }
    }
    Ok(
        if (!arrive || strict) && checks.iter().any(|c| c.state == State::Refused) {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    // Parity: device_preflight_test.py -> native checks below:
    // seven_ok/order -> CLI schema/order; missing epr/brit + status -> candidates_and_absence;
    // no_key -> missing_and_invalid_keys; unbound/pending/first device -> roster_cures;
    // model absence/expansion -> absent_model_degrades + embedder::tests::resolve_*;
    // failed/lag/missing fold -> fold_states; missing berth -> moorings_use_recorded_session;
    // no epr no longer prevents library reads (CLI keyless test); dependencies are native calls.
    // guard registry fail-open -> CLI arrive_bad_registry; bootstrap/follow ordering -> arrival_work tests.
    // hand-installed bytes/shadows -> device-runtime bin_store::tests::test_guard_* and missing_launcher_*.
    #[test]
    fn missing_and_invalid_keys_refuse_without_minting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key/seed");
        let mount = Ok("explicit-mount".to_owned());
        let check = key_check(&path, "matthew", &mount);
        assert_eq!(check.state, State::Refused);
        assert!(check.cure.unwrap().contains("enroll --handle matthew"));
        assert!(!path.exists());
        fs::create_dir(path.parent().unwrap()).unwrap();
        fs::write(&path, b"bad").unwrap();
        assert_eq!(key_check(&path, "matthew", &mount).state, State::Refused);
        let valid = dir.path().join("valid/seed");
        device_key::DeviceKey::load_or_generate(&valid).unwrap();
        let ok = key_check(&valid, "matthew", &mount);
        assert_eq!(ok.state, State::Ok);
        assert!(ok.detail.ends_with("(persistence explicit-mount)"), "{ok}");
    }
    #[test]
    fn a_bound_key_whose_home_is_not_durable_degrades_naming_the_persistence_cure() {
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("device/ed25519.seed");
        device_key::DeviceKey::load_or_generate(&key).unwrap();
        let refused = Err(
            "envelope-persistent-config-required: set XDG_CONFIG_HOME to an \
                           absolute persistent config directory; or declare it once with `epr \
                           actor device persistence --declare durable-home`"
                .to_owned(),
        );
        let check = key_check(&key, "matthew", &refused);
        assert_eq!(check.state, State::Degraded);
        assert!(check
            .detail
            .contains("cannot open here (envelope-persistent-config-required)"));
        let cure = check.cure.unwrap();
        assert!(cure.starts_with("set XDG_CONFIG_HOME"), "{cure}");
        assert!(cure.contains("--declare durable-home"), "{cure}");
        let declared = Ok("declared-durable-home, declared 2026-11-16T00:00:00Z".to_owned());
        let ok = key_check(&key, "matthew", &declared);
        assert_eq!(ok.state, State::Ok);
        assert!(
            ok.detail.contains("persistence declared-durable-home"),
            "{ok}"
        );
    }
    #[test]
    fn the_layout_line_names_every_place_and_refuses_an_unwritable_destination() {
        let dir = tempfile::tempdir().unwrap();
        let resolve = |pairs: &[(&str, String)]| {
            let env = |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| std::ffi::OsString::from(v))
            };
            let exists = |path: &Path| path.exists() && !path.starts_with("/projects");
            layout::resolve_with(&layout::Host {
                env: &env,
                exists: &exists,
                cwd: None,
            })
        };
        let home = dir.path().join("home").display().to_string();
        let ok = layout_check(resolve(&[("HOME", home.clone())]));
        assert_eq!(ok.state, State::Ok, "{ok}");
        for label in [
            "config ",
            " · bin-store ",
            " · state ",
            " · pool ",
            " · checkout ",
        ] {
            assert!(ok.detail.contains(label), "{label} in {ok}");
        }
        assert!(ok
            .detail
            .contains(&format!("state {home}/.local/state/elohim (host default)")));
        let plain = dir.path().join("plain");
        fs::write(&plain, "").unwrap();
        let blocked = layout_check(resolve(&[
            ("HOME", home.clone()),
            (
                "ELOHIM_STATE_DIR",
                plain.join("state").display().to_string(),
            ),
        ]));
        assert_eq!(blocked.state, State::Refused, "{blocked}");
        assert!(blocked.detail.contains("layout-destination-unwritable"));
        let cure = blocked.cure.unwrap();
        assert!(cure.contains("set ELOHIM_STATE_DIR"), "{cure}");
        assert!(cure.contains("--field stateDir"), "{cure}");
        let none = layout_check(resolve(&[]));
        assert_eq!(
            none.state,
            State::Refused,
            "no home names no bin-store or state"
        );
        assert!(none
            .cure
            .unwrap()
            .contains("BIN_STORE and ELOHIM_STATE_DIR"));
    }
    #[test]
    fn a_layout_override_that_disagrees_with_the_saved_place_degrades() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("xdg/elohim/device");
        fs::create_dir_all(&config).unwrap();
        let saved = dir.path().join("saved-bins").display().to_string();
        fs::write(
            config.join(layout::SAVED_FILE),
            format!(r#"{{"version":1,"binStore":"{saved}"}}"#),
        )
        .unwrap();
        let xdg = dir.path().join("xdg").display().to_string();
        let named = dir.path().join("named-bins").display().to_string();
        let home = dir.path().join("home").display().to_string();
        let pairs = [
            ("XDG_CONFIG_HOME", xdg),
            ("HOME", home),
            ("BIN_STORE", named.clone()),
        ];
        let env = |name: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| std::ffi::OsString::from(v))
        };
        let exists = |path: &Path| path.exists() && !path.starts_with("/projects");
        let check = layout_check(layout::resolve_with(&layout::Host {
            env: &env,
            exists: &exists,
            cwd: None,
        }));
        assert_eq!(check.state, State::Degraded, "{check}");
        assert!(check.detail.contains(&format!(
            "layout-override-differs binStore env {named} saved {saved}"
        )));
        assert!(check.cure.unwrap().contains("epr device layout save"));
    }
    #[test]
    fn berth_moorings_are_found_where_berth_keeps_its_store() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_owned())
            }
        };
        assert_eq!(
            berth_dir(
                &env(&[("BERTH_DIR", "/b"), ("CLAUDE_CONFIG_DIR", "/c")]),
                None
            )
            .unwrap(),
            PathBuf::from("/b/moorings")
        );
        assert_eq!(
            berth_dir(&env(&[("CLAUDE_CONFIG_DIR", "/c")]), None).unwrap(),
            PathBuf::from("/c/berth/moorings")
        );
        let state_env = |name: &str| -> Option<std::ffi::OsString> {
            (name == "ELOHIM_STATE_DIR").then(|| "/state".into())
        };
        let exists = |_: &Path| false;
        let device_layout = layout::resolve_with(&layout::Host {
            env: &state_env,
            exists: &exists,
            cwd: None,
        })
        .unwrap();
        assert_eq!(
            berth_dir(&env(&[]), Some(&device_layout)).unwrap(),
            PathBuf::from("/state/berth/moorings")
        );
        assert!(berth_dir(&env(&[]), None).is_err());
    }
    #[test]
    fn roster_cures_distinguish_first_unbound_and_pending_devices() {
        let dir = tempfile::tempdir().unwrap();
        let first = roster_check(dir.path(), &actor::DeviceState::Unwitnessed, "matthew", &[]);
        assert!(first.cure.unwrap().contains("actor witness"));
        let roster = dir.path().join(actor::PARTICIPANTS_REL);
        fs::create_dir_all(&roster).unwrap();
        fs::write(roster.join("matthew.jsonl"), "").unwrap();
        let unbound = roster_check(dir.path(), &actor::DeviceState::Unwitnessed, "matthew", &[]);
        for verb in ["enroll", "authorize", "bind"] {
            assert!(unbound.cure.as_ref().unwrap().contains(verb));
        }
        fs::write(roster.join(".enroll-matthew.json"), "{}").unwrap();
        assert!(
            roster_check(dir.path(), &actor::DeviceState::Unwitnessed, "matthew", &[])
                .detail
                .contains("pending")
        );
    }
    #[test]
    fn absent_model_degrades_without_refusing_local_work() {
        let dir = tempfile::tempdir().unwrap();
        let check = model_check(dir.path());
        assert_eq!(check.state, State::Degraded);
        assert!(check.cure.unwrap().contains("embed-model-provision"));
    }
    #[test]
    fn fold_states_preserve_failure_reason_and_catch_up_size() {
        let check = |state: &str, lag: u64| {
            fold_check(
                Ok(json!({"lag":lag,"last":{"state":{"state":state,"why":"model unavailable"}}})),
                25,
            )
        };
        assert_eq!(check("complete", 25).state, State::Ok);
        assert_eq!(check("complete", 26).state, State::Degraded);
        assert!(check("failed", 400).detail.contains("model unavailable"));
        assert!(check("failed", 400).cure.unwrap().contains("embed model"));
        assert!(check("degraded", 400)
            .cure
            .unwrap()
            .contains("--max-files 450"));
        assert_eq!(fold_check(Ok(json!({})), 25).state, State::Degraded);
        assert_eq!(
            fold_check(
                Err(crate::flow::FlowError::Unavailable("unreadable".into())),
                25
            )
            .state,
            State::Degraded
        );
    }
    #[test]
    fn moorings_use_recorded_session_not_file_presence() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("empty.json"), "{}").unwrap();
        assert_eq!(berth_check(dir.path(), Some("s")).state, State::Degraded);
        fs::write(dir.path().join("record.json"), r#"{"session":"s"}"#).unwrap();
        assert_eq!(berth_check(dir.path(), Some("s")).state, State::Ok);
        assert_eq!(
            berth_check(dir.path(), Some("other")).state,
            State::Degraded
        );
        assert_eq!(berth_check(dir.path(), None).state, State::Ok);
    }
    #[test]
    fn toolchain_pin_is_read_from_pool_policy_and_found_under_rustup_home() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        let rustup = dir.path().join("rustup");
        let env = |k: &str| (k == "RUSTUP_HOME").then(|| rustup.display().to_string());
        assert_eq!(
            toolchain_check(&root, &env).state,
            State::Ok,
            "no pin declared"
        );
        fs::create_dir_all(root.join("genesis/agentic")).unwrap();
        fs::write(
            root.join("genesis/agentic/pool-policy.json"),
            r#"{"cargo_env_overrides":{"*":{"RUSTUP_TOOLCHAIN":"1.96.1"}}}"#,
        )
        .unwrap();
        let missing = toolchain_check(&root, &env);
        assert_eq!(missing.state, State::Degraded);
        assert!(missing
            .cure
            .unwrap()
            .contains("rustup toolchain install 1.96.1"));
        // Another version's toolchain does not satisfy the pin.
        let other = rustup.join("toolchains/1.96.10-x86_64-unknown-linux-gnu/bin");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("cargo"), "").unwrap();
        assert_eq!(toolchain_check(&root, &env).state, State::Degraded);
        let pinned = rustup.join("toolchains/1.96.1-x86_64-unknown-linux-gnu/bin");
        fs::create_dir_all(&pinned).unwrap();
        fs::write(pinned.join("cargo"), "").unwrap();
        assert_eq!(toolchain_check(&root, &env).state, State::Ok);
    }
    #[test]
    fn governed_hooks_need_an_absolute_hooks_path_holding_every_shim() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        fs::create_dir_all(root.join(".husky")).unwrap();
        let git = |args: &[&str]| {
            assert!(git::native_command(&root)
                .args(args)
                .status()
                .unwrap()
                .success());
        };
        git(&["init", "-q"]);
        assert_eq!(
            hooks_check(&root).state,
            State::Ok,
            "no governed hooks declared"
        );
        fs::write(root.join(".husky/hooks-path.mjs"), "").unwrap();
        assert_eq!(hooks_check(&root).state, State::Refused, "unset");
        git(&["config", "core.hooksPath", ".husky/_"]);
        assert!(hooks_check(&root).detail.contains("relative"));
        let shims = root.join(".husky/_");
        fs::create_dir_all(&shims).unwrap();
        git(&["config", "core.hooksPath", shims.to_str().unwrap()]);
        let partial = hooks_check(&root);
        assert_eq!(partial.state, State::Refused);
        assert!(partial.detail.contains("pre-push"));
        for hook in GOVERNED_HOOKS {
            fs::write(shims.join(hook), "#!/bin/sh\n").unwrap();
            fs::set_permissions(shims.join(hook), fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(hooks_check(&root).state, State::Ok);
    }
    #[test]
    fn packs_are_counted_in_the_common_dir_of_a_linked_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let common = dir.path().join("main/.git");
        let gitdir = common.join("worktrees/wt");
        fs::create_dir_all(&gitdir).unwrap();
        fs::write(gitdir.join("commondir"), "../..\n").unwrap();
        let worktree = dir.path().join("wt");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", gitdir.display()),
        )
        .unwrap();
        assert_eq!(
            common_git_dir(&worktree).unwrap().canonicalize().unwrap(),
            common.canonicalize().unwrap()
        );
        let packs = common.join("objects/pack");
        fs::create_dir_all(&packs).unwrap();
        for n in 0..REDUCED_TRUST_SLOTS - 1 {
            fs::write(packs.join(format!("pack-{n}.idx")), "").unwrap();
            fs::write(packs.join(format!("pack-{n}.pack")), "").unwrap();
        }
        assert_eq!(packs_check(&worktree).state, State::Ok);
        fs::write(packs.join("pack-last.idx"), "").unwrap();
        let over = packs_check(&worktree);
        assert_eq!(over.state, State::Degraded);
        assert!(over.cure.unwrap().contains("repack"));
        // This user owns the fixture, so gix would grant full trust.
        assert_eq!(brit_trust_check(&worktree).state, State::Ok);
    }
    #[test]
    fn safe_directory_follows_gix_order_reset_and_prefix_rules() {
        let dir = tempfile::tempdir().unwrap();
        let wt = dir.path().join("wt");
        fs::create_dir(&wt).unwrap();
        let v = |values: &[&str]| -> Vec<String> { values.iter().map(|s| (*s).into()).collect() };
        let here = wt.display().to_string();
        let parent = format!("{}/*", dir.path().display());
        assert!(safe_directory_grants(&wt, &v(&["*"])));
        // A later empty value resets an earlier wildcard.
        assert!(!safe_directory_grants(&wt, &v(&["*", ""])));
        assert!(safe_directory_grants(&wt, &v(&["*", "", &here])));
        assert!(safe_directory_grants(&wt, &v(&[&parent])));
        assert!(!safe_directory_grants(&wt, &v(&["relative/wt"])));
        assert!(!safe_directory_grants(&wt, &v(&["/elsewhere"])));
        assert!(!safe_directory_grants(&wt, &v(&[])));
    }
    #[test]
    fn candidates_and_absence_are_read_through_bin_store() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("candidate");
        fs::write(&binary, b"candidate bytes").unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        let registry: Registry = serde_json::from_value(json!({"version":1,"tools":{
            "epr":{"bins":["epr"],"candidateEnv":{"epr":["EPR_BIN"]}},
            "brit":{"bins":["brit"],"candidateEnv":{"brit":["BRIT_BIN"]}}
        }}))
        .unwrap();
        let identity = session::SessionIdentity {
            runtime: "none".into(),
            session: None,
            owner_pid: None,
            owner_start_time: None,
            source: session::Source::None,
        };
        for tool in ["epr", "brit"] {
            let store = Store::new(dir.path().join(tool), tool, &registry).unwrap();
            let absent = tool_check(tool, &store, &identity, &|_| None);
            // Only the device's own epr is essential; brit's absence is an enhancement lost.
            let expected = if tool == "epr" {
                State::Refused
            } else {
                State::Degraded
            };
            assert_eq!(absent.state, expected, "{tool}");
            assert!(absent.cure.unwrap().contains(&format!("--tool {tool}")));
            let candidate = tool_check(tool, &store, &identity, &|k| {
                (k == format!("{}_BIN", tool.to_uppercase())).then(|| binary.display().to_string())
            });
            assert_eq!(candidate.state, State::Degraded);
        }
    }
    #[test]
    fn schema_contract_covers_every_state_and_only_declared_fields() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../sdk/schemas/v1/views/device-status.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["type"], "array");
        for state in [State::Ok, State::Degraded, State::Refused, State::Unknown] {
            let check = Check::new(
                "condition",
                state,
                "evidence",
                (state != State::Ok).then(|| "cure".into()),
            );
            let value = serde_json::to_value(&check).unwrap();
            assert_eq!(
                value.as_object().unwrap().len(),
                schema["items"]["properties"].as_object().unwrap().len()
            );
            for key in schema["items"]["required"].as_array().unwrap() {
                assert!(value.get(key.as_str().unwrap()).is_some());
            }
            assert!(schema["items"]["properties"]["state"]["enum"]
                .as_array()
                .unwrap()
                .contains(&value["state"]));
            assert!(value["name"].is_string() && value["detail"].is_string());
            assert!(value["cure"].is_null() || value["cure"].is_string());
            assert!(check.to_string().contains("condition: evidence"));
        }
    }
    #[test]
    fn next_step_repeats_the_first_owed_ceremony_cure_and_never_refuses() {
        let line = |name: &str, state: State, cure: Option<&str>| {
            Check::new(
                name,
                state,
                format!("{name} evidence"),
                cure.map(str::to_owned),
            )
        };
        let mut checks = vec![
            line("epr binary", State::Ok, None),
            line("embed model", State::Degraded, Some("provision the model")),
            line("device key", State::Ok, None),
            line(
                ROSTER_CHECK,
                State::Refused,
                Some("enroll, authorize, bind"),
            ),
            line("peer daemon", State::Degraded, Some("start the peer")),
            line(
                SIBLING_CHECK,
                State::Degraded,
                Some("keep the peer running"),
            ),
        ];
        // The embed model is not part of the ceremony; the refused roster comes first.
        let next = next_step(&checks);
        assert_eq!(next.state, State::Degraded, "the next step never refuses");
        assert_eq!(next.detail, ROSTER_CHECK);
        assert_eq!(next.cure.as_deref(), Some("enroll, authorize, bind"));
        checks[3] = line(
            ROSTER_CHECK,
            State::Unknown,
            Some("fetch the integration branch"),
        );
        let next = next_step(&checks);
        assert_eq!(next.state, State::Unknown);
        assert_eq!(next.cure.as_deref(), Some("fetch the integration branch"));
        checks[3] = line(ROSTER_CHECK, State::Ok, None);
        checks[4] = line("peer daemon", State::Ok, None);
        assert_eq!(
            next_step(&checks).cure.as_deref(),
            Some("keep the peer running")
        );
        checks[5] = line(SIBLING_CHECK, State::Ok, None);
        let done = next_step(&checks);
        assert_eq!(done.state, State::Ok);
        assert_eq!(
            done.to_string(),
            "ok next step: nothing owed; this device is arrived and exchanging"
        );
        // One device: no sibling is a fact without a cure, so nothing is owed, and the line says why.
        checks[5] = Check::new(
            SIBLING_CHECK,
            State::Degraded,
            "this device has no sibling in the roster",
            None,
        );
        let alone = next_step(&checks);
        assert_eq!(alone.state, State::Ok);
        assert!(alone.detail.contains("no sibling in the roster"), "{alone}");
    }
    #[test]
    fn published_refs_are_the_declared_remote_tracking_refs_only() {
        let registry: Registry = serde_json::from_value(json!({"version":1,"tools":{
            "epr":{"bins":["epr"],"source":{"publishedRef":["refs/heads/dev","refs/remotes/origin/dev"]}}
        }}))
        .unwrap();
        assert_eq!(published_refs(Some(&registry)), ["refs/remotes/origin/dev"]);
        assert!(published_refs(None).is_empty());
        let standing = actor::Standing {
            subject: "human:matthew".into(),
            handle: "matthew".into(),
            record_cid: "row".into(),
            witnessed_by: None,
            claimed_at: String::new(),
            device: "did:key:z6MkFixtureDevice0000000000000000".into(),
            roster: None,
        };
        let check = |p| publication_check(&standing, p);
        let local = check(Publication::Local {
            reference: "refs/remotes/origin/dev".into(),
            committed: true,
        });
        assert_eq!(local.state, State::Degraded);
        assert!(local.detail.contains("matthew's roster row for this device is local only (in a local commit), not on origin/dev (as of the last fetch)"), "{local}");
        assert!(local
            .cure
            .unwrap()
            .starts_with("publish the roster rows through the normal gate"));
        let uncommitted = check(Publication::Local {
            reference: "refs/remotes/origin/dev".into(),
            committed: false,
        });
        assert!(uncommitted.detail.contains("in the working tree"));
        assert!(uncommitted
            .cure
            .unwrap()
            .starts_with("commit .eprfs/status/participants/matthew.jsonl"));
        let published = check(Publication::Published("refs/remotes/origin/dev".into()));
        assert_eq!(published.state, State::Ok);
        assert!(published
            .detail
            .ends_with("published on origin/dev (as of the last fetch)"));
        for absent in [
            Publication::NeverFetched(vec!["refs/remotes/origin/dev".into()]),
            Publication::Undeclared,
            Publication::Unreadable("dubious ownership".into()),
        ] {
            let unknown = check(absent);
            assert_eq!(unknown.state, State::Unknown, "{unknown}");
        }
        assert_eq!(
            check(Publication::NeverFetched(vec![
                "refs/remotes/origin/dev".into()
            ]))
            .cure
            .as_deref(),
            Some("fetch the integration branch")
        );
    }
    #[cfg(feature = "peer")]
    #[test]
    fn sibling_exchange_needs_a_folded_head_that_acknowledges_ours() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let siblings = vec![sibling.clone()];
        assert_eq!(
            exchange_check(&[], ours, Ok(None)).detail,
            "this device has no sibling in the roster"
        );
        let unsaved = exchange_check(&siblings, ours, Ok(None));
        assert_eq!(unsaved.state, State::Unknown);
        assert_eq!(
            unsaved.cure.as_deref(),
            Some("start the peer on demand: epr peer serve")
        );
        // Known endpoints and an empty table: nothing exchanged, never ok.
        let mut table = eprfs_peer::HeadTable::default();
        let silent = exchange_check(&siblings, ours, Ok(Some(table.clone())));
        assert_eq!(silent.state, State::Degraded);
        assert!(silent.detail.contains("no verified head"), "{silent}");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let now = u64::try_from(now).unwrap();
        let mut head = eprfs_peer::DeviceHead {
            seq: 4,
            root: eprfs_core::BlobCid::compute_bytes(b"sibling root"),
            at: now,
            folded: std::collections::BTreeMap::new(),
            folded_at: now.saturating_sub(90_000),
            events_folded: 0,
            nack: Default::default(),
        };
        table.devices.insert(sibling.clone(), head.clone());
        let unacked = exchange_check(&siblings, ours, Ok(Some(table.clone())));
        assert_eq!(unacked.state, State::Degraded);
        assert!(
            unacked.detail.contains("no sibling acknowledges"),
            "{unacked}"
        );
        head.folded.insert(ours.to_owned(), 2);
        table.devices.insert(sibling.clone(), head);
        let exchanging = exchange_check(&siblings, ours, Ok(Some(table)));
        assert_eq!(exchanging.state, State::Ok, "{exchanging}");
        assert!(exchanging.detail.contains(&actor::short_did(&sibling)));
        assert!(exchanging.detail.contains("to seq 2"));
        assert!(exchanging.detail.contains("90s ago"), "{exchanging}");
        // A head from a device that is not this person's sibling is no exchange.
        assert_eq!(
            exchange_check(
                &["did:key:z6MkSomeoneElse000000000000000000000".into()],
                ours,
                Ok(Some(eprfs_peer::HeadTable::default()))
            )
            .state,
            State::Degraded
        );
    }
}
