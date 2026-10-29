# Integrations

Adapters that let coding agents use Agentic OOXML. The binaries they need are installed from source or from
GitHub Releases (see the [README](../README.md#install)); nothing is on crates.io or npm.

| Target | Folder | Tested here | Status |
|---|---|---|---|
| Claude Code plugin + marketplace | claude-code/ (+ `../.claude-plugin/marketplace.json`) | `claude plugin validate` passes | Supported-and-tested (manifest only); runtime Experimental |
| Agent Skills package | skills/ | own spec checker passes | Supported-and-tested (format); behaviour Experimental |
| MCP server config | mcp/ | not run: no binary at time of writing | Experimental |
| Codex CLI | codex/ | not run | Experimental |
| Cursor | cursor/ | not run (Cursor not installed) | Experimental |
| Pi | pi/ | not run | Experimental (skills + CLI); MCP and package Planned |

Run `integrations/verify.sh` from the repo root. It prints PASS, FAIL or SKIP per check; SKIP means untested. Re-run it once `agentic-ooxml-mcp` is built and update the table from its output, not from this file.
