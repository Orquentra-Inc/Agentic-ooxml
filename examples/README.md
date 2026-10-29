# Examples

Three synthetic files that carry real native content and embedded Agentic OOXML parts. Every number
is invented (EUR M): Q1 17.2, Q2 17.9, Q3 18.6, Q4 forecast 21.3, annual 75.0. Nothing here is real
financial data.

| File | Native content | Embedded AO parts |
|---|---|---|
| `board-deck.pptx` | 4 slides: title, revenue by quarter (text plus bars scaled to the values), full-year outlook, risks | `agents`, `design`, `intent`, `sources` |
| `financial-model.xlsx` | sheets `Summary` and `Quarterly` with numbers and formulas | `agents`, `design`, `intent`, `sources` |
| `report.docx` | a title, Heading 1/2 sections, a table of the quarterly figures | `agents`, `design`, `intent`, `sources` |

`demo-output.txt` is the captured output of `scripts/demo.sh`, the full walkthrough below.

## What is and is not verified

* The files are built by `examples/generator` (`make-examples generate`), byte-reproducibly (fixed
  timestamps and entry order; a test generates twice and compares).
* Each file round-trips through the `agentic-ooxml` library (`validate`, `list`, `read`, `write`) and
  passes `python3 -I scripts/check-examples.py`, an independent standard-library check (zip CRC, XML
  well-formedness, content types, relationship targets, AO parts present).
* **They have not been opened in Microsoft Office, LibreOffice or any other application.** The
  OOXML is small and hand-written. Do not read "valid" here as "Office opens it".

## Cross-document example

`board-deck.pptx` quotes two figures from `financial-model.xlsx`. Its `sources` part records that:

* `q3-kpi`: the Q3 text box on slide 2 (shape id 5) quotes `Summary!B1` of `financial-model.xlsx`. It carries an `external_claim`, so the library itself reads that workbook cell.
* `annual`: the headline on slide 3 (shape id 3) quotes `Summary!B2`.
* `q4-forecast`: a manual record (no hash, never reported stale by the library).

`report.docx` has an `annual-total` record claiming its whole body part against `Summary!B2`.

What the library does with this: it hashes the claimed slide shape when a record is written as
`verified`, and reports `stale` when it later changes or disappears. For `q3-kpi` it also opens
`financial-model.xlsx` (read-only, same directory), reads the cached value of `Summary!B1`, and reports
`stale` with `external_claim_changed` when that value no longer matches the baseline, so editing the
WORKBOOK is enough to flag the deck. It compares cached values only and never recalculates formulas.
`annual` and `annual-total` are still in-package claims; the example helper
`make-examples check-links` reads the cell and compares it with what the claimed shape shows.
It never edits the other file: updating the slide and re-verifying is a step you (or an agent) run.

## Walkthrough

Build once, then work on a copy so the committed examples stay untouched.

```sh
cd <repo root>
source $HOME/.cargo/env
cargo build -q -p agentic-ooxml-cli -p agentic-ooxml-examples
AO=$PWD/target/debug/agentic-ooxml
MK=$PWD/target/debug/make-examples
mkdir -p /tmp/ao-try && cp examples/board-deck.pptx examples/financial-model.xlsx examples/report.docx /tmp/ao-try/
cd /tmp/ao-try
```

**Inspect**

```sh
$AO inspect --file board-deck.pptx          # format, every part, AO parts, the four slides
$AO list    --file board-deck.pptx          # the four AO parts
$AO read    --file board-deck.pptx --kind intent --section purpose
$AO sources --file board-deck.pptx          # three records; q3-kpi and annual are verified
```

**Validate (clean)**

```sh
$AO validate --file board-deck.pptx         # ok: true, no warnings; exit code 0
$MK check-links --file board-deck.pptx      # both links "consistent"
```

**Edit the workbook: change Q3.** The library does not edit cells; `make-examples set-text` is a
demo helper that replaces exact text inside one package part.

```sh
mkdir before && cp board-deck.pptx financial-model.xlsx before/
$MK set-text --file financial-model.xlsx --part xl/worksheets/sheet1.xml \
    --find "<v>18.6</v>" --replace "<v>19.1</v>"
$MK set-text --file financial-model.xlsx --part xl/worksheets/sheet2.xml \
    --find "<v>18.6</v>" --replace "<v>19.1</v>"
```

**See the deck's source go stale, though the deck was not touched**

```sh
$AO sources  --file board-deck.pptx   # q3-kpi: stale, stale_reason "external_claim_changed", external_check.current_value 19.1
$AO validate --file board-deck.pptx   # still exit 0: staleness is a warning (sources.stale)
$MK check-links --file board-deck.pptx   # exit 1: the slide says 18.6, the workbook says 19.1
```

**Update the slide, reconcile, re-verify**

```sh
$MK set-text --file board-deck.pptx --part ppt/slides/slide2.xml --find "EUR 18.6M" --replace "EUR 19.1M"
$AO diff --before before/board-deck.pptx --after board-deck.pptx   # q3-kpi verified -> stale, slide 2 changed
$AO reconcile --file board-deck.pptx --dry-run   # reports q3-kpi (external_claim_changed), writes nothing
$AO reconcile --file board-deck.pptx             # persists status="stale"
echo '{"sources":[{"id":"q3-kpi","status":"verified"}]}' > verify.json
$AO write --file board-deck.pptx --kind sources --input verify.json   # re-reads Summary!B1, new baseline
$AO sources --file board-deck.pptx               # q3-kpi verified again
```

`reconcile` only records that the claim moved; it never re-verifies. Re-verification is the `write`
above, after the slide was brought in line. (The annual total's cached value is not recalculated;
the workbook sets `fullCalcOnLoad`, so a spreadsheet application recalculates on open.
`scripts/demo.sh` also updates the annual figure and re-verifies it with `make-examples link-patch`.)

## Everything at once

```sh
scripts/demo.sh                     # prints the real output; works on a temp copy
```

## Regenerate the examples

```sh
cargo run -q -p agentic-ooxml-examples --bin make-examples -- generate --out examples
python3 -I scripts/check-examples.py
git diff --stat examples/           # empty: the output is reproducible
```
