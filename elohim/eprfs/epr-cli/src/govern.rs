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

use eprfs_meta::{evaluate_path_with, hex_lower, resolve_decision, GovernanceWrite};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    error::{Error, Result},
    repository_validators::ElohimRepositoryValidators,
};

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
            other => {
                return Err(Error::InvalidArguments(format!(
                    "unknown govern argument `{other}`"
                )))
            }
        }
    }
    Ok(parsed)
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

    Ok(json!({
        "path": normalized,
        "evaluator": evaluator_identity(),
        "decision": resolved.decision,
        "winningClass": resolved.winning_class,
        "ruleId": resolved.rule_id,
        "referReason": resolved.refer_reason,
        "reason": reason,
        "verdicts": evaluation.verdicts,
        "diagnostics": evaluation.diagnostics,
    }))
}

pub fn usage() -> &'static str {
    "usage: epr [--repo PATH] govern --path REL [--new] [--new-subdir] [--content-stdin | --content-file PATH]"
}
