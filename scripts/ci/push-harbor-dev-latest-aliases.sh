#!/usr/bin/env bash
# push-harbor-dev-latest-aliases.sh — on the dev branch only, additionally tag
# + push each already-built component as `:dev-latest` (backward-compat alias
# consumed by manifests that track dev-latest rather than a per-build tag).
# Extracted 2026-07-30 from the Jenkinsfile's stage('Push to Harbor') to
# shrink the edge Jenkinsfile's CPS pipeline-block bytecode — see CLAUDE.md
# "Jenkinsfile Size Limit" (MethodTooLargeException, 64KB JVM method-size
# limit).
#
# The Jenkinsfile only invokes this script inside `if (env.BRANCH_NAME ==
# 'dev')` — the branch gate itself is NOT re-implemented here, matching the
# original which relied on the surrounding Groovy `if` rather than a bash
# check.
#
# Behavior is preserved EXACTLY from the original inline heredoc: five
# independent `if <image found> then tag+push else nothing fi` blocks, back
# to back, no `set -e`, no explicit exit anywhere. Each block's failure (or
# absence of the image) has zero effect on the others — a failed push for one
# component does not stop the tag/push attempt for the next. The script's own
# overall exit status is therefore whatever the LAST command of the LAST
# block (elohim-agent-sdk) leaves it, exactly as it was inline. Do NOT add
# set -e or early exits — that would make an earlier component's failure abort
# later ones, which the original never did.
#
# Env (already exported by the Jenkinsfile's withBuildVars/withEnv — read
# directly, never re-passed as argv):
#   IMAGE_TAG    the just-built local tag suffix, used for each `images | grep` check

if nerdctl -n k8s.io images | grep -q "elohim-edgenode.*${IMAGE_TAG}"; then
    nerdctl -n k8s.io tag elohim-edgenode:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-edgenode:dev-latest
    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-edgenode:dev-latest
fi

if nerdctl -n k8s.io images | grep -q "elohim-doorway.*${IMAGE_TAG}"; then
    nerdctl -n k8s.io tag elohim-doorway:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-doorway:dev-latest
    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-doorway:dev-latest
fi

if nerdctl -n k8s.io images | grep -q "elohim-storage.*${IMAGE_TAG}"; then
    nerdctl -n k8s.io tag elohim-storage:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-storage:dev-latest
    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-storage:dev-latest
fi

if nerdctl -n k8s.io images | grep -q "elohim-happ-installer.*${IMAGE_TAG}"; then
    nerdctl -n k8s.io tag elohim-happ-installer:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-happ-installer:dev-latest
    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-happ-installer:dev-latest
fi

if nerdctl -n k8s.io images | grep -q "doorway-app.*${IMAGE_TAG}"; then
    nerdctl -n k8s.io tag doorway-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/doorway-app:dev-latest
    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/doorway-app:dev-latest
fi

if nerdctl -n k8s.io images | grep -q "elohim-agent-sdk.*${IMAGE_TAG}"; then
    nerdctl -n k8s.io tag elohim-agent-sdk:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-agent-sdk:dev-latest
    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-agent-sdk:dev-latest
fi
