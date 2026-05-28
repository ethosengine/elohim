//! Scores incoming inventory_gossip advertisements against the local peer's
//! active replicates-* commitments. Output: priority tier consumed by the
//! existing inventory subscriber to decide which advertised blobs to fetch.
//!
//! Per spec §8.2: the substrate's "commitments shape what peers cache"
//! mechanism. Without it, peers fetch indiscriminately and commitments are
//! decorative.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchPriority {
    High,
    #[allow(dead_code)] // reserved: commons-tier follow-up
    Medium,
    Skip,
}

#[derive(Debug, Clone)]
pub struct AdvertisedBlob {
    pub blob_cid: String,
    pub source_peer_cid: String,
    pub blob_size_bytes: Option<u64>,
    pub recipient_hub_id_hint: Option<String>,
    pub epr_kind_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActiveCommitment {
    pub commitment_cid: String,
    pub action: String,           // "replicates-dwelling" etc.
    pub recipient_hub_id: String,
    pub scope_epr_kinds: Option<Vec<String>>,
    pub bytes_per_blob_max: Option<u64>,
}

pub fn score_advertised_blob(
    advertised: &AdvertisedBlob,
    active_commitments: &[ActiveCommitment],
) -> FetchPriority {
    for commitment in active_commitments {
        if commitment.action != "replicates-dwelling" {
            continue;
        }
        // Recipient match
        if let Some(rcpt) = &advertised.recipient_hub_id_hint {
            if rcpt != &commitment.recipient_hub_id {
                continue;
            }
        } else {
            // Without recipient hint, can't match. Skip.
            continue;
        }
        // Scope match (epr_kind)
        if let (Some(kinds), Some(kind)) = (&commitment.scope_epr_kinds, &advertised.epr_kind_hint) {
            if !kinds.iter().any(|k| k == kind) {
                continue;
            }
        }
        // Size match
        if let (Some(max), Some(size)) = (commitment.bytes_per_blob_max, advertised.blob_size_bytes) {
            if size > max {
                continue;
            }
        }
        return FetchPriority::High;
    }
    // Commons-tier eligible — deferred. Sprint 3 always returns Skip when no dwelling match.
    FetchPriority::Skip
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commitment(action: &str, recipient: &str) -> ActiveCommitment {
        ActiveCommitment {
            commitment_cid: "comm:test".into(),
            action: action.into(),
            recipient_hub_id: recipient.into(),
            scope_epr_kinds: Some(vec!["Content".into()]),
            bytes_per_blob_max: Some(1_000_000_000),
        }
    }

    fn ad(recipient: &str, kind: &str, size: u64) -> AdvertisedBlob {
        AdvertisedBlob {
            blob_cid: "bafkrei:test".into(),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(size),
            recipient_hub_id_hint: Some(recipient.into()),
            epr_kind_hint: Some(kind.into()),
        }
    }

    #[test]
    fn high_when_recipient_and_scope_match() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "Content", 500_000_000);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::High);
    }

    #[test]
    fn skip_when_no_matching_recipient() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:Z", "Content", 100);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_blob_exceeds_size_ceiling() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "Content", 5_000_000_000);  // > 1GB max
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_kind_not_in_scope() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "EconomicEvent", 100);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_no_commitments() {
        let a = ad("hub:B", "Content", 100);
        assert_eq!(score_advertised_blob(&a, &[]), FetchPriority::Skip);
    }
}
