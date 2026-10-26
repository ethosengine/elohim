//! Unit tests for the mux state machine: one per correctness item of the slice-1 checklist.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::*;
use crate::attrib::{EditEvent, EditSource, SessionKey};
use crate::deliver::AreaFs;

/// Packages `core`, `client` and `other` under `/r`, and a disk the test writes to.
#[derive(Clone, Default)]
struct Fs {
    disk: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl Fs {
    fn write(&self, uri: &str, text: &str) {
        let path = crate::uri::uri_to_path(uri).unwrap();
        self.disk.lock().unwrap().insert(path, text.to_owned());
    }
}

impl AreaFs for Fs {
    fn manifest(&self, dir: &Path) -> Option<String> {
        ["/r/core", "/r/client", "/r/other"]
            .contains(&dir.to_str()?)
            .then(|| "[package]\nname = \"x\"\n".to_owned())
    }

    fn document(&self, path: &Path) -> Option<String> {
        self.disk.lock().unwrap().get(path).cloned()
    }
}

const A: ClientId = ClientId(1);
const B: ClientId = ClientId(2);
const SESSION_A: &str = "aaaaaaaa-0000-0000-0000-000000000001";
const SESSION_B: &str = "bbbbbbbb-0000-0000-0000-000000000002";

fn new_mux(fs: &Fs) -> Mux<Fs> {
    Mux::new(
        MuxConfig {
            root: "/r".into(),
            initialization_options: json!({"check": {"workspace": false}, "cargo": {"targetDir": false}}),
            config_section: Some("rust-analyzer".into()),
            mux_pid: 4242,
            window_ms: 30_000,
            lang: "rust".into(),
        },
        fs.clone(),
    )
}

fn to_server(out: &[Output]) -> Vec<Value> {
    out.iter()
        .filter_map(|o| match o {
            Output::ToServer(v) => Some(v.clone()),
            _ => None,
        })
        .collect()
}

fn to_client(out: &[Output], client: ClientId) -> Vec<Value> {
    out.iter()
        .filter_map(|o| match o {
            Output::ToClient(c, v) if *c == client => Some(v.clone()),
            _ => None,
        })
        .collect()
}

fn publishes(out: &[Output], client: ClientId) -> Vec<(String, Vec<Value>)> {
    to_client(out, client)
        .into_iter()
        .filter(|m| m["method"] == "textDocument/publishDiagnostics")
        .map(|m| {
            (
                m["params"]["uri"].as_str().unwrap().to_owned(),
                m["params"]["diagnostics"].as_array().unwrap().clone(),
            )
        })
        .collect()
}

fn attach(mux: &mut Mux<Fs>, client: ClientId, session: &str) {
    mux.handle(
        Input::Attach {
            client,
            session: SessionKey::named(session),
        },
        0,
    );
}

fn from(mux: &mut Mux<Fs>, client: ClientId, message: Value, now: i64) -> Vec<Output> {
    mux.handle(Input::FromClient { client, message }, now)
}

fn server(mux: &mut Mux<Fs>, message: Value, now: i64) -> Vec<Output> {
    mux.handle(Input::FromServer(message), now)
}

fn initialize(id: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "initialize", "params": {
        "processId": 999,
        "rootUri": "file:///elsewhere",
        "initializationOptions": {"client": "options"},
        "capabilities": {"general": {"positionEncodings": ["utf-8"]}, "textDocument": {"diagnostic": {}}}
    }})
}

/// A and B attached and initialized over `fs`.
fn ready_pair(fs: &Fs) -> Mux<Fs> {
    let mut mux = new_mux(fs);
    attach(&mut mux, A, SESSION_A);
    attach(&mut mux, B, SESSION_B);
    let out = from(&mut mux, A, initialize(json!(1)), 0);
    let init = &to_server(&out)[0];
    server(
        &mut mux,
        json!({"jsonrpc": "2.0", "id": init["id"], "result": {"capabilities": {"textDocumentSync": 2}}}),
        0,
    );
    from(
        &mut mux,
        A,
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        0,
    );
    from(&mut mux, B, initialize(json!("b-init")), 0);
    from(
        &mut mux,
        B,
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        0,
    );
    mux
}

fn open(uri: &str, text: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": {"uri": uri, "languageId": "rust", "version": 1, "text": text}
    }})
}

fn change(uri: &str, buffer: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": 2},
        "contentChanges": [{"text": buffer}]
    }})
}

/// An agent edit: the file changes on disk, then the editing client reports it.
fn edit(
    mux: &mut Mux<Fs>,
    fs: &Fs,
    client: ClientId,
    uri: &str,
    text: &str,
    now: i64,
) -> Vec<Output> {
    fs.write(uri, text);
    from(mux, client, change(uri, text), now)
}

fn save(uri: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didSave", "params": {"textDocument": {"uri": uri}, "text": "client buffer"}})
}

fn diagnostic(line: u64, severity: u64, message: &str) -> Value {
    json!({
        "range": {"start": {"line": line, "character": 0}, "end": {"line": line, "character": 4}},
        "severity": severity,
        "message": message
    })
}

fn publish(uri: &str, diagnostics: Vec<Value>) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {
        "uri": uri, "version": 7, "diagnostics": diagnostics
    }})
}

// (a)
#[test]
fn only_the_first_initialize_reaches_the_server_with_daemon_options() {
    let fs = Fs::default();
    let mut mux = new_mux(&fs);
    attach(&mut mux, A, SESSION_A);
    attach(&mut mux, B, SESSION_B);
    let out = from(&mut mux, A, initialize(json!(1)), 0);
    let sent = to_server(&out);
    assert_eq!(sent.len(), 1);
    let params = &sent[0]["params"];
    assert_eq!(
        params["processId"], 4242,
        "the server watches the daemon, not client A"
    );
    assert_eq!(params["rootUri"], "file:///r");
    assert_eq!(params["initializationOptions"]["check"]["workspace"], false);
    assert!(params["capabilities"]["general"]
        .get("positionEncodings")
        .is_none());
    assert!(params["capabilities"]["textDocument"]
        .get("diagnostic")
        .is_none());

    // B asks while A's initialize is in flight: it waits rather than reaching the server.
    let out = from(&mut mux, B, initialize(json!("b-1")), 0);
    assert!(to_server(&out).is_empty());

    let out = server(
        &mut mux,
        json!({"jsonrpc": "2.0", "id": sent[0]["id"], "result": {"capabilities": {"textDocumentSync": {"openClose": true, "change": 2}}}}),
        0,
    );
    let a = to_client(&out, A);
    let b = to_client(&out, B);
    assert_eq!(a[0]["id"], 1);
    assert_eq!(
        b[0]["id"], "b-1",
        "each client gets the result under its own id"
    );
    assert_eq!(
        a[0]["result"]["capabilities"]["textDocumentSync"]["change"],
        1
    );

    let initialized = json!({"jsonrpc": "2.0", "method": "initialized", "params": {}});
    let out = from(&mut mux, A, initialized.clone(), 0);
    assert_eq!(to_server(&out), vec![initialized.clone()]);
    let out = from(&mut mux, B, initialized, 0);
    assert!(to_server(&out).is_empty(), "initialized is forwarded once");

    // A third client, after the fact, gets the cached result with no server round trip.
    attach(&mut mux, ClientId(3), "c");
    let out = from(&mut mux, ClientId(3), initialize(json!(5)), 0);
    assert!(to_server(&out).is_empty());
    assert_eq!(to_client(&out, ClientId(3))[0]["id"], 5);
}

// (b)
#[test]
fn a_client_shutdown_is_answered_locally_and_exit_detaches_only_it() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let out = from(
        &mut mux,
        A,
        json!({"jsonrpc": "2.0", "id": 9, "method": "shutdown"}),
        0,
    );
    assert!(to_server(&out).is_empty());
    assert_eq!(
        to_client(&out, A),
        vec![json!({"jsonrpc": "2.0", "id": 9, "result": null})]
    );
    let out = from(&mut mux, A, json!({"jsonrpc": "2.0", "method": "exit"}), 0);
    assert!(out.contains(&Output::Close(A)));
    assert!(
        to_server(&out)
            .iter()
            .all(|m| m["method"] != "exit" && m["method"] != "shutdown"),
        "the server never hears a client's shutdown or exit"
    );
    assert_eq!(mux.client_count(), 1);
    let (_, reap) = mux.shutdown_request();
    assert_eq!(
        reap["method"], "shutdown",
        "only the mux shuts the server down"
    );
}

// (c)
#[test]
fn request_ids_are_remapped_per_client_and_cancel_follows() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let hover = |id: Value| json!({"jsonrpc": "2.0", "id": id, "method": "textDocument/hover", "params": {}});
    let a_out = from(&mut mux, A, hover(json!(1)), 0);
    let b_out = from(&mut mux, B, hover(json!("1")), 0);
    let a_sid = to_server(&a_out)[0]["id"].clone();
    let b_sid = to_server(&b_out)[0]["id"].clone();
    assert_ne!(
        a_sid, b_sid,
        "two clients' id 1 never collide on the server"
    );

    let out = from(
        &mut mux,
        B,
        json!({"jsonrpc": "2.0", "method": "$/cancelRequest", "params": {"id": "1"}}),
        0,
    );
    assert_eq!(to_server(&out)[0]["params"]["id"], b_sid);

    let out = server(
        &mut mux,
        json!({"jsonrpc": "2.0", "id": b_sid, "result": "b"}),
        0,
    );
    assert_eq!(
        to_client(&out, B),
        vec![json!({"jsonrpc": "2.0", "id": "1", "result": "b"})]
    );
    assert!(to_client(&out, A).is_empty());
    let out = server(
        &mut mux,
        json!({"jsonrpc": "2.0", "id": a_sid, "result": "a"}),
        0,
    );
    assert_eq!(to_client(&out, A)[0]["id"], 1);
}

// (d)
#[test]
fn server_requests_are_answered_from_daemon_state_or_refused_never_routed() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let ask = |id: u64, method: &str, params: Value| json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    let silent = |out: &[Output]| to_client(out, A).is_empty() && to_client(out, B).is_empty();

    // Configuration is the daemon's own answer, never a client's.
    let out = server(
        &mut mux,
        ask(
            1,
            "workspace/configuration",
            json!({"items": [{"section": "rust-analyzer"}, {"section": "rust-analyzer.cargo"}, {"section": "editor"}]}),
        ),
        0,
    );
    assert_eq!(
        to_server(&out)[0]["result"],
        json!([{"check": {"workspace": false}, "cargo": {"targetDir": false}}, {"targetDir": false}, null])
    );
    assert!(silent(&out));

    // Progress is accepted locally; its reports reach no client.
    let out = server(
        &mut mux,
        ask(2, "window/workDoneProgress/create", json!({"token": "t"})),
        0,
    );
    assert_eq!(to_server(&out)[0]["result"], Value::Null);
    let out = server(
        &mut mux,
        json!({"jsonrpc": "2.0", "method": "$/progress", "params": {"token": "t", "value": {"kind": "begin"}}}),
        0,
    );
    assert!(silent(&out));

    // Edits are refused: agents edit through their own tools.
    let out = server(
        &mut mux,
        ask(3, "workspace/applyEdit", json!({"edit": {}})),
        0,
    );
    assert_eq!(to_server(&out)[0]["result"]["applied"], false);
    assert!(silent(&out));

    // Anything unclassified is refused, not handed to whichever client spoke last.
    for (id, method) in [
        (4, "window/showMessageRequest"),
        (5, "window/showDocument"),
        (6, "x/unknown"),
    ] {
        let out = server(&mut mux, ask(id, method, json!({})), 0);
        assert_eq!(to_server(&out)[0]["error"]["code"], -32601, "{method}");
        assert!(silent(&out), "{method}");
    }

    // A registration is accepted and reaches every client, and a late client on `initialized`.
    let out = server(
        &mut mux,
        ask(
            7,
            "client/registerCapability",
            json!({"registrations": [{"id": "w", "method": "workspace/didChangeWatchedFiles"}]}),
        ),
        0,
    );
    assert_eq!(to_server(&out)[0]["result"], Value::Null);
    assert_eq!(to_client(&out, A).len(), 1);
    assert_eq!(to_client(&out, B).len(), 1);
    attach(&mut mux, ClientId(3), "c");
    from(&mut mux, ClientId(3), initialize(json!(1)), 0);
    let out = from(
        &mut mux,
        ClientId(3),
        json!({"jsonrpc": "2.0", "method": "initialized"}),
        0,
    );
    assert_eq!(
        to_client(&out, ClientId(3))[0]["method"],
        "client/registerCapability"
    );
    // A client's answer to the broadcast is swallowed.
    let reply = json!({"jsonrpc": "2.0", "id": "epr-mux-b:1", "result": null});
    assert!(to_server(&from(&mut mux, A, reply, 0)).is_empty());
}

#[test]
fn a_detaching_client_cancels_its_requests_and_late_responses_are_dropped() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let out = from(
        &mut mux,
        A,
        json!({"jsonrpc": "2.0", "id": 1, "method": "textDocument/hover", "params": {}}),
        0,
    );
    let sid = to_server(&out)[0]["id"].clone();
    let out = mux.handle(Input::Detach { client: A }, 0);
    assert!(to_server(&out)
        .iter()
        .any(|m| m["method"] == "$/cancelRequest" && m["params"]["id"] == sid));
    let out = server(
        &mut mux,
        json!({"jsonrpc": "2.0", "id": sid, "result": 1}),
        0,
    );
    assert!(
        out.iter().all(|o| !matches!(o, Output::ToClient(..))),
        "late response dropped"
    );
}

// (e)
#[test]
fn opens_and_closes_are_refcounted_per_uri() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let uri = "file:///r/core/src/lib.rs";
    fs.write(uri, "fn a() {}");
    let out = from(&mut mux, A, open(uri, "fn a() {}"), 0);
    assert_eq!(to_server(&out)[0]["method"], "textDocument/didOpen");
    let out = from(&mut mux, B, open(uri, "fn a() {}"), 0);
    assert!(to_server(&out).is_empty(), "a second open is not forwarded");
    let close = json!({"jsonrpc": "2.0", "method": "textDocument/didClose", "params": {"textDocument": {"uri": uri}}});
    let out = from(&mut mux, A, close.clone(), 0);
    assert!(to_server(&out).is_empty());
    let out = from(&mut mux, B, close, 0);
    assert_eq!(
        to_server(&out)[0]["method"],
        "textDocument/didClose",
        "the last close is forwarded"
    );
}

#[test]
fn the_server_only_ever_sees_disk_text_never_a_divergent_buffer() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let uri = "file:///r/core/src/lib.rs";
    fs.write(uri, "disk v1");

    // A's buffer and B's buffer both differ from the disk, and from each other.
    let out = from(&mut mux, A, open(uri, "buffer A"), 0);
    assert_eq!(
        to_server(&out)[0]["params"]["textDocument"]["text"],
        "disk v1",
        "the first open carries the disk's text"
    );
    let out = from(&mut mux, B, open(uri, "buffer B"), 0);
    assert!(to_server(&out).is_empty(), "the disk has not moved");

    // Buffer-only changes (the disk is unchanged) reach the server as nothing at all.
    for (client, buffer) in [(A, "buffer A2"), (B, "buffer B2")] {
        let out = from(&mut mux, client, change(uri, buffer), 10);
        assert!(to_server(&out).is_empty(), "{buffer}");
    }

    // The disk moves: the next change from either client syncs the disk's text, once.
    fs.write(uri, "disk v2");
    let out = from(&mut mux, B, change(uri, "buffer B3"), 20);
    let sent = to_server(&out);
    assert_eq!(sent[0]["method"], "textDocument/didChange");
    assert_eq!(sent[0]["params"]["contentChanges"][0]["text"], "disk v2");
    assert_eq!(sent[0]["params"]["textDocument"]["version"], 2);
    let out = from(&mut mux, A, change(uri, "buffer A3"), 21);
    assert!(
        to_server(&out).is_empty(),
        "the server already holds disk v2"
    );

    // A save syncs the disk and forwards the save without the client's buffer.
    fs.write(uri, "disk v3");
    let out = from(&mut mux, A, save(uri), 30);
    let sent = to_server(&out);
    assert_eq!(sent[0]["params"]["contentChanges"][0]["text"], "disk v3");
    assert_eq!(sent[1]["method"], "textDocument/didSave");
    assert!(sent[1]["params"].get("text").is_none());

    // A client that opens after the disk moved re-syncs the server to the disk, not to itself.
    fs.write(uri, "disk v4");
    attach(&mut mux, ClientId(3), "c");
    from(&mut mux, ClientId(3), initialize(json!(1)), 30);
    let out = from(&mut mux, ClientId(3), open(uri, "buffer C"), 31);
    assert_eq!(
        to_server(&out)[0]["params"]["contentChanges"][0]["text"],
        "disk v4"
    );

    // A document with no file behind it is the only one a buffer may stand in for.
    let scratch = "file:///r/scratch.rs";
    let out = from(&mut mux, A, open(scratch, "unsaved"), 40);
    assert_eq!(
        to_server(&out)[0]["params"]["textDocument"]["text"],
        "unsaved"
    );
    let out = from(&mut mux, A, change(scratch, "still unsaved"), 41);
    assert!(
        to_server(&out).is_empty(),
        "no disk: the server keeps its copy"
    );
}

// (f)
#[test]
fn a_new_diagnostic_is_attributed_and_delivered_per_session() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let core = "file:///r/core/src/lib.rs";
    let client = "file:///r/client/src/main.rs";
    fs.write(core, "pub struct T;");
    fs.write(client, "use core::T;");
    from(&mut mux, A, open(core, "pub struct T;"), 0);
    from(&mut mux, B, open(client, "use core::T;"), 0);
    edit(&mut mux, &fs, A, core, "pub struct U;", 1_000);
    let out = from(&mut mux, A, save(core), 1_000);
    assert_eq!(
        to_server(&out).last().unwrap()["method"],
        "textDocument/didSave"
    );

    let out = server(
        &mut mux,
        publish(client, vec![diagnostic(0, 1, "unresolved import")]),
        5_000,
    );
    let a = publishes(&out, A);
    let b = publishes(&out, B);
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].0, client);
    assert_eq!(a[0].1[0]["data"]["epr"]["attribution"], "caused-by-me");
    assert_eq!(a[0].1[0]["message"], "unresolved import");
    assert!(a[0].1[0].get("version").is_none());
    assert_eq!(b[0].1[0]["message"], "[by aaaaaaaa] unresolved import");
    assert_eq!(b[0].1[0]["data"]["epr"]["owners"], json!(["aaaaaaaa"]));
    assert!(out.contains(&Output::SnapshotChanged));

    // B's later error in another area reaches A only as a fold, from the C2 log.
    mux.handle(
        Input::Edits(vec![EditEvent {
            at_ms: 60_000,
            session: SessionKey::named(SESSION_B),
            path: Some("/r/other/src/lib.rs".into()),
            source: EditSource::Log,
        }]),
        60_000,
    );
    let other = "file:///r/other/src/lib.rs";
    let out = server(
        &mut mux,
        publish(
            other,
            vec![
                diagnostic(1, 1, "e1"),
                diagnostic(2, 1, "e2"),
                diagnostic(3, 2, "w"),
            ],
        ),
        61_000,
    );
    let a = publishes(&out, A);
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].0, "file:///r/other/Cargo.toml");
    assert_eq!(
        a[0].1[0]["message"],
        "other: 2 errors, 1 warnings from session bbbbbbbb (in flight)"
    );
    let b = publishes(&out, B);
    assert_eq!(b[0].0, other);
    assert_eq!(
        b[0].1.len(),
        3,
        "the area's editor receives all three in full"
    );

    // An edit in a sibling worktree is not this root's business.
    mux.handle(
        Input::Edits(vec![EditEvent {
            at_ms: 100_000,
            session: SessionKey::named("elsewhere"),
            path: Some("/s/core/src/lib.rs".into()),
            source: EditSource::Log,
        }]),
        100_000,
    );
    let out = server(
        &mut mux,
        publish(core, vec![diagnostic(9, 1, "late")]),
        101_000,
    );
    assert_eq!(
        publishes(&out, A)[0].1[0]["data"]["epr"]["attribution"],
        "unattributed"
    );
}

#[test]
fn edits_by_two_sessions_in_one_window_make_a_diagnostic_ambiguous() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let a_file = "file:///r/core/src/a.rs";
    let b_file = "file:///r/core/src/b.rs";
    fs.write(a_file, "a0");
    fs.write(b_file, "b0");
    from(&mut mux, A, open(a_file, "a0"), 0);
    from(&mut mux, B, open(b_file, "b0"), 0);
    edit(&mut mux, &fs, A, a_file, "a1", 1_000);
    edit(&mut mux, &fs, B, b_file, "b1", 2_000);
    let out = server(
        &mut mux,
        publish("file:///r/other/src/lib.rs", vec![diagnostic(1, 1, "E")]),
        3_000,
    );
    for client in [A, B] {
        let caused = publishes(&out, client);
        assert_eq!(caused[0].1[0]["data"]["epr"]["attribution"], "ambiguous");
        assert_eq!(
            caused[0].1[0]["data"]["epr"]["owners"],
            json!(["aaaaaaaa", "bbbbbbbb"])
        );
    }
}

#[test]
fn the_c2_log_is_the_authority_on_who_edited_a_file() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let core = "file:///r/core/src/lib.rs";
    fs.write(core, "v0");
    from(&mut mux, A, open(core, "v0"), 0);
    from(&mut mux, B, open(core, "v0"), 0);
    // A edits the file through its tool (C2), and B's client is the first to report the change.
    fs.write(core, "v1");
    mux.handle(
        Input::Edits(vec![EditEvent {
            at_ms: 1_000,
            session: SessionKey::named(SESSION_A),
            path: Some("/r/core/src/lib.rs".into()),
            source: EditSource::Log,
        }]),
        1_000,
    );
    let out = from(&mut mux, B, change(core, "v1"), 1_100);
    assert_eq!(
        to_server(&out).len(),
        1,
        "the disk's text still reaches the server"
    );
    let out = server(
        &mut mux,
        publish("file:///r/client/src/main.rs", vec![diagnostic(0, 1, "E")]),
        2_000,
    );
    let b = publishes(&out, B);
    let delivered = b
        .iter()
        .find(|(uri, _)| uri.ends_with("client/Cargo.toml"))
        .expect("B never edited: the break folds for it");
    assert!(delivered.1[0]["message"]
        .as_str()
        .unwrap()
        .contains("from session aaaaaaaa"));
}

// (g)
#[test]
fn a_fold_dropping_to_zero_clears_its_anchor() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let other = "file:///r/other/src/lib.rs";
    let out = server(&mut mux, publish(other, vec![diagnostic(1, 1, "e")]), 100);
    let anchor = "file:///r/other/Cargo.toml".to_owned();
    let folded = publishes(&out, A);
    assert_eq!(folded.len(), 1);
    assert_eq!(folded[0].0, anchor);
    assert_eq!(
        folded[0].1[0]["message"],
        "other: 1 errors, 0 warnings (unattributed)"
    );
    let out = server(&mut mux, publish(other, vec![]), 200);
    assert_eq!(publishes(&out, A), vec![(anchor, vec![])]);
}

// (h)
#[test]
fn a_newly_attached_client_receives_the_current_view() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let core = "file:///r/core/src/lib.rs";
    fs.write(core, "x");
    from(&mut mux, A, open(core, "x"), 0);
    edit(&mut mux, &fs, A, core, "y", 1_000);
    server(&mut mux, publish(core, vec![diagnostic(0, 1, "E")]), 2_000);
    attach(&mut mux, ClientId(3), "cccccccc");
    let out = from(&mut mux, ClientId(3), initialize(json!(1)), 3_000);
    let late = publishes(&out, ClientId(3));
    assert_eq!(late.len(), 1);
    assert_eq!(late[0].0, "file:///r/core/Cargo.toml");
    assert_eq!(
        late[0].1[0]["message"],
        "core: 1 errors, 0 warnings from session aaaaaaaa (in flight)"
    );
}

// (i), the mux half: what every client hears before the daemon closes it.
#[test]
fn a_server_crash_or_pin_change_reaches_every_client_as_message_and_diagnostic() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    from(&mut mux, A, open("file:///r/core/src/lib.rs", "x"), 0);
    let out = mux.crash_notices("exit status 101");
    let a = to_client(&out, A);
    assert_eq!(a[0]["method"], "window/showMessage");
    assert_eq!(a[1]["params"]["uri"], "file:///r/core/src/lib.rs");
    assert_eq!(a[1]["params"]["diagnostics"][0]["source"], "epr-lsp");
    let b = to_client(&out, B);
    assert_eq!(
        b[1]["params"]["uri"], "file:///r/Cargo.toml",
        "B opened nothing"
    );
    let out = mux.pin_notices("1.96.1", "1.97.0");
    assert_eq!(
        to_client(&out, B)[0]["params"]["message"],
        "toolchain pin changed 1.96.1 → 1.97.0; restarting"
    );
}

#[test]
fn code_action_context_is_unstamped_and_folds_removed() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let out = from(
        &mut mux,
        A,
        json!({"jsonrpc": "2.0", "id": 4, "method": "textDocument/codeAction", "params": {
            "textDocument": {"uri": "file:///r/x.rs"},
            "context": {"diagnostics": [
                {"message": "[by bbbbbbbb] boom", "data": {"epr": {"attribution": "other", "owners": ["bbbbbbbb"]}}},
                {"message": "fold", "data": {"epr": {"fold": true}}}
            ]}
        }}),
        0,
    );
    assert_eq!(
        to_server(&out)[0]["params"]["context"]["diagnostics"],
        json!([{"message": "boom"}])
    );
}

#[test]
fn snapshot_lists_sessions_and_owned_diagnostics() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let core = "file:///r/core/src/lib.rs";
    fs.write(core, "x");
    from(&mut mux, A, open(core, "x"), 0);
    edit(&mut mux, &fs, A, core, "y", 10);
    server(&mut mux, publish(core, vec![diagnostic(0, 1, "E")]), 20);
    let snapshot = mux.snapshot(30);
    assert_eq!(snapshot.diagnostics.len(), 1);
    assert_eq!(
        snapshot.diagnostics[0].attribution.owner[0].session,
        SESSION_A
    );
    assert_eq!(snapshot.view(SESSION_A).mine.len(), 1);
    assert_eq!(snapshot.view(SESSION_B).others.len(), 1);
}

#[test]
fn an_external_publisher_rides_the_same_attribution_and_keeps_its_tag() {
    let fs = Fs::default();
    let mut mux = ready_pair(&fs);
    let core = "file:///r/core/src/lib.rs";
    fs.write(core, "x");
    from(&mut mux, A, open(core, "x"), 0);
    edit(&mut mux, &fs, A, core, "y", 1_000);
    server(
        &mut mux,
        publish(core, vec![diagnostic(0, 1, "from the server")]),
        1_500,
    );
    let out = mux.handle(
        Input::External {
            source: "gate-live".into(),
            uri: core.into(),
            diagnostics: vec![diagnostic(4, 1, "clippy: unused")],
        },
        2_000,
    );
    let a = publishes(&out, A);
    assert_eq!(
        a[0].1.len(),
        2,
        "the external set joins, never replaces, the server's"
    );
    let external = &a[0].1[1];
    assert_eq!(external["data"]["epr"]["source"], "gate-live");
    assert_eq!(external["data"]["epr"]["attribution"], "mine");
    let b = publishes(&out, B);
    assert_eq!(
        b[0].1[0]["message"],
        "core: 2 errors, 0 warnings from session aaaaaaaa (in flight)"
    );
}
