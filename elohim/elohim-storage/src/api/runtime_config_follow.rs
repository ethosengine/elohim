//! `POST /admin/runtime-config/follow` — a peer joins (or leaves) a release
//! channel through its OWN API.
//!
//! ## What this is, in the p2p-design-gate's terms
//!
//! The follow set is **Ephemeral (class C)**: node-local operator configuration
//! naming which release channels THIS peer watches, and at what participation
//! mode. It is deliberately NOT a DHT entry type — the channel and its heads are
//! the notarized entities and already live on the DHT (minted by the release
//! ceremony); what a given peer chooses to watch is nobody else's truth and
//! nothing projects it.
//!
//! ## Why a route at all
//!
//! Rung 4 made the follow set a WATCHED file, so a change applies to a running
//! node in seconds. But every writer of that file was outside the node: a
//! Jenkins ConfigMap render, an operator's `$EDITOR`, an a2o fixture's
//! `writeFileSync`. "Enrol this peer in a channel" therefore meant "have
//! filesystem access to the box", which is exactly the shape a p2p-native
//! substrate must not require — a peer should be able to be ASKED, over its own
//! API, to follow a channel.
//!
//! ## The one-config-home rule
//!
//! This route writes the SAME file the boot config and the ConfigMap render
//! write, through [`crate::runtime_config::set_watched_key`], and it rewrites
//! exactly the `ELOHIM_RELEASE_CHANNELS` line — every other line in the file
//! (comments, `[section]` headers, other keys another seat set) is carried
//! through verbatim. There is no second config home, no in-memory follow set
//! that could disagree with the file, and no path where a restart loses what
//! this route did.
//!
//! With no watched file the answer is **503**, never a silent success: writing
//! to a path nothing reads is the config-lever failure this crate refuses to
//! ship.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response};
use serde::Deserialize;

use crate::error::StorageError;
use crate::runtime_config::{self, SetKeyError};
use crate::services::release_adoption::state::{self, AdoptionMode, RELEASE_CHANNELS_KEY};
use crate::services::response;

/// Request body. `mode` defaults to `observe` — the same default a bare
/// `channelId` (no `=mode`) carries in the file, so the two vocabularies cannot
/// drift; `remove` drops the channel from the follow set entirely.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowRequest {
    pub channel: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub remove: bool,
}

/// The follow decision, resolved from a request body. Pure — the whole
/// validation rule is testable without a filesystem or an HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowDecision {
    pub channel_id: String,
    /// `None` when the request asked to REMOVE the channel.
    pub mode: Option<AdoptionMode>,
}

/// Validate a request into a decision, or a message naming what was wrong.
///
/// Refuses **by name**, never by silent downgrade: a misspelled mode is a 400
/// quoting the misspelling and listing the legal values, because a typo that
/// quietly became `observe` would make a peer that asked for `canary` look like
/// it was following instructions while doing something else — and a typo that
/// quietly became `apply` would be the far worse half of the same bug
/// ([`AdoptionMode::parse`] holds the same line for the file path).
pub fn decide(req: &FollowRequest) -> Result<FollowDecision, String> {
    let channel_id = state::validate_channel_id(&req.channel)?.to_string();
    if req.remove {
        return Ok(FollowDecision {
            channel_id,
            mode: None,
        });
    }
    let raw_mode = req.mode.as_deref().unwrap_or("observe");
    let mode = AdoptionMode::parse(raw_mode).map_err(|_| {
        format!(
            "mode '{raw_mode}' is not a mode — legal values are 'observe' (the default), \
             'canary' and 'apply'"
        )
    })?;
    Ok(FollowDecision {
        channel_id,
        mode: Some(mode),
    })
}

/// `POST /admin/runtime-config/follow`.
pub async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: FollowRequest = match super::parse_body(req).await {
        Ok(b) => b,
        Err(e) => {
            return Ok(response::bad_request(&format!(
                "invalid body — expected {{\"channel\":\"<id>\",\"mode\":\"observe|canary|apply\",\
                 \"remove\":false}} ({e})"
            )))
        }
    };

    let decision = match decide(&body) {
        Ok(d) => d,
        Err(message) => return Ok(response::bad_request(&message)),
    };

    // 503 BEFORE the read: with no watched file there is nothing this route
    // could edit, and a 200 here would be a lie an operator only discovers when
    // the channel never shows up on /admin/adoption.
    let Some(path) = runtime_config::config_path() else {
        return Ok(response::service_unavailable(
            &SetKeyError::NotWatched.to_string(),
        ));
    };

    // Read the file's OWN value for the key, not `get_text` — the effective
    // value can come from the boot environment, and merging into that would
    // write the boot-env list into the file and then never restore it.
    let previous = match std::fs::read_to_string(&path) {
        Ok(text) => runtime_config::parse(&text)
            .get(RELEASE_CHANNELS_KEY)
            .cloned()
            .unwrap_or_default(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Ok(response::internal_error(&format!(
                "read {}: {e}",
                path.display()
            )))
        }
    };

    let next = state::merge_follow_entry(&previous, &decision.channel_id, decision.mode);
    // An empty follow set is written as an empty VALUE, not as a removed line:
    // `ELOHIM_RELEASE_CHANNELS = ""` states "this peer follows nothing" out
    // loud, which is what a reader of the file needs to see after a remove.
    let (written_path, reload) =
        match runtime_config::set_watched_key(RELEASE_CHANNELS_KEY, Some(next.as_str())) {
            Ok(v) => v,
            Err(SetKeyError::NotWatched) => {
                return Ok(response::service_unavailable(
                    &SetKeyError::NotWatched.to_string(),
                ))
            }
            Err(e @ SetKeyError::Io(_)) => return Ok(response::internal_error(&e.to_string())),
        };

    // The follow set as the CONTROLLER will read it — including any typed
    // refusal a neighbouring entry earns. A caller gets one round-trip answer
    // to "did it land, and what is this peer following now?".
    let followed = state::parse_followed_channels(&next);

    tracing::info!(
        target: "elohim_storage::release_adoption",
        channel = %decision.channel_id,
        mode = decision.mode.map(|m| m.label()).unwrap_or("<removed>"),
        path = %written_path.display(),
        channels = %next,
        "POST /admin/runtime-config/follow — follow set rewritten and reloaded"
    );

    Ok(response::ok(&serde_json::json!({
        "channel": decision.channel_id,
        "mode": decision.mode.map(|m| m.label()),
        "removed": decision.mode.is_none(),
        "path": written_path.display().to_string(),
        "previousChannels": previous,
        "channels": next,
        "followed": followed,
        "reload": reload.to_json(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(channel: &str, mode: Option<&str>, remove: bool) -> FollowRequest {
        FollowRequest {
            channel: channel.to_string(),
            mode: mode.map(str::to_string),
            remove,
        }
    }

    #[test]
    fn a_bare_request_defaults_to_observe() {
        let d = decide(&body("runtime:coordinators:elohim:workspace", None, false)).unwrap();
        assert_eq!(d.channel_id, "runtime:coordinators:elohim:workspace");
        assert_eq!(d.mode, Some(AdoptionMode::Observe));
    }

    #[test]
    fn canary_and_apply_must_be_asked_for_by_name() {
        assert_eq!(
            decide(&body("ch", Some("canary"), false)).unwrap().mode,
            Some(AdoptionMode::Canary)
        );
        assert_eq!(
            decide(&body("ch", Some("apply"), false)).unwrap().mode,
            Some(AdoptionMode::Apply)
        );
    }

    #[test]
    fn a_misspelled_mode_is_refused_by_name_never_downgraded() {
        let err = decide(&body("ch", Some("aply"), false)).unwrap_err();
        assert!(
            err.contains("aply"),
            "the refusal must quote the typo: {err}"
        );
        assert!(err.contains("observe") && err.contains("apply") && err.contains("canary"));
    }

    #[test]
    fn a_malformed_channel_id_is_refused_by_name() {
        for bad in ["", "   ", "a,b", "a=b", "a\"b", "a b", "a#b", "a;b"] {
            let err = decide(&body(bad, None, false)).unwrap_err();
            assert!(
                err.contains("channel id"),
                "'{bad}' must be refused as a channel id, got: {err}"
            );
        }
    }

    #[test]
    fn remove_ignores_the_mode_entirely() {
        // A remove carrying a nonsense mode still removes: the mode is
        // meaningless for an entry that is about to stop existing, and refusing
        // it would make "stop following this" fail for a caller that simply
        // echoed back whatever it last sent.
        let d = decide(&body("ch", Some("nonsense"), true)).unwrap();
        assert_eq!(d.mode, None);
    }

    #[test]
    fn a_malformed_channel_id_is_refused_even_on_remove() {
        assert!(decide(&body("a,b", None, true)).is_err());
    }
}
