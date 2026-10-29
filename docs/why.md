# Why Agentic OOXML?

## The problem

Agents edit Office files all the time, and the file is a poor place to read intent from. A `.pptx`
does not say that slide 7 must label forecasts, that the audience is a board, or that an earlier approach was rejected. A
shadow file next to the document can say these things, but it drifts as soon as the document is
copied or saved elsewhere. So each new session starts from a guess.

Agentic OOXML puts a small amount of typed, readable context **inside** the OOXML package, finds it
without trusting file names, treats it as untrusted data, and keeps a fingerprint of what a claim
pointed at so a changed number is noticed instead of silently trusted.

## What it is not

- It is not a new document format. The host file is an ordinary OOXML package.
- It is not a published standard. The semantics of the parts are a project convention, not part of
  ECMA-376 or ISO/IEC 29500. "Custom XML is allowed" does not mean "universally preserved".
- It does not replace an OOXML library. It does not edit slides, cells or paragraphs.
- It does not verify that your numbers are right.

## Compared with prior work

| approach | what it does well | how Agentic OOXML differs |
|---|---|---|
| [OOXML](https://ecma-international.org/publications-and-standards/standards/ecma-376/) native structure (styles, themes, notes, alt text, named ranges) | the real source of truth for content and layout | used as is; this project does not copy it. It adds only what it cannot express: purpose, policy, provenance. |
| [Open XML SDK](https://learn.microsoft.com/en-us/office/open-xml/open-xml-sdk), [python-docx](https://python-docx.readthedocs.io), Apache POI | read and edit documents; the SDK can attach [custom XML parts](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.packaging.presentationpart.addcustomxmlpart) | the mechanism this project uses. It supplies no schema, no retrieval pattern, no staleness model. |
| Converters to Markdown or JSON ([MarkItDown](https://github.com/microsoft/markitdown), export to HTML) | give an LLM readable text of a file | one-way and lossy by design; they produce a view, not a place to keep context in the file. |
| Sidecar JSON next to the document | easy; no file changes | does not travel; drifts. |
| [AGENTS.md](https://agents.md) and similar repository files | tell an agent how to work in a repository | repository scope, not document scope. A single `.pptx` sent by e-mail loses it. Agentic OOXML is the per-file counterpart. |
| [Open Knowledge Format](https://github.com/GoogleCloudPlatform/open-knowledge-format) | Markdown with frontmatter, with provenance, trust and freshness conventions | a neighbour, not an OOXML packaging spec. The section-and-Markdown shape borrows the same instinct. Agentic OOXML parts are XML inside a package and are not OKF-conformant bundles. |
| [XMP](https://developer.adobe.com/xmp/docs/) | embedded metadata in files (images, PDF) | similar idea in a different ecosystem: carry metadata in the file. XMP is about descriptive metadata; Agentic OOXML is about editing context and claim freshness. |
| [C2PA](https://c2pa.org) | cryptographically signed provenance for media | signed and tamper-evident; Agentic OOXML's hashes are **not** signed and detect change, not authorship. The two are complementary, and C2PA is the model for the integrity step this project lacks. |
| [W3C PROV](https://www.w3.org/TR/prov-overview/) | a general model of provenance | richer and more general. Agentic OOXML's source record is a deliberately small, single-purpose subset: origin, location, hash, status. |
| [Model Context Protocol](https://modelcontextprotocol.io) | a standard way to expose tools to agents | the intended transport for the agent interface. It says nothing about what a document carries. |

## Novelty, stated honestly

Most of the pieces exist. The claim is about the combination and its discipline, not any single piece.

| class | what | assessment |
|---|---|---|
| Established | custom XML parts in OPC, ZIP/XML hardening, content hashing, atomic file replacement | not new; used as intended |
| Combination | discovering parts by relationship **and** namespace, never by file name; progressive disclosure over those parts; an instruction hierarchy that treats embedded text as data | plausible but not shown to be novel; a literature check is not done |
| Specific design choice | staleness **computed from current bytes at read time** rather than trusted from storage, with a per-shape hash on slides | a clear design choice; its value is untested against alternatives |
| Unproven | that embedded context improves agent outcomes across sessions; that selective loading saves context without losing accuracy; that parts survive Office and LibreOffice | research questions, no results yet. See the [paper](paper/architecture.md) |

If you know prior work that already covers something here, please open an issue. This project would
rather cite it than claim it.

## Who it is for

- People building agents that edit documents and who have lost context between sessions.
- Teams who want a changed number on a slide to be noticed.
- Researchers who want a concrete, small artefact to test the questions in the paper.

It is probably not for you if you need guaranteed behaviour in every Office application today. That
evidence does not exist yet.
