//! The multiplexer as a pure state machine: [`Input`]s in, [`Output`]s out.
//!
//! One language server, many clients. The mux owns everything the server must see exactly once:
//! the `initialize` exchange (with the daemon's policy, not a client's), the `initialized`
//! notification, one open document per file, and the server's lifetime (a client's `shutdown`
//! and `exit` detach only that client). It remaps request ids per client, answers the
//! server→client requests it can answer from the daemon's own state and refuses the rest, and
//! turns every `publishDiagnostics` into a per-session view through [`crate::attrib`] and
//! [`crate::deliver`].
//!
//! Disk is the authority on document text. Agents edit through their own tools, so the file on
//! disk is what every session shares; a client's editor buffer is its private view. The server
//! is therefore only ever sent disk content: the first `didOpen` carries the file as read from
//! disk, and every `didChange` or `didSave` re-reads the file and, when it differs from what the
//! server holds, sends it as a full change. A client's buffer reaches the server only for a
//! document with no file behind it. Document versions are the mux's own per-document counter.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::attrib::{Attributor, DiagnosticSource, EditEvent, EditSource, SessionKey};
use crate::deliver::{
    changes, publish_notification, render, session_view, unstamp, Area, AreaFs, AreaResolver,
    OwnedDiag, SessionEntry, Snapshot,
};
use crate::uri::{path_key, path_to_uri, uri_to_path};

/// LSP/JSON-RPC error codes the mux answers with.
const SERVER_NOT_INITIALIZED: i64 = -32002;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;

/// What the mux needs from the daemon.
#[derive(Debug, Clone)]
pub struct MuxConfig {
    /// The checkout root the server analyses.
    pub root: PathBuf,
    /// The daemon's `initializationOptions` (C5 policy), sent instead of any client's.
    pub initialization_options: Value,
    /// The configuration section the server asks for (`rust-analyzer`), answered from the
    /// same options.
    pub config_section: Option<String>,
    /// The daemon's pid, sent as `processId` so the server watches the daemon, not a client.
    pub mux_pid: u32,
    /// The attribution window (C5 `attribution_window_ms`).
    pub window_ms: i64,
    /// The server's language, for the snapshot.
    pub lang: String,
}

/// One attached client connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientId(pub u64);

/// An event the daemon feeds the mux.
#[derive(Debug, Clone)]
pub enum Input {
    /// A client connected and said hello.
    Attach {
        /// Its connection.
        client: ClientId,
        /// Its session.
        session: SessionKey,
    },
    /// A message from a client.
    FromClient {
        /// The sender.
        client: ClientId,
        /// The message.
        message: Value,
    },
    /// A client's connection ended.
    Detach {
        /// The client.
        client: ClientId,
    },
    /// A message from the server.
    FromServer(Value),
    /// Edit events read from the C2 log.
    Edits(Vec<EditEvent>),
    /// Diagnostics from a non-LSP publisher (e.g. a gate tail), attributed and folded like the
    /// server's, kept as their own set per URI and tagged with `source` on delivery.
    External {
        /// The publisher's tag.
        source: String,
        /// The URI.
        uri: String,
        /// The publisher's full current set for the URI.
        diagnostics: Vec<Value>,
    },
}

/// An effect the daemon performs for the mux.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// Send to the server.
    ToServer(Value),
    /// Send to one client.
    ToClient(ClientId, Value),
    /// Close one client's connection after its queued messages.
    Close(ClientId),
    /// A line for the daemon log.
    Log(String),
    /// The attribution snapshot changed and should be rewritten.
    SnapshotChanged,
}

#[derive(Debug)]
struct Client {
    session: SessionKey,
    ready: bool,
    shut_down: bool,
    sent: BTreeMap<String, Vec<Value>>,
    first_open: Option<String>,
}

#[derive(Debug)]
struct Doc {
    server_uri: String,
    openers: BTreeSet<ClientId>,
    version: i64,
    text_hash: u64,
}

#[derive(Debug)]
struct Pending {
    client: Option<ClientId>,
    id: Value,
}

#[derive(Debug)]
enum Init {
    Idle,
    Pending {
        server_id: u64,
        waiters: Vec<(ClientId, Value)>,
    },
    Ready(Value),
}

/// The multiplexer state.
#[derive(Debug)]
pub struct Mux<F> {
    config: MuxConfig,
    root_uri: String,
    clients: BTreeMap<ClientId, Client>,
    init: Init,
    initialized_sent: bool,
    next_server_id: u64,
    to_server: HashMap<u64, Pending>,
    by_client: HashMap<(ClientId, String), u64>,
    next_broadcast: u64,
    docs: HashMap<String, Doc>,
    attrib: Attributor,
    areas: AreaResolver<F>,
    area_of: HashMap<String, Area>,
    registrations: Vec<Value>,
    own_requests: HashSet<u64>,
}

enum Kind {
    Request(Value, String),
    Notification(String),
    Response(Value),
    Invalid,
}

fn classify(message: &Value) -> Kind {
    let method = message.get("method").and_then(Value::as_str);
    let id = message.get("id").filter(|id| !id.is_null());
    match (method, id) {
        (Some(method), Some(id)) => Kind::Request(id.clone(), method.to_owned()),
        (Some(method), None) => Kind::Notification(method.to_owned()),
        (None, Some(id)) => Kind::Response(id.clone()),
        (None, None) => Kind::Invalid,
    }
}

/// The table key of a JSON-RPC id: its JSON text, so `1` and `"1"` stay distinct.
fn id_key(id: &Value) -> String {
    id.to_string()
}

fn response(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn notification(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

fn request(id: Value, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

/// A `window/showMessage` notification of `kind` (1 error, 2 warning, 3 info, 4 log).
#[must_use]
pub fn show_message(kind: u8, text: &str) -> Value {
    notification("window/showMessage", json!({"type": kind, "message": text}))
}

/// An Information diagnostic at the top of a file, carrying a notice from epr lsp itself.
#[must_use]
pub fn notice_diagnostic(text: &str) -> Value {
    json!({
        "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
        "severity": 3,
        "source": "epr-lsp",
        "message": text
    })
}

fn text_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

impl<F: AreaFs> Mux<F> {
    /// A mux for one server over `root`, resolving areas and reading documents through `fs`.
    pub fn new(config: MuxConfig, fs: F) -> Mux<F> {
        let root_uri = path_to_uri(&config.root);
        let areas = AreaResolver::new(fs, config.root.clone());
        let attrib = Attributor::new(config.window_ms);
        Mux {
            config,
            root_uri,
            clients: BTreeMap::new(),
            init: Init::Idle,
            initialized_sent: false,
            next_server_id: 1,
            to_server: HashMap::new(),
            by_client: HashMap::new(),
            next_broadcast: 1,
            docs: HashMap::new(),
            attrib,
            areas,
            area_of: HashMap::new(),
            registrations: Vec::new(),
            own_requests: HashSet::new(),
        }
    }

    /// How many clients are attached.
    #[must_use]
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Whether the server has answered `initialize`.
    #[must_use]
    pub fn initialized(&self) -> bool {
        matches!(self.init, Init::Ready(_))
    }

    /// The mux's own `shutdown` request (sent only at idle reap or a pin change), and its id.
    pub fn shutdown_request(&mut self) -> (u64, Value) {
        let id = self.server_id();
        self.own_requests.insert(id);
        (id, request(json!(id), "shutdown", Value::Null))
    }

    /// The `exit` notification that follows the mux's own `shutdown`.
    #[must_use]
    pub fn exit_notification() -> Value {
        json!({"jsonrpc": "2.0", "method": "exit"})
    }

    /// Feed one event at `now_ms` (ms since the Unix epoch).
    pub fn handle(&mut self, input: Input, now_ms: i64) -> Vec<Output> {
        let mut out = Vec::new();
        match input {
            Input::Attach { client, session } => {
                out.push(Output::Log(format!(
                    "attach client {} session {}",
                    client.0, session.session
                )));
                self.clients.insert(
                    client,
                    Client {
                        session,
                        ready: false,
                        shut_down: false,
                        sent: BTreeMap::new(),
                        first_open: None,
                    },
                );
                out.push(Output::SnapshotChanged);
            }
            Input::Detach { client } => self.detach(client, &mut out),
            Input::FromClient { client, message } => {
                self.on_client(client, message, now_ms, &mut out);
            }
            Input::FromServer(message) => self.on_server(message, now_ms, &mut out),
            Input::External {
                source,
                uri,
                diagnostics,
            } => self.publish_set(
                DiagnosticSource::External(&source),
                &uri,
                diagnostics,
                now_ms,
                &mut out,
            ),
            Input::Edits(events) => {
                let root = self.config.root.to_string_lossy().into_owned();
                let mut grew = false;
                for event in events {
                    // The C2 log is shared by every checkout of the repository; an edit in a
                    // sibling worktree says nothing about this root's diagnostics.
                    let here = event
                        .path
                        .as_deref()
                        .is_none_or(|path| std::path::Path::new(path).starts_with(&root));
                    if here {
                        grew |= self.attrib.record_edit(event);
                    }
                }
                if grew {
                    self.redeliver(&mut out);
                    out.push(Output::SnapshotChanged);
                }
            }
        }
        out
    }

    fn server_id(&mut self) -> u64 {
        let id = self.next_server_id;
        self.next_server_id += 1;
        id
    }

    fn broadcast_id(&mut self) -> Value {
        let id = self.next_broadcast;
        self.next_broadcast += 1;
        Value::String(format!("epr-mux-b:{id}"))
    }

    // ---- client → server ----------------------------------------------------------------

    fn on_client(&mut self, client: ClientId, message: Value, now_ms: i64, out: &mut Vec<Output>) {
        if !self.clients.contains_key(&client) {
            return;
        }
        match classify(&message) {
            Kind::Request(id, method) => self.client_request(client, id, &method, message, out),
            Kind::Notification(method) => {
                self.client_notification(client, &method, message, now_ms, out);
            }
            // The mux routes no server request to a client, so a client's response can only
            // answer one of the mux's own broadcasts; it is swallowed.
            Kind::Response(_) => {}
            Kind::Invalid => out.push(Output::Log(format!(
                "client {} sent a message that is neither request, notification nor response",
                client.0
            ))),
        }
    }

    fn client_request(
        &mut self,
        client: ClientId,
        id: Value,
        method: &str,
        mut message: Value,
        out: &mut Vec<Output>,
    ) {
        if method == "initialize" {
            self.client_initialize(client, id, &message, out);
            return;
        }
        let Some(state) = self.clients.get_mut(&client) else {
            return;
        };
        if method == "shutdown" {
            // The server outlives this client: only the client's view ends.
            state.shut_down = true;
            out.push(Output::ToClient(client, response(id, Value::Null)));
            return;
        }
        if !state.ready {
            out.push(Output::ToClient(
                client,
                error_response(id, SERVER_NOT_INITIALIZED, "initialize first"),
            ));
            return;
        }
        if state.shut_down {
            out.push(Output::ToClient(
                client,
                error_response(id, INVALID_REQUEST, "shutdown was already requested"),
            ));
            return;
        }
        if method == "textDocument/codeAction" {
            if let Some(list) = message
                .pointer_mut("/params/context/diagnostics")
                .and_then(Value::as_array_mut)
            {
                *list = list.iter().filter_map(unstamp).collect();
            }
        }
        self.rewrite_document_uri(&mut message);
        let server_id = self.server_id();
        self.by_client.insert((client, id_key(&id)), server_id);
        self.to_server.insert(
            server_id,
            Pending {
                client: Some(client),
                id,
            },
        );
        message["id"] = json!(server_id);
        out.push(Output::ToServer(message));
    }

    fn client_initialize(
        &mut self,
        client: ClientId,
        id: Value,
        message: &Value,
        out: &mut Vec<Output>,
    ) {
        match &mut self.init {
            Init::Ready(result) => {
                let result = result.clone();
                out.push(Output::ToClient(client, response(id, result)));
                if let Some(state) = self.clients.get_mut(&client) {
                    state.ready = true;
                }
                self.redeliver(out);
            }
            Init::Pending { waiters, .. } => waiters.push((client, id)),
            Init::Idle => {
                let server_id = self.server_id();
                let params = self.initialize_params(message.get("params").cloned());
                out.push(Output::ToServer(request(
                    json!(server_id),
                    "initialize",
                    params,
                )));
                self.init = Init::Pending {
                    server_id,
                    waiters: vec![(client, id)],
                };
            }
        }
    }

    /// The first client's `initialize` params, made the daemon's: its own pid, the root, the
    /// policy's options; position encoding left to the protocol default every client speaks;
    /// diagnostics pushed (never pulled) so every one passes through attribution.
    fn initialize_params(&self, params: Option<Value>) -> Value {
        let mut params = match params {
            Some(Value::Object(map)) => Value::Object(map),
            _ => json!({}),
        };
        let name = self
            .config
            .root
            .file_name()
            .map_or_else(|| "root".to_owned(), |n| n.to_string_lossy().into_owned());
        params["processId"] = json!(self.config.mux_pid);
        params["rootUri"] = json!(self.root_uri);
        params["rootPath"] = json!(self.config.root.to_string_lossy());
        params["workspaceFolders"] = json!([{"uri": self.root_uri, "name": name}]);
        params["initializationOptions"] = self.config.initialization_options.clone();
        if let Some(general) = params
            .pointer_mut("/capabilities/general")
            .and_then(Value::as_object_mut)
        {
            general.remove("positionEncodings");
        }
        if let Some(document) = params
            .pointer_mut("/capabilities/textDocument")
            .and_then(Value::as_object_mut)
        {
            document.remove("diagnostic");
        }
        params
    }

    fn client_notification(
        &mut self,
        client: ClientId,
        method: &str,
        message: Value,
        now_ms: i64,
        out: &mut Vec<Output>,
    ) {
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match method {
            "initialized" => {
                if !self.initialized_sent && self.initialized() {
                    self.initialized_sent = true;
                    out.push(Output::ToServer(message));
                } else if !self.registrations.is_empty() {
                    let id = self.broadcast_id();
                    let registrations = self.registrations.clone();
                    out.push(Output::ToClient(
                        client,
                        request(
                            id,
                            "client/registerCapability",
                            json!({"registrations": registrations}),
                        ),
                    ));
                }
            }
            "exit" => {
                out.push(Output::Close(client));
                self.detach(client, out);
            }
            "$/cancelRequest" => {
                let Some(id) = params.get("id") else { return };
                if let Some(server_id) = self.by_client.get(&(client, id_key(id))) {
                    out.push(Output::ToServer(notification(
                        "$/cancelRequest",
                        json!({"id": server_id}),
                    )));
                }
            }
            "textDocument/didOpen" => self.did_open(client, &params, out),
            "textDocument/didChange" => self.did_change(client, &params, now_ms, out),
            "textDocument/didSave" => self.did_save(client, message, now_ms, out),
            "textDocument/didClose" => self.did_close(client, &params, out),
            "workspace/didChangeConfiguration" => out.push(Output::Log(format!(
                "client {} configuration change ignored: the daemon owns the server's settings",
                client.0
            ))),
            _ => out.push(Output::ToServer(message)),
        }
    }

    /// The file behind a URI as it is on disk now, if there is one.
    fn disk_text(&self, uri: &str) -> Option<String> {
        uri_to_path(uri).and_then(|path| self.areas.document(&path))
    }

    fn did_open(&mut self, client: ClientId, params: &Value, out: &mut Vec<Output>) {
        let Some(uri) = params.pointer("/textDocument/uri").and_then(Value::as_str) else {
            return;
        };
        if let Some(state) = self.clients.get_mut(&client) {
            state.first_open.get_or_insert_with(|| uri.to_owned());
        }
        let key = path_key(uri);
        let disk = self.disk_text(uri);
        if let Some(doc) = self.docs.get_mut(&key) {
            doc.openers.insert(client);
            // Already open: the server keeps one document, and it is the disk's. A client
            // opening it says nothing new unless the disk moved since; that re-sync is no edit.
            if let Some(text) = disk {
                if text_hash(&text) != doc.text_hash {
                    out.push(Output::ToServer(sync(doc, &text)));
                }
            }
        } else {
            // The server's text is the file on disk; a client's buffer stands in only for a
            // document with no file behind it.
            let text = disk.unwrap_or_else(|| {
                params
                    .pointer("/textDocument/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            });
            let version = params
                .pointer("/textDocument/version")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let mut forwarded = params.clone();
            forwarded["textDocument"]["text"] = json!(text);
            self.docs.insert(
                key,
                Doc {
                    server_uri: uri.to_owned(),
                    openers: BTreeSet::from([client]),
                    version,
                    text_hash: text_hash(&text),
                },
            );
            out.push(Output::ToServer(notification(
                "textDocument/didOpen",
                forwarded,
            )));
        }
        self.redeliver(out);
        out.push(Output::SnapshotChanged);
    }

    /// A change is a signal to re-read the disk, never a text to forward: when the file moved
    /// since the server's copy, the server gets the file, and the change is an edit by its
    /// sender (unless the C2 log already names that file's editor, see [`Self::stamp_edit`]).
    fn did_change(&mut self, client: ClientId, params: &Value, now_ms: i64, out: &mut Vec<Output>) {
        let Some(uri) = params.pointer("/textDocument/uri").and_then(Value::as_str) else {
            return;
        };
        let key = path_key(uri);
        let opened = self
            .docs
            .get(&key)
            .is_some_and(|d| d.openers.contains(&client));
        if !opened {
            out.push(Output::Log(format!(
                "client {} changed {uri} without opening it; dropped",
                client.0
            )));
            return;
        }
        let Some(text) = self.disk_text(uri) else {
            out.push(Output::Log(format!(
                "client {} changed {uri}, which has no file on disk; a client's buffer is never \
                 the shared text, so the server keeps its copy",
                client.0
            )));
            return;
        };
        if self.sync_from_disk(&key, &text, out) {
            self.stamp_edit(client, key, EditSource::DidChange, now_ms, out);
        }
    }

    fn did_save(
        &mut self,
        client: ClientId,
        mut message: Value,
        now_ms: i64,
        out: &mut Vec<Output>,
    ) {
        let Some(uri) = message
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            return;
        };
        let key = path_key(&uri);
        if self.docs.contains_key(&key) {
            if let Some(text) = self.disk_text(&uri) {
                self.sync_from_disk(&key, &text, out);
            }
        }
        // A saved buffer's text is the client's; the server already has the disk's.
        if let Some(params) = message.get_mut("params").and_then(Value::as_object_mut) {
            params.remove("text");
        }
        self.rewrite_document_uri(&mut message);
        out.push(Output::ToServer(message));
        self.stamp_edit(client, key, EditSource::DidSave, now_ms, out);
    }

    /// Send the disk's `text` as a full change when it differs from the server's copy.
    fn sync_from_disk(&mut self, key: &str, text: &str, out: &mut Vec<Output>) -> bool {
        let Some(doc) = self.docs.get_mut(key) else {
            return false;
        };
        if text_hash(text) == doc.text_hash {
            return false;
        }
        out.push(Output::ToServer(sync(doc, text)));
        true
    }

    fn did_close(&mut self, client: ClientId, params: &Value, out: &mut Vec<Output>) {
        let Some(uri) = params.pointer("/textDocument/uri").and_then(Value::as_str) else {
            return;
        };
        self.release_doc(client, &path_key(uri), out);
        self.redeliver(out);
        out.push(Output::SnapshotChanged);
    }

    fn release_doc(&mut self, client: ClientId, key: &str, out: &mut Vec<Output>) {
        let Some(doc) = self.docs.get_mut(key) else {
            return;
        };
        doc.openers.remove(&client);
        if doc.openers.is_empty() {
            out.push(Output::ToServer(notification(
                "textDocument/didClose",
                json!({"textDocument": {"uri": doc.server_uri}}),
            )));
            self.docs.remove(key);
        }
    }

    /// Point a request or notification at the URI the server opened the document under, when a
    /// client encodes the same file differently.
    fn rewrite_document_uri(&self, message: &mut Value) {
        let Some(uri) = message
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return;
        };
        if let Some(doc) = self.docs.get(&path_key(uri)) {
            if doc.server_uri != uri {
                message["params"]["textDocument"]["uri"] = json!(doc.server_uri);
            }
        }
    }

    /// Record an LSP-observed edit by `client`'s session. The C2 log is the authority on who
    /// edited a file on disk: when it already names an edit of this file inside the window,
    /// the LSP event adds nothing (another session's client merely reporting the new disk text
    /// must not become a second owner). LSP events stand in for editors that write no C2 line.
    fn stamp_edit(
        &mut self,
        client: ClientId,
        key: String,
        source: EditSource,
        now_ms: i64,
        out: &mut Vec<Output>,
    ) {
        let Some(session) = self.clients.get(&client).map(|c| c.session.clone()) else {
            return;
        };
        if self.attrib.log_names_edit_of(&key, now_ms) {
            return;
        }
        let grew = self.attrib.record_edit(EditEvent {
            at_ms: now_ms,
            session,
            path: Some(key),
            source,
        });
        if grew {
            self.redeliver(out);
            out.push(Output::SnapshotChanged);
        }
    }

    // ---- server → client ----------------------------------------------------------------

    fn on_server(&mut self, message: Value, now_ms: i64, out: &mut Vec<Output>) {
        match classify(&message) {
            Kind::Response(id) => self.server_response(&id, message, out),
            Kind::Request(id, method) => self.server_request(id, &method, message, out),
            Kind::Notification(method) => match method.as_str() {
                "textDocument/publishDiagnostics" => self.publish(&message, now_ms, out),
                // Progress tokens are created by the mux itself, never by a client, so no
                // client knows them.
                "$/progress" => {}
                _ => {
                    for client in self.ready_clients() {
                        out.push(Output::ToClient(client, message.clone()));
                    }
                }
            },
            Kind::Invalid => out.push(Output::Log("server sent an invalid message".into())),
        }
    }

    fn server_response(&mut self, id: &Value, message: Value, out: &mut Vec<Output>) {
        let numeric = id.as_u64();
        if let Init::Pending { server_id, .. } = &self.init {
            if numeric == Some(*server_id) {
                self.initialize_response(message, out);
                return;
            }
        }
        let Some(numeric) = numeric else { return };
        if self.own_requests.remove(&numeric) {
            return;
        }
        let Some(pending) = self.to_server.remove(&numeric) else {
            return;
        };
        let Some(client) = pending.client.filter(|c| self.clients.contains_key(c)) else {
            return;
        };
        self.by_client.remove(&(client, id_key(&pending.id)));
        let mut message = message;
        message["id"] = pending.id;
        out.push(Output::ToClient(client, message));
    }

    fn initialize_response(&mut self, message: Value, out: &mut Vec<Output>) {
        let Init::Pending { waiters, .. } = std::mem::replace(&mut self.init, Init::Idle) else {
            return;
        };
        let Some(mut result) = message.get("result").cloned() else {
            // The server refused: every waiter hears it, and the next initialize retries.
            let error = message.get("error").cloned().unwrap_or(Value::Null);
            for (client, id) in waiters {
                out.push(Output::ToClient(
                    client,
                    json!({"jsonrpc": "2.0", "id": id, "error": error}),
                ));
            }
            return;
        };
        // Clients send whole text on change; the mux never forwards it, but a range change it
        // could not interpret against the disk would be worse than a full one it ignores.
        match result.pointer_mut("/capabilities/textDocumentSync") {
            Some(Value::Object(sync)) => {
                sync.insert("change".into(), json!(1));
            }
            Some(sync) => *sync = json!(1),
            None => {}
        }
        self.init = Init::Ready(result.clone());
        for (client, id) in waiters {
            if let Some(state) = self.clients.get_mut(&client) {
                state.ready = true;
                out.push(Output::ToClient(client, response(id, result.clone())));
            }
        }
        self.redeliver(out);
    }

    /// The server's requests are answered from the daemon's own state or refused; none is
    /// routed to a client. Agents act through their own tools, and a request to one client of
    /// many has no right recipient.
    fn server_request(&mut self, id: Value, method: &str, message: Value, out: &mut Vec<Output>) {
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match method {
            // The daemon owns the server's settings: a client's answer would override policy.
            "workspace/configuration" => {
                let items = params
                    .get("items")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let answers: Vec<Value> = items
                    .iter()
                    .map(|item| self.config_value(item.get("section").and_then(Value::as_str)))
                    .collect();
                out.push(Output::ToServer(response(id, Value::Array(answers))));
            }
            "workspace/workspaceFolders" => {
                let name = self
                    .config
                    .root
                    .file_name()
                    .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned());
                out.push(Output::ToServer(response(
                    id,
                    json!([{"uri": self.root_uri, "name": name}]),
                )));
            }
            // Progress is shown nowhere: the token is accepted and its reports dropped.
            "window/workDoneProgress/create" => {
                out.push(Output::ToServer(response(id, Value::Null)));
            }
            // A registration belongs to the server, not to one client: it is accepted here and
            // every client hears it, including clients that attach later, so file watching
            // survives any detach.
            "client/registerCapability" => {
                if let Some(list) = params.get("registrations").and_then(Value::as_array) {
                    self.registrations.extend(list.iter().cloned());
                }
                out.push(Output::ToServer(response(id, Value::Null)));
                self.broadcast_request(method, &params, out);
            }
            "client/unregisterCapability" => {
                let gone: HashSet<String> = params
                    .get("unregisterations")
                    .and_then(Value::as_array)
                    .map(|list| {
                        list.iter()
                            .filter_map(|u| u.get("id").and_then(Value::as_str))
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                self.registrations.retain(|r| {
                    r.get("id")
                        .and_then(Value::as_str)
                        .is_none_or(|rid| !gone.contains(rid))
                });
                out.push(Output::ToServer(response(id, Value::Null)));
                self.broadcast_request(method, &params, out);
            }
            // A refresh asks clients to re-pull what they show; it is acknowledged here and
            // passed to every client.
            refresh if refresh.ends_with("/refresh") => {
                out.push(Output::ToServer(response(id, Value::Null)));
                self.broadcast_request(method, &params, out);
            }
            // Agents edit through their own tools; the shared server applies nothing.
            "workspace/applyEdit" => out.push(Output::ToServer(response(
                id,
                json!({
                    "applied": false,
                    "failureReason": "epr lsp: edits are made through each session's own tools"
                }),
            ))),
            other => out.push(Output::ToServer(error_response(
                id,
                METHOD_NOT_FOUND,
                &format!("epr lsp: the shared server does not route {other} to a client"),
            ))),
        }
    }

    fn config_value(&self, section: Option<&str>) -> Value {
        let options = &self.config.initialization_options;
        match (section, self.config.config_section.as_deref()) {
            (None, _) => options.clone(),
            (Some(asked), Some(own)) if asked == own => options.clone(),
            (Some(asked), Some(own)) => asked
                .strip_prefix(own)
                .and_then(|rest| rest.strip_prefix('.'))
                .and_then(|path| {
                    path.split('.')
                        .try_fold(options, |value, key| value.get(key))
                })
                .cloned()
                .unwrap_or(Value::Null),
            (Some(_), None) => Value::Null,
        }
    }

    fn broadcast_request(&mut self, method: &str, params: &Value, out: &mut Vec<Output>) {
        for client in self.ready_clients() {
            let id = self.broadcast_id();
            out.push(Output::ToClient(
                client,
                request(id, method, params.clone()),
            ));
        }
    }

    fn ready_clients(&self) -> Vec<ClientId> {
        self.clients
            .iter()
            .filter(|(_, c)| c.ready)
            .map(|(id, _)| *id)
            .collect()
    }

    fn publish(&mut self, message: &Value, now_ms: i64, out: &mut Vec<Output>) {
        let Some(uri) = message.pointer("/params/uri").and_then(Value::as_str) else {
            return;
        };
        let diagnostics = message
            .pointer("/params/diagnostics")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        self.publish_set(DiagnosticSource::Server, uri, diagnostics, now_ms, out);
    }

    fn publish_set(
        &mut self,
        source: DiagnosticSource<'_>,
        uri: &str,
        diagnostics: Vec<Value>,
        now_ms: i64,
        out: &mut Vec<Output>,
    ) {
        if !self.area_of.contains_key(uri) {
            let area = self.areas.area(uri);
            self.area_of.insert(uri.to_owned(), area);
        }
        self.attrib.publish(source, uri, diagnostics, now_ms);
        self.redeliver(out);
        out.push(Output::SnapshotChanged);
    }

    // ---- delivery -----------------------------------------------------------------------

    fn owned(&self) -> Vec<OwnedDiag> {
        let mut owned = Vec::new();
        for ((source, uri), list) in self.attrib.current() {
            let Some(area) = self.area_of.get(uri) else {
                continue;
            };
            for tracked in list {
                owned.push(OwnedDiag {
                    uri: uri.clone(),
                    area: area.clone(),
                    diagnostic: tracked.diagnostic.clone(),
                    attribution: tracked.attribution.clone(),
                    source: source.clone(),
                });
            }
        }
        owned.sort_by(|a, b| (&a.uri, &a.source).cmp(&(&b.uri, &b.source)));
        owned
    }

    /// Every path a session opened (through any of its clients) or edited.
    fn touched(&self, session: &str) -> BTreeSet<String> {
        let mut touched: BTreeSet<String> =
            self.attrib.touched(session).cloned().unwrap_or_default();
        for (key, doc) in &self.docs {
            let opened = doc.openers.iter().any(|id| {
                self.clients
                    .get(id)
                    .is_some_and(|c| c.session.same_session(session))
            });
            if opened {
                touched.insert(key.clone());
            }
        }
        touched
    }

    fn redeliver(&mut self, out: &mut Vec<Output>) {
        let owned = self.owned();
        let ready: Vec<(ClientId, String)> = self
            .clients
            .iter()
            .filter(|(_, c)| c.ready)
            .map(|(id, c)| (*id, c.session.session.clone()))
            .collect();
        for (client, session) in ready {
            let touched = self.touched(&session);
            let next = render(&session_view(&owned, &session, &touched));
            let Some(state) = self.clients.get_mut(&client) else {
                continue;
            };
            for (uri, list) in changes(&state.sent, &next) {
                out.push(Output::ToClient(client, publish_notification(&uri, list)));
            }
            state.sent = next;
        }
    }

    // ---- detach -------------------------------------------------------------------------

    fn detach(&mut self, client: ClientId, out: &mut Vec<Output>) {
        let Some(state) = self.clients.remove(&client) else {
            return;
        };
        out.push(Output::Log(format!(
            "detach client {} session {}",
            client.0, state.session.session
        )));
        let held: Vec<String> = self
            .docs
            .iter()
            .filter(|(_, doc)| doc.openers.contains(&client))
            .map(|(key, _)| key.clone())
            .collect();
        for key in held {
            self.release_doc(client, &key, out);
        }
        for (server_id, pending) in &mut self.to_server {
            if pending.client == Some(client) {
                pending.client = None;
                out.push(Output::ToServer(notification(
                    "$/cancelRequest",
                    json!({"id": server_id}),
                )));
            }
        }
        self.by_client.retain(|(owner, _), _| *owner != client);
        if let Init::Pending { waiters, .. } = &mut self.init {
            waiters.retain(|(waiter, _)| *waiter != client);
        }
        self.redeliver(out);
        out.push(Output::SnapshotChanged);
    }

    /// What every client hears when the server child dies (contract C4 amendment): a
    /// `window/showMessage` and the same reason as an Information diagnostic, because a client
    /// may not surface a showMessage at all.
    #[must_use]
    pub fn crash_notices(&self, reason: &str) -> Vec<Output> {
        self.notices(
            1,
            &format!(
                "epr lsp: the shared {} server exited ({reason}); diagnostics stop until this \
                 session's language server restarts",
                self.config.lang
            ),
        )
    }

    /// What this root's clients hear when the policy's pin moves under a running server.
    #[must_use]
    pub fn pin_notices(&self, old: &str, new: &str) -> Vec<Output> {
        self.notices(
            3,
            &format!("toolchain pin changed {old} → {new}; restarting"),
        )
    }

    /// A showMessage of `kind` and an Information diagnostic carrying `text`, for every client,
    /// published on the first URI it opened or else on `<root>/Cargo.toml` (its anchor).
    fn notices(&self, kind: u8, text: &str) -> Vec<Output> {
        let fallback = path_to_uri(&self.config.root.join("Cargo.toml"));
        let mut out = Vec::new();
        for (client, state) in &self.clients {
            out.push(Output::ToClient(*client, show_message(kind, text)));
            let uri = state.first_open.clone().unwrap_or_else(|| fallback.clone());
            out.push(Output::ToClient(
                *client,
                publish_notification(&uri, vec![notice_diagnostic(text)]),
            ));
        }
        out
    }

    /// The attribution snapshot (C3) at `now_ms`.
    #[must_use]
    pub fn snapshot(&self, now_ms: i64) -> Snapshot {
        let mut sessions: BTreeMap<String, SessionEntry> = BTreeMap::new();
        for client in self.clients.values() {
            sessions
                .entry(client.session.session.clone())
                .or_insert_with(|| SessionEntry {
                    key: client.session.clone(),
                    attached: true,
                    touched: Vec::new(),
                });
        }
        for (session, _) in self.attrib.touched_all() {
            sessions
                .entry(session.clone())
                .or_insert_with(|| SessionEntry {
                    key: SessionKey::named(session.clone()),
                    attached: false,
                    touched: Vec::new(),
                });
        }
        for entry in sessions.values_mut() {
            entry.touched = self.touched(&entry.key.session).into_iter().collect();
        }
        Snapshot {
            v: 1,
            lang: self.config.lang.clone(),
            root: self.config.root.to_string_lossy().into_owned(),
            at: crate::attrib::rfc3339(now_ms),
            window_ms: self.config.window_ms,
            sessions: sessions.into_values().collect(),
            diagnostics: self.owned(),
        }
    }
}

/// A full change carrying `text` as the document's next version.
fn sync(doc: &mut Doc, text: &str) -> Value {
    doc.version += 1;
    doc.text_hash = text_hash(text);
    notification(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": doc.server_uri, "version": doc.version},
            "contentChanges": [{"text": text}]
        }),
    )
}

#[cfg(test)]
mod tests;
