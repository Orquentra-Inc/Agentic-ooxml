#!/usr/bin/env bash
# Before/after walkthrough on the synthetic examples. Everything printed is real output from the
# binaries in this repo; two outputs are filtered for length and say so. Works on a temp copy:
# the committed files under examples/ are never modified. No Office needed, and nothing is opened
# in Office either.
#   scripts/demo.sh                      # print to the terminal
#   scripts/demo.sh | tee examples/demo-output.txt
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
cargo build -q -p agentic-ooxml-cli -p agentic-ooxml-examples

BIN="$PWD/target/debug"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cp examples/board-deck.pptx examples/financial-model.xlsx examples/report.docx "$WORK/"
cd "$WORK"
# `before/` keeps the original deck with the original workbook beside it, so diff can evaluate the
# external claim against the workbook as it was.
mkdir before
cp board-deck.pptx financial-model.xlsx before/

show() { printf '$'; printf ' %q' "$@"; printf '\n'; }
ao() { show agentic-ooxml "$@"; "$BIN/agentic-ooxml" "$@"; }
mk() { show make-examples "$@"; "$BIN/make-examples" "$@"; }
# Run a command that is expected to exit non-zero; show the exit code.
expect_fail() { set +e; "$@"; local rc=$?; set -e; echo "(exit code $rc)"; }
step() { printf '\n== %s ==\n' "$*"; }
py() { python3 -I -c "$1"; }

step "1. Inspect the deck (filtered: units, AO parts, and the part inventory by role)"
echo "\$ agentic-ooxml inspect --file board-deck.pptx   # output filtered with python3 for length"
"$BIN/agentic-ooxml" inspect --file board-deck.pptx | py '
import json, sys
j = json.load(sys.stdin)
print("format:", j["format"], "| main part:", j["main_part"], "| entries:", j["entry_count"])
for u in j["units"]:
    print("  slide", u["n"], "id", u["id"], "|", u.get("text"))
for p in j["ao_parts"]:
    print("  AO part:", p["kind"], p["part"], "| sections:", p.get("sections"), "| sources:", p.get("source_count"))
print("  parts by role:", {r: sum(1 for p in j["parts"] if p["role"] == r) for r in sorted({p["role"] for p in j["parts"]})})
'

step "2. The embedded knowledge: read the intent, list the sources"
ao read --file board-deck.pptx --kind intent --section purpose
ao sources --file board-deck.pptx

step "3. Validate (clean) and check the cross-document links (deck vs workbook)"
ao validate --file board-deck.pptx
mk check-links --file board-deck.pptx

step "4. Edit the WORKBOOK: Q3 changes from 18.6 to 19.1 (demo helper; the library does not edit cells)"
mk set-text --file financial-model.xlsx --part xl/worksheets/sheet1.xml --find "<v>18.6</v>" --replace "<v>19.1</v>"
mk set-text --file financial-model.xlsx --part xl/worksheets/sheet2.xml --find "<v>18.6</v>" --replace "<v>19.1</v>"
echo "the deck was not touched: slide 2 still says, read straight from the package with Python zipfile:"
py 'import re, zipfile
x = zipfile.ZipFile("board-deck.pptx").read("ppt/slides/slide2.xml").decode()
print("  ", re.search(r"Q3 revenue[^<]*", x).group(0))'

step "5. The deck's Q3 source goes stale because the workbook cell moved (the annual claim is untouched)"
ao sources --file board-deck.pptx
echo "validate still exits 0: a stale source is a warning, not corruption"
ao validate --file board-deck.pptx
echo "the cross-document helper agrees: the slide shows 18.6, the workbook says 19.1:"
show make-examples check-links --file board-deck.pptx
expect_fail "$BIN/make-examples" check-links --file board-deck.pptx

step "6. The agent updates slide 2 (demo helper); the claim stays stale until it is re-verified"
mk set-text --file board-deck.pptx --part ppt/slides/slide2.xml --find "EUR 18.6M" --replace "EUR 19.1M"
ao sources --file board-deck.pptx --part /ppt/slides/slide2.xml --object 5
echo "the slide now shows the workbook value, but the record has not been re-verified, so the helper still exits 1:"
show make-examples check-links --file board-deck.pptx
expect_fail "$BIN/make-examples" check-links --file board-deck.pptx

step "7. Diff before and after (native part hashes, source status, the external value)"
ao diff --before before/board-deck.pptx --after board-deck.pptx

step "8. Reconcile: dry run, then persist the stale status"
ao reconcile --file board-deck.pptx --dry-run
ao reconcile --file board-deck.pptx
echo "after reconcile the stored status itself is stale, and validate says so:"
ao validate --file board-deck.pptx

step "9. Re-verify: the sources write path re-reads Summary!B1 and takes a new baseline"
cat > verify.json <<'JSON'
{"sources": [{"id": "q3-kpi", "status": "verified", "checked_at": "2026-10-29"}]}
JSON
echo "\$ cat verify.json"
cat verify.json
ao write --file board-deck.pptx --kind sources --input verify.json
ao sources --file board-deck.pptx --part /ppt/slides/slide2.xml --object 5

step "10. The annual figure: workbook 75.5, then slide 3, then re-verify from the generated patch"
mk set-text --file financial-model.xlsx --part xl/worksheets/sheet1.xml --find "<v>75</v>" --replace "<v>75.5</v>"
mk set-text --file financial-model.xlsx --part xl/worksheets/sheet2.xml --find "<f>SUM(B2:B5)</f><v>75</v>" --replace "<f>SUM(B2:B5)</f><v>75.5</v>"
mk set-text --file board-deck.pptx --part ppt/slides/slide3.xml --find "EUR 75.0M" --replace "EUR 75.5M"
"$BIN/make-examples" link-patch --file board-deck.pptx > relink.json
echo "\$ make-examples link-patch --file board-deck.pptx > relink.json   # contents of relink.json:"
cat relink.json
ao write --file board-deck.pptx --kind sources --input relink.json

step "11. After: sources verified, links consistent, package valid"
ao sources --file board-deck.pptx
mk check-links --file board-deck.pptx
ao validate --file board-deck.pptx
echo
echo "Note: the workbook cached values were edited as text; a spreadsheet application would recalculate"
echo "them on open (the workbook sets fullCalcOnLoad), and this library never recalculates formulas: it"
echo "compares the cached cell value it can read. This demo does not open any file in Office."
