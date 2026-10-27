//! `epr peer` — this device as one of its person's devices on the steward channel (spec
//! `2026-10-23-workspaces-cooperate-as-devices-design.md` §3.2–§3.3).
//!
//! - `id` prints the node id this device's peer answers on, minting the iroh key on first use, so
//!   `epr actor device endpoint --node-id` can name it on the roster.
//! - `serve` runs the device peer: the tier ALPN over this device's channel tree and the steward
//!   topic's gossip, both answering admitted devices only. Long-running; declared as an on-demand
//!   command, never started at workspace boot.
//! - `status` prints one line per admitted device: its last folded head, its age, and how many
//!   events were folded from it, read from the head table the daemon persists.
//! - `leases` prints the steward lease view (§3.4): for each steward-scoped resource a live lease
//!   names, whether this device's lease is effective, pending or unconfirmed, or another device
//!   holds it. It reads the flow store and the head table only, never the network, so it answers
//!   while `serve` runs; berth polls it to settle a steward-scoped claim.
//!
//! The iroh key lives at `$XDG_CONFIG_HOME/epr/peer/iroh.key` (mode 0600). Neither it nor the
//! device key is ever printed, logged or copied: the device key is reached only as a signer.

use std::fmt;
use std::net::SocketAddrV4;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use elohim_epr_rea::ParticipantRow;
use eprfs_peer::{
    lease_events, other_devices, peer_from_arg, relay_from_env, roster_rows_from_file,
    steward_lease_view, Admission, CrossingAllowList, HeadTable, LeaseVerdict, NodeAddr,
    PeerConfig, PeerDaemon, PeerError, PeerKey, StewardScope,
};
use serde::Serialize;

use crate::actor::{
    channel_standing, steward_lineage, ActorError, ChannelStanding, StewardLineage,
};
use crate::device_key::{self, DeviceKey};

/// Where the crossing allow-list is read from, under the checkout.
pub const POLICY_REL: &str = "genesis/agentic/pool-policy.json";

/// Why an `epr peer` verb could not run.
#[derive(Debug, thiserror::Error)]
pub enum PeerCliError {
    /// The device bin store could not observe this daemon.
    #[error("{0}")]
    BinStore(#[from] elohim_device_runtime::bin_store::Refusal),

    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("{0}")]
    Peer(#[from] PeerError),

    #[error("{0}")]
    Actor(#[from] ActorError),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// This device's `epr peer serve` holds its serve lock.
    #[error(
        "epr peer serve (or another epr flow fold) is running for this device (it holds `{}`); \
         a serving daemon folds every newer head on its own — stop it, or let it fold. A second \
         endpoint on the same node id would confuse the other devices' path to this one",
        .0.display()
    )]
    Serving(PathBuf),
}

pub type PeerResult<T> = Result<T, PeerCliError>;

pub fn usage() -> String {
    "usage: epr peer <\n  \
     id [--json]\n  \
     | serve [--root DIR] [--handle H] [--listen IP:PORT] [--peer NODE_ID@IP:PORT]... \
     [--policy PATH] \
     (the device peer: the tier ALPN over this device's channel tree and the steward topic, \
     admitted devices only; runs until stopped. EPR_PEER_RELAY_URL homes it to a relay; \
     --peer names a device's direct address when there is none)\n  \
     | status [--root DIR] [--handle H] [--json] \
     (one line per admitted device: its last folded head, its age and the events folded)\n  \
     | leases [--root DIR] [--handle H] [--policy PATH] [--json] \
     (the steward lease view: per steward-scoped resource, effective | pending | unconfirmed | \
     held_by, read from the flow store and the head table; exits non-zero only when it cannot \
     be computed)>"
        .to_string()
}

/// Entry point for `epr peer …`.
pub fn run(args: &[String]) -> PeerResult<ExitCode> {
    match args.first().map(String::as_str) {
        Some("--help" | "-h" | "help") => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        Some("id") => id(&Options::parse(&args[1..])?),
        Some("serve") => serve(Options::parse(&args[1..])?),
        Some("status") => status(&Options::parse(&args[1..])?),
        Some("leases") => leases(&Options::parse(&args[1..])?),
        Some(other) => Err(PeerCliError::InvalidArguments(format!(
            "unknown peer verb `{other}`{}\n{}",
            crate::hint::did_you_mean(other, "epr peer", &["id", "serve", "status", "leases"]),
            usage()
        ))),
        None => Err(PeerCliError::InvalidArguments(usage())),
    }
}

/// The options every `epr peer` verb (and `epr flow fold`) reads.
#[derive(Debug, Default)]
pub struct Options {
    pub root: PathBuf,
    pub handle: Option<String>,
    pub listen: Option<SocketAddrV4>,
    pub peers: Vec<String>,
    pub policy: Option<PathBuf>,
    pub json: bool,
}

impl Options {
    fn parse(args: &[String]) -> PeerResult<Options> {
        let mut options = Options {
            root: PathBuf::from("."),
            ..Options::default()
        };
        let mut i = 0;
        while i < args.len() {
            let flag = args[i].as_str();
            if flag == "--json" {
                options.json = true;
                i += 1;
                continue;
            }
            let value = args.get(i + 1).cloned().ok_or_else(|| {
                PeerCliError::InvalidArguments(format!("{flag} needs a value\n{}", usage()))
            })?;
            match flag {
                "--root" => options.root = PathBuf::from(value),
                "--handle" => options.handle = Some(value),
                "--policy" => options.policy = Some(PathBuf::from(value)),
                "--peer" => options.peers.push(value),
                "--listen" => {
                    options.listen = Some(value.parse().map_err(|e| {
                        PeerCliError::InvalidArguments(format!("--listen {value}: {e}"))
                    })?);
                }
                other => {
                    return Err(PeerCliError::InvalidArguments(format!(
                        "unknown peer argument `{other}`\n{}",
                        usage()
                    )))
                }
            }
            i += 2;
        }
        options.root = std::fs::canonicalize(&options.root).unwrap_or(options.root);
        Ok(options)
    }
}

/// `$XDG_CONFIG_HOME/epr/peer` (or `$HOME/.config/epr/peer`): the iroh key and the head table.
pub fn peer_dir() -> PeerResult<PathBuf> {
    let non_empty = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let config_home = match non_empty("XDG_CONFIG_HOME").map(PathBuf::from) {
        Some(xdg) if xdg.is_absolute() => xdg,
        _ => non_empty("HOME")
            .map(|home| PathBuf::from(home).join(".config"))
            .ok_or_else(|| {
                PeerCliError::InvalidArguments(
                    "no peer directory: XDG_CONFIG_HOME and HOME are both unset".into(),
                )
            })?,
    };
    Ok(config_home.join("epr").join("peer"))
}

/// The lock `epr peer serve` holds on `<peer dir>/serve.lock` for its whole life, so no second
/// endpoint ever answers on this device's node id beside it: `epr flow fold` and a second
/// `serve` refuse while it is held. Released when dropped (and by the kernel when the process
/// ends, however it ends).
#[derive(Debug)]
pub struct ServeLock {
    _file: std::fs::File,
}

impl ServeLock {
    /// The lock file's name inside the peer directory.
    pub const FILE_NAME: &'static str = "serve.lock";

    /// Take the lock without waiting.
    ///
    /// # Errors
    ///
    /// [`PeerCliError::Serving`] when another process holds it; [`PeerCliError::Io`] when the
    /// file cannot be opened or locked.
    pub fn acquire(peer_dir: &Path) -> PeerResult<ServeLock> {
        std::fs::create_dir_all(peer_dir)?;
        let path = peer_dir.join(Self::FILE_NAME);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(ServeLock { _file: file }),
            Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => {
                Err(PeerCliError::Serving(path))
            }
            Err(errno) => Err(PeerCliError::Io(errno.into())),
        }
    }

    /// Whether a daemon holds this device's serve lock now: `Some(true)` when it is held,
    /// `Some(false)` when it is free, `None` when the lock could not be probed. The probe takes
    /// the lock without waiting and drops it at once, so it never keeps a daemon from starting.
    #[must_use]
    pub fn held(peer_dir: &Path) -> Option<bool> {
        match ServeLock::acquire(peer_dir) {
            Ok(_probe) => Some(false),
            Err(PeerCliError::Serving(_)) => Some(true),
            Err(_) => None,
        }
    }
}

/// This device as a member of its lineage: its device key, its lineage and its peer directory.
pub struct Device {
    pub key: DeviceKey,
    pub lineage: StewardLineage,
    pub peer_dir: PathBuf,
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("did", &self.key.did_key())
            .field("lineage", &self.lineage)
            .finish_non_exhaustive()
    }
}

impl Device {
    /// This device's existing key (never minted here), the lineage it stands on in the checkout
    /// at `root`, and its peer directory.
    pub fn open(root: &Path, handle: Option<&str>) -> PeerResult<Device> {
        let key_file = device_key::resolve_path()?;
        let key = DeviceKey::load(&key_file).map_err(|e| {
            PeerCliError::InvalidArguments(format!(
                "this device has no device key ({e}) — enroll it first with `epr actor witness` \
                 or `epr actor device enroll`"
            ))
        })?;
        let lineage = steward_lineage(root, handle, &key.did_key(), &key_file)?;
        Ok(Device {
            key,
            lineage,
            peer_dir: peer_dir()?,
        })
    }

    /// Admission for this lineage, read from the roster in this checkout.
    pub fn admission(&self) -> PeerResult<Admission> {
        let rows = roster_rows_from_file(&self.lineage.roster)?;
        Ok(Admission::from_roster(
            &rows,
            &self.lineage.chain_root,
            &verifier(),
        ))
    }
}

/// The device-key verifier heads and roster rows are checked with.
pub fn verifier() -> impl Fn(&str, &[u8], &[u8]) -> bool + Send + Sync + 'static {
    |did: &str, message: &[u8], signature: &[u8]| device_key::verify(did, message, signature)
}

/// The crossing allow-list `options` names, or the checkout's pool-policy.json.
pub fn allow_list(options: &Options) -> PeerResult<CrossingAllowList> {
    let path = options
        .policy
        .clone()
        .unwrap_or_else(|| options.root.join(POLICY_REL));
    Ok(CrossingAllowList::from_policy_file(&path)?)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IdOutcome {
    node_id: String,
    key_dir: PathBuf,
}

fn id(options: &Options) -> PeerResult<ExitCode> {
    let dir = peer_dir()?;
    let key = PeerKey::load_or_generate(&dir)?;
    if options.json {
        let outcome = IdOutcome {
            node_id: key.node_id().to_string(),
            key_dir: dir,
        };
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        println!("{}", key.node_id());
    }
    Ok(ExitCode::SUCCESS)
}

fn runtime() -> PeerResult<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

fn serve(options: Options) -> PeerResult<ExitCode> {
    crate::bin::daemon_store()?.record_running_daemon(
        &std::env::current_exe()?,
        std::process::id(),
        &crate::bin::daemon_proc_root(),
        std::env::var_os("EPR_BIN")
            .as_deref()
            .map(std::path::Path::new),
    )?;
    let device = Device::open(&options.root, options.handle.as_deref())?;
    let _serving = ServeLock::acquire(&device.peer_dir)?;
    let allow = allow_list(&options)?;
    if allow.is_empty() {
        eprintln!(
            "epr peer: the crossing allow-list is empty — nothing of this device crosses until \
             {POLICY_REL} declares berth.channel.crosses"
        );
    }
    let peers = options
        .peers
        .iter()
        .map(|arg| peer_from_arg(arg))
        .collect::<Result<Vec<_>, _>>()?;
    let key = PeerKey::load_or_generate(&device.peer_dir)?;
    let did = device.key.did_key();
    let signer_key = device.key;
    let config = PeerConfig {
        key,
        device: did.clone(),
        chain_root: device.lineage.chain_root.clone(),
        flow_root: options.root.clone(),
        roster: device.lineage.roster.clone(),
        state_dir: device.peer_dir.clone(),
        allow,
        relay: relay_from_env()?,
        listen: options.listen,
        peers,
        signer: Arc::new(move |message: &[u8]| signer_key.sign(message)),
        verifier: Arc::new(verifier()),
        log: Arc::new(|line: &str| eprintln!("epr peer: {line}")),
    };
    let runtime = runtime()?;
    runtime.block_on(async move {
        let mut daemon = PeerDaemon::start(config).await?;
        let sockets: Vec<String> = daemon
            .bound_sockets()
            .iter()
            .map(ToString::to_string)
            .collect();
        eprintln!(
            "epr peer: serving {} as {} on {} for {}'s devices",
            short(&did),
            daemon.node_id(),
            sockets.join(", "),
            device.lineage.handle
        );
        // The loop ends on its own only for a reason worth a non-zero exit; it is named.
        let ended = daemon.wait().await;
        daemon.shutdown().await;
        ended.map_err(PeerCliError::Peer)
    })?;
    Ok(ExitCode::SUCCESS)
}

/// One admitted device as `status` reports it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStatus {
    device: String,
    node_id: String,
    seq: Option<u64>,
    root: Option<String>,
    /// Seconds since this device folded the head (this device's clock).
    age_s: Option<u64>,
    events_folded: u64,
    /// The highest seq of this device's own heads that device has acknowledged folding.
    acked: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusOutcome {
    cli_cid: Option<String>,
    daemon_cids: Vec<String>,
    daemon_differs: bool,
    handle: String,
    chain_root: String,
    device: String,
    node_id: Option<String>,
    own_seq: Option<u64>,
    devices: Vec<DeviceStatus>,
    dropped: eprfs_peer::DroppedHeads,
    channel: eprfs_peer::ChannelCounters,
    /// Connections this device's daemon refused, per node id, with the device the roster names
    /// for it (a contested device's old endpoint row still names it); `None` for a stranger.
    refused_connections: Vec<RefusedConnections>,
    /// Connections refused from node ids the roster does not name, in one total.
    refused_strangers: u64,
}

/// One node id's refused connections, as `status` reports them.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefusedConnections {
    node_id: String,
    device: Option<String>,
    count: u64,
}

fn status(options: &Options) -> PeerResult<ExitCode> {
    let device = Device::open(&options.root, options.handle.as_deref())?;
    let admission = device.admission()?;
    let table = HeadTable::load(&device.peer_dir)?;
    let did = device.key.did_key();
    let now = unix_ms();
    let mut devices = Vec::new();
    let mut node_id = None;
    for node in admission.nodes() {
        let Some(member) = admission.admits(&node) else {
            continue;
        };
        if member == did {
            node_id = Some(node.to_string());
            continue;
        }
        let head = table.devices.get(member);
        devices.push(DeviceStatus {
            device: member.to_string(),
            node_id: node.to_string(),
            seq: head.map(|h| h.seq),
            root: head.map(|h| h.root.to_string()),
            age_s: head.map(|h| now.saturating_sub(h.folded_at) / 1000),
            events_folded: head.map_or(0, |h| h.events_folded),
            acked: head.and_then(|h| h.folded.get(&did).copied()),
        });
    }
    let rows = roster_rows_from_file(&device.lineage.roster)?;
    let refused_connections = table
        .channel
        .refused_connections
        .iter()
        .map(|(node, count)| RefusedConnections {
            node_id: node.clone(),
            device: rows.iter().find_map(|row| match row {
                ParticipantRow::PeerEndpoint {
                    device, node_id, ..
                } if node_id == node => Some(device.clone()),
                _ => None,
            }),
            count: *count,
        })
        .collect();
    let refused_strangers = table.channel.refused_strangers;
    let cli_cid = std::env::current_exe()
        .ok()
        .and_then(|path| crate::bin::daemon_store().ok()?.executable_cid(&path).ok());
    let daemon_cids: Vec<_> = crate::bin::daemon_store()?
        .live_daemons(&crate::bin::daemon_proc_root())?
        .into_iter()
        .map(|pin| pin.running_cid.unwrap_or(pin.cid))
        .collect();
    let daemon_differs = daemon_cids.iter().any(|cid| Some(cid) != cli_cid.as_ref());
    let outcome = StatusOutcome {
        cli_cid,
        daemon_cids,
        daemon_differs,
        refused_strangers,
        handle: device.lineage.handle.clone(),
        chain_root: device.lineage.chain_root.clone(),
        device: did,
        node_id,
        own_seq: table.own.as_ref().map(|own| own.seq),
        devices,
        dropped: table.dropped,
        channel: table.channel,
        refused_connections,
    };
    if options.json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        render(&outcome);
    }
    Ok(ExitCode::SUCCESS)
}

fn render(outcome: &StatusOutcome) {
    println!(
        "CLI CID: {}",
        outcome.cli_cid.as_deref().unwrap_or("unknown")
    );
    println!(
        "peer daemon CID: {}{}",
        if outcome.daemon_cids.is_empty() {
            "no daemon running".into()
        } else {
            outcome.daemon_cids.join(", ")
        },
        if outcome.daemon_differs {
            " (differs from this CLI)"
        } else {
            ""
        }
    );
    println!(
        "this device  {}  head seq {}  ({}'s devices)",
        short(&outcome.device),
        outcome
            .own_seq
            .map_or_else(|| "none yet".to_string(), |seq| seq.to_string()),
        outcome.handle
    );
    if outcome.devices.is_empty() {
        println!("no other device is admitted on this roster");
    }
    for device in &outcome.devices {
        match (device.seq, &device.root, device.age_s) {
            (Some(seq), Some(root), Some(age)) => println!(
                "device  {}  head seq {seq}  root {}  {age}s ago  {} events folded  acks ours to \
                 seq {}",
                short(&device.device),
                short(root),
                device.events_folded,
                device
                    .acked
                    .map_or_else(|| "none".to_string(), |seq| seq.to_string())
            ),
            _ => println!("device  {}  no head folded yet", short(&device.device)),
        }
    }
    let d = outcome.dropped;
    println!(
        "dropped heads  unadmitted {}  forged {}  rollback {}  replayed {}  malformed {}",
        d.unadmitted, d.forged, d.rollback, d.replayed, d.malformed
    );
    if !outcome.refused_connections.is_empty() || outcome.refused_strangers > 0 {
        let mut refused: Vec<String> = outcome
            .refused_connections
            .iter()
            .map(|r| {
                let who = r
                    .device
                    .as_deref()
                    .map_or_else(|| format!("stranger {}", short(&r.node_id)), short);
                format!("{who}: {}", r.count)
            })
            .collect();
        if outcome.refused_strangers > 0 {
            refused.push(format!("strangers: {}", outcome.refused_strangers));
        }
        println!("refused connections  {}", refused.join("  "));
    }
    let c = &outcome.channel;
    if let Some(error) = &c.last_projection_error {
        println!(
            "channel NOT PROJECTING ({} failures): {error} — nothing new of this device crosses",
            c.projection_failures
        );
    }
    if c.fold_failures + c.oversized_events + c.over_budget_folds > 0 {
        println!(
            "folds  failed {}  oversized events {}  over budget {}",
            c.fold_failures, c.oversized_events, c.over_budget_folds
        );
    }
}

/// `epr peer leases`: the steward lease view for this device, now.
///
/// It reads the channel store `epr peer serve` recorded in the head table (the checkout at
/// `--root` only when no daemon has served on this device yet), never creating one: a recorded
/// store that is gone is an error. The output says how this workspace stands first: `device`
/// (with the view), `not_a_device` (no device key, not on a roster here, or no endpoint row: its
/// steward-scoped resources are container-scoped) or `revoked` (its binding is contested). It
/// exits non-zero only when the view could not be computed.
fn leases(options: &Options) -> PeerResult<ExitCode> {
    let table = HeadTable::load(&peer_dir()?)?;
    let root = table
        .store
        .as_ref()
        .map_or_else(|| options.root.clone(), |place| place.root.clone());
    let policy = options
        .policy
        .clone()
        .unwrap_or_else(|| options.root.join(POLICY_REL));
    let scope = StewardScope::from_policy_file(&policy)?;
    // A recorded channel store that is gone is never read as an empty (free) one.
    if let Some(place) = table.store.as_ref().filter(|place| !place.log.is_file()) {
        return Err(PeerCliError::InvalidArguments(format!(
            "the channel store `{}` that epr peer serve recorded is gone; restart epr peer serve \
             on this device's checkout",
            place.log.display()
        )));
    }
    let now = unix_ms();
    let mut out = serde_json::Map::new();
    out.insert("now".into(), now.into());
    out.insert("confirm_wait_s".into(), scope.confirm_wait_s().into());
    let standing = match standing(&root, options.handle.as_deref())? {
        Standing::Device(device) => device,
        Standing::NotADevice(why) => {
            return answer(options, out, "not_a_device", None, &why);
        }
        Standing::Revoked(did) => {
            return answer(
                options,
                out,
                "revoked",
                Some(did),
                "this device's binding is contested",
            );
        }
    };
    let (device, admission) = *standing;
    let did = device.key.did_key();
    if !admission.admits_device(&did) {
        return answer(
            options,
            out,
            "not_a_device",
            Some(did),
            "no PeerEndpoint row names this device's peer (run: epr actor device endpoint)",
        );
    }
    let log = root.join(".eprfs/status/flows.jsonl");
    if !log.is_file() {
        return Err(PeerCliError::InvalidArguments(format!(
            "this checkout has no flow store at `{}`; nothing of this device's channel is here",
            log.display()
        )));
    }
    let store = elohim_epr_rea::SidecarFlowStore::open(&root)
        .map_err(|e| PeerCliError::Peer(PeerError::FlowStore(e)))?;
    let events = lease_events(&store)?;
    let mut view = steward_lease_view(
        &did,
        &events,
        &table,
        &admission,
        now,
        scope.confirm_wait_s(),
    )?;
    // The other devices as this one has folded them (berth status lists their moorings beside
    // the leases above), replayed in each device's own order, as the leases are.
    let others = other_devices(&did, &events, &table, &admission)?;
    view.retain(|resource, _| scope.covers(resource));
    let seen: serde_json::Map<String, serde_json::Value> = table
        .devices
        .iter()
        .filter(|(other, _)| admission.admits_device(other))
        .map(|(other, head)| (other.clone(), head.seq.into()))
        .collect();
    out.insert("standing".into(), "device".into());
    out.insert("device".into(), did.into());
    out.insert(
        "store".into(),
        std::fs::canonicalize(&root)
            .unwrap_or(root)
            .display()
            .to_string()
            .into(),
    );
    out.insert("seen".into(), seen.into());
    out.insert("others".into(), serde_json::to_value(others)?);
    // Whether this device's own daemon runs: an unconfirmed claim with none names that cure.
    out.insert(
        "serving".into(),
        ServeLock::held(&device.peer_dir).map_or(serde_json::Value::Null, Into::into),
    );
    if options.json {
        for (resource, verdict) in view {
            out.insert(resource, serde_json::to_value(verdict)?);
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(ExitCode::SUCCESS);
    }
    if view.is_empty() {
        println!("no live lease on a steward-scoped resource");
    }
    for (resource, verdict) in &view {
        let line = match verdict {
            LeaseVerdict::Effective { session } => format!("effective, held by {session} here"),
            LeaseVerdict::Pending {
                session,
                waiting_on,
            } => format!(
                "pending for {session} here, waiting on {}",
                waiting_on
                    .iter()
                    .map(|device| short(device))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            // berth formats the refusal line and its age; this line only names the device.
            LeaseVerdict::Unconfirmed {
                session, device, ..
            } => format!("unconfirmed for {session} here: {}", short(device)),
            LeaseVerdict::RefusedBy { session, device } => {
                format!("refused for {session} here by {}", short(device))
            }
            LeaseVerdict::HeldBy {
                device,
                session,
                overriding,
                ..
            } => format!(
                "held by {}/{session}{}",
                short(device),
                if *overriding {
                    " (an override of this device's silence)"
                } else {
                    ""
                }
            ),
        };
        println!("{resource:<18} {line}");
    }
    Ok(ExitCode::SUCCESS)
}

/// How this workspace stands for the lease view.
enum Standing {
    /// A device of its steward, with the admission its roster gives.
    Device(Box<(Device, Admission)>),
    NotADevice(String),
    Revoked(String),
}

fn standing(root: &Path, handle: Option<&str>) -> PeerResult<Standing> {
    let key_file = device_key::resolve_path()?;
    if !key_file.is_file() {
        return Ok(Standing::NotADevice(format!(
            "this workspace has no device key ({})",
            key_file.display()
        )));
    }
    let did = DeviceKey::load(&key_file)?.did_key();
    match channel_standing(root, handle, &did)? {
        ChannelStanding::NotADevice(why) => Ok(Standing::NotADevice(why)),
        ChannelStanding::Revoked { .. } => Ok(Standing::Revoked(did)),
        ChannelStanding::Stands { handle } => {
            let device = Device::open(root, Some(&handle))?;
            let admission = device.admission()?;
            Ok(Standing::Device(Box::new((device, admission))))
        }
    }
}

/// The answer for a workspace that has no view: how it stands and why.
fn answer(
    options: &Options,
    mut out: serde_json::Map<String, serde_json::Value>,
    standing: &str,
    device: Option<String>,
    why: &str,
) -> PeerResult<ExitCode> {
    out.insert("standing".into(), standing.into());
    out.insert("device".into(), device.into());
    out.insert("store".into(), serde_json::Value::Null);
    out.insert("why".into(), why.into());
    if options.json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{standing}: {why}");
    }
    Ok(ExitCode::SUCCESS)
}

/// A did:key or CID shortened for a status line (both are ASCII).
fn short(text: &str) -> String {
    crate::actor::short_did(text)
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// `epr flow fold --from <device> --root <cid>`: fold one channel root from one admitted device
/// into this device's flow store, once. The daemon folds every newer head the same way.
pub fn fold(options: &Options, from: &str, root: &str) -> PeerResult<eprfs_peer::FoldReport> {
    let device = Device::open(&options.root, options.handle.as_deref())?;
    // Held for the fold, so no `serve` starts on this node id meanwhile either.
    let _not_serving = ServeLock::acquire(&device.peer_dir)?;
    let admission = device.admission()?;
    let node = admission.node_for(from).ok_or_else(|| {
        PeerCliError::InvalidArguments(format!(
            "{from} is not an admitted device of {}'s lineage — nothing is folded from it",
            device.lineage.handle
        ))
    })?;
    let root = eprfs_core::BlobCid::parse(root)
        .map_err(|e| PeerCliError::InvalidArguments(format!("--root {root}: {e}")))?;
    let relay = relay_from_env()?;
    let mut peer = None;
    for arg in &options.peers {
        let addr = peer_from_arg(arg)?;
        if addr.node_id == node {
            peer = Some(addr);
        }
    }
    let peer = match (peer, &relay) {
        (Some(peer), _) => peer,
        (None, Some(relay)) => NodeAddr::new(node).with_relay_url(relay.clone().into()),
        (None, None) => {
            return Err(PeerCliError::InvalidArguments(format!(
                "no address for {from}: set EPR_PEER_RELAY_URL or pass --peer <node_id>@<ip:port>"
            )))
        }
    };
    let allow = allow_list(options)?;
    let since = HeadTable::load(&device.peer_dir)?
        .devices
        .get(from)
        .map(|head| head.root.clone());
    let key = PeerKey::load_or_generate(&device.peer_dir)?;
    let store = elohim_epr_rea::SidecarFlowStore::open(&options.root)
        .map_err(|e| PeerCliError::Peer(PeerError::FlowStore(e)))?;
    let reader = device.key.did_key();
    let state_dir = device.peer_dir.clone();
    let runtime = runtime()?;
    let report = runtime.block_on(async move {
        let endpoint = eprfs_peer::bind(&key, relay, vec![]).await?;
        let target = eprfs_peer::FoldTarget {
            device: from,
            since: since.as_ref(),
            root: &root,
            // Admission was read from this checkout's roster at the start of this one-shot fold.
            admission: None,
            reader: eprfs_peer::FoldReader {
                device: &reader,
                state_dir: &state_dir,
            },
        };
        let folded = eprfs_peer::fold_root(
            &endpoint,
            peer,
            target,
            &store,
            &allow,
            &eprfs_core::TreeLimits::default(),
            &eprfs_peer::FoldLimits::default(),
        )
        .await;
        endpoint.close().await;
        Ok::<_, PeerCliError>(folded?)
    })?;
    let mut table = HeadTable::load(&device.peer_dir)?;
    let nacks = table.nacks.entry(from.to_string()).or_default();
    nacks
        .refused
        .extend(report.outcome.refused_cids.iter().map(ToString::to_string));
    nacks
        .quarantined
        .extend(report.quarantined.iter().map(|entry| entry.cid.to_string()));
    table.save(&device.peer_dir)?;
    Ok(report)
}

#[cfg(test)]
mod status_contract {
    #[test]
    fn peer_machine_status_contract_includes_daemon_identity() {
        use super::*;
        let cid = eprfs_core::BlobCid::compute_raw(b"daemon").to_string();
        let value = serde_json::to_value(StatusOutcome {
            cli_cid: Some(eprfs_core::BlobCid::compute_raw(b"cli").to_string()),
            daemon_cids: vec![cid],
            daemon_differs: true,
            handle: "fixture".into(),
            chain_root: "fixture".into(),
            device: "device".into(),
            node_id: None,
            own_seq: None,
            devices: vec![DeviceStatus {
                device: "other".into(),
                node_id: "node".into(),
                seq: Some(1),
                root: None,
                age_s: None,
                events_folded: 0,
                acked: None,
            }],
            dropped: Default::default(),
            channel: Default::default(),
            refused_connections: vec![RefusedConnections {
                node_id: "stranger".into(),
                device: None,
                count: 1,
            }],
            refused_strangers: 1,
        })
        .expect("serialize status");
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../sdk/schemas/v1/views/peer-cli-status.schema.json"
        ))
        .expect("status schema");
        let validator = jsonschema::validator_for(&schema).expect("valid schema");
        validator
            .validate(&value)
            .expect("serialized status matches schema");
        for field in ["cliCid", "daemonCids", "daemonDiffers"] {
            let mut bad = value.clone();
            bad.as_object_mut().expect("object").remove(field);
            assert!(!validator.is_valid(&bad), "missing {field}");
        }
        let mut bad = value;
        bad["daemonCids"] = serde_json::json!(["sha256:legacy"]);
        assert!(!validator.is_valid(&bad));
    }
}
