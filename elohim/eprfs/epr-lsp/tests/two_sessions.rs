//! The device mux end to end over a real unix socket, with an in-process fake language server
//! behind the spawner seam (no python, no rust-analyzer).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use elohim_epr_lsp::attrib::{rfc3339, SessionKey};
use elohim_epr_lsp::daemon::{bind, Bound, Clock, DeviceConfig, Resolver, ServerPlan};
use elohim_epr_lsp::deliver::Snapshot;
use elohim_epr_lsp::frame::{read_message, write_message, Decoder};
use elohim_epr_lsp::limits::TOO_SLOW;
use elohim_epr_lsp::mux::MuxConfig;
use elohim_epr_lsp::server::{ServerIo, Spawner};
use elohim_epr_lsp::socket::{connect, device_socket, edits_log, Hello, RuntimeDir};
use elohim_epr_lsp::uri::path_to_uri;
use serde_json::{json, Value};
use tokio::io::{duplex, AsyncWriteExt};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{oneshot, Notify};
use tokio::task::JoinHandle;

const SESSION_A: &str = "aaaaaaaa-1111-4111-8111-111111111111";
const SESSION_B: &str = "bbbbbbbb-2222-4222-8222-222222222222";
/// Well past one client's queue bound plus what a socket buffer absorbs.
const FLOOD: usize = 3000;

/// Which diagnostics the fake server publishes when a file is saved.
type Script = Arc<dyn Fn(&str) -> Vec<(String, Vec<Value>)> + Send + Sync>;

/// Every message every fake server received, tagged with its spawn number.
type Received = Arc<Mutex<Vec<(usize, Value)>>>;

/// A fake language server: answers initialize, records every message, publishes scripted
/// diagnostics on didSave, answers other requests with null, and dies on demand.
#[derive(Clone)]
struct FakeSpawner {
    spawned: Arc<AtomicUsize>,
    received: Received,
    script: Script,
    die: Arc<Notify>,
}

impl Spawner for FakeSpawner {
    fn spawn(&mut self) -> std::io::Result<ServerIo> {
        let number = self.spawned.fetch_add(1, Ordering::SeqCst) + 1;
        let (mux_stdin, mut server_stdin) = duplex(1 << 20);
        let (mut server_stdout, mux_stdout) = duplex(1 << 20);
        let received = Arc::clone(&self.received);
        let script = Arc::clone(&self.script);
        let die = Arc::clone(&self.die);
        let task = tokio::spawn(async move {
            let mut decoder = Decoder::default();
            loop {
                let message = tokio::select! {
                    message = read_message(&mut server_stdin, &mut decoder) => message,
                    () = die.notified() => return,
                };
                let Ok(Some(message)) = message else { return };
                received.lock().unwrap().push((number, message.clone()));
                let method = message["method"].as_str().unwrap_or_default().to_owned();
                let id = message.get("id").cloned();
                match (method.as_str(), id) {
                    ("initialize", Some(id)) => {
                        let reply = json!({"jsonrpc": "2.0", "id": id, "result": {
                            "capabilities": {"textDocumentSync": 2}
                        }});
                        write_message(&mut server_stdout, &reply).await.unwrap();
                    }
                    ("exit", None) => return,
                    (_, Some(id)) if !method.is_empty() => {
                        let reply = json!({"jsonrpc": "2.0", "id": id, "result": null});
                        write_message(&mut server_stdout, &reply).await.unwrap();
                    }
                    ("textDocument/didSave", None) => {
                        let saved = message["params"]["textDocument"]["uri"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned();
                        if saved.ends_with("flood.rs") {
                            // Paced, so a reading client keeps up while a stalled one falls
                            // behind its queue bound.
                            let filler = "x".repeat(2048);
                            for index in 0..FLOOD {
                                let log = json!({"jsonrpc": "2.0", "method": "window/logMessage",
                                    "params": {"type": 4, "message": format!("{index} {filler}")}});
                                write_message(&mut server_stdout, &log).await.unwrap();
                                if index % 20 == 0 {
                                    tokio::time::sleep(Duration::from_millis(1)).await;
                                }
                            }
                        }
                        for (uri, diagnostics) in script(&saved) {
                            let publish = json!({"jsonrpc": "2.0",
                                "method": "textDocument/publishDiagnostics",
                                "params": {"uri": uri, "diagnostics": diagnostics}});
                            write_message(&mut server_stdout, &publish).await.unwrap();
                        }
                    }
                    _ => {}
                }
            }
        });
        Ok(ServerIo {
            stdin: Box::new(mux_stdin),
            stdout: Box::new(mux_stdout),
            exited: Box::pin(async move {
                let _ = task.await;
                "fake server exited".to_owned()
            }),
        })
    }
}

/// Plans the fake for the fixture root only, on whatever pin the test sets.
struct FakeResolver {
    root: PathBuf,
    pin: Mutex<String>,
    idle_ttl: Duration,
    spawner: FakeSpawner,
}

impl Resolver for FakeResolver {
    fn plan(&self, lang: &str, root: &Path) -> Result<ServerPlan, String> {
        if root != self.root || lang != "rust" {
            return Err(format!("no {lang} policy for {}", root.display()));
        }
        Ok(ServerPlan {
            mux: MuxConfig {
                root: root.to_path_buf(),
                initialization_options: json!({"checkOnSave": true}),
                config_section: Some("rust-analyzer".into()),
                mux_pid: std::process::id(),
                window_ms: 30_000,
                lang: lang.to_owned(),
            },
            pin: self.pin.lock().unwrap().clone(),
            idle_ttl: self.idle_ttl,
            spawner: Box::new(self.spawner.clone()),
            record: None,
        })
    }
}

struct Client {
    read: OwnedReadHalf,
    write: OwnedWriteHalf,
    decoder: Decoder,
    next_id: i64,
}

impl Client {
    async fn attach(sock: &Path, root: &Path, session: &str) -> Client {
        let mut client = Client::connect(sock, root, session).await;
        client
            .request("initialize", json!({"processId": 1, "capabilities": {}}))
            .await;
        client.notify("initialized", json!({})).await;
        client
    }

    async fn connect(sock: &Path, root: &Path, session: &str) -> Client {
        let hello = Hello {
            key: SessionKey::named(session),
            root: root.to_path_buf(),
            lang: "rust".into(),
        };
        let stream = connect(sock, &hello).await.unwrap();
        let (read, write) = stream.into_split();
        Client {
            read,
            write,
            decoder: Decoder::default(),
            next_id: 1,
        }
    }

    async fn send(&mut self, message: Value) {
        write_message(&mut self.write, &message).await.unwrap();
    }

    async fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}))
            .await;
    }

    async fn next(&mut self) -> Option<Value> {
        tokio::time::timeout(
            Duration::from_secs(10),
            read_message(&mut self.read, &mut self.decoder),
        )
        .await
        .expect("a message within 10 s")
        .ok()
        .flatten()
    }

    /// Every message up to and including the first that matches.
    async fn until(&mut self, what: &str, matches: impl Fn(&Value) -> bool) -> Vec<Value> {
        let mut seen = Vec::new();
        loop {
            let message = self
                .next()
                .await
                .unwrap_or_else(|| panic!("connection closed waiting for {what}; saw {seen:#?}"));
            let done = matches(&message);
            seen.push(message);
            if done {
                return seen;
            }
        }
    }

    /// Everything until the connection closes.
    async fn drain(&mut self) -> Vec<Value> {
        let mut seen = Vec::new();
        while let Some(message) = self.next().await {
            seen.push(message);
        }
        seen
    }

    /// A request and every message received before its response (a barrier).
    async fn request(&mut self, method: &str, params: Value) -> Vec<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .await;
        self.until(method, |m| m.get("method").is_none() && m["id"] == id)
            .await
    }

    async fn open(&mut self, uri: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "rust", "version": 1, "text": text}}),
        )
        .await;
    }

    async fn change(&mut self, uri: &str, text: &str) {
        self.notify(
            "textDocument/didChange",
            json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": text}]}),
        )
        .await;
    }

    async fn save(&mut self, uri: &str) {
        self.notify(
            "textDocument/didSave",
            json!({"textDocument": {"uri": uri}}),
        )
        .await;
    }
}

fn publish_on<'a>(messages: &'a [Value], uri: &str) -> Option<&'a Value> {
    messages
        .iter()
        .rev()
        .find(|m| m["method"] == "textDocument/publishDiagnostics" && m["params"]["uri"] == uri)
}

fn publishes_on(uri: &str) -> impl Fn(&Value) -> bool + '_ {
    move |m| publish_on(std::slice::from_ref(m), uri).is_some()
}

fn error(line: u64, message: &str) -> Value {
    json!({"range": {"start": {"line": line, "character": 0}, "end": {"line": line, "character": 3}},
           "severity": 1, "message": message})
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    runtime_dir: PathBuf,
    clock: Arc<AtomicI64>,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        for package in ["core", "client", "other", "shared"] {
            std::fs::create_dir_all(root.join(package).join("src")).unwrap();
            std::fs::write(
                root.join(package).join("Cargo.toml"),
                format!("[package]\nname = \"{package}\"\n"),
            )
            .unwrap();
        }
        Fixture {
            runtime_dir: root.join(".eprfs/status/lsp"),
            _dir: dir,
            root,
            clock: Arc::new(AtomicI64::new(1_000)),
        }
    }

    fn uri(&self, rel: &str) -> String {
        path_to_uri(&self.root.join(rel))
    }

    fn write(&self, rel: &str, text: &str) {
        std::fs::write(self.root.join(rel), text).unwrap();
    }

    fn sock(&self) -> PathBuf {
        device_socket(&self.runtime_dir)
    }

    fn at(&self, ms: i64) {
        self.clock.store(ms, Ordering::SeqCst);
    }
}

struct Harness {
    spawned: Arc<AtomicUsize>,
    received: Received,
    die: Arc<Notify>,
    resolver: Arc<FakeResolver>,
}

impl Harness {
    fn methods(&self, spawn: usize) -> Vec<String> {
        self.received
            .lock()
            .unwrap()
            .iter()
            .filter(|(number, _)| *number == spawn)
            .filter_map(|(_, m)| m["method"].as_str().map(str::to_owned))
            .collect()
    }
}

fn harness(fx: &Fixture, script: Script, idle_ttl: Duration) -> Harness {
    let spawner = FakeSpawner {
        spawned: Arc::new(AtomicUsize::new(0)),
        received: Arc::new(Mutex::new(Vec::new())),
        script,
        die: Arc::new(Notify::new()),
    };
    Harness {
        spawned: Arc::clone(&spawner.spawned),
        received: Arc::clone(&spawner.received),
        die: Arc::clone(&spawner.die),
        resolver: Arc::new(FakeResolver {
            root: fx.root.clone(),
            pin: Mutex::new("1.96.1".into()),
            idle_ttl,
            spawner,
        }),
    }
}

fn start(
    fx: &Fixture,
    harness: &Harness,
) -> (oneshot::Sender<()>, JoinHandle<std::io::Result<()>>) {
    let clock = Arc::clone(&fx.clock);
    let clock: Clock = Arc::new(move || clock.load(Ordering::SeqCst));
    let resolver: Arc<dyn Resolver> = harness.resolver.clone();
    let config = DeviceConfig {
        runtime_dir: fx.runtime_dir.clone(),
        clock,
        resolver,
    };
    let bound: Bound = bind(config.clone())
        .unwrap()
        .expect("the first device mux binds");
    assert!(
        bind(config).unwrap().is_none(),
        "a second device mux refuses: the lock is held"
    );
    let (stop, stopped) = oneshot::channel::<()>();
    let daemon = tokio::spawn(bound.run(async move {
        let _ = stopped.await;
    }));
    (stop, daemon)
}

fn no_script() -> Script {
    Arc::new(|_: &str| Vec::new())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_sessions_share_one_attributed_server() {
    let fx = Fixture::new();
    let core = fx.uri("core/src/lib.rs");
    let client_rs = fx.uri("client/src/main.rs");
    let other = fx.uri("other/src/lib.rs");
    let shared = fx.uri("shared/src/lib.rs");
    let (c, o, s) = (client_rs.clone(), other.clone(), shared.clone());
    let script: Script = Arc::new(move |saved: &str| {
        if saved.ends_with("core/src/lib.rs") {
            vec![(c.clone(), vec![error(3, "mismatched types")])]
        } else if saved.ends_with("other/src/lib.rs") {
            vec![(o.clone(), vec![error(1, "e1"), error(2, "e2")])]
        } else if saved.ends_with("shared/src/lib.rs") {
            vec![(s.clone(), vec![error(5, "both")])]
        } else if saved.ends_with("client/src/main.rs") {
            vec![(
                c.clone(),
                vec![error(3, "mismatched types"), error(9, "b alone")],
            )]
        } else {
            Vec::new()
        }
    });
    let harness = harness(&fx, script, Duration::from_secs(600));
    let (stop, daemon) = start(&fx, &harness);

    let sock = fx.sock();
    let mut a = Client::attach(&sock, &fx.root, SESSION_A).await;
    let mut b = Client::attach(&sock, &fx.root, SESSION_B).await;
    fx.write("core/src/lib.rs", "pub struct T;");
    fx.write("client/src/main.rs", "use core::T;");
    a.open(&core, "pub struct T;").await;
    b.open(&client_rs, "use core::T;").await;
    // A request round trip orders each client's open before the test touches the disk.
    a.request("textDocument/hover", json!({})).await;
    b.request("textDocument/hover", json!({})).await;

    // A's edit to core breaks B's file: A sees it as caused-by-me, B sees it as A's.
    fx.at(100_000);
    fx.write("core/src/lib.rs", "pub struct U;");
    a.change(&core, "pub struct U;").await;
    a.save(&core).await;
    let seen = a.until("A's caused-by-me", publishes_on(&client_rs)).await;
    let to_a = &publish_on(&seen, &client_rs).unwrap()["params"]["diagnostics"][0];
    assert_eq!(to_a["data"]["epr"]["attribution"], "caused-by-me");
    assert_eq!(to_a["message"], "mismatched types");
    let seen = b
        .until("B's view of A's break", publishes_on(&client_rs))
        .await;
    let to_b = &publish_on(&seen, &client_rs).unwrap()["params"]["diagnostics"][0];
    assert_eq!(to_b["message"], "[by aaaaaaaa] mismatched types");
    assert_eq!(to_b["data"]["epr"]["attribution"], "other");

    // B's errors in another area reach A only as one fold. B's edit arrives through the C2 log.
    fx.at(200_000);
    fx.write("other/src/lib.rs", "broken");
    let line = json!({"v": 1, "at": rfc3339(199_000), "session": SESSION_B, "ownerPid": 1,
        "ownerStartTime": "1", "path": fx.root.join("other/src/lib.rs"), "tool": "Edit"});
    let mut log = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(edits_log(&fx.runtime_dir))
        .await
        .unwrap();
    log.write_all(format!("{line}\n").as_bytes()).await.unwrap();
    b.save(&other).await;
    let anchor = fx.uri("other/Cargo.toml");
    let mut seen = a.until("A's fold", publishes_on(&anchor)).await;
    seen.extend(a.request("textDocument/hover", json!({})).await);
    let fold = &publish_on(&seen, &anchor).unwrap()["params"]["diagnostics"];
    assert_eq!(fold.as_array().unwrap().len(), 1);
    assert_eq!(
        fold[0]["message"],
        "other: 2 errors, 0 warnings from session bbbbbbbb (in flight)"
    );
    assert!(
        publish_on(&seen, &other).is_none(),
        "A never receives B's errors in full"
    );
    let seen = b.until("B's own errors", publishes_on(&other)).await;
    assert_eq!(
        publish_on(&seen, &other).unwrap()["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Both sessions edit inside one window: the new diagnostic is ambiguous for both.
    fx.at(300_000);
    fx.write("shared/src/lib.rs", "s0");
    a.open(&shared, "s0").await;
    b.open(&shared, "s0").await;
    a.request("textDocument/hover", json!({})).await;
    b.request("textDocument/hover", json!({})).await;
    fx.write("shared/src/lib.rs", "s-b");
    b.change(&shared, "s-b").await;
    b.request("textDocument/hover", json!({})).await;
    fx.write("shared/src/lib.rs", "s-a");
    a.change(&shared, "s-a").await;
    a.save(&shared).await;
    for (who, client) in [("A", &mut a), ("B", &mut b)] {
        let seen = client.until(who, publishes_on(&shared)).await;
        let d = &publish_on(&seen, &shared).unwrap()["params"]["diagnostics"][0];
        assert_eq!(d["data"]["epr"]["attribution"], "ambiguous", "{who}");
        assert_eq!(
            d["data"]["epr"]["owners"],
            json!(["aaaaaaaa", "bbbbbbbb"]),
            "{who}"
        );
    }

    let runtime = RuntimeDir::new(fx.runtime_dir.clone(), "rust", &fx.root);
    let snapshot: Snapshot =
        serde_json::from_slice(&std::fs::read(runtime.attrib()).unwrap()).unwrap();
    assert_eq!(
        snapshot.view(SESSION_A).caused_by_me.len(),
        1,
        "show agrees with delivery"
    );

    // A leaves; B keeps working and the server never hears a shutdown.
    let reply = a.request("shutdown", Value::Null).await;
    assert_eq!(reply.last().unwrap()["result"], Value::Null);
    a.notify("exit", Value::Null).await;
    assert!(
        a.next().await.is_none(),
        "the mux closes only A's connection"
    );
    fx.at(400_000);
    fx.write("client/src/main.rs", "use core::U;");
    b.change(&client_rs, "use core::U;").await;
    b.save(&client_rs).await;
    let seen = b
        .until("B after A left", |m| {
            publish_on(std::slice::from_ref(m), &client_rs).is_some_and(|p| {
                p["params"]["diagnostics"]
                    .as_array()
                    .is_some_and(|d| d.len() == 2)
            })
        })
        .await;
    let list = &publish_on(&seen, &client_rs).unwrap()["params"]["diagnostics"];
    assert_eq!(list[1]["data"]["epr"]["attribution"], "mine");
    assert_eq!(
        list[0]["data"]["epr"]["owners"],
        json!(["aaaaaaaa"]),
        "A's break stays A's"
    );

    let methods = harness.methods(1);
    assert_eq!(
        harness.spawned.load(Ordering::SeqCst),
        1,
        "one server for both sessions on one pin"
    );
    assert_eq!(methods.iter().filter(|m| *m == "initialize").count(), 1);
    assert!(!methods.iter().any(|m| m == "shutdown" || m == "exit"));
    assert!(!daemon.is_finished());

    // The supervisor stops the mux: the server is retired by the D9 sequence.
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(15), daemon)
        .await
        .expect("stopped")
        .unwrap()
        .unwrap();
    let methods = harness.methods(1);
    assert!(methods.contains(&"shutdown".to_owned()) && methods.contains(&"exit".to_owned()));
    assert!(!fx.sock().exists(), "no stale socket after a stop");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_server_crash_reaches_its_clients_and_the_mux_stays_up() {
    let fx = Fixture::new();
    let harness = harness(&fx, no_script(), Duration::from_secs(600));
    let (stop, daemon) = start(&fx, &harness);
    let mut a = Client::attach(&fx.sock(), &fx.root, SESSION_A).await;
    let core = fx.uri("core/src/lib.rs");
    fx.write("core/src/lib.rs", "x");
    a.open(&core, "x").await;
    a.request("textDocument/hover", json!({})).await;
    harness.die.notify_one();
    let seen = a.drain().await;
    assert!(seen.iter().any(|m| m["method"] == "window/showMessage"));
    let notice = publish_on(&seen, &core).expect("the reason is a diagnostic too");
    assert_eq!(notice["params"]["diagnostics"][0]["source"], "epr-lsp");
    assert!(
        !daemon.is_finished(),
        "the device mux outlives a root's server"
    );
    assert!(fx.sock().exists());

    // The next attach starts a fresh server for the root.
    let mut b = Client::attach(&fx.sock(), &fx.root, SESSION_B).await;
    b.request("textDocument/hover", json!({})).await;
    assert_eq!(harness.spawned.load(Ordering::SeqCst), 2);
    stop.send(()).unwrap();
    daemon.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_idle_root_is_reaped_while_the_mux_stays_up() {
    let fx = Fixture::new();
    let harness = harness(&fx, no_script(), Duration::from_millis(300));
    let (stop, daemon) = start(&fx, &harness);
    let mut a = Client::attach(&fx.sock(), &fx.root, SESSION_A).await;
    a.request("shutdown", Value::Null).await;
    a.notify("exit", Value::Null).await;
    assert!(a.next().await.is_none());
    let reaped = async {
        loop {
            let methods = harness.methods(1);
            if methods.contains(&"exit".to_owned()) {
                return methods;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    };
    let methods = tokio::time::timeout(Duration::from_secs(10), reaped)
        .await
        .expect("reaped");
    assert!(
        methods.contains(&"shutdown".to_owned()),
        "D9: shutdown before exit"
    );
    assert!(!daemon.is_finished() && fx.sock().exists());
    let mut b = Client::attach(&fx.sock(), &fx.root, SESSION_B).await;
    b.request("textDocument/hover", json!({})).await;
    assert_eq!(
        harness.spawned.load(Ordering::SeqCst),
        2,
        "started lazily again"
    );
    stop.send(()).unwrap();
    daemon.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pin_change_replaces_only_that_roots_server_and_a_matching_pin_joins() {
    let fx = Fixture::new();
    let harness = harness(&fx, no_script(), Duration::from_secs(600));
    let (stop, daemon) = start(&fx, &harness);
    let mut a = Client::attach(&fx.sock(), &fx.root, SESSION_A).await;
    let core = fx.uri("core/src/lib.rs");
    fx.write("core/src/lib.rs", "x");
    a.open(&core, "x").await;
    a.request("textDocument/hover", json!({})).await;

    // Same pin: the second session joins the running server.
    let mut b = Client::attach(&fx.sock(), &fx.root, SESSION_B).await;
    b.request("textDocument/hover", json!({})).await;
    assert_eq!(harness.spawned.load(Ordering::SeqCst), 1);

    // The policy moves the pin; the next attach replaces the server.
    *harness.resolver.pin.lock().unwrap() = "1.97.0".into();
    let mut c = Client::attach(&fx.sock(), &fx.root, "cccccccc-3").await;
    let text = "toolchain pin changed 1.96.1 → 1.97.0; restarting";
    let seen = a.drain().await;
    assert!(seen
        .iter()
        .any(|m| m["method"] == "window/showMessage" && m["params"]["message"] == text));
    let notice = publish_on(&seen, &core).expect("the reason is a diagnostic on A's anchor");
    assert_eq!(notice["params"]["diagnostics"][0]["message"], text);
    let seen = b.drain().await;
    assert!(
        publish_on(&seen, &fx.uri("Cargo.toml")).is_some(),
        "B opened nothing: its notice lands on the root manifest"
    );
    c.request("textDocument/hover", json!({})).await;
    assert_eq!(harness.spawned.load(Ordering::SeqCst), 2);
    let retired = async {
        loop {
            let methods = harness.methods(1);
            if methods.contains(&"exit".to_owned()) {
                return methods;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    };
    let methods = tokio::time::timeout(Duration::from_secs(10), retired)
        .await
        .expect("the old server is retired");
    assert!(methods.contains(&"shutdown".to_owned()));
    assert!(!daemon.is_finished());
    stop.send(()).unwrap();
    daemon.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_that_stops_reading_is_disconnected_without_stalling_the_other() {
    let fx = Fixture::new();
    let harness = harness(&fx, no_script(), Duration::from_secs(600));
    let (stop, daemon) = start(&fx, &harness);
    let mut a = Client::attach(&fx.sock(), &fx.root, SESSION_A).await;
    let mut b = Client::attach(&fx.sock(), &fx.root, SESSION_B).await;
    // B stops reading; A floods and keeps reading through it.
    a.save(&fx.uri("flood.rs")).await;
    let seen = a.request("textDocument/hover", json!({})).await;
    let logs = seen
        .iter()
        .filter(|m| m["method"] == "window/logMessage")
        .count();
    assert_eq!(logs, FLOOD, "the reading client receives every message");
    let tail = b.drain().await;
    assert!(
        tail.iter()
            .any(|m| m["method"] == "window/showMessage" && m["params"]["message"] == TOO_SLOW),
        "the stalled client is told why before it is dropped"
    );
    assert!(
        tail.iter()
            .filter(|m| m["method"] == "window/logMessage")
            .count()
            < FLOOD,
        "the stalled client was cut off, not fed everything"
    );
    a.request("textDocument/hover", json!({})).await;
    stop.send(()).unwrap();
    daemon.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_oversized_frame_closes_only_that_client() {
    let fx = Fixture::new();
    let harness = harness(&fx, no_script(), Duration::from_secs(600));
    let (stop, daemon) = start(&fx, &harness);
    let mut a = Client::attach(&fx.sock(), &fx.root, SESSION_A).await;
    let mut b = Client::attach(&fx.sock(), &fx.root, SESSION_B).await;
    let header = format!(
        "Content-Length: {}\r\n\r\n",
        elohim_epr_lsp::limits::MAX_FRAME + 1
    );
    a.write.write_all(header.as_bytes()).await.unwrap();
    assert!(a.next().await.is_none(), "the oversized client is closed");
    b.request("textDocument/hover", json!({})).await;
    stop.send(()).unwrap();
    daemon.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_root_the_device_cannot_plan_gets_the_null_server_with_the_reason() {
    let fx = Fixture::new();
    let harness = harness(&fx, no_script(), Duration::from_secs(600));
    let (stop, daemon) = start(&fx, &harness);
    let elsewhere = fx.root.join("core");
    let mut a = Client::connect(&fx.sock(), &elsewhere, SESSION_A).await;
    let seen = a
        .request("initialize", json!({"processId": 1, "capabilities": {}}))
        .await;
    assert_eq!(
        seen.last().unwrap()["result"]["serverInfo"]["name"],
        "epr-lsp-null"
    );
    let notice = a.next().await.unwrap();
    assert_eq!(notice["method"], "window/showMessage");
    assert!(notice["params"]["message"]
        .as_str()
        .unwrap()
        .starts_with("no diagnostics: no rust policy"));
    assert_eq!(harness.spawned.load(Ordering::SeqCst), 0);
    stop.send(()).unwrap();
    daemon.await.unwrap().unwrap();
}
