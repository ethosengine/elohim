//! `epr device` — this device asks its person's steward to recognize it.
//!
//! A device cannot recognize itself as someone's device. From a terminal on the
//! device, the person runs `ask`: it builds the request for this device's own
//! node, keeps the PKCE verifier and state privately on disk, and prints a link
//! to a steward's portal. The person approves there, and `redeem` takes the
//! one-time code back: it collects the signed consent from the steward, checks
//! it before using it, and has this device's own node enroll itself with it.
//!
//! The steward is named by the person, never assumed: the portal base is a
//! required argument, and no doorway is involved unless the person names one.
//!
//! The rules are `consent_grant`'s; this module only asks, keeps, checks and
//! reports. The device's key never leaves its node: the node signs possession
//! and notarizes the joining record itself, asked over node-local HTTP
//! (`/auth/device/self`, `/auth/device/enroll`).

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

/// The relying party this terminal is, as the steward's policy names it.
const CLIENT_ID: &str = "epr-cli";
/// Where a terminal on this machine reaches its own node unless told otherwise.
const DEFAULT_NODE: &str = "http://127.0.0.1:8090";
/// How long a code waits, matching the steward's five-minute window.
const WINDOW: Duration = Duration::from_secs(5 * 60);
/// How long one call to a node may take: a node's answer can wait on the network.
const CALL_TIMEOUT: Duration = Duration::from_secs(180);

type Outcome<T> = Result<T, String>;

pub fn usage() -> &'static str {
    "usage:\n  epr device ask --portal <steward portal URL> --label <name> \
     [--act device.enroll] [--act device.bind-root] [--loopback] \
     [--steward <steward node URL>] [--node <this node URL>]\n  \
     epr device redeem '<code#state>' [--node <this node URL>]\n  \
     epr device approve '<link or request>' [--only <act>]... [--yes] [--node <this node URL>]"
}

pub fn run(args: &[String]) -> Outcome<ExitCode> {
    match args.first().map(String::as_str) {
        Some("ask") => ask(&args[1..]),
        Some("approve") => crate::steward::approve(&args[1..]),
        Some("redeem") => {
            let (pasted, opts) = match args.get(1) {
                Some(p) if !p.starts_with("--") => (p.clone(), Options::parse(&args[2..])?),
                _ => return Err(format!("redeem needs the pasted code\n{}", usage())),
            };
            let (code, state) = parse_pasted(&pasted)
                .ok_or("that is not a code: it should look like code#state")?;
            let pending = Pending::load(state)?;
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
    steward: Option<String>,
    node: Option<String>,
    label: Option<String>,
    acts: Vec<String>,
    loopback: bool,
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
                "--steward" => o.steward = Some(value()?),
                "--node" => o.node = Some(value()?),
                "--label" => o.label = Some(value()?),
                "--act" => o.acts.push(value()?),
                "--loopback" => o.loopback = true,
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
    steward: String,
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
        crate::steward::node_declaration(node.trim_end_matches('/'))
            .declared
            .and_then(|d| d.asking)
    } else {
        None
    };
    let portal = opts
        .portal
        .clone()
        .or_else(|| declared.as_ref().map(|a| a.portal.clone()))
        .ok_or("ask needs --portal: the portal of the steward who will approve (or declare it in [asking])")?;
    let label = opts
        .label
        .clone()
        .or_else(|| declared.as_ref().map(|a| a.label.clone()))
        .ok_or("ask needs --label: what to call this device (or declare it in [asking])")?;
    let steward = opts
        .steward
        .clone()
        .or_else(|| declared.as_ref().and_then(|a| a.steward.clone()))
        .unwrap_or_else(|| portal.clone());

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
        steward,
        node: node.clone(),
        asked_at_unix: now_unix(),
    };
    pending.save()?;

    let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&request).map_err(|e| e.to_string())?);
    let link = format!(
        "{}/consent/device?request={encoded}",
        portal.trim_end_matches('/')
    );
    println!("Open this link in your steward's portal and approve this device:\n\n  {link}\n");
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

fn redeem(pending: Pending, code: &str, node: &str) -> Outcome<ExitCode> {
    let request = &pending.request;
    let redemption = consent_grant::Redemption {
        code: code.into(),
        code_verifier: pending.code_verifier.clone(),
        client_id: request.client_id.clone(),
        device_key: request.device_key.clone(),
    };
    let body = serde_json::to_vec(&redemption).map_err(|e| e.to_string())?;
    let url = format!(
        "{}/auth/consent/redeem",
        pending.steward.trim_end_matches('/')
    );
    let delivered: Delivered = match json_call("POST", &url, Some(&body)) {
        Ok(d) => d,
        Err(e) => {
            // Any refusal leaves the code spent at the steward; a new request is needed.
            pending.forget();
            return Err(format!("the steward did not hand over the consent: {e}"));
        }
    };
    if let Err(r) = check_delivered(&delivered, request) {
        pending.forget();
        return Err(format!(
            "{}: what the steward handed over is not a valid consent for this request; nothing was signed",
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

/// A minimal HTTP/1.1 client for `http://` nodes. A steward or device node is
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
    fn the_steward_is_never_assumed() {
        // Port 1 never answers, so no declaration can supply a portal either.
        let refused = ask(&[
            "--label".into(),
            "workspace".into(),
            "--node".into(),
            "http://127.0.0.1:1".into(),
        ])
        .unwrap_err();
        assert!(refused.contains("--portal"), "{refused}");
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
            http("GET", "https://steward.example/auth/consent/redeem", None)
                .unwrap_err()
                .contains("only http://")
        );
    }

    #[test]
    fn a_pasted_state_cannot_name_a_path() {
        assert!(Pending::load("../../etc/passwd").is_err());
    }
}
