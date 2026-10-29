# Troubleshooting

Messages below are copied from the code or from running the CLI.

## `Error: open FILE` / `No such file or directory`

The path is wrong, or relative to a different directory. The CLI does not search.

## `list` returns `"parts": []`

The file has no Agentic OOXML parts that are reachable from the main part by a `customXml`
relationship in the `ao` namespace. Either none were ever written, or a save in another editor
removed them (see [compatibility](compatibility.md)), or a part is present but unlinked. An unlinked
part is not discovered by design.

## `Error: no 'design' knowledge part in FILE`

You asked to `read` a kind the file does not have. Run `list` first. The same shape of message appears for `agents`
and `intent`.

## `Error: no v0.2 <ao:doc> part in FILE`

`read --kind doc` (and `units`, `spec`) only read v0.2 context. A file written by this tool has v0.1
parts; read them with `--kind agents`, `design`, `intent`.

## `... carries Agentic OOXML v0.2 context ... one writer per file`

The file has a v0.2 `ao:doc`. Writes here are refused so the two formats do not split the file. Use the
Agentic Office servers' writer for that file, or work on a copy without the v0.2 part.

## `the existing 'K' part ... is malformed ... pass --replace to overwrite it`

The part's XML cannot be parsed. Look at it (`unzip -p FILE customXml/ao-K.xml`). If it is junk, write
again with `--replace`.

## `... has schemaVersion X, newer than 0.1; merging would drop ...`

A newer tool wrote the part. Merging with this version would lose fields it does not know. Upgrade
the tool, or pass `--replace` knowing that the extra fields will be lost.

## `AO part contains <!DOCTYPE, which is not allowed`

The part contains a document type declaration. This is rejected on purpose (entity expansion). Remove it.

## `package has N entries (cap 10000)` or a part over the size cap

Package or part is bigger than the safety caps. See [schema: limits](schema.md#limits-that-apply-to-every-part).

## `... is read-only`

The file's permissions do not allow replacement, so `write` and `reconcile` refuse. Fix the permissions or work on a copy.

## `effective_status` is `stale` and I did not change that number

The claim hash covers either one shape (`granularity: object`) or the whole part (`granularity: part`).
For a part-level claim, any edit anywhere in the part makes it stale. Check `granularity` in the
`sources` output. Re-save in another editor can also rewrite XML for shapes you did not touch, which changes
the bytes the hash covers.

## `stale_reason: no_baseline`

The record is `verified` but has no stored hash, for example because it was written by another tool.
Re-verify it with this tool to take a baseline.

## `stale_reason: claim_missing`

The claim part or the shape id no longer exists. A shape id can change when a slide is copied or
re-saved. Find the object, update `claim_object_id`, and verify again.

## `reconcile` says `"written": false` and I expected a change

`written` is false for `--dry-run`, and when nothing needs to change (every record is already
consistent), which makes the command safe to run repeatedly.

## `cargo` fails to build

Use a recent stable Rust toolchain. If a build fails on a fresh checkout, include the full output in a bug report.
