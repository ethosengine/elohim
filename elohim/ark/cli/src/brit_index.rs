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

    /// `build find` for this closure's step and key.
    pub fn find(&self) -> Result<Vec<Row>, String> {
        let bin = self.build_ref.as_ref().ok_or_else(|| {
            "brit-build-ref not on BRIT_BUILD_REF_BIN/BRIT_BUILD_REF or PATH".to_string()
        })?;
        let out = Command::new(bin)
            .arg("--repo")
            .arg(&self.repo)
            .args(["build", "find", "--step", &self.step, "--inputs-hash"])
            .arg(&self.inputs_hash)
            .current_dir(&self.repo)
            .output()
            .map_err(|error| format!("build find: {error}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let last = stderr.trim().lines().last().unwrap_or("").to_string();
            return Err(format!("build find failed: {last}"));
        }
        serde_json::from_slice(&out.stdout).map_err(|_| "build find: not JSON rows".to_string())
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
        let rows = self.find()?;
        let held = !rows.is_empty();
        match reusable(
            &rows,
            &self.ctx,
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
        let rows = self.find().map_err(refuse)?;
        let stands = rows.iter().any(|row| {
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
