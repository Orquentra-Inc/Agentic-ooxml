# Security

Agentic OOXML opens ZIP files and XML from sources you do not control, and hands text from them to
agents. This page states the threat model, the defences that exist in code today, and the ones that do not.
If you find a hole, see [SECURITY.md](../SECURITY.md).

## The core rule: embedded text is untrusted document data

A knowledge part is text written by whoever last edited the file. It may have been written by a
person, an agent, or an attacker who sent you the file. So:

- Every JSON result that returns embedded text carries `"trust": "document"`.
- The text is **preferences and evidence**, never authority. It cannot grant network access, code
  execution, file access outside the user's workspace, or a change to tool policy. No code path in the library
  reads embedded text to decide what it may do.
- An `agents` part is guidance for editing. It is not a security policy. A modified `agents` part cannot disable a check,
  because the checks are Rust code that never consults it.

### Instruction hierarchy

When an agent combines sources of guidance, the order of authority, highest first:

1. the application's safety and permission policy
2. the user's explicit request in this session
3. trusted skills and tool rules (code you installed)
4. embedded document preferences (the `agents`, `design`, `intent` and v0.2 text)

If an embedded preference conflicts with a higher level, the higher level wins and the conflict
should be shown to the user, not silently resolved. This hierarchy is a rule for integrators; the library
supports it by marking trust and by never acting on embedded text, but it cannot enforce what your
agent does with the text after reading it.

## Threat model

| threat | what an attacker does | defence today | residual risk |
|---|---|---|---|
| Prompt injection through embedded text | puts instructions in `intent`, `agents`, a source `location` or a v0.2 field | trust marker; no code acts on the text; documentation tells integrators to treat it as data | the agent that reads it can still be fooled. The library cannot prevent that. |
| Decompression bomb | a small ZIP that expands enormously | entry count cap (10 000); every read goes through a bounded reader with a 2 MiB cap (64 MiB for hashed claim parts) | the caps have no dedicated test yet (a zip-bomb test is on the to-do list) |
| Path traversal ("zip slip") | an entry named `../evil` or `/etc/x` | names with `..` segments or a leading `/` are rejected | none known |
| XML entity abuse | a `<!DOCTYPE>` with entities | `<!DOCTYPE` is rejected in the prolog of every Agentic OOXML part | foreign custom XML parts are skipped, not parsed beyond their root |
| Resource exhaustion in v0.2 | huge or deeply nested context | 20 000 nodes, depth 16, caps on units, lists, sources and attribute length | none known |
| Corrupt output | a crash mid-write leaves a broken file | the new package is built in a temp file in the same directory and renamed over the original | rename is atomic on one filesystem; on others, not guaranteed |
| Native content altered by a write | a bug rewrites slides | native entries are raw-copied with their original compression; a test checks byte identity | the two package-plumbing entries (`[Content_Types].xml`, the main part's `.rels`) are rewritten to add entries |
| Data leakage | `resource_uri`, `location` or a section reveals internal paths or client names | none in code: nothing is redacted | see [Privacy](#privacy) |
| Tampering | someone edits parts, hashes or statuses | none: nothing is signed | see [below](#what-this-does-not-protect-against) |

Malformed or hostile Agentic OOXML parts do not stop a read of the others: they are listed with an
`error`, and a missing or broken optional `sources` part never prevents ordinary use of the file.

## Privacy

A source record can hold a workbook path, a cell address and a name. Section text is whatever you
wrote. All of it sits in an unencrypted ZIP that anyone with the file can unpack. This tool does not classify, redact or
strip anything on export. Before sharing a file outside your organisation, list its parts
(`agentic-ooxml list --file F`), read them, and remove what should not leave. Do not put secrets in any part.

## What this does not protect against

- **Authenticity.** `claim_hash` and `source_hash` live inside the same editable file. They detect
  accidental and honest change. They do not prove who changed the file: anyone who edits the claim can
  also rewrite the hash and status. Strong integrity needs an external anchor or a signature, which is not built.
- **A wrong "verified".** The tool records that a person or process said a claim was checked. It does not
  check the source.
- **A malicious source URI.** `resource_uri` is stored, not followed. An integrator that follows it must validate it and
  keep inside its own workspace boundary.
- **Third-party editors dropping or altering the parts.** Not measured. See [compatibility](compatibility.md).

## MCP server

`agentic-ooxml-mcp` is a stdio MCP server over the library. It is implemented and has integration tests over stdio; it is new, and has had no outside review. Its safety defaults:

- read-only tools (`ao_inspect`, `ao_list`, `ao_read`, `ao_sources`, `ao_validate`, `ao_diff`) are always available;
- write tools (`ao_write`, `ao_reconcile`) refuse unless the operator sets `AGENTIC_OOXML_ALLOW_WRITE=1` in the server's environment. Running the server without it and calling `ao_write` returns `write_disabled`;
- every path must sit under a root listed in `AGENTIC_OOXML_ROOTS` (colon-separated, default the current directory). A path outside returns `path_outside_roots`, and paths are checked after resolving symlinks and `..`;
- writes produce a sibling `.ao-proposed` copy unless the caller asks for `in_place: true`.

The gating is set by the operator's environment, not by the agent or the document. The two refusals
above were observed by running the server. See also [integrations/mcp](../integrations/mcp/README.md).

## Operating guidance

- Run agents on a copy of important files, and diff before replacing the original.
- Treat `"trust": "document"` as a hard boundary in your own prompts: put embedded text in a clearly
  delimited data block and never in the instruction channel.
- Do not feed `write` input that was built from document text without review.
- Keep `resource_uri` handling in your own code strict. Do not resolve relative paths outside the workspace.
