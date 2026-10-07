//! `epr device` — this device asks a node that speaks for a person to
//! recognize it as one of theirs.
//!
//! A device cannot recognize itself as someone's device. Whoever operates the
//! device runs `ask` from a terminal on it: it builds the request for this
//! device's own node and keeps the PKCE verifier and state privately on disk.
//! Then either it prints a link to an approving node's portal, or, with no
//! portal named or declared, it announces the ask on the private network and
//! waits for a node that speaks for the person to approve. Approving is the
//! person's act, done on the approving node. `redeem` (or the announce, when
//! the code comes back over the network) collects the signed consent, checks
//! it before using it, and has this device's own node enroll itself with it.
//!
//! `join` is the same ask in one command: when the approving node is reached
//! over plain http, the code stays with that node and this terminal collects
//! the consent itself with its verifier (`POST /auth/consent/collect`), so the
//! person carries nothing back. `ask` and `redeem` stay for the paste path.
//!
//! The approving node is never assumed: it is named by a portal, declared, or
//! found on the private network by local discovery. No doorway is involved
//! unless one is named. This device's own node is found the same honest way:
//! named, or the one node of this machine that answers; two that answer are
//! named back, never guessed between.
//!
//! The rules are `consent_grant`'s; this module only asks, keeps, checks and
//! reports. The device's key never leaves its node: the node signs possession
//! and notarizes the joining record itself, asked over node-local HTTP
//! (`/auth/device/self`, `/auth/device/enroll`, `/auth/device/announce`).

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use consent_grant::{
    act, check_delivered, parse_pasted, pkce, Delivered, GrantRequest, RequestRefusal,
    RequestedAct, ReturnPath, GRANT_DOMAIN,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::device_key::{self, DeviceKey};

/// The relying party this terminal is, as the approving node's policy names it.
const CLIENT_ID: &str = "epr-cli";
/// The ports a node of this machine is looked for on, in this order, when
/// nothing names it: the default, the port a workspace node takes beside a
/// running household, then the household's own.
const PROBE_PORTS: [u16; 5] = [8090, 8095, 8091, 8092, 8093];
/// How long a port may take to say whether a node is there.
const PROBE_TIMEOUT: Duration = Duration::from_secs(1);
/// How long a node may take to say what it can do, or an approving node
/// whether it takes approvals.
const ASK_TIMEOUT: Duration = Duration::from_secs(10);
/// How often `join` asks the approving node whether the person has said yes.
const POLL_EVERY: Duration = Duration::from_secs(2);
/// How long a code waits, matching the approving node's five-minute window.
const WINDOW: Duration = Duration::from_secs(5 * 60);
/// How long one call to a node may take: a node's answer can wait on the network.
const CALL_TIMEOUT: Duration = Duration::from_secs(180);

/// Said when an approving node is named by an https origin.
const HTTPS_APPROVER: &str = "this terminal reaches nodes over plain http only; a doorway \
     portal's node must be named with --approver <http url>, or approve on a device that is \
     already yours with `epr device approve '<link>'`";

type Outcome<T> = Result<T, String>;

pub fn usage() -> &'static str {
    "usage:\n  epr device join [--label <name>] [--portal <approving node's portal URL>] \
     [--approver <approving node URL>] [--act <act>]... [--announce] [--node <this node URL>]\n    \
     (asks, then waits and finishes on its own once a device that is already yours says yes)\n  \
     epr device ask [--portal <approving node's portal URL>] --label <name> \
     [--act device.enroll] [--act device.bind-root] [--loopback | --announce] \
     [--approver <approving node URL>] [--node <this node URL>]\n    \
     (with no portal named or declared, the device announces on its private network)\n  \
     epr device redeem '<code#state>' [--approver <approving node URL>] [--node <this node URL>]\n  \
     epr device pending [--node <this node URL>]\n  \
     epr device approve '<link | number | key fingerprint>' [--only <act>]... [--yes] [--node <this node URL>]\n  \
     epr device approve <number | key fingerprint> --decline\n\
     This machine's node is --node, else ELOHIM_NODE_URL, else the one node of this machine \
     that answers on 8090/8095/8091/8092/8093."
}

pub fn run(args: &[String]) -> Outcome<ExitCode> {
    match args.first().map(String::as_str) {
        Some("join") => join(&args[1..]),
        Some("ask") => ask(&args[1..]),
        Some("approve") => crate::approver::approve(&args[1..]),
        Some("pending") => crate::approver::pending(&args[1..]),
        Some("redeem") => {
            let (pasted, opts) = match args.get(1) {
                Some(p) if !p.starts_with("--") => (p.clone(), Options::parse(&args[2..])?),
                _ => return Err(format!("redeem needs the pasted code\n{}", usage())),
            };
            let (code, state) = parse_pasted(&pasted)
                .ok_or("that is not a code: it should look like code#state")?;
            let mut pending = Pending::load(state)?;
            // The approving node may be named again, by its own http address,
            // when the portal that showed the code is one this terminal cannot
            // reach (an https origin).
            if let Some(approver) = opts.approver.clone() {
                pending.approver = approver;
            }
            let node = opts.node.unwrap_or_else(|| pending.node.clone());
            capable(&node)?;
            redeem(pending, code, &node)
        }
        Some("--help" | "-h" | "help") => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        None => {
            println!("{}", usage());
            // Which side of a join this machine is on, when its node can say.
            if let Ok(node) = this_node(None) {
                if let Some(name) = crate::approver::speaks_for(&node) {
                    println!("\n{}.", crate::approver::role_words(&name));
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Some(other) => Err(format!("unknown device command `{other}`\n{}", usage())),
    }
}

/// This machine's node: `--node`; else `ELOHIM_NODE_URL`; else the storage
/// port the launcher recorded in `.hc_ports`, when it records one; else the
/// one node that answers on this machine's usual ports. None, or more than
/// one, is refused: a terminal that guessed would ask on the wrong node's
/// behalf.
pub(crate) fn this_node(flag: Option<&str>) -> Outcome<String> {
    let env = std::env::var("ELOHIM_NODE_URL").ok();
    find_node(
        flag,
        env.as_deref(),
        recorded_storage_port(),
        &PROBE_PORTS,
        PROBE_TIMEOUT,
    )
}

fn find_node(
    flag: Option<&str>,
    env: Option<&str>,
    recorded: Option<u16>,
    ports: &[u16],
    timeout: Duration,
) -> Outcome<String> {
    let named = |v: Option<&str>| {
        v.map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.trim_end_matches('/').to_string())
    };
    if let Some(node) = named(flag).or_else(|| named(env)) {
        return Ok(node);
    }
    if let Some(port) = recorded {
        return Ok(format!("http://127.0.0.1:{port}"));
    }
    // Every port at once, so looking costs one timeout, not five.
    let answering: Vec<u16> = std::thread::scope(|s| {
        let looks: Vec<_> = ports
            .iter()
            .map(|&port| s.spawn(move || is_a_node(&format!("http://127.0.0.1:{port}"), timeout)))
            .collect();
        ports
            .iter()
            .zip(looks)
            .filter_map(|(&port, look)| look.join().unwrap_or(false).then_some(port))
            .collect()
    });
    match answering.as_slice() {
        [one] => {
            // One answering is not proof there is only one: a stopped or slow
            // node does not answer either. Say which was used, and why.
            eprintln!("using the only node of this machine that answers, at 127.0.0.1:{one}");
            Ok(format!("http://127.0.0.1:{one}"))
        }
        [] => Err(format!(
            "no node of this machine answers on {}; start one (`just dev start`) or name it with --node",
            ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join("/")
        )),
        many => Err(format!(
            "more than one node of this machine answers ({}); name the one to use with --node",
            many.iter()
                .map(|p| format!("127.0.0.1:{p}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Whether a node answers at `origin`: its device route answers, or its
/// health does. Anything else on the port is not a node.
fn is_a_node(origin: &str, timeout: Duration) -> bool {
    let ok = |path: &str| {
        matches!(
            http_timed("GET", &format!("{origin}{path}"), None, &[], timeout),
            Ok((status, _, _)) if (200..300).contains(&status)
        )
    };
    ok("/auth/device/self") || ok("/health")
}

/// The storage port the launcher recorded beside its conductor ports, when
/// it records one (`storage_port=` in `elohim/holochain/local-dev/.hc_ports`,
/// found from the working directory up).
fn recorded_storage_port() -> Option<u16> {
    let here = std::env::current_dir().ok()?;
    here.ancestors().find_map(|dir| {
        let file = dir.join("elohim/holochain/local-dev/.hc_ports");
        storage_port_in(&fs::read_to_string(file).ok()?)
    })
}

fn storage_port_in(ports_file: &str) -> Option<u16> {
    ports_file.lines().find_map(|line| {
        line.trim()
            .strip_prefix("storage_port=")
            .and_then(|p| p.trim().parse().ok())
    })
}

/// Refuse a node that cannot take part in device joining, before anything is
/// asked of it: one that answers its health but not its device route was
/// built before device joining existed.
pub(crate) fn capable(node: &str) -> Outcome<()> {
    let node = node.trim_end_matches('/');
    match http_timed(
        "GET",
        &format!("{node}/auth/device/self"),
        None,
        &[],
        ASK_TIMEOUT,
    ) {
        Err(e) => Err(format!("{node}: the node did not answer ({e})")),
        Ok((404, _, _)) => {
            match http_timed("GET", &format!("{node}/health"), None, &[], ASK_TIMEOUT) {
                Ok((status, _, _)) if (200..300).contains(&status) => Err(format!(
                    "{node}: this node's build predates device joining; rebuild and restart it \
                 (`just mesh build storage`)"
                )),
                _ => Err(format!(
                    "{node}: nothing there answers as a node (no device route, no health)"
                )),
            }
        }
        Ok(_) => Ok(()),
    }
}

/// Refuse an approving node this terminal cannot reach, or one that does not
/// take device approvals, before a link is printed. An empty request to its
/// consent screen is refused by a node that has one (400) and unknown to one
/// that does not (404). Any other answer says nothing either way, and an ask
/// is not made on a guess.
fn approver_takes_approvals(approver: &str) -> Outcome<()> {
    if is_https(approver) {
        return Err(HTTPS_APPROVER.into());
    }
    let approver = approver.trim_end_matches('/');
    match http_timed(
        "POST",
        &format!("{approver}/auth/consent/view"),
        Some(b"{}"),
        &[],
        ASK_TIMEOUT,
    ) {
        Ok((status, _, _)) if status == 400 || (200..300).contains(&status) => Ok(()),
        Ok((404, _, _)) => Err(format!(
            "the node at {approver} does not take device approvals"
        )),
        Ok((421, _, _)) => Err(only_its_own_names(approver)),
        Ok((status, _, _)) => Err(could_not_tell(approver, status)),
        Err(e) => Err(unreachable_approver(approver, &e)),
    }
}

/// Said when the approving node gave an answer that settles nothing.
fn could_not_tell(approver: &str, status: u16) -> String {
    format!(
        "could not tell whether {approver} takes approvals (answered {status}); try again shortly"
    )
}

/// Said when the approving node cannot be reached at all.
fn unreachable_approver(approver: &str, e: &str) -> String {
    format!(
        "the approving node at {approver} does not answer ({e}); start it, or name the node that \
         speaks for you with --approver <http url>"
    )
}

/// Said when the approving node answers its identity routes only under its
/// own names, and the name this terminal reached it by is not one of them.
fn only_its_own_names(approver: &str) -> String {
    format!(
        "the node at {approver} answers only under its own names; add {} to \
         ELOHIM_ALLOWED_HOSTS on that node, or approve there with `epr device approve '<link>'`",
        host_of(approver)
    )
}

/// The name in an origin, as the node compares it: no scheme, path or port,
/// and an IPv6 address without its brackets.
fn host_of(origin: &str) -> String {
    let rest = origin.split_once("://").map_or(origin, |(_, r)| r);
    let authority = rest.split('/').next().unwrap_or(rest);
    match authority.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(v6).to_string(),
        None => authority
            .rsplit_once(':')
            .map_or(authority, |(name, _)| name)
            .to_string(),
    }
}

fn is_https(url: &str) -> bool {
    url.trim_start()
        .to_ascii_lowercase()
        .starts_with("https://")
}

#[derive(Default)]
struct Options {
    portal: Option<String>,
    approver: Option<String>,
    node: Option<String>,
    label: Option<String>,
    acts: Vec<String>,
    loopback: bool,
    announce: bool,
}

impl Options {
    fn parse(args: &[String]) -> Outcome<Self> {
        let mut o = Self::default();
        let mut i = 0;
        while i < args.len() {
            let flag = args[i].as_str();
            let mut value = || {
                i += 1;
                args.get(i).cloned().ok_or(format!("{flag} needs a value"))
            };
            match flag {
                "--portal" => o.portal = Some(value()?),
                "--approver" => o.approver = Some(value()?),
                "--node" => o.node = Some(value()?),
                "--label" => o.label = Some(value()?),
                "--act" => o.acts.push(value()?),
                "--loopback" => o.loopback = true,
                "--announce" => o.announce = true,
                other => return Err(format!("unknown argument `{other}`\n{}", usage())),
            }
            i += 1;
        }
        Ok(o)
    }
}

/// What this terminal keeps while it waits for the code: the request it made
/// and the verifier only it holds. Private to this OS user, gone once used.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pending {
    request: GrantRequest,
    code_verifier: String,
    approver: String,
    node: String,
    asked_at_unix: u64,
    /// The approving node's key, when it was declared.
    #[serde(default)]
    approver_key: Option<String>,
    /// Whether the ask was announced on the private network rather than
    /// shown as a link.
    #[serde(default)]
    announced: bool,
    /// Until when the approving node last said it still shows this ask; its
    /// window starts when the person opens the link, not when this device
    /// asked.
    #[serde(default)]
    waiting_until_unix: Option<u64>,
}

fn pending_dir() -> Outcome<PathBuf> {
    let key = device_key::resolve_path().map_err(|e| format!("no device key home: {e}"))?;
    let dir = key
        .parent()
        .ok_or("the device key has no directory")?
        .join("consent-requests");
    Ok(dir)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Write `bytes` to `path` in `dir`, private to this OS user: the directory
/// 0700, the file 0600. `fresh` refuses a file that already exists.
fn write_private(dir: &Path, path: &Path, bytes: &[u8], fresh: bool) -> Outcome<()> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    let mut options = OpenOptions::new();
    if fresh {
        options.write(true).create_new(true);
    } else {
        options.write(true).create(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}

/// [`write_private`], replacing `path` only once the new bytes are whole: they
/// are written beside it and renamed over it, so a kept file is never left
/// half-written or emptied.
fn replace_private(dir: &Path, path: &Path, bytes: &[u8]) -> Outcome<()> {
    let mut beside = path.as_os_str().to_owned();
    beside.push(".tmp");
    let beside = PathBuf::from(beside);
    write_private(dir, &beside, bytes, false)?;
    fs::rename(&beside, path).map_err(|e| {
        let _ = fs::remove_file(&beside);
        e.to_string()
    })
}

impl Pending {
    fn path(state: &str) -> Outcome<PathBuf> {
        Ok(Self::path_in(&pending_dir()?, state))
    }

    fn path_in(dir: &Path, state: &str) -> PathBuf {
        dir.join(format!("{state}.json"))
    }

    fn save_in(&self, dir: &Path) -> Outcome<()> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        write_private(dir, &Self::path_in(dir, &self.request.state), &bytes, true)
            .map_err(|e| format!("cannot keep the request: {e}"))
    }

    /// Keep what has changed about an ask already kept.
    fn rewrite_in(&self, dir: &Path) -> Outcome<()> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        replace_private(dir, &Self::path_in(dir, &self.request.state), &bytes)
    }

    /// When this ask stops being worth waiting on: five minutes after it was
    /// made, or later when the approving node says it still shows it.
    fn open_until(&self) -> u64 {
        (self.asked_at_unix + WINDOW.as_secs()).max(self.waiting_until_unix.unwrap_or(0))
    }

    /// Whether this kept ask is the one `intent` would make.
    fn is_ask(&self, intent: &Intent) -> bool {
        let acts = |a: &[RequestedAct]| {
            let mut a: Vec<&str> = a.iter().map(|a| a.as_str()).collect();
            a.sort_unstable();
            a
        };
        self.request.label == intent.label
            && self.approver.trim_end_matches('/') == intent.approver.trim_end_matches('/')
            && acts(&self.request.acts) == acts(&intent.acts)
            && self.announced == intent.announcing
    }

    /// The kept ask, in a few words.
    fn words(&self) -> String {
        let acts: Vec<&str> = self.request.acts.iter().map(|a| a.as_str()).collect();
        let carried = if self.announced {
            "announced on the private network".to_string()
        } else {
            format!("approved at {}", self.approver.trim_end_matches('/'))
        };
        format!(
            "{} asking {}, {carried}",
            self.request.label,
            acts.join(", ")
        )
    }

    fn load(state: &str) -> Outcome<Self> {
        // The state is a URL-safe token, so it cannot name a path elsewhere.
        if !consent_grant::return_path::is_token(state, 16, 128) {
            return Err("that code's state is malformed".into());
        }
        let bytes = fs::read(Self::path(state)?).map_err(|_| {
            "no request with that state is waiting on this device; ask again".to_string()
        })?;
        serde_json::from_slice(&bytes).map_err(|e| format!("the kept request is unreadable: {e}"))
    }

    fn forget(&self) {
        if let Ok(path) = Self::path(&self.request.state) {
            let _ = fs::remove_file(path);
        }
    }

    fn forget_in(&self, dir: &Path) {
        let _ = fs::remove_file(Self::path_in(dir, &self.request.state));
    }
}

/// What a kept consent is filed under, beside its ask.
const KEPT_SUFFIX: &str = ".delivered.json";

/// A consent collected and checked, kept beside its ask before this device's
/// node is asked to enroll with it, and forgotten once it has. When the node
/// cannot enroll yet (it said to try again shortly), the next `join` enrolls
/// with it without asking the person again. Private to this OS user, kept
/// five minutes by this device's own clock: the node checks the consent
/// itself when it enrolls, so the approving node's clock does not decide.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeptConsent {
    pending: Pending,
    delivered: Delivered,
    /// The approving node's key, when one was known.
    #[serde(default)]
    expected: Option<String>,
    /// When this device stops keeping it.
    until_unix: u64,
}

impl KeptConsent {
    fn path_in(dir: &Path, state: &str) -> PathBuf {
        dir.join(format!("{state}{KEPT_SUFFIX}"))
    }

    fn keep_in(&self, dir: &Path) -> Outcome<()> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        replace_private(
            dir,
            &Self::path_in(dir, &self.pending.request.state),
            &bytes,
        )
    }

    /// Forget the kept consent and the ask it answers.
    fn forget_in(&self, dir: &Path) {
        let _ = fs::remove_file(Self::path_in(dir, &self.pending.request.state));
        self.pending.forget_in(dir);
    }

    /// The consents kept for `node` whose window is still open, newest last.
    /// Kept consents whose window has closed are forgotten, with their asks.
    fn find_in(dir: &Path, node: &str, now_unix: u64) -> Vec<Self> {
        let node = node.trim_end_matches('/');
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut found: Vec<Self> = entries
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(KEPT_SUFFIX))
            .filter_map(|e| serde_json::from_slice::<Self>(&fs::read(e.path()).ok()?).ok())
            .filter(|k| {
                let open = now_unix < k.until_unix;
                if !open {
                    k.forget_in(dir);
                }
                open
            })
            .filter(|k| k.pending.node.trim_end_matches('/') == node)
            .collect();
        found.sort_by_key(|k| k.until_unix);
        found
    }
}

fn token(bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    OsRng.fill_bytes(&mut raw);
    URL_SAFE_NO_PAD.encode(raw)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceSelf {
    device_key: String,
    network_dna: String,
    content_dna: String,
}

/// Said when an announced ask is also asked to wait on a browser.
const LOOPBACK_ANNOUNCED: &str = "--loopback hands the code back through a browser; an \
     announced ask takes it back over the private network";

/// What the flags alone refuse, before any node is spoken to: the acts, two
/// ways back that cannot both hold, and an approving node this terminal could
/// never reach.
fn refuse_from_flags(opts: &Options) -> Outcome<Vec<RequestedAct>> {
    let texts = if opts.acts.is_empty() {
        vec!["device.enroll".to_string()]
    } else {
        opts.acts.clone()
    };
    let acts = texts
        .iter()
        .map(|t| RequestedAct::parse(t).map_err(|r| format!("{t}: {}", r.code())))
        .collect::<Outcome<Vec<_>>>()?;
    // The crate's own rule, before anything is asked of anyone.
    if !act::coherent(&acts) {
        return Err(format!(
            "{}: a device root is bound only for a device being enrolled, and each act once",
            RequestRefusal::ActsIncoherent.code()
        ));
    }
    if opts.announce && opts.loopback {
        return Err(LOOPBACK_ANNOUNCED.into());
    }
    // The approving node, when the flags name it without the node's
    // declaration (which is read only when a flag is left out).
    if !opts.announce {
        let named = opts.approver.clone().or_else(|| {
            opts.label
                .as_ref()
                .and(opts.portal.as_deref())
                .map(origin_of)
        });
        if named.as_deref().is_some_and(is_https) {
            return Err(HTTPS_APPROVER.into());
        }
    }
    Ok(acts)
}

/// Which command is asking: `ask` shows or hands back a code; `join` has the
/// approving node hold it and collects it itself.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Asking {
    Ask,
    Join,
}

/// An ask made and kept on disk, ready to be carried.
struct Asked {
    pending: Pending,
    /// The link to show the person; none when the ask is announced on the
    /// private network instead.
    link: Option<String>,
    listener: Option<TcpListener>,
    approver_key: Option<String>,
}

/// The ask the flags and this node's declaration describe, before anything is
/// asked of anyone.
struct Intent {
    label: String,
    portal: Option<String>,
    /// The approving node's origin; empty when nothing names one.
    approver: String,
    approver_key: Option<String>,
    acts: Vec<RequestedAct>,
    announcing: bool,
}

/// Resolve the ask the flags describe, the node's declaration filling what
/// they leave out.
fn intent(opts: &Options, acts: Vec<RequestedAct>, node: &str, asking: Asking) -> Outcome<Intent> {
    // What this node declares about asking fills what the flags leave out.
    let declared = if opts.portal.is_none() || opts.label.is_none() {
        crate::approver::node_declaration(node)
            .declared
            .and_then(|d| d.asking)
    } else {
        None
    };
    let portal = opts
        .portal
        .clone()
        .or_else(|| declared.as_ref().and_then(|a| a.portal.clone()));
    let approver_key = declared.as_ref().and_then(|a| a.approver_key.clone());
    let label = opts
        .label
        .clone()
        .or_else(|| declared.as_ref().map(|a| a.label.clone()))
        .ok_or(match asking {
            Asking::Ask => {
                "ask needs --label: what to call this device (or declare it in [asking])"
            }
            Asking::Join => {
                "join needs --label: what to call this device (or declare it in [asking])"
            }
        })?;
    let approver = opts
        .approver
        .clone()
        .or_else(|| declared.as_ref().and_then(|a| a.approver.clone()));
    // With nothing naming where to show a link, or when asked to, the device
    // announces on its private network instead. `join` also holds with an
    // approving node named and no portal: the link is then that node's own.
    let announcing =
        opts.announce || portal.is_none() && (asking == Asking::Ask || approver.is_none());
    if announcing && opts.loopback {
        return Err(LOOPBACK_ANNOUNCED.into());
    }
    let approver = approver
        .or_else(|| portal.as_deref().map(origin_of))
        .unwrap_or_default();
    Ok(Intent {
        label,
        portal,
        approver,
        approver_key,
        acts,
        announcing,
    })
}

/// Make the ask `intent` describes for this device's own `node` and keep it.
/// Everything that can be refused is refused before the ask is kept: the
/// approving node is reached first, so no link is printed for a node that
/// cannot take it.
fn prepare(
    opts: &Options,
    intent: Intent,
    node: &str,
    asking: Asking,
    dir: &Path,
) -> Outcome<Asked> {
    let Intent {
        label,
        portal,
        approver,
        approver_key,
        acts,
        announcing,
    } = intent;
    // A node that already speaks for someone has nothing to join with an
    // announce: it is the other side of the join.
    if asking == Asking::Join && announcing {
        if let Some(name) = crate::approver::speaks_for(node) {
            return Err(crate::approver::role_words(&name));
        }
    }
    let mut holding = false;
    if !announcing {
        approver_takes_approvals(&approver)?;
        if asking == Asking::Join {
            holding = approver_collects(&approver)?;
            if !holding {
                println!(
                    "The approving node at {} was built before it could hand the consent back \
                     on its own; this device will show a code to carry instead.",
                    approver.trim_end_matches('/')
                );
            }
        }
    }

    let me: DeviceSelf = json_call("GET", &format!("{node}/auth/device/self"), None)
        .map_err(|e| format!("this device's node did not answer: {e}"))?;
    let device_root_key = if acts.contains(&RequestedAct::BindDeviceRoot) {
        let key = DeviceKey::open().map_err(|e| format!("no device root key: {e}"))?;
        Some(key.did_key())
    } else {
        None
    };
    let listener = if opts.loopback {
        Some(
            TcpListener::bind("127.0.0.1:0")
                .map_err(|e| format!("cannot listen for the code: {e}"))?,
        )
    } else {
        None
    };
    // The announced ask keeps the paste path on both sides: the carrier
    // hands its code back over the private network.
    let return_path = match &listener {
        _ if holding => ReturnPath::Hold,
        Some(l) => ReturnPath::Loopback {
            port: l.local_addr().map_err(|e| e.to_string())?.port(),
        },
        None => ReturnPath::Paste,
    };
    let verifier = token(32);
    let request = GrantRequest {
        domain: GRANT_DOMAIN.into(),
        client_id: CLIENT_ID.into(),
        device_key: me.device_key,
        device_root_key,
        label,
        network_dna: me.network_dna,
        content_dna: me.content_dna,
        acts,
        code_challenge: pkce::challenge(&verifier),
        state: token(32),
        return_path,
    };
    // Every rule a portal applies, applied here first.
    consent_grant::admit_request(
        &request,
        &consent_grant::GrantPolicy::for_clients([CLIENT_ID]),
    )
    .map_err(|r| format!("{}: this request would be refused", r.code()))?;
    let link = if announcing {
        None
    } else {
        let encoded =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&request).map_err(|e| e.to_string())?);
        let base = portal.as_deref().unwrap_or(&approver);
        Some(format!(
            "{}/consent/device?request={encoded}",
            base.trim_end_matches('/')
        ))
    };
    let pending = Pending {
        request,
        code_verifier: verifier,
        approver,
        node: node.to_string(),
        asked_at_unix: now_unix(),
        approver_key: approver_key.clone(),
        announced: announcing,
        waiting_until_unix: None,
    };
    pending.save_in(dir)?;
    Ok(Asked {
        pending,
        link,
        listener,
        approver_key,
    })
}

/// The link and this device's fingerprints, for a code the person carries.
fn print_link(link: &str, request: &GrantRequest) {
    println!("Open this link in the portal of a node that speaks for you, and approve this device:\n\n  {link}\n");
    println!(
        "This device is {} on network {}.",
        consent_grant::hash_shape::fingerprint(&request.device_key),
        consent_grant::hash_shape::fingerprint(&request.network_dna)
    );
}

const REDEEM_LINE: &str =
    "Then run:  epr device redeem '<code#state>'   (the code is good for five minutes)";

fn ask(args: &[String]) -> Outcome<ExitCode> {
    let opts = Options::parse(args)?;
    let acts = refuse_from_flags(&opts)?;
    let node = this_node(opts.node.as_deref())?;
    capable(&node)?;
    let Asked {
        pending,
        link,
        listener,
        approver_key,
    } = prepare(
        &opts,
        intent(&opts, acts, &node, Asking::Ask)?,
        &node,
        Asking::Ask,
        &pending_dir()?,
    )?;
    let Some(link) = link else {
        return announce(pending, approver_key, &node);
    };
    print_link(&link, &pending.request);
    let Some(listener) = listener else {
        println!("{REDEEM_LINE}");
        return Ok(ExitCode::SUCCESS);
    };
    println!("Waiting for the portal to hand the code back to this terminal…");
    let (code, state) = wait_for_callback(&listener)?;
    if state != pending.request.state {
        pending.forget();
        return Err("the code that arrived was not for this request".into());
    }
    redeem(pending, &code, &node)
}

/// `epr device join`: ask, then wait for the person's yes and finish on its
/// own. Nothing is carried back by hand when the approving node holds the
/// code; with no approving node named, the ask is announced as `ask` does.
fn join(args: &[String]) -> Outcome<ExitCode> {
    join_in(args, &pending_dir()?)
}

/// [`join`], keeping its asks and consents in `dir`.
fn join_in(args: &[String], dir: &Path) -> Outcome<ExitCode> {
    let opts = Options::parse(args)?;
    if opts.loopback {
        return Err(
            "join waits on the approving node itself; --loopback belongs to `epr device ask`"
                .into(),
        );
    }
    let acts = refuse_from_flags(&opts)?;
    let node = this_node(opts.node.as_deref())?;
    capable(&node)?;
    let intent = intent(&opts, acts, &node, Asking::Join)?;
    // A consent the person already gave for this same ask, which this
    // device's node could not enroll with yet, is used rather than asked for
    // again. One kept for a different ask is left to its window.
    let mut consents = KeptConsent::find_in(dir, &node, now_unix());
    if let Some(i) = consents.iter().rposition(|k| k.pending.is_ask(&intent)) {
        let kept = consents.swap_remove(i);
        println!(
            "This device already holds consent {} for joining as {}; its node did not enroll \
             with it before. Enrolling now, without asking again.",
            kept.delivered.consent.cid, kept.pending.request.label
        );
        return enroll_with(kept, &node, dir, None);
    }
    if let Some(other) = consents.last() {
        println!(
            "A different ask is kept ({}); asking afresh.",
            other.pending.words()
        );
    }
    // An ask this device already made for the same thing and is still
    // waiting on is picked up, not made again: closing the terminal does not
    // lose it.
    if let Some(kept) = kept_join(&node, &intent, dir) {
        let ago = now_unix().saturating_sub(kept.asked_at_unix);
        println!(
            "This device asked to join as {} {}:{:02} ago and is still waiting; picking that ask up.",
            kept.request.label,
            ago / 60,
            ago % 60
        );
        return collect_and_finish(kept, &node, dir);
    }
    let Asked {
        pending,
        link,
        approver_key,
        ..
    } = prepare(&opts, intent, &node, Asking::Join, dir)?;
    let Some(link) = link else {
        return announce(pending, approver_key, &node);
    };
    if pending.request.return_path != ReturnPath::Hold {
        print_link(&link, &pending.request);
        println!("{REDEEM_LINE}");
        return Ok(ExitCode::SUCCESS);
    }
    println!(
        "Asking to join as {}. This device is {}. Confirm it on a device that is already yours.\n\n  {link}\n",
        pending.request.label,
        consent_grant::hash_shape::fingerprint(&pending.request.device_key)
    );
    println!("Open that link there, or run there:  epr device approve '<that link>'");
    collect_and_finish(pending, &node, dir)
}

/// A join this device already asked for the ask `intent` describes and is
/// still waiting on: the newest such held ask from this node whose window is
/// open. Held asks whose window has closed are forgotten; an open one for a
/// different ask is named and left to its window.
fn kept_join(node: &str, intent: &Intent, dir: &Path) -> Option<Pending> {
    let now = now_unix();
    let node = node.trim_end_matches('/');
    let mut kept: Vec<Pending> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.ends_with(".json") && !name.ends_with(KEPT_SUFFIX)
        })
        .filter_map(|entry| serde_json::from_slice::<Pending>(&fs::read(entry.path()).ok()?).ok())
        .filter(|p| {
            p.request.return_path == ReturnPath::Hold && p.node.trim_end_matches('/') == node
        })
        .filter(|p| {
            let open = now < p.open_until();
            if !open {
                p.forget_in(dir);
            }
            open
        })
        .collect();
    kept.sort_by_key(|p| p.asked_at_unix);
    match kept.iter().rposition(|p| p.is_ask(intent)) {
        Some(i) => Some(kept.swap_remove(i)),
        None => {
            if let Some(other) = kept.last() {
                println!(
                    "A different ask is kept ({}); asking afresh.",
                    other.words()
                );
            }
            None
        }
    }
}

/// Whether the approving node hands a held consent to the terminal that
/// asked. An empty collection is refused by a node that can (400) and unknown
/// to one built before it could (404). Any other answer settles nothing, and
/// no answer at all says the node is unreachable, not that it is old.
fn approver_collects(approver: &str) -> Outcome<bool> {
    let approver = approver.trim_end_matches('/');
    match http_timed(
        "POST",
        &format!("{approver}/auth/consent/collect"),
        Some(b"{}"),
        &[],
        ASK_TIMEOUT,
    ) {
        Ok((400, _, _)) => Ok(true),
        Ok((404, _, _)) => Ok(false),
        Ok((421, _, _)) => Err(only_its_own_names(approver)),
        Ok((status, _, _)) => Err(could_not_tell(approver, status)),
        Err(e) => Err(unreachable_approver(approver, &e)),
    }
}

/// A refusal's code, when the answer carries one.
fn refusal_code(bytes: &[u8]) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()?
        .get("code")?
        .as_str()
        .map(str::to_string)
}

/// Wait for the held consent, then check it and enroll with it.
fn collect_and_finish(mut pending: Pending, node: &str, dir: &Path) -> Outcome<ExitCode> {
    let left = (pending.open_until() + 10).saturating_sub(now_unix());
    let deadline = Instant::now() + Duration::from_secs(left);
    let bytes = collect(
        &mut pending,
        POLL_EVERY,
        deadline,
        &mut |line| println!("{line}"),
        dir,
    )?;
    let delivered: Delivered = match serde_json::from_slice(&bytes) {
        Ok(d) => d,
        Err(e) => {
            pending.forget_in(dir);
            return Err(format!("the approving node's answer is unreadable: {e}"));
        }
    };
    println!("A device that is yours said yes; checking the consent and enrolling this device.");
    let expected = pending.approver_key.clone();
    finish(pending, delivered, node, expected, dir)
}

/// Say `line` only when what is waited on has changed.
fn tell_once(told: &mut &'static str, key: &'static str, line: String, say: &mut dyn FnMut(&str)) {
    if *told != key {
        *told = key;
        say(&line);
    }
}

/// Ask the approving node for the held consent every `every` until it hands
/// it over, the ask ends, or the deadline passes. The deadline follows the
/// node: while it says it still shows the ask, the wait lasts as long as it
/// says, and that is kept with the ask. Each change in what is waited on is
/// said once through `say`. The kept ask is forgotten once it can no longer
/// complete; while the node cannot be reached, or answers only under names
/// this terminal did not use, it is kept.
fn collect(
    pending: &mut Pending,
    every: Duration,
    mut deadline: Instant,
    say: &mut dyn FnMut(&str),
    dir: &Path,
) -> Outcome<Vec<u8>> {
    let approver = pending.approver.trim_end_matches('/').to_string();
    let approver = approver.as_str();
    let url = format!("{approver}/auth/consent/collect");
    let request = &pending.request;
    let body = serde_json::to_vec(&consent_grant::Collection {
        state: request.state.clone(),
        code_verifier: pending.code_verifier.clone(),
        client_id: request.client_id.clone(),
        device_key: request.device_key.clone(),
    })
    .map_err(|e| e.to_string())?;
    // Only the ask is forgotten here, never a consent kept beside it.
    let ended = |pending: &Pending, why: String| -> Outcome<Vec<u8>> {
        pending.forget_in(dir);
        Err(why)
    };
    let mut told: &'static str = "";
    loop {
        match http_timed("POST", &url, Some(&body), &[], ASK_TIMEOUT) {
            Err(e) => tell_once(
                &mut told,
                "unreachable",
                format!("The approving node at {approver} does not answer ({e}); waiting for it."),
                say,
            ),
            Ok((status, _, bytes)) => {
                let code = refusal_code(&bytes).unwrap_or_default();
                match status {
                    202 => {
                        let seconds = serde_json::from_slice::<serde_json::Value>(&bytes)
                            .ok()
                            .and_then(|v| v["secondsLeft"].as_u64());
                        if let Some(n) = seconds {
                            // The node's window, not this device's guess.
                            deadline = deadline.max(Instant::now() + Duration::from_secs(n + 10));
                            let until = now_unix() + n + 10;
                            if pending.waiting_until_unix.is_none_or(|w| w < until) {
                                pending.waiting_until_unix = Some(until);
                                let _ = pending.rewrite_in(dir);
                            }
                        }
                        let left = seconds
                            .map(|s| format!(" ({}:{:02} left)", s / 60, s % 60))
                            .unwrap_or_default();
                        tell_once(
                            &mut told,
                            "waiting",
                            format!(
                                "The approving node at {approver} shows this ask; waiting for \
                                 your yes there{left}."
                            ),
                            say,
                        );
                    }
                    s if (200..300).contains(&s) => return Ok(bytes),
                    // Also what is answered before the person opens the link:
                    // keep waiting while the window lasts.
                    404 if code.contains("unknown") => tell_once(
                        &mut told,
                        "unseen",
                        format!(
                            "Waiting for you to open the link on a device that is already \
                             yours; the approving node at {approver} has not seen this ask yet."
                        ),
                        say,
                    ),
                    404 => {
                        return ended(
                            pending,
                            format!(
                                "the node at {approver} does not take device approvals; ask \
                                 with `epr device ask` and carry the code instead"
                            ),
                        )
                    }
                    403 => {
                        return ended(pending, format!(
                            "the approving node at {approver} would not hand the consent to this \
                             device ({}); something is wrong on this device's side. Nothing was \
                             spent and nothing was enrolled.",
                            refusal_text(status, &bytes)
                        ))
                    }
                    410 => return ended(pending, ended_words(&code, status, &bytes)),
                    // Nothing about the ask is wrong: it is kept, so once the
                    // name is added, join picks it up without a new yes.
                    421 => {
                        return Err(format!(
                            "{}. The ask is kept: run join again once that is done.",
                            only_its_own_names(approver)
                        ))
                    }
                    429 | 500..=599 => tell_once(
                        &mut told,
                        "busy",
                        format!("The approving node at {approver} is busy (HTTP {status}); asking again."),
                        say,
                    ),
                    _ => {
                        return ended(
                            pending,
                            format!(
                                "the approving node did not hand over the consent: {}",
                                refusal_text(status, &bytes)
                            ),
                        )
                    }
                }
            }
        }
        // Checked after asking, so an ask picked up late is still asked once.
        if Instant::now() > deadline {
            return ended(
                pending,
                if told == "unseen" {
                    format!(
                        "the approving node at {approver} never saw this ask: the link was not \
                         opened there within five minutes. Run join again."
                    )
                } else {
                    "no yes reached this device while the approving node waited for it; the ask \
                     has ended. Run join again."
                        .to_string()
                },
            );
        }
        std::thread::sleep(every);
    }
}

/// Why an ask ended, by the code the approving node gave.
fn ended_words(code: &str, status: u16, bytes: &[u8]) -> String {
    if code.contains("declined") {
        "you said no on the approving node; nothing was approved and this device is not enrolled"
            .into()
    } else if code.contains("spent") || code.contains("already_collected") {
        "the consent for this ask was already collected; if this device is not enrolled, run \
         join again"
            .into()
    } else if code.contains("expired") {
        "the ask's five minutes ended before the consent was collected; run join again".into()
    } else {
        format!("the ask has ended: {}", refusal_text(status, bytes))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Announced {
    state: consent_grant::NodeState,
    peers: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnnounceStatus {
    status: String,
    #[serde(default)]
    seconds_left: i64,
    #[serde(default)]
    listed_by: Vec<ListedBy>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    declined_by: Option<DeclinedBy>,
    /// The key whose proof came with the code.
    #[serde(default)]
    code_approver: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclinedBy {
    approver_fingerprint: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListedBy {
    approver_fingerprint: Option<String>,
    number: u32,
}

/// Offer the ask to whichever approving node on the private network lists it, wait
/// for one to approve, and enroll with the code it hands back.
fn announce(pending: Pending, approver_key: Option<String>, node: &str) -> Outcome<ExitCode> {
    let node = node.trim_end_matches('/');
    let body = serde_json::to_vec(&serde_json::json!({
        "request": pending.request,
        "approverKey": approver_key,
    }))
    .map_err(|e| e.to_string())?;
    let announced: Announced =
        match json_call("POST", &format!("{node}/auth/device/announce"), Some(&body)) {
            Ok(a) => a,
            Err(e) => {
                pending.forget();
                return Err(format!(
                    "{e}\nAsk with --portal <approving node's portal> to print a link instead."
                ));
            }
        };
    let fp = consent_grant::hash_shape::fingerprint(&pending.request.device_key);
    println!(
        "Announcing on this private network ({} peer{} in reach). This device is {fp}; it {}.",
        announced.peers,
        if announced.peers == 1 { "" } else { "s" },
        announced.state.words()
    );
    match &approver_key {
        Some(k) => println!(
            "Waiting for node {} to approve.",
            consent_grant::hash_shape::fingerprint(k)
        ),
        None => println!("Waiting for an approving node to approve."),
    }
    let mut told: Vec<u32> = Vec::new();
    let deadline = Instant::now() + WINDOW + Duration::from_secs(10);
    let (code, proven_approver) = loop {
        if Instant::now() > deadline {
            pending.forget();
            return Err("no approving node approved within five minutes; ask again".into());
        }
        let status: AnnounceStatus =
            json_call("GET", &format!("{node}/auth/device/announce"), None)?;
        for listed in &status.listed_by {
            if !told.contains(&listed.number) {
                told.push(listed.number);
                println!(
                    "Node {} lists this ask as number {}. Check it shows this device as {fp}; waiting for it to approve.",
                    listed.approver_fingerprint.as_deref().unwrap_or("(key not yet known)"),
                    listed.number
                );
            }
        }
        match (status.status.as_str(), status.code) {
            ("code", Some(code)) => break (code, status.code_approver),
            ("declined", _) => {
                pending.forget();
                let by = status
                    .declined_by
                    .and_then(|d| d.approver_fingerprint)
                    .map(|fp| format!(" (node {fp})"))
                    .unwrap_or_default();
                return Err(format!(
                    "the request was declined by a node that speaks for you{by}; nothing was \
                     approved and this device is not enrolled"
                ));
            }
            ("none", _) => {
                pending.forget();
                return Err("the ask ended without an approval; ask again".into());
            }
            _ => {}
        }
        let _ = status.seconds_left;
        std::thread::sleep(Duration::from_secs(2));
    };
    let (code, state) = parse_pasted(&code).ok_or("the approving node's code was malformed")?;
    if state != pending.request.state {
        pending.forget();
        return Err("the code that arrived was not for this request".into());
    }
    println!("The approving node approved; collecting the consent over the private network.");
    let body = serde_json::to_vec(&serde_json::json!({
        "redemption": redemption_for(&pending, code),
    }))
    .map_err(|e| e.to_string())?;
    let delivered: Delivered = match json_call(
        "POST",
        &format!("{node}/auth/device/announce/redeem"),
        Some(&body),
    ) {
        Ok(d) => d,
        Err(e) => {
            pending.forget();
            return Err(format!(
                "the approving node did not hand over the consent: {e}"
            ));
        }
    };
    let expected = approver_key.or(proven_approver);
    finish(pending, delivered, node, expected, &pending_dir()?)
}

/// Take one `GET /callback?code=…&state=…` on the terminal's listener.
fn wait_for_callback(listener: &TcpListener) -> Outcome<(String, String)> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + WINDOW;
    while Instant::now() < deadline {
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
        let mut head = [0u8; 4096];
        let n = stream.read(&mut head).unwrap_or(0);
        let line = String::from_utf8_lossy(&head[..n]);
        let target = line.split_whitespace().nth(1).unwrap_or("");
        let found = target.strip_prefix("/callback?").and_then(|query| {
            let field = |name: &str| {
                query
                    .split('&')
                    .find_map(|kv| kv.strip_prefix(&format!("{name}=")[..]))
                    .map(str::to_string)
            };
            Some((field("code")?, field("state")?))
        });
        let (status, body) = match found {
            Some(_) => (
                "200 OK",
                "The code reached your terminal. You can close this page.",
            ),
            None => ("404 Not Found", "Not the callback."),
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        if let Some(pair) = found {
            return Ok(pair);
        }
    }
    Err("no code arrived within five minutes; ask again".into())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Refusal {
    error: Option<String>,
    code: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BindingReceipt {
    binding_action: String,
}

/// The origin (`scheme://host[:port]`) of a portal URL: the approving node
/// answers its routes at the root, whatever path its portal is served under
/// (`/auth/portal`).
fn origin_of(url: &str) -> String {
    match url.find("://") {
        Some(i) => {
            let rest = &url[i + 3..];
            let end = rest.find('/').unwrap_or(rest.len());
            format!("{}{}", &url[..i + 3], &rest[..end])
        }
        None => url.trim_end_matches('/').to_string(),
    }
}

fn redemption_for(pending: &Pending, code: &str) -> consent_grant::Redemption {
    let request = &pending.request;
    consent_grant::Redemption {
        code: code.into(),
        code_verifier: pending.code_verifier.clone(),
        client_id: request.client_id.clone(),
        device_key: request.device_key.clone(),
    }
}

fn redeem(pending: Pending, code: &str, node: &str) -> Outcome<ExitCode> {
    let body = serde_json::to_vec(&redemption_for(&pending, code)).map_err(|e| e.to_string())?;
    let url = format!(
        "{}/auth/consent/redeem",
        pending.approver.trim_end_matches('/')
    );
    // Unreached, nothing was spent: the request is kept and the same code can
    // be tried again. Any answer the node refuses with spends the code.
    let (status, _, bytes) = match http_with("POST", &url, Some(&body), &[]) {
        Ok(answer) => answer,
        Err(e) => {
            return Err(format!(
                "the approving node could not be reached ({e}); nothing was spent, so try the \
                 same code again, naming the node's http address with --approver <url> if \
                 needed"
            ))
        }
    };
    if !(200..300).contains(&status) {
        pending.forget();
        return Err(format!(
            "the approving node did not hand over the consent: {}",
            refusal_text(status, &bytes)
        ));
    }
    let delivered: Delivered = match serde_json::from_slice(&bytes) {
        Ok(d) => d,
        Err(e) => {
            pending.forget();
            return Err(format!("the approving node's answer is unreadable: {e}"));
        }
    };
    let expected = pending.approver_key.clone();
    finish(pending, delivered, node, expected, &pending_dir()?)
}

/// Check what was collected, have this node enroll with it, and report.
/// `expected` is the approving node's key when one is known (declared, or
/// proven over the carrier): it must be among the signers. The approving
/// node and the identity are shown before this device enrolls (adversarial
/// review, high 3); asking the person to confirm them is a recorded dial.
fn finish(
    pending: Pending,
    delivered: Delivered,
    node: &str,
    expected: Option<String>,
    dir: &Path,
) -> Outcome<ExitCode> {
    let request = &pending.request;
    if let Err(r) = check_delivered(&delivered, request, expected.as_deref()) {
        pending.forget_in(dir);
        return Err(format!(
            "{}: what the approving node handed over is not a valid consent for this request; nothing was signed",
            r.code()
        ));
    }
    let record = &delivered.consent.record;
    println!("Consent {}", delivered.consent.cid);
    let list = |acts: &[RequestedAct]| {
        if acts.is_empty() {
            "nothing".to_string()
        } else {
            acts.iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    println!("  agreed:   {}", list(&record.agreed_acts));
    println!("  declined: {}", list(&record.declined_acts()));
    if delivered.enrollment.is_none() {
        pending.forget_in(dir);
        println!("Enrolling this device was not agreed, so nothing was enrolled.");
        return Ok(ExitCode::SUCCESS);
    }
    let named: Vec<String> =
        consent_grant::verify::signers(&delivered, consent_grant::SignerRole::Controller)
            .iter()
            .map(|k| consent_grant::hash_shape::fingerprint(k))
            .collect();
    println!(
        "Approved by node {} for identity {}.",
        named.join(", "),
        consent_grant::hash_shape::fingerprint(&record.identity_root)
    );
    keep_then_enroll(pending, delivered, node, expected, dir)
}

/// Keep a checked consent on this device, then have its node enroll with it.
/// It is kept first, whole or not at all, so nothing between here and the
/// node's answer can lose it; it is forgotten once the node has enrolled.
fn keep_then_enroll(
    pending: Pending,
    delivered: Delivered,
    node: &str,
    expected: Option<String>,
    dir: &Path,
) -> Outcome<ExitCode> {
    let kept = KeptConsent {
        pending,
        delivered,
        expected,
        until_unix: now_unix() + WINDOW.as_secs(),
    };
    let not_kept = kept.keep_in(dir).err();
    enroll_with(kept, node, dir, not_kept)
}

/// Have this device's node enroll with a consent already checked and kept.
/// When the node cannot, the consent stays kept until its time on this
/// device runs out, so the next `join` enrolls with it without asking the
/// person again. `not_kept` says why it could not be kept, when it could not.
/// A kept consent is not checked again when it is used: it was checked before
/// it was kept, the file is this OS user's alone, and the node checks what it
/// enrolls with.
fn enroll_with(
    kept: KeptConsent,
    node: &str,
    dir: &Path,
    not_kept: Option<String>,
) -> Outcome<ExitCode> {
    let enroll = serde_json::to_vec(&serde_json::json!({
        "request": kept.pending.request,
        "delivered": kept.delivered,
        "approverKey": kept.expected,
    }))
    .map_err(|e| e.to_string())?;
    let binds_root = kept
        .delivered
        .consent
        .record
        .agreed_acts
        .contains(&RequestedAct::BindDeviceRoot);
    let receipt: BindingReceipt = match json_call(
        "POST",
        &format!("{}/auth/device/enroll", node.trim_end_matches('/')),
        Some(&enroll),
    ) {
        Ok(receipt) => receipt,
        Err(e) => {
            let why = format!("this device's node did not enroll it: {e}");
            let left = kept.until_unix.saturating_sub(now_unix());
            if let Some(k) = not_kept {
                kept.forget_in(dir);
                return Err(format!(
                    "{why}\nThe consent could not be kept on this device ({k}); run `epr device \
                     join` to ask again."
                ));
            }
            if left == 0 {
                kept.forget_in(dir);
                return Err(format!(
                    "{why}\nThe consent's five minutes on this device have passed, so it is no \
                     longer kept; run `epr device join` to ask again."
                ));
            }
            return Err(format!(
                "{why}\nThe consent is kept on this device; run `epr device join` again to enroll \
                 without asking again (it keeps {}:{:02} more).",
                left / 60,
                left % 60
            ));
        }
    };
    kept.forget_in(dir);
    println!(
        "This device is enrolled. Joining record: {}",
        receipt.binding_action
    );
    if binds_root {
        println!(
            "Binding this device's root key was agreed and is recorded in the consent, \
             but nothing on the network binds a root key yet."
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// One JSON call over plain HTTP, answered by a 2xx body of `T` or a named
/// refusal.
fn json_call<T: serde::de::DeserializeOwned>(
    method: &str,
    url: &str,
    body: Option<&[u8]>,
) -> Outcome<T> {
    json_call_with(method, url, body, &[]).map(|(value, _)| value)
}

/// [`json_call`] with extra request headers, also returning the response's
/// header block.
pub(crate) fn json_call_with<T: serde::de::DeserializeOwned>(
    method: &str,
    url: &str,
    body: Option<&[u8]>,
    headers: &[(&str, &str)],
) -> Outcome<(T, String)> {
    let (status, head, bytes) = http_with(method, url, body, headers)?;
    if (200..300).contains(&status) {
        let value =
            serde_json::from_slice(&bytes).map_err(|e| format!("unreadable answer: {e}"))?;
        return Ok((value, head));
    }
    Err(refusal_text(status, &bytes))
}

/// A node's refusal as a person reads it: its code, then its plain sentence.
pub(crate) fn refusal_text(status: u16, bytes: &[u8]) -> String {
    match serde_json::from_slice::<Refusal>(bytes) {
        Ok(Refusal {
            error,
            code: Some(code),
        }) => format!("{code}: {}", error.unwrap_or_default()),
        _ => format!("HTTP {status}: {}", String::from_utf8_lossy(bytes)),
    }
}

/// A minimal HTTP/1.1 client for `http://` nodes. An approving node or device node is
/// reached on this machine or the person's own network; TLS is not spoken here.
#[cfg(test)]
fn http(method: &str, url: &str, body: Option<&[u8]>) -> Outcome<(u16, Vec<u8>)> {
    http_with(method, url, body, &[]).map(|(status, _, body)| (status, body))
}

/// [`http`] with extra request headers, returning the status, the header
/// block and the body.
pub(crate) fn http_with(
    method: &str,
    url: &str,
    body: Option<&[u8]>,
    headers: &[(&str, &str)],
) -> Outcome<(u16, String, Vec<u8>)> {
    http_timed(method, url, body, headers, CALL_TIMEOUT)
}

/// [`http_with`], giving up after `timeout` to connect and after `timeout`
/// of silence: a probe must not wait as long as a call that does work.
pub(crate) fn http_timed(
    method: &str,
    url: &str,
    body: Option<&[u8]>,
    headers: &[(&str, &str)],
    timeout: Duration,
) -> Outcome<(u16, String, Vec<u8>)> {
    let rest = url.strip_prefix("http://").ok_or_else(|| {
        format!("{url}: only http:// node addresses are reachable from this terminal")
    })?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let mut last = format!("{authority}: no address");
    let mut connected = None;
    for addr in authority
        .to_socket_addrs()
        .map_err(|e| format!("{authority}: {e}"))?
    {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => {
                connected = Some(stream);
                break;
            }
            Err(e) => last = format!("{authority}: {e}"),
        }
    }
    let mut stream = connected.ok_or(last)?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let body = body.unwrap_or_default();
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {authority}\r\nAccept: application/json\r\nConnection: close\r\n"
    );
    if method != "GET" {
        request.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        ));
    }
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .and_then(|()| stream.write_all(body))
        .map_err(|e| format!("{authority}: {e}"))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|e| format!("{authority}: {e}"))?;
    parse_response_with_head(&raw)
}

#[cfg(test)]
fn parse_response(raw: &[u8]) -> Outcome<(u16, Vec<u8>)> {
    parse_response_with_head(raw).map(|(status, _, body)| (status, body))
}

fn parse_response_with_head(raw: &[u8]) -> Outcome<(u16, String, Vec<u8>)> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("malformed HTTP answer")?;
    let head = String::from_utf8_lossy(&raw[..split]);
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or("malformed HTTP status")?;
    let body = &raw[split + 4..];
    let chunked = head.lines().any(|l| {
        l.to_ascii_lowercase().starts_with("transfer-encoding:")
            && l.to_ascii_lowercase().contains("chunked")
    });
    let body = if chunked {
        dechunk(body)?
    } else {
        body.to_vec()
    };
    Ok((status, head.into_owned(), body))
}

fn dechunk(mut body: &[u8]) -> Outcome<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let line_end = body
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("malformed chunk")?;
        let size_text = String::from_utf8_lossy(&body[..line_end]);
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| "malformed chunk size")?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if body.len() < size {
            return Err("truncated chunk".into());
        }
        out.extend_from_slice(&body[..size]);
        body = body.get(size + 2..).unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_root_without_enrollment_is_refused_by_the_crates_rule() {
        let refused = ask(&[
            "--portal".into(),
            "http://127.0.0.1:1".into(),
            "--label".into(),
            "workspace".into(),
            "--act".into(),
            "device.bind-root".into(),
        ])
        .unwrap_err();
        assert!(refused.starts_with("request_acts_incoherent"), "{refused}");
    }

    #[test]
    fn an_unknown_act_is_refused_before_anything_is_asked() {
        let refused = ask(&[
            "--portal".into(),
            "http://127.0.0.1:1".into(),
            "--label".into(),
            "workspace".into(),
            "--act".into(),
            "content.publish".into(),
        ])
        .unwrap_err();
        assert!(refused.contains("act_unknown"), "{refused}");
    }

    #[test]
    fn with_no_portal_the_device_announces_and_never_assumes_an_approver() {
        // Port 1 never answers: with no portal the terminal asks its own node
        // to announce, and prints no link to anyone.
        let refused = ask(&[
            "--label".into(),
            "workspace".into(),
            "--node".into(),
            "http://127.0.0.1:1".into(),
        ])
        .unwrap_err();
        assert!(refused.contains("node did not answer"), "{refused}");
        let refused = ask(&[
            "--label".into(),
            "workspace".into(),
            "--announce".into(),
            "--loopback".into(),
        ])
        .unwrap_err();
        assert!(refused.contains("--loopback"), "{refused}");
    }

    #[test]
    fn answers_are_read_plain_and_chunked() {
        let plain = b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(parse_response(plain).unwrap(), (201, b"{}".to_vec()));
        let chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n{\"a\r\n4\r\n\":1}\r\n0\r\n\r\n";
        assert_eq!(
            parse_response(chunked).unwrap(),
            (200, b"{\"a\":1}".to_vec())
        );
        assert!(parse_response(b"garbage").is_err());
    }

    #[test]
    fn only_plain_http_nodes_are_reached() {
        assert!(
            http("GET", "https://approver.example/auth/consent/redeem", None)
                .unwrap_err()
                .contains("only http://")
        );
    }

    #[test]
    fn a_pasted_state_cannot_name_a_path() {
        assert!(Pending::load("../../etc/passwd").is_err());
    }

    /// A node on a port of this machine, answering each request by `answer`
    /// and recording each as `METHOD PATH BODY`.
    fn stub(
        answer: impl Fn(&str, &str, usize) -> (u16, String) + Send + 'static,
    ) -> (u16, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        use std::sync::{Arc, Mutex};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let record = seen.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                let head_end = loop {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break None;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    if let Some(i) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                        break Some(i + 4);
                    }
                };
                let Some(head_end) = head_end else { continue };
                let head = String::from_utf8_lossy(&raw[..head_end]).to_string();
                let length: usize = head
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap_or(0))
                    })
                    .unwrap_or(0);
                while raw.len() < head_end + length {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                }
                let mut words = head.split_whitespace();
                let method = words.next().unwrap_or("").to_string();
                let path = words.next().unwrap_or("").to_string();
                let body = String::from_utf8_lossy(&raw[head_end..]).to_string();
                let count = {
                    let mut seen = record.lock().unwrap();
                    seen.push(format!("{method} {path} {body}"));
                    seen.iter()
                        .filter(|r| r.starts_with(&format!("{method} {path} ")))
                        .count()
                };
                let (status, reply) = answer(&method, &path, count);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        (port, seen)
    }

    /// A port of this machine nothing listens on.
    fn closed_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn a_node(_: &str, path: &str, _: usize) -> (u16, String) {
        match path {
            "/auth/device/self" => (
                200,
                r#"{"deviceKey":"uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl",
                    "networkDna":"uhC0kQwOEwmIBZhBT3I7vGZPz0kEL3_hqavyFW0upoO_hyEwuEglj",
                    "contentDna":"uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt"}"#
                    .into(),
            ),
            "/health" => (200, "{}".into()),
            _ => (404, String::new()),
        }
    }

    const QUICK: Duration = Duration::from_millis(500);

    #[test]
    fn discovery_uses_the_one_node_that_answers_and_never_guesses() {
        let (one, _) = stub(a_node);
        let (two, _) = stub(a_node);
        // Something on a port that is not a node does not count.
        let (other, _) = stub(|_, _, _| (404, String::new()));
        let none = closed_port();
        assert_eq!(
            find_node(None, None, None, &[none, other, one], QUICK).unwrap(),
            format!("http://127.0.0.1:{one}")
        );
        let refused = find_node(None, None, None, &[one, none, two], QUICK).unwrap_err();
        assert!(refused.contains("more than one node"), "{refused}");
        assert!(refused.contains(&format!("127.0.0.1:{one}")), "{refused}");
        assert!(refused.contains(&format!("127.0.0.1:{two}")), "{refused}");
        let refused = find_node(None, None, None, &[none, other], QUICK).unwrap_err();
        assert_eq!(
            refused,
            format!(
                "no node of this machine answers on {none}/{other}; start one (`just dev start`) \
                 or name it with --node"
            )
        );
        // Named comes before found: the flag, then the environment, then the
        // launcher's record, and nothing is probed.
        assert_eq!(
            find_node(
                Some("http://a:1/"),
                Some("http://b:2"),
                Some(3),
                &[one],
                QUICK
            )
            .unwrap(),
            "http://a:1"
        );
        assert_eq!(
            find_node(None, Some("http://b:2"), Some(3), &[one], QUICK).unwrap(),
            "http://b:2"
        );
        assert_eq!(
            find_node(None, Some(" "), Some(8095), &[one, two], QUICK).unwrap(),
            "http://127.0.0.1:8095"
        );
    }

    #[test]
    fn the_launchers_ports_file_names_storage_only_when_it_records_it() {
        assert_eq!(storage_port_in("admin_port=39097\napp_port=4485\n"), None);
        assert_eq!(
            storage_port_in("admin_port=39097\nstorage_port=8095\n"),
            Some(8095)
        );
    }

    #[test]
    fn a_node_built_before_device_joining_is_refused_before_anything_is_asked() {
        let (old, seen) = stub(|_, path, _| match path {
            "/health" => (200, "{}".into()),
            _ => (404, String::new()),
        });
        let node = format!("http://127.0.0.1:{old}");
        let refused = capable(&node).unwrap_err();
        assert!(refused.contains("predates device joining"), "{refused}");
        let refused = ask(&[
            "--label".into(),
            "workspace".into(),
            "--portal".into(),
            "http://127.0.0.1:1".into(),
            "--node".into(),
            node.clone(),
        ])
        .unwrap_err();
        assert!(refused.contains("predates device joining"), "{refused}");
        let refused = join(&["--label".into(), "w".into(), "--node".into(), node]).unwrap_err();
        assert!(refused.contains("predates device joining"), "{refused}");
        assert!(
            seen.lock().unwrap().iter().all(|r| r.starts_with("GET ")),
            "nothing was asked of it"
        );
        let (current, _) = stub(a_node);
        assert!(capable(&format!("http://127.0.0.1:{current}")).is_ok());
    }

    #[test]
    fn an_https_approver_is_refused_before_any_network_call() {
        // No --node: had anything been reached for, discovery would have run.
        for command in [ask as fn(&[String]) -> Outcome<ExitCode>, join] {
            let refused = command(&[
                "--portal".into(),
                "https://doorway-alpha.elohim.host/threshold".into(),
                "--label".into(),
                "workspace".into(),
            ])
            .unwrap_err();
            assert_eq!(refused, HTTPS_APPROVER);
            let refused = command(&["--approver".into(), "https://x.example".into()]).unwrap_err();
            assert_eq!(refused, HTTPS_APPROVER);
        }
    }

    #[test]
    fn an_approver_without_the_consent_screen_is_refused() {
        let (none, _) = stub(|_, _, _| (404, String::new()));
        let refused = approver_takes_approvals(&format!("http://127.0.0.1:{none}")).unwrap_err();
        assert_eq!(
            refused,
            format!("the node at http://127.0.0.1:{none} does not take device approvals")
        );
        let (some, seen) = stub(|_, path, _| match path {
            "/auth/consent/view" => (400, r#"{"code":"request_unreadable"}"#.into()),
            "/auth/consent/collect" => (400, r#"{"code":"bad"}"#.into()),
            _ => (404, String::new()),
        });
        let approver = format!("http://127.0.0.1:{some}");
        approver_takes_approvals(&approver).unwrap();
        assert!(approver_collects(&approver).unwrap());
        assert!(seen.lock().unwrap()[0].starts_with("POST /auth/consent/view {}"));
        // One built before collect: the join falls back to carrying a code.
        assert!(!approver_collects(&format!("http://127.0.0.1:{none}")).unwrap());
    }

    #[test]
    fn an_approver_reached_by_a_name_it_does_not_answer_says_which_name_to_add() {
        let misdirected = r#"{"code":"host_not_this_node","error":"only its own names"}"#;
        let (port, _) = stub(move |_, _, _| (421, misdirected.into()));
        let approver = format!("http://127.0.0.1:{port}");
        let words = format!(
            "the node at {approver} answers only under its own names; add 127.0.0.1 to \
             ELOHIM_ALLOWED_HOSTS on that node, or approve there with `epr device approve '<link>'`"
        );
        assert_eq!(approver_takes_approvals(&approver).unwrap_err(), words);
        assert_eq!(approver_collects(&approver).unwrap_err(), words);
        // While waiting, the ask is kept: once the name is added, join picks
        // it up without a new yes.
        let dir = scratch();
        let mut pending = waiting(port);
        pending.save_in(dir.path()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let refused = collect(
            &mut pending,
            Duration::from_millis(10),
            deadline,
            &mut |_| {},
            dir.path(),
        )
        .unwrap_err();
        assert!(refused.starts_with(&words), "{refused}");
        assert!(refused.contains("The ask is kept"), "{refused}");
        assert!(Pending::path_in(dir.path(), &pending.request.state).exists());
        for (origin, host) in [
            ("http://10.1.19.170:8095/auth/portal", "10.1.19.170"),
            ("http://che-shem.local:8090", "che-shem.local"),
            ("http://[fd00::1]:8095", "fd00::1"),
            ("http://shem", "shem"),
        ] {
            assert_eq!(host_of(origin), host, "{origin}");
        }
    }

    fn waiting(approver: u16) -> Pending {
        Pending {
            request: GrantRequest {
                domain: GRANT_DOMAIN.into(),
                client_id: CLIENT_ID.into(),
                device_key: "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl".into(),
                device_root_key: None,
                label: "che-shem".into(),
                network_dna: "uhC0kQwOEwmIBZhBT3I7vGZPz0kEL3_hqavyFW0upoO_hyEwuEglj".into(),
                content_dna: "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt".into(),
                acts: vec![RequestedAct::EnrollDevice],
                code_challenge: pkce::challenge(&"v".repeat(43)),
                state: token(32),
                return_path: ReturnPath::Hold,
            },
            code_verifier: "v".repeat(43),
            approver: format!("http://127.0.0.1:{approver}"),
            node: "http://127.0.0.1:1".into(),
            asked_at_unix: now_unix(),
            approver_key: None,
            announced: false,
            waiting_until_unix: None,
        }
    }

    #[test]
    fn join_collects_on_200_saying_each_wait_once() {
        let (approver, seen) = stub(|_, _, n| match n {
            1 | 2 => (
                404,
                r#"{"code":"consent_unknown","error":"not seen"}"#.into(),
            ),
            3 | 4 => (202, r#"{"status":"waiting","secondsLeft":241}"#.into()),
            _ => (200, r#"{"delivered":true}"#.into()),
        });
        let mut pending = waiting(approver);
        let mut said = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(20);
        let bytes = collect(
            &mut pending,
            Duration::from_millis(10),
            deadline,
            &mut |l| said.push(l.to_string()),
            scratch().path(),
        )
        .unwrap();
        assert_eq!(bytes, br#"{"delivered":true}"#);
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].contains("has not seen this ask yet"), "{said:?}");
        assert!(
            said[1].contains("waiting for your yes there (4:01 left)"),
            "{said:?}"
        );
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 5);
        let body: serde_json::Value =
            serde_json::from_str(seen[0].strip_prefix("POST /auth/consent/collect ").unwrap())
                .unwrap();
        assert_eq!(body["state"], pending.request.state);
        assert_eq!(body["codeVerifier"], pending.code_verifier);
        assert_eq!(body["clientId"], CLIENT_ID);
        assert_eq!(body["deviceKey"], pending.request.device_key);
    }

    #[test]
    fn join_stops_on_410_saying_which_and_on_403() {
        for (code, words) in [
            ("consent_declined", "you said no"),
            ("consent_spent", "already collected"),
            ("consent_expired", "five minutes ended"),
        ] {
            let reply = format!(r#"{{"code":"{code}","error":"ended"}}"#);
            let (approver, seen) = stub(move |_, _, n| match n {
                1 => (202, r#"{"status":"waiting","secondsLeft":60}"#.into()),
                _ => (410, reply.clone()),
            });
            let deadline = Instant::now() + Duration::from_secs(20);
            let refused = collect(
                &mut waiting(approver),
                Duration::from_millis(10),
                deadline,
                &mut |_| {},
                scratch().path(),
            )
            .unwrap_err();
            assert!(refused.contains(words), "{code}: {refused}");
            assert_eq!(seen.lock().unwrap().len(), 2, "stopped at the 410");
        }
        let (approver, seen) =
            stub(|_, _, _| (403, r#"{"code":"verifier_mismatch","error":"no"}"#.into()));
        let deadline = Instant::now() + Duration::from_secs(20);
        let refused = collect(
            &mut waiting(approver),
            Duration::from_millis(10),
            deadline,
            &mut |_| {},
            scratch().path(),
        )
        .unwrap_err();
        assert!(refused.contains("verifier_mismatch"), "{refused}");
        assert_eq!(seen.lock().unwrap().len(), 1);
        // An ask never seen by the approving node is said so when the window
        // closes.
        let (approver, _) = stub(|_, _, _| (404, r#"{"code":"consent_unknown"}"#.into()));
        let deadline = Instant::now() + Duration::from_millis(100);
        let refused = collect(
            &mut waiting(approver),
            Duration::from_millis(10),
            deadline,
            &mut |_| {},
            scratch().path(),
        )
        .unwrap_err();
        assert!(refused.contains("never saw this ask"), "{refused}");
    }

    /// A private directory for asks and consents, gone when the test ends.
    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    /// A consent shaped as the approving node hands one over. Its signatures
    /// are not real: what is tested here comes after it was checked.
    fn handed_over(pending: &Pending) -> Delivered {
        let r = &pending.request;
        let agreed_at_micros = i64::try_from(now_unix()).unwrap() * 1_000_000;
        let record = consent_grant::ConsentRecord {
            domain: GRANT_DOMAIN.into(),
            client_id: r.client_id.clone(),
            identity_root: "uhCkkzY_ZvJaVbaFzi46J_LbGgFEUEQVMlWN5rpSdxGYdOG_3sQYp".into(),
            authority: "uhCkkN_k9u6glm7eRydJ_WWbyUSSWbBYMHd_6aWO4ag-PrcWHA5_-".into(),
            device_key: r.device_key.clone(),
            device_root_key: None,
            network_dna: r.network_dna.clone(),
            content_dna: r.content_dna.clone(),
            asked_acts: r.acts.clone(),
            agreed_acts: r.acts.clone(),
            agreed_at_micros,
            request_binding: r.code_challenge.clone(),
        };
        let intent = consent_grant::enrollment::EnrollmentIntent::agreed_in(&record).unwrap();
        Delivered {
            consent: consent_grant::SignedConsent {
                cid: "bafykept".into(),
                record,
                signatures: vec![consent_grant::ConsentSignature {
                    role: consent_grant::SignerRole::Controller,
                    signer: "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl".into(),
                    signature: "c2ln".into(),
                }],
            },
            enrollment: Some(consent_grant::enrollment::Enrollment {
                intent,
                controllers: vec![consent_grant::enrollment::ControllerProof {
                    agent: "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl".into(),
                    signature: "c2ln".into(),
                }],
                approved_via: vec![],
            }),
        }
    }

    #[test]
    fn a_consent_the_node_could_not_enroll_with_is_kept_for_its_window() {
        let dir = scratch();
        let (node, _) = stub(|method, path, n| match path {
            "/auth/device/enroll" => (
                503,
                r#"{"code":"consent_signing_unavailable",
                    "error":"this node cannot reach the key that would sign; try again shortly"}"#
                    .into(),
            ),
            _ => a_node(method, path, n),
        });
        let node = format!("http://127.0.0.1:{node}");
        let mut pending = waiting(closed_port());
        pending.node = node.clone();
        pending.save_in(dir.path()).unwrap();
        let state = pending.request.state.clone();
        let delivered = handed_over(&pending);
        let refused = keep_then_enroll(pending, delivered, &node, None, dir.path()).unwrap_err();
        assert!(refused.contains("consent_signing_unavailable"), "{refused}");
        assert!(
            refused.contains(
                "The consent is kept on this device; run `epr device join` again to enroll \
                 without asking again"
            ),
            "{refused}"
        );
        let kept = KeptConsent::path_in(dir.path(), &state);
        assert!(kept.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&kept).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        // The approving node saying the code is spent forgets the ask, never
        // the consent kept beside it.
        let (approver, _) = stub(|_, _, _| (410, r#"{"code":"consent_spent"}"#.into()));
        let mut asked = KeptConsent::find_in(dir.path(), &node, now_unix())
            .pop()
            .unwrap()
            .pending;
        asked.approver = format!("http://127.0.0.1:{approver}");
        let deadline = Instant::now() + Duration::from_secs(20);
        let refused = collect(
            &mut asked,
            Duration::from_millis(10),
            deadline,
            &mut |_| {},
            dir.path(),
        )
        .unwrap_err();
        assert!(refused.contains("already collected"), "{refused}");
        assert!(!Pending::path_in(dir.path(), &state).exists());
        assert!(kept.exists());
    }

    #[test]
    fn the_next_join_enrolls_with_a_kept_consent_without_asking_again() {
        let dir = scratch();
        let (approver, asked_of_approver) = stub(|_, _, _| (500, String::new()));
        let (node, asked_of_node) = stub(|method, path, n| match path {
            "/auth/device/enroll" => (200, r#"{"bindingAction":"uhCkkjoining"}"#.into()),
            _ => a_node(method, path, n),
        });
        let node = format!("http://127.0.0.1:{node}");
        let mut pending = waiting(approver);
        pending.node = node.clone();
        pending.save_in(dir.path()).unwrap();
        let state = pending.request.state.clone();
        let delivered = handed_over(&pending);
        KeptConsent {
            pending,
            delivered,
            expected: None,
            until_unix: now_unix() + 120,
        }
        .keep_in(dir.path())
        .unwrap();
        join_in(
            &[
                "--label".into(),
                "che-shem".into(),
                "--approver".into(),
                format!("http://127.0.0.1:{approver}"),
                "--node".into(),
                node,
            ],
            dir.path(),
        )
        .unwrap();
        assert!(asked_of_approver.lock().unwrap().is_empty());
        let asked = asked_of_node.lock().unwrap();
        assert!(
            asked
                .iter()
                .any(|r| r.starts_with("POST /auth/device/enroll {")),
            "{asked:?}"
        );
        assert!(!KeptConsent::path_in(dir.path(), &state).exists());
        assert!(!Pending::path_in(dir.path(), &state).exists());
    }

    #[test]
    fn a_kept_consent_past_its_window_is_forgotten_and_join_asks_afresh() {
        let dir = scratch();
        let (approver, asked_of_approver) = stub(|_, path, n| match (path, n) {
            ("/auth/consent/view", _) => (400, r#"{"code":"request_unreadable"}"#.into()),
            ("/auth/consent/collect", 1) => (400, r#"{"code":"bad"}"#.into()),
            ("/auth/consent/collect", _) => (410, r#"{"code":"consent_declined"}"#.into()),
            _ => (404, String::new()),
        });
        let (node, asked_of_node) = stub(a_node);
        let node = format!("http://127.0.0.1:{node}");
        let mut pending = waiting(approver);
        pending.node = node.clone();
        pending.save_in(dir.path()).unwrap();
        let state = pending.request.state.clone();
        let delivered = handed_over(&pending);
        KeptConsent {
            pending,
            delivered,
            expected: None,
            until_unix: now_unix() - 1,
        }
        .keep_in(dir.path())
        .unwrap();
        let refused = join_in(
            &[
                "--label".into(),
                "che-shem".into(),
                "--approver".into(),
                format!("http://127.0.0.1:{approver}"),
                "--node".into(),
                node,
            ],
            dir.path(),
        )
        .unwrap_err();
        assert!(refused.contains("you said no"), "{refused}");
        assert!(!KeptConsent::path_in(dir.path(), &state).exists());
        assert!(!Pending::path_in(dir.path(), &state).exists());
        let asked = asked_of_approver.lock().unwrap();
        assert!(
            asked[0].starts_with("POST /auth/consent/view {}"),
            "{asked:?}"
        );
        assert_eq!(
            asked
                .iter()
                .filter(|r| r.starts_with("POST /auth/consent/collect {\""))
                .count(),
            1,
            "a fresh ask was collected for: {asked:?}"
        );
        assert!(!asked_of_node
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.starts_with("POST /auth/device/enroll")));
    }

    /// The approving node a fresh ask is made against: it takes approvals,
    /// collects, and the person says no, so the ask ends at once.
    fn declining_approver() -> (u16, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        stub(|_, path, n| match (path, n) {
            ("/auth/consent/view", _) => (400, r#"{"code":"request_unreadable"}"#.into()),
            ("/auth/consent/collect", 1) => (400, r#"{"code":"bad"}"#.into()),
            ("/auth/consent/collect", _) => (410, r#"{"code":"consent_declined"}"#.into()),
            _ => (404, String::new()),
        })
    }

    fn join_args(approver: u16, node: &str) -> Vec<String> {
        vec![
            "--label".into(),
            "che-shem".into(),
            "--approver".into(),
            format!("http://127.0.0.1:{approver}"),
            "--node".into(),
            node.into(),
        ]
    }

    /// The `.json` files kept in `dir`.
    fn kept_files(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .map(|d| {
                d.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn the_wait_follows_the_approving_node_not_the_time_asked() {
        // Already past this device's own deadline: the node's word extends it.
        let dir = scratch();
        let (approver, _) = stub(|_, _, n| match n {
            1 => (202, r#"{"status":"waiting","secondsLeft":400}"#.into()),
            _ => (200, r#"{"delivered":true}"#.into()),
        });
        let mut pending = waiting(approver);
        pending.save_in(dir.path()).unwrap();
        let bytes = collect(
            &mut pending,
            Duration::from_millis(10),
            Instant::now(),
            &mut |_| {},
            dir.path(),
        )
        .unwrap();
        assert_eq!(bytes, br#"{"delivered":true}"#);
        let kept: Pending = serde_json::from_slice(
            &fs::read(Pending::path_in(dir.path(), &pending.request.state)).unwrap(),
        )
        .unwrap();
        assert!(kept.waiting_until_unix.unwrap() >= now_unix() + 400);
        assert!(kept.open_until() > kept.asked_at_unix + WINDOW.as_secs());

        // An ask made long ago that the node still shows is picked up, not
        // discarded at five minutes from when it was made.
        let dir = scratch();
        let (approver, asked_of_approver) =
            stub(|_, _, _| (410, r#"{"code":"consent_declined"}"#.into()));
        let (node, _) = stub(a_node);
        let node = format!("http://127.0.0.1:{node}");
        let mut old = waiting(approver);
        old.node = node.clone();
        old.asked_at_unix = now_unix() - 400;
        old.waiting_until_unix = Some(now_unix() + 60);
        old.save_in(dir.path()).unwrap();
        let refused = join_in(&join_args(approver, &node), dir.path()).unwrap_err();
        assert!(refused.contains("you said no"), "{refused}");
        let asked = asked_of_approver.lock().unwrap();
        assert_eq!(asked.len(), 1, "{asked:?}");
        assert!(
            asked[0].contains(&old.request.state),
            "the old ask was collected: {asked:?}"
        );
    }

    #[test]
    fn a_kept_ask_is_the_same_ask_only_when_everything_asked_matches() {
        let pending = waiting(8191);
        let intent = Intent {
            label: "che-shem".into(),
            portal: None,
            approver: "http://127.0.0.1:8191/".into(),
            approver_key: None,
            acts: vec![RequestedAct::EnrollDevice],
            announcing: false,
        };
        assert!(pending.is_ask(&intent));
        let differs: [fn(&mut Intent); 4] = [
            |i| i.label = "home".into(),
            |i| i.approver = "http://127.0.0.1:8192".into(),
            |i| i.acts.push(RequestedAct::BindDeviceRoot),
            |i| i.announcing = true,
        ];
        for change in differs {
            let mut other = Intent {
                label: intent.label.clone(),
                portal: None,
                approver: intent.approver.clone(),
                approver_key: None,
                acts: intent.acts.clone(),
                announcing: intent.announcing,
            };
            change(&mut other);
            assert!(!pending.is_ask(&other));
        }
    }

    #[test]
    fn a_different_kept_ask_or_consent_is_left_and_join_asks_afresh() {
        let dir = scratch();
        let (approver, asked_of_approver) = declining_approver();
        let (node, asked_of_node) = stub(a_node);
        let node = format!("http://127.0.0.1:{node}");
        let mut other_ask = waiting(approver);
        other_ask.node = node.clone();
        other_ask.request.label = "home".into();
        other_ask.save_in(dir.path()).unwrap();
        let mut other_consent = waiting(approver);
        other_consent.node = node.clone();
        other_consent.request.label = "home".into();
        let delivered = handed_over(&other_consent);
        let consent_state = other_consent.request.state.clone();
        KeptConsent {
            pending: other_consent,
            delivered,
            expected: None,
            until_unix: now_unix() + 120,
        }
        .keep_in(dir.path())
        .unwrap();
        let refused = join_in(&join_args(approver, &node), dir.path()).unwrap_err();
        assert!(refused.contains("you said no"), "{refused}");
        // Asked afresh: the approving node was probed and a new ask collected.
        let asked = asked_of_approver.lock().unwrap();
        assert!(
            asked[0].starts_with("POST /auth/consent/view {}"),
            "{asked:?}"
        );
        assert!(!asked.iter().any(|r| r.contains(&other_ask.request.state)));
        assert!(!asked_of_node
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.starts_with("POST /auth/device/enroll")));
        // Both are inside their windows, so both are left as they were.
        assert!(Pending::path_in(dir.path(), &other_ask.request.state).exists());
        assert!(KeptConsent::path_in(dir.path(), &consent_state).exists());
    }

    #[test]
    fn a_consent_is_kept_before_enrolling_and_by_this_devices_clock() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let dir = scratch();
        let mut pending = waiting(closed_port());
        let state = pending.request.state.clone();
        let kept_path = KeptConsent::path_in(dir.path(), &state);
        let was_kept = Arc::new(AtomicBool::new(false));
        let (node, _) = {
            let kept_path = kept_path.clone();
            let was_kept = was_kept.clone();
            stub(move |method, path, n| match path {
                "/auth/device/enroll" => {
                    was_kept.store(kept_path.exists(), Ordering::SeqCst);
                    (200, r#"{"bindingAction":"uhCkkjoining"}"#.into())
                }
                _ => a_node(method, path, n),
            })
        };
        let node = format!("http://127.0.0.1:{node}");
        pending.node = node.clone();
        pending.save_in(dir.path()).unwrap();
        // An approving node whose clock is an hour behind does not shorten it.
        let mut delivered = handed_over(&pending);
        delivered.consent.record.agreed_at_micros -= 3_600_000_000;
        keep_then_enroll(pending, delivered, &node, None, dir.path()).unwrap();
        assert!(
            was_kept.load(Ordering::SeqCst),
            "kept before the node was asked"
        );
        assert!(!kept_path.exists(), "forgotten once enrolled");

        let dir = scratch();
        let (node, _) = stub(|method, path, n| match path {
            "/auth/device/enroll" => (503, r#"{"code":"consent_signing_unavailable"}"#.into()),
            _ => a_node(method, path, n),
        });
        let node = format!("http://127.0.0.1:{node}");
        let mut pending = waiting(closed_port());
        pending.node = node.clone();
        let state = pending.request.state.clone();
        let mut delivered = handed_over(&pending);
        delivered.consent.record.agreed_at_micros -= 3_600_000_000;
        let refused = keep_then_enroll(pending, delivered, &node, None, dir.path()).unwrap_err();
        assert!(refused.contains("The consent is kept"), "{refused}");
        let mut kept: KeptConsent =
            serde_json::from_slice(&fs::read(KeptConsent::path_in(dir.path(), &state)).unwrap())
                .unwrap();
        assert!(kept.until_unix + 5 >= now_unix() + WINDOW.as_secs());
        // Keeping again replaces the file whole, leaving nothing beside it.
        kept.until_unix += 1;
        kept.keep_in(dir.path()).unwrap();
        let again: KeptConsent =
            serde_json::from_slice(&fs::read(KeptConsent::path_in(dir.path(), &state)).unwrap())
                .unwrap();
        assert_eq!(again.until_unix, kept.until_unix);
        assert!(
            kept_files(dir.path()).iter().all(|f| !f.ends_with(".tmp")),
            "{:?}",
            kept_files(dir.path())
        );
    }

    #[test]
    fn probes_settle_only_on_answers_that_say_something() {
        for status in [401u16, 302, 429, 503] {
            let (port, _) = stub(move |_, _, _| (status, String::new()));
            let approver = format!("http://127.0.0.1:{port}");
            assert_eq!(
                approver_takes_approvals(&approver).unwrap_err(),
                format!(
                    "could not tell whether {approver} takes approvals (answered {status}); try \
                     again shortly"
                )
            );
            assert!(approver_collects(&approver)
                .unwrap_err()
                .starts_with("could not tell"));
        }
        let (ok, _) = stub(|_, _, _| (200, "{}".into()));
        let ok = format!("http://127.0.0.1:{ok}");
        approver_takes_approvals(&ok).unwrap();
        // For collect only a refused empty collection says the route is there.
        assert!(approver_collects(&ok)
            .unwrap_err()
            .contains("(answered 200)"));
        // No answer at all is an unreachable node, never an old build.
        let gone = format!("http://127.0.0.1:{}", closed_port());
        assert!(approver_collects(&gone)
            .unwrap_err()
            .contains("does not answer"));
        // An approving node that settles nothing: no ask is made or kept.
        let dir = scratch();
        let (approver, _) = stub(|_, _, _| (503, String::new()));
        let (node, _) = stub(a_node);
        let node = format!("http://127.0.0.1:{node}");
        let refused = join_in(&join_args(approver, &node), dir.path()).unwrap_err();
        assert!(refused.starts_with("could not tell"), "{refused}");
        assert!(kept_files(dir.path()).is_empty());
    }

    #[test]
    fn the_approving_node_is_the_portals_origin_not_its_path() {
        assert_eq!(
            origin_of("https://10.1.19.170:9443/auth/portal"),
            "https://10.1.19.170:9443"
        );
        assert_eq!(origin_of("http://127.0.0.1:8191/"), "http://127.0.0.1:8191");
        assert_eq!(origin_of("http://127.0.0.1:8191"), "http://127.0.0.1:8191");
    }
}
