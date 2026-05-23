//! Share-routing evaluator. Pure function over ShareAllocation + event value.
//!
//! Per spec §6.1 (Form A only for M1). Form B affinity-derived deferred to M2.

use crate::error::StorageError;
use elohim_views::{ShareAllocation, ShareAllocationForm};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct RoutedAmount {
    pub collective_cid: String,
    pub amount: f64,
}

pub fn evaluate_share_routing(
    allocation: &ShareAllocation,
    event_value: f64,
    _at_block_height: u64,
) -> Result<Vec<RoutedAmount>, StorageError> {
    validate_allocation_invariants(allocation)?;
    let active_set: HashSet<String> = allocation
        .shares
        .as_ref()
        .ok_or_else(|| StorageError::InvalidInput("Declared form requires shares[]".into()))?
        .iter()
        .map(|s| s.collective_cid.clone())
        .collect();
    evaluate_share_routing_active_only(allocation, event_value, _at_block_height, &active_set)
}

pub fn evaluate_share_routing_active_only(
    allocation: &ShareAllocation,
    event_value: f64,
    _at_block_height: u64,
    active_set: &HashSet<String>,
) -> Result<Vec<RoutedAmount>, StorageError> {
    validate_allocation_invariants(allocation)?;
    let shares = allocation
        .shares
        .as_ref()
        .ok_or_else(|| StorageError::InvalidInput("Declared form requires shares[]".into()))?;

    let mut routed = Vec::new();
    let mut commons_amount = event_value * allocation.commons_pool_tribute;
    for share in shares {
        if active_set.contains(&share.collective_cid) {
            routed.push(RoutedAmount {
                collective_cid: share.collective_cid.clone(),
                amount: event_value * share.share,
            });
        } else {
            // Withdrawn member's share flows entirely to the commons pool
            // (no re-normalization — prevents oscillation around withdrawal events).
            commons_amount += event_value * share.share;
        }
    }
    routed.push(RoutedAmount {
        collective_cid: "commons-pool".into(),
        amount: commons_amount,
    });
    Ok(routed)
}

fn validate_allocation_invariants(allocation: &ShareAllocation) -> Result<(), StorageError> {
    if !matches!(allocation.form, ShareAllocationForm::Declared) {
        return Err(StorageError::InvalidInput(
            "M1 only supports ShareAllocationForm::Declared".into(),
        ));
    }
    if allocation.commons_pool_tribute <= 0.0 {
        return Err(StorageError::InvalidInput(
            "commons_pool_tribute must be > 0 (substrate refuses zero tribute)".into(),
        ));
    }
    if allocation.commons_pool_tribute > 1.0 {
        return Err(StorageError::InvalidInput(
            "commons_pool_tribute must be <= 1.0".into(),
        ));
    }
    let shares = allocation
        .shares
        .as_ref()
        .ok_or_else(|| StorageError::InvalidInput("Declared form requires shares[]".into()))?;
    let share_sum: f64 = shares.iter().map(|s| s.share).sum();
    if (share_sum + allocation.commons_pool_tribute - 1.0).abs() > 1e-6 {
        return Err(StorageError::InvalidInput(format!(
            "shares ({}) + commons_pool_tribute ({}) must sum to 1.0",
            share_sum, allocation.commons_pool_tribute
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("share_routing_tests.rs");
}
