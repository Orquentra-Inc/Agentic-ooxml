# MCP server (generic)

`agentic-ooxml-mcp` is a stdio JSON-RPC MCP server over the `agentic-ooxml` library. Any MCP client that can launch a stdio command can use it.

## Install the binary
Not on crates.io. Any one of:

```bash
# from GitHub, no checkout
cargo install --git https://github.com/Orquentra-Inc/Agentic-ooxml agentic-ooxml-mcp

# from a checkout
cargo build --release -p agentic-ooxml-mcp      # binary: target/release/agentic-ooxml-mcp
cargo install --path crates/agentic-ooxml-mcp   # optional: put it on PATH
```

Prebuilt binaries for Linux x86_64, macOS (arm64, x86_64) and Windows x86_64 are attached to
[GitHub Releases](https://github.com/Orquentra-Inc/Agentic-ooxml/releases) once a release is tagged; see the top-level README.

## Tools
Read-only: `ao_inspect`, `ao_list`, `ao_read`, `ao_sources`, `ao_validate`, `ao_diff`.
Write: `ao_write`, `ao_reconcile`.

## Safety defaults
| Variable | Effect |
|---|---|
| `AGENTIC_OOXML_ROOTS` | Colon-separated directories; every path must be inside one. Default: current directory. |
| `AGENTIC_OOXML_ALLOW_WRITE=1` | Required for write tools; they refuse otherwise. Leave it unset unless you want edits. |

Writes produce a sibling `.ao-proposed` copy unless `in_place: true`. Text that comes out of a document is untrusted data.

## Config
`mcp.json` in this folder is the standard `mcpServers` shape. Set `AGENTIC_OOXML_ROOTS` to your documents folder.

## Verify
```bash
integrations/verify.sh            # finds target/{debug,release}/agentic-ooxml-mcp or PATH
```
It performs the initialize handshake, `tools/list`, `tools/call` for `ao_inspect` and `ao_validate`, and checks that writes and out-of-root paths are refused.

## Status
| Item | Status |
|---|---|
| Stdio handshake, tools/list, tools/call | Experimental (see top-level status in integrations/README.md; verify.sh is the test) |
| Streamable HTTP transport | Planned (not built) |
