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
//!
//! The iroh key lives at `$XDG_CONFIG_HOME/epr/peer/iroh.key` (mode 0600). Neither it nor the
//! device key is ever printed, logged or copied: the device key is reached only as a signer.

use std::fmt;
use std::net::SocketAddrV4;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use eprfs_peer::{
    peer_from_arg, relay_from_env, roster_rows_from_file, Admission, CrossingAllowList, HeadTable,
    NodeAddr, PeerConfig, PeerDaemon, PeerError, PeerKey,
};
use serde::Serialize;

use crate::actor::{steward_lineage, ActorError, StewardLineage};
use crate::device_key::{self, DeviceKey};

/// Where the crossing allow-list is read from, under the checkout.
pub const POLICY_REL: &str = "genesis/agentic/pool-policy.json";

/// Why an `epr peer` verb could not run.
#[derive(Debug, thiserror::Error)]
pub enum PeerCliError {
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
     (one line per admitted device: its last folded head, its age and the events folded)>"
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
        Some(other) => Err(PeerCliError::InvalidArguments(format!(
            "unknown peer verb `{other}`{}\n{}",
            crate::hint::did_you_mean(other, "epr peer", &["id", "serve", "status"]),
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
    let device = Device::open(&options.root, options.handle.as_deref())?;
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
        daemon.wait().await;
        daemon.shutdown().await;
        Ok::<_, PeerCliError>(())
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
    handle: String,
    chain_root: String,
    device: String,
    node_id: Option<String>,
    own_seq: Option<u64>,
    devices: Vec<DeviceStatus>,
    dropped: eprfs_peer::DroppedHeads,
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
    let outcome = StatusOutcome {
        handle: device.lineage.handle.clone(),
        chain_root: device.lineage.chain_root.clone(),
        device: did,
        node_id,
        own_seq: table.own.as_ref().map(|own| own.seq),
        devices,
        dropped: table.dropped,
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
pub fn fold(
    options: &Options,
    from: &str,
    root: &str,
) -> PeerResult<(eprfs_peer::FoldOutcome, bool)> {
    let device = Device::open(&options.root, options.handle.as_deref())?;
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
    let runtime = runtime()?;
    runtime.block_on(async move {
        let endpoint = eprfs_peer::bind(&key, relay, vec![]).await?;
        let target = eprfs_peer::FoldTarget {
            device: from,
            since: since.as_ref(),
            root: &root,
        };
        let folded = eprfs_peer::fold_root(
            &endpoint,
            peer,
            target,
            &store,
            &allow,
            &eprfs_core::TreeLimits::default(),
        )
        .await;
        endpoint.close().await;
        Ok(folded?)
    })
}
