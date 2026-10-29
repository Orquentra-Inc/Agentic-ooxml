# Pi coding agent

Based on Pi docs (earendil-works/pi, packages/coding-agent). Local Pi: 0.85.1.

## MCP
The Pi docs we read do not describe MCP support; we did not find it. We ship no Pi MCP config. Treat MCP in Pi as Planned/unknown.

## Skills + CLI (works without MCP)
Pi loads Agent Skills from `.agents/skills/` and `~/.agents/skills/`. Pi has a bash tool, so the skills can use the `agentic-ooxml` CLI. The skills name MCP tools first and give the CLI equivalent in parentheses; with Pi, use the CLI form.
```bash
mkdir -p .agents/skills && cp -R integrations/skills/ooxml-* .agents/skills/
```
Run `/reload` or restart after copying. Invoke explicitly with `/skill:ooxml-inspect`.

## Caveat
Pi runs tools without a permission prompt by default as far as the docs we read say; point it at copies of files, and note the CLI `write` replaces the file atomically in place (the `.ao-proposed` default is an MCP behaviour). Back up first.

## Status
| Item | Status |
|---|---|
| Skills via `.agents/skills` | Experimental: structure matches Pi docs; not loaded in a live Pi session by us |
| CLI use from Pi | Experimental: depends on the CLI inspect/validate/diff subcommands from stream A |
| MCP | Planned / unknown |
| Pi package (`pi install`) | Planned (no package.json written) |
