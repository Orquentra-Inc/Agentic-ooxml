# Roadmap

No dates. Each item is in exactly one of four states, and the state is the claim: anything not under
**Implemented** is not in the code that runs today. Details of what each implemented item does are in
the [schema reference](docs/schema.md) and the [architecture paper](docs/paper/architecture.md).

## Implemented

- Discover knowledge parts by `customXml` relationship plus the `ao` namespace, not by file name,
  in `.docx`, `.pptx` and `.xlsx`.
- Read, create, merge and delete sections of the `agents`, `design` and `intent` parts.
- `sources` part: records with a claim hash, an effective status computed at read time, and
  `reconcile` to persist staleness.
- Atomic package rewrite; native entries copied byte for byte.
- Safety caps: entry count, 2 MiB per part, `<!DOCTYPE` refusal, path traversal refusal.
- Read-only access to v0.2 context (`doc`, `units`, `spec`) and the legacy `ao:deck`.
- Synthetic example files (`.pptx`, `.xlsx`, `.docx`), a deterministic generator, and a walkthrough script with its committed output. Checked structurally; never opened in Office.
- Command line with JSON output: `inspect`, `validate`, `diff`, `list`, `read`, `write`, `sources`, `reconcile`.
- `validate`: structural checks (ZIP, XML well-formedness, content types, relationships, parts, stale sources). It does not establish that Office will open the file.
- `diff`: structural diff of two packages (parts, source status changes, native part hashes).
- MCP server over the library (stdio): eight tools, write tools off by default, paths confined to configured roots.

## In progress

Work that exists in the working tree or is being built, but is not finished or not yet covered by
this documentation as a feature. See [CHANGELOG.md](CHANGELOG.md) for what has landed.

- Agent integrations (Claude Code plugin and skills, adapters for Codex, Cursor and Pi): installable, behaviour not verified end to end. Status table in [integrations/README.md](integrations/README.md).
- The project website (GitHub Pages) deploys from `website/` on a push to `main`.
- A Streamable HTTP transport for the MCP server is planned and not built.

## Proposed

Described in the original design, not implemented. Do not build on these yet.

- Typed design tokens in `design` (today `design` is free-form Markdown).
- A `validation` part.
- An agent registry in `agents` with capabilities and authorisation (today `agents` is free-form
  Markdown policy).
- Automatic cross-file edits: detecting that a workbook value changed is implemented (external claims); proposing the matching slide edit is not.
- A writer for v0.2 context in this repository (today the v0.2 writer lives in the Agentic Office
  TypeScript servers, and this library only reads it).
- Per-slide or per-sheet parts. The first measurement suggests slide-level parts do not survive one
  editor's round trip; see [architecture](docs/architecture.md).
- A public, versioned specification with a conformance corpus.

## Research

Open questions, each needing an experiment before a claim is made. They are RQ1 to RQ4 in the
[paper](docs/paper/architecture.md#9-research-questions).

- Round-trip retention in PowerPoint, Word, Excel, LibreOffice, ONLYOFFICE and Google import and
  export (not measured).
- Whether embedded context improves cross-session edit fidelity against a strong native-OOXML baseline.
- Whether selective disclosure lowers context use without lowering accuracy.
- A stable locator for claims under copy, move and reorder.
- Signed or otherwise authenticated context (a hash inside the same file proves nothing about authorship).
