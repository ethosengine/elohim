//! Capture under a declared claim: a tool names the kind of problem it felt with a fixed word
//! from its declaration (`EPR_FEEDBACK_CLAIM`), and one session reporting it many times leaves
//! one record. Nothing here reads the network, starts a process or takes a lock of its own.

use super::{now, required, Result};
use elohim_epr_rea::feedback::{Occurrence, Outcome, Policy, Source};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The environment variable that selects the claim path.
pub(crate) const CLAIM_VAR: &str = "EPR_FEEDBACK_CLAIM";
/// The environment variable naming the emitting tool's release.
pub(crate) const EMITTER_VAR: &str = "EPR_FEEDBACK_EMITTER";
/// The session variables, in the order brit's `gap_notice_key()` reads them.
const SESSION_VARS: [&str; 5] = [
    "ELOHIM_SESSION_ID",
    "CLAUDE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ID",
    "CODEX_THREAD_ID",
    "CODEX_SESSION_ID",
];
const INVOCATION_VAR: &str = "EPR_FEEDBACK_INVOCATION";
const CLAIM_MAX: usize = 128;
const EMITTER_MAX: usize = 256;

/// The declared claim, when `EPR_FEEDBACK_CLAIM` is set and non-empty. A value that is not
/// Unicode is still a declaration, so it is refused rather than read as absent.
pub(crate) fn declared() -> Result<Option<String>> {
    match std::env::var_os(CLAIM_VAR) {
        None => Ok(None),
        Some(value) if value.is_empty() => Ok(None),
        Some(value) => value
            .into_string()
            .map(Some)
            .map_err(|_| "EPR_FEEDBACK_CLAIM is not a claim id".into()),
    }
}

/// Whether `claim` is `<producer>/<name>[:<param>]`: producer and name one or more of
/// `[a-z0-9-]`, param one or more of `[a-z0-9.-]`, at most 128 bytes in all.
pub(crate) fn valid_claim(claim: &str) -> bool {
    let word = |s: &str, extra: &[u8]| {
        !s.is_empty()
            && s.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || extra.contains(&b)
            })
    };
    if claim.len() > CLAIM_MAX {
        return false;
    }
    let Some((producer, rest)) = claim.split_once('/') else {
        return false;
    };
    let (name, param) = match rest.split_once(':') {
        Some((name, param)) => (name, Some(param)),
        None => (rest, None),
    };
    word(producer, &[]) && word(name, &[]) && param.is_none_or(|p| word(p, b"."))
}

/// The emitter, when it is a release identity: 1 to 256 ASCII bytes from `A-Z a-z 0-9 . _ : + -`
/// (a release record's CID, or `unrecorded:<version>+exe.<sha12>`). Anything else, a path
/// included, is dropped and the capture still succeeds.
pub(crate) fn valid_emitter(emitter: Option<&str>) -> Option<&str> {
    emitter.filter(|e| {
        !e.is_empty()
            && e.len() <= EMITTER_MAX
            && e.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".:_+-".contains(&b))
    })
}

/// The reporting unit of one session, equal to what brit's `gap_notice_key()` computes for the
/// same environment: `invocation-<hex>`, `session-<hex>` or `day-<unix days>`.
pub(crate) fn session_key(env: &dyn Fn(&str) -> Option<String>, unix_seconds: u64) -> String {
    let session = SESSION_VARS
        .iter()
        .filter_map(|name| env(name))
        .find(|value| !value.trim().is_empty());
    let invocation = env(INVOCATION_VAR).filter(|value| !value.trim().is_empty());
    if session.is_none() && invocation.is_none() {
        return format!("day-{}", unix_seconds / 86_400);
    }
    let mut hasher = Sha256::new();
    hasher.update(session.as_deref().unwrap_or_default().trim().as_bytes());
    let prefix = if let Some(invocation) = &invocation {
        hasher.update(b"\0");
        hasher.update(invocation.trim().as_bytes());
        "invocation"
    } else {
        "session"
    };
    let hex: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("{prefix}-{hex}")
}

/// Build the occurrence for a claim. The operation word is accepted and not stored; the
/// producer is admitted by the policy's own list when the store appends.
pub(crate) fn capture(
    claim: &str,
    options: &BTreeMap<String, String>,
    policy: &Policy,
) -> Result<Occurrence> {
    if !valid_claim(claim) {
        return Err(
            "EPR_FEEDBACK_CLAIM must be <producer>/<name>[:<param>] in lowercase \
             letters, digits and '-' (param may add '.'), at most 128 characters"
                .into(),
        );
    }
    let producer = required(options, "producer")?;
    let outcome_word = required(options, "outcome")?;
    let outcome: Outcome = serde_json::from_value(json!(outcome_word))?;
    let env = |name: &str| std::env::var(name).ok();
    let unix_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let run = session_key(&env, unix_seconds);
    if run.len() > 128 || !run.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err("invocation must be an opaque alphanumeric token".into());
    }
    let emitter = env(EMITTER_VAR);
    let emitter = valid_emitter(emitter.as_deref());
    let item = match emitter {
        Some(emitter) => format!("{claim} {outcome_word} {emitter}"),
        None => format!("{claim} {outcome_word}"),
    };
    Ok(Occurrence {
        source: Source {
            producer: producer.into(),
            run,
            item,
        },
        observed_at: now(),
        participant: policy.participant.clone(),
        subject: claim.into(),
        outcome,
        count: 1,
        duration_ms: None,
        impact: None,
        message: None,
        detail: emitter.map_or(Value::Null, |emitter| json!({ "emitter": emitter })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_ids_are_closed_words() {
        for good in [
            "ferry/capability-absent:list.since",
            "epr/recorder-unavailable:timeout",
            "a/b",
            "brit-2/x-9:1.2-3",
        ] {
            assert!(valid_claim(good), "{good}");
        }
        let long = format!("p/{}", "n".repeat(127));
        assert_eq!(long.len(), 129);
        for bad in [
            "",
            "ferry",
            "Ferry/x",
            "ferry/a/b",
            "ferry/a b",
            "ferry/:x",
            "/x",
            "ferry/x:",
            "ferry/x:a:b",
            "ferry/x.y",
            "ferry/x:/etc",
            long.as_str(),
        ] {
            assert!(!valid_claim(bad), "{bad}");
        }
        assert!(valid_claim(&format!("p/{}", "n".repeat(126))));
    }

    #[test]
    fn emitters_with_spaces_or_controls_are_dropped() {
        assert_eq!(valid_emitter(Some("brit/0.3.6")), None);
        assert_eq!(valid_emitter(Some("")), None);
        assert_eq!(valid_emitter(Some("brit 0.3.6")), None);
        assert_eq!(valid_emitter(Some("brit\u{7}")), None);
        assert_eq!(valid_emitter(Some(&"e".repeat(257))), None);
        assert!(valid_emitter(Some(&"e".repeat(256))).is_some());
        assert_eq!(valid_emitter(None), None);
    }

    #[test]
    fn emitters_are_ascii_release_identities() {
        for good in [
            "unrecorded:0.3.6+exe.0123456789ab",
            "bafyreib2rxk3rh6kzwq",
            "A-Z_a.z:0+9",
        ] {
            assert_eq!(valid_emitter(Some(good)), Some(good), "{good}");
        }
        for bad in [
            "/home/alice/secret",
            "brit/0.3.6",
            "a,b",
            "a b",
            "\u{e9}",
            "a@b",
            "",
        ] {
            assert_eq!(valid_emitter(Some(bad)), None, "{bad}");
        }
        assert!(valid_emitter(Some(&"e".repeat(256))).is_some());
        assert_eq!(valid_emitter(Some(&"e".repeat(257))), None);
    }

    #[test]
    fn session_key_takes_the_first_of_two_set_sessions() {
        let both = |name: &str| match name {
            "CLAUDE_SESSION_ID" => Some("claude".to_owned()),
            "CODEX_THREAD_ID" => Some("codex".to_owned()),
            _ => None,
        };
        let claude_only = |name: &str| (name == "CLAUDE_SESSION_ID").then(|| "claude".to_owned());
        assert_eq!(session_key(&both, 0), session_key(&claude_only, 0));
        let codex_only = |name: &str| (name == "CODEX_THREAD_ID").then(|| "codex".to_owned());
        assert_ne!(session_key(&both, 0), session_key(&codex_only, 0));
    }

    #[test]
    fn session_key_prefers_the_first_named_session_and_trims() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_owned())
            }
        };
        let first = session_key(&env(&[("ELOHIM_SESSION_ID", " one ")]), 0);
        assert_eq!(first, session_key(&env(&[("CODEX_SESSION_ID", "one")]), 0));
        assert_eq!(
            first,
            session_key(
                &env(&[("ELOHIM_SESSION_ID", "  "), ("CLAUDE_SESSION_ID", "one")]),
                0
            )
        );
        assert!(first.starts_with("session-") && first.len() == 8 + 64);
        assert_eq!(session_key(&env(&[]), 86_400 * 3 + 5), "day-3");
        let invocation = session_key(&env(&[("EPR_FEEDBACK_INVOCATION", "probe")]), 0);
        assert!(invocation.starts_with("invocation-"));
    }
}
