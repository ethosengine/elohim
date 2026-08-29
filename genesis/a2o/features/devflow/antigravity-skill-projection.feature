@devflow @agentic @wip @concern:agent-runtime-projection
Feature: One Elohim package equips Antigravity with governed skills
  As a developer opening the Elohim workspace with Antigravity
  I want its skills to grow from the same packages Claude and Codex use
  So that changing agent harnesses does not fork capability or governance

  # A SkillPackage is the canonical Elohim definition of one skill. AgentPackage,
  # CommandPackage, HookPackage, and McpProfilePackage are sibling artifact types;
  # each package type defines its own supported-runtime set. Projected SKILL.md files contain
  # YAML frontmatter followed by the package's Markdown instructions.body. In the graph JSON,
  # a BlobCid is the opaque content address in nodes[].cid that eprfs computes from file bytes.
  # Antigravity's installed 1.1.x skills guide is the design evidence for the supported
  # .agents/skills/<name>/SKILL.md layout; the scenarios test Elohim's checked-in contract.
  Background:
    Given commands run from the repository root "/projects/elohim"
    And ".epr-meta/elohim/projections/<runtime>" contains checked-in projection fixtures
    And ".claude", ".codex", and ".agents" contain files consumed by the three runtimes
    And ".epr-meta/elohim/packages" contains the complete checked-in package catalog validated by the projector
    And ".epr-meta/elohim/packages/skills/example-skill.json" is otherwise schema-valid with:
      | field                          | value                                        |
      | kind                           | SkillPackage                                 |
      | metadata.id                    | example-skill                                |
      | metadata.name                  | example-skill                                |
      | metadata.master                | package                                      |
      | metadata.runtimeTargets        | ["claude", "codex", "antigravity"]         |
      | metadata.governance.eprRef     | epr:elohim-agent/skills/example-skill        |
      | instructions.body              | # Shared example instruction                 |

  Scenario: Antigravity discovers a packaged skill through its documented workspace layout
    Given "metadata.master" makes the example package JSON the authoritative source
    When the developer runs "node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project --write-fixtures --write-runtime --only SkillPackage:example-skill"
    Then Claude receives ".claude/skills/example-skill/SKILL.md"
    And Codex receives ".codex/skills/example-skill/SKILL.md"
    And Antigravity receives ".agents/skills/example-skill/SKILL.md"
    And the fixture files ".epr-meta/elohim/projections/claude/skills/example-skill/SKILL.md", ".epr-meta/elohim/projections/codex/skills/example-skill/SKILL.md", and ".epr-meta/elohim/projections/antigravity/skills/example-skill/SKILL.md" exist
    And all six SKILL.md files have frontmatter name "example-skill"
    And all six SKILL.md Markdown bodies equal the package's "instructions.body"
    And the Claude files' YAML "metadata.governance" equals the package JSON "metadata.governance.eprRef"
    And the Codex and Antigravity files' YAML "governance" equals the package JSON "metadata.governance.eprRef"

  Scenario: Relative skill assets remain usable and content-addressed
    Given the example package's "assets" maps path "references/guide.md" to "contentBase64" value "IyBHdWlkZQo="
    And decoding that value yields the UTF-8 bytes "# Guide\n"
    And that package's "metadata.assetRefs" is ["references/guide.md"]
    When the developer runs "node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project --write-fixtures --write-runtime --only SkillPackage:example-skill"
    Then ".epr-meta/elohim/projections/claude/skills/example-skill/references/guide.md" equals the bytes decoded from "assets[].contentBase64"
    And ".epr-meta/elohim/projections/codex/skills/example-skill/references/guide.md" equals those bytes
    And ".epr-meta/elohim/projections/antigravity/skills/example-skill/references/guide.md" equals those bytes
    And ".claude/skills/example-skill/references/guide.md" equals those bytes
    And ".codex/skills/example-skill/references/guide.md" equals those bytes
    And ".agents/skills/example-skill/references/guide.md" equals those bytes
    When the developer runs "RUSTFLAGS='' CARGO_TARGET_DIR=/tmp/eprfs-agent-story-target cargo run --manifest-path elohim/sdk/domains/elohim-agent/adapter/Cargo.toml -q --bin eprfs-agent -- compose-graph .epr-meta/elohim/packages --projections-root .epr-meta/elohim/projections"
    Then its JSON "nodes" contains three records with "metadata.role" equal to "projection-asset", "metadata.id" equal to "example-skill", and "metadata.assetPath" equal to "references/guide.md"
    And each asset node's "cid" is the BlobCid that eprfs computes from that projected file's bytes
    And the package node is the record with "metadata.role" equal to "package" and "metadata.id" equal to "example-skill"
    And its JSON "edges[]" contains one record per asset node whose "source" equals the package node "cid", whose "derived" equals that asset node "cid", and whose "relation" equals "projection"
    And each asset node's "metadata.runtime" is one of "claude", "codex", or "antigravity"
    And each such "edges[]" record has "metadata.runtime" equal to its derived asset node's "metadata.runtime"
    And each such "edges[]" record has "metadata.assetPath" equal to "references/guide.md"

  Scenario: Runtime support is claimed only for an implemented Antigravity contract
    When "node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify" validates the schemas and package JSON below ".epr-meta/elohim/packages"
    Then the command exits successfully
    And "elohim/sdk/domains/elohim-agent/schemas/skill-package.schema.json" accepts ["claude", "codex", "antigravity"] as "metadata.runtimeTargets"
    And that schema rejects the array when any of those three values is omitted
    And that schema rejects a runtime value outside those three
    And the package schemas in "elohim/sdk/domains/elohim-agent/schemas" for AgentPackage, CommandPackage, HookPackage, and McpProfilePackage do not enumerate "antigravity"
    And package JSON files below ".epr-meta/elohim/packages/agents", "commands", "hooks", and "mcp-profiles" contain no "projections.antigravity" entry
