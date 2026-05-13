---
name: Prefer schema-able YAML config over CLI args for node configuration
description: Node configuration should be declarative YAML files (schema-validated) that the elohim operator manages, not CLI args. CLI args are for dev/testing only.
type: feedback
originSessionId: 63499c63-1cde-41b5-a0b0-66503d4c008c
---
When configuring elohim-node (conductor settings, resource budgets, P2P topology), prefer schema-validatable YAML files over CLI args.

**Why:** The elohim operator needs to manage a fleet of nodes declaratively — like kubectl applies manifests. CLI args work for dev/testing but don't compose into operator-managed config. YAML configs can be schema-validated, version-controlled, and applied by automation.

**How to apply:** When adding new configuration to elohim-storage/elohim-node:
1. CLI args are fine for dev convenience and override
2. But the primary config path should be a YAML file (like k8s manifests, application.yml, devfile.yaml)
3. The YAML schema should be defined (like devices.schema.json) so configs can be validated
4. Think "kubectl analogue for elohim-node" — the operator reads these configs to adjust resources across blade servers
