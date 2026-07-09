---
id: "plan-crdt-content-dataplane-full1c-implementation"
status: "active"
cites:
  - genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md
  - genesis/data/timeline/backlog/crdt-authoritative-content-state-dht-notary-decouple.md
---

# Implementation Plan — CRDT-Authoritative Content Dataplane (full-1c-now + deploy-producer)

**Status:** ACTIVE plan · 2026-07-01
**Spec:** `genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md`
**Decisions locked:** OD-1 = full-1c-now (notarized declared-HEAD version DAG); OD-2 = deploy-time non-notarized producer. Analogy: CRDT:HTTP :: CRDT+DHT-notary:HTTPS.
**Origin:** grounded + sequenced + adversarially verified by the `plan-crdt-http-https-content-dataplane` workflow (7 agents, code-cited).

## Corrected build order (adversarial-verified)

```
A1 → A2 → A4 → A3   (Phase A: deploy-producer + amber-serve = the LIVE elohim.host 404 fix)
   → B1 → B2 → B3   (Phase B: CRDT-heal + L1 convergence, notary-independent)
   → C1 → C2 → C3 → C4 → C5  (Phase C: HEAD-DAG full-1c + notary overlay + author-sig)
```

## Adversarial corrections folded (do NOT lose these)

1. **A3 does not un-404 standalone.** `lookup_slug_blob_hash` passes `require_provenance=true` (`http.rs:5811`); an amber-only row 404s until A2's tri-state gate admits amber. The "earliest live fix" is the **A1+A2+A4+A3 bundle**. No class-A violation: Content is NOT in `is_integrity_kind` (`write_through.rs:207` = KeyRotation|KeyRevocation|RevocationAttestation|AgentPeerBinding), so a diesel-direct `blob_hash` write authors no DHT entry and no reconciler reverts it.
2. **A4 co-lands BEFORE A3:** never emit an amber content node to a client before the `trust:"unconfirmed"` label exists (`get_content_with_tags` at `http.rs:376,4659`).
3. **C2 must dual-write the ROOT `blobHash` scalar** alongside the new `versions` map through the mixed-version window, else an old-code peer reading a new-format doc (`get_doc_field(ROOT,"blobHash")`, `sync/mod.rs:167-190`) serves empty. Additive map keys mean the landed flat docs (08b284fc8) need no data migration — but backward-read compat is not free.
4. **C3 must preserve the "green stamp overwrites amber" precedence** A3 relies on when it replaces the recency `.set(dht_anchor_hash)` (`rea_projection.rs:684-700`) with HEAD-election.
5. **REQ-N5 author-sig (the "TLS"):** satisfied-by-tier-contract for Phases A/B — the amber write is permanently admin-gated (X-API-Key) and amber is never consumed for authority/economic/attribution reads (A2 asserts `list_content(Green)` excludes amber); a real Ed25519 sync-path author-sig is **C5** that upgrades amber→published(blue) for verified authors. Amber-before-C5 is the bounded HTTP-before-HTTPS window: functional, labeled unconfirmed, never authoritative.

6. **MULTI-ROOT CONVERGENCE — the false-green C3 does not close (live evidence 2026-07-09; folded from the notary-authority grounding).** C1/C3 assume a *single* chain per id where HEAD "advances only if supersedes." The live substrate falsifies that assumption: `elohim-host-landing` reads `trust:"notarized"` on BOTH alpha-A and elohim.host but over **divergent `dhtAnchorHash` AND divergent `blobHash`** — two genesis doorways each ran the null-anchor bootstrap branch and each `create_content`'d the same id under its **own agent key**, becoming root author of its **own** chain (`content_store` has no id-uniqueness at Create; `gather_content_chain` picks `records[0]` root-author with no deterministic sort). "supersedes" cannot relate two competing roots, and the reconcile heal loop **re-affirms each peer's own head by invariant** (`projection_reconcile.rs`: "we never adopt the peer's value") — so the split is a stable no-op, not convergence. The landed sweep-driven witness-bootstrap ("un-anchored rows earn a conductor-authored head", `b7e010214`) authors a head **per peer** — it makes rows green, it does NOT converge them; per-peer green must NOT be mistaken for done. **Correction:** Phase C needs an explicit **cross-peer canonical-HEAD election among competing root authorings** — a peer adopts another's head **iff** it carries the earned-authority proof (C5 reach-cohort membership + Ed25519 author/community sig) verified against the DHT-notary witness (verified adoption, never a blind copy, honoring the trust boundary). This is NOT first-writer-wins / id-reservation — that lock is a dev-time convenience, never the goal; the goal is earned, socially-derived, dynamically-negotiated authority (councils-of-Elohim / holonic sociocracy, see spec §5.6 + the reframed `genesis/a2o/features/dataplane/notary-authority.feature`). Convergence observable: **every federation peer resolves the SAME canonical head** (reframed scenario 58: `resolves the same canonical head across peers`), not merely per-peer `notarized`. Sequence with C3 (HEAD-election) and C5 (earned-authority gate); until the social-grant path exists, the promotion decision is a *labeled* god-mode dev stand-in on the clean-up-toward trajectory, not a network role.

---

## PHASE A — deploy-producer + amber-serve (LIVE fix; A1/A2/A4 Che-unit, A3 Che-unit + operator deploy)

- **A1 — `crdt_converged_at` amber column.** Migration `2026-07-01-120000_content_add_crdt_converged_at` (distinct ts, collision-guard), `crdt_converged_at Nullable<Text>` in `diesel_schema.rs` + `Content` model. Proof: model round-trip reads NULL. Reversible: drop dir + 2 lines.
- **A2 — tri-state `require_min_trust` gate.** `MinTrust{Invisible,Amber,Blue,Green}` replaces `require_provenance: bool` in `get_content`/`get_content_with_tags`/`list_content`/`count_content`. Internal call-sites → `Invisible`; serving → `Amber`; economic/attribution → `Green`. `lookup_slug_blob_hash` (`http.rs:5811`) → `Amber` = the REQ-F7 MUST. Proof: `serves_amber_row_at_slug_lookup` (amber row returned by Amber, excluded by Green). Per-row degrade, never fail-closed collect (REQ-N7).
- **A4 — `ContentView.trust` field (REQ-F10).** `pub trust: String` computed in `From<Content>` (notarized/published/unconfirmed) + `content_view_from_epr_head`; regen ts-rs + view schema + `INTERFACE_FILES`. Proof: `schema_contract` drift + byte-identical regen. **Co-lands before A3.**
- **A3 — deploy-producer diesel-direct amber write** ← fixes LIVE 404. Admin-gated amber marker on the PATCH; the `(true,None)` 503 branch (`http.rs:4965-4972`) becomes diesel-direct `services.content.update` stamping `blob_hash`+`crdt_converged_at`, never `dht_anchor_hash`; notary-present path unchanged; write-iff-`(local blob_hash NULL/empty)`. `stage-spa-blob.sh` passes the marker. Proof: `amber_patch_no_conductor_serves_200`. Live gate: operator redeploy.

## PHASE B — CRDT-heal + L1 convergence (Che-real-libp2p, plain cargo test)

- **B1 — reverse projector DocStore→SQL + empty-never-wins guard.** On doc apply, write `content.blob_hash` under `default_lamad` iff source non-empty AND local NULL/empty; stamp `crdt_converged_at`. Proof: `empty_never_overwrites_real_hash`.
- **B2 — amber write emits `ContentUpdated` → DocStore seeds real hash** → converges to peers via `spawn_content_projection_listener`. Proof: extend `sync_libp2p_convergence.rs` — B's DocStore carries the real hash, zero conductor bridge.
- **B3 — serve-path convergence proof.** Wire `TestSyncNode` to a SQLite content DB + B1; after convergence assert `get_content(MinTrust::Amber)` returns the converged blobHash (closes VERDICT-L2 #1: DocStore-only proof today). Proof: `converges_and_serves_zero_notary`.

## PHASE C — HEAD-DAG (full 1c) + notary overlay (sweettest/CI + operator; C2 Che-now)

- **C1 — DNA HEAD notarization (rust-architect + sweettest gate).** Category-A2 LINK (not new entry type): `Supersedes`/`ContentVersion` + `ContentHead` selector link on existing Content (EntryTypes 75/100 preserved). Coordinator `declare_content_head` + `ContentHeadDeclared` signal; HEAD-election advances only if supersedes. Proof: sweettest `declare_head_notarizes_and_supersedes`. Blocked-by-env (sweettest/DNA pipeline, post dev-merge).
- **C2 — CRDT doc version-DAG structure (+ROOT dual-write).** Grow-only `versions` map (key=content-addressed `versionCid`) + `head`/`headActionHash` scalar; **retain ROOT `blobHash` dual-write** for mixed-version compat; readers do head→versions[head], fall back to ROOT. Proof: `distinct_versions_coexist_head_notary_set`.
- **C3 — `declared_head_action_hash` column + HEAD-election replaces recency.** Replace unconditional `.set(dht_anchor_hash)` (`rea_projection.rs:684-700`) with a `ContentHeadDeclared`-aware arm; **preserve green-overwrites-amber precedence**. `resolve_head(id)` serves from it. Proof: `head_election_not_recency`.
- **C4 — notary-overlay serving (green padlock) + rebuild-from-L1.** Serve green over amber; rebuild SQL from converged L1. Proof: `rebuild_from_L1_serves_green_when_notarized`.
- **C5 — reach-cohort edit-membership + author-signature (the "TLS", REQ-N5/F12).** The sync-path "author-sig" is really *proof of membership in the origin EPR's reach+attestation edit-cohort* (spec §5.6): reach is inherited at fork (REQ-F11), the concurrent CRDT edit is open only to the co-authorized cohort (REQ-F12, resolves the poisoning vector — not open gossip), and reach is re-certified at republish via `declare_content_head` (REQ-F13). Ed25519 verifies cohort membership → upgrades verified amber to published(blue); enables non-admin peer heal. **OD-7:** the "agreement to republish" trigger (author-alone / cohort-quorum / governance) is a reach-gate-epic decision; the OD-2 deploy-producer is the trivial system-authored (admin-gated) republish case.

## Revocation (OD-6, from the CRL analogy) — cross-cutting, Phase C

Notary revocation dominates convergence: *authority-revocation* → downgrade to amber; *safety-revocation* → hard-suppress serving (HSTS-style). Needs a Content revocation arm in `reconcile/controller.rs` (absent today). Tracked; sequence with C3/C4.

## Env map

`A1–A4, B1–B3, C2` = Che-verifiable now (A3 live confirmation needs operator deploy). `C1, C3-e2e, C4-notarized, C5` = sweettest/DNA pipeline (operator, post dev-merge). All Phase A/B get CI only at operator dev-merge (`feat/*` not orchestrator-indexed).
