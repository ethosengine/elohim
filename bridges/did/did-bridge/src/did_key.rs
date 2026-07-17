//! The `did:key` resolver — fully offline.
//!
//! Resolves to a DID document whose single verification method is the Multikey
//! derived from the key itself, with all five verification relationships
//! referencing it, per the did:key method convention. (The ed25519 key is used
//! directly for `keyAgreement` here rather than deriving a separate X25519 key —
//! the simplified convention the phase-1 brief specifies.)

use async_trait::async_trait;
use did_types::{Context, Did, DidDocument, VerificationMethod, VerificationRelationship};

use crate::codec::{core32_to_multikey, did_key_to_core32};
use crate::resolver::{DidResolutionError, DidResolutionResult, DidResolver};

/// Assemble the `did:key` DID document for a validated `did:key` DID.
///
/// Shared with `did:elohim` assembly, which reuses the same Multikey +
/// relationship shape anchored on a different subject DID.
pub fn assemble_did_key_document(did: &Did) -> Result<DidDocument, DidResolutionError> {
    // The method-specific-id IS the multibase key (`z…`); validate by decoding.
    let core = did_key_to_core32(&format!("did:key:{}", did.method_specific_id()))
        .map_err(|e| DidResolutionError::InvalidDid(e.to_string()))?;
    let multikey = core32_to_multikey(&core);

    let vm_id = did.with_fragment(&multikey);
    let vm = VerificationMethod::multikey(vm_id.clone(), did.clone(), multikey);

    let reference = || vec![VerificationRelationship::Reference(vm_id.clone())];

    let mut doc = DidDocument::new(Context::did_v1_1_multikey(), did.clone());
    doc.verification_method = Some(vec![vm]);
    doc.authentication = Some(reference());
    doc.assertion_method = Some(reference());
    doc.key_agreement = Some(reference());
    doc.capability_invocation = Some(reference());
    doc.capability_delegation = Some(reference());
    Ok(doc)
}

/// Offline resolver for the `did:key` method.
#[derive(Debug, Default, Clone)]
pub struct DidKeyResolver;

impl DidKeyResolver {
    /// Construct the resolver.
    pub fn new() -> Self {
        DidKeyResolver
    }
}

#[async_trait]
impl DidResolver for DidKeyResolver {
    fn method(&self) -> &'static str {
        "key"
    }

    async fn resolve(&self, did: &Did) -> Result<DidResolutionResult, DidResolutionError> {
        if did.method() != "key" {
            return Err(DidResolutionError::MethodNotSupported(
                did.method().to_string(),
            ));
        }
        let doc = assemble_did_key_document(did)?;
        Ok(DidResolutionResult::success(doc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_all_five_relationships_referencing_the_key() {
        let did = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr").unwrap();
        let doc = assemble_did_key_document(&did).unwrap();
        assert_eq!(doc.verification_method.as_ref().unwrap().len(), 1);
        for rel in [
            &doc.authentication,
            &doc.assertion_method,
            &doc.key_agreement,
            &doc.capability_invocation,
            &doc.capability_delegation,
        ] {
            let entries = rel.as_ref().expect("relationship present");
            assert!(matches!(entries[0], VerificationRelationship::Reference(_)));
        }
    }

    #[test]
    fn rejects_malformed_key_material() {
        // A well-formed did:key syntax but the multibase decodes to the wrong
        // length must fail as InvalidDid.
        let did = Did::parse("did:key:z6Mk").unwrap();
        assert!(matches!(
            assemble_did_key_document(&did),
            Err(DidResolutionError::InvalidDid(_))
        ));
    }
}
