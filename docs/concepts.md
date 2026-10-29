# Concepts

The ideas behind Agentic OOXML, in the order you need them. Every statement about behaviour is
about the code in `crates/agentic-ooxml`; statements about the design that is not built are marked
**proposed**.

## OPC in two minutes

A `.docx`, `.pptx` or `.xlsx` file is a ZIP archive that follows the Open Packaging Conventions
(OPC), part of ECMA-376 and ISO/IEC 29500. Three things matter here:

- **Parts.** Every file inside the ZIP is a part with a name such as `/ppt/slides/slide1.xml`.
- **Content types.** `[Content_Types].xml` says what each part is.
- **Relationships.** `.rels` files link parts together. `_rels/.rels` points at the main part
  (`word/document.xml`, `ppt/presentation.xml` or `xl/workbook.xml`); the main part's own `.rels` points at
  everything it uses.

OPC also defines "custom XML parts": arbitrary XML a document can carry, linked from the main part
by a `customXml` relationship. Agentic OOXML uses exactly that mechanism. It adds no new package
feature and changes none of the native parts.

## Why a semantic layer

An agent can open the raw parts of a file, but the parts do not say who the audience is, why a slide
exists, which figure was audited and which is a forecast, or what a reviewer rejected last week. That context
usually lives in a chat, and is gone when the session ends, the file is emailed, or another model
opens it.

The usual fix is a sidecar file next to the document, or a copy of the document's structure in a
JSON file. Both drift: the document is copied, renamed or saved in another editor and the sidecar
no longer matches. Agentic OOXML takes the opposite position on both points:

- **Carry the context inside the file**, so it travels with the bytes.
- **Do not copy the native structure.** Shapes, styles, sheets and relationships are already in the
  OOXML. The extra parts hold only what OOXML cannot say: purpose, design intent, policy and provenance.

## Identity

A knowledge part is not found by its file name. A part is an Agentic OOXML part when two things are
both true:

1. the main part reaches it through a `customXml` relationship, and
2. its root element is in the `ao` namespace.

So `customXml/ao-intent.xml` and `customXml/item7.xml` are both found, as long as the relationship and
the namespace are intact. When an existing part is updated, it is rewritten in place under whatever
name another editor gave it. The names the tool creates (`customXml/ao-<kind>.xml`) are a convenience.

The same care applies to claims. A claim points at a part and an object id (`p:cNvPr id`, the shape's local id on a
slide). A shape id is not a universal identity: it can change when the slide is copied, split or
saved elsewhere. The tool therefore stores a hash of what the claim pointed at, and when the object cannot be
found the record turns stale (`claim_missing`) rather than attaching to a different object.
Whether a more stable locator exists is an open question ([paper, RQ3](paper/architecture.md#9-research-questions)).

## Provenance

A source record answers three questions about a claim in the document: where it came from
(`resource_uri`, `location`, `source_kind`), when it was last checked (`checked_at`), and what it
looked like when it was checked (`claim_hash`). It does not copy the value. It is a pointer plus
a fingerprint, not a second table of facts. See [schema](schema.md#aosources).

## Staleness

Staleness is **computed when you read**, not trusted from storage. A record stored as `verified` is
reported stale when its stored hash is absent or differs from the hash of the current claim, or when the claim part or
object is gone. The reason is one of `claim_changed`, `claim_missing` or `no_baseline`.

This design choice matters: it makes detection independent of which tool edited the file. A person
changing a number in PowerPoint, a script, another agent and a Rust tool all produce the same
result, because the comparison is against the bytes in the file.

`reconcile` turns the computed result into stored state (`status="stale"`), keeping the old hash as
the baseline. It is idempotent and has a dry-run mode.

What staleness is not: it says a claim-bearing object changed. It cannot say whether the new value
is correct. A change to a different shape on the same slide does not invalidate a claim that points
at one shape. A change to anything on a part-level claim does. Where the file cannot say whether a
claim is still supported, the design prefers a visible "stale" to a silently "verified".

## Specification versus intent

Two words that are easy to confuse:

- **Intent** is the *why* behind the file: audience, purpose, narrative, what a section is for. It is
  guidance, written by a person or agent, and can be wrong or out of date. The `intent` part (and v0.2 `intent`
  fields) carry it.
- **Specification** (`spec`, in the v0.2 context) is the *contract* for a task: the request, the
  requirements, assumptions still to confirm, open questions, and acceptance criteria with a status
  (`met` or `unmet`) and how each is checked. Its summary line reads, for example,
  `spec (deep): 1 requirement(s), 1 unconfirmed assumption(s), 1 open question(s), 1/2 criteria met`.

Intent describes what the document is for. A specification describes what must be true before the work is done. This repository
reads the v0.2 `spec` and does not write it. A separate use of the word: "the specification" of Agentic OOXML
(the format itself) has no versioned, standalone specification document yet: the [schema reference](schema.md)
and the Rust code are the definition. See the [roadmap](../ROADMAP.md).

## Progressive disclosure

Not every edit needs every part. The access pattern is layered:

| level | what the agent loads | CLI call |
|---|---|---|
| L0 discovery | kinds, versions, summaries, section ids | `list` |
| L1 guidance | one section of `agents`, `design` or `intent` | `read --kind K --section S` |
| L2 evidence | the source record attached to a claim | `sources --part P --object O` |
| L3 objects | the live OOXML object and its neighbours | `inspect` lists native units (for example slides with their first text); full object editing is the agent's own OOXML tooling |

This repository does not edit slide, cell or paragraph content. It inventories packages (`inspect`), checks their structure (`validate`) and compares two packages (`diff`).
Whether this layered reading cuts context use without hurting accuracy is a hypothesis, not a
result ([paper, RQ4](paper/architecture.md#9-research-questions)).

## Trust

Embedded text is **document data**. It is not an instruction, however it is phrased, and it cannot widen
what the tool or the agent is allowed to do. Every JSON result that returns embedded text carries
`"trust": "document"`. The order of authority, highest first: application safety policy, the user's
explicit request, trusted tool rules and skills, embedded document preferences. See [security](security.md).
