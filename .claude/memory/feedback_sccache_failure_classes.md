---
name: feedback_sccache_failure_classes
title: sccache failure classes (umbrella)
description: "Two sccache failure classes: cache-corruption null-byte/unclosed-delimiter and intermittent spawn ENOENT; RUSTC_WRAPPER='' or heal sentinel."
metadata:
  type: feedback
---

# sccache failure classes (umbrella)

Folds the two distinct sccache failure-mode entries. Members:

- [[feedback_sccache_cache_corruption_recovery]] — 'unclosed delimiter'/null-byte = .sccache_check 404 leaked into rustc probe; an EMPTY bucket (a full wipe!) triggers it; fix RUSTC_WRAPPER='' or heal the sentinel.
- [[feedback_sccache_spawn_enoent_rca]] — cargo intermittently fails to spawn the sccache binary itself (~1.7%, matches sccache #2023/#2687); classifier grep = `could not execute process .sccache rustc`, NOT ENOENT.*build-script.
