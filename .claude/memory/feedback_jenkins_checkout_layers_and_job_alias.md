---
id: feedback-jenkins-checkout-layers-and-job-alias
name: feedback_jenkins_checkout_layers_and_job_alias
description: "Jenkins checkout on this 316MB / 65k-object repo needs three layers (skipDefaultCheckout + shallow CloneOption + honorRefspec single-branch fetch) or it dies at ~10:15 on the implicit Declarative-checkout default timeout; and the multibranch job 'elohim-holochain' loads the DNA Jenkinsfile, not the edge one."
metadata:
  node_type: memory
  type: feedback
  originSessionId: 2026-04-30T22-30-orchestrator-781-recover
cites:
  - genesis/orchestrator/Jenkinsfile
  - elohim/holochain/dna/Jenkinsfile
---

**Jenkins checkout on this 316MB / 65k-object repo needs three layers; and `elohim-holochain` loads the DNA Jenkinsfile, not the edge one.**

**Why (the checkout failures):** repeated SCM failures (#1179/#783/#785) all died at ~10:15 elapsed with `git-remote-https died of signal 15`. That is the git-plugin's **10-min per-attempt default** firing on the implicit `Declarative: Checkout SCM` step — which uses the **job-level SCM config**, NOT the `CloneOption` you wrote in the Jenkinsfile. So tuning the in-pipeline checkout does nothing; the implicit one times out first.

**The three-layer fix:**
1. `skipDefaultCheckout(true)` in `options{}` — bypasses the implicit job-level checkout entirely so your in-pipeline checkout is the only one.
2. `CloneOption shallow:true depth:200 timeout:30` — ~10× size reduction and 3× the per-attempt budget. (depth:200 is conservative; depth:50 usually suffices unless a pipeline has been red for days and you need history back to the last green.)
3. `honorRefspec:true` + explicit `+refs/heads/${BRANCH}:refs/remotes/origin/${BRANCH}` — fetches ONE branch, not all heads (the default refspec pulls every head on a 65k-object repo).

**Job-alias trap:** the multibranch job named `elohim-holochain` is configured to load `elohim/holochain/dna/Jenkinsfile` (the **DNA** pipeline), NOT `elohim/holochain/Jenkinsfile` (the **edge** pipeline). Verify which file a build actually ran via the console's `Obtained …/Jenkinsfile from <sha>` line — do not assume from the job name.

**How to apply:**
- A checkout dying at a clean ~10-min mark with `signal 15` is the implicit-checkout default timeout, not a network flake → reach for `skipDefaultCheckout(true)` first.
- Before editing "the holochain Jenkinsfile," confirm which file the failing job loads (`Obtained ... from`); the `elohim-holochain` job-name points at the DNA file.

Related: [[feedback_jenkinsfile_safe_directory_required]] (checkout-as-different-UID), [[feedback_jenkins_token_strictly_guarded]], [[project_alpha_topology_bootstrap_pair]]. Pairs with the broader checkout-reliability backlog item (`backlog/jenkins-checkout-reliability.md`).
