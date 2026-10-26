//! Attribution: which session's edit made a diagnostic appear (contract C7).
//!
//! A diagnostic is NEW when it was not in the previous publish for its URI (matched by range,
//! message and code). A new diagnostic is owned by the sessions whose edit events fall in
//! `[publish_at - window, publish_at]`: none is unattributed, more than one distinct session is
//! ambiguous with all of them, never a guess. A diagnostic keeps its owner until it clears.
//!
//! Edit events arrive from the C2 log ([`EditLines`]) and from the mux's didChange/didSave.
//! Everything here is pure; time is passed in as milliseconds since the Unix epoch.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A client's identity: the `epr session` tuple (contract C1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionKey {
    /// The session id.
    pub session: String,
    /// The session owner's pid, when known.
    #[serde(default)]
    pub owner_pid: Option<u32>,
    /// The owner's start time in clock ticks, as decimal text (C1 / C2 wire form).
    #[serde(default, deserialize_with = "ticks")]
    pub owner_start_time: Option<String>,
}

impl SessionKey {
    /// A session with no owner tuple.
    #[must_use]
    pub fn named(session: impl Into<String>) -> SessionKey {
        SessionKey {
            session: session.into(),
            owner_pid: None,
            owner_start_time: None,
        }
    }

    /// The first eight characters of the session id, as attribution prints it.
    #[must_use]
    pub fn short8(&self) -> String {
        short8(&self.session)
    }

    /// Whether this key names the same session as `other`. Sessions are compared by id: the
    /// owner tuple is carried for display and liveness, and a fallback resolver may report a
    /// different owner pid for the same session id.
    #[must_use]
    pub fn same_session(&self, other: &str) -> bool {
        self.session == other
    }
}

/// The first eight characters of a session id.
#[must_use]
pub fn short8(session: &str) -> String {
    session.chars().take(8).collect()
}

/// Accept the start time as a string (C1/C2) or a number (lenient readers), empty as absent.
fn ticks<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Ok(match Option::<Value>::deserialize(deserializer)? {
        Some(Value::String(text)) if !text.is_empty() => Some(text),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    })
}

/// Where an edit event came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditSource {
    /// A line of the C2 edit log.
    Log,
    /// A client's `textDocument/didChange`.
    DidChange,
    /// A client's `textDocument/didSave`.
    DidSave,
}

/// One edit by one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditEvent {
    /// When the edit happened, in ms since the Unix epoch.
    pub at_ms: i64,
    /// The editing session.
    pub session: SessionKey,
    /// The edited file's absolute path, when known.
    pub path: Option<String>,
    /// Where the event came from.
    pub source: EditSource,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditLine {
    v: u32,
    at: String,
    session: String,
    #[serde(default)]
    owner_pid: Option<u32>,
    #[serde(default, deserialize_with = "ticks")]
    owner_start_time: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

/// Parse one C2 line. Torn, unknown-version and malformed lines are `None` (C2: readers skip).
#[must_use]
pub fn parse_edit_line(line: &str) -> Option<EditEvent> {
    let parsed: EditLine = serde_json::from_str(line.trim()).ok()?;
    if parsed.v != 1 || parsed.session.is_empty() {
        return None;
    }
    let at_ms = chrono::DateTime::parse_from_rfc3339(&parsed.at)
        .ok()?
        .timestamp_millis();
    Some(EditEvent {
        at_ms,
        session: SessionKey {
            session: parsed.session,
            owner_pid: parsed.owner_pid,
            owner_start_time: parsed.owner_start_time,
        },
        path: parsed.path,
        source: EditSource::Log,
    })
}

/// Splits appended C2 bytes into whole lines, holding a torn trailing line until it completes.
#[derive(Debug, Default)]
pub struct EditLines {
    partial: Vec<u8>,
}

impl EditLines {
    /// Feed newly appended bytes; return the events of every completed line.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<EditEvent> {
        self.partial.extend_from_slice(bytes);
        let Some(last) = self.partial.iter().rposition(|b| *b == b'\n') else {
            return Vec::new();
        };
        let complete: Vec<u8> = self.partial.drain(..=last).collect();
        complete
            .split(|b| *b == b'\n')
            .filter_map(|line| std::str::from_utf8(line).ok())
            .filter_map(parse_edit_line)
            .collect()
    }

    /// Forget a torn line, as when the writer prunes the log in place.
    pub fn reset(&mut self) {
        self.partial.clear();
    }
}

/// The edit an attribution points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditRef {
    /// The edited path, when known.
    pub path: Option<String>,
    /// When it was edited (RFC 3339).
    pub at: String,
}

/// Who a diagnostic belongs to (C7 attribution record).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribution {
    /// The owning sessions: empty when unattributed, more than one when ambiguous.
    pub owner: Vec<SessionKey>,
    /// Whether more than one distinct session edited in the window.
    pub ambiguous: bool,
    /// When the diagnostic first appeared (RFC 3339).
    pub first_seen: String,
    /// The latest edit in the window, when any.
    pub edit: Option<EditRef>,
}

impl Attribution {
    /// Whether `session` is among the owners.
    #[must_use]
    pub fn names(&self, session: &str) -> bool {
        self.owner.iter().any(|key| key.same_session(session))
    }
}

/// One live diagnostic and its attribution.
#[derive(Debug, Clone, PartialEq)]
pub struct Tracked {
    /// The diagnostic as the server published it.
    pub diagnostic: Value,
    /// Its owner.
    pub attribution: Attribution,
}

/// RFC 3339 with milliseconds, UTC.
#[must_use]
pub fn rfc3339(at_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(at_ms)
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// The identity of a diagnostic across publishes: its range, message and code.
#[must_use]
pub fn diagnostic_key(diagnostic: &Value) -> String {
    serde_json::json!([
        diagnostic.get("range"),
        diagnostic.get("message"),
        diagnostic.get("code")
    ])
    .to_string()
}

/// Who published a diagnostic set: the language server, or a non-LSP publisher (a gate tail)
/// whose diagnostics ride the same attribution and fold path under their own tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSource<'a> {
    /// The language server.
    Server,
    /// Another publisher, named by its tag.
    External(&'a str),
}

impl DiagnosticSource<'_> {
    /// The tag kept on each diagnostic: `None` for the server.
    #[must_use]
    pub fn tag(self) -> Option<String> {
        match self {
            DiagnosticSource::Server => None,
            DiagnosticSource::External(tag) => Some(tag.to_owned()),
        }
    }
}

/// A publisher's set of diagnostics for one URI: `(source tag, uri)`.
pub type SetKey = (Option<String>, String);

/// The attribution state for one server.
#[derive(Debug)]
pub struct Attributor {
    window_ms: i64,
    edits: Vec<EditEvent>,
    seen: HashSet<(i64, String, Option<String>)>,
    current: HashMap<SetKey, Vec<Tracked>>,
    cleared: HashMap<(SetKey, String), (Attribution, i64)>,
    touched: HashMap<String, BTreeSet<String>>,
}

/// The most paths remembered per session as "edited by it" (a bound on a long-lived daemon).
const TOUCHED_CAP: usize = 10_000;

impl Attributor {
    /// A fresh state with the policy's attribution window.
    #[must_use]
    pub fn new(window_ms: i64) -> Attributor {
        Attributor {
            window_ms,
            edits: Vec::new(),
            seen: HashSet::new(),
            current: HashMap::new(),
            cleared: HashMap::new(),
            touched: HashMap::new(),
        }
    }

    /// Record an edit. Returns whether the session's set of edited paths grew, which changes
    /// what that session receives in full.
    pub fn record_edit(&mut self, event: EditEvent) -> bool {
        let identity = (
            event.at_ms,
            event.session.session.clone(),
            event.path.clone(),
        );
        if !self.seen.insert(identity) {
            return false;
        }
        let mut grew = false;
        if let Some(path) = &event.path {
            let set = self
                .touched
                .entry(event.session.session.clone())
                .or_default();
            if set.len() < TOUCHED_CAP {
                grew = set.insert(path.clone());
            }
        }
        self.edits.push(event);
        grew
    }

    /// Whether the C2 log names an edit of `path` inside the window ending at `now_ms`.
    #[must_use]
    pub fn log_names_edit_of(&self, path: &str, now_ms: i64) -> bool {
        let from = now_ms - self.window_ms;
        self.edits.iter().any(|edit| {
            edit.source == EditSource::Log
                && edit.path.as_deref() == Some(path)
                && edit.at_ms >= from
                && edit.at_ms <= now_ms
        })
    }

    /// The paths a session has edited while this daemon has run.
    #[must_use]
    pub fn touched(&self, session: &str) -> Option<&BTreeSet<String>> {
        self.touched.get(session)
    }

    /// Every session that has edited, with its paths.
    pub fn touched_all(&self) -> impl Iterator<Item = (&String, &BTreeSet<String>)> {
        self.touched.iter()
    }

    /// Apply one publish for `uri` at `now_ms`, attributing the new diagnostics. Each source
    /// replaces only its own set for the URI.
    pub fn publish(
        &mut self,
        source: DiagnosticSource<'_>,
        uri: &str,
        diagnostics: Vec<Value>,
        now_ms: i64,
    ) {
        let set: SetKey = (source.tag(), uri.to_owned());
        self.prune(now_ms);
        let mut previous: HashMap<String, Vec<Attribution>> = HashMap::new();
        for tracked in self.current.remove(&set).unwrap_or_default() {
            previous
                .entry(diagnostic_key(&tracked.diagnostic))
                .or_default()
                .push(tracked.attribution);
        }
        let mut next = Vec::with_capacity(diagnostics.len());
        for diagnostic in diagnostics {
            let key = diagnostic_key(&diagnostic);
            let attribution = previous
                .get_mut(&key)
                .and_then(Vec::pop)
                .or_else(|| {
                    self.cleared
                        .remove(&(set.clone(), key.clone()))
                        .map(|(attribution, _)| attribution)
                })
                .unwrap_or_else(|| self.attribute(now_ms));
            next.push(Tracked {
                diagnostic,
                attribution,
            });
        }
        // A diagnostic that clears and reappears within the window (a server that empties a URI
        // before re-checking it) keeps its owner rather than being re-attributed to whoever
        // happened to save in between.
        for (key, attributions) in previous {
            if let Some(attribution) = attributions.into_iter().next() {
                self.cleared
                    .insert((set.clone(), key), (attribution, now_ms + self.window_ms));
            }
        }
        if !next.is_empty() {
            self.current.insert(set, next);
        }
    }

    fn attribute(&self, now_ms: i64) -> Attribution {
        let from = now_ms - self.window_ms;
        let mut in_window: Vec<&EditEvent> = self
            .edits
            .iter()
            .filter(|edit| edit.at_ms >= from && edit.at_ms <= now_ms)
            .collect();
        // The record's `edit` is the latest edit in the window; on a tie the lexicographically
        // smaller session wins (D8). It is a pointer for the reader, never what decides
        // ambiguity: `owners` lists every candidate and `ambiguous` alone says whether several.
        in_window.sort_by(|a, b| {
            a.at_ms
                .cmp(&b.at_ms)
                .then_with(|| b.session.session.cmp(&a.session.session))
        });
        let mut owner: Vec<SessionKey> = Vec::new();
        for edit in &in_window {
            if !owner.iter().any(|key| key.session == edit.session.session) {
                owner.push(edit.session.clone());
            }
        }
        owner.sort_by(|a, b| a.session.cmp(&b.session));
        Attribution {
            ambiguous: owner.len() > 1,
            owner,
            first_seen: rfc3339(now_ms),
            edit: in_window.last().map(|edit| EditRef {
                path: edit.path.clone(),
                at: rfc3339(edit.at_ms),
            }),
        }
    }

    fn prune(&mut self, now_ms: i64) {
        let horizon = now_ms - self.window_ms;
        self.edits.retain(|edit| edit.at_ms >= horizon);
        self.seen.retain(|(at, _, _)| *at >= horizon);
        self.cleared.retain(|_, (_, expires)| *expires >= now_ms);
    }

    /// Every live diagnostic, by (source, URI).
    pub fn current(&self) -> impl Iterator<Item = (&SetKey, &Vec<Tracked>)> {
        self.current.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn diag(line: u64, message: &str) -> Value {
        json!({
            "range": {"start": {"line": line, "character": 0}, "end": {"line": line, "character": 1}},
            "severity": 1,
            "message": message
        })
    }

    fn edit(at_ms: i64, session: &str, path: &str) -> EditEvent {
        EditEvent {
            at_ms,
            session: SessionKey::named(session),
            path: Some(path.into()),
            source: EditSource::Log,
        }
    }

    fn owners(state: &Attributor, uri: &str) -> Vec<(Vec<String>, bool)> {
        state
            .current
            .get(&(None, uri.to_owned()))
            .map(|list| {
                list.iter()
                    .map(|t| {
                        (
                            t.attribution
                                .owner
                                .iter()
                                .map(|k| k.session.clone())
                                .collect(),
                            t.attribution.ambiguous,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn new_diagnostic_is_owned_by_the_session_that_edited_in_the_window() {
        let mut state = Attributor::new(30_000);
        state.record_edit(edit(1_000, "A", "/r/core/src/lib.rs"));
        state.publish(
            DiagnosticSource::Server,
            "file:///r/client/src/main.rs",
            vec![diag(1, "E")],
            20_000,
        );
        assert_eq!(
            owners(&state, "file:///r/client/src/main.rs"),
            vec![(vec!["A".to_owned()], false)]
        );
    }

    #[test]
    fn edits_outside_the_window_leave_a_diagnostic_unattributed() {
        let mut state = Attributor::new(30_000);
        state.record_edit(edit(1_000, "A", "/p"));
        state.publish(
            DiagnosticSource::Server,
            "file:///u",
            vec![diag(1, "E")],
            40_000,
        );
        assert_eq!(owners(&state, "file:///u"), vec![(vec![], false)]);
    }

    #[test]
    fn two_sessions_in_the_window_make_the_diagnostic_ambiguous_with_both() {
        let mut state = Attributor::new(30_000);
        state.record_edit(edit(10_000, "B", "/p"));
        state.record_edit(edit(11_000, "A", "/q"));
        state.record_edit(edit(12_000, "A", "/q"));
        state.publish(
            DiagnosticSource::Server,
            "file:///u",
            vec![diag(1, "E")],
            20_000,
        );
        assert_eq!(
            owners(&state, "file:///u"),
            vec![(vec!["A".to_owned(), "B".to_owned()], true)]
        );
    }

    #[test]
    fn the_edit_pointer_is_the_latest_edit_and_a_tie_goes_to_the_smaller_session() {
        let mut state = Attributor::new(30_000);
        state.record_edit(edit(5_000, "B", "/b"));
        state.record_edit(edit(5_000, "A", "/a"));
        state.record_edit(edit(1_000, "C", "/c"));
        state.publish(
            DiagnosticSource::Server,
            "file:///u",
            vec![diag(1, "E")],
            9_000,
        );
        let tracked = &state.current[&(None, "file:///u".to_owned())][0];
        let pointer = tracked.attribution.edit.as_ref().unwrap();
        assert_eq!(pointer.path.as_deref(), Some("/a"), "tie at 5000: A < B");
        assert!(tracked.attribution.ambiguous);
        assert_eq!(tracked.attribution.owner.len(), 3);
    }

    #[test]
    fn a_diagnostic_keeps_its_owner_until_it_clears() {
        let mut state = Attributor::new(30_000);
        state.record_edit(edit(1_000, "A", "/p"));
        state.publish(
            DiagnosticSource::Server,
            "file:///u",
            vec![diag(1, "E")],
            2_000,
        );
        state.record_edit(edit(3_000, "B", "/q"));
        state.publish(
            DiagnosticSource::Server,
            "file:///u",
            vec![diag(1, "E"), diag(2, "F")],
            4_000,
        );
        assert_eq!(
            owners(&state, "file:///u"),
            vec![
                (vec!["A".to_owned()], false),
                (vec!["A".to_owned(), "B".to_owned()], true)
            ],
            "the old diagnostic stays A's; only the new one sees both edits"
        );
        state.publish(DiagnosticSource::Server, "file:///u", vec![], 5_000);
        assert!(owners(&state, "file:///u").is_empty());
    }

    #[test]
    fn a_flicker_inside_the_window_keeps_the_owner() {
        let mut state = Attributor::new(1_000);
        state.record_edit(edit(0, "A", "/p"));
        state.publish(
            DiagnosticSource::Server,
            "file:///u",
            vec![diag(1, "E")],
            500,
        );
        state.publish(DiagnosticSource::Server, "file:///u", vec![], 600);
        state.record_edit(edit(700, "B", "/q"));
        state.publish(
            DiagnosticSource::Server,
            "file:///u",
            vec![diag(1, "E")],
            800,
        );
        assert_eq!(
            owners(&state, "file:///u"),
            vec![(vec!["A".to_owned()], false)]
        );
        // Past the window, a reappearance is a new diagnostic.
        state.publish(DiagnosticSource::Server, "file:///u", vec![], 900);
        state.record_edit(edit(2_500, "B", "/q"));
        state.publish(
            DiagnosticSource::Server,
            "file:///u",
            vec![diag(1, "E")],
            3_000,
        );
        assert_eq!(
            owners(&state, "file:///u"),
            vec![(vec!["B".to_owned()], false)]
        );
    }

    #[test]
    fn duplicate_events_are_counted_once_and_grow_the_touched_set() {
        let mut state = Attributor::new(30_000);
        assert!(state.record_edit(edit(1, "A", "/p")));
        assert!(!state.record_edit(edit(1, "A", "/p")));
        assert!(!state.record_edit(edit(2, "A", "/p")), "same path again");
        assert_eq!(state.touched("A").unwrap().len(), 1);
    }

    #[test]
    fn edit_lines_skip_torn_and_unknown_lines_and_hold_a_partial_tail() {
        let mut lines = EditLines::default();
        let good = r#"{"v":1,"at":"2026-10-26T10:00:00.250Z","session":"abc","ownerPid":7,"ownerStartTime":"99","path":"/r/x.rs","tool":"Edit"}"#;
        let bytes = format!("{good}\n{{torn\n{{\"v\":2}}\n{good}");
        let events = lines.feed(bytes.as_bytes());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].session.owner_start_time.as_deref(), Some("99"));
        assert_eq!(events[0].at_ms % 1000, 250);
        let rest = lines.feed(b"\n");
        assert_eq!(rest.len(), 1, "the held tail completes");
    }

    #[test]
    fn empty_start_time_from_the_fallback_writer_is_absent() {
        let line = r#"{"v":1,"at":"2026-10-26T10:00:00Z","session":"anon-1","ownerPid":1,"ownerStartTime":"","path":"/x"}"#;
        assert_eq!(
            parse_edit_line(line).unwrap().session.owner_start_time,
            None
        );
    }
}
