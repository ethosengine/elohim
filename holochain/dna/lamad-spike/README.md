# Lamad Spike DNA

Minimal Holochain DNA for testing browser-to-conductor connectivity and learning path coordination.

## Architecture

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

# Build the WASM zomes (IMPORTANT: must specify wasm32 target and RUSTFLAGS)
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown

# Pack the DNA (IMPORTANT: must repack after rebuilding WASM!)
hc dna pack .

# Package as hApp
hc app pack ./workdir
```

**Important build notes:**
1. `cargo build --release` without `--target wasm32-unknown-unknown` builds for native target and does NOT update the WASM files
2. After rebuilding WASM, you MUST run `hc dna pack .` to update the `.dna` bundle
3. After repacking, restart Holochain to load the new DNA

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

- ContentNode with all fields
- LearningPath entries
- Progress tracking via mastery events (see code comments in integrity zome)

## Self-Healing Schema Evolution

Lamad uses the RNA module's flexible healing architecture to handle schema migrations without external tools or downtime.

### How It Works

When the schema changes (new fields, renamed fields, etc.), the DNA can heal itself:

1. **Dual-role hApp**: Both v1 and v2 DNAs are bundled together
2. **Lazy healing**: On first read, entries are fetched from v1 via bridge call
3. **Automatic transformation**: v1 data is transformed to v2 schema
4. **Graceful degradation**: If healing fails, entries remain accessible but marked "Degraded"

### Architecture

```
zomes/content_store/src/
├── lib.rs                    # init() registers providers
├── providers.rs              # Entry type providers (ContentProvider, etc.)
├── healing_impl.rs           # V1→V2 transform functions
└── healing_integration.rs    # Read path healing glue
```

### Adding New Entry Types

To add a new entry type with healing support:

```rust
// 1. Create provider (in providers.rs)
pub struct QuizProvider;
impl EntryTypeProvider for QuizProvider {
    fn entry_type(&self) -> &str { "quiz" }
    fn validator(&self) -> &dyn Validator { &QuizValidator }
    fn transformer(&self) -> &dyn Transformer { &QuizTransformer }
    // ...
}

// 2. Register in init_flexible_orchestrator() (lib.rs)
registry.register(Arc::new(QuizProvider))?;
```

No changes needed to RNA framework, orchestrator, or other providers.

### Key Files

| File | Purpose |
|------|---------|
| `holochain/rna/rust/src/` | Core healing framework |
| `zomes/content_store/src/providers.rs` | Lamad entry type providers |
| `zomes/content_store/src/healing_impl.rs` | V1→V2 transformations |

## Vision & Design

For design rationale and governance policy, see:
- `docs/content/elohim-protocol/` - Elohim Protocol Manifesto
- `docs/content/lamad/` - Lamad learning system
- `zomes/content_store_integrity/src/lib.rs` - LinkTypes with inline comments
