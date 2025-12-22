//! Read and Write Path Integration for Self-Healing DNA
//!
//! This module provides the glue between coordinator zome functions and the healing_impl module.
//! It's kept separate for clarity, but these implementations should be merged into lib.rs.

use hdk::prelude::*;
use content_store_integrity::*;
use crate::healing_impl;

// ============================================================================
// READ PATH INTEGRATION - Content
// ============================================================================

/// Enhanced get_content_by_id with healing fallback
/// Call this instead of direct DHT query to enable healing
pub fn get_content_by_id_with_healing(id: &str) -> ExternResult<Option<Content>> {
    // Try to get from v2 (current DNA)
    match get_content_by_id_v2(id) {
        Ok(Some(mut entry)) => {
            // Validate entry
            match entry.validate() {
                Ok(_) => {
                    // Valid, return it
                    return Ok(Some(entry));
                }
                Err(validation_err) => {
                    // Validation failed, mark as degraded and try healing
                    entry.set_validation_status(hc_rna::ValidationStatus::Degraded);

                    // Emit signal about degraded entry
                    let _ = healing_impl::emit_healing_signal(
                        hc_rna::HealingSignal::DegradedEntryFound {
                            entry_id: id.to_string(),
                            entry_type: "Content".to_string(),
                            reason: validation_err,
                        }
                    );

                    // Still return the degraded entry so app doesn't crash
                    return Ok(Some(entry));
                }
            }
        }
        Ok(None) => {
            // Not in v2, will try v1 below
        }
        Err(e) => {
            // DHT query error, log and try v1
            return Err(e);
        }
    }

    // Entry not in v2, try to heal from v1
    heal_content_from_v1(id)
}

/// Get content from v2 only (no healing)
fn get_content_by_id_v2(id: &str) -> ExternResult<Option<Content>> {
    let anchor = StringAnchor::new("content_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;
        get_content_from_hash(&action_hash)
    } else {
        Ok(None)
    }
}

/// Get content record by action hash
fn get_content_from_hash(action_hash: &ActionHash) -> ExternResult<Option<Content>> {
    let record = get(action_hash.clone(), GetOptions::default())?;

    match record {
        Some(record) => {
            let content: Content = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Could not deserialize content".to_string()
                )))?;

            Ok(Some(content))
        }
        None => Ok(None),
    }
}

/// Attempt to heal content from v1
fn heal_content_from_v1(id: &str) -> ExternResult<Option<Content>> {
    // Create orchestrator
    let orchestrator = healing_impl::create_healing_orchestrator();

    // Call v1 to export this content
    let v1_entry: healing_impl::ContentV1Export = match hc_rna::bridge_call(
        orchestrator.v1_role_name(),
        "coordinator",
        "export_content_by_id",
        serde_json::json!({ "id": id }),
    ) {
        Ok(data) => data,
        Err(_) => {
            // v1 doesn't have it either
            return Ok(None);
        }
    };

    // Emit signal: healing started
    let _ = healing_impl::emit_healing_signal(
        hc_rna::HealingSignal::HealingStarted {
            entry_id: id.to_string(),
            attempt: 1,
        }
    );

    // Transform v1 to v2
    let mut healed = healing_impl::transform_content_v1_to_v2(v1_entry);

    // Validate
    match healed.validate() {
        Ok(_) => {
            // Valid, update status
            healed.set_validation_status(hc_rna::ValidationStatus::Migrated);
            healed.set_healed_at(sys_time().ok().map(|t| t.as_millis() as u64).unwrap_or(0));

            // Cache in v2 for next time
            let _ = create_entry(&EntryTypes::Content(healed.clone()));

            // Create index link so it's findable
            let action_hash = create_entry(&EntryTypes::Content(healed.clone()))?;
            let _ = crate::create_id_to_content_link(&healed.id, &action_hash);

            // Emit success signal
            let _ = healing_impl::emit_healing_signal(
                hc_rna::HealingSignal::HealingSucceeded {
                    entry_id: id.to_string(),
                    entry_type: "Content".to_string(),
                    was_migrated_from_v1: true,
                }
            );

            Ok(Some(healed))
        }
        Err(validation_err) => {
            // Healing failed validation
            let _ = healing_impl::emit_healing_signal(
                hc_rna::HealingSignal::HealingFailed {
                    entry_id: id.to_string(),
                    entry_type: "Content".to_string(),
                    final_error: validation_err,
                }
            );

            // Return None - can't heal this entry
            Ok(None)
        }
    }
}

// ============================================================================
// READ PATH INTEGRATION - LearningPath
// ============================================================================

pub fn get_path_by_id_with_healing(id: &str) -> ExternResult<Option<LearningPath>> {
    match get_path_by_id_v2(id) {
        Ok(Some(mut entry)) => {
            match entry.validate() {
                Ok(_) => Ok(Some(entry)),
                Err(_) => {
                    entry.set_validation_status(hc_rna::ValidationStatus::Degraded);
                    Ok(Some(entry))
                }
            }
        }
        Ok(None) => heal_path_from_v1(id),
        Err(e) => Err(e),
    }
}

fn get_path_by_id_v2(id: &str) -> ExternResult<Option<LearningPath>> {
    let anchor = StringAnchor::new("path_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        match record {
            Some(record) => {
                let path: LearningPath = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(e))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest(
                        "Could not deserialize path".to_string()
                    )))?;

                Ok(Some(path))
            }
            None => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn heal_path_from_v1(id: &str) -> ExternResult<Option<LearningPath>> {
    let orchestrator = healing_impl::create_healing_orchestrator();

    let v1_entry: healing_impl::LearningPathV1Export = match hc_rna::bridge_call(
        orchestrator.v1_role_name(),
        "coordinator",
        "export_path_by_id",
        serde_json::json!({ "id": id }),
    ) {
        Ok(data) => data,
        Err(_) => return Ok(None),
    };

    let mut healed = healing_impl::transform_learning_path_v1_to_v2(v1_entry);

    match healed.validate() {
        Ok(_) => {
            healed.set_validation_status(hc_rna::ValidationStatus::Migrated);
            let _ = create_entry(&EntryTypes::LearningPath(healed.clone()));
            Ok(Some(healed))
        }
        Err(_) => Ok(None),
    }
}

// ============================================================================
// READ PATH INTEGRATION - PathStep
// ============================================================================

pub fn get_step_by_id_with_healing(id: &str) -> ExternResult<Option<PathStep>> {
    match get_step_by_id_v2(id) {
        Ok(Some(mut entry)) => {
            match entry.validate() {
                Ok(_) => Ok(Some(entry)),
                Err(_) => {
                    entry.set_validation_status(hc_rna::ValidationStatus::Degraded);
                    Ok(Some(entry))
                }
            }
        }
        Ok(None) => heal_step_from_v1(id),
        Err(e) => Err(e),
    }
}

fn get_step_by_id_v2(id: &str) -> ExternResult<Option<PathStep>> {
    let anchor = StringAnchor::new("step_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPathStep)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        match record {
            Some(record) => {
                let step: PathStep = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(e))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest(
                        "Could not deserialize step".to_string()
                    )))?;

                Ok(Some(step))
            }
            None => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn heal_step_from_v1(id: &str) -> ExternResult<Option<PathStep>> {
    let orchestrator = healing_impl::create_healing_orchestrator();

    let v1_entry: healing_impl::PathStepV1Export = match hc_rna::bridge_call(
        orchestrator.v1_role_name(),
        "coordinator",
        "export_step_by_id",
        serde_json::json!({ "id": id }),
    ) {
        Ok(data) => data,
        Err(_) => return Ok(None),
    };

    let mut healed = healing_impl::transform_path_step_v1_to_v2(v1_entry);

    match healed.validate() {
        Ok(_) => {
            healed.set_validation_status(hc_rna::ValidationStatus::Migrated);
            let _ = create_entry(&EntryTypes::PathStep(healed.clone()));
            Ok(Some(healed))
        }
        Err(_) => Ok(None),
    }
}

// ============================================================================
// READ PATH INTEGRATION - ContentMastery
// ============================================================================

pub fn get_mastery_by_id_with_healing(id: &str) -> ExternResult<Option<ContentMastery>> {
    match get_mastery_by_id_v2(id) {
        Ok(Some(mut entry)) => {
            match entry.validate() {
                Ok(_) => Ok(Some(entry)),
                Err(_) => {
                    entry.set_validation_status(hc_rna::ValidationStatus::Degraded);
                    Ok(Some(entry))
                }
            }
        }
        Ok(None) => heal_mastery_from_v1(id),
        Err(e) => Err(e),
    }
}

fn get_mastery_by_id_v2(id: &str) -> ExternResult<Option<ContentMastery>> {
    let anchor = StringAnchor::new("mastery_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContentMastery)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        match record {
            Some(record) => {
                let mastery: ContentMastery = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(e))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest(
                        "Could not deserialize mastery".to_string()
                    )))?;

                Ok(Some(mastery))
            }
            None => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn heal_mastery_from_v1(id: &str) -> ExternResult<Option<ContentMastery>> {
    let orchestrator = healing_impl::create_healing_orchestrator();

    let v1_entry: healing_impl::ContentMasteryV1Export = match hc_rna::bridge_call(
        orchestrator.v1_role_name(),
        "coordinator",
        "export_mastery_by_id",
        serde_json::json!({ "id": id }),
    ) {
        Ok(data) => data,
        Err(_) => return Ok(None),
    };

    let mut healed = healing_impl::transform_content_mastery_v1_to_v2(v1_entry);

    match healed.validate() {
        Ok(_) => {
            healed.set_validation_status(hc_rna::ValidationStatus::Migrated);
            let _ = create_entry(&EntryTypes::ContentMastery(healed.clone()));
            Ok(Some(healed))
        }
        Err(_) => Ok(None),
    }
}

// ============================================================================
// WRITE PATH INTEGRATION
// ============================================================================

/// Ensure content has current schema version and is validated
pub fn prepare_content_for_storage(mut content: Content) -> ExternResult<Content> {
    // Always use current schema version
    content.schema_version = 2;
    content.validation_status = "Valid".to_string();

    // Validate before storing
    content.validate()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    Ok(content)
}

/// Ensure learning path has current schema version and is validated
pub fn prepare_path_for_storage(mut path: LearningPath) -> ExternResult<LearningPath> {
    path.schema_version = 2;
    path.validation_status = "Valid".to_string();

    path.validate()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    Ok(path)
}

/// Ensure path step has current schema version and is validated
pub fn prepare_step_for_storage(mut step: PathStep) -> ExternResult<PathStep> {
    step.schema_version = 2;
    step.validation_status = "Valid".to_string();

    step.validate()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    Ok(step)
}

/// Ensure mastery has current schema version and is validated
pub fn prepare_mastery_for_storage(mut mastery: ContentMastery) -> ExternResult<ContentMastery> {
    mastery.schema_version = 2;
    mastery.validation_status = "Valid".to_string();

    mastery.validate()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    Ok(mastery)
}
