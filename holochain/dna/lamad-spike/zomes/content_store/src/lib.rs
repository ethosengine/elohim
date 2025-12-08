//! Content Store Coordinator Zome
//!
//! Implements zome functions for creating and retrieving content.
//! This is the minimal API for browser connectivity testing.

use hdk::prelude::*;
use content_store_integrity::*;

/// Input for creating content
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateContentInput {
    pub id: String,
    pub title: String,
    pub body: String,
}

/// Output when retrieving content
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentOutput {
    pub action_hash: ActionHash,
    pub content: Content,
}

/// Create a new content entry
///
/// Returns the ActionHash of the created entry.
/// Browser can use this hash to retrieve the content later.
#[hdk_extern]
pub fn create_content(input: CreateContentInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;
    let now = sys_time()?;

    let content = Content {
        id: input.id.clone(),
        title: input.title,
        body: input.body,
        created_at: format!("{:?}", now),
        author: agent_info.agent_initial_pubkey.to_string(),
    };

    // Create the entry
    let action_hash = create_entry(&EntryTypes::Content(content.clone()))?;

    Ok(action_hash)
}

/// Get content by its ActionHash
#[hdk_extern]
pub fn get_content(action_hash: ActionHash) -> ExternResult<Option<ContentOutput>> {
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

            Ok(Some(ContentOutput {
                action_hash,
                content,
            }))
        }
        None => Ok(None),
    }
}

/// List all content created by the current agent
#[hdk_extern]
pub fn get_my_content(_: ()) -> ExternResult<Vec<ContentOutput>> {
    let query = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::Content.try_into()?);

    let records = query(query)?;

    let mut results = Vec::new();
    for record in records {
        let action_hash = record.action_address().clone();
        let content: Content = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(e))?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Could not deserialize content".to_string()
            )))?;

        results.push(ContentOutput {
            action_hash,
            content,
        });
    }

    Ok(results)
}
