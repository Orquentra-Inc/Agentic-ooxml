# Round-trip evidence, 2026-10-19

Raw results and scripts for the only part-retention measurements that exist for Agentic OOXML. They
are published so that you can read exactly what was measured and challenge it.

## What this is, and what it is not

- It ran **inside the Agentic Office monorepo**, through **that project's own `.pptx` editor** (its
  import and save pipeline) and through **OfficeCLI**. The library in this repository was not
  involved.
- It measured **one format (`.pptx`) in one editor**. It says nothing about Microsoft PowerPoint,
  Word or Excel, LibreOffice, ONLYOFFICE or Google. **Those remain unmeasured.** Do not read these
  numbers as applying to them.
- Date of the run: 2026-10-19. The scripts are copied unchanged, so they contain absolute paths from
  the machine they ran on and import code from the monorepo. **You cannot re-run them from this
  repository.** They document the method; they are not a reproducible benchmark here.

## Method

A real `.pptx` was built with OfficeCLI and four Agentic OOXML parts (`agents`, `design`, `intent`,
`sources`) were added in five different encodings. Each deck was run through the editor's import
followed by its save, twice: unchanged ("no edit") and after reordering the slides ("reorder
slides"). A part counts as **kept** only if it was found, byte-identical, **still related** from the
same kind of source with the same relationship type, with its content type intact, and OfficeCLI's
validator reported no new errors.

## Results: five encodings through the editor's import and save

| encoding | no edit | reorder slides | outcome |
|---|---|---|---|
| V1: PowerPoint-standard `customXml/itemN.xml`, props part, standard `customXml` relationship | 4/4 present, referenced | 4/4 present, referenced | kept |
| V2: friendly names (`customXml/agents.xml`...), standard relationship, no props part | 4/4 present, referenced | 4/4 present, referenced | kept |
| V3: `customXml/ao/*.xml`, a custom relationship type, own content type | 4/4 present, referenced | 4/4 present, referenced | kept |
| V4: slide-level (one part related from slide 2) | 1/1 present, **not referenced** | 1/1 present, **not referenced** | part kept, **relationship lost** |
| V5: orphan control (no relationship from anything) | 4/4 present, none referenced | 4/4 present, none referenced | as built; a control, not a result |

## Results: other write paths

Starting from a deck carrying the four parts at presentation level (V1 encoding):

| write path | present | referenced |
|---|---|---|
| input, before any write (reference) | 4/4 | 4/4 |
| OfficeCLI add slide | 4/4 | 4/4 |
| OfficeCLI remove slide | 4/4 | 4/4 |
| OfficeCLI set (slide background) | 4/4 | 4/4 |
| OfficeCLI merge (template fill) | 4/4 | 4/4 |
| `svg_to_deck` package merge (JSZip) | 4/4 | 4/4 |
| plain JSZip rewrite (control) | 4/4 | 4/4 |

That is the input plus six writers; all six kept all four parts present and referenced.

## What it supports

- Presentation-level parts survived that editor's save and every write path listed.
- Slide-level parts did not: the editor dropped the slide's relationship, leaving the part orphaned.
  This is why this project links its parts from the main part only.

## A later fix, not re-run here

On 2026-10-19, after these runs, the editor was changed to re-attach per-slide relationships. Its
regression test is `apps/editors/pptx/extension/webview/tests/ao-deck-roundtrip.test.ts` in the
Agentic Office monorepo (5 tests). **We have not re-run it**, and the V4 numbers above are the
pre-fix measurement.

## Files

| file | what |
|---|---|
| `results-editor-roundtrip.txt` | raw output: V1 to V5, both runs |
| `results-write-paths.txt` | raw output: the other write paths |
| `ao-parts.test.ts` | the test that builds the five encodings and checks retention |
| `run.mjs` | bundles the test and runs it against the editor's import and save |
| `paths.mjs` | the OfficeCLI, merge and `svg_to_deck` write paths |

## Help measure the rest

The unmeasured consumers are the ones that matter most. If you can open and save a file in
PowerPoint, Word, Excel, LibreOffice, ONLYOFFICE or Google, see
[how to report a round trip](../../contributing.md#1-report-a-round-trip).
