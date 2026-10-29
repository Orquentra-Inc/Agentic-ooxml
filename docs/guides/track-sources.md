# Track and re-check source claims

Use a `sources` part when a number or statement in the file came from somewhere else and you want
to know when it may have gone out of date. The walk-through in [Getting started](../getting-started.md#5-track-a-claim)
shows the full loop; this page covers the variations.

```bash
mkdir -p target/ao-guide2
python3 docs/guides/make-minimal-deck.py target/ao-guide2/deck.pptx
```

## Granularity: object or part

A claim points at a `claim_part` and, optionally, a `claim_object_id`.

- On a `.pptx` slide part with a matching shape id, the hash covers that one shape
  (`"granularity": "object"`). Editing a different shape on the slide does not invalidate it.
- In every other case the hash covers the whole part (`"granularity": "part"`): any `claim_object_id`
  on a `.docx` or `.xlsx` part, or an id that is not found. A missing object also reports
  `object_missing`.

```bash
cat > target/ao-guide2/s.json <<'JSON'
{ "sources": [
  { "id": "whole-slide", "status": "verified", "claim_part": "ppt/slides/slide1.xml" },
  { "id": "one-shape",   "status": "verified", "claim_part": "ppt/slides/slide1.xml", "claim_object_id": "3" }
] }
JSON
agentic-ooxml write --file target/ao-guide2/deck.pptx --kind sources --input target/ao-guide2/s.json
agentic-ooxml sources --file target/ao-guide2/deck.pptx
```

## Filter by where the claim lives

```bash
agentic-ooxml sources --file target/ao-guide2/deck.pptx --part /ppt/slides/slide1.xml --object 3
```

This is the "which evidence is attached to this object" lookup.

## Statuses

| status | meaning |
|---|---|
| `verified` | someone checked the claim against the source. The tool takes a hash baseline when you write it. |
| `unverified` | recorded but not checked. Passes through unchanged. |
| `stale` | the claim changed after it was verified, or its object is gone. |
| `manual` | a person overrode the check. Passes through unchanged. |

Only `verified` records are re-checked. A `verified` record whose hash is missing, differs, or whose
claim is gone is reported with `effective_status: "stale"` and a `stale_reason` of `claim_changed`,
`claim_missing` or `no_baseline`.

## Re-verify after you checked

```bash
cat > target/ao-guide2/v.json <<'JSON'
{ "sources": [ { "id": "one-shape", "status": "verified" } ] }
JSON
agentic-ooxml write --file target/ao-guide2/deck.pptx --kind sources --input target/ao-guide2/v.json
```

This takes a new baseline from the current bytes. Writing `verified` on a record whose claim object
no longer exists is refused.

## Remove a record

```bash
cat > target/ao-guide2/d.json <<'JSON'
{ "sources": [ { "id": "whole-slide", "delete": true } ] }
JSON
agentic-ooxml write --file target/ao-guide2/deck.pptx --kind sources --input target/ao-guide2/d.json
```

## Claims about another file (cross-document staleness)

If the number came from a workbook (or another document) next to this one, record that with an
`external_claim`. The library then opens the other file, read-only, and compares the value it finds
with a baseline taken when you verified.

```bash
cat > target/ao-guide2/x.json <<'JSON'
{ "sources": [ { "id": "q3-kpi", "status": "verified",
  "claim_part": "ppt/slides/slide1.xml", "claim_object_id": "3",
  "external_claim": { "file": "model.xlsx", "sheet": "Summary", "cell": "B1" } } ] }
JSON
agentic-ooxml write --file target/ao-guide2/deck.pptx --kind sources --input target/ao-guide2/x.json
agentic-ooxml sources --file target/ao-guide2/deck.pptx   # external_check shows the value read now
```

When the cell changes, the record reports `effective_status: "stale"` with `stale_reason`
`external_claim_changed` (other reasons: `external_file_missing`, `external_object_missing`,
`external_path_invalid`, `external_outside_roots`, `external_unreadable`; see
[schema](../schema.md#external-claims-aoexternalclaim)). `validate`, `diff`, `reconcile` and the MCP
tools use the same check. The loop an agent runs:

1. `sources` shows the record stale with `external_check.current_value` (document data).
2. Update the slide or paragraph by whatever means you have for editing it. This library does not.
3. `write` the record again with `"status": "verified"`: it re-reads the workbook value and takes a
   new baseline.

Limits, stated plainly:

* It compares values it can read: cached `.xlsx` cell values (numbers, shared and inline strings,
  booleans) or the text of a `.docx`/`.pptx` part or slide shape. It does **not** recalculate
  formulas; a formula cell with no cached value is `external_object_missing`.
* It does not compare the claim's text with the workbook value. It detects that the source moved
  since you last verified, not that the slide is right. Re-verifying is a human or agent judgement.
* The file must be relative, at or below the document's directory, with no `..`, absolute path or
  escaping symlink. The MCP server additionally keeps it inside `AGENTIC_OOXML_ROOTS`. Nothing is
  edited in the other file.
* `diff` evaluates each side against the files in its own directory as they are now.

## Do not trust the stored hash as proof

The hash lives in the same editable file as the claim. It detects change; it does not prove who
made the change. See [security](../security.md#what-this-does-not-protect-against).
