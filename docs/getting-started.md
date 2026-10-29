# Getting started

This page takes you from a clean checkout to a deck that carries its own editing context, a source
claim that goes stale when the number changes, and a reconcile step that records it. Every command
below was run against this repository. You need a Rust toolchain (the workspace uses edition 2021)
and Python 3 for the sample-file helpers.

## 1. Build the command line

```bash
cargo build --release
export PATH="$PWD/target/release:$PATH"
agentic-ooxml --version
```

To install the binaries into `~/.cargo/bin` instead, run `cargo install --path crates/agentic-ooxml-cli`
and `cargo install --path crates/agentic-ooxml-mcp` (`cargo install` takes one `--path` per command), or
`cargo install --git https://github.com/Orquentra-Inc/Agentic-ooxml agentic-ooxml-cli agentic-ooxml-mcp`.
Prebuilt binaries and a Claude Code plugin are covered in the [README](../README.md#install). The crates
are `publish = false`: there is no crates.io package.

## 2. Make a sample file

The helper writes a tiny one-slide `.pptx` with synthetic figures (a Q4 forecast of EUR 21.3M). It is
a valid OPC package for these tools, not a polished deck.

```bash
mkdir -p target/ao-demo
python3 docs/guides/make-minimal-deck.py target/ao-demo/deck.pptx
agentic-ooxml list --file target/ao-demo/deck.pptx
```

The file carries no Agentic OOXML parts yet, so `parts` is empty:

```json
{
  "file": "target/ao-demo/deck.pptx",
  "format": "pptx",
  "trust": "document",
  "parts": []
}
```

## 3. Write intent into the file

```bash
cat > target/ao-demo/intent.json <<'JSON'
{
  "summary": "Board narrative and constraints",
  "sections": [
    {
      "id": "slide-1",
      "format": "text/markdown",
      "text": "Purpose: show acceleration.\nConstraint: label forecasts; do not alter audited actuals."
    }
  ]
}
JSON
agentic-ooxml write --file target/ao-demo/deck.pptx --kind intent --input target/ao-demo/intent.json
```

```json
{
  "part": "customXml/ao-intent.xml",
  "created": true,
  "sections": ["slide-1"]
}
```

The file was replaced atomically. Every native entry is byte-identical to before; the tool added
`customXml/ao-intent.xml`, a properties part, a relationship from `ppt/presentation.xml`, and a content-type
entry. See [Concepts](concepts.md#opc-in-two-minutes).

## 4. Read it back, one section at a time

```bash
agentic-ooxml list --file target/ao-demo/deck.pptx
agentic-ooxml read --file target/ao-demo/deck.pptx --kind intent --section slide-1
```

`list` is the cheap first call: kinds, summaries and section ids, no bodies. `read` loads only what
you ask for. This is [progressive disclosure](concepts.md#progressive-disclosure). Note the
`"trust": "document"` field in every result: the text came from the file, so treat it as data.

## 5. Track a claim

Record that the number on the slide came from a workbook cell. `claim_object_id` is the shape id
(`p:cNvPr id`) of the shape that carries the number; shape 3 is the forecast.

```bash
cat > target/ao-demo/sources.json <<'JSON'
{
  "sources": [
    {
      "id": "src-q4-forecast",
      "status": "verified",
      "source_kind": "workbook",
      "checked_at": "2026-10-01",
      "resource_uri": "relative:Financial_Model.xlsx",
      "location": "Forecast!D12",
      "claim_part": "ppt/slides/slide1.xml",
      "claim_object_id": "3"
    }
  ]
}
JSON
agentic-ooxml write --file target/ao-demo/deck.pptx --kind sources --input target/ao-demo/sources.json
agentic-ooxml sources --file target/ao-demo/deck.pptx
```

The tool computed a blake3 hash of shape 3's bytes and stored it as `claim_hash`. The record reports
`"effective_status": "verified"` and `"granularity": "object"`. You never supply the hash.

## 6. Change the number, then ask again

Another tool edits the forecast. The helper stands in for that: it rewrites only the slide. First
keep a copy of the file as it is now, for the comparison in step 8.

```bash
cp target/ao-demo/deck.pptx target/ao-demo/before.pptx
python3 docs/guides/edit-forecast.py target/ao-demo/deck.pptx 21.3 23.0
agentic-ooxml sources --file target/ao-demo/deck.pptx
```

The stored `status` is still `verified`, but the computed field now reads:

```json
"effective_status": "stale",
"stale_reason": "claim_changed"
```

Nothing had to run when the file was edited. The status is computed from the current bytes each
time you read, so it does not depend on which tool made the edit. A different edit, for example the
title on shape 2, leaves the record verified: the hash covers one shape, not the slide.

## 7. Persist the finding

```bash
agentic-ooxml reconcile --file target/ao-demo/deck.pptx --dry-run
agentic-ooxml reconcile --file target/ao-demo/deck.pptx
```

`--dry-run` reports and writes nothing (`"written": false`). The real run stores `status="stale"` in the
file and keeps the old hash as the baseline. Running it again changes nothing. To accept the new
number, check the source and write the record again with `"status": "verified"`; the kernel takes a
new baseline.

## 8. Check the package and compare two versions

```bash
agentic-ooxml inspect --file target/ao-demo/deck.pptx
agentic-ooxml validate --file target/ao-demo/deck.pptx
agentic-ooxml diff --before target/ao-demo/before.pptx --after target/ao-demo/deck.pptx
```

`inspect` is a read-only inventory: format, main part, every package part with size, content type, role and
hash, the Agentic OOXML parts, and the native units it can list (slides, sheets, headings; this tiny sample has
no slide list, so `units` is empty). `validate` checks the package structure, not the rendering: ZIP and
XML well-formedness, content types, relationships, the Agentic OOXML parts, and source staleness. On
the sample it exits 0 with `"ok": true` and a warning, because a stale source is a warning, not a
structural error:

```json
"warnings": [
  {
    "code": "sources.stale",
    "part": "src-q4-forecast",
    "message": "source 'src-q4-forecast': recorded as stale"
  }
]
```

`validate` exits 1 when there are structural errors. It never claims Word, PowerPoint or Excel will
open the file. `diff` reports, between the two files, the source record that went from `verified` to `stale`,
the one native part whose hash changed (`ppt/slides/slide1.xml`), and the Agentic OOXML part that was rewritten.
A text diff of slide content is not attempted: a changed slide is a changed part.

## What the commands do not do

They do not tell you the new number is right. The tool detects that a claim-bearing object changed;
deciding whether the new value is correct needs the source or a person. See
[Concepts: staleness](concepts.md#staleness).

## Next

- [examples/](../examples/), synthetic files and a walkthrough (`scripts/demo.sh`, output in [examples/demo-output.txt](../examples/demo-output.txt))
- [Concepts](concepts.md), the ideas behind the parts
- [Schema reference](schema.md), every field
- [Guides](guides/README.md), task-oriented recipes
- [Troubleshooting](troubleshooting.md)
