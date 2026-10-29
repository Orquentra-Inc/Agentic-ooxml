# Compatibility

This page separates what has been measured from what has not. Where a cell says "not measured", no
test has been run, and nothing here claims the outcome either way.

## Formats

| format | list and read | write `agents`, `design`, `intent`, `sources` | v0.2 read | claim hash granularity |
|---|---|---|---|---|
| `.docx` | tested | tested | tested | part |
| `.pptx` | tested | tested | tested | shape (by `p:cNvPr` id) or part |
| `.xlsx` | tested | tested | handled in code (sheet names as unit refs), no dedicated test | part |

As the **target** of an `external_claim` (a value in another file): `.xlsx` cached cell values (numbers, shared strings, inline strings, formula string results, booleans), and `.docx`/`.pptx` part or slide-shape text, all tested with minimal hand-built packages. Not tested against files saved by Excel, Word or PowerPoint; formulas are not recalculated.

"Tested" means a unit test builds a minimal package of that format in memory and exercises the
function. The fixtures are minimal packages, not documents saved by Office. The three files in `examples/` pass `agentic-ooxml validate` and an independent standard-library check (`scripts/check-examples.py`); neither shows that Office or LibreOffice opens them.

## Consumers: do the parts survive a save?

The proposal's compatibility matrix is open work. Status of each row:

| consumer | opens the file | keeps the parts on save | notes |
|---|---|---|---|
| This library (all of its own writes) | yes | yes: every non-target entry is raw-copied | tested |
| Agentic Office pptx editor (import then save, source monorepo), 2026-10-19 | measured by that project | presentation-level parts kept 4 of 4 in all three encodings, with and without an edit; a slide-level part was kept but lost its relationship | the numbers are below; raw results in [docs/evidence](evidence/round-trip-2026-10-19/README.md); not re-runnable from this repository |
| OfficeCLI write paths (add and remove slide, set, template merge) and a package merge | measured by that project | 4 of 4 present and referenced on every path | same evidence |
| Microsoft PowerPoint, Word, Excel | **not measured** | **not measured** | the biggest open question |
| LibreOffice | **not measured** | **not measured** | may rewrite package structure |
| ONLYOFFICE | **not measured** | **not measured** | |
| Google import and export | **not measured** | **not measured** | conversion may discard metadata |

### The measurements that exist

All of the numbers on this page come from one run on 2026-10-19, inside the Agentic Office
monorepo, through its own `.pptx` editor and OfficeCLI. **They apply to those tools and to nothing
else.** Method, raw output and scripts: [docs/evidence/round-trip-2026-10-19](evidence/round-trip-2026-10-19/README.md).

| encoding of the four parts, through the editor's import and save | no edit | slides reordered |
|---|---|---|
| PowerPoint-standard `customXml/itemN.xml` with props part | 4/4 kept | 4/4 kept |
| friendly names (`customXml/agents.xml`), standard relationship | 4/4 kept | 4/4 kept |
| `customXml/ao/*.xml`, custom relationship type | 4/4 kept | 4/4 kept |
| slide-level (related from slide 2) | part kept, relationship lost | part kept, relationship lost |
| orphan control (no relationship) | present, unreferenced (as built) | same |

Six other writers (OfficeCLI add slide, remove slide, set, template merge; a package merge; a plain
rewrite) each left all four presentation-level parts present and referenced (4/4).

A later change (2026-10-19) made that editor re-attach per-slide relationships; its regression test
is `ao-deck-roundtrip.test.ts` (5 tests) in the monorepo. We have **not re-run it**, so the
slide-level row above is the pre-fix result.

**Still unmeasured: Microsoft PowerPoint, Word and Excel, LibreOffice, ONLYOFFICE, and Google
import and export.** Nothing above predicts what they do. If you can run one, please
[report the result](contributing.md#1-report-a-round-trip).

Custom XML parts are a recognised OPC mechanism, and Microsoft's Open XML SDK exposes APIs to add
them. That justifies the approach. It is not evidence that a given editor preserves them.

Practical consequence: assume a save in an editor you have not tested may remove the parts. If
that happens, the Office document remains the document it was; what is lost is the extra context.
Re-run `agentic-ooxml list --file F` after a save in another tool, and treat an empty result as "the context
was lost", not "there was none".

## Tool and platform

| item | status |
|---|---|
| Rust | built and tested with rustc 1.97 on macOS (Apple silicon). CI runs ubuntu and macOS. |
| Windows | not tested |
| Dependencies | `anyhow`, `blake3`, `quick-xml`, `serde`, `serde_json`, `uuid`, `zip`; licences not yet audited |
| File names | UTF-8 entry names; parts must be UTF-8 |

## Reporting results

If you run a round trip in an editor, a report with the editor, version, the command you ran, and
the output of `agentic-ooxml list` before and after is the most useful thing you can contribute. The exact
method is in [Contributing](contributing.md#1-report-a-round-trip). Use the bug report issue template.
