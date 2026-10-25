//! Node-local adapter to the Private (B) device manifest writer.
//! The route grants no remote authority and exposes no manifest resource.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response};
use serde::Deserialize;

use crate::error::StorageError;
use crate::runtime_config;
use crate::services::release_adoption::state::{self, AdoptionMode};
use crate::services::response;
use ark_core::declaration;
use elohim_device_runtime::manifest_store;

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
    let channel_id = req.channel.trim().to_owned();
    declaration::validate_channel(&channel_id)
        .map_err(|error| format!("invalid channel id: {error}"))?;
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

    let Some(path) = runtime_config::config_path() else {
        return Ok(response::service_unavailable(
            "no watched runtime-config path",
        ));
    };
    let store = match manifest_store::for_runtime_config(&path) {
        Ok(store) => store,
        Err(e) => return Ok(response::internal_error(&e.to_string())),
    };
    let previous = match store.read() {
        Ok(head) => declaration::render_channels(&head.follows),
        Err(e) => return Ok(response::internal_error(&e.to_string())),
    };
    let head = match store.follow_mode(decision.channel_id.clone(), decision.mode) {
        Ok(head) => head,
        Err(e) => return Ok(response::internal_error(&e.to_string())),
    };
    let next = declaration::render_channels(&head.follows);
    let written_path = path;
    let reload = runtime_config::reload_now();
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
    fn surrounding_channel_whitespace_is_normalized_for_follow_and_remove() {
        for remove in [false, true] {
            let d = decide(&body(
                " \n runtime:coordinators:elohim:workspace \t",
                None,
                remove,
            ))
            .unwrap();
            assert_eq!(d.channel_id, "runtime:coordinators:elohim:workspace");
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
            decide(&body("bin:ch", Some("canary"), false)).unwrap().mode,
            Some(AdoptionMode::Canary)
        );
        assert_eq!(
            decide(&body("bin:ch", Some("apply"), false)).unwrap().mode,
            Some(AdoptionMode::Apply)
        );
    }

    #[test]
    fn a_misspelled_mode_is_refused_by_name_never_downgraded() {
        let err = decide(&body("bin:ch", Some("aply"), false)).unwrap_err();
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
        let d = decide(&body("bin:ch", Some("nonsense"), true)).unwrap();
        assert_eq!(d.mode, None);
    }

    #[test]
    fn a_malformed_channel_id_is_refused_even_on_remove() {
        assert!(decide(&body("a,b", None, true)).is_err());
    }
}
