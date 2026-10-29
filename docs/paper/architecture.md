# Agentic OOXML: carrying editing context inside Office documents

Technical architecture paper, version 0.1 (working draft, 2026-10-29). Status: **a design with a
small reference implementation and one narrow measurement. It is not an evaluated system.** Statements
marked *implemented* describe code in this repository; *proposed* describes design that is not built; *measured*
refers to the single experiment reported in section 7.

## Abstract

Agents that edit Office documents lose the context that makes edits correct: who the document is for,
which figures are audited, what a reviewer rejected. That context normally lives in a conversation. We
describe a small convention that stores it inside the document as custom XML parts of the Open
Packaging Conventions, discovers it by relationship and namespace, treats it as untrusted data, and
records a fingerprint of each sourced claim so that a changed claim is reported stale when read. A Rust
library and command line implement the v0.1 parts. We report one measurement of part retention in
one editor's pipeline and state what is not measured: Microsoft Office, LibreOffice, ONLYOFFICE and Google
round trips, and any effect on agent task success. We pose four research questions.

## 1. Motivation

A native OOXML file records content and layout (ECMA-376 / ISO/IEC 29500 [1]). It does not reliably
record purpose, audience, an editing policy, or the origin of a number. Two failure modes follow.

**Lost context.** When a session ends, the file is transferred, or a different model opens it, the
guidance is gone, and regenerating it is costly and error-prone.

**Shadow structure.** An external object graph or sidecar duplicates what OOXML already says (slides,
shapes, sheets, relationships) and drifts when the file is copied, renamed or saved elsewhere.

The design goal is therefore narrow: *persist only what OOXML cannot say, inside the file, and
never mirror its structure.*

## 2. Representation

Four kinds of knowledge are carried (v0.1, *implemented*):

| kind | content | form |
|---|---|---|
| `agents` | editing guidance for this document | summary plus Markdown sections |
| `design` | visual and layout conventions | summary plus Markdown sections |
| `intent` | audience, purpose, narrative, per-section intent | summary plus Markdown sections |
| `sources` | provenance records for claims | typed XML records |

The three guidance kinds share an envelope, `ao:knowledge` in the namespace
`urn:agentic-office:ooxml:v0.1`, with `kind`, `schemaVersion`, an `ao:summary` and `ao:section`
elements (`id`, `format`) whose bodies are CDATA. The text stays Markdown so a language model can read it directly, and the
envelope gives selective retrieval by section id. Sources are typed XML because their value is in being checkable.

A v0.2 context (`ao:doc`, `ao:units`, `ao:unit`, `ao:spec`, namespace `urn:agentic-office:context:v0.2`)
is richer: file-level design and intent, per-unit (slide, section, sheet) role, constraints and sources,
and a requirement contract with assumptions, open questions and acceptance criteria. This repository
*reads* v0.2 and does not write it.

*Proposed, not built:* typed design tokens, a `validation` part, an agent registry with capabilities
and authorisation, and cross-document dependency tracking.

Guidance is deliberately not authority. Section text is free Markdown and the library never
interprets it.

## 3. OOXML integration

Parts are ordinary custom XML parts [2, 3]: an item part, a standard properties part carrying a GUID and the
schema namespace, and a `customXml` relationship from the **main** document part (`word/document.xml`,
`ppt/presentation.xml` or `xl/workbook.xml`). The content-types file gains one override per
properties part.

**Discovery** requires both a `customXml` relationship from the main part and a root element in the
`ao` namespace. File names are not identifiers. A part renamed by another editor
(`customXml/item7.xml`) is found, and updated in place under that name. Foreign custom XML parts (for
example SharePoint metadata) are skipped.

**Scope.** Parts are linked from the main part only. Slide-level parts were considered and
rejected for v0.1; section 7 gives the reason.

**Writes** are atomic: a new package is built beside the original and renamed over it. Entries other than
the new parts, `[Content_Types].xml` and the main part's relationship file are copied raw with their original
compression. Permissions are kept; a read-only file is refused.

## 4. Agent interface

Access is layered so an agent loads only what a task needs (*implemented* as CLI commands, JSON on stdout):

| level | purpose | command |
|---|---|---|
| L0 | discovery: kinds, versions, summaries, section ids | `list` |
| L1 | guidance: one section | `read --kind K --section S` |
| L2 | evidence: records attached to a claim | `sources --part P --object O` |
| L3 | the live OOXML object and neighbours | left to the agent's own OOXML tooling |

Package-level reads are `inspect` (inventory with per-part hashes and native units) and `validate`
(structural checks), and `diff` compares two packages. Mutation is `write` (merge, upsert, delete by
section or record id) and `reconcile`. A stdio MCP server (*implemented*) exposes the same eight
operations: the six reads are always available; the two writes refuse unless the operator sets
`AGENTIC_OOXML_ALLOW_WRITE=1`; every path must sit under an operator-configured root; and writes
produce a sibling proposed copy unless the caller asks for an in-place write.

Every result that includes embedded text carries `"trust": "document"`.

## 5. Verification model

**Claim fingerprints.** A source record points at a part and optionally a shape id. The kernel stores a
blake3 hash [4] of the claim: the raw bytes of the smallest `p:sp`, `p:graphicFrame`, `p:pic`, `p:cxnSp` or
`p:grpSp` element whose own `p:cNvPr` has that id (granularity `object`), or of the whole part
(granularity `part`, including every fallback). The caller cannot supply the hash.

**Effective status.** The status a consumer should act on is computed at read time. A record stored
`verified` is effectively `stale` if its hash is missing (`no_baseline`), differs from the current claim
(`claim_changed`), or its claim is gone (`claim_missing`). The rule is "prefer a visible *unverified*
to a silently *verified*", and it makes detection independent of which tool edited the file.

**Reconcile** persists the computed result as `status="stale"`, keeps the old hash, and is idempotent.

**Structural validation.** `validate` checks ZIP and XML well-formedness, content types, that
relationships resolve, that Agentic OOXML parts parse, and source staleness. It is not schema
validation and does not establish that Office will open or render the file.

**What it can and cannot establish.** It detects that claim-bearing bytes changed. It cannot determine whether the new
value is correct; that needs the source or a person. Formatting edits elsewhere on the slide do not
invalidate an object-level claim (tested). A hash inside the same editable file is evidence of change, not of authorship.

**Locators.** A shape id is not a durable identity across copy, split, reorder or foreign saves. The
design answer is to invalidate rather than guess: a missing object makes the record stale. A more
stable locator is an open question (RQ3).

## 6. Threat model

Assets: the integrity of the user's document; the user's and agent's permissions; confidential metadata.
Adversary: the author of a file the user opens.

| threat | defence (*implemented*) | gap |
|---|---|---|
| Prompt injection through embedded text (indirect injection, [5]) | trust marker; no code acts on embedded text; authority order: application policy, user request, trusted rules, document preferences | the consuming agent can still be influenced |
| Decompression bomb | entry cap 10 000; bounded reads, 2 MiB per part, 64 MiB for hashed claims | no dedicated test |
| Path traversal | `..` and absolute entry names rejected | none known |
| XML entity abuse | `<!DOCTYPE` rejected in Agentic OOXML parts | foreign parts are skipped, not validated |
| Resource exhaustion (v0.2) | node, depth, list and length caps on read | none known |
| Partial write | temp file plus rename | atomicity of rename depends on the filesystem |
| Metadata leakage | none | no redaction or classification; parts are in an unencrypted ZIP |
| Tampering | none | hashes are not signed; strong integrity needs an external anchor |

The central rule is that embedded text is untrusted input even when the file was last written by
this tool. The checks are code and never consult embedded policy.

## 7. Evaluation status

**Measured (one experiment, one editor, one format; 2026-10-19).** The Agentic Office project's `.pptx` editor was
tested with five encodings of the four parts built on a real deck, through the editor's own import
and save, with and without an edit. Retention required: found, byte-identical, still related with the
same relationship type, content type intact.

| encoding | outcome |
|---|---|
| standard `customXml/itemN.xml` plus properties plus `customXml` relationship | 4 of 4 kept, both runs |
| friendly names, standard relationship, no properties part | 4 of 4 kept, both runs |
| custom relationship type and content type | 4 of 4 kept, both runs |
| slide-level (related from a slide) | part kept, relationship lost, both runs |
| orphan control (no relationship) | present, unreferenced, as built |

Six other write paths in the same project (OfficeCLI add slide, remove slide, set and template merge;
a package merge; a plain rewrite) each kept all four parts present and referenced. The raw results and
scripts are in [docs/evidence/round-trip-2026-10-19](../evidence/round-trip-2026-10-19/README.md); they
depend on the source monorepo and cannot be re-run from this repository. A later fix in that editor
re-attaches per-slide relationships; its regression test (5 tests) was not re-run here, so the
slide-level row is the pre-fix result. None of this is evidence about PowerPoint, Word, Excel,
LibreOffice, ONLYOFFICE or Google.

**Tested in this repository.** Unit tests on synthetic minimal packages (43 in the library, 5 for the command line, 9 over stdio for the MCP server at the time of writing). They cover: write, list and read on
`.docx`, `.pptx`, `.xlsx`; merge and delete; renamed-part discovery; CDATA and DOCTYPE round trips;
unsafe entry names; malformed-part isolation; object-level staleness without false invalidation on a
neighbouring shape, and reconcile; permissions; v0.2 and legacy reads.

**Not measured.** Microsoft PowerPoint, Word and Excel; LibreOffice; ONLYOFFICE; Google import and
export. Any effect on agent edit success, context use, or provenance recall. Index latency (there is no
index). Cross-session handoff. The proposal's target of at least 95 percent provenance recall is a
proposed threshold, not a result. No benchmark has been run.

## 8. Limitations

- Retention is established for one editor's pipeline only; the dominant compatibility risk is open.
- `design` and `agents` are free text. Nothing is typed or enforced.
- Staleness is per file and per claim. Nothing propagates across documents.
- Hashes are unsigned; the model detects change, not authorship.
- Object-level hashing covers PresentationML shapes. For `.docx` and `.xlsx` claims the hash covers the whole part,
  which over-invalidates.
- A source `resource_uri` is stored, not resolved or checked.
- The v0.2 writer is outside this repository.
- No privacy tooling for export.
- Embedded context can be wrong or stale; nothing validates the prose.

## 9. Research questions

These are carried from the project's proposal. None is answered.

**RQ1.** Can embedded semantic guidance improve cross-session edit fidelity compared with prompts and
native OOXML alone? *Design:* baseline A (native OOXML and a general skill), baseline B (workspace-level
guidance only), treatment C (embedded `agents`, `design`, `intent`, selective retrieval), treatment D (C plus
`sources` and reconciliation); tasks spanning factual update, restyling, restructuring, copy and
paste, and handoff; human-judged success.

**RQ2.** Can a minimal set of custom XML parts persist across common create, edit and save pipelines
without impairing compatibility? *Partly measured (section 7); the Office, LibreOffice, ONLYOFFICE and
Google matrix is open.*

**RQ3.** Can a reconciliation engine prevent or detect metadata drift at low incremental cost?
*Open:* what locator is stable under copy, move and reorder; what recall and false-invalidation rates a
shape-level hash achieves on a realistic edit corpus.

**RQ4.** Can progressive disclosure reduce agent context consumption while preserving task accuracy?
*Open:* compare input tokens per completed task against loading everything.

## References

1. ECMA-376, Office Open XML File Formats. <https://ecma-international.org/publications-and-standards/standards/ecma-376/>
2. Microsoft Learn, About the Open XML SDK for Office. <https://learn.microsoft.com/en-us/office/open-xml/open-xml-sdk>
3. Microsoft Learn, PresentationPart.AddCustomXmlPart. <https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.packaging.presentationpart.addcustomxmlpart>
4. BLAKE3. <https://github.com/BLAKE3-team/BLAKE3>
5. K. Greshake et al., Not what you've signed up for: Compromising real-world LLM-integrated applications with indirect prompt injection, 2023. <https://arxiv.org/abs/2302.12173>
6. Model Context Protocol. <https://modelcontextprotocol.io>
7. Open Knowledge Format. <https://github.com/GoogleCloudPlatform/open-knowledge-format>
8. C2PA, Coalition for Content Provenance and Authenticity. <https://c2pa.org>
9. W3C, PROV Overview. <https://www.w3.org/TR/prov-overview/>
