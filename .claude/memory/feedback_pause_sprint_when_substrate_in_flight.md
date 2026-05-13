---
name: Pause sprint depth when substrate change is in flight
description: When base images / cluster state are mid-rebuild, surface fixes only — don't dive into investigation that the new substrate may invalidate
type: feedback
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
When an infra change is in flight that will reset the substrate the pipeline runs on (e.g. base image rebuild, k8s upgrade, cluster restart, helm chart roll-forward), DO NOT dive deep into investigating cascade failures or kicking off speculative fixes. Apply the minimum revert to keep things runnable, then wait.

**Why:** On 2026-05-09 during an orchestrator clean-cascade shift, the user paused a ci-investigator dispatch with: "the rust-nix-dev container is building now, so we'll reboot once that clears, so we don't want to get too deep into any sprint until our infro shakes this change out." The current failure (CARGO_TARGET_DIR breaking DNA pack) was real, but the planned replacement (sccache via Garage S3) was already in motion as part of the image rebuild — investigating-then-fixing the CARGO_TARGET_DIR + sweettest-cache pairing in detail would have produced a fix that was about to be obsoleted by the sccache wiring.

**How to apply:**
- When the user mentions an in-flight image rebuild, helm change, k8s reset, cluster restart, or "infra is changing" → switch from full sprint loop to minimum-revert mode.
- Apply the smallest revert that restores known-working behavior. Don't speculate on the proper fix; the new substrate may make it moot.
- Save the architectural context for what the substrate change ENABLES (e.g. sccache binary in image + already-mounted Secret + Garage S3 bucket = caching coming back via different mechanism), so the followup work after the substrate lands is one-shot, not re-investigated.
- Don't run subagents that will go several layers deep into investigation — those token costs are wasted when the answer flips with the substrate.
- Surface a short summary back to the user of what was reverted, what the substrate change will enable, and what the followup is, so they can pick up the followup at the right time.
