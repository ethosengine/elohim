//! The device mux: one resident process, one socket, one language server per `(lang, root)`.
//!
//! `epr lsp serve --device` runs this in the foreground under the device supervisor (ark). Every
//! connection, every child's stdout and every child's exit become [`Event`]s on one bounded
//! channel, consumed by a single task that owns every root's [`Mux`], so no mux needs a lock.
//!
//! A root's server starts lazily, on the first attach that names it, from the [`ServerPlan`] the
//! [`Resolver`] reads fresh for that attach. Its life ends in one of three ways, and the device
//! mux stays up through all of them:
//! - idle reap: zero clients for the plan's idle TTL;
//! - pin change: an attach whose plan pins a different toolchain than the running server; that
//!   root's clients hear "toolchain pin changed <old> → <new>; restarting" and are closed (their
//!   editors reattach), and the attach starts a server on the new pin;
//! - death: that root's clients hear the reason (message and diagnostic) and are closed.
//!
//! Retiring a server follows the D9 sequence without ever blocking the loop: `shutdown`, up to
//! [`SHUTDOWN_WAIT`] for its reply, `exit`, up to [`EXIT_WAIT`], then SIGKILL.
//!
//! Each client has a bounded outbound queue; a client over its bound is told it is too slow and
//! disconnected rather than allowed to stall every other session.

use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Instant};

use crate::attach::null_server;
use crate::attrib::{EditEvent, EditLines};
use crate::deliver::RealFs;
use crate::frame::{encode, read_message, write_message, Decoder};
use crate::limits::{Outbox, CLIENT_QUEUE_MESSAGES, EXIT_WAIT, SHUTDOWN_WAIT, TOO_SLOW};
use crate::mux::{show_message, ClientId, Input, Mux, MuxConfig, Output};
use crate::server::Spawner;
use crate::socket::{
    device_lock, device_socket, edits_log, read_hello, write_atomic, DaemonRecord, Hello,
    RuntimeDir, ServeLock,
};
use crate::uri::path_to_uri;

/// Milliseconds since the Unix epoch, injectable for tests.
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// The wall clock.
#[must_use]
pub fn system_clock() -> Clock {
    Arc::new(|| chrono::Utc::now().timestamp_millis())
}

/// Resolves when the process is asked to stop (SIGTERM from the supervisor, or SIGINT). If the
/// handlers cannot be installed it never resolves, and the supervisor's kill is the stop.
pub async fn termination() {
    use tokio::signal::unix::{signal, SignalKind};
    match (
        signal(SignalKind::terminate()),
        signal(SignalKind::interrupt()),
    ) {
        (Ok(mut term), Ok(mut interrupt)) => {
            tokio::select! {
                _ = term.recv() => {}
                _ = interrupt.recv() => {}
            }
        }
        _ => {
            log("cannot install stop signal handlers; only a kill stops this mux");
            std::future::pending::<()>().await;
        }
    }
}

/// The bound on the event channel between tasks.
const QUEUE: usize = 1024;
/// The bound on one server's inbound queue.
const SERVER_QUEUE: usize = 4096;
/// How long a connection has to say hello.
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// How long client writers get to flush a closing notice.
const FLUSH_WAIT: Duration = Duration::from_secs(2);
/// How often idle and retirement deadlines are checked.
const TICK: Duration = Duration::from_millis(250);
/// How long the device stop waits for every retiring server.
const STOP_WAIT: Duration = Duration::from_secs(8);
/// The most C2 bytes read in one poll.
const EDIT_READ_CAP: u64 = 4 << 20;
/// The most C2 events kept to seed a root that starts later.
const RECENT_EDITS: usize = 10_000;

/// Everything one root's server is started from, read fresh at each attach.
pub struct ServerPlan {
    /// The mux's configuration (root, options, window).
    pub mux: MuxConfig,
    /// The toolchain pin; a running server on another pin is replaced.
    pub pin: String,
    /// Zero-client time before this root's server is reaped.
    pub idle_ttl: Duration,
    /// Starts the server.
    pub spawner: Box<dyn Spawner>,
    /// The C3 record written while this root's server runs, if any.
    pub record: Option<DaemonRecord>,
}

impl std::fmt::Debug for ServerPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerPlan")
            .field("root", &self.mux.root)
            .field("pin", &self.pin)
            .finish_non_exhaustive()
    }
}

/// Reads the plan for `(lang, root)`, or says why there is none (the client then gets the null
/// server with that reason).
pub trait Resolver: Send + Sync {
    /// The plan for one attach.
    ///
    /// # Errors
    ///
    /// Why this device cannot serve `lang` for `root`.
    fn plan(&self, lang: &str, root: &Path) -> Result<ServerPlan, String>;
}

/// What the device mux needs.
#[derive(Clone)]
pub struct DeviceConfig {
    /// `<main checkout>/.eprfs/status/lsp`.
    pub runtime_dir: PathBuf,
    /// The clock.
    pub clock: Clock,
    /// Plans each root's server.
    pub resolver: Arc<dyn Resolver>,
}

impl std::fmt::Debug for DeviceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceConfig")
            .field("runtime_dir", &self.runtime_dir)
            .finish_non_exhaustive()
    }
}

type RootKey = (String, PathBuf);

enum Event {
    Attached {
        client: ClientId,
        hello: Hello,
        plan: Box<ServerPlan>,
        conn: ClientConn,
        writer: JoinHandle<()>,
    },
    Client(ClientId, Value),
    ClientGone(ClientId),
    Server {
        key: RootKey,
        generation: u64,
        message: Value,
    },
    ServerGone {
        key: RootKey,
        generation: u64,
        reason: String,
    },
    External {
        key: RootKey,
        input: Input,
    },
}

/// A handle that feeds diagnostics from a non-LSP publisher (a gate tail) into a root's
/// attribution and fold path, tagged with the publisher's name.
#[derive(Clone, Debug)]
pub struct ExternalFeed {
    events: mpsc::Sender<Event>,
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Event")
    }
}

impl ExternalFeed {
    /// Publish `source`'s full current set of diagnostics for `uri` under `(lang, root)`. A root
    /// with no running server has no sessions to deliver to, and drops the set. Returns `false`
    /// when the device mux has stopped.
    pub async fn publish(
        &self,
        lang: &str,
        root: &Path,
        source: &str,
        uri: &str,
        diagnostics: Vec<Value>,
    ) -> bool {
        let event = Event::External {
            key: (lang.to_owned(), root.to_path_buf()),
            input: Input::External {
                source: source.to_owned(),
                uri: uri.to_owned(),
                diagnostics,
            },
        };
        self.events.send(event).await.is_ok()
    }
}

enum ClientOut {
    /// One encoded frame.
    Frame(Vec<u8>),
    Close,
}

/// The loop's handle on one client: its queue, and that queue's accounting (D9). The channel
/// holds two slots past the message bound, so the closing notice and the close always fit.
#[derive(Clone)]
struct ClientConn {
    sender: mpsc::Sender<ClientOut>,
    outbox: Arc<Outbox>,
}

impl ClientConn {
    /// Queue one message, or `false` when the client is over its bound.
    fn offer(&self, message: &Value) -> bool {
        let frame = encode(message);
        if !self.outbox.admit(frame.len()) {
            return false;
        }
        // Admission bounds the queue below the channel's capacity, so this cannot be Full; a
        // Closed channel means the client already left and its ClientGone event is on the way.
        let _ = self.sender.try_send(ClientOut::Frame(frame));
        true
    }

    /// Queue a final message past the bound, then close.
    fn close_with(&self, message: Option<&Value>) {
        if let Some(message) = message {
            let _ = self.sender.try_send(ClientOut::Frame(encode(message)));
        }
        let _ = self.sender.try_send(ClientOut::Close);
    }
}

pub(crate) fn log(line: &str) {
    eprintln!(
        "{} epr lsp: {line}",
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    );
}

/// Reads the C2 log incrementally, starting from its beginning, tolerating in-place pruning.
#[derive(Debug)]
struct EditTail {
    path: PathBuf,
    offset: u64,
    lines: EditLines,
}

impl EditTail {
    async fn poll(&mut self) -> Vec<EditEvent> {
        let Ok(mut file) = tokio::fs::File::open(&self.path).await else {
            return Vec::new();
        };
        let Ok(len) = file.metadata().await.map(|m| m.len()) else {
            return Vec::new();
        };
        if len < self.offset {
            // The writer pruned the log in place: read it again from the top. Events already
            // seen are deduplicated by each attributor.
            self.offset = 0;
            self.lines.reset();
        }
        if file.seek(SeekFrom::Start(self.offset)).await.is_err() {
            return Vec::new();
        }
        let mut bytes = Vec::new();
        let Ok(read) = (&mut file)
            .take(EDIT_READ_CAP)
            .read_to_end(&mut bytes)
            .await
        else {
            return Vec::new();
        };
        self.offset += read as u64;
        self.lines.feed(&bytes)
    }
}

/// A device mux that holds the device lock and listens on the device socket.
pub struct Bound {
    listener: UnixListener,
    _lock: ServeLock,
    config: DeviceConfig,
    sock: PathBuf,
    events_tx: mpsc::Sender<Event>,
    events: mpsc::Receiver<Event>,
}

impl std::fmt::Debug for Bound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bound")
            .field("sock", &self.sock)
            .finish_non_exhaustive()
    }
}

/// Take the device lock and bind the device socket. `None` when another device mux holds the
/// lock.
///
/// # Errors
///
/// The runtime directory, the lock or the socket could not be created.
pub fn bind(config: DeviceConfig) -> std::io::Result<Option<Bound>> {
    std::fs::create_dir_all(&config.runtime_dir)?;
    let Some(lock) = ServeLock::try_acquire(&device_lock(&config.runtime_dir))? else {
        return Ok(None);
    };
    let sock = device_socket(&config.runtime_dir);
    // We hold the lock, so any socket file left here belongs to a dead mux.
    match std::fs::remove_file(&sock) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let listener = UnixListener::bind(&sock)?;
    let (events_tx, events) = mpsc::channel(QUEUE);
    Ok(Some(Bound {
        listener,
        _lock: lock,
        config,
        sock,
        events_tx,
        events,
    }))
}

impl Bound {
    /// The socket this mux listens on.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.sock
    }

    /// A feed for non-LSP diagnostic publishers.
    #[must_use]
    pub fn external_feed(&self) -> ExternalFeed {
        ExternalFeed {
            events: self.events_tx.clone(),
        }
    }

    /// Serve until `stop` resolves, then retire every server (D9 sequence) and return. The
    /// socket file is removed before this returns.
    ///
    /// # Errors
    ///
    /// None today; reserved for a failure that ends the device mux.
    pub async fn run<S: Future<Output = ()> + Send>(self, stop: S) -> std::io::Result<()> {
        let Bound {
            listener,
            _lock,
            config,
            sock,
            events_tx,
            events,
        } = self;
        log(&format!("device mux listening on {}", sock.display()));
        let accept = tokio::spawn(accept_loop(
            listener,
            events_tx.clone(),
            Arc::clone(&config.resolver),
        ));
        let mut device = Device {
            tail: EditTail {
                path: edits_log(&config.runtime_dir),
                offset: 0,
                lines: EditLines::default(),
            },
            config,
            roots: HashMap::new(),
            retiring: Vec::new(),
            clients: HashMap::new(),
            writers: Vec::new(),
            events,
            events_tx,
            next_generation: 1,
            recent: VecDeque::new(),
        };
        let initial = device.tail.poll().await;
        device.remember(initial);
        device.serve(stop).await;
        accept.abort();
        let _ = std::fs::remove_file(&sock);
        device.stop().await;
        log("device mux stopped");
        Ok(())
    }
}

struct Root {
    key: RootKey,
    generation: u64,
    pin: String,
    mux: Mux<RealFs>,
    server_tx: mpsc::Sender<Value>,
    tasks: Vec<JoinHandle<()>>,
    exit_watch: JoinHandle<()>,
    clients: HashSet<ClientId>,
    idle_since: Option<Instant>,
    idle_ttl: Duration,
    runtime: RuntimeDir,
    record: bool,
    snapshot_dirty: bool,
}

enum Stage {
    AwaitShutdown(u64),
    AwaitExit,
}

struct Retiring {
    key: RootKey,
    generation: u64,
    server_tx: mpsc::Sender<Value>,
    tasks: Vec<JoinHandle<()>>,
    exit_watch: JoinHandle<()>,
    stage: Stage,
    deadline: Instant,
}

impl Retiring {
    fn finish(self) {
        // Dropping the exit watch drops the child, which kills it (SIGKILL, `kill_on_drop`).
        self.exit_watch.abort();
        for task in self.tasks {
            task.abort();
        }
    }
}

struct Device {
    config: DeviceConfig,
    roots: HashMap<RootKey, Root>,
    retiring: Vec<Retiring>,
    clients: HashMap<ClientId, (RootKey, ClientConn)>,
    writers: Vec<JoinHandle<()>>,
    events: mpsc::Receiver<Event>,
    events_tx: mpsc::Sender<Event>,
    next_generation: u64,
    tail: EditTail,
    recent: VecDeque<EditEvent>,
}

impl Device {
    fn now(&self) -> i64 {
        (self.config.clock)()
    }

    async fn serve<S: Future<Output = ()> + Send>(&mut self, stop: S) {
        let mut tick = tokio::time::interval(TICK);
        tokio::pin!(stop);
        loop {
            tokio::select! {
                () = &mut stop => return,
                _ = tick.tick() => self.on_tick(),
                event = self.events.recv() => match event {
                    Some(event) => self.on_event(event).await,
                    None => return,
                },
            }
            self.writers.retain(|writer| !writer.is_finished());
        }
    }

    async fn on_event(&mut self, event: Event) {
        match event {
            Event::Attached {
                client,
                hello,
                plan,
                conn,
                writer,
            } => {
                self.writers.push(writer);
                self.attach(client, hello, *plan, conn).await;
            }
            Event::Client(client, message) => {
                let Some((key, _)) = self.clients.get(&client).cloned() else {
                    return;
                };
                let method = message.get("method").and_then(Value::as_str);
                if matches!(
                    method,
                    Some("textDocument/didChange" | "textDocument/didSave")
                ) {
                    // Who edited a file is the C2 log's to say; read it before the LSP event.
                    self.poll_edits().await;
                }
                self.feed(&key, Input::FromClient { client, message }).await;
            }
            Event::ClientGone(client) => {
                if let Some((key, _)) = self.clients.remove(&client) {
                    self.feed(&key, Input::Detach { client }).await;
                    self.forget_client(&key, client);
                }
            }
            Event::Server {
                key,
                generation,
                message,
            } => {
                if self
                    .roots
                    .get(&key)
                    .is_some_and(|r| r.generation == generation)
                {
                    if message.get("method").and_then(Value::as_str)
                        == Some("textDocument/publishDiagnostics")
                    {
                        self.poll_edits().await;
                    }
                    self.feed(&key, Input::FromServer(message)).await;
                } else {
                    self.retiring_reply(&key, generation, &message);
                }
            }
            Event::ServerGone {
                key,
                generation,
                reason,
            } => {
                if let Some(index) = self
                    .retiring
                    .iter()
                    .position(|r| r.key == key && r.generation == generation)
                {
                    self.retiring.swap_remove(index).finish();
                } else if self
                    .roots
                    .get(&key)
                    .is_some_and(|r| r.generation == generation)
                {
                    log(&format!(
                        "server for {} {} died: {reason}",
                        key.0,
                        key.1.display()
                    ));
                    if let Some(root) = self.roots.remove(&key) {
                        let notices = root.mux.crash_notices(&reason);
                        self.close_root(root, notices, false);
                    }
                }
            }
            Event::External { key, input } => {
                if self.roots.contains_key(&key) {
                    self.feed(&key, input).await;
                }
            }
        }
    }

    async fn attach(&mut self, client: ClientId, hello: Hello, plan: ServerPlan, conn: ClientConn) {
        let key: RootKey = (hello.lang.clone(), hello.root.clone());
        if let Some(running) = self.roots.get(&key) {
            if running.pin != plan.pin {
                let old = running.pin.clone();
                log(&format!(
                    "pin for {} {} moved {old} → {}; replacing its server",
                    key.0,
                    key.1.display(),
                    plan.pin
                ));
                if let Some(root) = self.roots.remove(&key) {
                    let notices = root.mux.pin_notices(&old, &plan.pin);
                    self.close_root(root, notices, true);
                }
            }
        }
        if !self.roots.contains_key(&key) {
            match self.start_root(&key, plan) {
                Ok(root) => {
                    self.roots.insert(key.clone(), root);
                    let recent: Vec<EditEvent> = self.recent.iter().cloned().collect();
                    self.feed(&key, Input::Edits(recent)).await;
                }
                Err(error) => {
                    let text = format!("epr lsp: cannot start the {} server: {error}", key.0);
                    log(&text);
                    conn.close_with(Some(&show_message(1, &text)));
                    return;
                }
            }
        }
        self.clients.insert(client, (key.clone(), conn));
        if let Some(root) = self.roots.get_mut(&key) {
            root.clients.insert(client);
            root.idle_since = None;
        }
        self.feed(
            &key,
            Input::Attach {
                client,
                session: hello.key,
            },
        )
        .await;
    }

    fn start_root(&mut self, key: &RootKey, mut plan: ServerPlan) -> std::io::Result<Root> {
        let io = plan.spawner.spawn()?;
        let generation = self.next_generation;
        self.next_generation += 1;
        let (server_tx, server_rx) = mpsc::channel(SERVER_QUEUE);
        let writer = tokio::spawn(write_server(io.stdin, server_rx));
        let reader = tokio::spawn(read_server(
            io.stdout,
            self.events_tx.clone(),
            key.clone(),
            generation,
        ));
        let gone = self.events_tx.clone();
        let gone_key = key.clone();
        let exited = io.exited;
        let exit_watch = tokio::spawn(async move {
            let reason = exited.await;
            let _ = gone
                .send(Event::ServerGone {
                    key: gone_key,
                    generation,
                    reason,
                })
                .await;
        });
        let runtime = RuntimeDir::new(self.config.runtime_dir.clone(), &key.0, &key.1);
        let record = match &plan.record {
            Some(record) => serde_json::to_vec_pretty(record)
                .ok()
                .is_some_and(|bytes| write_atomic(&runtime.json(), &bytes).is_ok()),
            None => false,
        };
        log(&format!(
            "started the {} server for {} on {}",
            key.0,
            key.1.display(),
            plan.pin
        ));
        Ok(Root {
            key: key.clone(),
            generation,
            pin: plan.pin,
            mux: Mux::new(plan.mux, RealFs),
            server_tx,
            tasks: vec![writer, reader],
            exit_watch,
            clients: HashSet::new(),
            idle_since: None,
            idle_ttl: plan.idle_ttl,
            runtime,
            record,
            snapshot_dirty: false,
        })
    }

    /// Tell a root's clients why, close them, remove its files, and (when the child still
    /// runs) begin its D9 retirement.
    fn close_root(&mut self, mut root: Root, notices: Vec<Output>, retire: bool) {
        for output in notices {
            if let Output::ToClient(client, message) = output {
                if let Some((_, conn)) = self.clients.get(&client) {
                    if !conn.offer(&message) {
                        conn.close_with(Some(&message));
                    }
                }
            }
        }
        for client in root.clients.drain() {
            if let Some((_, conn)) = self.clients.remove(&client) {
                conn.close_with(None);
            }
        }
        let _ = std::fs::remove_file(root.runtime.attrib());
        if root.record {
            let _ = std::fs::remove_file(root.runtime.json());
        }
        let key = root.key.clone();
        if !retire {
            Retiring {
                key,
                generation: root.generation,
                server_tx: root.server_tx,
                tasks: root.tasks,
                exit_watch: root.exit_watch,
                stage: Stage::AwaitExit,
                deadline: Instant::now(),
            }
            .finish();
            return;
        }
        let now = Instant::now();
        let (stage, deadline) = if root.mux.initialized() {
            let (id, request) = root.mux.shutdown_request();
            let _ = root.server_tx.try_send(request);
            (Stage::AwaitShutdown(id), now + SHUTDOWN_WAIT)
        } else {
            let _ = root.server_tx.try_send(Mux::<RealFs>::exit_notification());
            (Stage::AwaitExit, now + EXIT_WAIT)
        };
        self.retiring.push(Retiring {
            key,
            generation: root.generation,
            server_tx: root.server_tx,
            tasks: root.tasks,
            exit_watch: root.exit_watch,
            stage,
            deadline,
        });
    }

    /// A retiring server answered its `shutdown`: send `exit` and give it [`EXIT_WAIT`].
    fn retiring_reply(&mut self, key: &RootKey, generation: u64, message: &Value) {
        let Some(retiring) = self
            .retiring
            .iter_mut()
            .find(|r| &r.key == key && r.generation == generation)
        else {
            return;
        };
        if let Stage::AwaitShutdown(id) = retiring.stage {
            if message.get("id").and_then(Value::as_u64) == Some(id)
                && message.get("method").is_none()
            {
                let _ = retiring
                    .server_tx
                    .try_send(Mux::<RealFs>::exit_notification());
                retiring.stage = Stage::AwaitExit;
                retiring.deadline = Instant::now() + EXIT_WAIT;
            }
        }
    }

    fn on_tick(&mut self) {
        let now = Instant::now();
        let idle: Vec<RootKey> = self
            .roots
            .iter()
            .filter(|(_, root)| {
                root.clients.is_empty()
                    && root
                        .idle_since
                        .is_some_and(|since| now.duration_since(since) >= root.idle_ttl)
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in idle {
            if let Some(root) = self.roots.remove(&key) {
                log(&format!(
                    "the {} server for {} is idle; reaping it",
                    key.0,
                    key.1.display()
                ));
                // No clients: nothing to tell, nothing to await but the retirement itself.
                self.close_root(root, Vec::new(), true);
            }
        }
        let mut index = 0;
        while index < self.retiring.len() {
            if self.retiring[index].deadline > now {
                index += 1;
                continue;
            }
            match self.retiring[index].stage {
                Stage::AwaitShutdown(_) => {
                    log("a retiring server did not answer shutdown; sending exit");
                    let retiring = &mut self.retiring[index];
                    let _ = retiring
                        .server_tx
                        .try_send(Mux::<RealFs>::exit_notification());
                    retiring.stage = Stage::AwaitExit;
                    retiring.deadline = now + EXIT_WAIT;
                    index += 1;
                }
                Stage::AwaitExit => {
                    log("a retiring server did not exit; killing it");
                    self.retiring.swap_remove(index).finish();
                }
            }
        }
    }

    async fn poll_edits(&mut self) {
        let edits = self.tail.poll().await;
        if edits.is_empty() {
            return;
        }
        self.remember(edits.clone());
        let keys: Vec<RootKey> = self.roots.keys().cloned().collect();
        for key in keys {
            self.feed(&key, Input::Edits(edits.clone())).await;
        }
    }

    fn remember(&mut self, edits: Vec<EditEvent>) {
        self.recent.extend(edits);
        while self.recent.len() > RECENT_EDITS {
            self.recent.pop_front();
        }
    }

    fn forget_client(&mut self, key: &RootKey, client: ClientId) {
        if let Some(root) = self.roots.get_mut(key) {
            root.clients.remove(&client);
            if root.clients.is_empty() {
                root.idle_since.get_or_insert_with(Instant::now);
            }
        }
    }

    async fn feed(&mut self, key: &RootKey, input: Input) {
        let mut queue = vec![input];
        while let Some(input) = queue.pop() {
            let now = self.now();
            let Some(root) = self.roots.get_mut(key) else {
                return;
            };
            let outputs = root.mux.handle(input, now);
            let server_tx = root.server_tx.clone();
            for output in outputs {
                match output {
                    Output::ToServer(message) => {
                        if server_tx.send(message).await.is_err() {
                            log("a server's stdin is closed");
                        }
                    }
                    Output::ToClient(client, message) => {
                        let Some((_, conn)) = self.clients.get(&client) else {
                            continue;
                        };
                        if !conn.offer(&message) {
                            // Never block the loop on one client (D9): it is told, dropped, and
                            // its editor's restart-on-crash reattaches it.
                            log(&format!(
                                "client {} is too slow; disconnecting it",
                                client.0
                            ));
                            if let Some((_, conn)) = self.clients.remove(&client) {
                                conn.close_with(Some(&show_message(2, TOO_SLOW)));
                            }
                            self.forget_client(key, client);
                            queue.push(Input::Detach { client });
                        }
                    }
                    Output::Close(client) => {
                        if let Some((_, conn)) = self.clients.remove(&client) {
                            conn.close_with(None);
                        }
                        self.forget_client(key, client);
                    }
                    Output::Log(line) => log(&line),
                    Output::SnapshotChanged => {
                        if let Some(root) = self.roots.get_mut(key) {
                            root.snapshot_dirty = true;
                        }
                    }
                }
            }
        }
        self.flush_snapshot(key).await;
    }

    async fn flush_snapshot(&mut self, key: &RootKey) {
        let now = self.now();
        let Some(root) = self.roots.get_mut(key) else {
            return;
        };
        if !std::mem::take(&mut root.snapshot_dirty) {
            return;
        }
        let snapshot = root.mux.snapshot(now);
        let path = root.runtime.attrib();
        let Ok(bytes) = serde_json::to_vec_pretty(&snapshot) else {
            return;
        };
        let written = tokio::task::spawn_blocking(move || write_atomic(&path, &bytes)).await;
        if !matches!(written, Ok(Ok(()))) {
            log("could not write an attribution snapshot");
        }
    }

    /// Retire every server, then wait (bounded) for them to go.
    async fn stop(&mut self) {
        let keys: Vec<RootKey> = self.roots.keys().cloned().collect();
        for key in keys {
            if let Some(root) = self.roots.remove(&key) {
                let notices = root.mux.crash_notices("the device supervisor is stopping");
                self.close_root(root, notices, true);
            }
        }
        let deadline = Instant::now() + STOP_WAIT;
        while !self.retiring.is_empty() && Instant::now() < deadline {
            match timeout(TICK, self.events.recv()).await {
                Ok(Some(Event::ServerGone {
                    key, generation, ..
                })) => {
                    if let Some(index) = self
                        .retiring
                        .iter()
                        .position(|r| r.key == key && r.generation == generation)
                    {
                        self.retiring.swap_remove(index).finish();
                    }
                }
                Ok(Some(Event::Server {
                    key,
                    generation,
                    message,
                })) => self.retiring_reply(&key, generation, &message),
                Ok(Some(_)) | Err(_) => {}
                Ok(None) => break,
            }
            self.on_tick();
        }
        for retiring in self.retiring.drain(..) {
            retiring.finish();
        }
        for writer in self.writers.drain(..) {
            let _ = timeout(FLUSH_WAIT, writer).await;
        }
    }
}

async fn write_server(
    mut stdin: Box<dyn AsyncWrite + Send + Unpin>,
    mut messages: mpsc::Receiver<Value>,
) {
    while let Some(message) = messages.recv().await {
        if write_message(&mut stdin, &message).await.is_err() {
            break;
        }
    }
    let _ = stdin.shutdown().await;
}

async fn read_server(
    mut stdout: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
    events: mpsc::Sender<Event>,
    key: RootKey,
    generation: u64,
) {
    let mut decoder = Decoder::default();
    let reason = loop {
        match read_message(&mut stdout, &mut decoder).await {
            Ok(Some(message)) => {
                let event = Event::Server {
                    key: key.clone(),
                    generation,
                    message,
                };
                if events.send(event).await.is_err() {
                    return;
                }
            }
            Ok(None) => break "server closed its output".to_owned(),
            Err(error) if error.is_recoverable() => log(&format!("server sent {error}")),
            Err(error) => break format!("server output unreadable: {error}"),
        }
    };
    let _ = events
        .send(Event::ServerGone {
            key,
            generation,
            reason,
        })
        .await;
}

async fn accept_loop(
    listener: UnixListener,
    events: mpsc::Sender<Event>,
    resolver: Arc<dyn Resolver>,
) {
    let mut next = 1;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let client = ClientId(next);
                next += 1;
                tokio::spawn(connection(
                    stream,
                    client,
                    events.clone(),
                    Arc::clone(&resolver),
                ));
            }
            Err(error) => {
                log(&format!("accept failed: {error}"));
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

async fn connection(
    stream: UnixStream,
    client: ClientId,
    events: mpsc::Sender<Event>,
    resolver: Arc<dyn Resolver>,
) {
    let (mut read, write) = stream.into_split();
    let mut hello = match timeout(HELLO_TIMEOUT, read_hello(&mut read)).await {
        Ok(Ok(hello)) => hello,
        Ok(Err(error)) => {
            log(&format!("connection refused: {error}"));
            return;
        }
        Err(_) => {
            log("connection refused: no hello within 5 s");
            return;
        }
    };
    hello.root = hello.root.canonicalize().unwrap_or(hello.root);
    let (lang, root) = (hello.lang.clone(), hello.root.clone());
    let planned = tokio::task::spawn_blocking(move || resolver.plan(&lang, &root)).await;
    let plan = match planned {
        Ok(Ok(plan)) => plan,
        Ok(Err(reason)) => return refuse(read, write, &hello, &reason).await,
        Err(error) => return refuse(read, write, &hello, &error.to_string()).await,
    };
    let (sender, outbound) = mpsc::channel(CLIENT_QUEUE_MESSAGES + 2);
    let outbox = Arc::new(Outbox::default());
    let writer = tokio::spawn(write_client(write, outbound, Arc::clone(&outbox)));
    let attached = Event::Attached {
        client,
        hello,
        plan: Box::new(plan),
        conn: ClientConn { sender, outbox },
        writer,
    };
    if events.send(attached).await.is_err() {
        return;
    }
    let mut decoder = Decoder::default();
    loop {
        match read_message(&mut read, &mut decoder).await {
            Ok(Some(message)) => {
                if events.send(Event::Client(client, message)).await.is_err() {
                    return;
                }
            }
            Ok(None) => break,
            Err(error) if error.is_recoverable() => {
                log(&format!("client {} sent {error}", client.0));
            }
            // An oversized frame (D9) or a broken header closes this client only.
            Err(error) => {
                log(&format!("client {}: {error}; closing it", client.0));
                break;
            }
        }
    }
    let _ = events.send(Event::ClientGone(client)).await;
}

/// Serve a client this device cannot plan a server for as the null server, with the reason.
async fn refuse(
    read: tokio::net::unix::OwnedReadHalf,
    write: OwnedWriteHalf,
    hello: &Hello,
    reason: &str,
) {
    log(&format!(
        "no {} server for {}: {reason}",
        hello.lang,
        hello.root.display()
    ));
    let anchor = path_to_uri(&hello.root.join("Cargo.toml"));
    let _ = null_server(read, write, reason, &anchor).await;
}

async fn write_client(
    mut write: OwnedWriteHalf,
    mut outbound: mpsc::Receiver<ClientOut>,
    outbox: Arc<Outbox>,
) {
    while let Some(out) = outbound.recv().await {
        match out {
            ClientOut::Frame(frame) => {
                let written = write.write_all(&frame).await;
                outbox.retire(frame.len());
                if written.is_err() || write.flush().await.is_err() {
                    break;
                }
            }
            ClientOut::Close => break,
        }
    }
    let _ = write.shutdown().await;
}
