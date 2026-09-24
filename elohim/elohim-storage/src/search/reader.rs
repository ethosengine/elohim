//! Who a content search answer is shaped for.
//!
//! A [`Reader`] (from `elohim_epr_index::reader`) is an agent and the reach tier it reads at. On
//! the storage peer the agent is the EXPLICIT `X-Agent-Cid` a request carried, resolved to a
//! human this peer knows — never the ambient local session: a hosted peer holds an active session
//! for its own human, and resolving an anonymous request through it would serve that human's
//! reach (the same rule `/db/content` follows). The tier is the most open reach ring the reader
//! may see without a per-row authorization: `commons` for an anonymous reader, `community` for a
//! resolved one. Rings above that are authorized row by row at the reach gate; the reader is its
//! input and is never printed on the wire.
pub use elohim_epr_index::reader::Reader;

/// The tier of a reader no agent could be resolved for: open reach only.
pub const ANONYMOUS_TIER: &str = "commons";

/// The tier of a reader resolved to a known human: every ring up to community without a per-row
/// check (a resolved identity IS the community gate).
pub const RESOLVED_TIER: &str = "community";

/// The reader for a request: `explicit_agent_cid` is the `X-Agent-Cid` header as sent (or
/// `None`), `resolved_human` the id of the human that header resolved to on this peer (or `None`
/// when it named nobody this peer knows). An unresolved header reads as anonymous — the agent is
/// kept for the omissions line, the tier is not widened.
pub fn reader_for(explicit_agent_cid: Option<&str>, resolved_human: Option<&str>) -> Reader {
    let agent_cid = explicit_agent_cid
        .map(str::trim)
        .filter(|cid| !cid.is_empty())
        .map(str::to_string);
    let tier = match (agent_cid.as_ref(), resolved_human) {
        (Some(_), Some(human)) if !human.trim().is_empty() => RESOLVED_TIER,
        _ => ANONYMOUS_TIER,
    };
    Reader {
        agent_cid,
        tier: tier.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anonymous_and_unresolved_readers_read_open_reach_only() {
        assert_eq!(reader_for(None, None).tier, ANONYMOUS_TIER);
        assert_eq!(reader_for(Some("  "), Some("h")).agent_cid, None);
        let unresolved = reader_for(Some("uhCAk-stranger"), None);
        assert_eq!(unresolved.agent_cid.as_deref(), Some("uhCAk-stranger"));
        assert_eq!(unresolved.tier, ANONYMOUS_TIER);
    }

    #[test]
    fn a_resolved_reader_reads_to_community() {
        let reader = reader_for(Some("uhCAk-susan"), Some("susan"));
        assert_eq!(reader.agent_cid.as_deref(), Some("uhCAk-susan"));
        assert_eq!(reader.tier, RESOLVED_TIER);
    }
}
