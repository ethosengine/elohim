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
//! The approving node is never assumed: it is named by a portal, declared, or
//! found on the private network by local discovery. No doorway is involved
//! unless one is named.
//!
//! The rules are `consent_grant`'s; this module only asks, keeps, checks and
//! reports. The device's key never leaves its node: the node signs possession
//! and notarizes the joining record itself, asked over node-local HTTP
//! (`/auth/device/self`, `/auth/device/enroll`, `/auth/device/announce`).

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
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
/// Where a terminal on this machine reaches its own node unless told otherwise.
const DEFAULT_NODE: &str = "http://127.0.0.1:8090";
/// How long a code waits, matching the approving node's five-minute window.
const WINDOW: Duration = Duration::from_secs(5 * 60);
/// How long one call to a node may take: a node's answer can wait on the network.
const CALL_TIMEOUT: Duration = Duration::from_secs(180);

type Outcome<T> = Result<T, String>;

pub fn usage() -> &'static str {
    "usage:\n  epr device ask [--portal <approving node's portal URL>] --label <name> \
     [--act device.enroll] [--act device.bind-root] [--loopback | --announce] \
     [--approver <approving node URL>] [--node <this node URL>]\n    \
     (with no portal named or declared, the device announces on its private network)\n  \
     epr device redeem '<code#state>' [--approver <approving node URL>] [--node <this node URL>]\n  \
     epr device pending [--node <this node URL>]\n  \
     epr device approve '<link | number | key fingerprint>' [--only <act>]... [--yes] [--node <this node URL>]\n  \
     epr device approve <number | key fingerprint> --decline"
}

pub fn run(args: &[String]) -> Outcome<ExitCode> {
    match args.first().map(String::as_str) {
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
            redeem(pending, code, &node)
        }
        Some("--help" | "-h" | "help") | None => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        Some(other) => Err(format!("unknown device command `{other}`\n{}", usage())),
    }
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

impl Pending {
    fn path(state: &str) -> Outcome<PathBuf> {
        Ok(pending_dir()?.join(format!("{state}.json")))
    }

    fn save(&self) -> Outcome<()> {
        let dir = pending_dir()?;
        fs::create_dir_all(&dir).map_err(|e| format!("cannot keep the request: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(Self::path(&self.request.state)?)
            .map_err(|e| format!("cannot keep the request: {e}"))?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .map_err(|e| format!("cannot keep the request: {e}"))
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

fn ask(args: &[String]) -> Outcome<ExitCode> {
    let opts = Options::parse(args)?;
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
    let node = opts.node.clone().unwrap_or_else(|| DEFAULT_NODE.into());
    // What this node declares about asking fills what the flags leave out.
    let declared = if opts.portal.is_none() || opts.label.is_none() {
        crate::approver::node_declaration(node.trim_end_matches('/'))
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
        .ok_or("ask needs --label: what to call this device (or declare it in [asking])")?;
    // With no portal named or declared, or when asked to, the device announces
    // on its private network instead of printing a link.
    let announcing = opts.announce || portal.is_none();
    if announcing && opts.loopback {
        return Err("--loopback hands the code back through a browser; an announced ask takes it back over the private network".into());
    }
    let portal = portal.unwrap_or_default();
    let approver = opts
        .approver
        .clone()
        .or_else(|| declared.as_ref().and_then(|a| a.approver.clone()))
        .unwrap_or_else(|| origin_of(&portal));

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
    let return_path = match &listener {
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
    let pending = Pending {
        request: request.clone(),
        code_verifier: verifier,
        approver,
        node: node.clone(),
        asked_at_unix: now_unix(),
    };
    pending.save()?;
    if announcing {
        return announce(pending, approver_key, &node);
    }

    let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&request).map_err(|e| e.to_string())?);
    let link = format!(
        "{}/consent/device?request={encoded}",
        portal.trim_end_matches('/')
    );
    println!("Open this link in the portal of a node that speaks for you, and approve this device:\n\n  {link}\n");
    println!(
        "This device is {} on network {}.",
        consent_grant::hash_shape::fingerprint(&request.device_key),
        consent_grant::hash_shape::fingerprint(&request.network_dna)
    );
    let Some(listener) = listener else {
        println!(
            "Then run:  epr device redeem '<code#state>'   (the code is good for five minutes)"
        );
        return Ok(ExitCode::SUCCESS);
    };
    println!("Waiting for the portal to hand the code back to this terminal…");
    let (code, state) = wait_for_callback(&listener)?;
    if state != request.state {
        pending.forget();
        return Err("the code that arrived was not for this request".into());
    }
    redeem(pending, &code, &node)
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
    let code = loop {
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
            ("code", Some(code)) => break code,
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
    finish(pending, delivered, node)
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
    finish(pending, delivered, node)
}

/// Check what was collected, have this node enroll with it, and report.
fn finish(pending: Pending, delivered: Delivered, node: &str) -> Outcome<ExitCode> {
    let request = &pending.request;
    if let Err(r) = check_delivered(&delivered, request) {
        pending.forget();
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
        pending.forget();
        println!("Enrolling this device was not agreed, so nothing was enrolled.");
        return Ok(ExitCode::SUCCESS);
    }
    let enroll = serde_json::to_vec(&serde_json::json!({
        "request": request,
        "delivered": delivered,
    }))
    .map_err(|e| e.to_string())?;
    let receipt: BindingReceipt = json_call(
        "POST",
        &format!("{}/auth/device/enroll", node.trim_end_matches('/')),
        Some(&enroll),
    )
    .map_err(|e| format!("this device's node did not enroll it: {e}"))?;
    pending.forget();
    println!(
        "This device is enrolled. Joining record: {}",
        receipt.binding_action
    );
    if record.agreed_acts.contains(&RequestedAct::BindDeviceRoot) {
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
    let rest = url.strip_prefix("http://").ok_or_else(|| {
        format!("{url}: only http:// node addresses are reachable from this terminal")
    })?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let mut stream = TcpStream::connect(authority).map_err(|e| format!("{authority}: {e}"))?;
    let _ = stream.set_read_timeout(Some(CALL_TIMEOUT));
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
