//! Coordinator functions for PortalHost CRUD.
//!
//! Pre-commit gate (Category A2 derived discipline per
//! `project_hdi_no_get_links_in_validators`): coordinator verifies the
//! `AgentKeyToHuman` link and confirms the linked Human entry is authored by
//! the calling agent before committing. The integrity validator only checks
//! deterministic shape (URL format, reach, timestamp skew, human_action_hash
//! existence via `must_get_valid_record`).
//!
//! Human lookup pattern mirrors `resolve_human_id_for_agent` and
//! `get_human_by_agent_key` in lib.rs (same `LinkQuery::try_new` +
//! `GetStrategy::default()` + `.into_action_hash()` chain).

use hdk::prelude::*;
use imagodei_integrity::{EntryTypes, Human, LinkTypes, PortalHost, PortalHostReach};

// =============================================================================
// I/O Types
// =============================================================================

/// Input for `add_portal_host`.
///
/// `reach` defaults to `Trusted` when omitted — M5 only ships the Trusted
/// path; Intimate/Public are reserved for future milestones.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPortalHostInput {
    pub host_url: String,
    pub label: Option<String>,
    pub reach: Option<PortalHostReach>,
}

// =============================================================================
// Helpers
// =============================================================================

/// Resolve the calling agent's Human entry, returning `(action_hash, record)`.
///
/// Mirrors the `AgentKeyToHuman` traversal in `get_human_by_agent_key` and
/// `resolve_human_id_for_agent`.  Returns a coordinator `Guest` error if the
/// agent has no Human entry.
fn get_my_human_record() -> ExternResult<(ActionHash, Human)> {
    let caller_pubkey = agent_info()?.agent_initial_pubkey;

    let links = get_links(
        LinkQuery::try_new(caller_pubkey.clone(), LinkTypes::AgentKeyToHuman)?,
        GetStrategy::default(),
    )?;

    let first = links.first().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "No Human entry for this agent".to_string()
        ))
    })?;

    let action_hash = first.target.clone().into_action_hash().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "AgentKeyToHuman target is not an action hash".to_string()
        ))
    })?;

    let record = get(action_hash.clone(), GetOptions::default())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "Human entry missing from DHT".to_string()
        ))
    })?;

    // Authorship gate: the Human entry must have been authored by the calling agent.
    // This is the coordinator pre-commit gate described in portal_host.rs (integrity).
    if record.action().author() != &caller_pubkey {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Human entry is not authored by the calling agent".to_string()
        )));
    }

    let human: Human = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "Linked entry cannot be decoded as Human".to_string()
            ))
        })?;

    Ok((action_hash, human))
}

// =============================================================================
// add_portal_host
// =============================================================================

/// Add a portal host for the calling agent's Human.
///
/// Pre-commit gate: locates the calling agent's Human via `AgentKeyToHuman`
/// and confirms the Human is authored by the calling agent.  Commits a
/// `PortalHost` entry and creates a `Human → PortalHost` link via
/// `LinkTypes::PortalHosts`.
#[hdk_extern]
pub fn add_portal_host(input: AddPortalHostInput) -> ExternResult<ActionHash> {
    // Pre-commit gate — locate + verify the calling agent's Human.
    let (human_action_hash, _human) = get_my_human_record()?;

    let entry = PortalHost {
        human_action_hash: human_action_hash.clone(),
        host_url: input.host_url,
        label: input.label,
        added_at: sys_time()?,
        reach: input.reach.unwrap_or(PortalHostReach::Trusted),
    };

    let action_hash = create_entry(EntryTypes::PortalHost(entry))?;

    // Human ActionHash → PortalHost ActionHash
    create_link(
        human_action_hash,
        action_hash.clone(),
        LinkTypes::PortalHosts,
        (),
    )?;

    Ok(action_hash)
}

// =============================================================================
// remove_portal_host
// =============================================================================

/// Remove portal host(s) by URL match.
///
/// Multi-add semantics: if the same URL was added twice (two separate DHT
/// entries), both are deleted.  Calls `get_my_portal_hosts` to enumerate
/// current entries then deletes every entry whose `host_url` matches.
#[hdk_extern]
pub fn remove_portal_host(host_url: String) -> ExternResult<()> {
    let portal_hosts = get_my_portal_hosts(())?;
    for (action_hash, ph) in portal_hosts {
        if ph.host_url == host_url {
            delete_entry(action_hash)?;
        }
    }
    Ok(())
}

// =============================================================================
// get_my_portal_hosts
// =============================================================================

/// List all portal hosts for the calling agent's Human.
///
/// Walks `Human → PortalHosts` links, fetching each `PortalHost` entry.
/// Skips any link whose target cannot be resolved (network partition tolerance).
#[hdk_extern]
pub fn get_my_portal_hosts(_: ()) -> ExternResult<Vec<(ActionHash, PortalHost)>> {
    let (human_action_hash, _human) = get_my_human_record()?;

    let portal_links = get_links(
        LinkQuery::try_new(human_action_hash, LinkTypes::PortalHosts)?,
        GetStrategy::default(),
    )?;

    let mut out = Vec::with_capacity(portal_links.len());
    for link in portal_links {
        let action_hash = match link.target.clone().into_action_hash() {
            Some(h) => h,
            None => continue,
        };
        let Some(record) = get(action_hash.clone(), GetOptions::default())? else {
            continue;
        };
        let Ok(Some(ph)) = record.entry().to_app_option::<PortalHost>() else {
            continue;
        };
        out.push((action_hash, ph));
    }

    Ok(out)
}
