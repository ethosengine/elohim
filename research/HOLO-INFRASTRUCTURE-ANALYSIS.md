# Holo Infrastructure Analysis for Elohim

**Date**: December 2024
**Purpose**: Evaluate feasibility of adopting/contributing to Holo's hosting infrastructure

## Executive Summary

Holo's infrastructure is split across multiple repositories at different maturity levels. The billing infrastructure exists but uses **ancient Holochain APIs** (v0.0.18-alpha1 era). The newer Rust-based hosting platform (holo-host) has **no billing integration yet**. This represents a significant contribution opportunity for Elohim.

## Repository Landscape

| Repository | Language | Status | Billing | Holochain Version |
|------------|----------|--------|---------|-------------------|
| [holo-host](https://github.com/Holo-Host/holo-host) | Rust | Active | ❌ None | 0.3, 0.4, 0.5 |
| [holo-envoy](https://github.com/Holo-Host/holo-envoy) | TypeScript | Legacy | ✅ ServiceLogger | v0.0.117 (ancient) |
| [servicelogger](https://github.com/Holo-Host/servicelogger) | Rust (HDK) | Stale | ✅ Core DNA | v0.0.18-alpha1 (ancient) |
| [web-sdk](https://github.com/Holo-Host/web-sdk) | TypeScript | Active | - | 0.18.0-dev |

## Architecture Analysis

### holo-host (Rust Monorepo) - The Future

**Components**:
- **Gateway**: HTTP/WebSocket → NATS routing
- **Orchestrator**: Workload lifecycle, MongoDB state
- **Host Agent**: Container management, Holochain instances
- **HAM**: Holochain App Manager CLI

**Key Strengths**:
- NATS-based microservices architecture
- Nix reproducible builds
- systemd-nspawn container isolation
- Multi-version Holochain support (0.3-0.5)
- Clean workload lifecycle management

**Critical Gap**: No billing/economic infrastructure

### holo-envoy (TypeScript) - Legacy but Has Billing

**Key Features**:
- Per-user agent provisioning: `{hha_hash}:{agent_id}:###zero###`
- Wormhole signing (Lair shim interceptor)
- ServiceLogger DNA integration
- Three-phase billing: Request → Response → Confirmation

**Problems**:
- Uses `@holochain/conductor-api: ^0.2.1` (ancient)
- Would need 60-70% rewrite for modern Holochain
- Complex Chaperone/iframe dependency

### servicelogger DNA - The Billing Core

**Billing Flow**:
```
Client Request → Host Response → Client Confirmation
                       ↓
              ServiceLog Entry (signed proof of service)
                       ↓
              Accumulate until threshold
                       ↓
              Bridge to HoloFuel → Generate Invoice
```

**Data Tracked**:
- `agent_id`: Who made the request
- `response_hash`: Cryptographic proof of response
- `confirmation_signature`: Client attestation
- `host_metrics`: CPU, bandwidth, duration
- `client_metrics`: Request duration

**Critical Problem**: Uses HDK from holochain-rust v0.0.18-alpha1 - **completely incompatible** with modern Holochain (0.3+)

## Upgrade Path Analysis

### Adding Holochain 0.6 to holo-host

**Effort**: Low
**Changes Required**:
1. Add to `flake.nix`:
   ```nix
   holonix_0_6 = {
     url = "github:holochain/holonix?ref=main-0.6";
     inputs = { /* ... */ };
   };
   ```
2. Update `supported-holochain-versions.json`:
   ```json
   {
     "default_version": "0.6",
     "supported_versions": ["0.3", "0.4", "0.5", "0.6", "latest"],
     "version_mappings": {
       "0.6": "holonix_0_6",
       "latest": "holonix_0_6"
     }
   }
   ```
3. Test container builds with 0.6 conductor
4. Verify NATS routing still works

### Porting ServiceLogger to Modern Holochain

**Effort**: High (but valuable)
**Changes Required**:
1. Rewrite DNA using modern HDI (integrity) + HDK (coordinator) split
2. Update entry definitions to new format
3. Reimplement validation callbacks
4. Update HoloFuel bridge calls (or integrate with Unyt)
5. Test cryptographic signature validation

**Entry Types to Port**:
- `SetupPrefs` - Configuration
- `ClientRequest` - Incoming requests
- `HostResponse` - Service responses
- `ServiceLog` - Signed proof of service
- `InvoicedLogs` - Invoice batches

### Modernizing holo-envoy

**Effort**: Very High
**Would Require**:
- Upgrade `@holochain/conductor-api` 0.2.1 → 0.19+
- Rewrite WebSocket wrappers
- Update Lair shim for new protocol
- Modernize AppInfo handling
- Update signal forwarding

**Recommendation**: Don't modernize envoy - instead port the billing concepts to holo-host

## Strategic Recommendation

### Phase 1: Quick Win - Holochain 0.6 Support
**Target**: holo-host repository
**Effort**: 1-2 weeks
**Value**: Immediate - enables Elohim DNAs on Holo infrastructure

Steps:
1. Fork holo-host
2. Add holonix_0_6 input
3. Update version mappings
4. Test with Elohim's content_store DNA
5. Submit PR upstream

### Phase 2: Billing DNA Rewrite
**Target**: New servicelogger-v2 DNA
**Effort**: 4-6 weeks
**Value**: High - enables economic sustainability

Steps:
1. Design modern HDI/HDK structure
2. Port entry types and validation
3. Integrate with holo-host orchestrator
4. Design Unyt integration (or standalone invoicing)
5. Submit as new repository or PR

### Phase 3: Gateway Integration
**Target**: holo-host gateway + host-agent
**Effort**: 2-4 weeks
**Value**: Production readiness

Steps:
1. Add billing hooks to gateway HTTP handler
2. Integrate ServiceLogger calls in host-agent
3. Add billing configuration to workload manifests
4. Dashboard for billing visibility

## Contribution Value Proposition

For Holo:
- Holochain 0.6 support they haven't shipped yet
- Modern billing DNA they need but haven't built
- Active maintenance from motivated team

For Elohim:
- Production-ready hosting infrastructure
- Economic sustainability mechanism
- Path to hREA-care based economy
- Operational cost coverage for node operators

## Files Cloned for Research

```
/projects/elohim/research/
├── holo-host/           # Rust monorepo (Gateway, Orchestrator, Host Agent)
├── holo-envoy/          # TypeScript legacy envoy (has billing)
├── web-sdk/             # Client SDK
└── servicelogger/       # Billing DNA (ancient HDK)
```

## Key Contacts/Resources

- [ServiceLogger RFC](https://github.com/Holo-Host/rfcs/blob/master/service-logger/README.md)
- [Holo Infrastructure Article](https://medium.com/holochain/a-look-at-the-holo-hosting-infrastructure-e1b034d85386)
- [Key Management in Holo](https://blog.holochain.org/key-management-and-source-chain-entry-signing-in-holo/)

## Conclusion

The opportunity is real. Holo has solid infrastructure but appears to have stalled on:
1. Holochain 0.6 support
2. Modern billing integration

Elohim can provide both while bootstrapping its own network. The contribution would be substantial and genuinely useful to the ecosystem.
