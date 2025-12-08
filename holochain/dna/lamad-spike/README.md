# Lamad Spike DNA

Minimal Holochain DNA for testing browser-to-conductor connectivity.

## Purpose

This DNA exists to validate the web-first architecture:
1. Browser connects to Edge Node via WebSocket
2. Browser can create entries via zome calls
3. Browser can retrieve entries
4. Authentication and signing work correctly

## Zome: content_store

A minimal zome with two functions:

- `create_content(ContentInput) -> ActionHash` - Store a content entry
- `get_content(ActionHash) -> Option<Content>` - Retrieve a content entry

## Building

Requires Rust and Holochain HDK:

```bash
# Install Holochain dev tools
nix develop

# Build the DNA
hc dna pack .

# Package as hApp
hc app pack ./workdir
```

## Entry Type

```rust
#[hdk_entry_helper]
pub struct Content {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: Timestamp,
}
```

## Testing Flow

1. Start Edge Node: `docker compose up -d`
2. Build and install DNA
3. Connect from browser using HolochainClientService
4. Create content entry
5. Retrieve content entry
6. Verify round-trip

## Next Steps

Once this spike works, expand to full Lamad DNA with:
- ContentNode with all fields
- LearningPath entries
- Progress tracking
- Cross-references (links)
