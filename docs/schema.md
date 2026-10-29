# Schema reference

What the code reads and writes today, field by field. Status labels:

- **Implemented**: read and written by this library, with tests.
- **Read-only**: parsed by this library; written by the Agentic Office TypeScript servers.
- **Proposed**: described in the design, not implemented. Not part of any file this tool produces.

All examples are real. The v0.1 ones were produced by running the CLI on the sample file from
[Getting started](getting-started.md); the v0.2 ones are the fixtures in the library's unit tests.

## Namespaces and part layout

| namespace | prefix | used by | status |
|---|---|---|---|
| `urn:agentic-office:ooxml:v0.1` | `ao` | `agents`, `design`, `intent`, `sources`, and the legacy `ao:deck` | implemented (the deck is read-only) |
| `urn:agentic-office:context:v0.2` | `ao` | `ao:doc`, `ao:units`, `ao:unit`, `ao:spec` | read-only |

Package layout of a part this library creates, for a `.pptx`:

```text
customXml/ao-<kind>.xml                  the knowledge part
customXml/ao-<kind>Props.xml             ds:datastoreItem with a GUID derived from the kind
customXml/_rels/ao-<kind>.xml.rels       customXmlProps relationship to the props part
ppt/_rels/presentation.xml.rels          customXml relationship to the item (the main part's .rels)
[Content_Types].xml                      Override for the props part
```

`<kind>` is `agents`, `design`, `intent` or `sources`. For `.docx` the relationship goes in
`word/_rels/document.xml.rels`; for `.xlsx`, `xl/_rels/workbook.xml.rels`. Parts are discovered by relationship
plus namespace, so the file names are not significant. The properties part is the standard
Office custom XML properties part:

```xml
<ds:datastoreItem ds:itemID="{E7AC6DCC-6D08-37C4-730B-231AD57275A6}" xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml"><ds:schemaRefs><ds:schemaRef ds:uri="urn:agentic-office:ooxml:v0.1"/></ds:schemaRefs></ds:datastoreItem>
```

## ao:knowledge: agents, design, intent

**Status: implemented (v0.1).** One root element for three kinds, distinguished by the `kind`
attribute. The kind decides what the text is for; the format does not yet check what you write.

| element or attribute | meaning |
|---|---|
| `ao:knowledge` | root. Attributes `kind` (`agents`, `design` or `intent`) and `schemaVersion` (`0.1`). |
| `ao:summary` | one line, shown by `list` |
| `ao:section` | zero or more. Attributes `id` (unique within the part) and `format` (default `text/markdown`). The body is CDATA. |

What the three kinds are for:

- `intent`: audience, purpose, narrative, what each section or slide is for, durable decisions.
- `design`: visual and layout preferences, chart and table conventions. Today this is free Markdown
  text. It is **not** a set of typed design tokens (see [proposed](#proposed-not-implemented)).
- `agents`: document-specific editing rules and maintenance triggers. Today this is free Markdown
  text. It is **not** an agent registry with capabilities or authorisation. It is guidance only: it
  cannot grant a permission or disable a check.

Real output (`write --kind intent`, then the file's `customXml/ao-intent.xml`):

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ao:knowledge xmlns:ao="urn:agentic-office:ooxml:v0.1" kind="intent" schemaVersion="0.1">
  <ao:summary>Board narrative and constraints</ao:summary>
  <ao:section id="slide-1" format="text/markdown"><![CDATA[Purpose: show acceleration.
Constraint: label forecasts; do not alter audited actuals.]]></ao:section>
</ao:knowledge>
```

### Write input

`write --kind K --input FILE` takes JSON. All fields are optional.

```json
{
  "summary": "Board narrative and constraints",
  "sections": [
    { "id": "slide-1", "format": "text/markdown", "text": "..." },
    { "id": "old-section", "delete": true }
  ]
}
```

Merge rules: `summary` replaces if present; sections are upserted by `id`, keeping existing order and
appending new ids; `"delete": true` removes a section. `--replace` writes the input as the whole part.

### Behaviour worth knowing

- Text containing the CDATA terminator `]]>` round-trips (it is split on write).
- Text containing `<!DOCTYPE` inside a section body round-trips; a `<!DOCTYPE` in the part's prolog is rejected.
- Characters that are illegal in XML 1.0 are rejected on write.
- Unknown child elements are skipped on read (forward compatibility).
- A part with `schemaVersion` newer than `0.1` is not merged into.
- A malformed part is reported with an `error` in `list` and does not hide the other parts.

### Read output

```json
{
  "file": "target/ao-demo/deck.pptx",
  "kind": "intent",
  "trust": "document",
  "summary": "Board narrative and constraints",
  "sections": [
    { "id": "slide-1", "format": "text/markdown", "text": "Purpose: show acceleration.\nConstraint: label forecasts; do not alter audited actuals." }
  ]
}
```

## ao:sources

**Status: implemented (v0.1).** Source records: where a claim came from, and a fingerprint of the claim.

```xml
<ao:sources xmlns:ao="urn:agentic-office:ooxml:v0.1" schemaVersion="0.1">
  <ao:source id="src-q4-forecast" status="verified" sourceKind="workbook" checkedAt="2026-10-01">
    <ao:resource uri="relative:Financial_Model.xlsx"/>
    <ao:location>Forecast!D12</ao:location>
    <ao:claimLocator part="/ppt/slides/slide1.xml" objectLocalId="3" claimHash="blake3:62dd5530b94d607cec6bd9eeb71254a292df3b00af18b624362ba50ae30f3564"/>
    <ao:externalClaim file="Financial_Model.xlsx" sheet="Forecast" cell="D12" claimHash="blake3:9e269f41177c68adfd68c6c0a04103dd6634c1a309f384c0e10fda811e54c2f6"/>
    <ao:sourceHash algorithm="blake3">abc123</ao:sourceHash>
  </ao:source>
</ao:sources>
```

`ao:externalClaim` is an optional child added after v0.1 was first written, with no schema version bump:
a file without it is unchanged, and a reader that does not know it skips it (this library's own
rewrites re-serialize the part from the records it understands, as for any unknown element).

| field (JSON input and output) | XML | meaning |
|---|---|---|
| `id` | `ao:source/@id` | required, unique |
| `status` | `@status` | `verified`, `unverified`, `stale` or `manual`. Default when missing on read: `unverified`. |
| `source_kind` | `@sourceKind` | free text, for example `workbook` |
| `checked_at` | `@checkedAt` | free text date |
| `resource_uri` | `ao:resource/@uri` | where the source is. A label: not resolved or trusted by this tool (use `external_claim` for a checked link). |
| `location` | `ao:location` | a position inside the source, for example `Forecast!D12` |
| `claim_part` | `ao:claimLocator/@part` | OPC part name of the claim, stored with a leading `/` |
| `claim_object_id` | `ao:claimLocator/@objectLocalId` | the shape id on a slide part |
| `claim_hash` | `ao:claimLocator/@claimHash` | **kernel-set**, `blake3:<hex>`. Ignored if you supply it. |
| `source_hash`, `source_hash_algorithm` | `ao:sourceHash` | a hash of the external source, as recorded by the caller. The tool stores it and does not check it. |
| `external_claim` | `ao:externalClaim` | optional cross-document claim, see below |
| `delete` | n/a | write input only: remove the record |

### External claims (`ao:externalClaim`)

A record can claim a value that lives in another file. The library reads that file (read-only) when
the sources are read and compares the value with a baseline.

| field | XML attribute | meaning |
|---|---|---|
| `file` | `file` | **required.** Relative path of a `.docx`, `.pptx` or `.xlsx`, at or below the directory of the file that holds the record. |
| `sheet`, `cell` | `sheet`, `cell` | `.xlsx` locator: sheet name (case-insensitive) and an A1 cell reference (stored upper case). Both required for an `.xlsx`. |
| `part`, `object_id` | `part`, `objectLocalId` | `.docx`/`.pptx` locator: an OPC part name (stored with a leading `/`) and optionally a slide shape id on `ppt/slides/`. `part` required. |
| `claim_hash` | `claimHash` | **kernel-set**, `blake3:<hex>` of the canonical value text; ignored if you supply it. |

The value text that is hashed:

* `.xlsx`: the cached cell content. Numbers are normalised (`75`, `75.0`, `7.5E1` agree), shared
  strings and inline strings are resolved (phonetic runs excluded), `t="str"` formula results, booleans
  (`TRUE`/`FALSE`) and error texts are read as stored. A formula cell with no cached `<v>` has no
  readable value. Formulas are **not recalculated**.
* `.docx`/`.pptx`: the text runs (`t` elements) of the part, or of the one slide shape, one line per
  paragraph. Formatting-only edits do not change it.

Path rules, checked on write (the record is refused) and again on every read: the path must be
relative; an absolute path, a `..` segment, a backslash, a drive letter or scheme (`:`), an empty
segment or a control character is refused; symlinks are resolved and the real file must stay under
the record's directory (a symlink or linked directory that leaves it is refused). The MCP server also
requires the real file to sit under `AGENTIC_OOXML_ROOTS`; a file outside the roots is never opened.
The target is opened with the same entry-count and part-size caps as any package this library reads.

Reasons reported (`stale_reason`, `external_check.reason`):

| reason | meaning |
|---|---|
| `external_claim_changed` | the value read now hashes differently from the baseline |
| `external_file_missing` | the target file does not exist (or is a dangling symlink) |
| `external_object_missing` | the file opens, but the sheet, cell, part or shape is not there, or the cell has no cached value |
| `external_path_invalid` | the path breaks a rule above, or resolves outside the record's directory |
| `external_outside_roots` | MCP only: the real file is outside `AGENTIC_OOXML_ROOTS`; not opened |
| `external_unreadable` | not a readable package, over a size cap, or malformed XML |
| `no_baseline` | verified but no stored external hash |

A `verified` record with any of these is `effective_status: "stale"`. Other statuses keep their status
and still report `external_check`. When both an external and an in-package reason apply, the external
one is `stale_reason` and all are listed in `stale_reasons`.

Write path: put `external_claim` in a `sources` patch. Writing `status: "verified"` (with or without
the claim in the same patch) re-reads the value and stores the new `claim_hash`; if the value cannot be
read, the verification is refused with the reason. Changing the locator clears the old baseline.
A record may have only an `external_claim` (no `claim_part`).

```json
{ "sources": [ { "id": "q3-kpi", "status": "verified",
  "claim_part": "ppt/slides/slide2.xml", "claim_object_id": "5",
  "external_claim": { "file": "financial-model.xlsx", "sheet": "Summary", "cell": "B1" } } ] }
```

The library never edits the other file or the claim itself: it tells you the value moved; a person or
agent updates the content and re-verifies.

Read adds computed fields:

| field | values |
|---|---|
| `effective_status` | as `status`, except `verified` becomes `stale` when the claim has changed |
| `stale_reason` | `claim_changed`, `claim_missing`, `no_baseline`, or an `external_*` reason (above) |
| `stale_reasons` | every reason, only when there is more than one |
| `external_check` | for a record with an external claim: `file`, `locator`, `reason`, `current_value` (clipped, document data), `current_hash` |
| `granularity` | `object` (hash covers one slide shape) or `part` (hash covers the whole part) |

Rules: writing `status: "verified"` makes the kernel compute and store `claim_hash` from the current
file; verifying a claim whose object is missing is refused; a claim part that does not exist is
`claim_missing`. See [Concepts: staleness](concepts.md#staleness) and the
[sources guide](guides/track-sources.md).

## v0.2 context (read-only)

**Status: read-only.** Written by the Agentic Office servers (TypeScript), not by this library. Files
with a v0.2 `ao:doc` refuse `write` here. Output carries `"trust": "document"`. The shapes below are
the library's unit-test fixtures. The v0.2 format is defined by its writer, which lives outside this
repository; the library reads the fields below and ignores others.

### ao:doc

```xml
<ao:doc xmlns:ao="urn:agentic-office:context:v0.2" schemaVersion="0.2" kind="docx" rev="2">
  <ao:intent purpose="inform"/>
</ao:doc>
```

Read: `kind`, `rev`, `schemaVersion`, attributes of `ao:design` and `ao:intent`, `ao:pref` lines,
file-level `ao:source` records, and the `ao:index` of units.

### ao:units / ao:unit

```xml
<ao:units xmlns:ao="urn:agentic-office:context:v0.2" schemaVersion="0.2" kind="section">
  <ao:unit id="u-1" ref="1A2B3C4D" rev="1" status="fresh"><ao:role>summary</ao:role><ao:source id="s1" kind="report" status="verified"/></ao:unit>
  <ao:unit id="u-2" ref="t:9f8e7d6c" rev="3" status="stale"><ao:role>risks</ao:role><ao:constraint>Name an owner per risk</ao:constraint><ao:source id="s2" kind="memo"/></ao:unit>
  <ao:unit id="u-3" ref="DEADBEEF" rev="1" status="orphan"><ao:intent>Was the appendix</ao:intent></ao:unit>
</ao:units>
```

A unit is a slide, a heading section or a sheet. `status` is `fresh`, `stale` or `orphan`. On `.pptx` and
`.xlsx` the reader checks each unit's `ref` against the package (slide ids, sheet names) and reports a
unit whose ref no longer exists as an orphan. A `.pptx` stores one part per slide; the listing names
them all in `unit_parts`.

### ao:spec

```xml
<ao:spec xmlns:ao="urn:agentic-office:context:v0.2" schemaVersion="0.2" rev="4" mode="deep" format="docx">
  <ao:request>Write the memo</ao:request>
  <ao:requirement id="r1" source="file">Use the Q3 actuals</ao:requirement>
  <ao:assumption id="a1" status="unconfirmed">British spelling</ao:assumption>
  <ao:question id="q1" status="open"><ao:text>Which audience?</ao:text></ao:question>
  <ao:criterion id="c1" check="tool" status="met">No overlaps</ao:criterion>
  <ao:grounding>Q3 revenue (actual): 18.6m (Q3 Actuals.xlsx)</ao:grounding>
</ao:spec>
```

Only `open` questions are returned. Assumption status is `confirmed` or otherwise `unconfirmed`;
criterion status is `met` or otherwise `unmet`; criterion `check` defaults to `manual`.

### Legacy `ao:deck` (v0.1)

The earlier TypeScript pptx writer's single-part form, in the v0.1 namespace. Listed as `legacy-deck`
and readable with `read --kind legacy-deck`. Writes to `agents`, `design`, `intent` and `sources` beside it still work.

### Read caps

400 units, 20 items per list, 60 sources per scope, 600 characters per attribute, 20 000 XML nodes,
nesting depth 16.

## Limits that apply to every part

| limit | value |
|---|---|
| entries in a package | 10 000 |
| size of an Agentic OOXML part (uncompressed, read through a bounded reader) | 2 MiB |
| size of a claim part (hashed only) | 64 MiB |
| entry names | no `..` segments, no leading `/` |
| XML | `<!DOCTYPE` rejected |

## Proposed, not implemented

These appear in the original design. None is produced, read or checked by this library. Do not rely on them.

| item | what the design says | what exists today |
|---|---|---|
| Typed design tokens | `design` carries named colour, type and spacing roles | `design` is free Markdown |
| `validation.xml` | a part of checks a document must satisfy | no such part |
| Agent registry in `agents` | capabilities and authorisation per agent | `agents` is free Markdown policy |
| v0.2 writer here | one Rust writer for v0.2 | v0.2 is written by the TypeScript servers; read-only here |
| Automatic cross-file edits | proposing the slide edit when a workbook value changes | detection is implemented (`external_claim`); the edit is not |
| Per-slide parts | context attached to one slide | not supported; the one editor measured dropped the relationship |
| Formal schema registry and a published specification | versioned XSD and conformance tests | none yet |
