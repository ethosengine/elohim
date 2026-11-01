//! Commit decoration derived from a live, owner-validated envelope and declared conventions.
//! The caller validates ownership. These strings assert participation, never actor authority.

use crate::{envelope::Envelope, session::Registry};

/// Derive Git trailer pairs without reading messages, writing records or inventing a model.
pub fn trailers(record: &Envelope, registry: &Registry) -> Result<Vec<(String, String)>, String> {
    if !record.is_live() {
        return Err("session is closed".into());
    }
    let convention = registry
        .runtimes
        .iter()
        .find(|runtime| runtime.id == record.identity.runtime)
        .and_then(|runtime| runtime.commit_attribution.as_ref())
        .ok_or("runtime has no declared commit attribution convention")?;
    let model = record
        .model
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or(
            "session model is unmeasured; restore the runtime model observation or mounted context",
        )?;
    let safe = |s: &str| !s.chars().any(char::is_control);
    if !safe(model)
        || model.contains(['<', '>'])
        || !safe(&convention.model_suffix)
        || convention.model_suffix.contains(['<', '>'])
        || !safe(&convention.email)
        || convention.email.chars().any(char::is_whitespace)
        || convention.email.contains(['<', '>'])
        || !convention.email.contains('@')
        || convention.session_trailer.is_empty()
        || !convention
            .session_trailer
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || !safe(record.id())
    {
        return Err("invalid session attribution fields".into());
    }
    Ok(vec![
        (
            "Co-Authored-By".into(),
            format!(
                "{}{} <{}>",
                model.trim(),
                convention.model_suffix,
                convention.email
            ),
        ),
        (convention.session_trailer.clone(), record.id().into()),
    ])
}
