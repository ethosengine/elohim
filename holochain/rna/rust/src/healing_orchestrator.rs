//! Background healing orchestrator
//!
//! Manages the healing workflow:
//! 1. Detect degraded entries
//! 2. Attempt healing (from v1 bridge or self-repair)
//! 3. Track progress
//! 4. Emit signals
//! 5. Handle retries and failures gracefully
//!
//! # Usage
//!
//! In your DNA's init():
//!
//! ```rust,ignore
//! pub fn init(_: InitPayload) -> InitResult {
//!     let orchestrator = HealingOrchestrator::new("my-dna-v1", "my-dna-v2");
//!
//!     match orchestrator.check_v1_on_startup()? {
//!         Some(available) => {
//!             if available {
//!                 // v1 exists, will heal on first query
//!                 set_flag("v1_available", true)?;
//!             }
//!         }
//!         None => {
//!             // No v1 bridge, fresh start
//!         }
//!     }
//!
//!     Ok(InitResult::Pass)
//! }
//! ```
//!
//! In your read paths:
//!
//! ```rust,ignore
//! pub fn get_content(id: String) -> ExternResult<Content> {
//!     // Try v2 first
//!     if let Some(entry) = query_v2(&id)? {
//!         if entry.validate().is_ok() {
//!             return Ok(entry);
//!         }
//!     }
//!
//!     // Not in v2 or validation failed, try v1
//!     HealingOrchestrator::heal_from_v1::<Content>(&id)
//! }
//! ```

use crate::healing::{HealingReport, HealingSignal};
use crate::self_healing::SelfHealingEntry;
use hdk::prelude::*;

/// Orchestrates healing operations across DNA versions
pub struct HealingOrchestrator {
    /// Name of the previous/v1 DNA role
    v1_role_name: String,
    /// Name of the current/v2 DNA role
    v2_role_name: String,
    /// Maximum healing attempts per entry
    max_healing_attempts: u32,
}

impl HealingOrchestrator {
    /// Create a new healing orchestrator
    ///
    /// # Arguments
    ///
    /// * `v1_role_name` - Role name of previous DNA (e.g., "lamad-v1")
    /// * `v2_role_name` - Role name of current DNA (e.g., "lamad-v2")
    pub fn new(v1_role_name: &str, v2_role_name: &str) -> Self {
        Self {
            v1_role_name: v1_role_name.to_string(),
            v2_role_name: v2_role_name.to_string(),
            max_healing_attempts: 3,
        }
    }

    /// Set maximum healing attempts (default 3)
    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_healing_attempts = attempts;
        self
    }

    /// Check if v1 bridge is available during startup
    ///
    /// Returns:
    /// - `Ok(Some(true))` - v1 is available and has data
    /// - `Ok(Some(false))` - v1 is available but empty
    /// - `Ok(None)` - v1 bridge doesn't exist (fresh start)
    /// - `Err(e)` - Error checking bridge
    pub fn check_v1_on_startup(&self) -> ExternResult<Option<bool>> {
        // Try calling a simple function on v1
        match self.call_v1::<(), bool>("coordinator", "is_data_present", ()) {
            Ok(has_data) => Ok(Some(has_data)),
            Err(_) => {
                // v1 role doesn't exist, this is a fresh start
                Ok(None)
            }
        }
    }

    /// Try to heal a specific entry from v1
    ///
    /// This is called when:
    /// - Entry doesn't exist in v2
    /// - Entry exists but failed validation
    ///
    /// Returns the healed entry if successful.
    pub fn heal_from_v1<T: SelfHealingEntry>(
        &self,
        entry_id: &str,
    ) -> ExternResult<T> {
        // Call v1 to export this specific entry
        let v1_data: serde_json::Value = match self.call_v1(
            "coordinator",
            "export_entry_by_id",
            serde_json::json!({ "id": entry_id }),
        ) {
            Ok(data) => data,
            Err(_) => {
                return Err(wasm_error!(WasmErrorInner::Guest(
                    format!("Entry {} not found in v1", entry_id)
                )))
            }
        };

        // Emit signal: found in v1
        emit_healing_signal(HealingSignal::HealingStarted {
            entry_id: entry_id.to_string(),
            attempt: 1,
        })?;

        // Transform v1 data to v2
        // The app should implement this by extending this orchestrator
        let mut healed_entry: T = serde_json::from_value(v1_data)
            .map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(
                    format!("Failed to deserialize v1 entry: {:?}", e)
                ))
            })?;

        // Validate the healed entry
        match healed_entry.validate() {
            Ok(_) => {
                healed_entry.set_validation_status(
                    crate::healing::ValidationStatus::Migrated,
                );
                healed_entry.set_healed_at(
                    sys_time()
                        .ok()
                        .map(|t| t.as_millis() as u64)
                        .unwrap_or(0),
                );

                emit_healing_signal(HealingSignal::HealingSucceeded {
                    entry_id: entry_id.to_string(),
                    entry_type: std::any::type_name::<T>().to_string(),
                    was_migrated_from_v1: true,
                })?;

                Ok(healed_entry)
            }
            Err(validation_error) => {
                emit_healing_signal(HealingSignal::HealingFailed {
                    entry_id: entry_id.to_string(),
                    entry_type: std::any::type_name::<T>().to_string(),
                    final_error: validation_error.clone(),
                })?;

                Err(wasm_error!(WasmErrorInner::Guest(
                    format!("Healed entry failed validation: {}", validation_error)
                )))
            }
        }
    }

    /// Try to heal a degraded entry with self-repair
    ///
    /// Calls the entry's self-healing logic.
    pub fn heal_with_self_repair<T: SelfHealingEntry>(
        entry: &mut T,
    ) -> ExternResult<bool> {
        match entry.try_self_heal() {
            Ok(was_modified) => {
                if was_modified {
                    entry.set_validation_status(crate::healing::ValidationStatus::Valid);
                }
                Ok(was_modified)
            }
            Err(e) => {
                entry.set_validation_status(crate::healing::ValidationStatus::Degraded);
                Err(wasm_error!(WasmErrorInner::Guest(
                    format!("Self-healing failed: {}", e)
                )))
            }
        }
    }

    /// Find all degraded entries of a type and attempt healing
    ///
    /// This would typically be called periodically or triggered by a signal.
    ///
    /// Note: Implementation depends on your chain query capabilities.
    /// This is a template that apps should customize.
    pub fn find_and_heal_degraded<T: SelfHealingEntry>(
        &self,
    ) -> ExternResult<HealingReport> {
        let mut report = HealingReport::new();

        // Query for all entries of type T
        // This is pseudocode - actual implementation depends on your zome
        // let filter = ChainQueryFilter::new()
        //     .entry_type(EntryTypes::MyEntry.try_into()?);
        // let records = query(filter)?;

        // for record in records {
        //     match self.heal_single_entry::<T>(record) {
        //         Ok(_) => report.record_healed(std::any::type_name::<T>()),
        //         Err(_) => report.record_failed(std::any::type_name::<T>()),
        //     }
        // }

        report.complete();
        Ok(report)
    }

    /// Call a function on the v1 DNA role via bridge
    fn call_v1<I, O>(
        &self,
        zome_name: &str,
        fn_name: &str,
        payload: I,
    ) -> ExternResult<O>
    where
        I: Serialize + std::fmt::Debug,
        O: serde::de::DeserializeOwned + std::fmt::Debug,
    {
        use crate::bridge::bridge_call;
        bridge_call(&self.v1_role_name, zome_name, fn_name, payload)
    }

    /// Get the v1 role name
    pub fn v1_role_name(&self) -> &str {
        &self.v1_role_name
    }

    /// Get the v2 role name
    pub fn v2_role_name(&self) -> &str {
        &self.v2_role_name
    }
}

/// Emit a healing signal that the app can listen for
///
/// Used internally by the orchestrator, but exposed so custom
/// healing logic can emit signals too.
pub fn emit_healing_signal(signal: HealingSignal) -> ExternResult<()> {
    // Emit as a zome call signal
    // Apps subscribe to these to show healing progress in UI
    hdk::prelude::emit_signal(signal)
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(
                format!("Failed to emit healing signal: {:?}", e)
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = HealingOrchestrator::new("lamad-v1", "lamad-v2");

        assert_eq!(orchestrator.v1_role_name(), "lamad-v1");
        assert_eq!(orchestrator.v2_role_name(), "lamad-v2");
        assert_eq!(orchestrator.max_healing_attempts, 3);
    }

    #[test]
    fn test_orchestrator_with_max_attempts() {
        let orchestrator = HealingOrchestrator::new("v1", "v2")
            .with_max_attempts(5);

        assert_eq!(orchestrator.max_healing_attempts, 5);
    }
}
