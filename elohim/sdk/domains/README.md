# Protocol Domain Definitions

Each subdirectory is a protocol domain — a vocabulary that defines content types,
coupling declarations, metadata schemas, and signals for a pillar of the protocol.

Domains are part of the SDK. They enforce integrity — what signals the protocol
MUST see. Apps compose domain vocabulary into human experiences.

| Domain | Pillar | DNA Source | Purpose |
|--------|--------|------------|---------|
| lamad | Learning | elohim (content_store) | Concepts, paths, assessments, mastery |
| imagodei | Identity | imagodei | Humans, attestations, presence, relationships |
| shefa | Economy | elohim (content_store) | Economic events, stewardship, resources |
| qahal | Social + Governance | mishpat | Collectives, proposals, governance |
| avodah | Work | elohim (content_store) | Services, flow planning, insurance |

## Domain Artifacts

Each domain directory can contain three IoC artifact types:

```
elohim/sdk/domains/{domain}/
  manifest.json     ← domain vocabulary: content types, coupling, signals
  schemas/          ← JSON schemas for metadata per content type
  types/            ← Rust wire types crate (coordinator I/O)
  scripts/          ← codegen from manifest + schemas
```

### Wire Types (`types/`)

Rust crates defining the MessagePack-serialized inputs and outputs for
coordinator zome functions. These are the **compiler-enforced contract**
between DNAs and their consumers (doorway, storage, future clients).

See `CLAUDE.md` in this directory for the pattern, rules, and template.

## For App Developers

Import a domain manifest to build on its vocabulary. Your app manifest
references the domain and adds app-specific content types.

See `lamad/CLAUDE.md` for the reference pattern.
