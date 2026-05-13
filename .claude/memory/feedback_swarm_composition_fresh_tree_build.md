---
name: After editing swarm composition, do a fresh-tree cargo build before committing
description: just check on a DNA worktree does not verify that crate-level type/field references resolve on a clean checkout of dev; for elohim-storage swarm/behaviour edits, run cargo build on the actual storage crate from a clean state
type: feedback
originSessionId: 7cbbcc6f-985c-471c-9b04-22720c83ef2a
---
When touching libp2p swarm composition (NetworkBehaviour derived structs, event variant mappings, `From<FooEvent>` impls), **run `cargo build` on `elohim-storage` from a clean tree** before committing. Do not rely on `just check` against a DNA worktree — that verifies a different workspace and doesn't catch crate-level field/variant references.

**Why:** In the M3 recovery session, Tasks 14-15 referenced `ElohimStorageBehaviour.gossipsub` and `ElohimStorageBehaviourEvent::Gossipsub` under the assumption the field + variant were already declared on dev. They were only in the working tree of a parallel session — uncommitted. My commits built locally because my working tree had both sets of changes, but `dev` HEAD did not compile on a clean checkout. A parallel agent had to ship an "unbreak dev HEAD" bundle that included the missing foundation (feature flag, field, variant, `From` impl, subscribe) that I'd tacitly assumed.

**How to apply:**
- Before committing any edit that references `swarm.behaviour_mut().<foo>` or `ElohimStorageBehaviourEvent::<Foo>(...)`, stash uncommitted changes on parallel work, verify `cargo build` passes on the elohim-storage crate from a clean state, then re-apply and commit.
- Run `cargo fmt --check` and `cargo clippy -- -D warnings` locally before `git push` — the husky pre-push hook blocks push if either fails, and fixing them after-the-fact on dev requires a follow-up cleanup commit that looks like debt.
- Multi-session concurrent work on the same repo is the dominant failure mode here — other agents may be editing the same swarm file in a working tree you can't see. Treat the swarm composition file as high-coordination surface.
