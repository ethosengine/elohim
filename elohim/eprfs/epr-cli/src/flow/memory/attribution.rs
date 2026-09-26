//! Who wrote a memory entry, and when — decided in ONE place.
//!
//! The harness hook used to regex-parse an entry's frontmatter for `originSessionId` and
//! `modified`, and import trusted the `--session`/`--as-of` it passed. Two parsers disagree (a
//! `description: |` block quoting `originSessionId: x` read as the origin), and a trusted flag is
//! a misattribution path. This module is the only authority: it reads the entry through
//! [`entries::parse`], the harness's durable write witness, and the actor plane, and every caller
//! — the entry-form import, `--steward-of-record`, the `attribution` report the hook reads —
//! asks it.
//!
//! ## The writer of the CURRENT bytes, in order of authority
//!
//! 1. the EARLIEST harness write witness (`.eprfs/status/memory-writes.jsonl`) whose sha256
//!    matches the entry's current bytes — the harness's own record, made at write time;
//! 2. only when no witness of these bytes exists (a legacy entry predating the log), the entry's
//!    own `originSessionId` — self-report, which an editor who leaves the first writer's
//!    frontmatter in place does not change: frontmatter names who STARTED the entry, a witness
//!    names who wrote THESE bytes;
//! 3. else a caller's `--session`.
//!
//! A caller's `--session` is refused when it names a writer other than (1) or (2).
//!
//! ## The write instant, in order of authority
//!
//! 1. the matching witness's `observedAt`, whenever (1) above named the writer;
//! 2. else the entry's own `modified`;
//! 3. otherwise **ambiguous**. Live mtime drifts forward (a checkout, a re-save), which can make a
//!    LATER claim look current, so an ambiguous entry is never attributed: it is unattributable,
//!    and never guessed.
//!
//! A caller's `--as-of` is not evidence. It must agree with (1) or (2) where they exist, and it
//! cannot rescue an ambiguous entry.
//!
//! ## An edit by another participant
//!
//! When the current bytes were written by a participant other than the entry's existing
//! contributor, the report says so (`editedByAnotherParticipant`, writer vs contributor) and the
//! entry is not importable: a contribution is never re-authored by someone else. Recording such
//! an edit as its own act is a different act, not this module's.
//!
//! When several witnesses carry the entry's current bytes, the EARLIEST wins: the first session
//! to produce those bytes is their author, and a later identical re-save by another session
//! never takes them over (the hook does not even witness a write that changes nothing).
//!
//! Frontmatter the parser cannot read without guessing — a block scalar with an empty body
//! followed by a line that is either its unindented body or a new key — is MALFORMED: the entry
//! is refused, never read either way.
//!
//! ## The boundary: an honesty fence, not a proof
//!
//! Every input here is SELF-REPORT among cooperating participants: an entry's
//! `originSessionId`/`modified` are bytes its writer (or anyone after) can type, a witness line
//! is appended by the harness but lives in a plain file anyone can append to, and an actor claim
//! is the actor's own statement. Within that fence this module prevents ACCIDENTAL
//! misattribution — the wrong session, a later claim, a drifted mtime, a quoted key, a re-save.
//! It does not, and cannot, prevent DELIBERATE forgery: a participant who writes a false origin,
//! a backdated `modified`, or a fabricated witness line is attributed as they claim. That is
//! closed only by device-signed acts (the `Signed` records the actor plane already carries for a
//! human's claim), where the proof is a signature over the act, not a line in a file.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use elohim_epr_rea::{ActorClaim, ActorStore, AsOfBasis, MemoryActorStore, SidecarActorStore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{entries, refused};
use crate::flow::FlowResult;

/// The harness's write witnesses: one JSON line per memory edit, appended by the PostToolUse hook
/// at the edit moment (`{path, sha256, session, observedAt}`). Private, gitignored, bounded.
pub(super) const WITNESS_LOG: &str = ".eprfs/status/memory-writes.jsonl";

/// An RFC 3339 instant, parsed. Instants are compared, never strings: `…:09.540Z` sorts before
/// `…:09Z` as text and after it as time.
pub(super) fn instant(at: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(at.trim())
        .ok()
        .map(|t| t.with_timezone(&chrono::Utc))
}

/// The comparator [`ActorStore::current_for_at`] takes: two instants, compared as time.
pub(super) fn compare(a: &str, b: &str) -> Option<Ordering> {
    Some(instant(a)?.cmp(&instant(b)?))
}

fn rfc3339(t: chrono::DateTime<chrono::Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// One harness write witness.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Witness {
    pub path: String,
    pub sha256: String,
    pub session: String,
    pub observed_at: String,
}

/// When the entry was written, and on what evidence.
#[derive(Debug, Clone, Copy)]
pub(super) enum Written {
    /// The entry's own `modified`.
    Declared(chrono::DateTime<chrono::Utc>),
    /// A harness witness of these exact bytes.
    Witnessed(chrono::DateTime<chrono::Utc>),
    /// Neither: only the live mtime, which may have drifted forward. Never attributed.
    Ambiguous(Option<chrono::DateTime<chrono::Utc>>),
}

impl Written {
    pub fn proven(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        match self {
            Written::Declared(t) | Written::Witnessed(t) => Some(*t),
            Written::Ambiguous(_) => None,
        }
    }
    fn basis(&self) -> &'static str {
        match self {
            Written::Declared(_) => "modified",
            Written::Witnessed(_) => "witness",
            Written::Ambiguous(_) => "ambiguous",
        }
    }
}

/// The actor plane and the witness log, read once for a whole batch.
pub(super) struct Evidence {
    claims: MemoryActorStore,
    witnesses: Vec<Witness>,
}

impl Evidence {
    pub fn load(root: &Path) -> FlowResult<Self> {
        let mut claims = MemoryActorStore::new();
        if root.join(".eprfs/status/actors.jsonl").exists() {
            for (_, record) in SidecarActorStore::open(root)?.records()? {
                claims.append(record)?;
            }
        }
        let witnesses = std::fs::read_to_string(root.join(WITNESS_LOG))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<Witness>(line).ok())
            .collect();
        Ok(Self { claims, witnesses })
    }

    /// The claim `session` held AS OF `at`, by instant (see [`ActorStore::current_for_at`]).
    pub fn claim_as_of(
        &self,
        session: &str,
        at: &str,
    ) -> FlowResult<Option<(cid::Cid, ActorClaim, AsOfBasis)>> {
        Ok(self.claims.current_for_at(session, at, &compare)?)
    }

    /// The instant of the session's EARLIEST claim, and whether it has any claim at all.
    fn first_claim(&self, session: &str) -> FlowResult<Option<chrono::DateTime<chrono::Utc>>> {
        Ok(self
            .claims
            .claims()?
            .into_iter()
            .filter(|(_, c)| c.session == session)
            .filter_map(|(_, c)| instant(c.ordering_instant().0))
            .min())
    }

    fn has_claim(&self, session: &str) -> FlowResult<bool> {
        Ok(self
            .claims
            .claims()?
            .iter()
            .any(|(_, c)| c.session == session))
    }

    /// The EARLIEST witness of exactly these bytes at this path: the first session to produce
    /// the bytes authored them; a later identical re-save by another session never takes over.
    fn witness(&self, rel: &str, sha: &str) -> Option<&Witness> {
        self.witnesses
            .iter()
            .find(|w| w.path == rel && w.sha256 == sha)
    }
}

/// Everything the evidence says about one entry's authorship.
#[derive(Debug)]
pub(super) struct Attribution {
    pub rel: PathBuf,
    /// The writing session, and where it came from (`frontmatter`, `witness`, `caller`).
    pub session: Option<(String, &'static str)>,
    pub written: Written,
    /// The claim held at the PROVEN write instant; `None` when ambiguous or none existed yet.
    pub claim: Option<(cid::Cid, String, AsOfBasis)>,
    /// Frontmatter that cannot be read without guessing (see [`entries::Frontmatter::malformed`]).
    malformed: Option<String>,
    has_claim: bool,
    first_claim: Option<chrono::DateTime<chrono::Utc>>,
}

/// The session the entry itself names (`metadata.originSessionId`, or a flat top-level key).
pub(super) fn declared_origin(fm: &entries::Frontmatter) -> Option<String> {
    [fm.meta("originSessionId"), fm.get("originSessionId")]
        .into_iter()
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
}

fn declared_modified(fm: &entries::Frontmatter) -> Option<chrono::DateTime<chrono::Utc>> {
    [fm.meta("modified"), fm.get("modified")]
        .into_iter()
        .find_map(instant)
}

/// Resolve one entry. `caller_session`/`caller_as_of` are what a caller asserted; they are checked
/// against the entry, never trusted over it (see the module doc).
pub(super) fn resolve(
    root: &Path,
    rel: &Path,
    evidence: &Evidence,
    caller_session: Option<&str>,
    caller_as_of: Option<&str>,
) -> FlowResult<Attribution> {
    let bytes = std::fs::read(root.join(rel))?;
    let fm = entries::parse(&String::from_utf8_lossy(&bytes));
    let rel_s = rel.to_string_lossy().replace('\\', "/");
    let sha = format!("{:x}", Sha256::digest(&bytes));
    let witness = evidence.witness(&rel_s, &sha);

    // The writer of THESE bytes: the harness's earliest witness of them → the entry's own origin
    // (legacy, unwitnessed) → the caller. A caller naming another writer is refused.
    let origin = declared_origin(&fm);
    let session = match (witness, origin, caller_session) {
        (Some(w), _, Some(caller)) if w.session != caller => {
            return Err(refused(format!(
                "{rel_s} was witnessed written by session {} (harness write witness); --session \
                 {caller} names a different writer",
                w.session
            )))
        }
        (Some(w), _, _) => Some((w.session.clone(), "witness")),
        (None, Some(origin), Some(caller)) if origin != caller => {
            return Err(refused(format!(
                "{rel_s} declares originSessionId {origin}; --session {caller} names a different \
                 writer — with no harness witness of its bytes the entry's own origin governs, \
                 and --session only names the writer of an entry that names none"
            )))
        }
        (None, Some(o), _) => Some((o, "frontmatter")),
        (None, None, Some(caller)) => Some((caller.to_string(), "caller")),
        (None, None, None) => None,
    };

    // The write instant: the witness that named the writer → `modified` → ambiguous.
    let written = match (
        witness.and_then(|w| instant(&w.observed_at)),
        declared_modified(&fm),
    ) {
        (Some(w), _) => Written::Witnessed(w),
        (None, Some(m)) => Written::Declared(m),
        (None, None) => Written::Ambiguous(
            std::fs::metadata(root.join(rel))
                .ok()
                .and_then(|m| m.modified().ok())
                .map(chrono::DateTime::<chrono::Utc>::from),
        ),
    };
    if let Some(asserted) = caller_as_of {
        let asserted_at = instant(asserted)
            .ok_or_else(|| refused(format!("--as-of `{asserted}` is not an RFC 3339 instant")))?;
        if let Some(proven) = written.proven() {
            if proven != asserted_at {
                return Err(refused(format!(
                    "{rel_s} was written at {} (its {}); --as-of {asserted} disagrees — the \
                     entry's own evidence governs",
                    rfc3339(proven),
                    written.basis()
                )));
            }
        }
    }

    let (claim, has_claim, first_claim) = match &session {
        Some((s, _)) => (
            match written.proven() {
                Some(t) => evidence
                    .claim_as_of(s, &rfc3339(t))?
                    .map(|(cid, c, basis)| (cid, c.claimed.0, basis)),
                None => None,
            },
            evidence.has_claim(s)?,
            evidence.first_claim(s)?,
        ),
        None => (None, false, None),
    };
    Ok(Attribution {
        rel: rel.to_path_buf(),
        session,
        written,
        claim,
        malformed: fm.malformed.clone(),
        has_claim,
        first_claim,
    })
}

impl Attribution {
    fn session_id(&self) -> Option<&str> {
        self.session.as_ref().map(|(s, _)| s.as_str())
    }

    /// The write instant the as-of rule used, as RFC 3339, when it is proven.
    pub fn as_of(&self) -> Option<String> {
        self.written.proven().map(rfc3339)
    }

    /// `Ok` when a witnessed agent (or claimed participant) authored the entry; otherwise the
    /// named reason it is unattributable.
    pub fn verdict(&self) -> Result<(), String> {
        if let Some(why) = &self.malformed {
            return Err(format!("its frontmatter is malformed: {why}"));
        }
        let Some(session) = self.session_id() else {
            return Err("no originSessionId and no harness write witness names its writer".into());
        };
        if self.claim.is_some() {
            return Ok(());
        }
        if !self.has_claim {
            return Err(format!(
                "origin session {session} registered no actor claim"
            ));
        }
        let first = self
            .first_claim
            .map(rfc3339)
            .unwrap_or_else(|| "undated".into());
        Err(match self.written {
            Written::Ambiguous(mtime) => format!(
                "its write time is ambiguous (no `modified`, no harness witness of its current \
                 bytes; mtime {}), and origin session {session} has claims (first {first}) — a \
                 later claim could look current, so it is never guessed",
                mtime.map(rfc3339).unwrap_or_else(|| "unknown".into())
            ),
            Written::Declared(t) | Written::Witnessed(t) => format!(
                "origin session {session} had registered no actor claim when it was written ({}, \
                 by its {}); its first claim is dated {first}",
                rfc3339(t),
                self.written.basis()
            ),
        })
    }

    /// `Ok` when NO agent author could possibly be witnessed — the only case a standing human may
    /// stand for the entry as steward of record: it has no origin session; or that session never
    /// claimed; or its PROVEN write predates that session's first claim. Live mtime never admits.
    pub fn steward_of_record(&self) -> Result<(), String> {
        if let Some(why) = &self.malformed {
            // Its origin cannot be read, so an agent author cannot be ruled out.
            return Err(format!("its frontmatter is malformed: {why}"));
        }
        let Some(session) = self.session_id() else {
            return Ok(());
        };
        if !self.has_claim {
            return Ok(());
        }
        if let (Some(t), Some(first)) = (self.written.proven(), self.first_claim) {
            if t < first {
                return Ok(());
            }
        }
        Err(match (self.written, &self.claim) {
            (_, Some((_, claimed, _))) => format!(
                "session {session} held the claim {claimed} when it was written — it has a \
                 witnessable author; steward of record never overrides a real author"
            ),
            (Written::Ambiguous(_), None) => format!(
                "its write time is ambiguous and session {session} has claimed — an agent author \
                 could have written it, so no one may stand for it until its `modified` or a \
                 harness witness proves the write predates that session's first claim"
            ),
            _ => format!("session {session} could be its author"),
        })
    }

    fn report(&self, contribution: &Contributed, indexed: bool) -> Value {
        let verdict = self.verdict();
        let steward = self.steward_of_record();
        let author = self.claim.as_ref().map(|(_, c, _)| c.clone());
        let contributed_by = contribution.author.clone();
        // An edit by another participant: the current bytes' writer is not the contributor, and
        // the contribution does not already cover these exact bytes (a re-save of contributed
        // bytes is not an edit, whoever the witness names).
        let edited_by_another = match (&contributed_by, &author) {
            (Some(by), Some(writer)) if by != writer && !contribution.covers_current => {
                Some(json!({"writer": writer, "contributor": by}))
            }
            _ => None,
        };
        let importable = verdict.is_ok()
            && edited_by_another.is_none()
            && contributed_by
                .as_deref()
                .is_none_or(|by| Some(by) == author.as_deref());
        json!({
            "entry": self.rel.file_name().map(|n| n.to_string_lossy().to_string()),
            "path": self.rel.to_string_lossy().replace('\\', "/"),
            "contributedBy": contributed_by,
            "indexed": indexed,
            "session": self.session_id(),
            "sessionSource": self.session.as_ref().map(|(_, s)| *s),
            "writtenAt": self.written.proven().map(rfc3339),
            "writtenBasis": self.written.basis(),
            "claim": author,
            "claimBasis": self.claim.as_ref().map(|(_, _, b)| match b {
                AsOfBasis::Recorded => "recordedAt",
                AsOfBasis::LegacyClaimedAt => "claimedAt (legacy)",
            }),
            "attributable": verdict.is_ok(),
            "reason": verdict.err(),
            "importable": importable,
            "editedByAnotherParticipant": edited_by_another,
            "supersededBy": contribution.superseded_by,
            "supersessionPending": contribution.supersession_pending,
            "stewardOfRecordAdmissible": steward.is_ok(),
            "stewardOfRecordReason": steward.err(),
        })
    }
}

/// What the contribution plane holds for one entry, as the report reads it.
#[derive(Debug, Default)]
struct Contributed {
    author: Option<String>,
    /// Whether the contribution's pinned source is the entry's CURRENT bytes.
    covers_current: bool,
    /// The umbrella entry whose EFFECTIVE fold hides this contribution from the index.
    superseded_by: Option<String>,
    /// The umbrella entry whose fold of this contribution awaits a Steward's verdict.
    supersession_pending: Option<String>,
}

/// `epr flow memory attribution <dir> | <entry.md>…` — the read the harness hook takes instead of
/// parsing frontmatter itself: per entry, its writer, write instant, the claim it resolves to,
/// whether it is attributable, importable, or admissible for steward of record. Writes nothing.
pub(super) fn report(
    root: &Path,
    files: &[PathBuf],
    contributions_rel: &Path,
) -> FlowResult<Value> {
    let evidence = Evidence::load(root)?;
    // The supersession folds, read once for the batch: recall and attribution still see a
    // superseded member; the report says who folded it and whether the fold stands.
    let (hidden, pending) = if root.join(contributions_rel).is_dir() {
        let mut reader = super::validation::Reader::new(root)?;
        let acts = super::ContributionActs::open(root)?;
        let folds = super::supersession::load(&mut reader, contributions_rel, &acts);
        let pending: BTreeMap<String, String> = folds
            .iter()
            .filter(|f| !f.effective())
            .flat_map(|f| {
                f.members
                    .iter()
                    .map(move |m| (m.path.clone(), f.umbrella.clone()))
            })
            .collect();
        (super::supersession::hidden(&folds), pending)
    } else {
        (BTreeMap::new(), BTreeMap::new())
    };
    let mut rows = Vec::new();
    for rel in files {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_default();
        let fm = entries::parse(&String::from_utf8_lossy(&bytes));
        let stem = rel
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let request = contributions_rel
            .join(format!("{stem}.json"))
            .to_string_lossy()
            .replace('\\', "/");
        let mut contributed = Contributed::default();
        if let Ok(text) = std::fs::read_to_string(root.join(&request)) {
            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                contributed.author = value["author"].as_str().map(str::to_string);
                let current = eprfs_core::BlobCid::compute_raw(&bytes).to_string();
                contributed.covers_current = value["sources"]
                    .as_array()
                    .is_some_and(|s| s.iter().any(|s| s["resource"]["cid"] == json!(current)));
            }
            let raw = eprfs_core::BlobCid::compute_raw(text.as_bytes()).to_string();
            contributed.superseded_by = hidden
                .get(&request)
                .filter(|(cid, _)| *cid == raw)
                .map(|(_, umbrella)| umbrella.clone());
            if contributed.superseded_by.is_none() {
                contributed.supersession_pending = pending.get(&request).cloned();
            }
        }
        let attribution = resolve(root, rel, &evidence, None, None)?;
        rows.push(attribution.report(&contributed, fm.indexed()));
    }
    Ok(json!({"operation": "attribution", "witnessLog": WITNESS_LOG, "entries": rows}))
}
