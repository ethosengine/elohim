# Gemini Runtime Surface Discovery (2026-08-15)

> **Superseded (2026-09-22).** The finding below that Gemini/Antigravity has no
> repository-local surface was wrong: Antigravity reads a workspace-root
> `AGENTS.md`, workspace rules under `.agents/rules/`, and skills under
> `.agents/skills/` — only `GEMINI.md` is global (`~/.gemini/GEMINI.md`). The
> separate `gemini` runtime this note designed was never merged. Commit
> `3a6ca5f7d` (2026-08-29) instead added `antigravity` as the third skill
> runtime, projecting to `.agents/skills/<id>/SKILL.md`; agents, commands and
> hooks deliberately do not claim Antigravity, and the root gospel reaches it
> through the existing `AGENTS.md` projection. Kept as the record of how that
> decision was reached.

## Objective
Discover the native persona/agent surface of the Gemini CLI runtime (`gemini-3.1-pro`) and design a projection shape that aligns with the established `elohim-package-authoring` skill discipline, avoiding forcing the `.claude/agents` structure onto a runtime with different requirements.

## Findings

1. **Global App Context**: The Gemini CLI runtime (Antigravity) relies on a global context primarily housed under `~/.gemini/`. 
   - Global Context File: `~/.gemini/GEMINI.md`
   - App Data Directory: `~/.gemini/antigravity/`
   - Config File: `~/.gemini/antigravity/mcp_config.json`

2. **Absence of Local Persona Config**: Unlike the Claude runtime, which heavily utilizes `.claude/agents/` and `.claude/skills/` within the repository for runtime-native configuration, the Gemini environment did not inherently provide or expect a repository-local `.gemini/` persona surface. Its behavior is primarily driven by system prompts and the overarching `GEMINI.md` context file.

3. **Projection Design**: Since there is no forced schema or native surface inside the workspace, the decision was made to establish a clear and explicit `.gemini/agents/` and `.gemini/skills/` projection target. 
   - This prevents conflating Gemini's runtime with Claude's or Codex's requirements.
   - The generated projections maintain a pure markdown frontmatter approach containing just the model (`gemini-3.1-pro` for agents), tools, name, and description.
   - This ensures the `gemini-3.1-pro` runtime has an explicit configuration target that is recognizable by the package projector without polluting the agent's root runtime.

## Conclusion

The `package-projections.mjs` script was successfully extended to support `projections/gemini` for Agent and Skill packages. The `blind-reader` and `storyteller` packages were successfully projected to `.gemini/agents/<name>.md`. 

*Note: In-flight execution of `epr actor claim` and `pnpm elohim-agent:packages:verify` was interrupted during the build phase by environment isolation constraints (`nsjail-sandbox`), requiring manual verification of the generated projections.*
