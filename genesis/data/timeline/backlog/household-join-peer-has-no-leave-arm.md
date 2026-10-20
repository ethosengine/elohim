---
id: "backlog-household-join-peer-has-no-leave-arm"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "`just mesh join-peer` has no leave arm: a joined peer stays on the sandbox roster forever, `conductors-restart` revives a stopped joiner's conductor, and a later joiner's pidfiles can name an earlier joiner's processes"
slug: "household-join-peer-has-no-leave-arm"
written: "2026-10-20"
author: "overnight shift 2026-10-20 (ci-runner-story-green)"
status: "open"
priority: "medium"
jobs: [household-mesh]
tags: [hc-mesh, join-peer, household]
---

**Evidence (2026-10-20).** The ci-runner story needs a fourth peer, staged with `join-peer`. Three
were staged: ruth (stock conductor — the fork lives in a non-pin12 tools dir auto-detect skips),
naomi (main-tree bundle, so a different lamad DHT — now refused by join-peer's DNA check), and
orpah (the household's bundle + the fork; the one in use). Stopping ruth and naomi by pid left
them in `genesis/local-dev/household-dowell/conductors/.hc`; the next
`HOLOCHAIN_BIN=… just mesh conductors-restart` started their conductors again (no storage behind
them), and they had to be stopped a second time. `pids/conductor-naomi` and `pids/storage-naomi`
held ruth's pids, not naomi's.

**Cure.** A `just mesh leave-peer <name>` arm that stops the joiner's conductor and storage,
drops its `.hc` line (or marks it retired so roster indexes — and the ports derived from them —
do not shift for later joiners), and clears its pidfiles; plus a pidfile write keyed by the
joiner's own name. Until then, `.hc` lines 4 and 5 (ruth, naomi) are dead weight on this
household; orpah is the sixth `.hc` line (index 5) and serves on :8095.
