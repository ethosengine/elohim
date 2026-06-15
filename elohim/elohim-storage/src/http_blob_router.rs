//! Backend selection for `GET /blob/{hash}`. Decides between
//! `IrohBlobStore` (BLAKE3) and the legacy `BlobStore` (SHA256) per
//! request, honoring the caller's transport-profile manifest and the
//! blob's known address aliases.
//!
//! Returns a [`BlobBackendChoice`] the HTTP handler switches on. Pure
//! over its inputs — no I/O, no network calls — so it is unit-testable
//! without spinning up either store.

use crate::p2p_iroh::peer_map::{PeerTransportManifest, Plane, TransportChoice};

/// Per-object transport affinity (Category C, operational). Lets a single
/// blob declare which transport should carry it, overriding the negotiated
/// per-request transport selection.
///
/// `Auto` (the default, and the meaning of a NULL
/// `peer_blob_inventory.transport_affinity` column) preserves exactly the
/// pre-affinity behavior: the negotiated peer-map verdict decides. The other
/// variants override that verdict for this object only.
///
/// Wire form (kebab strings) is the stable representation used by the
/// migration column and any operator setter route. Unknown strings parse to
/// `Auto` so a forward-compatible value degrades to default policy rather
/// than failing the read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportAffinity {
    /// Use the negotiated transport (today's behavior). NULL column == this.
    #[default]
    Auto,
    /// Prefer iroh when reachable; same outcome as the negotiated verdict,
    /// which already prefers iroh on the blob plane.
    PreferIroh,
    /// Prefer the legacy (libp2p/SHA256) store unless only iroh can serve.
    PreferLibp2p,
    /// Force iroh. Degrades to the legacy path (not a panic) when no BLAKE3
    /// alias is known for the object — effectively unavailable-over-iroh.
    IrohOnly,
    /// Force the legacy (libp2p/SHA256) store even when iroh would be chosen.
    Libp2pOnly,
}

impl TransportAffinity {
    /// Stable kebab wire string.
    pub fn as_str(&self) -> &'static str {
        match self {
            TransportAffinity::Auto => "auto",
            TransportAffinity::PreferIroh => "prefer-iroh",
            TransportAffinity::PreferLibp2p => "prefer-libp2p",
            TransportAffinity::IrohOnly => "iroh-only",
            TransportAffinity::Libp2pOnly => "libp2p-only",
        }
    }

    /// Parse a kebab wire string. Unknown / NULL-mapped values fall back to
    /// `Auto` (forward-compatible: never fails the read on a new value).
    pub fn parse(s: &str) -> Self {
        match s {
            "prefer-iroh" => TransportAffinity::PreferIroh,
            "prefer-libp2p" => TransportAffinity::PreferLibp2p,
            "iroh-only" => TransportAffinity::IrohOnly,
            "libp2p-only" => TransportAffinity::Libp2pOnly,
            // "auto" and anything unrecognized → Auto.
            _ => TransportAffinity::Auto,
        }
    }

    /// Map an optional DB column value (NULL → `Auto`).
    pub fn from_db(opt: Option<&str>) -> Self {
        opt.map(TransportAffinity::parse).unwrap_or_default()
    }
}

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
    /// Per-object transport affinity for this blob (from
    /// `peer_blob_inventory.transport_affinity`; NULL → `Auto`). When
    /// `Auto`, the negotiated verdict decides exactly as before. Any other
    /// value overrides the negotiated `chose_iroh` for this object only.
    pub affinity: TransportAffinity,
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
    // absent, degrades to libp2p-only. This is the NEGOTIATED verdict.
    let negotiated_iroh = match (inputs.self_manifest, inputs.caller_manifest) {
        (Some(self_m), Some(caller_m)) => {
            matches!(
                crate::p2p_iroh::peer_map::select_transport(self_m, caller_m, Plane::Blob,),
                Ok(TransportChoice::Iroh)
            )
        }
        _ => false,
    };

    // Whether the legacy (SHA256) store can serve this object. It can only
    // serve a BLAKE3-input object if a SHA256 alias is known; a SHA256-input
    // object always has a usable SHA256 form. (`sha256_hash` falls back to
    // the BLAKE3 input only as a placeholder for the existing 404 wire
    // shape, so that does NOT count as legacy-serveable.)
    let legacy_serveable = !is_blake3_input || inputs.sha256_alias_for_blake3.is_some();

    // Apply the per-object affinity override to the negotiated verdict.
    // `Auto` is a pure pass-through so behavior is byte-identical to the
    // pre-affinity path. All other variants override `chose_iroh`.
    let chose_iroh = match inputs.affinity {
        // Pass-through: exactly today's logic.
        TransportAffinity::Auto => negotiated_iroh,
        // Prefer iroh when reachable; the negotiated verdict already prefers
        // iroh on the blob plane, so this matches Auto.
        TransportAffinity::PreferIroh => negotiated_iroh,
        // Force iroh. If no BLAKE3 alias exists this degrades to legacy via
        // the `blake3_hash_opt` match below (not a panic).
        TransportAffinity::IrohOnly => true,
        // Force legacy regardless of the negotiated verdict.
        TransportAffinity::Libp2pOnly => false,
        // Prefer legacy unless ONLY iroh can serve (legacy can't reach the
        // object — a BLAKE3-only blob with no SHA256 alias). Otherwise pick
        // legacy.
        TransportAffinity::PreferLibp2p => !legacy_serveable,
    };

    match (chose_iroh, blake3_hash_opt) {
        (true, Some(blake3_hash)) => BlobBackendChoice::IrohThenLibp2p {
            blake3_hash,
            sha256_hash,
        },
        // IrohOnly with no BLAKE3 alias, or any non-iroh choice: legacy.
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
            affinity: TransportAffinity::Auto,
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
            affinity: TransportAffinity::Auto,
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
            affinity: TransportAffinity::Auto,
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
            affinity: TransportAffinity::Auto,
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
            affinity: TransportAffinity::Auto,
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
            affinity: TransportAffinity::Auto,
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
            affinity: TransportAffinity::Auto,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    // ---- Per-object transport affinity matrix --------------------------
    //
    // Covers each TransportAffinity × {blake3 alias present, absent} for a
    // SHA256-input object, plus the BLAKE3-input cases that exercise
    // legacy-serveability. Both manifests are iroh-capable so the NEGOTIATED
    // verdict is iroh; this isolates the affinity override as the variable.

    fn iroh_both() -> (PeerTransportManifest, PeerTransportManifest) {
        (iroh_capable_manifest(), iroh_capable_manifest())
    }

    /// `Auto` reproduces today's outcomes exactly — regression guard.
    /// With iroh-capable peers and a blake3 alias, Auto picks iroh; with no
    /// alias, Auto degrades to legacy. Identical to the pre-affinity tests.
    #[test]
    fn auto_with_blake3_alias_reproduces_iroh_then_libp2p() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::Auto,
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
    fn auto_without_blake3_alias_reproduces_libp2p_only() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::Auto,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    /// `Libp2pOnly` forces legacy even when iroh would have been chosen
    /// (iroh-capable peers + blake3 alias present).
    #[test]
    fn libp2p_only_forces_legacy_even_when_iroh_would_be_chosen() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::Libp2pOnly,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    #[test]
    fn libp2p_only_with_no_alias_is_legacy() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::Libp2pOnly,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    /// `IrohOnly` forces iroh when a blake3 alias exists — even where the
    /// negotiated verdict already would (so the override is observable in
    /// the next case).
    #[test]
    fn iroh_only_forces_iroh_when_blake3_alias_exists() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::IrohOnly,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::IrohThenLibp2p {
                blake3_hash: "blake3-bbbb".to_string(),
                sha256_hash: "sha256-aaaa".to_string(),
            }
        );
    }

    /// `IrohOnly` overrides a libp2p-only caller (negotiated verdict would
    /// be legacy) — forces iroh when a blake3 alias exists.
    #[test]
    fn iroh_only_overrides_libp2p_caller_when_blake3_alias_exists() {
        let self_m = iroh_capable_manifest();
        let caller_m = libp2p_only_manifest();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::IrohOnly,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::IrohThenLibp2p {
                blake3_hash: "blake3-bbbb".to_string(),
                sha256_hash: "sha256-aaaa".to_string(),
            }
        );
    }

    /// `IrohOnly` with NO blake3 alias is the one hard case: degrade to
    /// legacy (effectively unavailable-over-iroh) rather than panic.
    #[test]
    fn iroh_only_without_blake3_alias_degrades_to_legacy() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::IrohOnly,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    /// `PreferIroh` matches Auto: iroh when a blake3 alias exists and peers
    /// are iroh-capable.
    #[test]
    fn prefer_iroh_matches_auto_with_blake3_alias() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::PreferIroh,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::IrohThenLibp2p {
                blake3_hash: "blake3-bbbb".to_string(),
                sha256_hash: "sha256-aaaa".to_string(),
            }
        );
    }

    /// `PreferLibp2p` chooses legacy for a SHA256-input object even with a
    /// blake3 alias and iroh-capable peers (legacy is serveable).
    #[test]
    fn prefer_libp2p_chooses_legacy_for_serveable_sha256_object() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "sha256-aaaa",
            blake3_alias_for_sha256: Some("blake3-bbbb".to_string()),
            sha256_alias_for_blake3: None,
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::PreferLibp2p,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    /// `PreferLibp2p` falls back to iroh when ONLY iroh can serve: a
    /// BLAKE3-input object with no SHA256 alias (legacy can't reach it).
    #[test]
    fn prefer_libp2p_falls_back_to_iroh_when_only_iroh_serveable() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "blake3-bbbb",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: None, // no legacy path for this object
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::PreferLibp2p,
        });
        // sha256_hash falls back to the blake3 input as the 404 placeholder.
        assert_eq!(
            choice,
            BlobBackendChoice::IrohThenLibp2p {
                blake3_hash: "blake3-bbbb".to_string(),
                sha256_hash: "blake3-bbbb".to_string(),
            }
        );
    }

    /// `PreferLibp2p` on a BLAKE3-input object WITH a SHA256 alias prefers
    /// legacy (legacy is serveable).
    #[test]
    fn prefer_libp2p_blake3_input_with_sha256_alias_picks_legacy() {
        let (self_m, caller_m) = iroh_both();
        let choice = choose_backend(ChooseInputs {
            normalized_hash: "blake3-bbbb",
            blake3_alias_for_sha256: None,
            sha256_alias_for_blake3: Some("sha256-aaaa".to_string()),
            self_manifest: Some(&self_m),
            caller_manifest: Some(&caller_m),
            affinity: TransportAffinity::PreferLibp2p,
        });
        assert_eq!(
            choice,
            BlobBackendChoice::Libp2pOnly {
                sha256_hash: "sha256-aaaa".to_string()
            }
        );
    }

    /// Wire-string round-trip + NULL/unknown → Auto.
    #[test]
    fn affinity_wire_strings_round_trip() {
        for a in [
            TransportAffinity::Auto,
            TransportAffinity::PreferIroh,
            TransportAffinity::PreferLibp2p,
            TransportAffinity::IrohOnly,
            TransportAffinity::Libp2pOnly,
        ] {
            assert_eq!(TransportAffinity::parse(a.as_str()), a);
        }
        assert_eq!(TransportAffinity::from_db(None), TransportAffinity::Auto);
        assert_eq!(
            TransportAffinity::from_db(Some("not-a-real-value")),
            TransportAffinity::Auto
        );
        assert_eq!(
            TransportAffinity::from_db(Some("iroh-only")),
            TransportAffinity::IrohOnly
        );
    }
}
