---
title: Using scoped native reconciliation and acceptance
id: acceptance-cli-usage
status: active
class: process-meta
cites:
  - "acceptance-plan | Acceptance-aware native reconciliation | sha256:d794ff0ca171a660 | path: genesis/docs/superpowers/plans/2026-09-09-acceptance-aware-reconciliation.md"
---

Read the declared source scope with `epr flow context <source-path> --json`; omit `--json` for the human readout. A source content CID also works, but an arbitrary Commitment CID is not a scope query. After editing source behavior, re-project and inspect historical candidates; matching labels do not transfer acceptance. Use JSON facets to identify exactly which production, review or acceptance is missing.

Register the implementer before claiming work:

```sh
epr actor claim --as agent:implementer@gpt-6 --session implementation-session --json
epr flow claim --on <gap-id> --session implementation-session --brief <plan-path> --json
```

For `flow claim`, omit `--as` when using a registered session: explicit names currently take precedence and lose the exact actor pin. If an earlier unpinned claim already exists, `--supersede` permits a new claim, but does not erase or durably supersede the old one. Retained uncertainty is visible in context.

Register a distinct acceptor with its own session. The operator/orchestrator appoints its returned actor claim CID to one exact commitment:

```sh
epr flow note --on <commitment-cid> --kind ruling --appoint <acceptor-claim-cid> --reason '<scope and intent>' --json
```

After production and independent technical approval, the acceptor tries the experience, writes the report JSON described by the cited contract, and records its own decision:

```sh
epr flow note --on <commitment-cid> --kind verdict --verdict approved --purpose acceptance --appointment <ruling-cid> --fulfillment <produce-cid> --review <technical-verdict-cid> --report <report-json-path> --session acceptance-session --reason '<observed fit and limits>' --json
```

Use `changes-requested` when the experience does not meet intent. Evidence must include the delivered report's exact body CID, and paths must resolve inside the repository. Re-read context after admission. Reports and evidence are immutable after pinning; a changed artifact requires fresh review and explicit acceptance supersession. Local appointment does not establish peer authority.

The exercised binary for this worktree is `/tmp/eprfs-gate-target/debug/epr`; the commands above assume `epr` resolves to that build. The independently pinned acceptance-report.json beside this guide records actual observations and known limits.
