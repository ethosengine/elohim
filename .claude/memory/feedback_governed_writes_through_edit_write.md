---
name: feedback-governed-writes-through-edit-write
description: "Compose gates match Edit|Write only; python/sed in Bash on backlog, research, specs or memory writes around every edit-time governance rule."
metadata:
  node_type: memory
  title: Governed writes go through Edit/Write
  type: feedback
  originSessionId: c94bb4e8-1d73-4a19-85d0-006adc1d8956
  modified: 2026-10-29T17:36:26.082Z
---

On 2026-10-29 (Nostr Compass survey mint pass) I wrote eleven backlog cluster rows and survey reframes
through `python3` heredocs in Bash. Every edit-time governance hook (`epr-meta-resolver`,
`sovereignty-guard-signal`, `blind-reader-review`) is registered on `Edit|Write` only, so none of them
saw those writes. The operator noticed that an absolutist authority claim had gone ungated. (The deeper
cause was that no rule covers that class; see `governance-terminal-authority-frame-guard-missing` in
the backlog.) The pre-push compose-gate is a backstop, but birth-time dispatch rules (the librarian
dedupe, cluster-first inject, blind-reader) fire at edit time.

**Why:** the compose gates are how directory-local governance reaches an agent at the moment of action.
Writing around them, even with the auto-mode allowance to edit by shell, removes the place where the
repository's own rules can speak.

**How to apply:** for any file under a directory with an `.epr-meta` (backlog, research, specs, canon
docs, memory), use Edit/Write, one call per insertion if needed. Use Bash only for reads, for
generated/projection files whose generator is the authority (`epr flow memory project --index` owns
MEMORY.md), and for ignored scratch. If a bulk shell write is unavoidable, replay the resolver
afterwards by feeding `.claude/hooks/epr-meta-resolver.py` a synthetic `Write` payload per file, and
say so. Related: [[feedback_human_loop_not_terminal_authority]], [[project_epr_meta_compose_gate]].
