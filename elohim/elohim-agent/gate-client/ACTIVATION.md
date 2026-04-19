# Gate Activation Runbook

## What "active" means

`Phase::ElohimActive` is OBSERVED from real inference, not set by any flag.
The gate correctly stamps decisions with:

- `Phase::DevContext` when the wisdom call returned a stub (no API key, mock
  transport, backend error, etc.)
- `Phase::ElohimActive` when the wisdom call actually reached a live LLM

There is no `ELOHIM_ACTIVE` env var and no flag that forces the phase.
Phase is a property of what **actually happened** — it cannot be asserted by
configuration.

## To activate (go live with real wisdom)

1. Set `ANTHROPIC_API_KEY=sk-ant-...` in elohim-agent-service's environment.
   This is what tells elohim-agent-service it can make real Claude calls.

2. Set `ELOHIM_AGENT_WISDOM_TRANSPORT=in-process` in any process consuming
   gate-client.  This selects the transport that routes to elohim-agent-service
   (in-process call in current architecture).

3. Call `gate_client::configure_runner_with_config(GateClientConfig::from_env())`
   once at process startup, BEFORE the first gate check.  Or construct the config
   programmatically and pass it explicitly:

   ```rust
   use gate_client::{configure_runner_with_config, transport::{GateClientConfig, WisdomTransport}};

   let config = GateClientConfig {
       wisdom_transport: WisdomTransport::InProcess,
       elohim_id: Some("your-elohim-agent-pubkey".to_string()),
       elohim_substance_cid: Some("epr:substance:elohim-prod-v1".to_string()),
       ..Default::default()
   };
   configure_runner_with_config(config)
       .expect("must be called before first gate check");
   ```

4. Verify: call `check()` with any event.  The returned `GateDecision.phase`
   field should be `Phase::ElohimActive`.  The decision attestation CID will
   carry the same phase marker.

## To verify without burning API credits

- Leave `ANTHROPIC_API_KEY` unset.
- Set `ELOHIM_AGENT_WISDOM_TRANSPORT=in-process`.
- Check decisions: phase will be `DevContext` (elohim-agent-service honestly
  reports the stub path).  This proves the pipe is alive end-to-end.

## Environment variables read by `GateClientConfig::from_env()`

| Variable                          | Values                     | Effect                              |
|-----------------------------------|----------------------------|-------------------------------------|
| `ELOHIM_AGENT_WISDOM_TRANSPORT`   | `in-process` / (unset)     | Selects InProcess or Mock transport |
| `ELOHIM_ID`                       | AgentPubKey (base64)       | Sets elohim identity for attestations |
| `ELOHIM_SUBSTANCE_CID`            | CID string                 | Sets substance CID for attestations |

`ANTHROPIC_API_KEY` is NOT read by gate-client.  It is elohim-agent-service's
concern.  gate-client is transport-agnostic about the key.

## Rolling back

- Set `ELOHIM_AGENT_WISDOM_TRANSPORT=mock` OR unset it.
- Restart any long-running processes.
- All decisions revert to `Phase::DevContext` via the hardcoded mock.

## Why there's no "active" flag

Phase is a property of what ACTUALLY happened, not what you intend.  A flag
that forces `Phase::ElohimActive` would allow the system to lie about whether
real inference occurred.  That lie would propagate into decision attestations,
poisoning the accountability graph.

The architecture inverts the usual risk: if the LLM call silently fails, the
decision is honestly stamped `DevContext` — no weight, no reputation accrual.
This is the correct fallback.  Operators observe `Phase::ElohimActive` in real
attestations only after confirming the full pipe is live.

For background see the `elohim-active-observed-not-flagged` memory entry and
`elohim/elohim-agent/spec/2026-04-18-gate-interface.md` §7.5.
