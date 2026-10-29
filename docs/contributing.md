# Contributing: how to help

This is the long version of [CONTRIBUTING.md](../CONTRIBUTING.md). The same guide is on the
[project site](https://orquentra-inc.github.io/Agentic-ooxml/contribute/). You do not need to be an
OOXML expert, and you do not need Rust for most of the useful things below.

The one rule: **every public claim must match code that runs.** If a doc says the tool does
something, there is a test or a command that shows it. If it is not built, say "proposed".

## Ways to help, ranked by impact

### 1. Report a round trip

This is the most valuable thing you can do. The central open question is whether Office
applications keep the Agentic OOXML parts when they save a file. The only measurements that exist
are from one editor's pipeline (see [compatibility](compatibility.md) and the
[evidence](evidence/round-trip-2026-10-19/README.md)). **PowerPoint, Word, Excel, LibreOffice,
ONLYOFFICE and Google are unmeasured.** A positive result, and especially a negative one, is
worth more than any code change.

You need the `agentic-ooxml` binary ([install](../README.md#install)) and the application you want to test.

```bash
# 1. Work on a copy of a synthetic example. Never use a real document.
mkdir -p rt && cp examples/board-deck.pptx rt/before.pptx     # or financial-model.xlsx, report.docx

# 2. Record the starting state.
agentic-ooxml list     --file rt/before.pptx > rt/list-before.json
agentic-ooxml validate --file rt/before.pptx

# 3. In the application: open rt/before.pptx, change one word, and Save As
#    rt/after.pptx in the SAME format. Then repeat with a second copy where you
#    change nothing and just save (rt/after-noedit.pptx).

# 4. Record the result.
agentic-ooxml list     --file rt/after.pptx > rt/list-after.json
agentic-ooxml validate --file rt/after.pptx
agentic-ooxml diff --before rt/before.pptx --after rt/after.pptx
```

What to look at: `parts` in the two `list` outputs (an empty list after the save means the context was
lost, not that there was none), and `ao_parts` and `native_parts` in the `diff`. Then open an
[issue with the "Round-trip report" template](https://github.com/Orquentra-Inc/Agentic-ooxml/issues/new?template=round_trip.yml)
and include:

- the application and its exact version, and your operating system;
- which file you started from, and exactly what you did in the application;
- the two `list` outputs, and the `diff` and `validate` output for the saved file;
- anything the application said or changed (a repair prompt, a format warning).

For Google, say how you got the file in and out (for example "uploaded, opened as Slides, downloaded
as .pptx"). For the file formats `.docx` and `.xlsx`, do the same with the matching example.

### 2. Add example documents

`examples/` holds synthetic files only. A new example that exercises something the current three do not
(a `.docx` with comments, an `.xlsx` with named ranges, a deck with a chart part) helps everyone test
against it. Every number is invented, with no real clients or figures. The generator is
`examples/generator`; the generated files must be reproducible. Open an issue first so the
example is not duplicated.

### 3. Propose a schema change

See [How to propose a schema change](#how-to-propose-a-schema-change). Typed design tokens, a `validation` part and an agent
registry are listed as proposed in the [roadmap](../ROADMAP.md) and need a worked proposal before code.

### 4. Integrations for other agents

The integrations for Claude Code, Codex, Cursor and Pi exist as manifests and config, and are **not
verified end to end**. The status table is in [integrations/README.md](../integrations/README.md).
Running one against a real agent and reporting what happened (or fixing it) is high value. A new
adapter for another agent is welcome: use the
[Integration issue template](https://github.com/Orquentra-Inc/Agentic-ooxml/issues/new?template=integration.yml)
and run `integrations/verify.sh` (see below).

### 5. Documentation fixes

Run the commands in a page; if the output differs from what is printed, or a claim goes further than the
code does, fix it or open a
[documentation issue](https://github.com/Orquentra-Inc/Agentic-ooxml/issues/new?template=docs.yml).

### 6. Concrete first tasks

These come from the [roadmap](../ROADMAP.md) and the known gaps in [compatibility](compatibility.md).
They are not yet filed as issues: comment on a new issue that you are taking one so that work is not
duplicated.

| task | needs | what "done" looks like |
|---|---|---|
| Run `cargo test --workspace` on Windows and report | Windows, Rust | an issue with the result; then a fix for anything that fails (Windows is currently untested) |
| Audit dependency licences | Rust | a table in [compatibility](compatibility.md) replacing "licences not yet audited" (for example from `cargo license` or `cargo deny`) |
| Add a dedicated test for `.xlsx` v0.2 context reads | Rust | a unit test beside the existing `.docx` and `.pptx` ones |
| Round-trip the three examples in LibreOffice, then in another app | an app | [round-trip reports](#1-report-a-round-trip), and open each example and report whether it opens cleanly |
| Verify the Codex, Cursor or Pi integration against the real agent | that agent | a status-table update in [integrations/README.md](../integrations/README.md) with the output |
| Write the Streamable HTTP transport design for the MCP server | Rust | a design note (it is planned, not built) |
| Draft a worked proposal for typed design tokens | schema thinking | a [schema proposal](https://github.com/Orquentra-Inc/Agentic-ooxml/issues/new?template=schema_proposal.yml) |
| Propose a stable locator for a claim under copy, move and reorder (research question RQ3) | OOXML knowledge | a design note with a test case that breaks a shape id |
| Start a conformance corpus: small files with the expected `validate` and `list` output | Python or Rust | files plus a script, with every expected output produced by the real command |

## Set up

You need a stable Rust toolchain. Python 3 is used for the doc helpers and Node 22 for the website.

```bash
git clone https://github.com/Orquentra-Inc/Agentic-ooxml
cd Agentic-ooxml
cargo build --workspace
cargo test --workspace
```

Before you open a pull request, run what CI runs:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 docs/check-links.py          # relative links and heading anchors in every .md
python3 scripts/check-examples.py    # independent check of the example files
integrations/verify.sh               # integration checks; SKIP means "not tested", not "passed"
```

Regenerate the examples (only when you change the generator; the output is byte-reproducible, so
`git status examples/` should show no change if you did not mean to change them) and the committed
demo output:

```bash
cargo run -q -p agentic-ooxml-examples --bin make-examples -- generate    # writes examples/*.pptx, .xlsx, .docx
scripts/demo.sh | tee examples/demo-output.txt                            # the before/after walkthrough
```

The website, in `website/`:

```bash
cd website
npm ci
npm run dev        # local dev server
npm run build      # type check, build, sitemap
npm run lint
npm run a11y       # axe-core on every page (installs a Playwright browser on first use)
npm run shots      # screenshots at 1440, 768 and 375 px into website/shots
```

## The pull request flow

1. For anything larger than a typo, open an issue first, so we can agree on the direction.
2. Fork, create a branch, make a small and focused change.
3. A change in behaviour needs a test. Tests build their packages in memory; do not commit binary
   Office files unless they are synthetic and small, and say where they came from.
4. Run the checks above. Update [CHANGELOG.md](../CHANGELOG.md) under Unreleased for user-visible changes.
5. Open a pull request using the template: what changed and why, how you checked it, and whether
   it changes how embedded document text, ZIP entries or paths are handled.
6. By contributing you agree that your work is licensed under the [MIT licence](../LICENSE).

## What to expect in review

- CI must pass: format, clippy with warnings as errors, tests (Linux and macOS) and the docs link check.
- Reviewers check claims first: does every statement in the change match code that runs? Expect
  requests to soften or remove a claim, or to add the command that shows it.
- Anything that makes the library act on embedded document text needs a design discussion before code.
- Small changes are usually answered within a few days. This is a small project: if you have heard
  nothing in a week, a polite ping on the pull request is fine.
- Feedback is about the change, not the person. Disagreement is welcome; see the
  [Code of Conduct](../CODE_OF_CONDUCT.md).

## How to propose a schema change

Open an issue with the
[schema proposal template](https://github.com/Orquentra-Inc/Agentic-ooxml/issues/new?template=schema_proposal.yml)
before writing code. State:

1. the part, element or field;
2. why native OOXML cannot already say it (the native structure is the source of truth and is not copied);
3. how an agent reads it, and what an older reader does when it does not know it (new fields must
   be optional and ignorable);
4. what happens when its text is hostile (embedded text is data and never authority);
5. the evidence it is needed.

Anything accepted is marked **proposed** in the [schema reference](schema.md) and the
[roadmap](../ROADMAP.md) until code and tests exist.

## Where to ask questions

- Questions and ideas: [GitHub Discussions](https://github.com/Orquentra-Inc/Agentic-ooxml/discussions)
  if they are enabled, otherwise a [GitHub issue](https://github.com/Orquentra-Inc/Agentic-ooxml/issues).
- Bugs, round-trip reports, documentation problems: issues, using the templates.
- **Conduct concerns and security reports only:** agenticoffcie70@gmail.com. Security problems go
  through [SECURITY.md](../SECURITY.md), never a public issue. This address is not for general support.

Participation is covered by the [Code of Conduct](../CODE_OF_CONDUCT.md).
