//! The first [`DerivationIndex`]: brit's build attestations, read for one gate closure.
//!
//! A gate verdict is already a derivation keyed by its input closure: `gate-runner.mjs` records it
//! with `brit-build-ref build put --step gate:<project> --inputs-hash <key>` and finds it with
//! `build find`. This adapter reads that same store and admits a record only under the reuse rules
//! of `genesis/orchestrator/gate-evidence.mjs::reusableHit`, ported rule for rule. It writes no
//! second store: brit stays the one writer, and [`DerivationIndex::record`] only confirms that the
//! attestation it names already stands.
//!
//! One index answers for one closure: it is opened on the closure's input root, its brit dedup
//! key and the gate method it was built for, and every other key reads as not recorded. It lives
//! in this CLI crate rather than eprfs-core because it shells out to a CLI.

use std::{
    collections::BTreeMap,
    env,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

use async_trait::async_trait;
use eprfs_core::{
    BlobCid, DerivationBasis, DerivationIndex, DerivationKey, DerivationLookup, DerivationRecord,
    DerivationUnanswered, EprfsError, KeyInput, MethodDescriptor, ProjectedRoot, Requester,
    TreeView, Witness,
};
use seam_contracts::{ComeBack, Meanwhile, Unanswered};
use serde::Deserialize;

/// The reuse decision's inputs that are not in brit: what `gate-runner.mjs` passes to
/// `reusableHit` beside the evidence rows.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReuseContext {
    /// `GATE_EVIDENCE`; only `reuse` admits a record.
    pub mode: Option<String>,
    /// The project is listed in `pool-policy.json` `gate_reuse`.
    pub enabled: bool,
    /// This workspace's brit signer, or null when it has no key.
    pub self_id: Option<String>,
    /// The declared inputs are dirty (true), clean (false) or unknown (null).
    pub dirty: Option<bool>,
    /// The commit being gated, against which an attested commit's changes are read.
    pub commit: String,
}

/// One `build find` row.
#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    pub commit: Option<String>,
    pub attestation: Attestation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attestation {
    pub agent_id: Option<String>,
    pub built_at: Option<String>,
    pub success: Option<bool>,
    #[serde(default)]
    pub output_cid: Vec<u8>,
    #[serde(default)]
    pub signature: String,
}

impl Attestation {
    /// `Date.parse(builtAt)` in milliseconds; an absent or unreadable time sorts oldest.
    fn built_at_ms(&self) -> i64 {
        self.built_at
            .as_deref()
            .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
            .map_or(i64::MIN, |at| at.timestamp_millis())
    }

    /// When the attested run ended, in seconds since the Unix epoch.
    pub fn built_at_secs(&self) -> u64 {
        u64::try_from(self.built_at_ms().max(0) / 1000).unwrap_or(0)
    }
}

/// `reusableHit`, rule for rule and in its order: the mode, the policy list and the signer key,
/// then the newest attestation (green, signed by this workspace), clean declared inputs, and a
/// selector check over the attested commit. A refusal says why in `reusableHit`'s own words.
pub fn reusable<'r>(
    rows: &'r [Row],
    ctx: &ReuseContext,
    selector: impl Fn(&str) -> Result<(), String>,
) -> Result<&'r Row, String> {
    if ctx.mode.as_deref() != Some("reuse") {
        return Err("report-only".into());
    }
    if !ctx.enabled {
        return Err("project not enabled for reuse".into());
    }
    let Some(self_id) = &ctx.self_id else {
        return Err("no workspace key".into());
    };
    // Newest first; a stable sort keeps brit's order between equal times, as Array#sort does.
    let mut newest: Vec<&Row> = rows.iter().collect();
    newest.sort_by_key(|row| std::cmp::Reverse(row.attestation.built_at_ms()));
    let Some(row) = newest.first().copied() else {
        return Err("no attestation for this closure yet".into());
    };
    if row.attestation.success != Some(true) {
        return Err("newest attestation is red".into());
    }
    if row.attestation.agent_id.as_deref() != Some(self_id.as_str()) {
        let signer = row.attestation.agent_id.as_deref().unwrap_or("unsigned");
        return Err(format!(
            "signer {} is not this workspace",
            &signer[..signer.len().min(12)]
        ));
    }
    match ctx.dirty {
        Some(false) => {}
        Some(true) => return Err("declared inputs dirty".into()),
        None => return Err("working tree state unknown".into()),
    }
    selector(row.commit.as_deref().unwrap_or(""))?;
    Ok(row)
}

/// `reuseSelector`, narrowed: an attested commit whose tree differs from the gated commit's is
/// refused rather than judged, because ark does not consult the rakia dependency oracle that
/// `gate-runner.mjs` asks. Stricter, never looser: an unchanged tree passes in both.
pub fn unchanged_since(repo: &Path, gate_commit: &str) -> impl Fn(&str) -> Result<(), String> {
    let repo = repo.to_path_buf();
    let gate_commit = gate_commit.to_string();
    move |attested: &str| {
        if attested.is_empty() {
            return Err("attested commit unreadable".into());
        }
        let diff = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["diff", "--name-only", attested, &gate_commit])
            .output()
            .map_err(|_| "attested commit unreadable".to_string())?;
        if !diff.status.success() {
            return Err("attested commit unreadable".into());
        }
        let changed = String::from_utf8_lossy(&diff.stdout);
        match changed.lines().find(|line| !line.is_empty()) {
            None => Ok(()),
            Some(first) => Err(format!(
                "changed since attested commit ({first}); ark reads no rakia oracle, so only an \
                 unchanged tree is reused"
            )),
        }
    }
}

/// `brit-build-ref` as `gate-evidence.mjs::resolveBin` finds it: the first set of
/// `BRIT_BUILD_REF_BIN` and `BRIT_BUILD_REF` (relative to the working directory), else `PATH`.
pub fn resolve_build_ref() -> Option<PathBuf> {
    let executable = |path: &Path| {
        path.metadata()
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    };
    for key in ["BRIT_BUILD_REF_BIN", "BRIT_BUILD_REF"] {
        let declared = env::var(key).unwrap_or_default();
        let declared = declared.trim();
        if declared.is_empty() {
            continue;
        }
        let path = env::current_dir().ok()?.join(declared);
        return executable(&path).then_some(path);
    }
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join("brit-build-ref"))
        .find(|path| executable(path))
}

/// brit's notes ref for a build step: `refs/notes/brit/build/<step>`, the characters git forbids
/// in a ref name percent-encoded as brit's `BritRefManager::encode_segment` encodes them.
pub fn build_ref_name(step: &str) -> String {
    let mut out = String::with_capacity(step.len());
    for c in step.chars() {
        match c {
            ':' => out.push_str("%3A"),
            '@' => out.push_str("%40"),
            '?' => out.push_str("%3F"),
            '*' => out.push_str("%2A"),
            '[' => out.push_str("%5B"),
            '\\' => out.push_str("%5C"),
            '^' => out.push_str("%5E"),
            '~' => out.push_str("%7E"),
            ' ' => out.push_str("%20"),
            c if c.is_ascii_control() => out.push_str(&format!("%{:02X}", u32::from(c))),
            c => out.push(c),
        }
    }
    format!("refs/notes/brit/build/{}", out.replace("..", "%2E%2E"))
}

/// One `unverifiable: <commit> <reason>` line on a brit read's stderr: a stored note brit refused
/// to verify. Every reader treats it as absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unverifiable {
    pub commit: String,
    pub reason: String,
}

/// The `unverifiable:` lines of a brit read's stderr, in order. Every other line (the miss line,
/// a usage error) is left alone.
pub fn parse_unverifiable(stderr: &str) -> Vec<Unverifiable> {
    stderr
        .lines()
        .filter_map(|line| line.strip_prefix("unverifiable: "))
        .map(|rest| {
            let (commit, reason) = rest.split_once(' ').unwrap_or((rest, ""));
            Unverifiable {
                commit: commit.to_string(),
                reason: reason.to_string(),
            }
        })
        .collect()
}

/// What `build find` answered: the rows brit verified, and the notes it refused.
#[derive(Debug, Clone, Default)]
pub struct Found {
    pub rows: Vec<Row>,
    pub unverifiable: Vec<Unverifiable>,
}

/// `build find` for a step and closure key. The rows are brit's stdout, the verified read; each
/// `unverifiable:` line on its stderr names a note brit refused, which is absent here (a row on
/// that commit is dropped) and returned for the look-up to say once ([`say_unverifiable`]). This
/// prints nothing itself, because one `ark task` reads the same closure twice (the look-up, then
/// `record`). A read git could not answer (exit 1) is an error, never an absence.
pub fn find_rows(bin: &Path, repo: &Path, step: &str, inputs_hash: &str) -> Result<Found, String> {
    let out = Command::new(bin)
        .arg("--repo")
        .arg(repo)
        .args([
            "build",
            "find",
            "--step",
            step,
            "--inputs-hash",
            inputs_hash,
        ])
        .current_dir(repo)
        .output()
        .map_err(|error| format!("build find: {error}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        let last = stderr.trim().lines().last().unwrap_or("").to_string();
        return Err(format!("build find failed: {last}"));
    }
    let rows: Vec<Row> =
        serde_json::from_slice(&out.stdout).map_err(|_| "build find: not JSON rows".to_string())?;
    let unverifiable = parse_unverifiable(&stderr);
    let rows = rows
        .into_iter()
        .filter(|row| {
            !unverifiable
                .iter()
                .any(|note| row.commit.as_deref() == Some(note.commit.as_str()))
        })
        .collect();
    Ok(Found { rows, unverifiable })
}

/// Say each note brit refused for `step`, once, with its ref and reason: the one place a
/// `find` result is voiced, so a closure read twice is said once.
pub fn say_unverifiable(step: &str, found: &Found) {
    let refused = build_ref_name(step);
    for note in &found.unverifiable {
        eprintln!(
            "ark brit: unverifiable attestation on {refused} at {}: {}; it vetoes reuse until replaced or removed",
            note.commit, note.reason
        );
    }
}

/// Whether a look-up consults the signer at all: `reusableHit` refuses on the mode and the
/// policy list before it reads the key, so a report-only or unlisted project asks brit for no
/// signer and `signer --json` is spawned only when its answer can decide anything.
pub fn asks_signer(ctx: &ReuseContext) -> bool {
    ctx.mode.as_deref() == Some("reuse") && ctx.enabled
}

/// What `brit-build-ref signer --json` says this repository's signer is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signer {
    /// brit answered: the key's agent id, or none when the repository keeps no key.
    Known(Option<String>),
    /// This brit predates `signer` (clap's usage error, exit 2): the caller's own reading of the
    /// key file stands.
    Unsupported,
    /// brit knows `signer` but could not answer; its first stderr line.
    Failed(String),
}

/// clap's first stderr line when the pinned brit has no `signer` subcommand.
const SIGNER_UNKNOWN: &str = "error: unrecognized subcommand 'signer'";

/// `signer --json`, which never mints a key. An error is why brit could not be run at all.
pub fn signer(bin: &Path, repo: &Path) -> Result<Signer, String> {
    let out = Command::new(bin)
        .arg("--repo")
        .arg(repo)
        .args(["signer", "--json"])
        .current_dir(repo)
        .output()
        .map_err(|error| format!("signer: {error}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let first = stderr.lines().next().unwrap_or("").trim().to_string();
    if out.status.code() == Some(2) && first.starts_with(SIGNER_UNKNOWN) {
        return Ok(Signer::Unsupported);
    }
    if !out.status.success() {
        return Ok(Signer::Failed(if first.is_empty() {
            format!("signer: {}", out.status)
        } else {
            first
        }));
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Answer {
        agent_id: Option<String>,
    }
    Ok(match serde_json::from_slice::<Answer>(&out.stdout) {
        Ok(answer) => Signer::Known(answer.agent_id),
        Err(_) => Signer::Failed("signer: not JSON".to_string()),
    })
}

/// This workspace's signer: brit's answer over the caller's own reading of the key file. The
/// caller's reading stands only under a brit that predates `signer`; a brit that knows the key
/// and cannot load it leaves the workspace with no key, said once.
pub fn self_id_from(answer: Signer, callers: Option<String>) -> Option<String> {
    match answer {
        Signer::Known(id) => id,
        Signer::Unsupported => callers,
        Signer::Failed(why) => {
            eprintln!("ark brit: {why}; this workspace has no key");
            None
        }
    }
}

/// The brit-backed index for one gate closure.
pub struct BritIndex {
    pub build_ref: Option<PathBuf>,
    pub repo: PathBuf,
    /// `gate:<project>`, the attestation step.
    pub step: String,
    /// brit's `--inputs-hash`: the closure key, a dedup key.
    pub inputs_hash: String,
    pub input: TreeView,
    pub projected: ProjectedRoot,
    pub method: MethodDescriptor,
    pub ctx: ReuseContext,
    /// What the last [`DerivationIndex::lookup`] saw beyond its answer, for the caller to say.
    pub last: Mutex<Option<Note>>,
}

/// Why the last look-up answered as it did: the trait answers only found or not, and a person
/// wants `reusableHit`'s reason, the admitted row's commit and time, and whether anything is held.
#[derive(Debug, Clone, Default)]
pub struct Note {
    /// `reusableHit`'s refusal, or why brit could not be read.
    pub why: Option<String>,
    pub row: Option<Row>,
    pub held: bool,
}

/// What a look-up decided, with `reusableHit`'s reason when no record was admitted.
pub struct Decided {
    pub lookup: DerivationLookup,
    pub why: Option<String>,
    /// The admitted row, for the completion it carries.
    pub row: Option<Row>,
    /// Some attestation exists for this closure, admitted or not.
    pub held: bool,
}

impl BritIndex {
    /// Ruling 7's answer when brit could not be read: come back once `brit-build-ref` answers,
    /// and meanwhile run the gate here. Its reason is [`BritIndex::decide`]'s error.
    pub fn unreachable() -> Unanswered<DerivationUnanswered> {
        Unanswered::new(
            DerivationUnanswered::IndexUnreachable,
            ComeBack::OnEvent {
                event: "brit-build-ref-available".into(),
            },
            Meanwhile::RunHere,
        )
    }

    fn bin(&self) -> Result<&Path, String> {
        self.build_ref.as_deref().ok_or_else(|| {
            "brit-build-ref not on BRIT_BUILD_REF_BIN/BRIT_BUILD_REF or PATH".to_string()
        })
    }

    /// `build find` for this closure's step and key: the verified rows, with every note brit
    /// refused absent ([`find_rows`]); [`BritIndex::decide`] names them, `record` reads silently.
    pub fn find(&self) -> Result<Found, String> {
        find_rows(self.bin()?, &self.repo, &self.step, &self.inputs_hash)
    }

    /// This workspace's signer, from `signer --json` first and from the reuse context's own
    /// reading of the key file only under a brit that has no `signer` ([`self_id_from`]). Asked
    /// only when the answer can decide anything ([`asks_signer`]).
    pub fn self_id(&self) -> Result<Option<String>, String> {
        Ok(self_id_from(
            signer(self.bin()?, &self.repo)?,
            self.ctx.self_id.clone(),
        ))
    }

    fn key(&self) -> Result<DerivationKey, String> {
        Ok(DerivationKey {
            method: self.method.cid().map_err(|error| error.to_string())?,
            input: KeyInput::Projected(self.projected.clone()),
        })
    }

    /// A brit row as the derivation record it is: (method, projected input) → the stored verdict
    /// node, computed by the row's signer, witnessed by its signature.
    fn record_of(&self, row: &Row) -> Result<DerivationRecord, String> {
        let output = cid::Cid::try_from(row.attestation.output_cid.as_slice())
            .map_err(|error| format!("attested output: {error}"))?;
        Ok(DerivationRecord {
            key: self.key()?,
            output: BlobCid::parse(&output.to_string()).map_err(|error| error.to_string())?,
            computed_by: row.attestation.agent_id.clone().unwrap_or_default(),
            basis: DerivationBasis::Computed,
            deterministic: self.method.deterministic,
            witness: Witness::Signature {
                sig: row.attestation.signature.clone(),
            },
        })
    }

    /// The look-up with its reason. Another input or method than the one this index was opened
    /// for is not recorded here. An error is why brit could not be read (the index is
    /// unreachable, [`BritIndex::unreachable`]).
    pub fn decide(&self, at: &TreeView, method: &MethodDescriptor) -> Result<Decided, String> {
        let other = |why: &str| Decided {
            lookup: DerivationLookup::NotRecorded,
            why: Some(why.to_string()),
            row: None,
            held: false,
        };
        if at != &self.input {
            return Ok(other(
                "this index answers only the closure it was opened on",
            ));
        }
        if method != &self.method {
            return Ok(other(
                "this index answers only the method it was opened for",
            ));
        }
        let found = self.find()?;
        say_unverifiable(&self.step, &found);
        let held = !found.rows.is_empty();
        // D5 (steward-space spec :850): a red of any provenance or verification state vetoes.
        // brit cannot say what closure or verdict a note it will not vouch for carried, so one
        // unverifiable note under this step is a possible red: nothing is reused, the task runs.
        if let Some(note) = found.unverifiable.first() {
            let why = format!(
                "unverifiable note under this step at {}: {} (a red of any verification state \
                 vetoes; replace or remove the note)",
                note.commit, note.reason
            );
            return Ok(Decided {
                held,
                ..other(&why)
            });
        }
        let self_id = if asks_signer(&self.ctx) {
            self.self_id()?
        } else {
            self.ctx.self_id.clone()
        };
        let ctx = ReuseContext {
            self_id,
            ..self.ctx.clone()
        };
        match reusable(
            &found.rows,
            &ctx,
            unchanged_since(&self.repo, &self.ctx.commit),
        ) {
            Ok(row) => {
                let record = self.record_of(row)?;
                Ok(Decided {
                    lookup: DerivationLookup::Found(vec![record]),
                    why: None,
                    row: Some(row.clone()),
                    held,
                })
            }
            // `ark task` prints this line.
            Err(why) => Ok(Decided {
                held,
                ..other(&why)
            }),
        }
    }
}

#[async_trait]
impl DerivationIndex for BritIndex {
    /// A local store on this device: `who` is the device itself, and nothing here is served.
    async fn lookup(
        &self,
        _who: &Requester,
        at: &TreeView,
        method: &MethodDescriptor,
    ) -> Result<DerivationLookup, Unanswered<DerivationUnanswered>> {
        let decided = self.decide(at, method);
        let (answer, note) = match decided {
            Ok(decided) => (
                Ok(decided.lookup),
                Note {
                    why: decided.why,
                    row: decided.row,
                    held: decided.held,
                },
            ),
            Err(why) => (
                Err(Self::unreachable()),
                Note {
                    why: Some(why),
                    ..Note::default()
                },
            ),
        };
        if let Ok(mut last) = self.last.lock() {
            *last = Some(note);
        }
        answer
    }

    async fn lookup_many(
        &self,
        who: &Requester,
        at: &TreeView,
        methods: &[MethodDescriptor],
    ) -> Result<BTreeMap<BlobCid, DerivationLookup>, Unanswered<DerivationUnanswered>> {
        let mut answers = BTreeMap::new();
        for method in methods {
            let cid = method.cid().map_err(|_| Self::unreachable())?;
            answers.insert(cid, self.lookup(who, at, method).await?);
        }
        Ok(answers)
    }

    /// Writes nothing: `brit-build-ref build put`, run by `gate-runner.mjs::recordEvidence`, is
    /// the one writer. Confirms that a green attestation for this closure, by the record's agent
    /// and with its output, already stands, and otherwise refuses naming where it must be written.
    async fn record(&self, record: &DerivationRecord) -> eprfs_core::Result<()> {
        let refuse = |why: String| {
            EprfsError::Storage(format!(
                "{why}; this index writes no second store: the attestation is \
                 `brit-build-ref build put --step {} --inputs-hash {}` \
                 (genesis/orchestrator/gate-evidence.mjs recordEvidence)",
                self.step, self.inputs_hash
            ))
        };
        if record.deterministic != self.method.deterministic {
            return Err(refuse(
                "the record's determinism is not its method's".into(),
            ));
        }
        if self.key().map_err(refuse)? != record.key {
            return Err(refuse("the record is keyed for another closure".into()));
        }
        let found = self.find().map_err(refuse)?;
        let stands = found.rows.iter().any(|row| {
            row.attestation.success == Some(true)
                && row.attestation.agent_id.as_deref() == Some(record.computed_by.as_str())
                && self
                    .record_of(row)
                    .is_ok_and(|held| held.output == record.output)
        });
        if stands {
            Ok(())
        } else {
            Err(refuse(
                "no green attestation by this agent with this output".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(agent: &str, at: &str, success: bool, commit: &str) -> Row {
        Row {
            commit: Some(commit.into()),
            attestation: Attestation {
                agent_id: Some(agent.into()),
                built_at: Some(at.into()),
                success: Some(success),
                output_cid: Vec::new(),
                signature: String::new(),
            },
        }
    }

    fn ctx() -> ReuseContext {
        ReuseContext {
            mode: Some("reuse".into()),
            enabled: true,
            self_id: Some("me".into()),
            dirty: Some(false),
            commit: "HEAD".into(),
        }
    }

    fn same(_: &str) -> Result<(), String> {
        Ok(())
    }

    /// A stand-in `brit-build-ref` that prints exactly what the contract says and exits as it
    /// does; written where the test runs, never in the tree.
    fn stub(dir: &Path, name: &str, stdout: &str, stderr: &str, code: i32) -> PathBuf {
        let mut script = String::from("#!/bin/sh\n");
        if !stdout.is_empty() {
            script.push_str(&format!("cat <<'BRIT_OUT'\n{stdout}\nBRIT_OUT\n"));
        }
        if !stderr.is_empty() {
            script.push_str(&format!("cat >&2 <<'BRIT_ERR'\n{stderr}\nBRIT_ERR\n"));
        }
        script.push_str(&format!("exit {code}\n"));
        let path = dir.join(name);
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    const OLD_SIGNER_STDERR: &str = "error: unrecognized subcommand 'signer'\n\n\
        Usage: brit-build-ref [OPTIONS] <COMMAND>\n\n\
        For more information, try '--help'.";

    fn find_row(commit: &str) -> String {
        format!(
            r#"{{"commit":"{commit}","attestation":{{"agentId":"6d65","buildDurationMs":1200,"builtAt":"2026-10-26T10:00:00Z","hardwareProfile":"x","inputsHash":"k","manifestCid":[1,85,18,32],"outputCid":[1,85,18,32],"signature":"00","stepName":"gate:x","success":true}}}}"#
        )
    }

    #[test]
    fn a_note_brit_names_unverifiable_is_absent_and_the_rest_stand() {
        let dir = tempfile::tempdir().unwrap();
        let stdout = format!("[{},{}]", find_row("aaaa1111"), find_row("bbbb2222"));
        let stderr = "unverifiable: bbbb2222 signature does not verify against agentId \"6d65\"\n\
                      unverifiable: cccc3333 stepName \"gate:y\" does not match this ref's \"gate:x\"";
        let bin = stub(dir.path(), "brit-build-ref", &stdout, stderr, 0);
        let found = find_rows(&bin, dir.path(), "gate:x", "k").unwrap();
        let commits: Vec<&str> = found
            .rows
            .iter()
            .map(|row| row.commit.as_deref().unwrap())
            .collect();
        assert_eq!(commits, ["aaaa1111"], "a refused note is never a hit");
        assert_eq!(
            found.unverifiable,
            [
                Unverifiable {
                    commit: "bbbb2222".into(),
                    reason: "signature does not verify against agentId \"6d65\"".into(),
                },
                Unverifiable {
                    commit: "cccc3333".into(),
                    reason: "stepName \"gate:y\" does not match this ref's \"gate:x\"".into(),
                },
            ]
        );
        assert_eq!(found.rows[0].attestation.output_cid, [1, 85, 18, 32]);
    }

    #[test]
    fn a_find_miss_is_empty_and_a_failed_git_read_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let miss = stub(dir.path(), "miss", "[]", "", 0);
        let found = find_rows(&miss, dir.path(), "gate:x", "k").unwrap();
        assert!(found.rows.is_empty() && found.unverifiable.is_empty());
        let failed = stub(
            dir.path(),
            "failed",
            "",
            "Error: git command failed: git notes --ref refs/notes/brit/build/gate%3Ax list: \
             fatal: not a git repository",
            1,
        );
        let why = find_rows(&failed, dir.path(), "gate:x", "k").unwrap_err();
        assert_eq!(
            why,
            "build find failed: Error: git command failed: git notes --ref \
             refs/notes/brit/build/gate%3Ax list: fatal: not a git repository"
        );
    }

    #[test]
    fn parse_unverifiable_reads_only_its_lines() {
        let lines = "no build ref found for step=gate:x commit=HEAD\n\
                     unverifiable: aaaa1111 does not decode: unknown field `trusted`\n";
        assert_eq!(
            parse_unverifiable(lines),
            [Unverifiable {
                commit: "aaaa1111".into(),
                reason: "does not decode: unknown field `trusted`".into(),
            }]
        );
        assert!(parse_unverifiable("[]\n").is_empty());
    }

    #[test]
    fn build_ref_name_encodes_as_brit_does() {
        assert_eq!(build_ref_name("gate:x"), "refs/notes/brit/build/gate%3Ax");
        assert_eq!(
            build_ref_name("a b..c@d"),
            "refs/notes/brit/build/a%20b%2E%2Ec%40d"
        );
    }

    #[test]
    fn signer_json_is_read_first() {
        let dir = tempfile::tempdir().unwrap();
        let id = "6d65".repeat(16);
        let keyed = stub(
            dir.path(),
            "keyed",
            &format!(r#"{{"agentId":"{id}"}}"#),
            "",
            0,
        );
        assert_eq!(
            signer(&keyed, dir.path()).unwrap(),
            Signer::Known(Some(id.clone()))
        );
        let keyless = stub(dir.path(), "keyless", r#"{"agentId":null}"#, "", 0);
        assert_eq!(signer(&keyless, dir.path()).unwrap(), Signer::Known(None));
        assert_eq!(
            self_id_from(Signer::Known(Some(id.clone())), Some("file".into())),
            Some(id)
        );
        assert_eq!(self_id_from(Signer::Known(None), Some("file".into())), None);
    }

    #[test]
    fn a_brit_without_signer_leaves_the_callers_reading_of_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let old = stub(dir.path(), "old", "", OLD_SIGNER_STDERR, 2);
        assert_eq!(signer(&old, dir.path()).unwrap(), Signer::Unsupported);
        assert_eq!(
            self_id_from(Signer::Unsupported, Some("file".into())),
            Some("file".into())
        );
        assert_eq!(self_id_from(Signer::Unsupported, None), None);
    }

    #[test]
    fn a_brit_that_cannot_load_the_key_leaves_no_key() {
        let dir = tempfile::tempdir().unwrap();
        let broken = stub(
            dir.path(),
            "broken",
            "",
            "Error: cannot load the repository signer key\n\nCaused by:\n    seed has 31 bytes",
            1,
        );
        let answer = signer(&broken, dir.path()).unwrap();
        assert_eq!(
            answer,
            Signer::Failed("Error: cannot load the repository signer key".into())
        );
        assert_eq!(self_id_from(answer, Some("file".into())), None);
    }

    /// The signer is consulted only once the mode and the policy list would let it decide: a
    /// report-only or unlisted project spawns no `signer --json`, and its context stands as is.
    #[test]
    fn the_signer_is_asked_only_under_reuse_for_an_enabled_project() {
        assert!(asks_signer(&ctx()));
        assert!(!asks_signer(&ReuseContext {
            mode: None,
            ..ctx()
        }));
        assert!(!asks_signer(&ReuseContext {
            mode: Some("report".into()),
            ..ctx()
        }));
        assert!(!asks_signer(&ReuseContext {
            enabled: false,
            ..ctx()
        }));
    }

    /// Each refusal `reusableHit` makes, in its order and its words.
    #[test]
    fn reuse_follows_reusable_hit_rule_for_rule() {
        let green = [row("me", "2026-10-26T10:00:00Z", true, "c1")];
        let refused = |rows: &[Row], ctx: &ReuseContext| reusable(rows, ctx, same).unwrap_err();
        assert_eq!(
            refused(
                &green,
                &ReuseContext {
                    mode: None,
                    ..ctx()
                }
            ),
            "report-only"
        );
        assert_eq!(
            refused(
                &green,
                &ReuseContext {
                    enabled: false,
                    ..ctx()
                }
            ),
            "project not enabled for reuse"
        );
        assert_eq!(
            refused(
                &green,
                &ReuseContext {
                    self_id: None,
                    ..ctx()
                }
            ),
            "no workspace key"
        );
        assert_eq!(refused(&[], &ctx()), "no attestation for this closure yet");
        // The newest decides: a red after a green is not reusable.
        let red_after = [
            row("me", "2026-10-26T10:00:00Z", true, "c1"),
            row("me", "2026-10-26T11:00:00Z", false, "c2"),
        ];
        assert_eq!(refused(&red_after, &ctx()), "newest attestation is red");
        let theirs = [row("0123456789abcdef", "2026-10-26T10:00:00Z", true, "c1")];
        assert_eq!(
            refused(&theirs, &ctx()),
            "signer 0123456789ab is not this workspace"
        );
        assert_eq!(
            refused(
                &green,
                &ReuseContext {
                    dirty: Some(true),
                    ..ctx()
                }
            ),
            "declared inputs dirty"
        );
        assert_eq!(
            refused(
                &green,
                &ReuseContext {
                    dirty: None,
                    ..ctx()
                }
            ),
            "working tree state unknown"
        );
        assert_eq!(
            reusable(&green, &ctx(), |_| Err("changed".into())).unwrap_err(),
            "changed"
        );
        let green_after = [
            row("me", "2026-10-26T10:00:00Z", false, "c1"),
            row("me", "2026-10-26T11:00:00Z", true, "c2"),
        ];
        let admitted = reusable(&green_after, &ctx(), same).unwrap();
        assert_eq!(admitted.commit.as_deref(), Some("c2"));
    }

    #[test]
    fn an_attested_commit_with_another_tree_is_never_reused() {
        let repo = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(repo.path())
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}");
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        };
        git(&["init", "-q"]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "a",
        ]);
        let first = git(&["rev-parse", "HEAD"]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "b",
        ]);
        let empty = git(&["rev-parse", "HEAD"]);
        std::fs::write(repo.path().join("f"), "x").unwrap();
        git(&["add", "f"]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "c",
        ]);
        let changed = git(&["rev-parse", "HEAD"]);
        assert!(unchanged_since(repo.path(), &empty)(&first).is_ok());
        let refused = unchanged_since(repo.path(), &changed)(&first).unwrap_err();
        assert!(
            refused.contains("changed since attested commit (f)"),
            "{refused}"
        );
        assert_eq!(
            unchanged_since(repo.path(), &changed)("").unwrap_err(),
            "attested commit unreadable"
        );
    }
}
