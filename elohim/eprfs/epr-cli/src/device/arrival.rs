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
    collections::BTreeSet,
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
    /// Structured local capacity, present only on the one capacity line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<super::capacity::Capacity>,
}
impl Check {
    fn new(name: &str, state: State, detail: impl Into<String>, cure: Option<String>) -> Self {
        Self {
            name: name.into(),
            state,
            detail: detail.into(),
            cure,
            capacity: None,
        }
    }
    fn with_capacity(mut self, capacity: super::capacity::Capacity) -> Self {
        self.capacity = Some(capacity);
        self
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
    let toolchain = toolchain_fact(root, &env);
    let capacity = super::capacity::observe_bounded(device_layout.as_ref().ok(), &toolchain);
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
        peer_rules_check(root),
        sibling_check(root, &current),
        supervisor_check(root),
        // Restart-arrival facts (backlog container-restart-strands-session-envelope).
        toolchain_check(&toolchain, capacity.does_not_build),
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
        Check::new("device capacity", State::Ok, capacity.line(), None).with_capacity(capacity),
    ];
    // Each admitted sibling that has acknowledged none (or too little) of this device's records,
    // beside the exchange line (arrival backlog row 77, delta 2). Degraded, never refused; the
    // first `sibling exchange` line stays the one the next step reads, so an otherwise arrived
    // device's next step does not change.
    if let Some(at) = checks.iter().position(|c| c.name == SIBLING_CHECK) {
        for (offset, check) in acknowledgement_checks(root, &current)
            .into_iter()
            .chain(crossing_drop_checks())
            .enumerate()
        {
            checks.insert(at + 1 + offset, check);
        }
    }
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
const CROSSING_DROPS_CHECK: &str = "crossing drops";

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
    exchange_check(
        &siblings,
        &standing.device,
        saved_heads(),
        saved_recovery(),
        channel_freshness(root),
        unix_ms(),
    )
}

#[cfg(feature = "peer")]
fn channel_freshness(root: &Path) -> Option<u64> {
    crate::flow::carry::DeviceFold::read(root).freshness_s
}

#[cfg(not(feature = "peer"))]
fn channel_freshness(_root: &Path) -> Option<u64> {
    None
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
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

/// The peer daemon's bounded HEARD evidence. A missing file is an empty snapshot.
#[cfg(feature = "peer")]
fn saved_recovery() -> Result<eprfs_peer::PeerRecoveryStatus, String> {
    let dir = crate::peer::peer_dir().map_err(|e| e.to_string())?;
    Ok(eprfs_peer::PeerRecoveryStatus::load(&dir))
}

#[cfg(feature = "peer")]
fn crossing_drop_checks() -> Vec<Check> {
    saved_heads()
        .ok()
        .flatten()
        .as_ref()
        .and_then(crossing_drop_check)
        .into_iter()
        .collect()
}
#[cfg(not(feature = "peer"))]
fn crossing_drop_checks() -> Vec<Check> {
    Vec::new()
}

#[cfg(feature = "peer")]
fn crossing_drop_check(table: &eprfs_peer::HeadTable) -> Option<Check> {
    let rules_digest = table.rules.as_ref()?.digest.as_str();
    let rows: Vec<_> = table
        .crossing_drop_entries(rules_digest)
        .filter(|(_, _, _, drop)| drop.count > 0)
        .collect();
    let (_, first_measure, first_kind, _) = rows.first().copied()?;
    let overflow = table.crossing_drop_overflow(rules_digest);
    let total = rows.iter().fold(0u64, |total, (_, _, _, drop)| {
        total.saturating_add(drop.count)
    });
    let total = total.saturating_add(overflow.map_or(0, |overflow| overflow.count));
    let mut devices: BTreeSet<_> = rows.iter().map(|(device, _, _, _)| *device).collect();
    if let Some(overflow) = overflow {
        devices.extend(overflow.devices.keys().map(String::as_str));
    }
    Some(Check::new(
        CROSSING_DROPS_CHECK,
        State::Degraded,
        format!(
            "{total} events from {} device(s) not in this device's crosses \
             ({first_measure}/{first_kind}…)",
            devices.len()
        ),
        Some(
            "refresh the daemon's policy from genesis/agentic/pool-policy.json and restart it \
             by epr device supervise"
                .into(),
        ),
    ))
}
#[cfg(not(feature = "peer"))]
fn saved_heads() -> Result<Option<()>, String> {
    Err("this epr was built without the peer feature, so it cannot read the peer's status".into())
}

#[cfg(not(feature = "peer"))]
fn saved_recovery() -> Result<(), String> {
    Err("this epr was built without the peer feature, so it cannot read the peer's status".into())
}

#[cfg(not(feature = "peer"))]
fn exchange_check(
    siblings: &[String],
    _did: &str,
    heads: Result<Option<()>, String>,
    _recovery: Result<(), String>,
    _freshness_s: Option<u64>,
    _now_ms: u64,
) -> Check {
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
    recovery: Result<eprfs_peer::PeerRecoveryStatus, String>,
    freshness_s: Option<u64>,
    now_ms: u64,
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
    let recovery = match recovery {
        Ok(recovery) => recovery,
        Err(why) => {
            return Check::new(
                SIBLING_CHECK,
                State::Unknown,
                why,
                Some("restore access to the epr peer directory".into()),
            )
        }
    };
    let Some(freshness_s) = freshness_s else {
        return Check::new(
            SIBLING_CHECK,
            State::Unknown,
            "berth.channel.freshness_s is unreadable from the checkout's pool policy; no \
             freshness is guessed",
            Some("restore the checkout's pool policy".into()),
        );
    };
    let age_s = |at: u64| (at <= now_ms).then(|| now_ms.saturating_sub(at) / 1000);
    let heard_at = |device: &str| {
        recovery
            .devices
            .get(device)
            .and_then(|state| state.last_heard_at)
    };
    let heard_age = |device: &str| match heard_at(device) {
        Some(at) => age_s(at).map_or_else(
            || "HEARD timestamp is in this device's future".to_string(),
            |age_s| format!("HEARD {age_s}s ago"),
        ),
        None => "HEARD unknown".to_string(),
    };
    let heads: Vec<(&String, &eprfs_peer::DeviceHead)> = siblings
        .iter()
        .filter_map(|sibling| table.devices.get(sibling).map(|head| (sibling, head)))
        .collect();
    let acking = heads
        .iter()
        .filter_map(|(sibling, head)| head.folded.get(did).map(|seq| (sibling, head, seq)))
        .max_by_key(|(sibling, head, _)| (heard_at(sibling).unwrap_or(0), head.folded_at));
    let latest_mature_root_seq = table.own_log.iter().rev().find_map(|root| {
        age_s(root.at)
            .filter(|age_s| *age_s >= freshness_s)
            .map(|_| root.seq)
    });
    // Legacy tables can have an empty `own_log`. With no dated root to require, any
    // acknowledgement passes the sequence gate.
    if let Some((sibling, head, &acked_seq)) = acking {
        let short = actor::short_did(sibling);
        let Some(fold_age_s) = age_s(head.folded_at) else {
            return Check::new(
                SIBLING_CHECK,
                State::Degraded,
                format!(
                    "{short} has {}, FOLDED timestamp is in this device's future, so the \
                     acknowledgement carrier's age is invalid",
                    heard_age(sibling)
                ),
                Some(
                    "keep epr peer serve running on every admitted device until fresh heads and \
                     acknowledgements cross in both directions"
                        .into(),
                ),
            );
        };
        let recorded_heard_at = heard_at(sibling);
        let heard_age_s = recorded_heard_at.and_then(age_s);
        let (carrier_age_s, ages) = match (recorded_heard_at, heard_age_s) {
            (Some(_), Some(heard_age_s)) => (
                Some(heard_age_s),
                format!("HEARD {heard_age_s}s ago, FOLDED {fold_age_s}s ago"),
            ),
            (Some(_), None) => (
                None,
                format!("HEARD timestamp is in this device's future, FOLDED {fold_age_s}s ago"),
            ),
            (None, _) => (
                Some(fold_age_s),
                format!("HEARD unknown, FOLDED {fold_age_s}s ago"),
            ),
        };
        let carrier_is_fresh = carrier_age_s.is_some_and(|age_s| age_s < freshness_s);
        let unacknowledged_root_seq =
            latest_mature_root_seq.filter(|root_seq| acked_seq < *root_seq);
        if carrier_is_fresh && unacknowledged_root_seq.is_none() {
            let carrier_age_s = carrier_age_s.expect("a fresh carrier has a known age");
            return Check::new(
                SIBLING_CHECK,
                State::Ok,
                format!(
                    "{short} folded-from head seq {} is {fold_age_s}s old; liveness is judged by \
                     {ages}; it acknowledges this device to seq {acked_seq}, carried \
                     {carrier_age_s}s ago; both directions are within \
                     freshness_s={freshness_s}s",
                    head.seq
                ),
                None,
            );
        }
        let mut directions = Vec::new();
        match (recorded_heard_at, carrier_age_s) {
            (Some(_), Some(carrier_age_s)) if !carrier_is_fresh => {
                directions.push(format!(
                    "HEARD is {carrier_age_s}s old, FOLDED {fold_age_s}s ago"
                ));
                directions.push(format!(
                    "acknowledgement of this device's head was carried {carrier_age_s}s ago"
                ));
            }
            (Some(_), None) => directions.push(ages),
            (None, Some(carrier_age_s)) if !carrier_is_fresh => {
                directions.push(format!(
                    "HEARD unknown, so folded-from head is {fold_age_s}s old"
                ));
                directions.push(format!(
                    "acknowledgement of this device's head was carried {carrier_age_s}s ago"
                ));
            }
            _ => directions.push(ages),
        }
        if let Some(root_seq) = unacknowledged_root_seq {
            directions.push(format!(
                "acknowledges this device only to seq {acked_seq}, before its latest announced \
                 root at seq {root_seq}"
            ));
        }
        return Check::new(
            SIBLING_CHECK,
            State::Degraded,
            format!(
                "{short} {} (freshness_s={freshness_s}s)",
                directions.join(" and ")
            ),
            Some(
                "keep epr peer serve running on every admitted device until fresh heads and \
                 acknowledgements cross in both directions"
                    .into(),
            ),
        );
    }

    let named = siblings
        .iter()
        .map(|sibling| actor::short_did(sibling))
        .collect::<Vec<_>>()
        .join(", ");
    match heads
        .iter()
        .max_by_key(|(sibling, head)| (heard_at(sibling).unwrap_or(0), head.folded_at))
    {
        None => Check::new(
            SIBLING_CHECK,
            State::Degraded,
            format!(
                "the roster lists {named}, but no verified head from a sibling has been folded \
                 here: HEARD unknown, FOLDED unknown, so nothing has been exchanged"
            ),
            Some(format!(
                "keep epr peer serve running here and on {named}; epr peer status shows each \
                 head as it folds"
            )),
        ),
        Some((sibling, head)) => {
            let heard = heard_age(sibling);
            let folded = age_s(head.folded_at).map_or_else(
                || "FOLDED timestamp is in this device's future".to_string(),
                |age_s| format!("FOLDED {age_s}s ago"),
            );
            Check::new(
                SIBLING_CHECK,
                State::Degraded,
                format!(
                    "{} has a verified head here (seq {}, last {heard}, {folded}), but \
                     acknowledges none of this device's heads (acknowledgement direction absent)",
                    actor::short_did(sibling),
                    head.seq
                ),
                Some(format!(
                    "keep epr peer serve running here until {} folds this device's head; epr peer \
                 status shows its acknowledgement",
                    actor::short_did(sibling)
                )),
            )
        }
    }
}

/// How far an admitted sibling's acknowledgement may trail this device's own head and still read
/// as exchanging: a gossip round or two, not a device that has stopped folding.
const ACK_SLACK: u64 = 10;

/// One `degraded sibling exchange` line per admitted sibling that has acknowledged none of this
/// device's records, or trails this device's own head by more than [`ACK_SLACK`] seqs, read from
/// the head table the peer daemon saved. Nothing when this device stands on no roster or no table
/// is readable: the `sibling exchange` line above already says so.
#[cfg(feature = "peer")]
fn acknowledgement_checks(root: &Path, current: &actor::DeviceState) -> Vec<Check> {
    let actor::DeviceState::Standing(standing) = current else {
        return Vec::new();
    };
    let siblings = actor::roster_siblings(root, &standing.handle, &standing.device);
    match saved_heads() {
        Ok(Some(table)) => unacknowledged(&siblings, &standing.device, &table),
        _ => Vec::new(),
    }
}
#[cfg(not(feature = "peer"))]
fn acknowledgement_checks(_root: &Path, _current: &actor::DeviceState) -> Vec<Check> {
    Vec::new()
}

/// The admitted `siblings` of this device `did` that `table` shows have not acknowledged it.
#[cfg(feature = "peer")]
fn unacknowledged(siblings: &[String], did: &str, table: &eprfs_peer::HeadTable) -> Vec<Check> {
    let own = table.own.as_ref().map(|own| own.seq);
    siblings
        .iter()
        .filter_map(|sibling| {
            let short = actor::short_did(sibling);
            let what = match table.devices.get(sibling) {
                None => format!(
                    "{short} is admitted and has acknowledged none of this device's records \
                     (no head from it has been folded here)"
                ),
                Some(head) => match (head.folded.get(did), own) {
                    (None, _) => format!(
                        "{short} is admitted and has acknowledged none of this device's records \
                         (its head seq {} was folded here)",
                        head.seq
                    ),
                    (Some(&acked), Some(own)) if own.saturating_sub(acked) > ACK_SLACK => {
                        format!(
                            "{short} is admitted and has acknowledged this device's records only \
                             to seq {acked} of {own}"
                        )
                    }
                    (Some(_), _) => return None,
                },
            };
            Some(Check::new(
                SIBLING_CHECK,
                State::Degraded,
                format!(
                    "{what} — what this device sends may not have arrived there; a landing \
                     waits for it at the lease step"
                ),
                Some(format!(
                    "run `just status device` on {short}; keep epr peer serve running on both \
                     devices"
                )),
            ))
        })
        .collect()
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

/// One local reading shared by the arrival line and the capacity declaration.
#[derive(Debug, Clone)]
pub(super) struct ToolchainFact {
    pub(super) pin: Option<String>,
    pub(super) installed: Option<bool>,
    pub(super) location: Option<PathBuf>,
    pub(super) why_unknown: Option<String>,
}

/// The Rust toolchain the local gate pins (`pool-policy.json` `cargo_env_overrides["*"]`),
/// looked up in rustup's home. `$HOME` is wiped on a workspace restart, and a rustup home under
/// it loses the pin; the gate then refuses every cargo project. A read: never installs.
fn toolchain_fact(root: &Path, env: &dyn Fn(&str) -> Option<String>) -> ToolchainFact {
    let pin = gate_toolchain(root);
    let Some(pin_value) = pin.as_deref() else {
        return ToolchainFact {
            pin,
            installed: None,
            location: None,
            why_unknown: None,
        };
    };
    let home = env("RUSTUP_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".rustup")));
    let Some(home) = home else {
        return ToolchainFact {
            pin,
            installed: None,
            location: None,
            why_unknown: Some("neither RUSTUP_HOME nor HOME is set".into()),
        };
    };
    let toolchains = home.join("toolchains");
    let entries = match fs::read_dir(&toolchains) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ToolchainFact {
                pin,
                installed: Some(false),
                location: Some(toolchains),
                why_unknown: None,
            }
        }
        Err(error) => {
            return ToolchainFact {
                pin,
                installed: None,
                location: Some(toolchains.clone()),
                why_unknown: Some(format!("{}: {error}", toolchains.display())),
            }
        }
    };
    let prefix = format!("{pin_value}-");
    let installed = entries.filter_map(Result::ok).find(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        (*name == *pin_value || name.starts_with(&prefix))
            && entry.path().join("bin/cargo").is_file()
    });
    ToolchainFact {
        pin,
        installed: Some(installed.is_some()),
        location: installed.map(|entry| entry.path()).or(Some(toolchains)),
        why_unknown: None,
    }
}

fn toolchain_check(fact: &ToolchainFact, does_not_build: bool) -> Check {
    let Some(pin) = fact.pin.as_deref() else {
        return Check::new(
            "rust toolchain",
            State::Ok,
            "this checkout pins no gate toolchain",
            None,
        );
    };
    if fact.installed == Some(true) {
        return Check::new(
            "rust toolchain",
            State::Ok,
            format!(
                "pin {pin} at {}",
                fact.location.as_deref().map_or_else(
                    || "an installed toolchain".into(),
                    |path| path.display().to_string()
                )
            ),
            None,
        );
    }
    if does_not_build {
        return Check::new(
            "rust toolchain",
            State::Ok,
            format!("pin {pin} is not required; this device declares does not build"),
            None,
        );
    }
    match fact.installed {
        Some(false) => toolchain_missing(
            pin,
            fact.location
                .as_deref()
                .unwrap_or_else(|| Path::new("<rustup>")),
        ),
        None => Check::new(
            "rust toolchain",
            State::Unknown,
            format!(
                "pin {pin}; {}",
                fact.why_unknown
                    .as_deref()
                    .unwrap_or("installation unreadable")
            ),
            Some("set RUSTUP_HOME to rustup's home".into()),
        ),
        Some(true) => unreachable!("handled above"),
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
/// ignores `core.hooksPath` (its commits run no hooks) and sizes object index slots from disk up
/// to a ceiling of 512.
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
            format!("{detail}; brit runs at reduced trust (no hooks, at most 512 index slots)"),
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

/// gix's object-index slot ceiling at reduced trust (`gix::open::REDUCED_TRUST_MAX_PACK_INDEXES`).
/// Since brit 0.3.0 reduced trust sizes slots from what is on disk, floor 32, ceiling 512; full
/// trust sizes from disk without the ceiling. The fixed cap of 32 this check held until
/// 2026-11-25 warned on stores brit opens (51 and 79 packs observed open on 0.3.4).
const REDUCED_TRUST_SLOTS: usize = 512;

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
        format!("{packs} pack index(es); reduced-trust ceiling {REDUCED_TRUST_SLOTS} ({trust})");
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
    use elohim_device_runtime::flock::Flock;
    let dir = crate::peer::peer_dir().map_err(|e| e.to_string())?;
    let path = dir.join(crate::peer::ServeLock::FILE_NAME);
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((path, false)),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let held = match Flock::try_shared(file) {
        // Shared and dropped at once; `ServeLock::acquire` retries a contended exclusive take,
        // so this momentary probe does not read as a live daemon to a starting `serve`. The drop
        // unlocks, so a child forked meanwhile holds no shared lock through its copy.
        Ok(Ok(_probe)) => false,
        Ok(Err(_)) => true,
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
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

/// The peer daemon line, with what the newest restart after a landing said (row 35): a ready
/// restart on a line serving at current, anything else on the line whatever it reads.
fn peer_daemon_check(root: &Path) -> Check {
    use super::supervise::RestartNote;
    let mut check = peer_daemon_line();
    match super::supervise::restart_note(root) {
        Some(RestartNote::Ready(suffix)) => {
            if let Some(rest) = check.detail.strip_prefix("serving at current") {
                check.detail = format!("serving at current{suffix}{rest}");
            }
        }
        // A restart that did not come up keeps the line degraded until the supervisor reads
        // every child ready again (row 35, F1).
        Some(RestartNote::Degraded { note, cure }) => {
            check.detail.push_str(&note);
            if !super::supervise::check(root).0 {
                check.state = State::Degraded;
                check.cure = Some(cure);
            }
        }
        Some(RestartNote::Note(note)) => check.detail.push_str(&note),
        None => {}
    }
    check
}

fn peer_daemon_line() -> Check {
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
        Ok((true, evidence)) => peer_daemon_version_check(evidence),
        Err(error) => Check::new(
            "peer daemon",
            State::Unknown,
            error,
            Some("restore access to the epr peer directory".into()),
        ),
    }
}

const PEER_RULES_CHECK: &str = "peer rules";

/// Whether the live peer daemon loaded the rules this checkout now declares. The daemon side is
/// the digest and policy path it persisted in the head table; the checkout side is parsed and
/// digested by the same [`eprfs_peer::Rules`] implementation the daemon uses.
#[cfg(feature = "peer")]
fn peer_rules_check(root: &Path) -> Check {
    let running = serve_liveness().map(|(running, _)| running);
    let loaded = saved_heads().map(|table| table.and_then(|table| table.rules));
    peer_rules_line(root, running, loaded, published_rules(root))
}

/// The rules digest of `genesis/agentic/pool-policy.json` at `origin/dev` (as of the last
/// fetch), the side that says which of the daemon and the checkout is stale. `None` when that
/// ref or file cannot be read here.
#[cfg(feature = "peer")]
fn published_rules(root: &Path) -> Option<eprfs_peer::RulesDigest> {
    let text = roster_at(root, "refs/remotes/origin/dev", crate::peer::POLICY_REL)?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    eprfs_peer::Rules::from_policy(&value)
        .ok()
        .map(|rules| rules.digest())
}

#[cfg(feature = "peer")]
fn peer_rules_line(
    root: &Path,
    running: Result<bool, String>,
    loaded: Result<Option<eprfs_peer::RulesState>, String>,
    published: Option<eprfs_peer::RulesDigest>,
) -> Check {
    match running {
        Ok(false) => return Check::new(
            PEER_RULES_CHECK,
            State::Unknown,
            "no epr peer serve holds this device's serve lock, so no loaded rules can be compared",
            Some(peer_daemon_cure()),
        ),
        Err(error) => {
            return Check::new(
                PEER_RULES_CHECK,
                State::Unknown,
                error,
                Some("restore access to the epr peer directory".into()),
            )
        }
        Ok(true) => {}
    }
    let loaded = match loaded {
        Ok(Some(loaded)) => loaded,
        Ok(None) => {
            return Check::new(
                PEER_RULES_CHECK,
                State::Unknown,
                "no daemon has recorded the rules it loaded",
                Some("restart the daemon on the checkout policy (epr device supervise)".into()),
            )
        }
        Err(error) => {
            return Check::new(
                PEER_RULES_CHECK,
                State::Unknown,
                error,
                Some("restore access to the epr peer directory".into()),
            )
        }
    };
    let checkout_path = root.join(crate::peer::POLICY_REL);
    let checkout = match eprfs_peer::Rules::from_policy_file(&checkout_path) {
        Ok(rules) => rules.digest(),
        Err(error) => {
            return Check::new(
                PEER_RULES_CHECK,
                State::Unknown,
                format!(
                    "checkout {} rules digest cannot be read: {error}",
                    crate::peer::POLICY_REL
                ),
                Some(format!("restore {}", crate::peer::POLICY_REL)),
            )
        }
    };
    // origin/dev decides which side is stale: whichever differs from it is behind.
    let dev = published.as_ref();
    if loaded.digest == checkout {
        if let Some(dev) = dev.filter(|dev| **dev != checkout) {
            return Check::new(
                PEER_RULES_CHECK,
                State::Degraded,
                format!(
                    "daemon and checkout both load {}, origin/dev declares {}: both are behind origin/dev",
                    checkout.short(),
                    dev.short()
                ),
                Some(
                    "update this checkout to origin/dev, then restart the daemon on it (epr device supervise)"
                        .into(),
                ),
            );
        }
        return Check::new(
            PEER_RULES_CHECK,
            State::Ok,
            format!(
                "daemon loaded {} = checkout pool-policy.json",
                checkout.short()
            ),
            None,
        );
    }
    let drift = format!(
        "daemon loaded {} from {}, checkout {} is {}",
        loaded.digest.short(),
        loaded.policy.display(),
        crate::peer::POLICY_REL,
        checkout.short()
    );
    match dev {
        Some(dev) if *dev == loaded.digest => Check::new(
            PEER_RULES_CHECK,
            State::Degraded,
            format!("{drift}; origin/dev declares the daemon's rules, so the checkout is the stale side"),
            Some(
                "update this checkout to origin/dev; do not restart the daemon on the checkout policy, which would roll it back"
                    .into(),
            ),
        ),
        Some(dev) if *dev == checkout => Check::new(
            PEER_RULES_CHECK,
            State::Degraded,
            format!("{drift}; origin/dev declares the checkout's rules, so the daemon is the stale side"),
            Some(format!(
                "restart the daemon on the checkout policy (epr device supervise), or refresh {} from it",
                loaded.policy.display()
            )),
        ),
        Some(dev) => Check::new(
            PEER_RULES_CHECK,
            State::Degraded,
            format!("{drift}; origin/dev declares {}, matching neither", dev.short()),
            Some(format!(
                "update this checkout to origin/dev, then restart the daemon on it (epr device supervise); refresh or drop {}",
                loaded.policy.display()
            )),
        ),
        None => Check::new(
            PEER_RULES_CHECK,
            State::Degraded,
            format!("{drift}; origin/dev's policy cannot be read here, so which side is stale is unknown"),
            Some(format!(
                "fetch origin, then compare again; the cure is to restart the daemon on the checkout policy (epr device supervise) only if the checkout matches origin/dev, else to update the checkout; {} is the daemon's copy",
                loaded.policy.display()
            )),
        ),
    }
}

#[cfg(not(feature = "peer"))]
fn peer_rules_check(_root: &Path) -> Check {
    Check::new(
        PEER_RULES_CHECK,
        State::Unknown,
        "this epr was built without the peer feature, so it cannot read the peer's rules",
        Some("use an epr build with peer support".into()),
    )
}

fn peer_daemon_version_check(evidence: Option<String>) -> Check {
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
                .map(|pin| pin.cid)
                .filter(|cid| Some(cid) != head.as_ref())
                .collect();
            if stale.is_empty() {
                Check::new(
                    "peer daemon",
                    State::Ok,
                    format!(
                        "running and current release-record CIDs are {}{evidence}",
                        head.as_deref().unwrap_or("absent")
                    ),
                    None,
                )
            } else {
                Check::new(
                    "peer daemon",
                    State::Degraded,
                    format!(
                        "running release-record CID(s) {}; current release-record CID {}{evidence}",
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
    let mut would = None;
    let mut target = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => {
                root = Some(PathBuf::from(iter.next().ok_or_else(|| {
                    crate::error::Error::InvalidArguments("--root needs a path".into())
                })?))
            }
            "--json" => json = true,
            "--would" if !arrive => {
                would = Some(iter.next().ok_or_else(|| {
                    crate::error::Error::InvalidArguments("--would needs a commit".into())
                })?)
            }
            "--target" if !arrive => {
                target = Some(iter.next().ok_or_else(|| {
                    crate::error::Error::InvalidArguments("--target needs a ref".into())
                })?)
            }
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
    if json && would.is_some() {
        return Err(crate::error::Error::InvalidArguments(
            "--would is a human dry-run report; omit --json".into(),
        ));
    }
    if target.is_some() && would.is_none() {
        return Err(crate::error::Error::InvalidArguments(
            "--target belongs to --would".into(),
        ));
    }
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
    let would_lines = would.map(|commit| {
        let capacity = checks.iter().find_map(|check| check.capacity.as_ref());
        super::capacity::would_run(
            &root,
            commit,
            target.map(String::as_str).unwrap_or("origin/dev"),
            capacity,
        )
    });
    if json {
        println!("{}", serde_json::to_string(&checks)?);
    } else {
        for check in &checks {
            println!("{check}");
        }
        for line in would_lines.into_iter().flatten() {
            println!("{line}");
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

    fn write_rules_policy(path: &Path, kinds: &[&str]) -> eprfs_peer::RulesDigest {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            serde_json::to_vec_pretty(&json!({
                "berth": {"channel": {"crosses": {"dev-berth-events@1": kinds}}}
            }))
            .unwrap(),
        )
        .unwrap();
        eprfs_peer::Rules::from_policy_file(path).unwrap().digest()
    }

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
            toolchain_check(&toolchain_fact(&root, &env), false).state,
            State::Ok,
            "no pin declared"
        );
        fs::create_dir_all(root.join("genesis/agentic")).unwrap();
        fs::write(
            root.join("genesis/agentic/pool-policy.json"),
            r#"{"cargo_env_overrides":{"*":{"RUSTUP_TOOLCHAIN":"1.96.1"}}}"#,
        )
        .unwrap();
        let missing_fact = toolchain_fact(&root, &env);
        let missing = toolchain_check(&missing_fact, false);
        assert_eq!(missing.state, State::Degraded);
        assert!(missing
            .cure
            .unwrap()
            .contains("rustup toolchain install 1.96.1"));
        let non_builder = toolchain_check(&missing_fact, true);
        assert_eq!(non_builder.state, State::Ok);
        assert!(non_builder.detail.contains("declares does not build"));
        // Another version's toolchain does not satisfy the pin.
        let other = rustup.join("toolchains/1.96.10-x86_64-unknown-linux-gnu/bin");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("cargo"), "").unwrap();
        assert_eq!(
            toolchain_check(&toolchain_fact(&root, &env), false).state,
            State::Degraded
        );
        let pinned = rustup.join("toolchains/1.96.1-x86_64-unknown-linux-gnu/bin");
        fs::create_dir_all(&pinned).unwrap();
        fs::write(pinned.join("cargo"), "").unwrap();
        assert_eq!(
            toolchain_check(&toolchain_fact(&root, &env), false).state,
            State::Ok
        );
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
    fn peer_rules_match_the_checkout_policy() {
        let dir = tempfile::tempdir().unwrap();
        let checkout = dir.path().join(crate::peer::POLICY_REL);
        let digest = write_rules_policy(&checkout, &["claim"]);
        let loaded = eprfs_peer::RulesState {
            digest: digest.clone(),
            policy: checkout,
            loaded_at: 1,
            pending: None,
        };
        let check = peer_rules_line(dir.path(), Ok(true), Ok(Some(loaded)), Some(digest.clone()));
        assert_eq!(check.state, State::Ok);
        assert_eq!(
            check.to_string(),
            format!(
                "ok peer rules: daemon loaded {} = checkout pool-policy.json",
                digest.short()
            )
        );
    }
    #[test]
    fn peer_rules_drift_names_both_digests_and_the_daemon_policy_path() {
        let dir = tempfile::tempdir().unwrap();
        let checkout = dir.path().join(crate::peer::POLICY_REL);
        let checkout_digest = write_rules_policy(&checkout, &["claim", "release"]);
        let daemon_policy = dir.path().join("private/daemon/pool-policy.json");
        let daemon_digest = write_rules_policy(&daemon_policy, &["claim"]);
        let loaded = eprfs_peer::RulesState {
            digest: daemon_digest.clone(),
            policy: daemon_policy.clone(),
            loaded_at: 1,
            pending: None,
        };
        let check = peer_rules_line(
            dir.path(),
            Ok(true),
            Ok(Some(loaded)),
            Some(checkout_digest.clone()),
        );
        assert_eq!(check.state, State::Degraded);
        assert_eq!(
            check.to_string(),
            format!(
                "degraded peer rules: daemon loaded {} from {}, checkout {} is {}; origin/dev declares the checkout's rules, so the daemon is the stale side — restart the daemon on the checkout policy (epr device supervise), or refresh {} from it",
                daemon_digest.short(),
                daemon_policy.display(),
                crate::peer::POLICY_REL,
                checkout_digest.short(),
                daemon_policy.display()
            )
        );
    }
    /// 2026-11-23 on ethosengine: the daemon ran dev's rules from a refreshed private copy while
    /// the checkout it serves sat behind dev. Restarting on the checkout would roll it back.
    #[test]
    fn peer_rules_name_the_checkout_as_stale_when_the_daemon_matches_origin_dev() {
        let dir = tempfile::tempdir().unwrap();
        let checkout = dir.path().join(crate::peer::POLICY_REL);
        let checkout_digest = write_rules_policy(&checkout, &["claim"]);
        let daemon_policy = dir.path().join("private/daemon/pool-policy.json");
        let daemon_digest = write_rules_policy(&daemon_policy, &["claim", "release"]);
        let loaded = eprfs_peer::RulesState {
            digest: daemon_digest.clone(),
            policy: daemon_policy,
            loaded_at: 1,
            pending: None,
        };
        let check = peer_rules_line(dir.path(), Ok(true), Ok(Some(loaded)), Some(daemon_digest));
        assert_eq!(check.state, State::Degraded);
        let line = check.to_string();
        assert!(line.contains("so the checkout is the stale side"), "{line}");
        assert!(
            line.contains("update this checkout to origin/dev"),
            "{line}"
        );
        assert!(
            line.contains("do not restart the daemon on the checkout policy"),
            "{line}"
        );
        assert!(
            line.contains(&checkout_digest.short().to_string()),
            "{line}"
        );
    }
    #[test]
    fn peer_rules_say_both_are_behind_when_they_agree_but_origin_dev_moved() {
        let dir = tempfile::tempdir().unwrap();
        let checkout = dir.path().join(crate::peer::POLICY_REL);
        let digest = write_rules_policy(&checkout, &["claim"]);
        let dev_policy = dir.path().join("dev/pool-policy.json");
        let dev_digest = write_rules_policy(&dev_policy, &["claim", "release"]);
        let loaded = eprfs_peer::RulesState {
            digest,
            policy: checkout,
            loaded_at: 1,
            pending: None,
        };
        let check = peer_rules_line(
            dir.path(),
            Ok(true),
            Ok(Some(loaded)),
            Some(dev_digest.clone()),
        );
        assert_eq!(check.state, State::Degraded);
        let line = check.to_string();
        assert!(line.contains("both are behind origin/dev"), "{line}");
        assert!(line.contains(&dev_digest.short().to_string()), "{line}");
    }
    #[test]
    fn peer_rules_drift_without_origin_dev_says_the_stale_side_is_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let checkout = dir.path().join(crate::peer::POLICY_REL);
        write_rules_policy(&checkout, &["claim"]);
        let daemon_policy = dir.path().join("private/daemon/pool-policy.json");
        let daemon_digest = write_rules_policy(&daemon_policy, &["claim", "release"]);
        let loaded = eprfs_peer::RulesState {
            digest: daemon_digest,
            policy: daemon_policy,
            loaded_at: 1,
            pending: None,
        };
        let check = peer_rules_line(dir.path(), Ok(true), Ok(Some(loaded)), None);
        assert_eq!(check.state, State::Degraded);
        let line = check.to_string();
        assert!(line.contains("which side is stale is unknown"), "{line}");
        assert!(line.contains("fetch origin"), "{line}");
    }
    #[test]
    fn peer_rules_are_unknown_when_the_persisted_digest_is_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let check = peer_rules_line(
            dir.path(),
            Ok(true),
            Err("heads.json rules digest is unreadable".into()),
            None,
        );
        assert_eq!(check.state, State::Unknown);
        assert_eq!(
            check.to_string(),
            "unknown peer rules: heads.json rules digest is unreadable — restore access to the epr peer directory"
        );
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
    fn receiver_crossing_drop_line_follows_only_the_current_rules_digest() {
        let rules = |digit: char| eprfs_peer::RulesState {
            digest: serde_json::from_value(serde_json::json!(digit.to_string().repeat(64)))
                .expect("rules digest"),
            policy: "pool-policy.json".into(),
            loaded_at: 1,
            pending: None,
        };
        let mut table = eprfs_peer::HeadTable::default();
        assert!(crossing_drop_check(&table).is_none());

        let rules_a = rules('a');
        table.rules = Some(rules_a.clone());
        table.record_crossing_drop(
            rules_a.digest.as_str(),
            "did:key:zSender",
            "dev-landing@1",
            "misses",
            11,
        );
        table.record_crossing_drop(
            rules_a.digest.as_str(),
            "did:key:zSender",
            "dev-landing@1",
            "misses",
            19,
        );
        let check = crossing_drop_check(&table).expect("nonzero drop line");
        assert_eq!(check.name, CROSSING_DROPS_CHECK);
        assert_eq!(check.state, State::Degraded);
        assert_eq!(
            check.detail,
            "2 events from 1 device(s) not in this device's crosses \
             (dev-landing@1/misses…)"
        );
        assert_eq!(
            check.cure.as_deref(),
            Some(
                "refresh the daemon's policy from genesis/agentic/pool-policy.json and restart \
                 it by epr device supervise"
            )
        );

        let rules_b = rules('b');
        table.rules = Some(rules_b.clone());
        assert!(crossing_drop_check(&table).is_none());

        table.record_crossing_drop("rules-c", "device-c", "measure", "kind", 30);
        table.record_crossing_drop("rules-d", "device-d", "measure", "kind", 40);
        table.record_crossing_drop("rules-e", "device-e", "measure", "kind", 50);
        assert_eq!(table.evicted_totals["did:key:zSender"].count, 2);
        assert!(
            crossing_drop_check(&table).is_none(),
            "evicted rules are history, never an arrival degradation"
        );

        table.record_crossing_drop(
            rules_b.digest.as_str(),
            "did:key:zSender",
            "dev-landing@1",
            "misses",
            60,
        );
        let check = crossing_drop_check(&table).expect("new-rules drop line");
        assert_eq!(check.state, State::Degraded);
        assert!(check.detail.starts_with("1 events from 1 device(s)"));

        for index in 0..eprfs_peer::MAX_CROSSING_DROP_ROWS_PER_RULE_GROUP + 1 {
            table.record_crossing_drop(
                rules_b.digest.as_str(),
                "did:key:zSender",
                "dev-landing@1",
                &format!("kind-{index:02}"),
                100 + u64::try_from(index).unwrap(),
            );
        }
        let check = crossing_drop_check(&table).expect("current overflow remains actionable");
        assert!(check.detail.starts_with("34 events from 1 device(s)"));
    }

    #[cfg(feature = "peer")]
    fn exchange_table(
        ours: &str,
        sibling: &str,
        fold_age_s: u64,
        acked_seq: u64,
    ) -> eprfs_peer::HeadTable {
        exchange_table_with_root(ours, sibling, fold_age_s, acked_seq, 1, 600)
    }

    #[cfg(feature = "peer")]
    fn exchange_table_with_root(
        ours: &str,
        sibling: &str,
        fold_age_s: u64,
        acked_seq: u64,
        root_seq: u64,
        root_age_s: u64,
    ) -> eprfs_peer::HeadTable {
        const NOW_MS: u64 = 1_000_000;
        let root = eprfs_core::BlobCid::compute_bytes(b"own root");
        let own = eprfs_peer::ChannelHead {
            device: ours.to_owned(),
            root: root.clone(),
            seq: 7,
            at: NOW_MS,
            folded: std::collections::BTreeMap::new(),
            signature: String::new(),
        };
        let sibling_head = eprfs_peer::DeviceHead {
            seq: 4,
            root: eprfs_core::BlobCid::compute_bytes(b"sibling root"),
            at: NOW_MS,
            folded: std::collections::BTreeMap::from([(ours.to_owned(), acked_seq)]),
            folded_at: NOW_MS.saturating_sub(fold_age_s.saturating_mul(1000)),
            events_folded: 0,
            nack: Default::default(),
            coverage: None,
        };
        eprfs_peer::HeadTable {
            own: Some(own),
            // The root last changed ten minutes ago. Re-announces advanced `own.seq` without
            // adding log rows, so an acknowledgement one round behind is still current.
            own_log: vec![eprfs_peer::OwnAnnounce {
                seq: root_seq,
                root,
                at: NOW_MS.saturating_sub(root_age_s.saturating_mul(1000)),
                events: 1,
                last: Some("event-cid".into()),
            }],
            devices: std::collections::BTreeMap::from([(sibling.to_owned(), sibling_head)]),
            ..Default::default()
        }
    }

    #[cfg(feature = "peer")]
    #[test]
    fn sibling_exchange_without_sibling_is_unchanged() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        assert_eq!(
            exchange_check(
                &[],
                ours,
                Ok(None),
                Ok(Default::default()),
                Some(90),
                1_000_000,
            )
            .to_string(),
            "degraded sibling exchange: this device has no sibling in the roster"
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn sibling_exchange_names_checkout_as_missing_freshness_source() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(exchange_table(ours, &sibling, 12, 7))),
            Ok(Default::default()),
            None,
            1_000_000,
        );
        assert_eq!(check.state, State::Unknown, "{check}");
        assert!(check.detail.contains("checkout's pool policy"), "{check}");
        assert_eq!(
            check.cure.as_deref(),
            Some("restore the checkout's pool policy")
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn sibling_exchange_needs_a_folded_head_that_acknowledges_ours() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let siblings = vec![sibling.clone()];
        let unsaved = exchange_check(
            &siblings,
            ours,
            Ok(None),
            Ok(Default::default()),
            Some(90),
            1_000_000,
        );
        assert_eq!(unsaved.state, State::Unknown);
        assert_eq!(
            unsaved.cure.as_deref(),
            Some("start the peer on demand: epr peer serve")
        );
        // Known endpoints and an empty table: nothing exchanged, never ok.
        let mut table = eprfs_peer::HeadTable::default();
        let silent = exchange_check(
            &siblings,
            ours,
            Ok(Some(table.clone())),
            Ok(Default::default()),
            Some(90),
            1_000_000,
        );
        assert_eq!(silent.state, State::Degraded);
        assert!(silent.detail.contains("no verified head"), "{silent}");
        const NOW_MS: u64 = 1_000_000;
        let mut recovery = eprfs_peer::PeerRecoveryStatus::default();
        recovery.devices.insert(
            sibling.clone(),
            eprfs_peer::DeviceRecoveryStatus {
                last_heard_at: Some(NOW_MS - 5_000),
                ..Default::default()
            },
        );
        let mut head = eprfs_peer::DeviceHead {
            seq: 4,
            root: eprfs_core::BlobCid::compute_bytes(b"sibling root"),
            at: NOW_MS,
            folded: std::collections::BTreeMap::new(),
            folded_at: NOW_MS - 90_000,
            events_folded: 0,
            nack: Default::default(),
            coverage: None,
        };
        table.devices.insert(sibling.clone(), head.clone());
        let unacked = exchange_check(
            &siblings,
            ours,
            Ok(Some(table.clone())),
            Ok(recovery.clone()),
            Some(90),
            NOW_MS,
        );
        assert_eq!(unacked.state, State::Degraded);
        assert!(unacked.detail.contains("acknowledges none"), "{unacked}");

        head.folded.insert(ours.to_owned(), 2);
        table.devices.insert(sibling.clone(), head);
        let exchanging = exchange_check(
            &siblings,
            ours,
            Ok(Some(table)),
            Ok(recovery),
            Some(90),
            NOW_MS,
        );
        assert_eq!(exchanging.state, State::Ok, "{exchanging}");
        assert!(exchanging.detail.contains(&actor::short_did(&sibling)));
        assert!(exchanging.detail.contains("to seq 2"));
        assert!(exchanging.detail.contains("HEARD 5s ago"), "{exchanging}");
        assert!(exchanging.detail.contains("FOLDED 90s ago"), "{exchanging}");

        assert_eq!(
            exchange_check(
                &["did:key:z6MkSomeoneElse000000000000000000000".into()],
                ours,
                Ok(Some(eprfs_peer::HeadTable::default())),
                Ok(Default::default()),
                Some(90),
                NOW_MS,
            )
            .state,
            State::Degraded
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn sibling_exchange_degrades_when_acknowledgement_carrier_is_stale() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(exchange_table(ours, &sibling, 91, 7))),
            Ok(Default::default()),
            Some(90),
            1_000_000,
        );
        assert_eq!(check.state, State::Degraded, "{check}");
        assert!(
            check.detail.contains("folded-from head is 91s old"),
            "{check}"
        );
        assert!(
            check
                .detail
                .contains("acknowledgement of this device's head was carried 91s ago"),
            "{check}"
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn fresh_heard_keeps_an_old_folded_carrier_live() {
        const NOW_MS: u64 = 1_000_000;
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let mut recovery = eprfs_peer::PeerRecoveryStatus::default();
        recovery.devices.insert(
            sibling.clone(),
            eprfs_peer::DeviceRecoveryStatus {
                last_heard_at: Some(NOW_MS - 5_000),
                ..Default::default()
            },
        );
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(exchange_table(ours, &sibling, 900, 7))),
            Ok(recovery),
            Some(90),
            NOW_MS,
        );
        assert_eq!(check.state, State::Ok, "{check}");
        assert!(check.detail.contains("HEARD 5s ago"), "{check}");
        assert!(check.detail.contains("FOLDED 900s ago"), "{check}");
    }

    #[cfg(feature = "peer")]
    #[test]
    fn idle_root_with_ack_one_round_behind_is_exchanging() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(exchange_table(ours, &sibling, 12, 6))),
            Ok(Default::default()),
            Some(90),
            1_000_000,
        );
        assert_eq!(check.state, State::Ok, "{check}");
        assert!(check.detail.contains("seq 6, carried 12s ago"), "{check}");
        assert!(!check.detail.contains("600s old"), "{check}");
    }

    #[cfg(feature = "peer")]
    #[test]
    fn sibling_exchange_is_ok_when_fold_and_ack_are_fresh() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(exchange_table(ours, &sibling, 12, 7))),
            Ok(Default::default()),
            Some(90),
            1_000_000,
        );
        assert_eq!(check.state, State::Ok, "{check}");
        assert!(check.detail.contains("both directions"), "{check}");
        assert!(check.detail.contains("12s old"), "{check}");
    }

    #[cfg(feature = "peer")]
    #[test]
    fn sibling_exchange_requires_acknowledgement_of_latest_root() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(exchange_table(ours, &sibling, 12, 0))),
            Ok(Default::default()),
            Some(90),
            1_000_000,
        );
        assert_eq!(check.state, State::Degraded, "{check}");
        assert!(
            check
                .detail
                .contains("only to seq 0, before its latest announced root at seq 1"),
            "{check}"
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn recent_root_does_not_require_the_next_acknowledgement_round() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(exchange_table_with_root(ours, &sibling, 12, 6, 7, 5))),
            Ok(Default::default()),
            Some(90),
            1_000_000,
        );
        assert_eq!(check.state, State::Ok, "{check}");
        assert!(check.detail.contains("seq 6, carried 12s ago"), "{check}");
    }

    #[cfg(feature = "peer")]
    #[test]
    fn mature_root_requires_acknowledgement_of_its_sequence() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(exchange_table_with_root(
                ours, &sibling, 12, 6, 7, 200,
            ))),
            Ok(Default::default()),
            Some(90),
            1_000_000,
        );
        assert_eq!(check.state, State::Degraded, "{check}");
        assert!(
            check
                .detail
                .contains("only to seq 6, before its latest announced root at seq 7"),
            "{check}"
        );
    }

    #[cfg(feature = "peer")]
    #[test]
    fn stalled_carrier_names_its_age_before_the_sequence_lag() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let sibling = "did:key:z6MkSiblingDevice0000000000000000000".to_owned();
        let mut table = exchange_table_with_root(ours, &sibling, 12, 6, 7, 200);
        table
            .devices
            .get_mut(&sibling)
            .expect("sibling head")
            .folded_at = 10_000_000 - 8_667_000;
        let check = exchange_check(
            std::slice::from_ref(&sibling),
            ours,
            Ok(Some(table)),
            Ok(Default::default()),
            Some(90),
            10_000_000,
        );
        assert_eq!(check.state, State::Degraded, "{check}");
        let age = check
            .detail
            .find("folded-from head is 8667s old")
            .expect("carrier age");
        let sequence = check
            .detail
            .find("only to seq 6, before its latest announced root at seq 7")
            .expect("sequence lag");
        assert!(age < sequence, "{check}");
    }
    #[cfg(feature = "peer")]
    #[test]
    fn an_admitted_device_that_never_acknowledges_is_named_and_the_next_step_holds() {
        let ours = "did:key:z6MkThisDevice000000000000000000000";
        let exchanging = "did:key:z6MkExchangingSibling00000000000000".to_owned();
        let laptop = "did:key:z6MkgHjuLaptopNeverHeard000000000xMbu".to_owned();
        let behind = "did:key:z6MkThreeSeqsBehind0000000000000000".to_owned();
        let head = |acked: Option<u64>| eprfs_peer::DeviceHead {
            seq: 7,
            root: eprfs_core::BlobCid::compute_bytes(b"sibling root"),
            at: 0,
            folded: acked
                .map(|seq| std::collections::BTreeMap::from([(ours.to_owned(), seq)]))
                .unwrap_or_default(),
            folded_at: 0,
            events_folded: 0,
            nack: Default::default(),
            coverage: None,
        };
        let mut table = eprfs_peer::HeadTable {
            own: Some(eprfs_peer::ChannelHead {
                device: ours.to_owned(),
                root: eprfs_core::BlobCid::compute_bytes(b"own root"),
                seq: 40,
                at: 0,
                folded: std::collections::BTreeMap::new(),
                signature: String::new(),
            }),
            ..Default::default()
        };
        table.devices.insert(exchanging.clone(), head(Some(40)));
        table.devices.insert(behind.clone(), head(Some(37)));
        // The laptop is admitted (it is in the roster) and no head of it has been folded.
        let siblings = vec![exchanging.clone(), laptop.clone(), behind.clone()];
        let named = unacknowledged(&siblings, ours, &table);
        assert_eq!(named.len(), 1, "{named:?}");
        let line = named[0].to_string();
        assert!(
            line.starts_with(&format!(
                "degraded sibling exchange: {} is admitted and has acknowledged none of this \
                 device's records",
                actor::short_did(&laptop)
            )),
            "{line}"
        );
        assert!(line.contains("did:key:z6MkgHju…xMbu"), "{line}");
        assert!(
            line.contains("a landing waits for it at the lease step"),
            "{line}"
        );
        assert!(line.contains(&format!(
            "run `just status device` on {}",
            actor::short_did(&laptop)
        )));
        assert_eq!(named[0].state, State::Degraded, "never refused");
        // A head that was folded but acknowledges nothing is named too, with its seq.
        table.devices.insert(laptop.clone(), head(None));
        let folded = unacknowledged(&siblings, ours, &table);
        assert_eq!(folded.len(), 1);
        assert!(folded[0].detail.contains("its head seq 7 was folded here"));
        // Eleven seqs behind is named; three behind (above) is not.
        table.devices.insert(laptop.clone(), head(Some(29)));
        let trailing = unacknowledged(&siblings, ours, &table);
        assert_eq!(trailing.len(), 1);
        assert!(trailing[0].detail.contains("only to seq 29 of 40"));
        table.devices.insert(laptop.clone(), head(Some(40)));
        assert!(unacknowledged(&siblings, ours, &table).is_empty());

        // An otherwise arrived device keeps its next step with the degraded line beside it.
        let ok = |name: &str| Check::new(name, State::Ok, format!("{name} evidence"), None);
        let arrived = vec![
            ok("epr binary"),
            ok("device key"),
            ok(ROSTER_CHECK),
            ok("peer daemon"),
            ok(SIBLING_CHECK),
        ];
        let before = next_step(&arrived).to_string();
        let mut with_line = arrived.clone();
        with_line.extend(named);
        assert_eq!(next_step(&with_line).to_string(), before);
        assert_eq!(
            before,
            "ok next step: nothing owed; this device is arrived and exchanging"
        );
    }
}
