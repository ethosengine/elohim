//! Manifest coordinator functions — Phase 3 P3.2.
//!
//! Authority gating is currently permissive (anyone can create manifests).
//! Phase 3.5 will introduce mishpat-DNA-notarized policy gating that
//! restricts who can create constitutional manifests.

use content_store_integrity::Manifest;
use hdk::prelude::*;

use crate::EntryTypes;

#[hdk_extern]
pub fn create_manifest(input: Manifest) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::Manifest(input))?;
    Ok(action_hash)
}

#[hdk_extern]
pub fn get_manifest(action_hash: ActionHash) -> ExternResult<Option<Manifest>> {
    let Some(record) = get(action_hash, GetOptions::default())? else {
        return Ok(None);
    };
    let manifest: Option<Manifest> = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;
    Ok(manifest)
}
