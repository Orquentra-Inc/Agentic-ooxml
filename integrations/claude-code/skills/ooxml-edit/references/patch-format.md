# Patch format (kinds agents, design, intent)

```json
{
  "summary": "optional one-line summary of the part",
  "sections": [
    { "id": "tone", "format": "markdown", "text": "Keep a formal register." },
    { "id": "obsolete", "delete": true }
  ]
}
```

- Sections are upserted by `id`; `delete: true` removes one. `format` and `text` are optional.
- Without `replace`, the patch merges into the existing part; with `replace` it replaces the part.

# Patch format (kind sources)

Takes `"sources": [...]` records instead of `summary`/`sections`; mixing them is an error. The record fields
are defined in the library (`SourcePatch` in `crates/agentic-ooxml/src/lib.rs`). Read existing records with `ao_sources` first and copy their field names.

Kinds `doc`, `units`, `legacy-deck`, `spec` are read-only here.
