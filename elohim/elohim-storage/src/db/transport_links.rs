//! CRUD for `transport_links`.
//!
//! Source of truth: the verified two-key [`TransportLink`] a node publishes inside its
//! transport manifest. Only [`record_verified`] writes, and it takes a link whose both
//! halves the caller has verified. Category C (rebuilt from gossip).
//!
//! [`TransportLink`]: crate::p2p::transport_link::TransportLink

use diesel::prelude::*;

use super::diesel_schema::transport_links;
use crate::error::StorageError;
use crate::p2p::transport_link::TransportLink;

/// Record a link that already verified ([`TransportLink::verify`]). Keeps the newest
/// link per iroh NodeId; an older or equal one changes nothing. Returns whether the
/// row changed.
pub fn record_verified(
    conn: &mut SqliteConnection,
    link: &TransportLink,
) -> Result<bool, StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        let held: Option<i64> = transport_links::table
            .find(&link.iroh_node_id)
            .select(transport_links::issued_at_ms)
            .first(conn)
            .optional()?;
        if held.is_some_and(|held| link.issued_at_ms <= held) {
            return Ok::<bool, diesel::result::Error>(false);
        }
        diesel::replace_into(transport_links::table)
            .values((
                transport_links::iroh_node_id.eq(&link.iroh_node_id),
                transport_links::libp2p_peer_id.eq(&link.libp2p_peer_id),
                transport_links::issued_at_ms.eq(link.issued_at_ms),
            ))
            .execute(conn)?;
        Ok(true)
    })
    .map_err(|e| StorageError::Database(format!("transport_links::record_verified: {e}")))
}

/// The libp2p PeerId a verified link ties to `iroh_node_id`, if any.
pub fn libp2p_for_iroh(
    conn: &mut SqliteConnection,
    iroh_node_id: &str,
) -> Result<Option<String>, StorageError> {
    transport_links::table
        .find(iroh_node_id)
        .select(transport_links::libp2p_peer_id)
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("transport_links::libp2p_for_iroh: {e}")))
}

/// The iroh NodeIds verified links tie to any of `libp2p_peer_ids`.
pub fn iroh_for_libp2p(
    conn: &mut SqliteConnection,
    libp2p_peer_ids: &[String],
) -> Result<Vec<String>, StorageError> {
    if libp2p_peer_ids.is_empty() {
        return Ok(Vec::new());
    }
    transport_links::table
        .filter(transport_links::libp2p_peer_id.eq_any(libp2p_peer_ids))
        .select(transport_links::iroh_node_id)
        .load(conn)
        .map_err(|e| StorageError::Database(format!("transport_links::iroh_for_libp2p: {e}")))
}
