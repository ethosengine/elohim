//! Per-session delivery (contract C7).
//!
//! A session receives in full every diagnostic on a URI it opened or edited, plus every
//! diagnostic attributed to it wherever that lands. Everything else folds into ONE synthetic
//! diagnostic per (area, owner), published on the area's anchor URI. The same pure view feeds
//! the mux's publishes and `epr lsp show`, so the two cannot disagree.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::attrib::{short8, Attribution, SessionKey};
use crate::uri::{path_key, path_to_uri, uri_to_path};

/// The unit diagnostics fold into: for rust, the cargo package directory.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Area {
    /// The area's directory (absolute).
    pub dir: String,
    /// How a fold names it: the directory relative to the server root.
    pub name: String,
    /// The URI folds are published on: `<area>/Cargo.toml` for rust.
    pub anchor: String,
}

/// The filesystem facts the mux needs, behind a trait so it can be tested: package manifests
/// for areas, and documents as they are on disk (the disk is the authority on document text).
pub trait AreaFs {
    /// The text of `dir/Cargo.toml`, if it exists.
    fn manifest(&self, dir: &Path) -> Option<String>;
    /// The text of the file at `path` as it is on disk now, if it is readable UTF-8.
    fn document(&self, path: &Path) -> Option<String>;
}

/// The real filesystem.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealFs;

impl AreaFs for RealFs {
    fn manifest(&self, dir: &Path) -> Option<String> {
        std::fs::read_to_string(dir.join("Cargo.toml")).ok()
    }

    fn document(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

/// Resolves a file to its area and caches which directories hold a package.
#[derive(Debug)]
pub struct AreaResolver<F> {
    fs: F,
    root: PathBuf,
    packages: HashMap<PathBuf, bool>,
    root_manifest: Option<bool>,
}

impl<F: AreaFs> AreaResolver<F> {
    /// A resolver for files under `root`.
    pub fn new(fs: F, root: PathBuf) -> AreaResolver<F> {
        AreaResolver {
            fs,
            root,
            packages: HashMap::new(),
            root_manifest: None,
        }
    }

    /// The nearest ancestor directory whose `Cargo.toml` declares `[package]`, anchored on that
    /// manifest. A file outside every package, or not a file URI, is its own area and anchor.
    pub fn area(&mut self, uri: &str) -> Area {
        let Some(path) = uri_to_path(uri) else {
            return self.own_area(uri);
        };
        // Never look above the root for a file under it: a manifest outside the checkout is not
        // this server's business.
        let under_root = path.starts_with(&self.root);
        for dir in path.ancestors().skip(1) {
            if under_root && !dir.starts_with(&self.root) {
                break;
            }
            let is_package = match self.packages.get(dir) {
                Some(known) => *known,
                None => {
                    let found = self
                        .fs
                        .manifest(dir)
                        .is_some_and(|text| declares_package(&text));
                    self.packages.insert(dir.to_path_buf(), found);
                    found
                }
            };
            if is_package {
                return Area {
                    dir: dir.to_string_lossy().into_owned(),
                    name: self.display(dir),
                    anchor: path_to_uri(&dir.join("Cargo.toml")),
                };
            }
        }
        // Outside every package but inside the checkout: the root is the area, anchored on its
        // manifest when it has one (a workspace manifest counts).
        if under_root {
            let root_manifest = *self
                .root_manifest
                .get_or_insert_with(|| self.fs.manifest(&self.root).is_some());
            if root_manifest {
                return Area {
                    dir: self.root.to_string_lossy().into_owned(),
                    name: self.display(&self.root),
                    anchor: path_to_uri(&self.root.join("Cargo.toml")),
                };
            }
        }
        self.own_area(uri)
    }

    /// The document at `path` as it is on disk now.
    pub fn document(&self, path: &Path) -> Option<String> {
        self.fs.document(path)
    }

    fn own_area(&self, uri: &str) -> Area {
        Area {
            dir: path_key(uri),
            name: uri_to_path(uri).map_or_else(|| uri.to_owned(), |p| self.display(&p)),
            anchor: uri.to_owned(),
        }
    }

    fn display(&self, path: &Path) -> String {
        match path.strip_prefix(&self.root) {
            Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().into_owned(),
            _ => path.file_name().map_or_else(
                || path.to_string_lossy().into_owned(),
                |n| n.to_string_lossy().into_owned(),
            ),
        }
    }
}

fn declares_package(manifest: &str) -> bool {
    manifest.lines().any(|line| line.trim() == "[package]")
}

/// One live diagnostic with its URI, area and owner: the unit of delivery and of the snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnedDiag {
    /// The URI the server published it on.
    pub uri: String,
    /// Its area.
    pub area: Area,
    /// The diagnostic as the server published it.
    pub diagnostic: Value,
    /// Its owner.
    pub attribution: Attribution,
    /// The non-LSP publisher's tag; absent for the language server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// How a delivered diagnostic relates to the receiving session (`data.epr.attribution`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Label {
    /// Owned by the receiver, on a URI the receiver opened or edited.
    Mine,
    /// Owned by the receiver, landing on a URI it never touched.
    CausedByMe,
    /// Owned by one other session.
    Other,
    /// Two or more sessions edited in the window.
    Ambiguous,
    /// No session edited in the window.
    Unattributed,
}

impl Label {
    /// The wire string.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Label::Mine => "mine",
            Label::CausedByMe => "caused-by-me",
            Label::Other => "other",
            Label::Ambiguous => "ambiguous",
            Label::Unattributed => "unattributed",
        }
    }
}

/// A diagnostic delivered in full.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Delivered {
    /// Its URI.
    pub uri: String,
    /// Its relation to the receiver.
    pub label: Label,
    /// The stamped diagnostic, as the receiver sees it.
    pub diagnostic: Value,
}

/// One folded summary: an area's diagnostics from one owner group.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Fold {
    /// The area.
    pub area: Area,
    /// The owners (empty when unattributed).
    pub owners: Vec<String>,
    /// Errors (severity 1, or no severity).
    pub errors: usize,
    /// Warnings (severity 2).
    pub warnings: usize,
}

impl Fold {
    /// The C7 message.
    #[must_use]
    pub fn message(&self) -> String {
        let counts = format!(
            "{}: {} errors, {} warnings",
            self.area.name, self.errors, self.warnings
        );
        if self.owners.is_empty() {
            format!("{counts} (unattributed)")
        } else {
            let shorts: Vec<String> = self.owners.iter().map(|s| short8(s)).collect();
            format!("{counts} from session {} (in flight)", shorts.join(", "))
        }
    }

    /// The synthetic diagnostic published on the anchor.
    #[must_use]
    pub fn diagnostic(&self) -> Value {
        let attribution = match self.owners.len() {
            0 => Label::Unattributed,
            1 => Label::Other,
            _ => Label::Ambiguous,
        };
        json!({
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
            "severity": 3,
            "source": "epr-lsp",
            "message": self.message(),
            "data": {"epr": {
                "attribution": attribution.as_str(),
                "owners": self.owners.iter().map(|s| short8(s)).collect::<Vec<_>>(),
                "fold": true
            }}
        })
    }
}

/// What one session receives.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SessionView {
    /// Full diagnostics on URIs the session opened or edited, whoever owns them.
    pub mine: Vec<Delivered>,
    /// Full diagnostics the session's edits caused on URIs it never touched.
    pub caused_by_me: Vec<Delivered>,
    /// Everything else, folded per (area, owner group).
    pub others: Vec<Fold>,
}

/// Label one diagnostic for `session`.
#[must_use]
pub fn label(attribution: &Attribution, session: &str, touched: bool) -> Label {
    if attribution.owner.is_empty() {
        Label::Unattributed
    } else if attribution.ambiguous {
        Label::Ambiguous
    } else if attribution.names(session) {
        if touched {
            Label::Mine
        } else {
            Label::CausedByMe
        }
    } else {
        Label::Other
    }
}

/// Stamp a diagnostic for its receiver: `data.epr`, and the `[by …] ` prefix when an owner is
/// not the receiver. A server-supplied `data` object keeps its keys; a non-object `data` is left
/// untouched (the prefix still names the owner) so the server's own round-trip data survives.
#[must_use]
pub fn stamp(
    diagnostic: &Value,
    attribution: &Attribution,
    label: Label,
    session: &str,
    source: Option<&str>,
) -> Value {
    let mut stamped = diagnostic.clone();
    let owners: Vec<String> = attribution.owner.iter().map(SessionKey::short8).collect();
    let foreign =
        !attribution.owner.is_empty() && (attribution.ambiguous || !attribution.names(session));
    if foreign {
        if let Some(message) = stamped.get("message").and_then(Value::as_str) {
            let prefixed = format!("[by {}] {message}", owners.join(","));
            stamped["message"] = Value::String(prefixed);
        }
    }
    let mut epr = json!({"attribution": label.as_str(), "owners": owners});
    if let Some(source) = source {
        epr["source"] = json!(source);
    }
    match stamped.get_mut("data") {
        // A publisher's own `data.epr` keys (a gate run's provenance) survive; the stamp's win.
        Some(Value::Object(data)) => match (data.get_mut("epr"), epr) {
            (Some(Value::Object(own)), Value::Object(stamp)) => own.extend(stamp),
            (_, epr) => {
                data.insert("epr".into(), epr);
            }
        },
        None | Some(Value::Null) => {
            stamped["data"] = json!({ "epr": epr });
        }
        Some(_) => {}
    }
    stamped
}

/// Undo [`stamp`] on a diagnostic a client sends back (a code-action context), so the server
/// sees its own diagnostic. `None` for a synthetic fold, which the server never published.
#[must_use]
pub fn unstamp(diagnostic: &Value) -> Option<Value> {
    let epr = diagnostic.get("data").and_then(|d| d.get("epr"));
    let Some(epr) = epr else {
        return Some(diagnostic.clone());
    };
    if epr.get("fold").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let mut plain = diagnostic.clone();
    let owners: Vec<String> = epr
        .get("owners")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let prefix = format!("[by {}] ", owners.join(","));
    if let Some(message) = plain.get("message").and_then(Value::as_str) {
        if let Some(rest) = message.strip_prefix(&prefix) {
            plain["message"] = Value::String(rest.to_owned());
        }
    }
    if let Some(Value::Object(data)) = plain.get_mut("data") {
        data.remove("epr");
        if data.is_empty() {
            if let Some(object) = plain.as_object_mut() {
                object.remove("data");
            }
        }
    }
    Some(plain)
}

fn severity(diagnostic: &Value) -> u64 {
    diagnostic
        .get("severity")
        .and_then(Value::as_u64)
        .unwrap_or(1)
}

/// The view `session` receives, given the paths it opened or edited (as [`path_key`]s).
#[must_use]
pub fn session_view(diags: &[OwnedDiag], session: &str, touched: &BTreeSet<String>) -> SessionView {
    let mut view = SessionView::default();
    let mut folds: BTreeMap<(Area, Vec<String>), (usize, usize)> = BTreeMap::new();
    for owned in diags {
        let on_touched = touched.contains(&path_key(&owned.uri));
        let names_me = owned.attribution.names(session);
        if on_touched || names_me {
            let label = label(&owned.attribution, session, on_touched);
            let delivered = Delivered {
                uri: owned.uri.clone(),
                label,
                diagnostic: stamp(
                    &owned.diagnostic,
                    &owned.attribution,
                    label,
                    session,
                    owned.source.as_deref(),
                ),
            };
            if on_touched {
                view.mine.push(delivered);
            } else {
                view.caused_by_me.push(delivered);
            }
            continue;
        }
        let counts = match severity(&owned.diagnostic) {
            1 => (1, 0),
            2 => (0, 1),
            // Information and hints are not counted and never make a fold on their own.
            _ => continue,
        };
        let owners = owned
            .attribution
            .owner
            .iter()
            .map(|key| key.session.clone())
            .collect();
        let entry = folds.entry((owned.area.clone(), owners)).or_default();
        entry.0 += counts.0;
        entry.1 += counts.1;
    }
    view.others = folds
        .into_iter()
        .map(|((area, owners), (errors, warnings))| Fold {
            area,
            owners,
            errors,
            warnings,
        })
        .collect();
    view
}

/// The per-URI diagnostic lists a view publishes: the full diagnostics by URI, with each fold
/// added to its anchor's list (beside any real diagnostics on that anchor).
#[must_use]
pub fn render(view: &SessionView) -> BTreeMap<String, Vec<Value>> {
    let mut rendered: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for delivered in view.mine.iter().chain(&view.caused_by_me) {
        rendered
            .entry(delivered.uri.clone())
            .or_default()
            .push(delivered.diagnostic.clone());
    }
    for fold in &view.others {
        rendered
            .entry(fold.area.anchor.clone())
            .or_default()
            .push(fold.diagnostic());
    }
    rendered
}

/// The publishes that move a client from `previous` to `next`: every URI whose list changed,
/// with an empty list for a URI that dropped out (so a fold dropping to zero clears its anchor).
#[must_use]
pub fn changes(
    previous: &BTreeMap<String, Vec<Value>>,
    next: &BTreeMap<String, Vec<Value>>,
) -> Vec<(String, Vec<Value>)> {
    let mut out = Vec::new();
    for (uri, list) in next {
        if previous.get(uri) != Some(list) {
            out.push((uri.clone(), list.clone()));
        }
    }
    for uri in previous.keys() {
        if !next.contains_key(uri) {
            out.push((uri.clone(), Vec::new()));
        }
    }
    out
}

/// A `textDocument/publishDiagnostics` notification.
#[must_use]
pub fn publish_notification(uri: &str, diagnostics: Vec<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {"uri": uri, "diagnostics": diagnostics}
    })
}

/// One session in the snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEntry {
    /// Its identity.
    pub key: SessionKey,
    /// Whether a client of this session is attached now.
    pub attached: bool,
    /// The paths it opened or edited.
    pub touched: Vec<String>,
}

/// The attribution snapshot (C3 `attrib-<lang>-<root12>.json`), read by `epr lsp show`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// Schema version (1).
    pub v: u32,
    /// The server's language.
    pub lang: String,
    /// The checkout root.
    pub root: String,
    /// When it was written (RFC 3339).
    pub at: String,
    /// The attribution window in ms.
    pub window_ms: i64,
    /// Every session seen.
    pub sessions: Vec<SessionEntry>,
    /// Every live diagnostic.
    pub diagnostics: Vec<OwnedDiag>,
}

impl Snapshot {
    /// The view `session` receives, by the same rule the mux delivers with.
    #[must_use]
    pub fn view(&self, session: &str) -> SessionView {
        let touched: BTreeSet<String> = self
            .sessions
            .iter()
            .filter(|entry| entry.key.same_session(session))
            .flat_map(|entry| entry.touched.iter().cloned())
            .collect();
        session_view(&self.diagnostics, session, &touched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MapFs(Vec<(&'static str, &'static str)>);

    impl AreaFs for MapFs {
        fn manifest(&self, dir: &Path) -> Option<String> {
            self.0
                .iter()
                .find(|(d, _)| Path::new(d) == dir)
                .map(|(_, text)| (*text).to_owned())
        }

        fn document(&self, _path: &Path) -> Option<String> {
            None
        }
    }

    fn attribution(owners: &[&str]) -> Attribution {
        Attribution {
            owner: owners.iter().map(|s| SessionKey::named(*s)).collect(),
            ambiguous: owners.len() > 1,
            first_seen: "t".into(),
            edit: None,
        }
    }

    fn owned(uri: &str, area: &str, severity: u64, owners: &[&str]) -> OwnedDiag {
        OwnedDiag {
            uri: uri.into(),
            area: Area {
                dir: format!("/r/{area}"),
                name: area.into(),
                anchor: format!("file:///r/{area}/Cargo.toml"),
            },
            diagnostic: json!({"range": {}, "severity": severity, "message": "boom"}),
            attribution: attribution(owners),
            source: None,
        }
    }

    #[test]
    fn rust_area_is_the_nearest_package_not_a_workspace_root() {
        let fs = MapFs(vec![
            ("/r", "[workspace]\nmembers = []\n"),
            ("/r/core", "[package]\nname = \"core\"\n"),
        ]);
        let mut resolver = AreaResolver::new(fs, PathBuf::from("/r"));
        let area = resolver.area("file:///r/core/src/deep/lib.rs");
        assert_eq!(area.dir, "/r/core");
        assert_eq!(area.name, "core");
        assert_eq!(area.anchor, "file:///r/core/Cargo.toml");
        let loose = resolver.area("file:///r/notes.rs");
        assert_eq!(
            loose.dir, "/r",
            "outside every package, the root is the area"
        );
        assert_eq!(loose.anchor, "file:///r/Cargo.toml");
        let mut bare = AreaResolver::new(MapFs(vec![]), PathBuf::from("/q"));
        let alone = bare.area("file:///q/notes.rs");
        assert_eq!(
            alone.anchor, "file:///q/notes.rs",
            "no root manifest: the file anchors"
        );
    }

    #[test]
    fn view_delivers_touched_and_caused_in_full_and_folds_the_rest() {
        let diags = vec![
            owned("file:///r/core/src/lib.rs", "core", 1, &["A"]),
            owned("file:///r/client/src/main.rs", "client", 1, &["A"]),
            owned("file:///r/other/src/lib.rs", "other", 1, &["B"]),
            owned("file:///r/other/src/lib.rs", "other", 1, &["B"]),
            owned("file:///r/other/src/a.rs", "other", 2, &["B"]),
            owned("file:///r/other/src/a.rs", "other", 4, &["B"]),
        ];
        let touched = BTreeSet::from(["/r/core/src/lib.rs".to_owned()]);
        let view = session_view(&diags, "A", &touched);
        assert_eq!(view.mine.len(), 1);
        assert_eq!(view.mine[0].label, Label::Mine);
        assert_eq!(view.caused_by_me.len(), 1);
        assert_eq!(view.caused_by_me[0].label, Label::CausedByMe);
        assert_eq!(view.others.len(), 1);
        assert_eq!(
            view.others[0].message(),
            "other: 2 errors, 1 warnings from session B (in flight)"
        );
        let rendered = render(&view);
        let fold = &rendered["file:///r/other/Cargo.toml"][0];
        assert_eq!(fold["severity"], 3);
        assert_eq!(fold["data"]["epr"]["fold"], true);
    }

    #[test]
    fn foreign_owner_prefixes_the_message_and_stamps_data() {
        let diags = vec![owned(
            "file:///r/client/src/main.rs",
            "client",
            1,
            &["aaaaaaaa-1111"],
        )];
        let touched = BTreeSet::from(["/r/client/src/main.rs".to_owned()]);
        let view = session_view(&diags, "bbbbbbbb-2222", &touched);
        let d = &view.mine[0];
        assert_eq!(d.label, Label::Other);
        assert_eq!(d.diagnostic["message"], "[by aaaaaaaa] boom");
        assert_eq!(d.diagnostic["data"]["epr"]["attribution"], "other");
        assert_eq!(d.diagnostic["data"]["epr"]["owners"], json!(["aaaaaaaa"]));
        let back = unstamp(&d.diagnostic).unwrap();
        assert_eq!(
            back, diags[0].diagnostic,
            "a code-action context gets the original"
        );
        assert!(unstamp(
            &render(&SessionView {
                others: vec![Fold {
                    area: diags[0].area.clone(),
                    owners: vec!["x".into()],
                    errors: 1,
                    warnings: 0,
                }],
                ..SessionView::default()
            })["file:///r/client/Cargo.toml"][0]
        )
        .is_none());
    }

    #[test]
    fn server_data_object_survives_stamping() {
        let mut d = owned("file:///u", "u", 1, &["A"]);
        d.diagnostic["data"] = json!({"rust": 1});
        let stamped = stamp(&d.diagnostic, &d.attribution, Label::Mine, "A", None);
        assert_eq!(stamped["data"]["rust"], 1);
        assert_eq!(
            stamped["message"], "boom",
            "own diagnostics carry no prefix"
        );
        assert_eq!(unstamp(&stamped).unwrap(), d.diagnostic);
    }

    #[test]
    fn a_publishers_own_epr_keys_survive_stamping() {
        let mut d = owned("file:///u", "u", 1, &["A"]);
        d.diagnostic["data"] = json!({"epr": {"gate": {"run": "r1"}, "attribution": "x"}});
        let stamped = stamp(
            &d.diagnostic,
            &d.attribution,
            Label::Mine,
            "A",
            Some("gate-live"),
        );
        assert_eq!(stamped["data"]["epr"]["gate"]["run"], "r1");
        assert_eq!(
            stamped["data"]["epr"]["attribution"], "mine",
            "the stamp wins"
        );
        assert_eq!(stamped["data"]["epr"]["source"], "gate-live");
    }

    #[test]
    fn ambiguous_reaches_every_owner_in_full_and_unattributed_folds() {
        let diags = vec![
            owned("file:///r/x/src/lib.rs", "x", 1, &["A", "B"]),
            owned("file:///r/y/src/lib.rs", "y", 1, &[]),
        ];
        for session in ["A", "B"] {
            let view = session_view(&diags, session, &BTreeSet::new());
            assert_eq!(view.caused_by_me.len(), 1, "{session}");
            assert_eq!(view.caused_by_me[0].label, Label::Ambiguous);
            assert_eq!(view.caused_by_me[0].diagnostic["message"], "[by A,B] boom");
            assert_eq!(
                view.others[0].message(),
                "y: 1 errors, 0 warnings (unattributed)"
            );
        }
    }

    #[test]
    fn a_fold_dropping_to_zero_publishes_empty_on_its_anchor() {
        let before = render(&session_view(
            &[owned("file:///r/o/src/lib.rs", "o", 1, &["B"])],
            "A",
            &BTreeSet::new(),
        ));
        let after = render(&session_view(&[], "A", &BTreeSet::new()));
        assert_eq!(
            changes(&before, &after),
            vec![("file:///r/o/Cargo.toml".to_owned(), vec![])]
        );
        assert!(changes(&after, &after).is_empty());
    }

    #[test]
    fn snapshot_view_matches_the_live_view() {
        let diags = vec![owned("file:///r/c/src/lib.rs", "c", 1, &["A"])];
        let snapshot = Snapshot {
            v: 1,
            lang: "rust".into(),
            root: "/r".into(),
            at: "t".into(),
            window_ms: 30_000,
            sessions: vec![SessionEntry {
                key: SessionKey::named("A"),
                attached: true,
                touched: vec!["/r/c/src/lib.rs".into()],
            }],
            diagnostics: diags.clone(),
        };
        let text = serde_json::to_string(&snapshot).unwrap();
        let back: Snapshot = serde_json::from_str(&text).unwrap();
        let touched = BTreeSet::from(["/r/c/src/lib.rs".to_owned()]);
        assert_eq!(back.view("A"), session_view(&diags, "A", &touched));
    }
}
