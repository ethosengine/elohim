//! The one declaration that elects an app slug's serving pointer.
//!
//! An app slug (`lamad-spa`, `elohim-host-landing`) is DECLARATIVE: its own
//! record names the release channel that elects its bytes, in
//! `metadata.releaseChannel`, and that is the whole statement. Everything else
//! is a reader of it. The channel's vehicle
//! (`services::release_adoption::apply::AppBundleVehicle`) is the ONLY writer
//! of a bound slug's pointer (`blob_hash` / `blob_cid` / `content_size_bytes`);
//! no heal, no converged doc, no legacy head may move it.
//!
//! This module holds the two pure questions every layer asks about that
//! declaration — "is this string a channel id?" and "what channel does this
//! row's metadata bind?" — so the db layer, the adoption controller and the
//! heal sweeps all read the SAME declaration instead of each keeping its own
//! copy. The guard that enforces it lives at the row-write layer
//! (`db::content_diesel`), where every pointer write passes, so a new heal
//! path cannot reopen the hole by forgetting to ask.
//!
//! Why this exists (alpha, 2026-10-07): the vehicle moved `lamad-spa` to a new
//! build at 21:07:51Z; at 21:08:41Z the pointer-audit sweep — trusting the
//! legacy head the pipeline once stamped on the row — "refreshed" the pointer
//! back to the 2026-09-22 blob, and the adoption ledger kept reading
//! `applied`. Three writers, one slug, no owner.

/// The metadata key a slug's record uses to name its elector.
pub const RELEASE_CHANNEL_KEY: &str = "releaseChannel";

/// `runtime:<class>:<network>:<name>` — three lower-slug segments after the
/// `runtime:` prefix. Shared by the manifest shape check and the binding
/// reader so a malformed value is "no declaration" to both.
pub fn is_channel_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("runtime:") else {
        return false;
    };
    let parts: Vec<&str> = rest.split(':').collect();
    parts.len() == 3 && parts.iter().all(|p| is_lower_slug(p))
}

/// `^[a-z0-9][a-z0-9-]*$` — the slug shape channel segments and app slugs share.
pub(crate) fn is_lower_slug(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The release channel a row's metadata binds it to, or `None` when the row
/// is its own elected head.
///
/// Only a value that IS a release channel id binds: a malformed or empty value
/// is not a declaration of anything, and reading it as one would strand the
/// slug with neither its own head nor a channel to follow.
pub fn binding_in_metadata(metadata_json: Option<&str>) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(metadata_json?.trim()).ok()?;
    let channel = parsed.get(RELEASE_CHANNEL_KEY)?.as_str()?.trim();
    is_channel_id(channel).then(|| channel.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_channel_id_has_the_runtime_prefix_and_three_lower_slugs() {
        assert!(is_channel_id("runtime:app-bundle:alpha:dev"));
        assert!(!is_channel_id("app-bundle:alpha:dev"));
        assert!(!is_channel_id("runtime:App-Bundle:alpha:dev"));
        assert!(!is_channel_id("runtime:app-bundle:alpha"));
        assert!(!is_channel_id(""));
    }

    #[test]
    fn only_a_well_formed_channel_binds() {
        assert_eq!(
            binding_in_metadata(Some(r#"{"releaseChannel":"runtime:app-bundle:alpha:dev"}"#))
                .as_deref(),
            Some("runtime:app-bundle:alpha:dev")
        );
        for unbound in [
            None,
            Some(""),
            Some("{}"),
            Some(r#"{"releaseChannel":""}"#),
            Some(r#"{"releaseChannel":"not a channel"}"#),
            Some(r#"{"releaseChannel":42}"#),
            Some("not json"),
        ] {
            assert_eq!(binding_in_metadata(unbound), None, "{unbound:?}");
        }
    }
}
