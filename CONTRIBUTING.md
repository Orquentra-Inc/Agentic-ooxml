# Contributing

Thanks for looking. This is an early, small project. The most useful contributions are measurements
(does a given editor keep the parts?), bug reports with a file that reproduces, and corrections to
claims in the docs that the code does not support.

**Full guide:** [docs/contributing.md](docs/contributing.md), also on the
[project site](https://orquentra-inc.github.io/Agentic-ooxml/contribute/). It lists the ways to help by impact, with
the exact method for a round-trip report and concrete first tasks. This file is the short, canonical version.

## Ways to help, most useful first

1. **Round-trip reports** from PowerPoint, Word, Excel, LibreOffice, ONLYOFFICE or Google: does the editor keep the parts on
   save? None of these is measured. Method and issue template: [how to report a round trip](docs/contributing.md#1-report-a-round-trip).
2. New synthetic example documents.
3. Schema proposals (see below).
4. Integrations for other agents, and verifying the existing ones.
5. Documentation fixes.
6. The first tasks listed in the [guide](docs/contributing.md#6-concrete-first-tasks).

## The one rule

**Every public claim must match code that runs.** If a doc says the tool does something, there is a test
or a command that shows it. If it is not built, say "proposed". Do not add badges, benchmarks or
compatibility claims without evidence behind them.

## Set up

You need a stable Rust toolchain. Python 3 is used for the doc helpers and Node for the website.

```bash
git clone https://github.com/<you>/Agentic-ooxml     # your fork
cd Agentic-ooxml
cargo build --workspace
cargo test --workspace
```

Before you open a pull request, run the checks CI runs:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 docs/check-links.py
python3 scripts/check-examples.py
```

`integrations/verify.sh` checks the integrations (SKIP means not tested). If you changed the website, also run
`npm ci && npm run build && npm run lint && npm run a11y` inside `website/`.

## Layout

| path | what |
|---|---|
| `crates/agentic-ooxml` | the library: packages, parts, claims, reconcile |
| `crates/agentic-ooxml-cli` | the command line, a thin JSON wrapper over the library |
| `crates/agentic-ooxml-mcp` | the MCP server |
| `examples/` | synthetic example files and their generator (`make-examples`) |
| `scripts/` | `check-examples.py` and `demo.sh` for the examples |
| `integrations/` | agent integrations (in progress) |
| `website/` | the project site |
| `docs/` | documentation; `docs/internal` is working notes, not public docs |

## Changing code

- Keep the library free of repository or monorepo-specific assumptions.
- A change to behaviour needs a test. Tests build their packages in memory; do not commit binary
  Office files unless they are synthetic and small, and say where they came from.
- Examples and fixtures use synthetic data only. No real client names, no real figures.
- Embedded document text is untrusted. A change that makes the library act on it needs a design
  discussion first.
- Match the surrounding comment style: full sentences that explain why.

## Changing docs

- Run every command you quote, and paste what it printed.
- Mark anything not implemented as **proposed**.
- Run `python3 docs/check-links.py`.
- Update [CHANGELOG.md](CHANGELOG.md) under Unreleased for user-visible changes.

## Proposing a schema change

Open an issue with the schema proposal template before writing code. State the field, why OOXML
cannot already say it, how it is read by an agent, what happens when a reader does not know it, and
how it stays safe when its text is hostile. New fields must be optional and ignorable by old readers.

## Commits and pull requests

Small, focused changes. Describe what changed and why, and how you checked it. By contributing you agree
that your contribution is licensed under the [MIT licence](LICENSE).

## Questions

Ask in [GitHub Discussions](https://github.com/Orquentra-Inc/Agentic-ooxml/discussions) if they are enabled,
otherwise in an issue. The e-mail address agenticoffcie70@gmail.com is for conduct and security reports only.

## Conduct

Participation is covered by the [Code of Conduct](CODE_OF_CONDUCT.md). Security problems go through
[SECURITY.md](SECURITY.md), not public issues.
