---
id: "backlog-retire-unwired-doorway-media-signaling"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Prepare deletion of the unwired doorway media signaling layer"
slug: "retire-unwired-doorway-media-signaling"
written: "2026-08-06"
author: "codex"
status: "wip"
priority: "medium"
relatedNodeIds: []
tags: [iroh, wave-2, doorway, media, retirement, peer-plane]
shift_objective: |
  Prepare the separately-ratified removal of the zero-dispatch WebRTC media
  session module while preserving the capability's future peer-plane home.
---

# Retire unwired doorway media signaling

Claimed by Codex on 2026-08-06 from relay-sovereignty design §6.1 and operator
checklist item 10.

## Claim fence

- `doorway/doorway-service/src/signal/media.rs`
- the media declaration/re-export documentation in `src/signal/mod.rs`
- the stale `MediaCmd` documentation reference in `src/services/recording.rs`
- this backlog claim

The live SBD server, HTTP dispatch, ZomeCaller boundary, and any new media
implementation are outside this claim.

## Prepared decision diff

The uncommitted diff deletes the 440-line module, its declaration/re-exports,
and the only non-module symbol reference (a recording-stub doc line). Repository
search found no route, dispatch, or application consumer. The recording stub now
records the retained concern explicitly: a future media-session implementation
belongs on the peer plane (iroh/EPR), not as a doorway SBD extension.

This deletion must not be committed until checklist item 10 is ratified. The
doorway Rust gate must then pass on the exact deletion diff.
