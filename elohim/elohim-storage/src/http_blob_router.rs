//! Backend selection for `GET /blob/{hash}`. Decides between
//! `IrohBlobStore` (BLAKE3) and the legacy `BlobStore` (SHA256) per
//! request, honoring the caller's transport-profile manifest and the
//! blob's known address aliases.
//!
//! Returns a [`BlobBackendChoice`] the HTTP handler switches on. Pure
//! over its inputs — no I/O, no network calls — so it is unit-testable
//! without spinning up either store.

use crate::p2p_iroh::peer_map::{PeerTransportManifest, Plane, TransportChoice};

/// Resolved backend choice and the hash to use against it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlobBackendChoice {
    /// Try Iroh first (with the BLAKE3 hash); fall through to libp2p
    /// (with the SHA256 hash) on miss.
    IrohThenLibp2p {
        blake3_hash: String,
        sha256_hash: String,
    },
    /// Use libp2p only (with the SHA256 hash). Either the caller is
    /// libp2p-only, or no BLAKE3 alias is known for this blob.
    Libp2pOnly { sha256_hash: String },
}

/// Inputs to the backend chooser. Constructed at the HTTP handler entry
/// point and then handed to [`choose_backend`] for a pure decision.
pub struct ChooseInputs<'a> {
    /// Caller-supplied hash, normalized: either `sha256-{hex}` or
    /// `blake3-{hex}`. Other forms (raw hex, CID) are normalized to
    /// `sha256-` upstream by `BlobStore::parse_content_address`.
    pub normalized_hash: &'a str,
    /// `peer_blob_inventory.blake3_hash` for the SHA256 form, if any.
    /// `None` when caller-hash is blake3-prefixed (irrelevant) or no
    /// row knows the alias.
    pub blake3_alias_for_sha256: Option<String>,
    /// `peer_blob_inventory.blob_hash` (the SHA256) for the BLAKE3
    /// form, if any. `None` when caller-hash is sha256-prefixed
    /// (irrelevant) or no row knows the alias.
    pub sha256_alias_for_blake3: Option<String>,
    /// This node's transport-profile manifest (Plan 1). When `None`
    /// (manifest not yet wired at startup), the router degrades to
    /// libp2p-only.
    pub self_manifest: Option<&'a PeerTransportManifest>,
    /// Caller's transport-profile manifest (Plan 1's
    /// `lookup_by_agent_cid`). `None` when caller is unauthenticated
    /// (visitor) or has no manifest published yet — degrades to
    /// libp2p-only.
    pub caller_manifest: Option<&'a PeerTransportManifest>,
}

/// Pure selection function. Returns the chosen backend(s) and the hash
/// to use for each.
pub fn choose_backend(inputs: ChooseInputs<'_>) -> BlobBackendChoice {
    let is_blake3_input = inputs.normalized_hash.starts_with("blake3-");

    // Resolve the SHA256 form for the libp2p path. For SHA256 inputs,
    // it is the input itself; for BLAKE3 inputs, look up the alias.
    let sha256_hash: String = if is_blake3_input {
        match &inputs.sha256_alias_for_blake3 {
            Some(s) => s.clone(),
            None => {
                // BLAKE3-only blob with no SHA256 alias known. The
                // caller MUST be iroh-capable for us to serve it; if
                // not, we'd be returning a 404 anyway. Encode that as
                // libp2p-only with the BLAKE3 form so the legacy path
                // produces the existing 404 wire shape.
                inputs.normalized_hash.to_string()
            }
        }
    } else {
        inputs.normalized_hash.to_string()
    };

    // Resolve the BLAKE3 form for the iroh path.
    let blake3_hash_opt: Option<String> = if is_blake3_input {
        Some(inputs.normalized_hash.to_string())
    } else {
        inputs.blake3_alias_for_sha256.clone()
    };

    // Run the cross-stack peer map's transport selector. Any path that
    // doesn't yield TransportChoice::Iroh, or where either manifest is
    // absent, degrades to libp2p-only.
    let chose_iroh = match (inputs.self_manifest, inputs.caller_manifest) {
        (Some(self_m), Some(caller_m)) => {
            matches!(
                crate::p2p_iroh::peer_map::select_transport(self_m, caller_m, Plane::Blob,),
                Ok(TransportChoice::Iroh)
            )
        }
        _ => false,
    };

    match (chose_iroh, blake3_hash_opt) {
        (true, Some(blake3_hash)) => BlobBackendChoice::IrohThenLibp2p {
            blake3_hash,
            sha256_hash,
        },
        _ => BlobBackendChoice::Libp2pOnly { sha256_hash },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iroh_capable_manifest() -> PeerTransportManifest {
        PeerTransportManifest::iroh_capable_for_test()
    }

    fn libp2p_only_manifest() -> PeerTransportManifest {
        PeerTransportManifest::libp2p_only_for_test()
    }

    #[test]
    fn caller_iroh_capable_and_blake3_known_picks_iroh_then_libp2p() {
        let self_m = iroh_capable_manifest();
        let caller_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::IrohThenLibp2p {
                blake3_hash: "blake3-bbbb".to_string(),
                sha256_hash: "sha256-aaaa".to_string(),
            }
        );
    }

    #[test]
    fn caller_libp2p_only_picks_libp2p_only() {
        let self_m = iroh_capable_manifest();
        let caller_m = libp2p_only_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn no_blake3_alias_picks_libp2p_only_even_when_iroh_capable() {
        let self_m = iroh_capable_manifest();
        let caller_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn caller_supplied_blake3_with_iroh_caller_picks_iroh() {
        let self_m = iroh_capable_manifest();
        let caller_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "blake3-bbbb",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: Some("sha256-aaaa".to_string()),
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::IrohThenLibp2p {
                blake3_hash: "blake3-bbbb".to_string(),
                sha256_hash: "sha256-aaaa".to_string(),
            }
        );
    }

    #[test]
    fn caller_supplied_blake3_with_libp2p_caller_falls_back_to_sha256() {
        let self_m = iroh_capable_manifest();
        let caller_m = libp2p_only_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "blake3-bbbb",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: Some("sha256-aaaa".to_string()),
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn no_caller_manifest_visitor_picks_libp2p_only() {
        let self_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: None,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn no_self_manifest_picks_libp2p_only() {
        let caller_m = iroh_capable_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: None,
            caller_manifest: Some(&caller_m),
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }
}
