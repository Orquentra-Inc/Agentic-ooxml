# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The crates are marked `publish = false`: they are
not on crates.io. Releases are GitHub releases built from a `v*` tag, and their notes are the matching
section below.

## [Unreleased]

## [0.1.0] - 2026-10-29

First public release. Early: version 0.1 of the schema, and Office, LibreOffice, ONLYOFFICE and Google round
trips are unmeasured.

### Added

- `agentic-ooxml` library: discover, read, write and merge the `agents`, `design` and `intent`
  knowledge parts (custom XML, namespace `urn:agentic-office:ooxml:v0.1`) in `.docx`, `.pptx` and
  `.xlsx` packages. Parts are found by `customXml` relationship plus namespace, not by file name.
- `sources` part: source records with a blake3 claim hash, an effective status computed at read
  time, and `reconcile` to persist staleness.
- Atomic package rewrite that copies every native entry byte for byte.
- Safety caps: entry count, per-part size, `<!DOCTYPE` refusal, path traversal refusal.
- Read-only reader for v0.2 context (`doc`, `units`, `spec`, and the old `legacy-deck`).
- `agentic-ooxml` command line with `inspect`, `validate`, `diff`, `list`, `read`, `write`, `sources` and
  `reconcile`, all printing JSON.
- `agentic-ooxml-mcp`: a stdio MCP server with eight tools (`ao_inspect`, `ao_list`, `ao_read`,
  `ao_sources`, `ao_validate`, `ao_diff`, `ao_write`, `ao_reconcile`). Write tools refuse unless
  `AGENTIC_OOXML_ALLOW_WRITE=1`; paths must sit under `AGENTIC_OOXML_ROOTS`.
- Documentation, architecture paper, governance files and CI workflows.
- Project website, a contribution guide (`docs/contributing.md`), and the recorded round-trip evidence
  for one editor's pipeline (`docs/evidence/round-trip-2026-10-19/`).
- Release workflow (prebuilt binaries for Linux x86_64, macOS arm64 and x86_64, Windows x86_64, with
  `SHA256SUMS`) and `scripts/install.sh`.

### Provenance

- The library began as the Rust module `ooxml_knowledge.rs` from the Agentic Office monorepo, extracted
  unchanged, with its 24 tests; `inspect`, `validate` and `diff` and the MCP server were added after extraction.

[Unreleased]: https://github.com/Orquentra-Inc/Agentic-ooxml/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Orquentra-Inc/Agentic-ooxml/releases/tag/v0.1.0
