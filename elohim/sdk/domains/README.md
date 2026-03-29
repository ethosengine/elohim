# Protocol Domain Definitions

Each subdirectory is a protocol domain — a vocabulary that defines content types,
coupling declarations, metadata schemas, and signals for a pillar of the protocol.

Domains are part of the SDK. They enforce integrity — what signals the protocol
MUST see. Apps compose domain vocabulary into human experiences.

| Domain | Pillar | Purpose |
|--------|--------|---------|
| lamad | Learning | Concepts, paths, assessments, mastery |
| imagodei | Identity | Humans, attestations, presence, relationships |
| shefa | Economy | Economic events, stewardship, resources |
| qahal | Social + Governance | Collectives, proposals, relationships |

## For App Developers

Import a domain manifest to build on its vocabulary. Your app manifest
references the domain and adds app-specific content types.

See `lamad/CLAUDE.md` for the reference pattern.
