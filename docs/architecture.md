# Architecture

![Architecture: agents call a trusted Rust runtime, which reads and atomically rewrites an OPC package that keeps its native content and gains Agentic OOXML parts](assets/architecture.svg)

Three layers, one direction of trust.

1. **Agents and tools** call the runtime: the command line and a stdio MCP server today; agent
   integrations (plugins, skills, adapters) are in progress (see the [roadmap](../ROADMAP.md)).
2. **The trusted runtime** is the Rust crate `agentic-ooxml`. It parses packages, finds parts,
   computes hashes, and performs writes. Its safety checks are code, not configuration, and
   embedded text cannot turn them off.
3. **The Office file** keeps its native content. The only things added are custom XML parts, and the
   one-line relationship and content-type entries that make them reachable.

## What a write changes

A write to a package that has no Agentic OOXML parts yet changes exactly these entries. This is
what the sample in [Getting started](getting-started.md) produced:

| entry | change |
|---|---|
| `customXml/ao-intent.xml` | new: the knowledge part |
| `customXml/ao-intentProps.xml` | new: the standard custom XML properties part (a GUID and the schema namespace) |
| `customXml/_rels/ao-intent.xml.rels` | new: links the item to its properties |
| `ppt/_rels/presentation.xml.rels` | one `customXml` relationship added (the main part's `.rels`) |
| `[Content_Types].xml` | one `Override` for the properties part |
| every other entry | copied raw, with its original compression |

The new package is built in a temporary file in the same directory and renamed over the original. A
read-only file is refused, and the original's permissions are kept.

## Where to put agent context: four options

The design question is where to store document-specific context so that it survives hand-offs
without becoming a second copy of the document. Four designs were considered.

| | A. Sidecar file | B. Custom OPC parts (this project) | C. Intermediate representation | D. Hybrid |
|---|---|---|---|---|
| Where context lives | next to the file (`deck.ao.json`) | inside the file, as `customXml` parts | the file is converted to a new model (JSON, Markdown) and edited there | context in the file; derived indexes outside |
| Travels with the bytes | no | yes | only if the IR is kept with the file | context yes, index no |
| Duplicates native structure | usually | no | by definition | no |
| Drift when the document is copied, renamed or saved elsewhere | high | lower: relationship and namespace discovery, hashes | high: the IR and the file are two truths | lower |
| Needs a conversion step | no | no | yes, in both directions | no |
| Office compatibility risk | none | parts may be dropped by some editors (see below) | the conversion can lose OOXML features | same as B |
| Status here | not built | **implemented** (v0.1 parts) | not built | the target design; the derived index is **proposed** |

**Verdict.** B, moving toward D. Context that must survive a hand-off lives in the file. Anything that
can be rebuilt, such as a text index or a locator map, stays outside as a disposable cache.
Option C is rejected for a stated reason: a lossless OOXML-to-model-and-back conversion is a much larger
problem than the one this project addresses, and the model becomes a second source of truth.
Option A is reasonable when you cannot modify the file at all, but it does not travel with it.

### Hybrid evaluation, with evidence

The evidence below comes from the Agentic Office project's own `.pptx` editor and OfficeCLI,
measured on 2026-10-19 in the monorepo this code was extracted from. The raw results and scripts
are copied into [docs/evidence/round-trip-2026-10-19](evidence/round-trip-2026-10-19/README.md);
they depend on that monorepo, so you can read them but not re-run them from here. They are the only
round-trip measurements that exist, and they say nothing about any other application.

Five encodings of the four proposed parts were built on a real `.pptx` and passed through that
editor's import and save, with and without an edit (slides reordered). "Kept" means found,
byte-identical, still related with the same relationship type, content type intact.

| encoding | result |
|---|---|
| `customXml/itemN.xml` with props part and standard `customXml` relationship | kept 4 of 4, both runs |
| friendly names (`customXml/agents.xml`), standard relationship, no props part | kept 4 of 4, both runs |
| `customXml/ao/*.xml`, a **custom relationship type**, own content type | kept 4 of 4, both runs |
| **slide-level** (related from slide 2) | part kept, **relationship lost**: orphaned, both runs |
| orphan control (no relationship) | present but unreferenced, as built |

Also: presentation-level parts survived six other write paths (OfficeCLI add slide, remove slide,
set and template merge; a deck builder's package merge; a plain rewrite): each 4 of 4 present and 4 of
4 referenced.

A later change (2026-10-19) made that editor re-attach per-slide relationships. Its regression test
(`ao-deck-roundtrip.test.ts`, 5 tests, in the monorepo) has not been re-run for this repository, so the
slide-level row above is the pre-fix result.

What this supports, and what it does not:

- Presentation-level (main-part) parts are safe **in that editor's pipeline**. This project therefore
  links its parts from the main part only.
- Slide-level parts did not survive. The design's own advice (document-level parts first) stands.
- **Not measured: Microsoft PowerPoint, Word and Excel, LibreOffice, ONLYOFFICE, Google import and
  export.** None were available. "Survives" is therefore established for one editor's pipeline and
  nothing else. See [compatibility](compatibility.md).
- The `.docx` editor was not measured in that experiment either.

## What is implemented, by layer

| layer | implemented | not implemented |
|---|---|---|
| Package | discovery by relationship and namespace, atomic rewrite, safety caps | a derived SQLite or full-text index; object locators beyond one shape id |
| Knowledge | `agents`, `design`, `intent` as summary plus Markdown sections; `sources` with claim hashes and cross-file value claims; reconcile; read-only v0.2 (`doc`, `units`, `spec`) | typed design tokens; a `validation` part; an agent registry; a v0.2 writer; automatic cross-file edits (detection is implemented) |
| Interface | command line (JSON); `inspect`, `validate`, `diff`; stdio MCP server | plugins and adapters are in progress; no HTTP transport |

## Module map

The library is `crates/agentic-ooxml/src/lib.rs` plus `inspect.rs`, `validate.rs` and `diff.rs`. The
original knowledge module's public functions are: `discover`, `list`, `read`, `write`, `sources`, `reconcile`, `read_context`,
`context_overview`, `rewrite_package`, and the parse and serialize pairs for knowledge and
sources. The command line in `crates/agentic-ooxml-cli` is a thin wrapper that prints their results as JSON.
