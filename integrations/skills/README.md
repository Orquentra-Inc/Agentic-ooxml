# Agent Skills package (standalone)

Four Agent Skills (agentskills.io spec): `ooxml-inspect`, `ooxml-edit`, `ooxml-validate`, `ooxml-provenance`. Each has a name matching its directory, a description under 1024 characters, and a short body; `ooxml-edit` has one file in `references/`.

They assume the `agentic-ooxml-mcp` tools or the `agentic-ooxml` CLI are available (see ../mcp/README.md; install the binaries from source or GitHub Releases, see the top-level README).

## Install (copy the folders into your agent's skills directory)
```bash
# Codex, Pi (both read .agents/skills; Claude Code reads .claude/skills)
mkdir -p ~/.agents/skills && cp -R integrations/skills/ooxml-* ~/.agents/skills/
mkdir -p ~/.claude/skills && cp -R integrations/skills/ooxml-* ~/.claude/skills/
```
Skills copied to `~/.claude/skills` are not namespaced; the plugin in ../claude-code namespaces them.

## Validate
`integrations/verify.sh` checks the spec constraints itself. If you have the reference validator (`skills-ref`, from github.com/agentskills/agentskills) it also runs it.

## Status
| Item | Status |
|---|---|
| Spec constraints (name, description length, size) | Supported-and-tested (verify.sh, own checker) |
| `skills-ref validate` | Not run (not installed) |
| Skills loaded and followed by a live agent | Not tested |
