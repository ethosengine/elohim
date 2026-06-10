---
name: feedback_workflow_structuredoutput_hang
description: Workflow agents with schema:/forced StructuredOutput can hang forever on empty-payload retry loops in this container — prefer schemaless prose returns + a stall-watcher
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 9c897e0b-7586-45e5-8a21-c5a69a149b03
---

In this container's `Workflow` tool, agents given a `schema:` option (which forces a `StructuredOutput` tool call) can get stuck emitting `StructuredOutput` with an empty `{}` payload and retry indefinitely — observed 48 then **481** calls in one agent, no cap, hanging the whole run at a `parallel()`/phase barrier. The completion notification NEVER fires (the agent stays "alive," just looping), so a hung run is indistinguishable from a slow one.

**Why:** the default workflow subagent here cannot reliably emit forced-tool arguments for non-trivial schemas (many required fields, arrays). Bigger/awkward schemas make it worse; a prompt guard telling it to "shorten and retry" egged the loop on rather than stopping it. Root cause is the forced-tool path itself, not payload size — softening the schema did not fix it; removing `schema:` did.

**How to apply:**
1. **Author workflow agents SCHEMALESS** — return markdown prose with explicit `### headed sections` instead of `schema:`; synthesize from prose. A schemaless rerun of the same 11-agent design workflow finished clean in **~8.7 min** after two schema'd attempts each hung ~1h.
2. **Always pair a background `Workflow` with a stall-watcher** (`Bash` `run_in_background`: loop polling for the result file `/tmp/claude-0/.../tasks/<taskid>.output`, else emit a `STALL` line at ~1.5× healthy duration reporting the stuck-phase agent-file count). The harness only notifies on completion, never on hang — without a watcher the operator has to prod.
3. **Diagnose a suspected hang** by inspecting the workflow transcript dir (`.../subagents/workflows/wf_*/`): agent-`*.jsonl` file count = phase reached; an actively-growing agent jsonl with repeated `StructuredOutput` tool_use + `"does not match required schema"` results = the loop. `journal.jsonl` freezing while one agent file keeps growing is the signature.

**Second gotcha — workflow agents have Write access and will author files to the repo unprompted.** In a multi-cluster spec run, synthesis/design agents wrote spec docs straight into `genesis/docs/superpowers/specs/` *before* the critique phase ran — producing (a) un-prefixed duplicates of docs the main loop also wrote, and (b) pre-critique versions carrying a known `does-not-hold` error. Reconcile to ONE authoritative set after the workflow lands: keep the main-loop's critique-corrected versions, `rm` the agents' pre-critique duplicates (they're untracked), and fold the adversarial fixes before any doc is treated as canonical. Don't assume the workflow's return value is the only artifact — check `git status` for files the agents wrote.

Related: [[feedback_workflow_long_cargo_orphan_lock]] (other workflow/background-task failure mode).
