# eprfs-agent

`eprfs-agent` is the domain adapter that turns hand-authored elohim-agent
capabilities (markdown + YAML frontmatter, e.g. `.claude/agents/*.md`) into
`eprfs` projection manifests for concrete runtime surfaces (`.claude`,
`.codex`). It owns all capability/dialect knowledge so `eprfs-core` and
`eprfs-local` stay domain-neutral: parse the canonical form once
(`CanonicalAgent::parse`), project it per runtime via a `ProjectionBinding`
(`project`), then materialize or verify-drift against a real tree with
`eprfs-local`.

A runnable dry-run over a live `.claude/agents` directory lives at
`examples/project_agents.rs`:

```bash
cargo run -p eprfs-agent --example project_agents -- <repo-root>
```

It parses every `.md` file under `<repo-root>/.claude/agents`, projects the
`.claude` runtime surface in memory, and reports drift against what is
actually on disk. It is read-only — parse, project in memory, verify vs
disk — and writes nothing. A non-zero drift (or skip) count is expected and
informative: it measures how far today's hand-authored agent files are from
the projector's normalized form, i.e. the per-file migration delta.

## Validation seam

Frontmatter is validated against the V1 `elohim-agent` metadata schemas
(`elohim/sdk/schemas/v1/agent/`, wired through
`elohim/sdk/domains/elohim-agent/`) by `pnpm run manifest:test` (node/ajv).
`eprfs-agent` itself does not re-validate — it parses and trusts.

The seam is: **author once** in markdown frontmatter, **validate in node**
(`pnpm run manifest:test`, schema-shape correctness), **project in Rust**
(`eprfs-agent`, runtime-shape rendering + drift detection). Keeping validation
in node and projection in Rust means the schema stays the single source of
truth for "is this capability well-formed," while `eprfs-agent` stays focused
on "what does this capability look like on each runtime."
