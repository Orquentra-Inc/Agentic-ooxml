# Carry intent and editing policy in a file

Use the `intent`, `design` and `agents` parts when a document has context that the next agent or
person should not have to rediscover. The three kinds share one shape: a short `summary` plus
named `sections`. Section bodies are free Markdown. Today none of them is typed or enforced; see
the [schema reference](../schema.md#aoknowledge-agents-design-intent).

```bash
mkdir -p target/ao-guide
python3 docs/guides/make-minimal-deck.py target/ao-guide/deck.pptx
```

## Create a part

```bash
cat > target/ao-guide/design.json <<'JSON'
{
  "summary": "Visual conventions for the board deck",
  "sections": [
    { "id": "charts", "text": "One accent colour. Label forecasts. Axes start at zero." }
  ]
}
JSON
agentic-ooxml write --file target/ao-guide/deck.pptx --kind design --input target/ao-guide/design.json
```

`format` defaults to `text/markdown` when you omit it.

## Merge, do not overwrite

A second write to an existing part is a merge: `summary` is replaced if you give one, sections are
upserted by `id` (existing order kept, new ids appended).

```bash
cat > target/ao-guide/design2.json <<'JSON'
{ "sections": [ { "id": "tables", "text": "Numbers right-aligned, thousands separated." } ] }
JSON
agentic-ooxml write --file target/ao-guide/deck.pptx --kind design --input target/ao-guide/design2.json
agentic-ooxml list --file target/ao-guide/deck.pptx
```

`sections` now lists `charts` and `tables`.

## Delete one section

```bash
cat > target/ao-guide/del.json <<'JSON'
{ "sections": [ { "id": "tables", "delete": true } ] }
JSON
agentic-ooxml write --file target/ao-guide/deck.pptx --kind design --input target/ao-guide/del.json
```

## Replace the whole part

`--replace` discards what is there and writes only your input. It is also the way to recover a part
whose XML is corrupt.

```bash
agentic-ooxml write --file target/ao-guide/deck.pptx --kind design --input target/ao-guide/design.json --replace
```

## What is refused

- A part whose `schemaVersion` is newer than this tool understands is not merged, because the merge would
  drop fields it does not know. Pass `--replace` to overwrite it deliberately.
- A malformed existing part is not merged; the error says so and suggests `--replace`.
- A file that already carries v0.2 context (`ao:doc`) refuses `write`: one writer per file. See
  [read v0.2 context](read-v02-context.md).
- A part larger than 2 MiB after the write is refused.

## What not to put in these parts

Anything you would not want extracted. The parts sit in an unencrypted ZIP; anyone with the file can
read them. Do not store secrets, and be careful with client names and internal paths (see
[security](../security.md#privacy)).
