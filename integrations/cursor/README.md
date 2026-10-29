# Cursor

Based on Cursor docs (MCP and Rules pages), read 2026-10-29.

## MCP
Copy `mcp.json` to `.cursor/mcp.json` (project) or `~/.cursor/mcp.json` (global). The `${workspaceFolder}` interpolation limits the server to the project.

```bash
mkdir -p .cursor && cp integrations/cursor/mcp.json .cursor/mcp.json
```
Needs `agentic-ooxml-mcp` on PATH (or an absolute `command`). Install the binary from source or GitHub Releases; see ../mcp/README.md.

## Rule
`agentic-ooxml.mdc` is a Cursor project rule (`.mdc`; plain `.md` is ignored by Cursor). It is attached when a .docx/.pptx/.xlsx is in context.
```bash
mkdir -p .cursor/rules && cp integrations/cursor/agentic-ooxml.mdc .cursor/rules/
```

## Agent Skills
The Cursor Rules/MCP pages we read do not mention SKILL.md. Whether Cursor loads `integrations/skills` is unknown to us, so we ship a rule instead.

## Status
| Item | Status |
|---|---|
| MCP config | Experimental: format checked against docs; Cursor is not installed here, never run |
| Rule file | Experimental: same |
| SKILL.md support | Unknown / Planned |
