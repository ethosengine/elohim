//! Content Store Coordinator Zome
//!
//! Implements zome functions for CRUD operations on content entries.
//! Supports bulk imports, ID-based lookups, type filtering, and learning paths.

use hdk::prelude::*;
use content_store_integrity::*;
use std::collections::HashMap;

// =============================================================================
// Input/Output Types for Content
// =============================================================================

/// Input for creating content (matches ContentNode from elohim-service)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateContentInput {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub content_format: String,
    pub tags: Vec<String>,
    pub source_path: Option<String>,
    pub related_node_ids: Vec<String>,
    pub reach: String,
    pub metadata_json: String,
}

/// Output when retrieving content
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub content: Content,
}

/// Input for bulk content creation
#[derive(Serialize, Deserialize, Debug)]
pub struct BulkCreateContentInput {
    pub import_id: String,
    pub contents: Vec<CreateContentInput>,
}

/// Output from bulk content creation
#[derive(Serialize, Deserialize, Debug)]
pub struct BulkCreateContentOutput {
    pub import_id: String,
    pub created_count: u32,
    pub action_hashes: Vec<ActionHash>,
    pub errors: Vec<String>,
}

/// Input for querying content by type
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryByTypeInput {
    pub content_type: String,
    pub limit: Option<u32>,
}

/// Input for querying content by ID
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryByIdInput {
    pub id: String,
}

/// Content statistics output
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentStats {
    pub total_count: u32,
    pub by_type: HashMap<String, u32>,
}

// =============================================================================
// Input/Output Types for Learning Paths
// =============================================================================

/// Input for creating a learning path
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePathInput {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub purpose: Option<String>,
    pub difficulty: String,
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    pub tags: Vec<String>,
}

/// Input for adding a step to a path
#[derive(Serialize, Deserialize, Debug)]
pub struct AddPathStepInput {
    pub path_id: String,
    pub order_index: u32,
    pub step_type: String,
    pub resource_id: String,
    pub step_title: Option<String>,
    pub step_narrative: Option<String>,
    pub is_optional: bool,
}

/// Output for learning path with steps
#[derive(Serialize, Deserialize, Debug)]
pub struct PathWithSteps {
    pub action_hash: ActionHash,
    pub path: LearningPath,
    pub steps: Vec<PathStepOutput>,
}

/// Output for a path step
#[derive(Serialize, Deserialize, Debug)]
pub struct PathStepOutput {
    pub action_hash: ActionHash,
    pub step: PathStep,
}

// =============================================================================
// Content CRUD Operations
// =============================================================================

/// Create a single content entry with all index links
#[hdk_extern]
pub fn create_content(input: CreateContentInput) -> ExternResult<ContentOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let content = Content {
        id: input.id.clone(),
        content_type: input.content_type.clone(),
        title: input.title,
        description: input.description,
        content: input.content,
        content_format: input.content_format,
        tags: input.tags.clone(),
        source_path: input.source_path,
        related_node_ids: input.related_node_ids,
        author_id: Some(agent_info.agent_initial_pubkey.to_string()),
        reach: input.reach,
        trust_score: 0.0,
        metadata_json: input.metadata_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    // Create the entry
    let action_hash = create_entry(&EntryTypes::Content(content.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::Content(content.clone()))?;

    // Create index links
    create_id_to_content_link(&input.id, &action_hash)?;
    create_type_to_content_link(&input.content_type, &action_hash)?;
    create_author_to_content_link(&action_hash)?;

    for tag in &input.tags {
        create_tag_to_content_link(tag, &action_hash)?;
    }

    Ok(ContentOutput {
        action_hash,
        entry_hash,
        content,
    })
}

/// Bulk create content entries (for import operations)
#[hdk_extern]
pub fn bulk_create_content(input: BulkCreateContentInput) -> ExternResult<BulkCreateContentOutput> {
    let mut action_hashes = Vec::new();
    let mut errors = Vec::new();

    for content_input in input.contents {
        match create_content(content_input.clone()) {
            Ok(output) => {
                // Create import batch link for traceability
                create_import_batch_link(&input.import_id, &output.action_hash)?;
                action_hashes.push(output.action_hash);
            }
            Err(e) => {
                errors.push(format!("Failed to create '{}': {:?}", content_input.id, e));
            }
        }
    }

    Ok(BulkCreateContentOutput {
        import_id: input.import_id,
        created_count: action_hashes.len() as u32,
        action_hashes,
        errors,
    })
}

/// Get content by ActionHash
#[hdk_extern]
pub fn get_content(action_hash: ActionHash) -> ExternResult<Option<ContentOutput>> {
    let record = get(action_hash.clone(), GetOptions::default())?;

    match record {
        Some(record) => {
            let entry_hash = record
                .action()
                .entry_hash()
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "No entry hash in record".to_string()
                )))?
                .clone();

            let content: Content = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Could not deserialize content".to_string()
                )))?;

            Ok(Some(ContentOutput {
                action_hash,
                entry_hash,
                content,
            }))
        }
        None => Ok(None),
    }
}

/// Get content by string ID (using IdToContent link)
#[hdk_extern]
pub fn get_content_by_id(input: QueryByIdInput) -> ExternResult<Option<ContentOutput>> {
    let anchor = StringAnchor::new("content_id", &input.id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor_hash, LinkTypes::IdToContent)?
            .build(),
    )?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;
        get_content(action_hash)
    } else {
        Ok(None)
    }
}

/// Get content by content_type (using TypeToContent links)
#[hdk_extern]
pub fn get_content_by_type(input: QueryByTypeInput) -> ExternResult<Vec<ContentOutput>> {
    let anchor = StringAnchor::new("content_type", &input.content_type);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor_hash, LinkTypes::TypeToContent)?
            .build(),
    )?;

    let limit = input.limit.unwrap_or(100) as usize;
    let mut results = Vec::new();

    for link in links.iter().take(limit) {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        if let Some(output) = get_content(action_hash)? {
            results.push(output);
        }
    }

    Ok(results)
}

/// Get content by tag (using TagToContent links)
#[hdk_extern]
pub fn get_content_by_tag(tag: String) -> ExternResult<Vec<ContentOutput>> {
    let anchor = StringAnchor::new("tag", &tag);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor_hash, LinkTypes::TagToContent)?
            .build(),
    )?;

    let mut results = Vec::new();

    for link in links.iter().take(100) {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        if let Some(output) = get_content(action_hash)? {
            results.push(output);
        }
    }

    Ok(results)
}

/// List all content created by the current agent
#[hdk_extern]
pub fn get_my_content(_: ()) -> ExternResult<Vec<ContentOutput>> {
    let agent_info = agent_info()?;
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_info.agent_initial_pubkey, LinkTypes::AuthorToContent)?
            .build(),
    )?;

    let mut results = Vec::new();

    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        if let Some(output) = get_content(action_hash)? {
            results.push(output);
        }
    }

    Ok(results)
}

/// Get content statistics (counts by type)
#[hdk_extern]
pub fn get_content_stats(_: ()) -> ExternResult<ContentStats> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::Content.try_into()?);

    let records = query(filter)?;

    let mut by_type: HashMap<String, u32> = HashMap::new();

    for record in &records {
        if let Some(content) = record
            .entry()
            .to_app_option::<Content>()
            .ok()
            .flatten()
        {
            *by_type.entry(content.content_type).or_insert(0) += 1;
        }
    }

    Ok(ContentStats {
        total_count: records.len() as u32,
        by_type,
    })
}

// =============================================================================
// Learning Path Operations
// =============================================================================

/// Create a learning path
#[hdk_extern]
pub fn create_path(input: CreatePathInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let path = LearningPath {
        id: input.id.clone(),
        version: input.version,
        title: input.title,
        description: input.description,
        purpose: input.purpose,
        created_by: agent_info.agent_initial_pubkey.to_string(),
        difficulty: input.difficulty,
        estimated_duration: input.estimated_duration,
        visibility: input.visibility,
        path_type: input.path_type,
        tags: input.tags,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::LearningPath(path))?;

    // Create ID lookup link
    let anchor = StringAnchor::new("path_id", &input.id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, action_hash.clone(), LinkTypes::IdToPath, ())?;

    Ok(action_hash)
}

/// Add a step to a learning path
#[hdk_extern]
pub fn add_path_step(input: AddPathStepInput) -> ExternResult<ActionHash> {
    // Generate step ID
    let step_id = format!("{}-step-{}", input.path_id, input.order_index);

    let step = PathStep {
        id: step_id,
        path_id: input.path_id.clone(),
        order_index: input.order_index,
        step_type: input.step_type,
        resource_id: input.resource_id.clone(),
        step_title: input.step_title,
        step_narrative: input.step_narrative,
        is_optional: input.is_optional,
    };

    let action_hash = create_entry(&EntryTypes::PathStep(step))?;

    // Link path to step
    let path_anchor = StringAnchor::new("path_id", &input.path_id);
    let path_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_anchor))?;
    let path_links = get_links(
        GetLinksInputBuilder::try_new(path_anchor_hash, LinkTypes::IdToPath)?
            .build(),
    )?;

    if let Some(path_link) = path_links.first() {
        let path_action_hash = ActionHash::try_from(path_link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;
        create_link(path_action_hash, action_hash.clone(), LinkTypes::PathToStep, ())?;
    }

    // Link step to content (if resource exists)
    let resource_anchor = StringAnchor::new("content_id", &input.resource_id);
    let resource_anchor_hash = hash_entry(&EntryTypes::StringAnchor(resource_anchor))?;
    let content_links = get_links(
        GetLinksInputBuilder::try_new(resource_anchor_hash, LinkTypes::IdToContent)?
            .build(),
    )?;

    if let Some(content_link) = content_links.first() {
        create_link(action_hash.clone(), content_link.target.clone(), LinkTypes::StepToContent, ())?;
    }

    Ok(action_hash)
}

/// Get a learning path with all its steps
#[hdk_extern]
pub fn get_path_with_steps(path_id: String) -> ExternResult<Option<PathWithSteps>> {
    // Find path by ID
    let anchor = StringAnchor::new("path_id", &path_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let path_links = get_links(
        GetLinksInputBuilder::try_new(anchor_hash, LinkTypes::IdToPath)?
            .build(),
    )?;

    let path_link = match path_links.first() {
        Some(link) => link,
        None => return Ok(None),
    };

    let path_action_hash = ActionHash::try_from(path_link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;

    let path_record = get(path_action_hash.clone(), GetOptions::default())?;
    let path_record = match path_record {
        Some(r) => r,
        None => return Ok(None),
    };

    let path: LearningPath = path_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize path".to_string()
        )))?;

    // Get steps
    let step_links = get_links(
        GetLinksInputBuilder::try_new(path_action_hash.clone(), LinkTypes::PathToStep)?
            .build(),
    )?;

    let mut steps = Vec::new();
    for link in step_links {
        let step_action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid step action hash".to_string())))?;

        let step_record = get(step_action_hash.clone(), GetOptions::default())?;
        if let Some(record) = step_record {
            if let Some(step) = record
                .entry()
                .to_app_option::<PathStep>()
                .ok()
                .flatten()
            {
                steps.push(PathStepOutput {
                    action_hash: step_action_hash,
                    step,
                });
            }
        }
    }

    // Sort steps by order_index
    steps.sort_by_key(|s| s.step.order_index);

    Ok(Some(PathWithSteps {
        action_hash: path_action_hash,
        path,
        steps,
    }))
}

// =============================================================================
// Link Helper Functions
// =============================================================================

fn create_id_to_content_link(id: &str, target: &ActionHash) -> ExternResult<()> {
    let anchor = StringAnchor::new("content_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, target.clone(), LinkTypes::IdToContent, ())?;
    Ok(())
}

fn create_type_to_content_link(content_type: &str, target: &ActionHash) -> ExternResult<()> {
    let anchor = StringAnchor::new("content_type", content_type);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, target.clone(), LinkTypes::TypeToContent, ())?;
    Ok(())
}

fn create_tag_to_content_link(tag: &str, target: &ActionHash) -> ExternResult<()> {
    let anchor = StringAnchor::new("tag", tag);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, target.clone(), LinkTypes::TagToContent, ())?;
    Ok(())
}

fn create_author_to_content_link(target: &ActionHash) -> ExternResult<()> {
    let agent_info = agent_info()?;
    create_link(
        agent_info.agent_initial_pubkey,
        target.clone(),
        LinkTypes::AuthorToContent,
        (),
    )?;
    Ok(())
}

fn create_import_batch_link(import_id: &str, target: &ActionHash) -> ExternResult<()> {
    let anchor = StringAnchor::new("import_batch", import_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, target.clone(), LinkTypes::ImportBatchToContent, ())?;
    Ok(())
}
