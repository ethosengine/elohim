---
name: Compute and model are independent diversity surfaces
description: Peer diversity has two distinct axes — compute (hardware, memory, storage) and model (which elohim runs, context size, specialties, skills). They correlate but must be independently observable and addressable.
type: project
originSessionId: 6ec4bfae-b3f0-4040-8a90-6ae504910fe7
---
Peer-status metadata must expose BOTH diversity axes independently:

**Compute diversity** (existing):
- Hardware class, memory, storage, bandwidth
- Uptime, reach level, peer-to-peer connectivity

**Model / elohim diversity** (the new axis):
- Whether elohim can run on this peer at all (some peers are pure-storage nodes, no model capacity)
- Which model is active: `claude-opus-4-7`, `claude-sonnet-4-6`, `llama-3.1-70b-quantized-q4`, etc.
- Model family and generation
- Context window size (bytes or tokens)
- Which constitution CID the elohim is primed with
- Declared specialties (domains the elohim is primed for)
- Declared skills (capability names it handles)
- Observed strengths (accrued from attestation history)
- Active-since timestamp
- Reputation/reach of this specific elohim instance (may differ from the peer's overall reach)

**Why the surfaces must stay independent:**

- A high-compute peer with no elohim (pure storage node) is useful for replication but not judgment
- A modest-compute peer running a constitutionally-well-tuned elohim may be MORE valuable for judgment than a big-iron peer with a generic model
- Gate-dispatch routing should be able to pick "best elohim for this contentType" independently of "peer with capacity to handle request right now"
- Reputation accrues to the elohim-substance (model+constitution+quantization), not to the peer — a peer can rotate elohim without losing node-level reputation

**How to apply:**

- PeerStatus view MUST carry an optional `elohimCapability` struct when the peer runs an elohim
- The struct is nullable (peer with no elohim declares `None`)
- Dispatch logic considers both surfaces — compute availability AND model fit for the judgment
- Attestation records the elohim-substance-CID (as Phase 4 already does), which composes (model + constitution + quantization + deployment context)

Flagged 2026-04-19 during Phase 9 planning. The user named compute and model as "independent surfaces" that correlate but must not collapse into one another.
