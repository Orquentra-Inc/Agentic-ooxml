# Agentic OOXML plugin for Claude Code

Experimental. Adds four skills (`ooxml-inspect`, `ooxml-edit`, `ooxml-validate`, `ooxml-provenance`) and an MCP server entry that launches `agentic-ooxml-mcp` for inspecting, validating and annotating the Agentic OOXML knowledge parts inside .docx, .pptx and .xlsx files. It does not edit slides, paragraphs or cells. Write tools stay disabled until you set `AGENTIC_OOXML_ALLOW_WRITE=1`.

## Install from a local checkout (the MCP binary must be on PATH)
```bash
claude plugin marketplace add /path/to/Agentic-ooxml
claude plugin install agentic-ooxml@agentic-ooxml
# or, for one session without installing:
claude --plugin-dir /path/to/Agentic-ooxml/integrations/claude-code
```

## Install from GitHub
```bash
claude plugin marketplace add Orquentra-Inc/Agentic-ooxml
claude plugin install agentic-ooxml@agentic-ooxml
```
Inside a session: `/plugin marketplace add Orquentra-Inc/Agentic-ooxml`, then `/plugin install agentic-ooxml@agentic-ooxml`.
The marketplace manifest is `.claude-plugin/marketplace.json` at the repository root, which is where `marketplace add <owner>/<repo>` looks; it points at `./integrations/claude-code`.

## MCP binary
`.mcp.json` runs `agentic-ooxml-mcp` from PATH; the plugin does not bundle it. Install it first: `cargo install --git https://github.com/Orquentra-Inc/Agentic-ooxml agentic-ooxml-cli agentic-ooxml-mcp`, or from a checkout `cargo install --path crates/agentic-ooxml-mcp`, or a prebuilt binary from GitHub Releases (see the [README](../../README.md#install)). Set `AGENTIC_OOXML_ROOTS` in your environment to restrict paths.

## Skills source
`skills/` is a copy of `integrations/skills/`; `integrations/verify.sh` fails if they drift.

## Hooks
None. No hook is justified: every guard (roots, write opt-in, proposed copy) is enforced by the server, and a hook would duplicate it.

## Status
| Item | Status |
|---|---|
| `claude plugin validate` (plugin and marketplace), Claude Code 2.1.312 | Supported-and-tested |
| Directory profile checks | Not clean: README_TRIVIAL (root README), no icon (warning) |
| Install into a live session and run a skill | Not tested |
| MCP server launches and answers | Not tested: binary did not exist when this was written |
