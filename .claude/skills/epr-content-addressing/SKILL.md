---
name: epr-content-addressing
description: Reference for content linking in the Elohim Protocol — how links carry knowledge, value, and governance context, how they adapt to where the learner is, and how verified content addressing works. Use when authoring content links, explaining the architecture, creating content with verified fingerprints, or helping developers understand context-aware linking.
metadata:
  author: elohim-protocol
  version: 1.1.0
---

# Content Linking in the Elohim Protocol

When explaining these concepts, lead with the human experience. Technical terms are tools, not the point. See the Glossary at the bottom for plain-language definitions of every technical term.

## What Content Links Do Here

Every content link in this system carries three things:

1. **Knowledge** — what the content is, how it connects to other content
2. **Value** — who stewards it, how they're recognized
3. **Governance** — what access rules apply, which community ratified it

You cannot create a link without all three. This is structural — the system enforces it. The architecture makes it difficult to circulate knowledge without recognizing stewards, and difficult to apply rules without knowing what knowledge is affected.

### The commons default

Unattributed content doesn't vanish — it flows to the commons. When no steward attests, the constitutional governance layer takes responsibility and value flows to the community pool. This solves the cold-start problem: content enters and circulates immediately. When a steward later attests ("I wrote this," "I maintain this"), value redirects to them. The incentive is natural: find your content, attest to your care, and the value flows — otherwise it serves the world under community stewardship.

## Context-Aware Linking

The same link resolves differently depending on where the learner is:

- **In a learning path** containing that content: navigate to that step (stay in the journey)
- **In a different path**: cross-reference link to the other path
- **Browsing freely**: standalone resource view

This is the system's most novel feature. Traditional links always go to the same destination. These links adapt to context.

### How it works architecturally

The resolver is a pure function — it never fetches data itself. The caller passes in:
- What to resolve (`epr:fair-exchange`)
- Where the learner currently is (path ID, current steps)
- Where else this content appears (cross-path matches)

This keeps the protocol layer independent of the learning domain. The protocol doesn't know about learning paths — it just resolves based on the context it's given.

**Key file**: `elohim-app/src/app/elohim/services/epr-resolver.service.ts`

### Resolution priority

```
1. Current path contains the target → stay in path ("in-path")
2. Another path contains the target → cross-reference ("cross-path")
3. Neither → standalone resource view ("standalone")
```

## The Metadata Envelope

Every piece of content has a small (~500 byte) metadata envelope that can be shared by word-of-mouth networking between computers. It contains:

```
+--------------------------------------------------+
| "Foundations of Fair Exchange"                     |
|                                                    |
| Knowledge:                                        |
|   Type: concept    Format: article                |
|   Tags: economics, fairness                       |
|                                                    |
| Stewardship:                                      |
|   Steward: Alice    Recognition: 100%             |
|                                                    |
| Governance:                                       |
|   Access: open to everyone                        |
|   Authority: community-level                      |
|                                                    |
| Connections:                                      |
|   Teaches → "mutual credit"                       |
|   Requires → "value flows"                        |
|                                                    |
| Content fingerprint: bafyrei...                    |
+--------------------------------------------------+
```

The envelope serializes in a compact binary format for efficient transmission, but auto-detects and accepts plain text format too for backward compatibility.

**Key files**:
- TypeScript model: `elohim-app/src/app/elohim/models/epr-head.model.ts`
- TypeScript encoder/decoder: `elohim-app/src/app/elohim/utils/epr-codec.ts`
- Rust encoder/decoder: `holochain/elohim-storage/src/epr_codec.rs`

## Content Link Format

Content links use the `epr:` prefix followed by the content's readable name:

```
epr:manifesto                    — the manifesto
epr:fair-exchange@2              — version 2
epr:manifesto/head               — just the metadata envelope
epr:manifesto#section-3          — a specific section
epr:manifesto?via=doorway.host   — transport hint
```

Parsing:
```typescript
import { parseEpr, eprToRoute } from '@app/elohim';

const ref = parseEpr('epr:fair-exchange@2#section-3');
// { id: 'fair-exchange', version: '2', fragment: 'section-3' }
```

**Key file**: `elohim-app/src/app/elohim/utils/epr-ref.ts` (40+ tests)

## Content Verification

Content is named by its fingerprint — a unique identifier computed from the content itself. If someone tampers with the content, the fingerprint won't match, and you know.

The browser verifies fingerprints using a lazy-loaded library (doesn't slow initial page load). It tries verified retrieval first (5-second timeout), then falls back to a standard download. This is invisible to the user.

Only fingerprint-style addresses (starting with `bafk...`) get verified. Older-style addresses skip straight to standard download.

### The blob fetcher abstraction

Content retrieval is abstracted behind an interface so it can be swapped and tested:

```typescript
// Production: uses verified retrieval with standard download fallback
// Tests: use a mock
import { BLOB_FETCHER } from '@app/elohim';

// In tests:
{ provide: BLOB_FETCHER, useValue: mockBlobFetcher }
```

**Key files**:
- Interface: `elohim-app/src/app/elohim/interfaces/blob-fetcher.interface.ts`
- Implementation: `elohim-app/src/app/elohim/services/helia-fetch.service.ts`

## Three Content Tiers

Content comes in three sizes, each optimized for different use:

| Tier | What it is | Typical size | Used for |
|------|-----------|-------------|----------|
| **Envelope** | Metadata label | ~500 bytes | Discovery, previews, word-of-mouth sharing |
| **Document** | Full content | 5-50 KB | Reading and rendering |
| **File** | Raw bytes | Any size | Images, media, interactive apps |

## Authoring Content Links

### In page templates

```html
<!-- Card with title, description, hover preview -->
<app-epr-link epr="epr:manifesto" display="card"></app-epr-link>

<!-- Inline text link -->
<app-epr-link epr="epr:fair-exchange" display="inline"></app-epr-link>
```

**Key file**: `elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts`

### In written content (markdown)

Content links in markdown are auto-detected:

```markdown
Learn about [fair exchange](epr:fair-exchange) to understand
how communities can share resources without exploitation.
```

**Key file**: `elohim-app/src/app/lamad/renderers/markdown-renderer/markdown-renderer.component.ts`

### In code

```typescript
import { EprResolverService } from '@app/elohim';

// Resolve a link to a URL (synchronous, no network)
const url = this.eprResolver.resolveUrl('epr:manifesto').url;

// Fetch full content metadata (async)
this.eprResolver.resolve('epr:manifesto').subscribe(resolved => {
  // resolved.content.title, resolved.blobUrl, resolved.route
});

// Context-aware resolution
const result = this.eprResolver.resolveInContext('epr:fair-exchange', pathId, steps);
// result.resolution = 'in-path' | 'cross-path' | 'standalone'
```

### Creating linkable content

Give content a readable name (slug). That name becomes its permanent address:

```json
{ "id": "fair-exchange", "title": "Foundations of Fair Exchange", "contentType": "concept" }
```

Reference it as `epr:fair-exchange` anywhere in the system.

## Interfaces and Abstractions

The system uses swappable interfaces, following the same pattern as the connection strategy:

| Token | What it abstracts | Default behavior |
|-------|-------------------|-----------------|
| `CONNECTION_STRATEGY` | How to reach the backend | Auto-detects (gateway, direct, desktop) |
| `BLOB_FETCHER` | How to retrieve content files | Verified retrieval with standard fallback |
| `ELOHIM_CLIENT` | Backend communication | Mode-aware routing |

### Resolver interfaces

Two interfaces formalize the split between pure logic and network calls:

```typescript
// Pure logic — no network, synchronous
interface IEprUriResolver {
  resolveUrl(input, blobHash?): ResolvedEpr;
  resolveBlobUrl(hash): string;
  resolveInContext(input, pathId, steps, cross?): ContextResolvedRoute;
}

// Network calls — async
interface IEprContentResolver {
  resolve(input): Observable<ResolvedContent | null>;
  resolveEprHead(input): Observable<EprHead | null>;
}
```

**Key file**: `elohim-app/src/app/elohim/interfaces/epr-resolver.interface.ts`

## Metadata Envelope Wire Format

The envelope requests include a content-type preference header. The gateway forwards this to storage, which returns the appropriate format:

```
Prefer compact binary → compact binary response
Prefer plain text     → plain text response (fallback)
```

The decoder auto-detects: if the first byte looks like plain text, it parses as text. Otherwise, it parses the compact binary. This ensures backward compatibility.

**Key files**:
- Gateway proxy: `doorway/src/routes/epr.rs`
- Storage encoder: `holochain/elohim-storage/src/epr_codec.rs`

## Key Files

| File | Purpose |
|------|---------|
| `elohim-app/.../utils/epr-ref.ts` | Content link parsing and route generation |
| `elohim-app/.../utils/epr-codec.ts` | Metadata envelope encode/decode |
| `elohim-app/.../models/epr-head.model.ts` | Metadata envelope data model |
| `elohim-app/.../services/epr-resolver.service.ts` | Link resolution (pure + network) |
| `elohim-app/.../services/helia-fetch.service.ts` | Verified content retrieval |
| `elohim-app/.../interfaces/blob-fetcher.interface.ts` | Content retrieval abstraction |
| `elohim-app/.../interfaces/epr-resolver.interface.ts` | Resolver abstractions |
| `elohim-app/.../components/epr-link/epr-link.component.ts` | Content link component |
| `elohim-app/.../components/epr-popover/epr-popover.component.ts` | Three-pillar hover preview |
| `holochain/elohim-storage/src/epr_codec.rs` | Rust metadata envelope codec |
| `doorway/src/routes/epr.rs` | Gateway proxy for metadata envelopes |
| `genesis/docs/.../protocol-specification.md` | Full protocol specification |
| `genesis/docs/.../epr-developer-guide.md` | Architecture guide (accessible version) |

## Common Tasks

### Add a content link to a template
1. Import `EprLinkComponent` in the component
2. Use `<app-epr-link epr="epr:{name}" display="card">` or `display="inline"`

### Test without a running backend
```typescript
TestBed.configureTestingModule({
  providers: [
    { provide: BLOB_FETCHER, useValue: { fetchVerified: jasmine.createSpy().and.resolveTo(new Uint8Array()) } },
    { provide: StorageClientService, useValue: mockStorage },
  ],
});
```

### Verify the compact binary wire format
```bash
curl -H 'Accept: application/vnd.ipld.dag-cbor' \
  http://localhost:8888/api/epr-head/manifesto-foundations | xxd | head
```

---

## Glossary

| Codebase term | Plain meaning |
|---------------|---------------|
| Blob | A raw content file (image, document, interactive app) |
| Commons default | Unattributed content flows to the commons — governed by the community until a steward attests |
| CID / content fingerprint | A unique name computed from the content itself — if the content changes, the name changes |
| DAG-CBOR | A compact binary format for metadata envelopes (like shorthand — same info, smaller package) |
| DHT gossip | Word-of-mouth networking — computers share small messages with neighbors |
| Doorway | A gateway server that connects browsers to the peer network |
| EPR | Elohim Protocol Reference — the content link format (`epr:content-name`) |
| EPR Head | The metadata envelope — the "label on the book" |
| Helia | Browser library that verifies content fingerprints |
| InjectionToken | An Angular pattern for swappable dependencies (lets tests use mocks) |
| IPFS / IPLD | Peer-to-peer content standards this system builds on |
| Lamad | "To learn" — the knowledge pillar |
| Qahal | "Assembly" — the governance pillar |
| Shefa | "Abundance" — the value/stewardship pillar |
| Three pillars | Every content link carries knowledge + value + governance context |
| Tier | Content size level: envelope (~500B), document (5-50KB), file (any) |
