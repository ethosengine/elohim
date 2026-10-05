//! The approving node's side, from a terminal on the node that speaks for a
//! person: `epr identity begin`, `epr identity standing`, `epr identity show`,
//! `epr device pending` and `epr device approve`.
//!
//! A node signs as its person only for a caller on its own machine. A headless
//! node (a workspace, a deployed conductor) has no browser there, so a
//! terminal on it is how an identity begins there and how a device is
//! approved. Every call goes to this machine's node over loopback; a node
//! elsewhere is refused before anything is sent.
//!
//! Three roles stay apart here. Whose node it is: the person it speaks for.
//! Who operates it: whoever runs the machine. Whether it may approve other
//! nodes: it is one of the controllers the person's authority names. Approving
//! a device is an act for the person's identity, so it is the person's. This
//! terminal cannot tell whether the one typing is that person or whoever
//! operates the machine: the loopback rule trusts the machine, not a person.
//! That gap is recorded in the device-recognition backlog cluster, not
//! papered over here.
//!
//! The rules are the node's and `consent_grant`'s. This module shows what is
//! asked, asks for the answer, and reports what the node answered.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use consent_grant::{
    AgreedView, ConsentView, Declaration, GrantRequest, PendingView, RequestedAct, ReturnPath,
    StandingView,
};

use crate::device::{http_with, json_call_with, refusal_text};
use crate::device_key;

const DEFAULT_NODE: &str = "http://127.0.0.1:8090";

type Outcome<T> = Result<T, String>;

pub fn usage() -> &'static str {
    "usage:\n  epr identity begin --name <display name> [--identifier <id>] [--node <this node URL>]\n  \
     epr identity standing [--node <this node URL>]\n  \
     epr identity show --declare [--node <this node URL>]\n  \
     epr device pending [--node <this node URL>]\n  \
     epr device approve '<link | number | key fingerprint>' [--only <act>]... [--yes] [--node <this node URL>]"
}

/// `epr identity …`.
pub fn run_identity(args: &[String]) -> Outcome<ExitCode> {
    match args.first().map(String::as_str) {
        Some("begin") => begin(&args[1..]),
        Some("standing") => standing(&args[1..]),
        Some("show") => show(&args[1..]),
        Some("--help" | "-h" | "help") | None => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        Some(other) => Err(format!("unknown identity command `{other}`\n{}", usage())),
    }
}

#[derive(Default)]
struct Options {
    node: Option<String>,
    name: Option<String>,
    identifier: Option<String>,
    only: Vec<String>,
    yes: bool,
    declare: bool,
    positional: Vec<String>,
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
                "--node" => o.node = Some(value()?),
                "--name" => o.name = Some(value()?),
                "--identifier" => o.identifier = Some(value()?),
                "--only" => o.only.push(value()?),
                "--yes" => o.yes = true,
                "--declare" => o.declare = true,
                other if other.starts_with("--") => {
                    return Err(format!("unknown argument `{other}`\n{}", usage()))
                }
                other => o.positional.push(other.to_string()),
            }
            i += 1;
        }
        Ok(o)
    }

    /// This machine's node. A node elsewhere is refused: it would not sign for
    /// this terminal anyway, and its answers would not be about this person.
    fn node(&self) -> Outcome<String> {
        let node = self.node.clone().unwrap_or_else(|| DEFAULT_NODE.into());
        if !names_this_machine(&node) {
            return Err(format!(
                "{node}: these steps talk only to this machine's own node (127.0.0.1, ::1 or localhost)"
            ));
        }
        Ok(node.trim_end_matches('/').to_string())
    }
}

/// Whether `url` is an `http://` address on this machine.
fn names_this_machine(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or("");
    let host = match authority.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => authority.split(':').next().unwrap_or(""),
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

/// Where the session the node gave this terminal is kept: beside the device
/// key, private to this OS user.
fn session_path() -> Outcome<PathBuf> {
    let key = device_key::resolve_path().map_err(|e| format!("no device key home: {e}"))?;
    Ok(key
        .parent()
        .ok_or("the device key has no directory")?
        .join("node-session"))
}

fn keep_session(id: &str) -> Outcome<()> {
    let path = session_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot keep the session: {e}"))?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .map_err(|e| format!("cannot keep the session: {e}"))?;
    file.write_all(id.as_bytes())
        .map_err(|e| format!("cannot keep the session: {e}"))
}

/// The `Cookie` header for the kept session, when there is one.
fn session_cookie() -> Option<String> {
    let id = fs::read_to_string(session_path().ok()?).ok()?;
    let id = id.trim();
    (!id.is_empty()).then(|| format!("elohim_session={id}"))
}

/// The session id a `Set-Cookie: elohim_session=…` header carries.
fn set_cookie_session(head: &str) -> Option<String> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("set-cookie") {
            return None;
        }
        let value = value.trim().strip_prefix("elohim_session=")?;
        Some(value.split(';').next()?.trim().to_string())
    })
}

fn cookie_headers(cookie: &Option<String>) -> Vec<(&str, &str)> {
    cookie
        .as_deref()
        .map(|c| vec![("Cookie", c)])
        .unwrap_or_default()
}

/// What a node says about its identity declaration.
#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct DeclarationAnswer {
    pub path: Option<String>,
    pub declared: Option<Declaration>,
    pub read_error: Option<String>,
    pub last_reconcile: Option<serde_json::Value>,
    pub current: Option<CurrentIdentity>,
}

#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentIdentity {
    pub human_id: String,
    pub display_name: String,
    pub profile_reach: String,
}

/// The node's declaration, as it reads it. A node that cannot say is treated
/// as declaring nothing: the person is then simply asked.
pub(crate) fn node_declaration(node: &str) -> DeclarationAnswer {
    json_call_with::<DeclarationAnswer>(
        "GET",
        &format!("{node}/auth/identity/declaration"),
        None,
        &[],
    )
    .map(|(answer, _)| answer)
    .unwrap_or_default()
}

/// The words for a declared approvals count the node cannot yet hold to.
pub(crate) fn declared_approvals_words(declared: usize, required: usize) -> Option<String> {
    (declared > required).then(|| {
        format!(
            "You declared that {declared} of the nodes that speak for you must approve a new \
             device. This node cannot hold to that yet: your identity's authority names how many \
             approve ({required}), and nothing yet writes a new authority that asks for more."
        )
    })
}

/// What an identity rests on, in plain words.
pub(crate) fn standing_words(s: &StandingView) -> Vec<String> {
    let mut lines = Vec::new();
    if s.rests_on_this_node_alone {
        lines.push("Your identity rests on this node alone.".to_string());
    } else {
        let nodes = if s.controller_count == 1 {
            "1 node speaks for you".to_string()
        } else {
            format!("{} nodes speak for you", s.controller_count)
        };
        let this = if s.this_node_is_controller {
            "this node is one of them"
        } else {
            "this node is not one of them"
        };
        lines.push(format!("{nodes}; {this}."));
    }
    lines.push(if s.required <= 1 {
        "Approving a new device needs one node that speaks for you.".to_string()
    } else {
        format!(
            "Approving a new device needs {} of the nodes that speak for you.",
            s.required
        )
    });
    lines.push(format!(
        "Identity {}, authority {}.",
        consent_grant::hash_shape::fingerprint(&s.identity_root),
        consent_grant::hash_shape::fingerprint(&s.authority)
    ));
    lines
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Begun {
    standing: StandingView,
    session: BegunSession,
    created: Created,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BegunSession {
    id: String,
    identifier: String,
}

#[derive(serde::Deserialize)]
struct Created {
    human: bool,
    authority: bool,
    session: bool,
}

fn begin(args: &[String]) -> Outcome<ExitCode> {
    let opts = Options::parse(args)?;
    let name = opts
        .name
        .clone()
        .ok_or("begin needs --name: what you are called")?;
    let node = opts.node()?;
    let mut body = serde_json::json!({ "displayName": name });
    if let Some(id) = &opts.identifier {
        body["identifier"] = serde_json::Value::String(id.clone());
    }
    let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
    let (begun, head): (Begun, String) = json_call_with(
        "POST",
        &format!("{node}/auth/identity/begin"),
        Some(&bytes),
        &[],
    )?;
    keep_session(&set_cookie_session(&head).unwrap_or(begun.session.id.clone()))?;
    let made: Vec<&str> = [
        (begun.created.human, "your person record"),
        (
            begun.created.authority,
            "your identity's authority, with this node as the first node that speaks for you",
        ),
        (begun.created.session, "a session on this node"),
    ]
    .into_iter()
    .filter_map(|(done, what)| done.then_some(what))
    .collect();
    if made.is_empty() {
        println!("Your identity had already begun here; nothing new was created.");
    } else {
        println!("Created {}.", made.join("; "));
    }
    println!("Signed in as {}.", begun.session.identifier);
    for line in standing_words(&begun.standing) {
        println!("{line}");
    }
    Ok(ExitCode::SUCCESS)
}

fn standing(args: &[String]) -> Outcome<ExitCode> {
    let opts = Options::parse(args)?;
    let node = opts.node()?;
    let cookie = session_cookie();
    let (status, _, bytes) = http_with(
        "GET",
        &format!("{node}/auth/identity/standing"),
        None,
        &cookie_headers(&cookie),
    )?;
    if status == 409 {
        return Err(format!(
            "{}\nYour identity has not begun on this node. Run: epr identity begin --name <your name>",
            refusal_text(status, &bytes)
        ));
    }
    if !(200..300).contains(&status) {
        return Err(refusal_text(status, &bytes));
    }
    let view: StandingView =
        serde_json::from_slice(&bytes).map_err(|e| format!("unreadable answer: {e}"))?;
    for line in standing_words(&view) {
        println!("{line}");
    }
    let declared = node_declaration(&node)
        .declared
        .and_then(|d| d.identity)
        .map(|i| i.approvals());
    if let Some(words) = declared.and_then(|n| declared_approvals_words(n, view.required)) {
        println!("{words}");
    }
    Ok(ExitCode::SUCCESS)
}

/// A TOML string, quoted.
fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into())
}

/// The declaration that matches the node as it is: the identity that exists,
/// and the devices and asking section already declared.
pub(crate) fn declaration_text(
    current: Option<&CurrentIdentity>,
    identifier: Option<&str>,
    declared: Option<&Declaration>,
) -> String {
    let mut out = String::from(
        "# This node's identity, declared. Point ELOHIM_IDENTITY_DECLARATION_PATH at this\n\
         # file and the node applies it at start and whenever it changes: it begins the\n\
         # identity when the node has none and never changes one that exists. No secret\n\
         # belongs here.\n",
    );
    let identity = declared.and_then(|d| d.identity.as_ref());
    if let Some(c) = current {
        out.push_str("\n[identity]\n");
        out.push_str(&format!("displayName = {}\n", quoted(&c.display_name)));
        out.push_str(&format!("humanId = {}\n", quoted(&c.human_id)));
        if let Some(i) = identifier.or_else(|| identity.and_then(|i| i.identifier.as_deref())) {
            out.push_str(&format!("identifier = {}\n", quoted(i)));
        }
        out.push_str(&format!("profileReach = {}\n", quoted(&c.profile_reach)));
        if let Some(n) = identity.and_then(|i| i.approvals_needed) {
            out.push_str(&format!("approvalsNeeded = {n}\n"));
        }
    } else {
        out.push_str("\n# No identity on this node yet. Declare the person it begins one for:\n");
        out.push_str("# [identity]\n# displayName = \"Your name\"\n");
    }
    let devices = declared.map(|d| d.devices.as_slice()).unwrap_or_default();
    for d in devices {
        out.push_str("\n[[devices]]\n");
        out.push_str(&format!("label = {}\n", quoted(&d.label)));
        out.push_str(&format!("deviceKey = {}\n", quoted(&d.device_key)));
        let acts: Vec<String> = d.acts.iter().map(|a| quoted(a.as_str())).collect();
        out.push_str(&format!("acts = [{}]\n", acts.join(", ")));
    }
    if devices.is_empty() {
        out.push_str(
            "\n# A device you expect, approved by `epr device approve` with no question\n\
             # when it asks from this key for no more than these acts:\n\
             # [[devices]]\n# label = \"home\"\n# deviceKey = \"uhCAk…\"\n# acts = [\"device.enroll\"]\n",
        );
    }
    if let Some(a) = declared.and_then(|d| d.asking.as_ref()) {
        out.push_str("\n[asking]\n");
        out.push_str(&format!("label = {}\n", quoted(&a.label)));
        if let Some(p) = &a.portal {
            out.push_str(&format!("portal = {}\n", quoted(p)));
        }
        if let Some(s) = &a.approver {
            out.push_str(&format!("approver = {}\n", quoted(s)));
        }
        if let Some(k) = &a.approver_key {
            out.push_str(&format!("approverKey = {}\n", quoted(k)));
        }
    }
    out
}

fn show(args: &[String]) -> Outcome<ExitCode> {
    let opts = Options::parse(args)?;
    if !opts.declare {
        return Err(format!("show needs --declare\n{}", usage()));
    }
    let node = opts.node()?;
    let answer = node_declaration(&node);
    if let Some(e) = &answer.read_error {
        eprintln!("note: the declared file does not read: {e}");
    }
    print!(
        "{}",
        declaration_text(answer.current.as_ref(), None, answer.declared.as_ref())
    );
    Ok(ExitCode::SUCCESS)
}

/// Read a request the way the portal does: a link carrying `request=`, the
/// base64url of the request on its own, or the request JSON itself.
pub(crate) fn decode_request(text: &str) -> Outcome<GrantRequest> {
    let text = text.trim();
    let unreadable = |why: &str| format!("request_unreadable: {why}");
    if text.starts_with('{') {
        return serde_json::from_str(text).map_err(|e| unreadable(&e.to_string()));
    }
    let encoded = match text.find("request=") {
        Some(i) => text[i + "request=".len()..]
            .split(['&', '#'])
            .next()
            .unwrap_or(""),
        None => text,
    };
    let unpadded = encoded.trim_end_matches('=');
    let bytes = URL_SAFE_NO_PAD
        .decode(unpadded)
        .map_err(|_| unreadable("the link's request is not base64url"))?;
    serde_json::from_slice(&bytes).map_err(|e| unreadable(&e.to_string()))
}

/// What an act asks, in the person's words.
pub(crate) fn act_words(act: RequestedAct) -> &'static str {
    match act {
        RequestedAct::EnrollDevice => "recognize this device as one of yours",
        RequestedAct::BindDeviceRoot => {
            "bind this device's root key, so what it makes traces to you"
        }
    }
}

/// How the person's answer is gathered.
pub(crate) enum Answer<'a> {
    /// Agree to these, named on the command line.
    Only(&'a [String]),
    /// Agree to everything asked (`--yes`).
    All,
    /// Ask at the terminal.
    Ask,
}

/// Which of `asked` the person agrees to. `prompt` asks one act and returns
/// whether they said yes. Refuses to guess when nobody can be asked.
pub(crate) fn choose(
    asked: &[RequestedAct],
    answer: Answer<'_>,
    interactive: bool,
    mut prompt: impl FnMut(RequestedAct) -> Outcome<bool>,
) -> Outcome<Vec<RequestedAct>> {
    match answer {
        Answer::Only(named) => named
            .iter()
            .map(|t| {
                let act = RequestedAct::parse(t).map_err(|r| format!("{t}: {}", r.code()))?;
                if asked.contains(&act) {
                    Ok(act)
                } else {
                    Err(format!(
                        "approve_act_not_asked: the device did not ask for {t}"
                    ))
                }
            })
            .collect(),
        Answer::All => Ok(asked.to_vec()),
        Answer::Ask if !interactive => Err(
            "approve_needs_answer: no terminal to ask you on; name the acts with --only, or agree to all with --yes"
                .into(),
        ),
        Answer::Ask => {
            let mut agreed = Vec::new();
            for act in asked {
                if prompt(*act)? {
                    agreed.push(*act);
                }
            }
            Ok(agreed)
        }
    }
}

fn ask_at_terminal(act: RequestedAct) -> Outcome<bool> {
    print!("  Agree to {} ({act})? [y/N] ", act_words(act));
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Ok(matches!(line.trim(), "y" | "Y" | "yes" | "Yes"))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingAnswer {
    carrier: String,
    approver: Option<String>,
    /// Whom the node speaks for; absent from a node that predates it.
    #[serde(default)]
    speaks_for: Option<consent_grant::Speaks>,
    asks: Vec<PendingView>,
}

/// What a waiting ask looks like to the person, a few lines each.
pub(crate) fn pending_lines(view: &PendingView) -> Vec<String> {
    let mut lines = vec![format!(
        "{:>3}  {}  key {}  {}:{:02} left",
        view.number,
        view.label,
        view.device_fingerprint,
        view.seconds_left / 60,
        view.seconds_left % 60
    )];
    if let Some(root) = &view.device_root_fingerprint {
        lines.push(format!("     root key {root}"));
    }
    for act in &view.asked_acts {
        lines.push(format!("     asks to {} ({act})", act_words(*act)));
    }
    lines.push(format!("     the asking node {}", view.state_words));
    if let Some(identity) = &view.for_identity {
        lines.push(format!(
            "     {}",
            consent_grant::Speaks::Person(identity.clone()).words()
        ));
    }
    lines
}

/// `epr device pending`: what is asking this node over its private network.
pub fn pending(args: &[String]) -> Outcome<ExitCode> {
    let opts = Options::parse(args)?;
    let node = opts.node()?;
    let (answer, _): (PendingAnswer, String) =
        json_call_with("GET", &format!("{node}/auth/consent/pending"), None, &[])?;
    if answer.carrier == "absent" {
        println!(
            "This node has no local discovery in its transport mode, so nothing can ask it over \
             the private network."
        );
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(me) = &answer.approver {
        println!(
            "This node is {}.",
            consent_grant::hash_shape::fingerprint(me)
        );
    }
    match &answer.speaks_for {
        Some(speaks @ consent_grant::Speaks::Person(_)) => println!("{}", speaks.words()),
        Some(speaks) => {
            println!("{}", speaks.words());
            return Ok(ExitCode::SUCCESS);
        }
        None => {}
    }
    if answer.asks.is_empty() {
        println!("Nothing is asking.");
    }
    for view in &answer.asks {
        for line in pending_lines(view) {
            println!("{line}");
        }
    }
    if !answer.asks.is_empty() {
        println!("Approve one with: epr device approve <number>");
    }
    Ok(ExitCode::SUCCESS)
}

/// Whether `text` names a waiting ask (its number, a device key, or a key
/// fingerprint) rather than a link or request.
pub(crate) fn names_pending(text: &str) -> bool {
    let t = text.trim();
    (!t.is_empty() && t.len() <= 9 && t.bytes().all(|b| b.is_ascii_digit()))
        || (t.starts_with("uhCAk") && !t.contains("request="))
}

/// Decide a waiting ask on this node, answering when the node needs it.
fn approve_pending(opts: &Options, node: &str, which: &str) -> Outcome<ExitCode> {
    let cookie = session_cookie();
    let headers = cookie_headers(&cookie);
    let url = format!("{node}/auth/consent/pending/decide");
    let ask_body = |answer: Option<&[RequestedAct]>| {
        let mut body = serde_json::json!({ "ask": which });
        if let Some(acts) = answer {
            body["answer"] = serde_json::json!({ "agreedActs": acts });
        }
        serde_json::to_vec(&body).unwrap_or_default()
    };
    // An answer given on the command line is the answer. Otherwise the node's
    // deciders go first, and the person is asked only when they defer.
    let decided = if !opts.only.is_empty() || opts.yes {
        let (list, _): (PendingAnswer, String) =
            json_call_with("GET", &format!("{node}/auth/consent/pending"), None, &[])?;
        let view = list
            .asks
            .into_iter()
            .find(|v| {
                v.number.to_string() == which
                    || v.device_key == which
                    || v.device_fingerprint == which
            })
            .ok_or("pending_unknown: nothing is asking under that number or key")?;
        println!("A device asks to act for you.");
        for line in pending_lines(&view) {
            println!("{line}");
        }
        let answer = if !opts.only.is_empty() {
            Answer::Only(&opts.only)
        } else {
            Answer::All
        };
        let agreed = choose(&view.asked_acts, answer, false, ask_at_terminal)?;
        answer_pending(&url, &headers, &ask_body(Some(&agreed)))?
    } else {
        let (status, _, bytes) = http_with("POST", &url, Some(&ask_body(None)), &headers)?;
        if (200..300).contains(&status) {
            bytes
        } else if status == 409 {
            let needs: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if needs["code"] != "pending_needs_answer" {
                return Err(refusal_text(409, &bytes));
            }
            let view: PendingView = serde_json::from_value(needs["ask"].clone())
                .map_err(|e| format!("unreadable answer: {e}"))?;
            println!("A device asks to act for you.");
            for line in pending_lines(&view) {
                println!("{line}");
            }
            if let Some(reason) = needs["reasons"].as_array().and_then(|r| r.last()) {
                println!("  {}", reason.as_str().unwrap_or_default());
            }
            let agreed = choose(
                &view.asked_acts,
                Answer::Ask,
                std::io::stdin().is_terminal(),
                ask_at_terminal,
            )?;
            answer_pending(&url, &headers, &ask_body(Some(&agreed)))?
        } else {
            return Err(refusal_text(status, &bytes));
        }
    };
    print_decided(&decided)
}

fn answer_pending(url: &str, headers: &[(&str, &str)], body: &[u8]) -> Outcome<Vec<u8>> {
    let (status, _, bytes) = http_with("POST", url, Some(body), headers)?;
    if (200..300).contains(&status) {
        Ok(bytes)
    } else {
        Err(refusal_text(status, &bytes))
    }
}

fn print_decided(bytes: &[u8]) -> Outcome<ExitCode> {
    let decided: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("unreadable answer: {e}"))?;
    let by = decided["decidedBy"].as_str().unwrap_or("?");
    if decided["declined"] == true {
        println!("Declined. No code was made and the device is not approved.");
        return Ok(ExitCode::SUCCESS);
    }
    match by {
        "declaration" => {
            println!("  Agreed by declaration: you declared this device for what it asks.")
        }
        "answer" => println!("  Agreed by your answer."),
        other => println!("  Agreed (decided by {other})."),
    }
    let view: AgreedView = serde_json::from_value(decided["agreed"].clone())
        .map_err(|e| format!("unreadable answer: {e}"))?;
    let lines = agreed_lines(&view, now_millis());
    for line in lines
        .iter()
        .filter(|l| l.starts_with("Signed by") || l.starts_with("  signed:"))
    {
        println!("{line}");
    }
    if decided["handedBack"]["taken"] == true {
        println!(
            "The code went back to the device over the private network; it enrolls itself now."
        );
    } else {
        println!(
            "The code did not reach the device over the private network ({}). Give it the code instead:",
            decided["handedBack"]
        );
        for line in lines.iter().take_while(|l| !l.starts_with("Signed by")) {
            println!("{line}");
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `epr device approve '<link or request>'`, or a waiting ask by number or key.
pub fn approve(args: &[String]) -> Outcome<ExitCode> {
    let opts = Options::parse(args)?;
    let node = opts.node()?;
    let [text] = opts.positional.as_slice() else {
        return Err(format!(
            "approve needs the link the device printed, or a waiting ask's number\n{}",
            usage()
        ));
    };
    if names_pending(text) {
        return approve_pending(&opts, &node, text.trim());
    }
    let request = decode_request(text)?;
    // The code can only reach a terminal waiting on loopback through a browser
    // on that terminal's own machine; from here it could not be handed back.
    if let ReturnPath::Loopback { port } = request.return_path {
        return Err(format!(
            "approve_return_needs_browser: the device is waiting for its code on its own \
             machine (port {port}); approve it from a browser there, or ask again from the \
             device without --loopback to paste the code"
        ));
    }
    let body = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    let (view, _): (ConsentView, String) = json_call_with(
        "POST",
        &format!("{node}/auth/consent/view"),
        Some(&body),
        &[],
    )?;
    println!("A device asks to act for you.");
    println!("  Name:     {}", view.label);
    println!("  Key:      {}", view.device_fingerprint);
    if let Some(root) = &view.device_root_fingerprint {
        println!("  Root key: {root}");
    }
    println!("  Asking, each agreed to or declined separately:");
    for act in &view.asked_acts {
        println!("    - {} ({act})", act_words(*act));
    }
    let declaration = node_declaration(&node).declared.unwrap_or_default();
    let agreed = if let (true, false, Some((device, acts))) = (
        opts.only.is_empty(),
        opts.yes,
        declaration.agreed_for(&request),
    ) {
        println!(
            "  Agreed by declaration: you declared device {} for {}.",
            quoted(&device.label),
            device
                .acts
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        acts
    } else {
        let answer = if !opts.only.is_empty() {
            Answer::Only(&opts.only)
        } else if opts.yes {
            Answer::All
        } else {
            Answer::Ask
        };
        choose(
            &view.asked_acts,
            answer,
            std::io::stdin().is_terminal(),
            ask_at_terminal,
        )?
    };
    if agreed.is_empty() {
        println!("You agreed to nothing, so no code was made and the device is not approved.");
        return Ok(ExitCode::SUCCESS);
    }
    let body = serde_json::to_vec(&serde_json::json!({
        "request": request,
        "agreedActs": agreed,
    }))
    .map_err(|e| e.to_string())?;
    let cookie = session_cookie();
    let (agreed_view, _): (AgreedView, String) = json_call_with(
        "POST",
        &format!("{node}/auth/consent/agree"),
        Some(&body),
        &cookie_headers(&cookie),
    )?;
    for line in agreed_lines(&agreed_view, now_millis()) {
        println!("{line}");
    }
    Ok(ExitCode::SUCCESS)
}

fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// What the person is told once the node has agreed.
pub(crate) fn agreed_lines(view: &AgreedView, now_millis: i64) -> Vec<String> {
    let mut lines = Vec::new();
    match &view.return_target {
        consent_grant::ReturnTargetView::Display { value } => {
            lines.push("Give this code to the device's terminal:".to_string());
            lines.push(String::new());
            lines.push(format!("  {value}"));
            lines.push(String::new());
        }
        consent_grant::ReturnTargetView::Redirect { url } => {
            lines.push(format!("Open this on the device's machine: {url}"));
        }
    }
    let minutes = ((view.expires_at - now_millis).max(0) + 59_999) / 60_000;
    lines.push(format!(
        "It is good for {minutes} minute{}.",
        if minutes == 1 { "" } else { "s" }
    ));
    let c = view.controllers;
    lines.push(format!(
        "Signed by {} node{} that speak{} for you; your identity asks for {}.",
        c.signed,
        if c.signed == 1 { "" } else { "s" },
        if c.signed == 1 { "s" } else { "" },
        c.required
    ));
    for w in &view.witnesses {
        let who = match w.relation {
            consent_grant::Relation::ThisDevice => "this node",
            consent_grant::Relation::YourDevice => "another of your nodes",
            consent_grant::Relation::VouchesForYou => "someone who vouches for you",
        };
        lines.push(format!(
            "  signed: {who} ({})",
            consent_grant::hash_shape::fingerprint(&w.id)
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use consent_grant::{pkce, GRANT_DOMAIN};

    fn request() -> GrantRequest {
        GrantRequest {
            domain: GRANT_DOMAIN.into(),
            client_id: "epr-cli".into(),
            device_key: "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl".into(),
            device_root_key: None,
            label: "home".into(),
            network_dna: "uhC0kQwOEwmIBZhBT3I7vGZPz0kEL3_hqavyFW0upoO_hyEwuEglj".into(),
            content_dna: "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt".into(),
            acts: vec![RequestedAct::EnrollDevice, RequestedAct::BindDeviceRoot],
            code_challenge: pkce::challenge(&"v".repeat(43)),
            state: "s".repeat(32),
            return_path: ReturnPath::Paste,
        }
    }

    #[test]
    fn a_request_is_read_from_a_link_its_encoding_or_its_json() {
        let json = serde_json::to_string(&request()).unwrap();
        let encoded = URL_SAFE_NO_PAD.encode(&json);
        for text in [
            format!("http://127.0.0.1:8191/consent/device?request={encoded}"),
            format!("https://x/threshold/consent/device?request={encoded}&other=1"),
            format!("  {encoded}==\n"),
            json.clone(),
        ] {
            assert_eq!(decode_request(&text).unwrap(), request(), "{text}");
        }
        assert!(decode_request("request=!!!")
            .unwrap_err()
            .starts_with("request_unreadable"));
    }

    #[test]
    fn a_waiting_ask_is_named_by_number_or_key_and_a_link_is_not() {
        for yes in [
            "1",
            "12",
            "uhCAkiqcz…TGMzzl",
            "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl",
        ] {
            assert!(names_pending(yes), "{yes}");
        }
        for no in [
            "http://127.0.0.1:8191/consent/device?request=eyJ",
            "{\"domain\":1}",
            "eyJkb21h",
            "",
        ] {
            assert!(!names_pending(no), "{no}");
        }
    }

    #[test]
    fn a_waiting_ask_is_shown_with_its_key_acts_time_and_state() {
        let view = PendingView {
            number: 2,
            label: "home".into(),
            device_key: "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl".into(),
            device_fingerprint: "uhCAkiqcz…TGMzzl".into(),
            device_root_fingerprint: None,
            asked_acts: vec![RequestedAct::EnrollDevice],
            seconds_left: 241,
            state: consent_grant::NodeState::Unassigned,
            state_words: "has no identity of its own".into(),
            addressed_here: false,
            for_identity: None,
        };
        let lines = pending_lines(&view);
        assert_eq!(lines[0], "  2  home  key uhCAkiqcz…TGMzzl  4:01 left");
        assert!(lines[1].contains("recognize this device as one of yours"));
        assert_eq!(lines[2], "     the asking node has no identity of its own");
        assert_eq!(lines.len(), 3);
        let mut joined = view.clone();
        joined.for_identity = Some(consent_grant::SpeaksFor {
            human_id: Some("matthew".into()),
            display_name: Some("Matthew".into()),
            identity_root: "uhCkkzY_ZvJaVbaFzi46J_LbGgFEUEQVMlWN5rpSdxGYdOG_3sQYp".into(),
            identity_fingerprint: "uhCkkzY_Z…_3sQYp".into(),
        });
        assert_eq!(
            pending_lines(&joined)[3],
            "     An approval here is for Matthew (matthew), identity uhCkkzY_Z…_3sQYp."
        );
    }

    #[test]
    fn only_this_machines_node_is_spoken_to() {
        for ok in [
            "http://127.0.0.1:8090",
            "http://localhost:8191/",
            "http://[::1]:8090",
        ] {
            assert!(names_this_machine(ok), "{ok}");
        }
        for no in [
            "http://10.1.19.170:8090",
            "https://127.0.0.1:8090",
            "http://localhost.example:8090",
            "127.0.0.1:8090",
        ] {
            assert!(!names_this_machine(no), "{no}");
        }
    }

    #[test]
    fn the_answer_is_the_persons_and_is_never_guessed() {
        use RequestedAct::{BindDeviceRoot, EnrollDevice};
        let asked = [EnrollDevice, BindDeviceRoot];
        let never = |_| -> Outcome<bool> { panic!("nobody may be asked here") };
        assert_eq!(choose(&asked, Answer::All, false, never).unwrap(), asked);
        let only = ["device.enroll".to_string()];
        assert_eq!(
            choose(&asked, Answer::Only(&only), false, never).unwrap(),
            [EnrollDevice]
        );
        let unasked = ["device.bind-root".to_string()];
        assert!(choose(&[EnrollDevice], Answer::Only(&unasked), true, never)
            .unwrap_err()
            .starts_with("approve_act_not_asked"));
        assert!(choose(&asked, Answer::Ask, false, never)
            .unwrap_err()
            .starts_with("approve_needs_answer"));
        // At a terminal, each act is its own question.
        let mut said = vec![false, true].into_iter();
        assert_eq!(
            choose(&asked, Answer::Ask, true, |_| Ok(said.next().unwrap())).unwrap(),
            [BindDeviceRoot]
        );
        let none = choose(&asked, Answer::Ask, true, |_| Ok(false)).unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn a_waiting_loopback_terminal_is_refused_before_anything_is_agreed() {
        let mut local = request();
        local.return_path = ReturnPath::Loopback { port: 49152 };
        let link = format!(
            "http://127.0.0.1:1/consent/device?request={}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&local).unwrap())
        );
        // Port 1 is never listening: refusing first means no call is attempted.
        let refused = approve(&[
            link,
            "--yes".into(),
            "--node".into(),
            "http://127.0.0.1:1".into(),
        ])
        .unwrap_err();
        assert!(
            refused.starts_with("approve_return_needs_browser"),
            "{refused}"
        );
    }

    #[test]
    fn a_node_elsewhere_is_refused_before_anything_is_sent() {
        let refused = approve(&[
            "x".into(),
            "--node".into(),
            "http://10.1.19.170:8090".into(),
        ])
        .unwrap_err();
        assert!(refused.contains("this machine's own node"), "{refused}");
    }

    #[test]
    fn the_set_cookie_session_is_kept_and_nothing_else() {
        let head = "HTTP/1.1 201 Created\r\ncontent-type: application/json\r\n\
                    set-cookie: elohim_session=abc-123; HttpOnly; SameSite=Lax; Path=/";
        assert_eq!(set_cookie_session(head).as_deref(), Some("abc-123"));
        assert_eq!(set_cookie_session("HTTP/1.1 200 OK\r\nx: y"), None);
    }

    fn view(alone: bool, count: usize, required: usize) -> StandingView {
        StandingView {
            identity_root: "uhCkkRrENFlI2RlXCelrj6C6ttNq8qTI_wSh0fFmsrSvBgdkkM39j".into(),
            authority: "uhCkkN_k9u6glm7eRydJ_WWbyUSSWbBYMHd_6aWO4ag-PrcWHA5_-".into(),
            network_dna: String::new(),
            controllers: vec![String::new(); count],
            controller_count: count,
            required,
            this_node_is_controller: true,
            rests_on_this_node_alone: alone,
        }
    }

    #[test]
    fn a_declared_count_the_node_cannot_hold_to_is_said_plainly() {
        assert_eq!(declared_approvals_words(1, 1), None);
        let words = declared_approvals_words(2, 1).unwrap();
        assert!(words.contains("cannot hold to that yet"), "{words}");
    }

    #[test]
    fn the_declaration_shown_matches_the_node_as_it_is() {
        let current = CurrentIdentity {
            human_id: "h-1".into(),
            display_name: "Matthew".into(),
            profile_reach: "private".into(),
        };
        let text = declaration_text(Some(&current), Some("matthew"), None);
        assert!(text.contains("[identity]\ndisplayName = \"Matthew\"\nhumanId = \"h-1\"\nidentifier = \"matthew\"\nprofileReach = \"private\"\n"), "{text}");
        assert!(text.contains("# [[devices]]"));
        let none = declaration_text(None, None, None);
        assert!(none.contains("No identity on this node yet"));
    }

    #[test]
    fn standing_is_told_in_plain_words() {
        let alone = standing_words(&view(true, 1, 1));
        assert_eq!(alone[0], "Your identity rests on this node alone.");
        assert_eq!(
            alone[1],
            "Approving a new device needs one node that speaks for you."
        );
        let shared = standing_words(&view(false, 3, 2));
        assert_eq!(
            shared[0],
            "3 nodes speak for you; this node is one of them."
        );
        assert_eq!(
            shared[1],
            "Approving a new device needs 2 of the nodes that speak for you."
        );
    }

    #[test]
    fn the_person_is_told_the_code_its_window_and_who_signed() {
        let view: AgreedView = serde_json::from_value(serde_json::json!({
            "returnTarget": {"kind": "display", "value": "c0de#st8"},
            "expiresAt": 300_000,
            "consentCid": "bafy",
            "controllers": {"required": 1, "signed": 1},
            "witnesses": [{"id": "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl",
                           "act": "signed", "relation": "this-device", "state": "done"}]
        }))
        .unwrap();
        let lines = agreed_lines(&view, 0);
        assert!(lines.contains(&"  c0de#st8".to_string()));
        assert!(lines.contains(&"It is good for 5 minutes.".to_string()));
        assert!(lines.contains(
            &"Signed by 1 node that speaks for you; your identity asks for 1.".to_string()
        ));
        assert!(lines
            .iter()
            .any(|l| l.starts_with("  signed: this node (uhCAkiqcz")));
    }
}
