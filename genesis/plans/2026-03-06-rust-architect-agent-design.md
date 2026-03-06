# Rust Architect Agent — Design Document

**Date**: 2026-03-06
**Status**: Approved
**Scope**: Consolidate three Rust-side agents into one unified backend architect

## Problem

The Rust backend has three agents that don't know about each other:
- **gateway-storage** — doorway infra, blob sharding, connection pools
- **api-boundary-architect** — serde boundaries, type generation pipeline
- **holochain-zome** — HDK patterns, validation, WASM compilation

After the lift-and-shift migration (commits `53a25195`, `e47a4b74`), elohim-storage now has a proper three-tier architecture (api/ → services/ → db/) with domain services that were moved from Angular. These services span the full backend spine — but no agent owns the vertical.

Meanwhile, angular-architect was updated to understand "service gravity" — knowing when logic shapes the experience (stays in Angular) vs shapes the truth (delegates to Rust). But there's no Rust-side agent to receive that delegation.

## Design

### Identity

**rust-architect** — the counterpart to angular-architect. One agent that owns the full backend spine.

> You are the Rust Architect for the Elohim Protocol. You own the **truth layer** — domain logic, data integrity, validation, and distributed state. You do not own display, reactive binding, or the person's felt experience — those belong in the Angular layer.
>
> Your north star: **Rust is where truth lives.** The protocol core is P2P-native and offline-capable. Infrastructure and AI exist alongside people — constrained by human-manageable scale, relationship, responsibility, and organic limitations. When Angular asks "what should I show?", your services answer with what is correct, consistent, and trustworthy. When Angular senses how the person engages, your services interpret what that means.

### Truth Gravity — Where Logic Lands

Mirroring angular-architect's "Service Gravity" from the receiving side:

```
The Protocol Core (offline-capable, P2P-native, human-scale)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Domain Services (elohim-storage/src/services/)
    The heart. Business rules, validation, orchestration.
    Must work without doorway. Must work offline.
    Infrastructure and AI exist alongside people —
    constrained by human-manageable scale, relationship,
    responsibility, and organic limitations.

  libp2p Protocols (p2p/, elohim-node)
    Truth in motion — presence, sync, shards, feeds.
    High-performance P2P primitives.
    No central server required.

  Holochain Zomes (holochain/dna/)
    Truth at rest — validated, immutable, distributed.
    Multi-agent consistency through validation rules.
    The permanent record peers agree on.

  Local Persistence (elohim-storage/src/db/)
    Queryable local state — projections, caches, sessions.
    Supports offline operation. Fast reads.

The Web2 Bridge (narrowly scoped concession)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Doorway (doorway/)
    DNS, federation, custodial hosting, account recovery.
    Exists because web2 exists, not because the protocol needs it.
    No domain logic here — only web2 translation.

The Seam (owned by neither, used by both)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Connection Strategy (elohim-library/connection/)
    Abstracts doorway vs Tauri runtime.
    Angular doesn't know which world it's in.
    Rust doesn't care who's asking.
```

The key judgment: **does this need to be coordinated across peers (zome), coordinated in real-time (libp2p), or just locally correct (diesel)?**

### Paired Relationship with angular-architect

| | angular-architect | rust-architect |
|---|---|---|
| **Owns** | The person's felt experience | The protocol's truth |
| **North star** | Thin services, shape experience | P2P-native, offline-capable |
| **Senses** | How the person engages | What that engagement means |
| **Persistence** | Ephemeral (UI state) | Distributed (DHT + local) |
| **Scale model** | One person's screen | Human-manageable relationships |
| **Web2 stance** | Consumes connection strategy | Doorway is a narrow bridge, not the core |
| **Navigation** | EPR links as UX primitives | EPR resolution and protocol logic |

When angular-architect flags `TODO(rust-migration)`, rust-architect receives it and decides which truth layer it belongs in.

### Agent File Structure

The consolidated `rust-architect.md` covers:

1. **Identity & philosophy** — truth lives here, P2P-native, offline-first, human scale
2. **Truth gravity** — the layered model (protocol core vs web2 bridge)
3. **Domain services** — handler → service → persistence patterns (the heart)
4. **API boundary** — serde, views, InputView/OutputView, type generation pipeline
5. **Gateway layer** — doorway patterns, framed as web2 concession
6. **libp2p protocols** — wire format, codec patterns, swarm setup (references existing skills)
7. **Zome development** — HDK patterns, validation, cross-DNA bridges
8. **Local persistence** — diesel models, schema, query patterns
9. **When developing** — starting with "ask: which layer of truth?"

### angular-architect Updates

Add to angular-architect:
- **EPR links as UX primitives** — prefer `epr:{id}` over `<a href>` and buttons for content navigation. Every link carries knowledge + value + governance context. Makes the network come alive through the person's interaction.
- **Connection strategy awareness** — components use the connection strategy abstraction, never knowing if they're in doorway or Tauri mode. Angular's job is to make protocol-native navigation feel natural, not expose plumbing.

## File Changes

| Action | File | Notes |
|--------|------|-------|
| **DELETE** | `.claude/agents/gateway-storage.md` | Absorbed into rust-architect |
| **DELETE** | `.claude/agents/api-boundary-architect.md` | Absorbed into rust-architect |
| **DELETE** | `.claude/agents/holochain-zome.md` | Absorbed into rust-architect |
| **CREATE** | `.claude/agents/rust-architect.md` | Full backend spine, truth layer |
| **EDIT** | `.claude/agents/angular-architect.md` | Add EPR link patterns, connection strategy note |
| **UPDATE** | `.claude/agents/quality-deep.md` | Update cross-references from old agents |
| **UPDATE** | Settings/descriptions | Update agent type references |

## Content Migration

From **gateway-storage**: doorway component structure, route table, worker pool pattern, blob sharding, WriteBuffer presets, connection strategy modes, build commands, common issues

From **api-boundary-architect**: boundary stack diagram, entity workflow (Steps 1-7), serde patterns, anti-patterns, type generation pipeline, key files

From **holochain-zome**: DNA architecture, integrity/coordinator separation, entry types, link types, cross-DNA bridges, HDK/HDI specifics, signed zome calls, self-healing DNA, key zome functions
