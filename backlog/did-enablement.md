# DID Enablement Implementation Plan

**Status:** Backlog
**Priority:** Future
**Created:** 2024-12-17

---

## Overview

Implement W3C Decentralized Identifiers (DIDs) across the Elohim Protocol's "human experience pipeline" to distinguish content trust contexts:

| Access Tier | DID Pattern | Trust Level |
|-------------|-------------|-------------|
| Web2.0 Session | `did:web:gateway.elohim.host:session:{sessionId}` | Ephemeral, lowest |
| Hosted Human | `did:web:hosted.elohim.host:humans:{humanId}` | Custodial, medium |
| Native (via gateway) | `did:web:{operator-domain}:agents:{agentPubKey}` | Self-sovereign, high |
| Native P2P (future) | `did:key:{base58-pubkey}` | Cryptographic, highest |

### Purpose

DIDs enable users to:
- Identify content provenance (proxy-gateway vs native Holochain)
- Make trust decisions based on content source
- Share content from Holochain back to Web2.0 via known gateways
- Track identity across sovereignty upgrades (session → hosted → native → P2P)

---

## Current State

### Already in Place (Not Active)

- **Agent model** has optional `did?: string` field with migration path documented
- **Verifiable Credentials** model aligned with W3C spec using DIDs
- **Holochain DNA** Agent entry has `did: Option<String>` and `holochain_agent_key`
- **Content entries** have `source_path` and `metadata_json` for extensible provenance

### Missing

- DIDs aren't being populated anywhere currently
- No DID resolution logic
- No did:holo method spec
- No explicit proxy vs native origin tagging

---

## Phase 1: DID Service Foundation

**Goal:** Create centralized DID generation and management.

### New File: `elohim-app/src/app/elohim/services/did.service.ts`

```typescript
export interface DIDGenerationOptions {
  trustTier: 'session' | 'hosted' | 'native' | 'operator';
  humanId?: string;
  sessionId?: string;
  agentPubKey?: string;
  operatorDomain?: string;
}

export interface DIDDocument {
  '@context': string[];
  id: string;
  controller?: string;
  verificationMethod?: VerificationMethod[];
  authentication?: string[];
  assertionMethod?: string[];
  service?: ServiceEndpoint[];
  'elohim:trustContext'?: TrustContext;
}

@Injectable({ providedIn: 'root' })
export class DIDService {
  generateDID(options: DIDGenerationOptions): string;
  generateDIDDocument(did: string, agentPubKey?: string): DIDDocument;
  parseDID(did: string): ParsedDID;
  getTrustTier(did: string): TrustTier;
  validateDID(did: string): ValidationResult;
}
```

### Modify: `elohim-app/src/app/imagodei/services/identity.service.ts`

- Inject `DIDService`
- Generate DID on identity mode changes
- Add to `IdentityState`:
  ```typescript
  did: string | null;
  didDocument: DIDDocument | null;
  didHistory: string[];  // For migration tracking
  ```

### Modify: `elohim-app/src/app/imagodei/models/identity.model.ts`

- Add DID fields to `IdentityState` interface

---

## Phase 2: Admin-Proxy DID Integration

**Goal:** Proxy generates and serves DID Documents, includes DID in JWT tokens.

### Modify: `holochain/admin-proxy/src/jwt.ts`

```typescript
export interface TokenPayload {
  humanId: string;
  agentPubKey: string;
  identifier: string;
  did: string;  // NEW
  trustTier: 'hosted' | 'native';  // NEW
  version: number;
}
```

### Modify: `holochain/admin-proxy/src/auth-service.ts`

- Generate DID during registration
- Include DID in auth responses

### New File: `holochain/admin-proxy/src/did-routes.ts`

```typescript
// DID Document resolution endpoints
GET /.well-known/did.json           // Root proxy DID Document
GET /humans/:humanId/did.json       // Individual human DID Documents
GET /sessions/:sessionId/did.json   // Session DID Documents (ephemeral)
```

### Modify: `holochain/admin-proxy/src/index.ts`

- Register DID routes

---

## Phase 3: DNA Integration

**Goal:** Holochain DNA stores and indexes DIDs.

### Modify: `holochain/dna/lamad-spike/zomes/content_store_integrity/src/lib.rs`

Add `did` field to `Human` struct (Agent already has it):

```rust
pub struct Human {
    // ... existing fields ...
    pub did: Option<String>,  // NEW: W3C DID
}
```

Add link type:
```rust
DIDToHuman,  // For DID-based human lookup
```

### Modify: `holochain/dna/lamad-spike/zomes/content_store/src/lib.rs`

Add zome functions:

```rust
#[hdk_extern]
pub fn set_human_did(input: SetHumanDIDInput) -> ExternResult<HumanOutput>

#[hdk_extern]
pub fn get_human_by_did(did: String) -> ExternResult<Option<HumanOutput>>
```

### Modify: `holochain/sdk/src/types.ts`

- Add `did` to `Human` interface
- Add `did` to `CreateHumanInput`

---

## Phase 4: Content Attribution

**Goal:** Use DIDs for content authorship.

### Modify: `elohim-app/src/app/elohim/models/holochain-connection.model.ts`

```typescript
export interface HolochainContent {
  // ... existing ...
  author_did: string | null;  // NEW: Explicit DID attribution
}
```

### Modify: `elohim-library/projects/elohim-service/src/services/standards.service.ts`

- Update `generateDid` to support agent DIDs
- Add `generateAgentDid(humanId, trustTier)` function

---

## Phase 5: DID Resolution

**Goal:** Enable DID resolution from web and P2P contexts.

### New File: `holochain/admin-proxy/src/did-resolver.ts`

```typescript
export class DIDResolver {
  async resolve(did: string): Promise<DIDDocument | null>;
  private resolveWebDID(did: string): Promise<DIDDocument>;
  private resolveKeyDID(did: string): Promise<DIDDocument>;
}
```

### Resolution Algorithm

```
did:web:hosted.elohim.host:humans:abc123
    → https://hosted.elohim.host/humans/abc123/did.json
    → Fetch, cache, verify
```

---

## Phase 6: Migration Flow

**Goal:** Smooth DID transition when users upgrade sovereignty.

```
Session (did:web:gateway.elohim.host:session:xyz)
    ↓ [User registers]
Hosted (did:web:hosted.elohim.host:humans:abc123)
    ↓ [User exports keys]
Native (did:web:node.alice.community:agents:uhCAk...)
    ↓ [Future]
P2P (did:key:z6Mkq...)
```

### Modify: `elohim-app/src/app/imagodei/services/identity.service.ts`

- Add `migrateDID(newTier: TrustTier): Promise<string>`
- Create migration attestation linking old and new DIDs

---

## DID Document Schema

```json
{
  "@context": [
    "https://www.w3.org/ns/did/v1",
    "https://w3id.org/security/suites/ed25519-2020/v1",
    "https://elohim-protocol.org/ns/v1"
  ],
  "id": "did:web:hosted.elohim.host:humans:human-uhCAk12345",
  "controller": "did:web:hosted.elohim.host:humans:human-uhCAk12345",

  "verificationMethod": [{
    "id": "did:web:hosted.elohim.host:humans:human-uhCAk12345#holochain-key",
    "type": "Ed25519VerificationKey2020",
    "controller": "did:web:hosted.elohim.host:humans:human-uhCAk12345",
    "publicKeyMultibase": "z6Mkq12345..."
  }],

  "authentication": ["...#holochain-key"],
  "assertionMethod": ["...#holochain-key"],

  "service": [
    {
      "id": "...#elohim-profile",
      "type": "ElohimProfile",
      "serviceEndpoint": "https://hosted.elohim.host/api/v1/agents/..."
    },
    {
      "id": "...#holochain-gateway",
      "type": "HolochainGateway",
      "serviceEndpoint": "wss://hosted.elohim.host/app/4445"
    }
  ],

  "elohim:trustContext": {
    "tier": "hosted",
    "keyLocation": "custodial",
    "sovereigntyStage": "hosted"
  }
}
```

---

## Critical Files Summary

| File | Change Type | Purpose |
|------|-------------|---------|
| `elohim-app/src/app/elohim/services/did.service.ts` | NEW | Core DID generation/resolution |
| `elohim-app/src/app/imagodei/services/identity.service.ts` | MODIFY | Integrate DID into identity state |
| `elohim-app/src/app/imagodei/models/identity.model.ts` | MODIFY | Add DID fields |
| `holochain/admin-proxy/src/jwt.ts` | MODIFY | Add DID to JWT payload |
| `holochain/admin-proxy/src/auth-service.ts` | MODIFY | Generate DID on registration |
| `holochain/admin-proxy/src/did-routes.ts` | NEW | DID Document endpoints |
| `holochain/admin-proxy/src/index.ts` | MODIFY | Register DID routes |
| `holochain/dna/lamad-spike/zomes/content_store_integrity/src/lib.rs` | MODIFY | Add `did` to Human struct |
| `holochain/dna/lamad-spike/zomes/content_store/src/lib.rs` | MODIFY | Add DID zome functions |
| `holochain/sdk/src/types.ts` | MODIFY | Add DID to TypeScript types |

---

## References

- [W3C DID Core Spec](https://www.w3.org/TR/did-core/)
- [W3C Verifiable Credentials](https://www.w3.org/TR/vc-data-model/)
- [did:web Method Spec](https://w3c-ccg.github.io/did-method-web/)
- [did:key Method Spec](https://w3c-ccg.github.io/did-method-key/)
- Existing Agent model: `elohim-app/src/app/elohim/models/agent.model.ts`
- Existing VC model: `elohim-app/src/app/elohim/models/verifiable-credential.model.ts`
