//! `epr govern` — evaluate ONE PROSPECTIVE write and print the decision as JSON.
//!
//! WHY THIS EXISTS. `epr check` answers "what is wrong with the tree as it
//! stands": it derives content from disk or from a git revision. An authoring
//! gate asks a different question — *if I write THIS content to THIS path, what
//! is the verdict?* — about content that is not on disk yet, at a path that may
//! not exist. `check` cannot answer it; pointed at a not-yet-written file it
//! would evaluate the pre-edit state and confidently return the wrong answer.
//!
//! The library has always been able to answer it (`GovernanceWrite` carries
//! `content` explicitly). What was missing was a way to ASK from outside the
//! process. Until this verb existed, "make the Python hook a client of the Rust
//! evaluator" was not a wiring task — there was no socket to wire to, and the
//! two evaluators had no choice but to remain two.
//!
//! CONTRACT. The decision lives in the PAYLOAD, never in the exit code:
//!
//!   exit 0 — the evaluator ran; read `decision` (`permit` | `refuse` | `refer`)
//!   exit 2 — the evaluator did NOT run (bad arguments, unreadable repo, …)
//!
//! A client must be able to tell "governance says refuse" from "governance did
//! not answer", because those demand opposite responses: the first is a verdict
//! to honour, the second is an absence to route on. Collapsing them into one
//! exit code is how a gate ends up silently permitting whenever its evaluator
//! breaks.

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use elohim_epr_rea::{ActorClaim, ActorStore, SidecarActorStore};
use eprfs_meta::{evaluate_path_with, hex_lower, resolve_decision, GovernanceWrite};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    error::{Error, Result},
    repository_validators::ElohimRepositoryValidators,
};

/// The actor sidecar, relative to the repo root. Checked for EXISTENCE before it is opened:
/// [`SidecarActorStore::open`] creates the tree, and a decision that merely READ the identity
/// plane must not leave one behind in a repository that never had one.
const ACTOR_LOG_REL: &str = ".eprfs/status/actors.jsonl";

/// Content address of THIS evaluator build.
///
/// A decision that does not name its evaluator cannot be disputed precisely:
/// "the native evaluator said refuse" is a claim about a moving target, since
/// the next build may say otherwise for reasons no record captured. Naming the
/// exact artifact turns a disagreement into something re-derivable — a second
/// host can state which build it disputes, and a third can fetch that build and
/// check.
///
/// The address is over the BINARY, not its inputs: the manifests and registries
/// are already named by the decision, and what is in question here is the thing
/// that read them. Hashing is memoized, so a one-shot `govern` process pays it
/// once (~60ms on the debug build, less on release).
fn evaluator_identity() -> Value {
    static IDENTITY: OnceLock<Value> = OnceLock::new();
    IDENTITY
        .get_or_init(|| {
            let cid = std::env::current_exe()
                .ok()
                .and_then(|path| std::fs::read(path).ok())
                .map(|bytes| {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    format!("sha256:{}", hex_lower(&hasher.finalize()))
                });
            json!({
                "id": "elohim-epr-cli",
                "version": env!("CARGO_PKG_VERSION"),
                // `None` when the executable cannot be read — honest absence,
                // never a placeholder that would read as a real address.
                "cid": cid,
            })
        })
        .clone()
}

/// Parsed `govern` invocation.
#[derive(Debug, Default)]
struct GovernArgs {
    path: Option<String>,
    is_new: bool,
    is_new_subdir: bool,
    content: Option<String>,
    /// True when the caller explicitly declared the content (even as empty).
    content_declared: bool,
    /// The run whose registered actor claim should stamp this decision. `None` means the caller
    /// never asked — a different claim from "we asked and nobody had claimed".
    session: Option<String>,
}

fn parse(args: &[String]) -> Result<GovernArgs> {
    let mut parsed = GovernArgs::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--path" => {
                parsed.path = Some(
                    args.get(index + 1)
                        .ok_or_else(|| Error::InvalidArguments("--path needs a value".into()))?
                        .clone(),
                );
                index += 2;
            }
            "--new" => {
                parsed.is_new = true;
                index += 1;
            }
            "--new-subdir" => {
                parsed.is_new_subdir = true;
                index += 1;
            }
            "--content-stdin" => {
                let mut buffer = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer).map_err(
                    |source| Error::Read {
                        path: PathBuf::from("<stdin>"),
                        source,
                    },
                )?;
                parsed.content = Some(buffer);
                parsed.content_declared = true;
                index += 1;
            }
            "--content-file" => {
                let file = args
                    .get(index + 1)
                    .ok_or_else(|| Error::InvalidArguments("--content-file needs a path".into()))?;
                let body = std::fs::read(file).map_err(|source| Error::Read {
                    path: PathBuf::from(file),
                    source,
                })?;
                parsed.content = Some(String::from_utf8_lossy(&body).into_owned());
                parsed.content_declared = true;
                index += 2;
            }
            "--session" => {
                parsed.session = Some(
                    args.get(index + 1)
                        .ok_or_else(|| Error::InvalidArguments("--session needs a value".into()))?
                        .clone(),
                );
                index += 2;
            }
            other => {
                return Err(Error::InvalidArguments(format!(
                    "unknown govern argument `{other}`"
                )))
            }
        }
    }
    Ok(parsed)
}

/// Who was acting when this decision was asked for, as the identity sidecar answers it.
///
/// `source` is the load-bearing field, and it carries exactly two values because there are only
/// two honest states: a claim was found (`"claim"`), or one was not (`"unclaimed"`). A record
/// that merely named a claimed identity could not be read back later without knowing whether the
/// blank case meant "nobody registered" or "the stamp gave up".
///
/// EVERY read failure collapses to `unclaimed` — a missing sidecar, an unreadable file, a
/// tampered line whose CID no longer matches its payload. This is the decision authority: a
/// governance verdict that could not be produced because the identity plane had one bad line
/// would trade the answer for the annotation. The decision lands; the missing attribution is
/// visible in the payload rather than in a non-answer. The consequence is deliberate and worth
/// naming — a tampered claim reads as `unclaimed`, never as the identity the tamperer wrote.
fn actor_stamp(root: &Path, session: &str) -> Value {
    match claimed_for(root, session) {
        Some((cid, claim)) => json!({
            "claimCid": cid.to_string(),
            "claimed": claim.claimed.0,
            "session": session,
            "definitionCid": claim.definition_cid,
            "source": "claim",
        }),
        None => json!({
            "claimCid": Value::Null,
            "claimed": Value::Null,
            "session": session,
            "definitionCid": Value::Null,
            "source": "unclaimed",
        }),
    }
}

/// The claim current for `session`, or `None` for every reason there is not one.
fn claimed_for(root: &Path, session: &str) -> Option<(cid::Cid, ActorClaim)> {
    if !root.join(ACTOR_LOG_REL).exists() {
        return None;
    }
    SidecarActorStore::open(root)
        .and_then(|store| store.current_for(session))
        .ok()
        .flatten()
}

/// Evaluate the prospective write and return the decision payload.
pub fn evaluate(root: &Path, args: &[String]) -> Result<Value> {
    let parsed = parse(args)?;
    let Some(rel) = parsed.path else {
        return Err(Error::InvalidArguments("govern needs --path".into()));
    };

    let target = if Path::new(&rel).is_absolute() {
        PathBuf::from(&rel)
    } else {
        root.join(&rel)
    };
    // Report the path the way the caller named it, but resolve governance
    // against the repo-relative form so cascade lookup behaves identically for
    // absolute and relative invocations.
    let normalized = target
        .strip_prefix(root)
        .unwrap_or(Path::new(&rel))
        .to_string_lossy()
        .replace('\\', "/");

    // Content the caller did not declare stays `None` rather than being read off
    // disk: "the caller supplied no content" and "the file is empty" are
    // different claims, and content-triggered rules must abstain on the first.
    let content = if parsed.content_declared {
        parsed.content
    } else {
        None
    };
    // Prior content IS read from disk — that is exactly what "prior" means for a
    // prospective write, and its absence is meaningful (a genuinely new file).
    let prior_content = std::fs::read(&target)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let write = GovernanceWrite {
        path: normalized.clone(),
        content,
        prior_content,
        is_new: parsed.is_new,
        is_new_subdir: parsed.is_new_subdir,
    };

    let evaluation = evaluate_path_with(root, &target, &write, &ElohimRepositoryValidators)?;
    let resolved = resolve_decision(&evaluation.verdicts);

    // The winning verdict's prose, so a client can surface WHY without
    // re-deriving the severity ordering.
    let reason = resolved
        .rule_id
        .as_ref()
        .and_then(|id| {
            evaluation
                .verdicts
                .iter()
                .find(|verdict| &verdict.rule_id == id)
        })
        .or_else(|| {
            // Manifest-integrity routes carry no rule id; there is exactly one
            // verdict in that case.
            (resolved.decision != "permit")
                .then(|| evaluation.verdicts.first())
                .flatten()
        })
        .map(|verdict| verdict.reason.clone());

    let mut decision = json!({
        "path": normalized,
        "evaluator": evaluator_identity(),
        "decision": resolved.decision,
        "winningClass": resolved.winning_class,
        "ruleId": resolved.rule_id,
        "referReason": resolved.refer_reason,
        "reason": reason,
        "verdicts": evaluation.verdicts,
        "diagnostics": evaluation.diagnostics,
    });

    // Additive: the key exists only when the caller declared a session. An `actor` stamp on every
    // decision would assert "we looked and found nobody" for callers that never asked, and every
    // existing client would start reading an answer to a question it did not pose.
    if let (Some(session), Some(object)) = (parsed.session.as_deref(), decision.as_object_mut()) {
        object.insert("actor".to_string(), actor_stamp(root, session));
    }

    Ok(decision)
}

pub fn usage() -> &'static str {
    "usage: epr [--repo PATH] govern --path REL [--new] [--new-subdir] [--content-stdin | --content-file PATH] [--session ID]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const SESSION: &str = "session-abc123";
    const CLAIMED: &str = "agent:rust-architect@opus-5";

    fn git(root: &Path, args: &[&str]) {
        let out = crate::process::build_command("git", args, root, &[])
            .env("GIT_AUTHOR_NAME", "Fixture Author")
            .env("GIT_AUTHOR_EMAIL", "author@example.test")
            .env("GIT_COMMITTER_NAME", "Fixture Author")
            .env("GIT_COMMITTER_EMAIL", "author@example.test")
            .output()
            .expect("git runs");
        assert!(out.status.success(), "git {args:?} failed");
    }

    fn write(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    /// A committed tempdir repo — `claim` dates itself from HEAD, so a fixture without one could
    /// not register anything to stamp with.
    fn fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "notes/subject.md", "fixture\n");
        write(
            dir.path(),
            ".epr-meta/elohim/packages/agents/rust-architect.json",
            "{\n  \"kind\": \"AgentPackage\",\n  \"metadata\": { \"id\": \"rust-architect\" }\n}\n",
        );
        git(dir.path(), &["init", "-q"]);
        git(dir.path(), &["add", "-A"]);
        git(dir.path(), &["commit", "-q", "-m", "fixture"]);
        dir
    }

    fn decide(root: &Path, session: Option<&str>) -> Value {
        let mut args = vec!["--path".to_string(), "notes/subject.md".to_string()];
        if let Some(session) = session {
            args.push("--session".to_string());
            args.push(session.to_string());
        }
        evaluate(root, &args).expect("the evaluator runs")
    }

    #[test]
    fn a_decision_asked_without_a_session_carries_no_actor_key_at_all() {
        let dir = fixture();
        let decision = decide(dir.path(), None);
        assert!(
            decision.get("actor").is_none(),
            "an `actor: null` on every decision would answer a question nobody asked: {decision}"
        );
        assert!(decision.get("decision").is_some());
    }

    #[test]
    fn a_session_with_no_sidecar_stamps_unclaimed_without_creating_one() {
        let dir = fixture();
        let root = dir.path();

        let decision = decide(root, Some(SESSION));
        let actor = &decision["actor"];
        assert_eq!(actor["source"], "unclaimed");
        assert_eq!(actor["claimed"], Value::Null);
        assert_eq!(actor["definitionCid"], Value::Null);
        assert_eq!(actor["session"], SESSION);
        assert!(
            !root.join(".eprfs").exists(),
            "reading the identity plane must not create it — SidecarActorStore::open would"
        );
    }

    #[test]
    fn a_registered_claim_stamps_the_identity_and_its_definition() {
        let dir = fixture();
        let root = dir.path();
        crate::actor::claim(root, CLAIMED, SESSION).expect("claim registers");

        let actor = decide(root, Some(SESSION))["actor"].clone();
        assert_eq!(actor["source"], "claim");
        assert_eq!(actor["claimed"], CLAIMED);
        assert_eq!(actor["session"], SESSION);
        let definition = actor["definitionCid"]
            .as_str()
            .expect("the package on disk gives a real definition address");
        assert!(definition.starts_with("sha256:"), "got: {definition}");

        // A session nobody registered still reads as unclaimed against the very same sidecar.
        let other = decide(root, Some("session-nobody-claimed"))["actor"].clone();
        assert_eq!(other["source"], "unclaimed");
    }

    #[test]
    fn a_tampered_sidecar_stamps_unclaimed_and_the_decision_still_lands() {
        let dir = fixture();
        let root = dir.path();
        crate::actor::claim(root, CLAIMED, SESSION).expect("claim registers");

        // Rewrite the claimed identity in place, leaving the stored CID untouched — the shape an
        // impersonation edit takes. `records()` refuses the line; the stamp must not refuse the
        // decision.
        let log = root.join(ACTOR_LOG_REL);
        let text = std::fs::read_to_string(&log)
            .unwrap()
            .replace(CLAIMED, "agent:storyteller@opus-5");
        std::fs::write(&log, text).unwrap();

        let decision = decide(root, Some(SESSION));
        assert_eq!(
            decision["actor"]["source"], "unclaimed",
            "a tampered claim reads as unclaimed, never as the identity the tamperer wrote"
        );
        assert!(
            decision.get("decision").is_some_and(|d| d.is_string()),
            "the identity plane must never cost the tree its verdict: {decision}"
        );
    }

    #[test]
    fn a_session_flag_with_no_value_is_refused_rather_than_read_as_absence() {
        let err = parse(&["--session".to_string()]).expect_err("a dangling flag must refuse");
        assert!(err.to_string().contains("--session"), "got: {err}");
    }
}
