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

    // Create a link from ID hash to content for lookup by ID
    let id_hash = hash_entry(&input.id)?;
    create_link(
        id_hash,
        action_hash.clone(),
        LinkTypes::IdToContent,
        (),
    )?;

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

/// Get content by its ID (string identifier)
#[hdk_extern]
pub fn get_content_by_id(id: String) -> ExternResult<Option<ContentOutput>> {
    let id_hash = hash_entry(&id)?;
    let links = get_links(
        GetLinksInputBuilder::try_new(id_hash, LinkTypes::IdToContent)?.build(),
    )?;

    // Return the most recent content with this ID
    if let Some(link) = links.first() {
        let action_hash = link.target.clone().into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Link target is not an ActionHash".to_string()
            )))?;
        return get_content(action_hash);
    }

    Ok(None)
}

/// List all content created by the current agent
#[hdk_extern]
pub fn get_my_content(_: ()) -> ExternResult<Vec<ContentOutput>> {
    let agent_info = agent_info()?;
    let query = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::Content.try_into()?);

    let records = query_mine(query)?;

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

/// Query this agent's source chain for entries
fn query_mine(filter: ChainQueryFilter) -> ExternResult<Vec<Record>> {
    query(filter)
}
